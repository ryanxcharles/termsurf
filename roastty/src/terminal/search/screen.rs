//! Searches a whole `Screen` (port of upstream `terminal/search/screen.zig`). A state machine over
//! `ActiveSearch` (the mutable active area) and `PageListSearch` (history) that caches results so a
//! background search survives screen changes. So far it lands the state-machine vocabulary and
//! struct skeleton, the read-only result accessors (`needle` / `matches_len` / `matches`), and the
//! `tick` state machine (`tick` / `tick_active` / `tick_history`), and `prune_history` (drop stale
//! cached history results); construction (`init` / `reload_active`), `feed`, and `select` are
//! deferred.

use std::ptr::NonNull;

use super::super::highlight::{Flattened, Tracked};
use super::super::page_list::Pin;
use super::super::screen::Screen;
use super::super::size::CellCountInt;
use super::active::ActiveSearch;
use super::pagelist::PageListSearch;

/// The search state machine's position (upstream `ScreenSearch.State`). Module-private, like
/// upstream's `State` (private to `ScreenSearch`); same-module tests exercise its predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Searching the active area.
    Active,
    /// Searching the history (scrollback) area.
    History,
    /// History search is waiting for more data to be fed before it can progress.
    HistoryFeed,
    /// Search is complete given the current terminal state.
    Complete,
}

impl State {
    /// Whether the search is complete (upstream `isComplete`).
    fn is_complete(self) -> bool {
        matches!(self, State::Complete)
    }

    /// Whether the search wants a `feed` (upstream `needsFeed`): `HistoryFeed`, or `Complete` (a
    /// complete search still prunes stale history results on the next feed).
    fn needs_feed(self) -> bool {
        matches!(self, State::HistoryFeed | State::Complete)
    }
}

/// The direction to step the selected match (upstream `ScreenSearch.Select`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::terminal) enum Select {
    /// Next selection, newest to oldest, non-wrapping.
    Next,
    /// Previous selection, oldest to newest, non-wrapping.
    Prev,
}

/// The outcome of a `tick` (upstream's `TickError` set, minus the OOM case which is infallible in
/// Rust). `Progressed` is upstream's `Ok(void)` "made progress".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::terminal) enum Tick {
    /// Progress was made; call `tick` again.
    Progressed,
    /// The search needs more data; the caller must `feed` (upstream `error.FeedRequired`).
    FeedRequired,
    /// The search is complete given the current screen state (upstream `error.SearchComplete`).
    Complete,
}

/// The currently-selected match (upstream `ScreenSearch.SelectedMatch`).
struct SelectedMatch {
    /// Index from the end of the match list (0 = most recent match).
    idx: usize,
    /// Tracked highlight so we can detect movement.
    highlight: Tracked,
}

/// History (scrollback) search state (upstream `ScreenSearch.HistorySearch`).
struct HistorySearch {
    /// The actual searcher.
    searcher: PageListSearch,
    /// The pin for the first node this searcher started from (to detect active-area growth over
    /// previously-searched history).
    start_pin: NonNull<Pin>,
}

/// Searches a needle within a whole `Screen`, caching results across screen changes (upstream
/// `ScreenSearch`).
pub(crate) struct ScreenSearch {
    /// The screen being searched (upstream `*Screen`).
    screen: NonNull<Screen>,
    /// The active-area search state.
    active: ActiveSearch,
    /// The history search state (`None` if there is no history yet).
    history: Option<HistorySearch>,
    /// The state machine's current position.
    state: State,
    /// The currently-selected match, if any.
    selected: Option<SelectedMatch>,
    /// History results (mostly immutable once found; reverse order, newest to oldest).
    history_results: Vec<Flattened>,
    /// Active-area results (may change on re-search; forward order).
    active_results: Vec<Flattened>,
    /// Screen dimensions; a change restarts the whole search.
    rows: CellCountInt,
    cols: CellCountInt,
}

impl ScreenSearch {
    /// The needle this search is using (upstream `needle`). The active window is always forward, so
    /// its stored bytes are the original needle.
    pub(in crate::terminal) fn needle(&self) -> &[u8] {
        self.active.needle()
    }

    /// The total number of matches found so far (upstream `matchesLen`).
    pub(in crate::terminal) fn matches_len(&self) -> usize {
        self.active_results.len() + self.history_results.len()
    }

    /// All matches, ordered newest-to-oldest (upstream `matches`): the active results (stored
    /// forward) reversed, then the history results (already newest-to-oldest) appended. Returns an
    /// owned `Vec` (Rust ownership replaces upstream's caller-frees slice).
    pub(in crate::terminal) fn matches(&self) -> Vec<Flattened> {
        let mut results =
            Vec::with_capacity(self.active_results.len() + self.history_results.len());
        results.extend(self.active_results.iter().rev().cloned());
        results.extend(self.history_results.iter().cloned());
        results
    }

    /// Make incremental progress on the search without accessing screen state (upstream `tick`).
    /// Returns whether progress was made, a feed is required, or the search is complete.
    pub(in crate::terminal) fn tick(&mut self) -> Tick {
        match self.state {
            State::Active => {
                self.tick_active();
                Tick::Progressed
            }
            State::History => {
                self.tick_history();
                Tick::Progressed
            }
            State::HistoryFeed => Tick::FeedRequired,
            State::Complete => Tick::Complete,
        }
    }

    /// Consume the entire active area into `active_results`, then move to history (upstream
    /// `tickActive`). The active area is small, so this drains it in one go.
    fn tick_active(&mut self) {
        while let Some(hl) = self.active.next() {
            self.active_results.push(hl);
        }
        self.state = State::History;
    }

    /// Consume the loaded history matches into `history_results` (deduping against the active area),
    /// then request a feed (upstream `tickHistory`). No history → complete.
    fn tick_history(&mut self) {
        let history = match &mut self.history {
            Some(h) => h,
            None => {
                self.state = State::Complete;
                return;
            }
        };

        while let Some(hl) = history.searcher.next() {
            // Skip matches whose first chunk is in the start node — that node overlaps the active
            // area, which is searched separately.
            // SAFETY: `start_pin` is a tracked pin in the (alive) screen's storage; the screen
            // outlives the search (the construction-time invariant).
            let start_node = unsafe { history.start_pin.as_ref() }.node();
            if hl.chunks[0].node == start_node {
                continue;
            }
            self.history_results.push(hl);
        }

        self.state = State::HistoryFeed;
    }

    /// Drop cached history results whose pages have been pruned from the scrollback (upstream
    /// `pruneHistory`). History results are stored newest-to-oldest, so the first result whose first
    /// chunk's serial is below the screen's minimum live page serial marks the boundary — it and
    /// everything older are truncated.
    fn prune_history(&mut self) {
        // SAFETY: the screen is alive (the construction-time invariant).
        let min = unsafe { self.screen.as_ref() }.page_serial_min();
        for i in 0..self.history_results.len() {
            let first_chunk_serial = self.history_results[i].chunks[0].serial;
            if first_chunk_serial < min {
                self.history_results.truncate(i);
                self.history_results.shrink_to_fit(); // mirror upstream's `shrinkAndFree`
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::highlight::Chunk;
    use super::super::super::page_list::PageList;
    use super::super::super::screen::Screen;
    use super::*;

    #[test]
    fn state_is_complete() {
        assert!(State::Complete.is_complete());
        assert!(!State::Active.is_complete());
        assert!(!State::History.is_complete());
        assert!(!State::HistoryFeed.is_complete());
    }

    #[test]
    fn state_needs_feed() {
        assert!(State::HistoryFeed.needs_feed());
        assert!(State::Complete.needs_feed());
        assert!(!State::Active.needs_feed());
        assert!(!State::History.needs_feed());
    }

    #[test]
    fn enums_are_copy_and_eq() {
        let s = State::History;
        let copy = s;
        assert_eq!(s, copy);
        assert_ne!(State::Active, State::History);

        let d = Select::Next;
        let copy = d;
        assert_eq!(d, copy);
        assert_ne!(Select::Next, Select::Prev);
    }

    /// A minimal `Flattened` distinguished by its `top_x`.
    fn flat(top_x: CellCountInt) -> Flattened {
        Flattened {
            chunks: Vec::new(),
            top_x,
            bot_x: 0,
        }
    }

    /// Build a `ScreenSearch` directly with populated result lists. The accessors never dereference
    /// `screen` (so it can be dangling) and there are no tracked pins (so dropping is safe).
    fn build(active: Vec<Flattened>, history: Vec<Flattened>) -> ScreenSearch {
        ScreenSearch {
            screen: NonNull::dangling(),
            active: ActiveSearch::new(b"foo"),
            history: None,
            state: State::Active,
            selected: None,
            history_results: history,
            active_results: active,
            rows: 10,
            cols: 10,
        }
    }

    #[test]
    fn needle_returns_the_active_needle() {
        let s = build(Vec::new(), Vec::new());
        assert_eq!(s.needle(), b"foo");
    }

    #[test]
    fn matches_len_sums_both_lists() {
        let s = build(vec![flat(1), flat(2)], vec![flat(10), flat(11)]);
        assert_eq!(s.matches_len(), 4);

        let empty = build(Vec::new(), Vec::new());
        assert_eq!(empty.matches_len(), 0);
    }

    #[test]
    fn matches_orders_newest_to_oldest() {
        // Active is stored forward (oldest-to-newest); history is newest-to-oldest.
        let s = build(vec![flat(1), flat(2)], vec![flat(10), flat(11)]);
        let order: Vec<CellCountInt> = s.matches().iter().map(|h| h.top_x).collect();
        // Reversed active, then history.
        assert_eq!(order, vec![2, 1, 10, 11]);

        let empty = build(Vec::new(), Vec::new());
        assert!(empty.matches().is_empty());
    }

    #[test]
    fn tick_drains_active_then_completes_with_no_history() {
        let mut list = PageList::init(10, 10, None).unwrap();
        list.set_screen_text_lines_for_tests(&["Fizz"]);
        let mut active = ActiveSearch::new(b"Fizz");
        // SAFETY: `list` outlives `s`; its pages are not mutated.
        unsafe { active.update(&list) };

        let mut s = ScreenSearch {
            screen: NonNull::dangling(),
            active,
            history: None,
            state: State::Active,
            selected: None,
            history_results: Vec::new(),
            active_results: Vec::new(),
            rows: 10,
            cols: 10,
        };

        // First tick drains the active area.
        assert_eq!(s.tick(), Tick::Progressed);
        assert_eq!(s.matches_len(), 1);
        // Second tick: no history -> complete.
        assert_eq!(s.tick(), Tick::Progressed);
        // Now complete.
        assert_eq!(s.tick(), Tick::Complete);
    }

    #[test]
    fn tick_in_history_feed_requests_a_feed() {
        let mut s = build(Vec::new(), Vec::new());
        s.state = State::HistoryFeed;
        assert_eq!(s.tick(), Tick::FeedRequired);
    }

    #[test]
    fn tick_when_complete_reports_complete() {
        let mut s = build(Vec::new(), Vec::new());
        s.state = State::Complete;
        assert_eq!(s.tick(), Tick::Complete);
    }

    /// A `Flattened` whose single chunk carries `serial` (its `node` is dangling — `prune_history`
    /// reads only the serial).
    fn flat_serial(serial: u64) -> Flattened {
        Flattened {
            chunks: vec![Chunk {
                node: NonNull::dangling(),
                serial,
                start: 0,
                end: 0,
            }],
            top_x: 0,
            bot_x: 0,
        }
    }

    /// Build a `ScreenSearch` over a real `screen` (so `prune_history` can read `page_serial_min`)
    /// with the given history results.
    fn build_over_screen(screen: &Screen, history: Vec<Flattened>) -> ScreenSearch {
        ScreenSearch {
            screen: NonNull::from(screen),
            active: ActiveSearch::new(b"x"),
            history: None,
            state: State::Active,
            selected: None,
            history_results: history,
            active_results: Vec::new(),
            rows: 10,
            cols: 10,
        }
    }

    fn history_serials(s: &ScreenSearch) -> Vec<u64> {
        s.history_results
            .iter()
            .map(|h| h.chunks[0].serial)
            .collect()
    }

    #[test]
    fn prune_history_drops_from_the_first_stale_result() {
        let mut screen = Screen::init(10, 10, None).unwrap();
        screen.set_page_serial_min_for_tests(4);

        // Newest-to-oldest serials [5, 4, 3, 2]; serial 3 < 4 marks the boundary.
        let mut s = build_over_screen(
            &screen,
            vec![
                flat_serial(5),
                flat_serial(4),
                flat_serial(3),
                flat_serial(2),
            ],
        );
        s.prune_history();
        assert_eq!(history_serials(&s), vec![5, 4]);
    }

    #[test]
    fn prune_history_keeps_all_live_results() {
        let mut screen = Screen::init(10, 10, None).unwrap();
        screen.set_page_serial_min_for_tests(0);

        let mut s = build_over_screen(&screen, vec![flat_serial(5), flat_serial(4)]);
        s.prune_history();
        assert_eq!(history_serials(&s), vec![5, 4]);
    }

    #[test]
    fn prune_history_on_empty_is_a_noop() {
        let screen = Screen::init(10, 10, None).unwrap();
        let mut s = build_over_screen(&screen, Vec::new());
        s.prune_history();
        assert!(s.history_results.is_empty());
    }
}
