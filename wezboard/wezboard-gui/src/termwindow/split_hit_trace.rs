use super::{TermWindow, UIItem, UIItemType};
use ::window::MouseEvent;
use mux::tab::{PositionedPane, PositionedSplit};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const TRACE_PATH: &str = "/Users/ryan/dev/termsurf/logs/split-hitbox.log";

static TRACE_FILE: OnceLock<Mutex<Option<File>>> = OnceLock::new();
static TRACE_FRAME: AtomicU64 = AtomicU64::new(0);
static TRACE_EVENT: AtomicU64 = AtomicU64::new(0);

fn trace_file() -> &'static Mutex<Option<File>> {
    TRACE_FILE.get_or_init(|| {
        let file = if std::env::var_os("TERMSURF_SPLIT_HIT_TRACE").is_some() {
            let path = Path::new(TRACE_PATH);
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path)
                .ok();
            if let Some(file) = file.as_mut() {
                let _ = writeln!(
                    file,
                    "split-hit meta resolution=first-match-wins item_order=reverse-paint-registration log=/Users/ryan/dev/termsurf/logs/split-hitbox.log"
                );
            }
            file
        } else {
            None
        };
        Mutex::new(file)
    })
}

pub fn enabled() -> bool {
    trace_file().lock().unwrap().is_some()
}

pub fn write_line(line: impl AsRef<str>) {
    let mut guard = trace_file().lock().unwrap();
    if let Some(file) = guard.as_mut() {
        let _ = writeln!(file, "{}", line.as_ref());
        let _ = file.flush();
    }
}

pub fn next_frame() -> u64 {
    TRACE_FRAME.fetch_add(1, Ordering::Relaxed) + 1
}

pub fn latest_frame() -> u64 {
    TRACE_FRAME.load(Ordering::Relaxed)
}

pub fn next_event() -> u64 {
    TRACE_EVENT.fetch_add(1, Ordering::Relaxed) + 1
}

pub fn item_label(item: &UIItemType) -> String {
    match item {
        UIItemType::TabBar(_) => "TabBar".to_string(),
        UIItemType::CloseTab(index) => format!("CloseTab[index={}]", index),
        UIItemType::AboveScrollThumb => "AboveScrollThumb".to_string(),
        UIItemType::ScrollThumb => "ScrollThumb".to_string(),
        UIItemType::BelowScrollThumb => "BelowScrollThumb".to_string(),
        UIItemType::Split(split) => format!(
            "Split[index={},direction={:?}]",
            split.index, split.direction
        ),
    }
}

pub fn ui_item_rect(item: &UIItem) -> String {
    format!(
        "rect_px=(x={} y={} w={} h={})",
        item.x, item.y, item.width, item.height
    )
}

impl TermWindow {
    pub fn split_hit_trace_frame(&self, panes: &[PositionedPane]) {
        if !enabled() {
            return;
        }

        let frame = next_frame();
        let cell_width = self.render_metrics.cell_size.width;
        let cell_height = self.render_metrics.cell_size.height;
        write_line(format!(
            "split-hit frame={} terminal_cells={}x{} cell_px={}x{} dpi={}",
            frame,
            self.terminal_size.cols,
            self.terminal_size.rows,
            cell_width,
            cell_height,
            self.dimensions.dpi
        ));

        for pane in panes {
            let content = self.split_hit_trace_content_rect(pane);
            let border = pane.border.as_ref().map(|border| {
                let cell_width = self.render_metrics.cell_size.width as f32;
                let cell_height = self.render_metrics.cell_size.height as f32;
                let (origin_x, origin_y) = self.split_hit_trace_origin();
                (
                    origin_x + border.outer_left as f32 * cell_width,
                    origin_y + border.outer_top as f32 * cell_height,
                    border.outer_width as f32 * cell_width,
                    border.outer_height as f32 * cell_height,
                )
            });
            if let Some((border_x, border_y, border_w, border_h)) = border {
                write_line(format!(
                    "split-hit pane frame={} pane_id={} active={} content_cell=(left={} top={} width={} height={}) content_px=(x={:.1} y={:.1} w={:.1} h={:.1}) border_cell=(left={} top={} width={} height={}) border_px=(x={:.1} y={:.1} w={:.1} h={:.1})",
                    frame,
                    pane.pane.pane_id(),
                    pane.is_active,
                    pane.left,
                    pane.top,
                    pane.width,
                    pane.height,
                    content.0,
                    content.1,
                    content.2,
                    content.3,
                    pane.border.as_ref().unwrap().outer_left,
                    pane.border.as_ref().unwrap().outer_top,
                    pane.border.as_ref().unwrap().outer_width,
                    pane.border.as_ref().unwrap().outer_height,
                    border_x,
                    border_y,
                    border_w,
                    border_h
                ));
            }
        }
    }

    pub fn split_hit_trace_pane_border_edge(
        &self,
        pane: &PositionedPane,
        edge: &str,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        if !enabled() {
            return;
        }

        write_line(format!(
            "split-hit pane-border-edge frame={} pane_id={} active={} edge={} rect_px=(x={:.1} y={:.1} w={:.1} h={:.1})",
            latest_frame(),
            pane.pane.pane_id(),
            pane.is_active,
            edge,
            x,
            y,
            width,
            height
        ));
    }

    pub fn split_hit_trace_split_ui(&self, split: &PositionedSplit, item: &UIItem) {
        if !enabled() {
            return;
        }

        write_line(format!(
            "split-hit split-ui frame={} index={} direction={:?} logical_cell=(left={} top={}) hit_cell=(left={} top={}) {}",
            latest_frame(),
            split.index,
            split.direction,
            split.left,
            split.top,
            split.hit_left,
            split.hit_top,
            ui_item_rect(item)
        ));
    }

    pub fn split_hit_trace_resolve_ui_item(&self, event: &MouseEvent) -> Option<UIItem> {
        let x = event.coords.x;
        let y = event.coords.y;
        if !enabled() || !self.split_hit_trace_near_split(x, y) {
            return self
                .ui_items
                .iter()
                .rev()
                .find(|item| item.hit_test(x, y))
                .cloned();
        }

        let event_id = next_event();
        let frame = latest_frame();
        let (col, row) = self.split_hit_trace_mouse_cell(x, y);
        let near_split = self.split_hit_trace_nearest_split(x, y);
        write_line(format!(
            "split-hit mouse event={} frame={} px=(x={} y={}) cell=(col={} row={}) near_split={}",
            event_id,
            frame,
            x,
            y,
            col,
            row,
            near_split
                .map(|index| index.to_string())
                .unwrap_or_else(|| "none".to_string())
        ));

        let mut winner = None;
        for (order, item) in self.ui_items.iter().rev().enumerate() {
            let contains = item.hit_test(x, y);
            write_line(format!(
                "split-hit candidate event={} frame={} order={} item={} {} contains={}",
                event_id,
                frame,
                order,
                item_label(&item.item_type),
                ui_item_rect(item),
                contains
            ));
            if contains {
                winner = Some(item.clone());
                break;
            }
        }

        write_line(format!(
            "split-hit winner event={} frame={} item={} {}",
            event_id,
            frame,
            winner
                .as_ref()
                .map(|item| item_label(&item.item_type))
                .unwrap_or_else(|| "None".to_string()),
            winner
                .as_ref()
                .map(ui_item_rect)
                .unwrap_or_else(|| "rect_px=(x=0 y=0 w=0 h=0)".to_string())
        ));

        winner
    }

    pub fn split_hit_trace_hover_change(
        &self,
        event: &MouseEvent,
        from: Option<&UIItem>,
        to: Option<&UIItem>,
    ) {
        if !enabled() {
            return;
        }

        let event_id = next_event();
        let frame = latest_frame();
        let (col, row) = self.split_hit_trace_mouse_cell(event.coords.x, event.coords.y);
        write_line(format!(
            "split-hit hover-change event={} frame={} from={} to={} px=(x={} y={}) cell=(col={} row={})",
            event_id,
            frame,
            from.map(|item| item_label(&item.item_type))
                .unwrap_or_else(|| "None".to_string()),
            to.map(|item| item_label(&item.item_type))
                .unwrap_or_else(|| "None".to_string()),
            event.coords.x,
            event.coords.y,
            col,
            row
        ));
    }

    fn split_hit_trace_content_rect(&self, pane: &PositionedPane) -> (f32, f32, f32, f32) {
        let cell_width = self.render_metrics.cell_size.width as f32;
        let cell_height = self.render_metrics.cell_size.height as f32;
        let (origin_x, origin_y) = self.split_hit_trace_origin();
        (
            origin_x + pane.left as f32 * cell_width,
            origin_y + pane.top as f32 * cell_height,
            pane.width as f32 * cell_width,
            pane.height as f32 * cell_height,
        )
    }

    fn split_hit_trace_origin(&self) -> (f32, f32) {
        let (padding_left, padding_top) = self.padding_left_top();
        let tab_bar_height = if self.show_tab_bar {
            self.tab_bar_pixel_height().unwrap_or(0.)
        } else {
            0.
        };
        let top_bar_height = if self.config.tab_bar_at_bottom {
            0.
        } else {
            tab_bar_height
        };
        let border = self.get_os_border();
        (
            padding_left + border.left.get() as f32,
            top_bar_height + padding_top + border.top.get() as f32,
        )
    }

    fn split_hit_trace_mouse_cell(&self, x: isize, y: isize) -> (isize, isize) {
        let (origin_x, origin_y) = self.split_hit_trace_origin();
        let cell_width = self.render_metrics.cell_size.width as f32;
        let cell_height = self.render_metrics.cell_size.height as f32;
        (
            (((x as f32) - origin_x) / cell_width).floor() as isize,
            (((y as f32) - origin_y) / cell_height).floor() as isize,
        )
    }

    fn split_hit_trace_near_split(&self, x: isize, y: isize) -> bool {
        self.split_hit_trace_nearest_split(x, y).is_some()
    }

    fn split_hit_trace_nearest_split(&self, x: isize, y: isize) -> Option<usize> {
        let cell_width = self.render_metrics.cell_size.width as isize;
        let cell_height = self.render_metrics.cell_size.height as isize;
        let threshold_x = cell_width.saturating_mul(2);
        let threshold_y = cell_height.saturating_mul(2);

        self.ui_items.iter().find_map(|item| {
            let UIItemType::Split(split) = &item.item_type else {
                return None;
            };
            let left = item.x as isize - threshold_x;
            let right = (item.x + item.width) as isize + threshold_x;
            let top = item.y as isize - threshold_y;
            let bottom = (item.y + item.height) as isize + threshold_y;
            if x >= left && x <= right && y >= top && y <= bottom {
                Some(split.index)
            } else {
                None
            }
        })
    }
}
