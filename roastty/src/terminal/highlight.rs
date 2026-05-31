//! Highlights are contiguous ranges of cells that should be called out,
//! most commonly for selection, search results, or semantic terminal regions.
//!
//! Within the terminal package, a highlight is a generic range over cells.

use std::ptr::NonNull;

use super::page_list::{Node, Pin};
use super::size::CellCountInt;

/// An untracked highlight stores its highlighted area as start and end screen
/// pins. Since it is untracked, the pins are only valid for the current
/// terminal state and may not be safe after terminal mutations.
///
/// To simplify operations, `start` must be before or equal to `end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Untracked {
    pub(super) start: Pin,
    pub(super) end: Pin,
}

/// A tracked highlight stores its highlighted area as tracked screen pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Tracked {
    pub(super) start: NonNull<Pin>,
    pub(super) end: NonNull<Pin>,
}

impl Tracked {
    pub(super) fn init_assume(start: NonNull<Pin>, end: NonNull<Pin>) -> Self {
        Self { start, end }
    }
}

/// A flattened highlight stores the highlighted area as serial-stamped page
/// chunks so callers can traverse a highlight without re-reading page-list
/// bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Flattened {
    pub(super) chunks: Vec<Chunk>,
    pub(super) top_x: CellCountInt,
    pub(super) bot_x: CellCountInt,
}

/// A flattened page chunk plus the page serial observed when the highlight was
/// created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Chunk {
    pub(super) node: NonNull<Node>,
    pub(super) serial: u64,
    pub(super) start: CellCountInt,
    pub(super) end: CellCountInt,
}

impl Default for Flattened {
    fn default() -> Self {
        Self {
            chunks: Vec::new(),
            top_x: 0,
            bot_x: 0,
        }
    }
}

impl Flattened {
    const EMPTY_PRECONDITION: &'static str = "flattened highlight must contain at least one chunk";

    pub(super) fn empty() -> Self {
        Self::default()
    }

    pub(super) fn start_pin(&self) -> Pin {
        let chunk = self.chunks.first().expect(Self::EMPTY_PRECONDITION);
        Pin::new(chunk.node, chunk.start, self.top_x)
    }

    pub(super) fn end_pin(&self) -> Pin {
        let chunk = self.chunks.last().expect(Self::EMPTY_PRECONDITION);
        Pin::new(chunk.node, chunk.end - 1, self.bot_x)
    }

    pub(super) fn untracked(&self) -> Untracked {
        Untracked {
            start: self.start_pin(),
            end: self.end_pin(),
        }
    }
}
