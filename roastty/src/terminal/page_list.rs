use std::ptr::NonNull;

use super::page::{page_layout, Capacity, CapacityAdjustment, Page, PageAllocError, STD_CAPACITY};
use super::size::CellCountInt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Viewport {
    Active,
    Top,
    Pin,
}

#[derive(Debug)]
pub(super) struct PageList {
    cols: CellCountInt,
    rows: CellCountInt,
    pages: Vec<Box<Node>>,
    page_serial: u64,
    page_serial_min: u64,
    page_size: usize,
    explicit_max_size: usize,
    min_max_size: usize,
    total_rows: CellCountInt,
    tracked_pins: Vec<NonNull<Pin>>,
    viewport: Viewport,
    viewport_pin: Box<Pin>,
}

#[derive(Debug)]
struct Node {
    page: Page,
    serial: u64,
}

#[derive(Debug)]
struct Pin {
    node: NonNull<Node>,
    y: CellCountInt,
    x: CellCountInt,
    garbage: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PageListAllocError {
    PageAlloc,
}

impl From<PageAllocError> for PageListAllocError {
    fn from(_: PageAllocError) -> Self {
        Self::PageAlloc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntegrityError {
    PageSerialInvalid,
    TotalRowsMismatch,
    TrackedPinInvalid,
    ViewportPinInvalid,
    ViewportPinGarbage,
}

fn standard_page_size() -> usize {
    page_layout(STD_CAPACITY).total_size()
}

fn initial_capacity(cols: CellCountInt) -> Capacity {
    if let Ok(capacity) = STD_CAPACITY.adjust(CapacityAdjustment::cols(cols)) {
        return capacity;
    }

    STD_CAPACITY.with_cols(cols)
}

fn min_max_size(cols: CellCountInt, rows: CellCountInt) -> usize {
    let capacity = initial_capacity(cols);
    let capacity_rows = capacity.rows() as usize;
    let rows = rows as usize;
    let pages_exact = if capacity_rows >= rows {
        1
    } else {
        rows.div_ceil(capacity_rows)
    };
    let pages = pages_exact + 1;
    debug_assert!(pages >= 2);

    standard_page_size() * pages
}

impl PageList {
    fn init(
        cols: CellCountInt,
        rows: CellCountInt,
        max_size: Option<usize>,
    ) -> Result<Self, PageListAllocError> {
        assert!(rows > 0);

        let mut page_serial = 0;
        let (pages, page_size) = init_pages(&mut page_serial, cols, rows)?;
        let first_node = NonNull::from(pages[0].as_ref());
        let mut viewport_pin = Box::new(Pin {
            node: first_node,
            y: 0,
            x: 0,
            garbage: false,
        });
        let tracked_pins = vec![NonNull::from(viewport_pin.as_mut())];

        let result = Self {
            cols,
            rows,
            pages,
            page_serial,
            page_serial_min: 0,
            page_size,
            explicit_max_size: max_size.unwrap_or(usize::MAX),
            min_max_size: min_max_size(cols, rows),
            total_rows: rows,
            tracked_pins,
            viewport: Viewport::Active,
            viewport_pin,
        };
        result
            .verify_integrity()
            .expect("newly initialized PageList should be valid");
        Ok(result)
    }

    fn verify_integrity(&self) -> Result<(), IntegrityError> {
        let mut actual_total_rows = 0usize;
        for node in &self.pages {
            actual_total_rows += node.page.size_rows() as usize;
            if node.serial < self.page_serial_min {
                return Err(IntegrityError::PageSerialInvalid);
            }
        }

        if actual_total_rows != self.total_rows as usize {
            return Err(IntegrityError::TotalRowsMismatch);
        }

        for pin in &self.tracked_pins {
            let pin = unsafe {
                // Safety: tracked pins are created from stable Box<Pin>
                // allocations owned by this PageList. Later mutation code must
                // remove pins before freeing them.
                pin.as_ref()
            };
            if !self.pin_is_valid(pin) {
                return Err(IntegrityError::TrackedPinInvalid);
            }
        }

        if self.viewport_pin.garbage {
            return Err(IntegrityError::ViewportPinGarbage);
        }

        if !self.pin_is_valid(&self.viewport_pin) {
            return Err(IntegrityError::ViewportPinInvalid);
        }

        Ok(())
    }

    fn pin_is_valid(&self, pin: &Pin) -> bool {
        let Some(node) = self.node_for_pin(pin) else {
            return false;
        };

        pin.x < node.page.size_cols() && pin.y < node.page.size_rows()
    }

    fn node_for_pin(&self, pin: &Pin) -> Option<&Node> {
        self.pages
            .iter()
            .map(Box::as_ref)
            .find(|node| NonNull::from(*node) == pin.node)
    }

    fn total_rows(&self) -> usize {
        self.pages
            .iter()
            .map(|node| node.page.size_rows() as usize)
            .sum()
    }

    fn active_top_left(&self) -> &Pin {
        &self.viewport_pin
    }
}

fn init_pages(
    serial: &mut u64,
    cols: CellCountInt,
    rows: CellCountInt,
) -> Result<(Vec<Box<Node>>, usize), PageListAllocError> {
    let capacity = initial_capacity(cols);
    let mut remaining_rows = rows as usize;
    let mut pages = Vec::new();
    let mut page_size = 0;

    while remaining_rows > 0 {
        let mut page = Page::init(capacity)?;
        let active_rows = remaining_rows.min(capacity.rows() as usize);
        page.set_size_rows(
            active_rows
                .try_into()
                .expect("active page row count must fit CellCountInt"),
        );
        remaining_rows -= active_rows;
        page_size += page.backing_len();

        pages.push(Box::new(Node {
            page,
            serial: *serial,
        }));
        *serial += 1;
    }

    debug_assert!(!pages.is_empty());
    Ok((pages, page_size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::page::page_layout;

    #[test]
    fn viewport_variants_compare_as_expected() {
        assert_eq!(Viewport::Active, Viewport::Active);
        assert_eq!(Viewport::Top, Viewport::Top);
        assert_eq!(Viewport::Pin, Viewport::Pin);
        assert_ne!(Viewport::Active, Viewport::Top);
        assert_ne!(Viewport::Active, Viewport::Pin);
        assert_ne!(Viewport::Top, Viewport::Pin);
    }

    #[test]
    fn initial_capacity_normal_width_preserves_standard_size() {
        let standard_size = standard_page_size();
        let capacity = initial_capacity(80);

        assert_eq!(capacity.cols(), 80);
        assert!(capacity.rows() >= 1);
        assert_eq!(page_layout(capacity).total_size(), standard_size);
    }

    #[test]
    fn initial_capacity_max_standard_width_preserves_standard_size() {
        let standard_size = standard_page_size();
        let max_cols = STD_CAPACITY
            .max_cols()
            .expect("standard capacity should fit at least one row");
        let capacity = initial_capacity(max_cols);

        assert_eq!(capacity.cols(), max_cols);
        assert_eq!(capacity.rows(), 1);
        assert_eq!(page_layout(capacity).total_size(), standard_size);
    }

    #[test]
    fn initial_capacity_too_wide_uses_non_standard_page() {
        let standard_size = standard_page_size();
        let max_cols = STD_CAPACITY
            .max_cols()
            .expect("standard capacity should fit at least one row");
        assert!(max_cols < CellCountInt::MAX);
        let requested_cols = max_cols + 1;
        let capacity = initial_capacity(requested_cols);

        assert_eq!(capacity.cols(), requested_cols);
        assert_eq!(capacity.rows(), STD_CAPACITY.rows());
        assert!(page_layout(capacity).total_size() > standard_size);
    }

    #[test]
    fn initial_capacity_max_columns_lays_out() {
        let capacity = initial_capacity(CellCountInt::MAX);
        let layout = page_layout(capacity);

        assert_eq!(capacity.cols(), CellCountInt::MAX);
        assert!(capacity.rows() >= 1);
        assert!(layout.total_size() >= standard_page_size());
    }

    #[test]
    fn min_max_size_normal_dimensions_are_two_standard_pages() {
        assert_eq!(min_max_size(80, 24), standard_page_size() * 2);
    }

    #[test]
    fn min_max_size_adds_extra_page_for_multi_page_active_area() {
        let cols = 80;
        let capacity = initial_capacity(cols);
        let rows = capacity.rows() + 1;
        let expected_pages = (rows as usize).div_ceil(capacity.rows() as usize) + 1;

        assert!(expected_pages > 2);
        assert_eq!(
            min_max_size(cols, rows),
            standard_page_size() * expected_pages
        );
    }

    #[test]
    fn page_list_init() {
        let list = PageList::init(80, 24, None).unwrap();

        assert_eq!(list.cols, 80);
        assert_eq!(list.rows, 24);
        assert_eq!(list.viewport, Viewport::Active);
        assert!(!list.pages.is_empty());
        assert_eq!(list.total_rows(), 24);
        assert_eq!(list.total_rows, 24);
        assert_eq!(list.page_serial, list.pages.len() as u64);
        assert_eq!(list.page_serial_min, 0);
        assert_eq!(list.explicit_max_size, usize::MAX);
        assert_eq!(list.min_max_size, min_max_size(80, 24));
        assert_eq!(list.page_size, list.pages[0].page.backing_len());

        let top_left = list.active_top_left();
        assert_eq!(top_left.node, NonNull::from(list.pages[0].as_ref()));
        assert_eq!(top_left.x, 0);
        assert_eq!(top_left.y, 0);
        assert!(!top_left.garbage);
        assert_eq!(list.tracked_pins.len(), 1);
        assert_eq!(list.tracked_pins[0], NonNull::from(&*list.viewport_pin));
        list.verify_integrity().unwrap();
    }

    #[test]
    fn page_list_init_respects_max_size_metadata() {
        let list = PageList::init(80, 24, Some(1024)).unwrap();

        assert_eq!(list.explicit_max_size, 1024);
        list.verify_integrity().unwrap();
    }

    #[test]
    fn page_list_init_rows_across_two_pages() {
        let rows = 100;
        let mut capacity = STD_CAPACITY.adjust(CapacityAdjustment::cols(50)).unwrap();
        while capacity.rows() >= rows {
            capacity = STD_CAPACITY
                .adjust(CapacityAdjustment::cols(capacity.cols() + 50))
                .unwrap();
        }

        let list = PageList::init(capacity.cols(), rows, None).unwrap();

        assert!(list.pages.len() > 1);
        assert_eq!(list.viewport, Viewport::Active);
        assert_eq!(list.total_rows(), rows as usize);
        assert_eq!(list.total_rows, rows);
        assert_eq!(list.pages[0].page.size_rows(), capacity.rows());
        assert_eq!(
            list.pages.last().unwrap().page.size_rows() as usize,
            rows as usize % capacity.rows() as usize
        );
        list.verify_integrity().unwrap();
    }

    #[test]
    fn page_list_init_more_than_max_cols() {
        let requested_cols = STD_CAPACITY.max_cols().unwrap() + 1;
        let list = PageList::init(requested_cols, 80, None).unwrap();

        assert_eq!(list.viewport, Viewport::Active);
        assert_eq!(list.total_rows(), 80);
        assert_eq!(list.total_rows, 80);
        assert_eq!(list.pages.len(), 1);
        assert_eq!(list.pages[0].page.size_cols(), requested_cols);
        assert!(list.pages[0].page.backing_len() > standard_page_size());
        list.verify_integrity().unwrap();
    }

    #[test]
    fn page_list_integrity_rejects_total_rows_mismatch() {
        let mut list = PageList::init(80, 24, None).unwrap();
        list.total_rows += 1;

        assert_eq!(
            list.verify_integrity(),
            Err(IntegrityError::TotalRowsMismatch)
        );
    }

    #[test]
    fn page_list_integrity_rejects_invalid_serial() {
        let mut list = PageList::init(80, 24, None).unwrap();
        list.page_serial_min = list.pages[0].serial + 1;

        assert_eq!(
            list.verify_integrity(),
            Err(IntegrityError::PageSerialInvalid)
        );
    }

    #[test]
    fn page_list_integrity_rejects_garbage_viewport_pin() {
        let mut list = PageList::init(80, 24, None).unwrap();
        list.viewport_pin.garbage = true;

        assert_eq!(
            list.verify_integrity(),
            Err(IntegrityError::ViewportPinGarbage)
        );
    }

    #[test]
    fn page_list_integrity_rejects_viewport_pin_x_out_of_bounds() {
        let mut list = PageList::init(80, 24, None).unwrap();
        list.viewport_pin.x = list.pages[0].page.size_cols();

        assert_eq!(
            list.verify_integrity(),
            Err(IntegrityError::TrackedPinInvalid)
        );
    }

    #[test]
    fn page_list_integrity_rejects_viewport_pin_y_out_of_bounds() {
        let mut list = PageList::init(80, 24, None).unwrap();
        list.viewport_pin.y = list.pages[0].page.size_rows();

        assert_eq!(
            list.verify_integrity(),
            Err(IntegrityError::TrackedPinInvalid)
        );
    }
}
