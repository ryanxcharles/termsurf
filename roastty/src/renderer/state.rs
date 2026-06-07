#![allow(dead_code)]
// Renderer state values are consumed by later renderer slices.

//! Renderer state values.
//!
//! Faithful value-level port of upstream `renderer/State.zig`: the IME preedit
//! text rendered over the cursor, renderer-relevant mouse state, and the outer
//! state values renderers consume. The upstream mutex, terminal pointer, and
//! inspector pointer are renderer-thread/frontend ownership details and remain
//! tracked by later integration slices.

use crate::input::key_mods::Mods;
use crate::terminal::point::Coordinate;

/// Cell-count unit. Mirrors `terminal::size::CellCountInt` (`u16`).
pub(crate) type Unit = u16;

/// A single preedit codepoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Codepoint {
    /// The Unicode scalar value. Mirrors upstream `u21`.
    pub codepoint: u32,
    pub wide: bool,
}

/// IME dead-key / preedit text to render over the cursor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Preedit {
    pub codepoints: Vec<Codepoint>,
}

/// The placement of preedit text: the start/end cell columns and any leading
/// codepoint offset needed to fit the text into the available space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreeditRange {
    pub start: Unit,
    pub end: Unit,
    pub cp_offset: usize,
}

/// Mouse state relevant to renderers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Mouse {
    /// The current viewport point of the mouse.
    pub point: Option<Coordinate>,

    /// The input modifiers active for the last mouse event.
    pub mods: Mods,
}

/// Value-level renderer state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct State {
    pub preedit: Option<Preedit>,
    pub mouse: Mouse,
}

impl State {
    pub(crate) fn new() -> State {
        State::default()
    }

    pub(crate) fn set_preedit(&mut self, preedit: Preedit) {
        self.preedit = Some(preedit);
    }

    pub(crate) fn clear_preedit(&mut self) {
        self.preedit = None;
    }

    pub(crate) fn set_mouse_point(&mut self, point: Coordinate) {
        self.mouse.point = Some(point);
    }

    pub(crate) fn clear_mouse_point(&mut self) {
        self.mouse.point = None;
    }

    pub(crate) fn set_mouse_mods(&mut self, mods: Mods) {
        self.mouse.mods = mods;
    }
}

impl Preedit {
    /// The width in cells of all codepoints in the preedit.
    pub(crate) fn width(&self) -> usize {
        let mut result = 0;
        for cp in &self.codepoints {
            result += if cp.wide { 2 } else { 1 };
        }
        result
    }

    /// Returns the start and end x position of the preedit text along with any
    /// codepoint offset necessary to fit the preedit into the available space.
    pub(crate) fn range(&self, start: Unit, max: Unit) -> PreeditRange {
        // If our width is greater than the number of cells we have then we need
        // to adjust our codepoint start to a point where our width fits.
        let len = self.codepoints.len();

        // max is inclusive, so add 1.
        let max_width = max - start + 1;

        // Rebuild our width in reverse order: we want to offset by the end
        // cells, not the start cells (if we have to). If the accumulated width
        // never exceeds max_width, the full width is used with cp_offset 0.
        let mut w: Unit = 0;
        let mut cp_offset: usize = 0;
        for i in 0..len {
            let reverse_i = len - i - 1;
            let cp = self.codepoints[reverse_i];
            w += if cp.wide { 2 } else { 1 };
            if w > max_width {
                cp_offset = reverse_i;
                break;
            }
        }

        // If our preedit goes off the end of the screen, shift it left.
        let end = if w > 0 { start + (w - 1) } else { start };
        let start_offset = if end > max { end - max } else { 0 };
        PreeditRange {
            start: start.saturating_sub(start_offset),
            end: end.saturating_sub(start_offset),
            cp_offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::key_mods::{Mod, Side};

    const HANGUL_GA: u32 = 0xAC00; // U+AC00 HANGUL SYLLABLE GA

    fn cp(codepoint: u32, wide: bool) -> Codepoint {
        Codepoint { codepoint, wide }
    }

    // Upstream "preedit range covers exact cell width".
    #[test]
    fn preedit_range_covers_exact_cell_width() {
        let p = Preedit {
            codepoints: vec![cp('a' as u32, false)],
        };
        assert_eq!(
            p.range(2, 9),
            PreeditRange {
                start: 2,
                end: 2,
                cp_offset: 0,
            }
        );

        let p = Preedit {
            codepoints: vec![cp(HANGUL_GA, true)],
        };
        assert_eq!(
            p.range(2, 9),
            PreeditRange {
                start: 2,
                end: 3,
                cp_offset: 0,
            }
        );
    }

    // Upstream "preedit range shifts left at right edge".
    #[test]
    fn preedit_range_shifts_left_at_right_edge() {
        let p = Preedit {
            codepoints: vec![cp(HANGUL_GA, true)],
        };
        assert_eq!(
            p.range(9, 9),
            PreeditRange {
                start: 8,
                end: 9,
                cp_offset: 0,
            }
        );
    }

    // Nonzero cp_offset truncation: not covered by the upstream tests.
    #[test]
    fn preedit_range_truncates_at_nonzero_offset() {
        let p = Preedit {
            codepoints: vec![cp('a' as u32, false); 4],
        };
        assert_eq!(
            p.range(8, 9),
            PreeditRange {
                start: 7,
                end: 9,
                cp_offset: 1,
            }
        );
    }

    #[test]
    fn preedit_width() {
        let p = Preedit {
            codepoints: vec![
                cp('a' as u32, false),
                cp(HANGUL_GA, true),
                cp('b' as u32, false),
            ],
        };
        assert_eq!(p.width(), 4);
    }

    #[test]
    fn state_defaults_to_no_preedit_and_empty_mouse() {
        let state = State::new();

        assert_eq!(state.preedit, None);
        assert_eq!(state.mouse.point, None);
        assert_eq!(state.mouse.mods, Mods::new());
    }

    #[test]
    fn state_sets_and_clears_preedit() {
        let mut state = State::new();
        let preedit = Preedit {
            codepoints: vec![cp('x' as u32, false)],
        };

        state.set_preedit(preedit.clone());
        assert_eq!(state.preedit.as_ref(), Some(&preedit));

        state.clear_preedit();
        assert_eq!(state.preedit, None);
    }

    #[test]
    fn state_clone_owns_preedit_codepoints() {
        let mut state = State::new();
        state.set_preedit(Preedit {
            codepoints: vec![cp('a' as u32, false)],
        });

        let mut cloned = state.clone();
        cloned
            .preedit
            .as_mut()
            .unwrap()
            .codepoints
            .push(cp('b' as u32, false));

        assert_eq!(state.preedit.as_ref().unwrap().codepoints.len(), 1);
        assert_eq!(cloned.preedit.as_ref().unwrap().codepoints.len(), 2);
    }

    #[test]
    fn state_updates_mouse_point_and_mods() {
        let mut state = State::new();
        let point = Coordinate::new(4, 9);
        let mut mods = Mods::for_mod(Mod::Super, Side::Right);
        mods.caps_lock = true;
        mods.num_lock = true;

        state.set_mouse_point(point);
        state.set_mouse_mods(mods);

        assert_eq!(state.mouse.point, Some(point));
        assert_eq!(state.mouse.mods, mods);
        assert!(state.mouse.mods.super_);
        assert!(state.mouse.mods.caps_lock);
        assert!(state.mouse.mods.num_lock);
        assert_eq!(state.mouse.mods.sides.super_, Side::Right);

        state.clear_mouse_point();
        assert_eq!(state.mouse.point, None);
        assert_eq!(state.mouse.mods, mods);
    }
}
