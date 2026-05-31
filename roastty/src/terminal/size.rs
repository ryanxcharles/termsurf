use std::marker::PhantomData;
use std::mem::align_of;

pub(super) const MAX_PAGE_SIZE: u32 = u32::MAX;

pub(super) type OffsetInt = u32;
pub(super) type CellCountInt = u16;
pub(super) type StyleCountInt = CellCountInt;
pub(super) type HyperlinkCountInt = CellCountInt;
pub(super) type GraphemeBytesInt = u32;
pub(super) type StringBytesInt = u32;

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Offset<T> {
    offset: OffsetInt,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Offset<T> {
    pub(super) const fn new(offset: OffsetInt) -> Self {
        Self {
            offset,
            _marker: PhantomData,
        }
    }

    pub(super) const fn offset(self) -> OffsetInt {
        self.offset
    }

    pub(super) fn ptr<B>(self, base: B) -> *const T
    where
        B: BaseAddress,
    {
        let addr = base.base_addr() + self.offset as usize;
        assert_eq!(addr % align_of::<T>(), 0);
        addr as *const T
    }

    pub(super) fn ptr_mut<B>(self, base: B) -> *mut T
    where
        B: BaseAddress,
    {
        self.ptr(base).cast_mut()
    }
}

impl<T> Clone for Offset<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Offset<T> {}

impl<T> std::hash::Hash for Offset<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.offset.hash(state);
    }
}

impl<T> Default for Offset<T> {
    fn default() -> Self {
        Self::new(0)
    }
}

#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub(super) struct OffsetSlice<T> {
    offset: Offset<T>,
    len: usize,
}

impl<T> OffsetSlice<T> {
    pub(super) const fn new(offset: Offset<T>, len: usize) -> Self {
        Self { offset, len }
    }

    pub(super) unsafe fn slice<'a, B>(self, base: B) -> &'a [T]
    where
        B: BaseAddress,
    {
        // Safety: callers must ensure the derived pointer is valid for `len`
        // contiguous instances of `T` for the returned lifetime.
        unsafe { std::slice::from_raw_parts(self.offset.ptr(base), self.len) }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct OffsetBuf {
    base: *mut u8,
    offset: usize,
}

impl OffsetBuf {
    pub(super) fn init<B>(base: B) -> Self
    where
        B: BaseAddress,
    {
        Self::init_offset(base, 0)
    }

    pub(super) fn init_offset<B>(base: B, offset: usize) -> Self
    where
        B: BaseAddress,
    {
        Self {
            base: base.base_addr() as *mut u8,
            offset,
        }
    }

    pub(super) fn start(self) -> *mut u8 {
        (self.base as usize + self.offset) as *mut u8
    }

    pub(super) fn member<T>(self, len: usize) -> Offset<T> {
        Offset::new(checked_offset(
            self.offset
                .checked_add(len)
                .expect("offset member calculation overflowed usize"),
        ))
    }

    pub(super) fn add(self, offset: usize) -> Self {
        Self {
            base: self.base,
            offset: self
                .offset
                .checked_add(offset)
                .expect("offset buffer add overflowed usize"),
        }
    }

    pub(super) fn rebase(self, offset: usize) -> Self {
        Self {
            base: (self.start() as usize + offset) as *mut u8,
            offset: 0,
        }
    }
}

pub(super) fn get_offset<T, B>(base: B, ptr: *const T) -> Offset<T>
where
    B: BaseAddress,
{
    let base_int = base.base_addr();
    let ptr_int = ptr as usize;
    let offset = ptr_int
        .checked_sub(base_int)
        .expect("pointer is before base address");
    Offset::new(checked_offset(offset))
}

pub(super) trait BaseAddress {
    fn base_addr(self) -> usize;
}

impl<T> BaseAddress for *const T {
    fn base_addr(self) -> usize {
        self as usize
    }
}

impl<T> BaseAddress for *mut T {
    fn base_addr(self) -> usize {
        self as usize
    }
}

impl<T> BaseAddress for &[T] {
    fn base_addr(self) -> usize {
        self.as_ptr() as usize
    }
}

impl<T> BaseAddress for &mut [T] {
    fn base_addr(self) -> usize {
        self.as_mut_ptr() as usize
    }
}

impl BaseAddress for OffsetBuf {
    fn base_addr(self) -> usize {
        self.base as usize
    }
}

fn checked_offset(offset: usize) -> OffsetInt {
    OffsetInt::try_from(offset).expect("offset does not fit in OffsetInt")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn offset() {
        assert_eq!(MAX_PAGE_SIZE, u32::MAX);
        assert_eq!(size_of::<OffsetInt>(), size_of::<u32>());
        assert_eq!(size_of::<Offset<u8>>(), size_of::<u32>());
        assert_eq!(align_of::<Offset<u8>>(), align_of::<u32>());
    }

    #[test]
    fn offset_ptr_u8() {
        let offset: Offset<u8> = Offset::new(42);
        let base_int = &offset as *const _ as usize;
        let actual = offset.ptr(&offset as *const _);

        assert_eq!(base_int + 42, actual as usize);
    }

    #[test]
    fn offset_ptr_structural() {
        #[repr(C)]
        struct Widget {
            x: u32,
            y: u32,
        }

        let offset: Offset<Widget> = Offset::new((align_of::<Widget>() * 4) as OffsetInt);
        let base_int = align_forward(&offset as *const _ as usize, align_of::<Widget>());
        let actual = offset.ptr(base_int as *const u8);

        assert_eq!(base_int + offset.offset() as usize, actual as usize);
    }

    #[test]
    fn get_offset_bytes() {
        let widgets = *b"ABCD";
        let offset = get_offset(widgets.as_ptr(), unsafe { widgets.as_ptr().add(2) });

        assert_eq!(2, offset.offset());
    }

    #[test]
    fn get_offset_structs() {
        #[repr(C)]
        struct Widget {
            x: u32,
            y: u32,
        }

        let widgets = [
            Widget { x: 1, y: 2 },
            Widget { x: 3, y: 4 },
            Widget { x: 5, y: 6 },
            Widget { x: 7, y: 8 },
            Widget { x: 9, y: 10 },
        ];
        let offset = get_offset(widgets.as_ptr(), &widgets[2] as *const Widget);

        assert_eq!((size_of::<Widget>() * 2) as OffsetInt, offset.offset());
    }

    #[test]
    fn offset_slice_maps_expected_range() {
        let values = [1_u32, 2, 3, 4];
        let offset = get_offset(values.as_ptr(), &values[1] as *const u32);
        let offset_slice = OffsetSlice::new(offset, 2);
        let actual = unsafe { offset_slice.slice(values.as_ptr()) };

        assert_eq!(&values[1..3], actual);
    }

    #[test]
    fn offset_buf_member() {
        let bytes = [0_u8; 16];
        let buf = OffsetBuf::init_offset(bytes.as_ptr(), 4);
        let member: Offset<u32> = buf.member(8);

        assert_eq!(12, member.offset());
    }

    #[test]
    fn offset_buf_add() {
        let bytes = [0_u8; 16];
        let buf = OffsetBuf::init(bytes.as_ptr()).add(6);

        assert_eq!(bytes.as_ptr() as usize + 6, buf.start() as usize);
    }

    #[test]
    fn offset_buf_rebase() {
        let bytes = [0_u8; 16];
        let buf = OffsetBuf::init_offset(bytes.as_ptr(), 4).rebase(2);

        assert_eq!(bytes.as_ptr() as usize + 6, buf.start() as usize);
    }

    #[test]
    #[should_panic(expected = "assertion `left == right` failed")]
    fn offset_ptr_misaligned_panics() {
        let bytes = [0_u8; 16];
        let offset: Offset<u32> = Offset::new(1);

        let _ = offset.ptr(bytes.as_ptr());
    }

    #[test]
    #[should_panic(expected = "pointer is before base address")]
    fn get_offset_rejects_negative_offset() {
        let bytes = [0_u8; 16];

        let _ = get_offset(unsafe { bytes.as_ptr().add(4) }, bytes.as_ptr());
    }

    #[test]
    #[should_panic(expected = "offset does not fit in OffsetInt")]
    fn offset_buf_member_rejects_too_large_offset() {
        let buf = OffsetBuf::init(std::ptr::null::<u8>());

        let _: Offset<u8> = buf.member(u32::MAX as usize + 1);
    }

    fn align_forward(value: usize, align: usize) -> usize {
        debug_assert!(align.is_power_of_two());
        (value + align - 1) & !(align - 1)
    }
}
