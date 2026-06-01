use std::ptr::NonNull;

use super::page_list::Pin;
use super::point;
use super::terminal::{Terminal, TerminalGridRef, TerminalScreenKey, TerminalSelection};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionGestureAutoscroll {
    None,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionGestureBehavior {
    Cell,
    Word,
    Line,
    Output,
}

pub(crate) const DEFAULT_BEHAVIORS: [SelectionGestureBehavior; 3] = [
    SelectionGestureBehavior::Cell,
    SelectionGestureBehavior::Word,
    SelectionGestureBehavior::Line,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectionGestureGeometry {
    pub(crate) columns: u32,
    pub(crate) cell_width: u32,
    pub(crate) padding_left: u32,
    pub(crate) screen_height: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectionGestureAnchor {
    pub(super) pin: NonNull<Pin>,
    pub(super) screen_key: TerminalScreenKey,
    pub(super) screen_generation: u64,
    pub(super) screen_owner_id: u64,
    pub(super) active_epoch: u64,
}

#[derive(Debug)]
pub(crate) struct SelectionGesture {
    anchor: Option<SelectionGestureAnchor>,
    click_count: u8,
    click_time_ns: Option<u64>,
    behavior: SelectionGestureBehavior,
    click_x: f64,
    click_y: f64,
    dragged: bool,
    autoscroll: SelectionGestureAutoscroll,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectionGesturePress<'a> {
    pub(crate) time_ns: Option<u64>,
    pub(crate) pin: Pin,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) max_distance: f64,
    pub(crate) repeat_interval_ns: u64,
    pub(crate) word_boundary_codepoints: Option<&'a [u32]>,
    pub(crate) behaviors: [SelectionGestureBehavior; 3],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectionGestureDrag<'a> {
    pub(crate) pin: Pin,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) rectangle: bool,
    pub(crate) word_boundary_codepoints: Option<&'a [u32]>,
    pub(crate) geometry: SelectionGestureGeometry,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectionGestureAutoscrollTick<'a> {
    pub(crate) viewport: point::Coordinate,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) rectangle: bool,
    pub(crate) word_boundary_codepoints: Option<&'a [u32]>,
    pub(crate) geometry: SelectionGestureGeometry,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectionGestureDeepPress<'a> {
    pub(crate) word_boundary_codepoints: Option<&'a [u32]>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectionGestureRelease {
    pub(crate) pin: Option<Pin>,
}

impl Default for SelectionGesture {
    fn default() -> Self {
        Self {
            anchor: None,
            click_count: 0,
            click_time_ns: None,
            behavior: SelectionGestureBehavior::Cell,
            click_x: 0.0,
            click_y: 0.0,
            dragged: false,
            autoscroll: SelectionGestureAutoscroll::None,
        }
    }
}

impl SelectionGesture {
    pub(crate) fn click_count(&self) -> u8 {
        self.click_count
    }

    pub(crate) fn dragged(&self) -> bool {
        self.dragged
    }

    pub(crate) fn autoscroll(&self) -> SelectionGestureAutoscroll {
        self.autoscroll
    }

    pub(crate) fn behavior(&self) -> SelectionGestureBehavior {
        self.behavior
    }

    pub(crate) fn anchor_ref(&self, terminal: &Terminal) -> Option<TerminalGridRef> {
        terminal.selection_gesture_anchor_ref(self.anchor.as_ref()?)
    }

    pub(crate) fn reset(&mut self, terminal: Option<&mut Terminal>) {
        self.click_count = 0;
        self.click_time_ns = None;
        self.behavior = SelectionGestureBehavior::Cell;
        self.dragged = false;
        self.autoscroll = SelectionGestureAutoscroll::None;
        self.untrack_anchor(terminal);
    }

    pub(crate) fn free(&mut self, terminal: Option<&mut Terminal>) {
        self.untrack_anchor(terminal);
    }

    pub(crate) fn press(
        &mut self,
        terminal: &mut Terminal,
        press: SelectionGesturePress<'_>,
    ) -> Option<TerminalSelection> {
        if self.click_count > 0 && self.press_repeat(terminal, &press) {
            return self.press_selection(terminal, &press);
        }

        self.press_initial(terminal, &press);
        self.press_selection(terminal, &press)
    }

    pub(crate) fn drag(
        &mut self,
        terminal: &mut Terminal,
        drag: SelectionGestureDrag<'_>,
    ) -> Option<TerminalSelection> {
        if self.click_count == 0 {
            self.autoscroll = SelectionGestureAutoscroll::None;
            return None;
        }

        let click_pin = self.validated_anchor_pin(terminal)?;
        if drag.pin != click_pin {
            self.dragged = true;
        }

        self.autoscroll = if drag.y <= 1.0 {
            SelectionGestureAutoscroll::Up
        } else if drag.y > drag.geometry.screen_height as f64 - 1.0 {
            SelectionGestureAutoscroll::Down
        } else {
            SelectionGestureAutoscroll::None
        };

        let selection = match self.behavior {
            SelectionGestureBehavior::Cell => terminal.drag_select_cells(
                click_pin,
                drag.pin,
                self.click_x.max(0.0) as u32,
                drag.x.max(0.0) as u32,
                drag.rectangle,
                drag.geometry,
            ),
            SelectionGestureBehavior::Word => {
                terminal.drag_select_word(click_pin, drag.pin, drag.word_boundary_codepoints)
            }
            SelectionGestureBehavior::Line => terminal.drag_select_line(click_pin, drag.pin),
            SelectionGestureBehavior::Output => terminal.drag_select_output(click_pin, drag.pin),
        };

        if matches!(self.behavior, SelectionGestureBehavior::Cell) && selection.is_some() {
            self.dragged = true;
        }

        selection
    }

    pub(crate) fn autoscroll_tick(
        &mut self,
        terminal: &mut Terminal,
        tick: SelectionGestureAutoscrollTick<'_>,
    ) -> Option<TerminalSelection> {
        if self.click_count == 0 {
            self.autoscroll = SelectionGestureAutoscroll::None;
            return None;
        }

        let delta = match self.autoscroll {
            SelectionGestureAutoscroll::None => return None,
            SelectionGestureAutoscroll::Up => -1,
            SelectionGestureAutoscroll::Down => 1,
        };

        if self.validated_anchor_pin(terminal).is_none() {
            self.reset(Some(terminal));
            return None;
        }

        terminal.scroll_selection_gesture_viewport(delta);
        let pin = terminal.viewport_pin(tick.viewport)?;
        self.drag(
            terminal,
            SelectionGestureDrag {
                pin,
                x: tick.x,
                y: tick.y,
                rectangle: tick.rectangle,
                word_boundary_codepoints: tick.word_boundary_codepoints,
                geometry: tick.geometry,
            },
        )
    }

    pub(crate) fn deep_press(
        &mut self,
        terminal: &mut Terminal,
        press: SelectionGestureDeepPress<'_>,
    ) -> Option<TerminalSelection> {
        let click_pin = self.validated_anchor_pin(terminal)?;
        let selection = terminal.select_word(
            TerminalGridRef::from(click_pin),
            press.word_boundary_codepoints,
        );

        self.click_count = 0;
        self.click_time_ns = None;
        self.behavior = SelectionGestureBehavior::Cell;
        self.dragged = true;
        self.autoscroll = SelectionGestureAutoscroll::None;
        self.untrack_anchor(Some(terminal));

        selection.ok().flatten()
    }

    pub(crate) fn release(&mut self, terminal: &Terminal, release: SelectionGestureRelease) {
        if self.click_count == 0 {
            self.autoscroll = SelectionGestureAutoscroll::None;
            return;
        }

        if let Some(release_pin) = release.pin {
            match self.validated_anchor_pin(terminal) {
                Some(click_pin) if release_pin == click_pin => {}
                _ => self.dragged = true,
            }
        } else {
            self.dragged = true;
        }
        self.autoscroll = SelectionGestureAutoscroll::None;
    }

    fn press_initial(&mut self, terminal: &mut Terminal, press: &SelectionGesturePress<'_>) {
        self.untrack_anchor(Some(terminal));
        self.anchor = terminal.track_selection_gesture_anchor(press.pin);
        self.click_count = 1;
        self.behavior = press.behaviors[0];
        self.click_x = press.x;
        self.click_y = press.y;
        self.click_time_ns = press.time_ns;
        self.dragged = false;
        self.autoscroll = SelectionGestureAutoscroll::None;
    }

    fn press_repeat(&mut self, terminal: &mut Terminal, press: &SelectionGesturePress<'_>) -> bool {
        let Some(time_ns) = press.time_ns else {
            self.reset(Some(terminal));
            return false;
        };
        let Some(previous_time_ns) = self.click_time_ns else {
            self.reset(Some(terminal));
            return false;
        };
        if time_ns.saturating_sub(previous_time_ns) > press.repeat_interval_ns {
            self.reset(Some(terminal));
            return false;
        }

        let dx = press.x - self.click_x;
        let dy = press.y - self.click_y;
        if (dx * dx + dy * dy).sqrt() > press.max_distance {
            self.reset(Some(terminal));
            return false;
        }

        if self.validated_anchor_pin(terminal).is_none() {
            self.reset(Some(terminal));
            return false;
        }

        self.click_time_ns = press.time_ns;
        self.dragged = false;
        self.autoscroll = SelectionGestureAutoscroll::None;
        self.click_count = self.click_count.saturating_add(1).min(3);
        self.behavior = press.behaviors[(self.click_count - 1) as usize];
        true
    }

    fn press_selection(
        &self,
        terminal: &Terminal,
        press: &SelectionGesturePress<'_>,
    ) -> Option<TerminalSelection> {
        let ref_ = TerminalGridRef::from(press.pin);
        match self.behavior {
            SelectionGestureBehavior::Cell => None,
            SelectionGestureBehavior::Word => terminal
                .select_word(ref_, press.word_boundary_codepoints)
                .ok()
                .flatten(),
            SelectionGestureBehavior::Line => {
                terminal.select_line(ref_, None, false).ok().flatten()
            }
            SelectionGestureBehavior::Output => terminal.select_output(ref_).ok().flatten(),
        }
    }

    fn validated_anchor_pin(&self, terminal: &Terminal) -> Option<Pin> {
        terminal.validated_selection_gesture_anchor(self.anchor.as_ref()?)
    }

    fn untrack_anchor(&mut self, terminal: Option<&mut Terminal>) {
        let Some(anchor) = self.anchor.take() else {
            return;
        };
        if let Some(terminal) = terminal {
            terminal.untrack_selection_gesture_anchor(anchor);
        }
    }
}

impl From<Pin> for TerminalGridRef {
    fn from(pin: Pin) -> Self {
        super::page_list::GridRef::from(pin).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terminal() -> Terminal {
        let mut terminal = Terminal::init(20, 4, Some(100)).unwrap();
        terminal.next_slice(b"abcde fghi\r\nsecond line").unwrap();
        terminal
    }

    fn active_pin(terminal: &Terminal, x: u16, y: u32) -> Pin {
        terminal.active_pin(point::Coordinate::new(x, y)).unwrap()
    }

    fn press<'a>(terminal: &Terminal, x: u16, time_ns: u64) -> SelectionGesturePress<'a> {
        SelectionGesturePress {
            time_ns: Some(time_ns),
            pin: active_pin(terminal, x, 0),
            x: f64::from(x) * 10.0,
            y: 0.0,
            max_distance: 20.0,
            repeat_interval_ns: 100,
            word_boundary_codepoints: None,
            behaviors: DEFAULT_BEHAVIORS,
        }
    }

    fn drag<'a>(terminal: &Terminal, x: u16, xpos: f64, ypos: f64) -> SelectionGestureDrag<'a> {
        SelectionGestureDrag {
            pin: active_pin(terminal, x, 0),
            x: xpos,
            y: ypos,
            rectangle: false,
            word_boundary_codepoints: None,
            geometry: SelectionGestureGeometry {
                columns: 20,
                cell_width: 10,
                padding_left: 0,
                screen_height: 100,
            },
        }
    }

    #[test]
    fn selection_gesture_single_double_triple_press() {
        let mut terminal = terminal();
        let mut gesture = SelectionGesture::default();

        let event = press(&terminal, 1, 1);
        assert_eq!(gesture.press(&mut terminal, event), None);
        assert_eq!(gesture.click_count(), 1);
        assert_eq!(gesture.behavior(), SelectionGestureBehavior::Cell);

        let event = press(&terminal, 1, 2);
        let word = gesture.press(&mut terminal, event).unwrap();
        assert_eq!((word.start.x, word.end.x), (0, 4));
        assert_eq!(gesture.click_count(), 2);
        assert_eq!(gesture.behavior(), SelectionGestureBehavior::Word);

        let event = press(&terminal, 1, 3);
        let line = gesture.press(&mut terminal, event).unwrap();
        assert_eq!((line.start.x, line.end.x), (0, 9));
        assert_eq!(gesture.click_count(), 3);
        assert_eq!(gesture.behavior(), SelectionGestureBehavior::Line);
    }

    #[test]
    fn selection_gesture_repeat_distance_and_interval_reset() {
        let mut terminal = terminal();
        let mut gesture = SelectionGesture::default();

        let event = press(&terminal, 1, 1);
        assert_eq!(gesture.press(&mut terminal, event), None);
        let event = press(&terminal, 10, 2);
        assert_eq!(gesture.press(&mut terminal, event), None);
        assert_eq!(gesture.click_count(), 1);

        let event = press(&terminal, 10, 500);
        assert_eq!(gesture.press(&mut terminal, event), None);
        assert_eq!(gesture.click_count(), 1);
    }

    #[test]
    fn selection_gesture_cell_drag_threshold_and_release() {
        let mut terminal = terminal();
        let mut gesture = SelectionGesture::default();

        let event = press(&terminal, 1, 1);
        assert_eq!(gesture.press(&mut terminal, event), None);
        let event = drag(&terminal, 1, 11.0, 10.0);
        assert_eq!(gesture.drag(&mut terminal, event), None);
        assert!(!gesture.dragged());

        let event = drag(&terminal, 3, 39.0, 10.0);
        let selection = gesture.drag(&mut terminal, event).unwrap();
        assert_eq!((selection.start.x, selection.end.x), (1, 3));
        assert!(gesture.dragged());

        gesture.release(
            &terminal,
            SelectionGestureRelease {
                pin: Some(active_pin(&terminal, 3, 0)),
            },
        );
        assert!(gesture.dragged());
        assert_eq!(gesture.autoscroll(), SelectionGestureAutoscroll::None);
    }

    #[test]
    fn selection_gesture_word_line_output_and_deep_press() {
        let mut terminal = terminal();
        let mut gesture = SelectionGesture::default();

        let event = press(&terminal, 1, 1);
        assert_eq!(gesture.press(&mut terminal, event), None);
        let word = gesture.deep_press(
            &mut terminal,
            SelectionGestureDeepPress {
                word_boundary_codepoints: None,
            },
        );
        assert_eq!(
            word.map(|selection| (selection.start.x, selection.end.x)),
            Some((0, 4))
        );
        assert_eq!(gesture.click_count(), 0);
        assert!(gesture.dragged());
    }

    #[test]
    fn selection_gesture_autoscroll_and_screen_invalidation() {
        let mut terminal = terminal();
        let mut gesture = SelectionGesture::default();

        let event = press(&terminal, 1, 1);
        assert_eq!(gesture.press(&mut terminal, event), None);
        let event = drag(&terminal, 3, 39.0, 101.0);
        let _ = gesture.drag(&mut terminal, event);
        assert_eq!(gesture.autoscroll(), SelectionGestureAutoscroll::Down);

        let tick = SelectionGestureAutoscrollTick {
            viewport: point::Coordinate::new(3, 0),
            x: 39.0,
            y: 101.0,
            rectangle: false,
            word_boundary_codepoints: None,
            geometry: SelectionGestureGeometry {
                columns: 20,
                cell_width: 10,
                padding_left: 0,
                screen_height: 100,
            },
        };
        let _ = gesture.autoscroll_tick(&mut terminal, tick);

        terminal.next_slice(b"\x1b[?1049h").unwrap();
        let event = drag(&terminal, 1, 10.0, 10.0);
        assert_eq!(gesture.drag(&mut terminal, event), None);
        assert_eq!(gesture.anchor_ref(&terminal), None);

        gesture.reset(Some(&mut terminal));
        assert_eq!(gesture.click_count(), 0);
    }

    #[test]
    fn selection_gesture_primary_reset_invalidates_but_allows_cleanup() {
        let mut terminal = terminal();
        let mut gesture = SelectionGesture::default();

        let event = press(&terminal, 1, 1);
        assert_eq!(gesture.press(&mut terminal, event), None);
        assert!(gesture.anchor_ref(&terminal).is_some());
        terminal.reset();
        assert_eq!(gesture.anchor_ref(&terminal), None);
        gesture.reset(Some(&mut terminal));
        assert_eq!(gesture.click_count(), 0);
    }

    #[test]
    fn selection_gesture_alternate_destroy_recreate_skips_stale_cleanup() {
        let mut terminal = terminal();
        let mut gesture = SelectionGesture::default();

        terminal.next_slice(b"\x1b[?1049h").unwrap();
        terminal.next_slice(b"alt text").unwrap();
        let event = press(&terminal, 1, 1);
        assert_eq!(gesture.press(&mut terminal, event), None);
        assert!(gesture.anchor_ref(&terminal).is_some());

        terminal.reset();
        terminal.next_slice(b"\x1b[?1049h").unwrap();
        terminal.next_slice(b"new alt").unwrap();

        assert_eq!(gesture.anchor_ref(&terminal), None);
        gesture.reset(Some(&mut terminal));
        assert_eq!(gesture.click_count(), 0);

        let event = press(&terminal, 1, 2);
        assert_eq!(gesture.press(&mut terminal, event), None);
        assert!(gesture.anchor_ref(&terminal).is_some());
        gesture.reset(Some(&mut terminal));
    }
}
