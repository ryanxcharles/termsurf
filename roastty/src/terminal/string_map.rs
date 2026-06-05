//! A flattened screen-text string with a per-byte map back to screen `Pin`s, plus regex search over
//! it (port of upstream `terminal/StringMap`). Uses the `regex` crate's byte engine in place of
//! oniguruma — the Rust regex engine is linear-time, so upstream's retry-budget machinery drops.

use regex::bytes::Regex;

use super::page_list::Pin;
use super::selection::Selection;

/// A flattened string and the screen `Pin` for each of its bytes (upstream `StringMap`).
pub(in crate::terminal) struct StringMap {
    string: Vec<u8>,
    /// One pin per byte of `string` (`map.len() == string.len()`).
    map: Vec<Pin>,
}

impl StringMap {
    pub(in crate::terminal) fn new(string: Vec<u8>, map: Vec<Pin>) -> StringMap {
        // Hard assert (not `debug_assert`) so a violation fails at construction, not later indexing.
        assert_eq!(string.len(), map.len(), "one pin per byte");
        StringMap { string, map }
    }

    /// Iterate the non-overlapping regex matches of the string (upstream `searchIterator`). The
    /// caller compiles the pattern (`regex::bytes::Regex::new`).
    pub(in crate::terminal) fn search_iterator<'a>(
        &'a self,
        regex: &'a Regex,
    ) -> SearchIterator<'a> {
        SearchIterator {
            map: self,
            regex,
            offset: 0,
        }
    }
}

/// Iterates the non-overlapping regex matches of a `StringMap` (upstream `SearchIterator`).
pub(in crate::terminal) struct SearchIterator<'a> {
    map: &'a StringMap,
    regex: &'a Regex,
    offset: usize,
}

impl Iterator for SearchIterator<'_> {
    type Item = Match;

    fn next(&mut self) -> Option<Match> {
        loop {
            if self.offset > self.map.string.len() {
                return None;
            }
            // `find_at` searches the full string from `offset`, returning absolute byte offsets.
            let m = self.regex.find_at(&self.map.string, self.offset)?;
            let (s, e) = (m.start(), m.end());
            if e > s {
                self.offset = e; // advance past the match (non-overlapping)
                return Some(Match {
                    start: self.map.map[s],
                    end: self.map.map[e - 1],
                });
            }
            // An empty match: advance one byte so the search makes progress (oniguruma's URL
            // patterns never match empty, but the `regex` crate can).
            self.offset = e + 1;
        }
    }
}

/// A single regex match, resolved to its start/end screen pins (upstream `Match`).
pub(in crate::terminal) struct Match {
    start: Pin,
    end: Pin,
}

impl Match {
    /// The selection spanning the full match (upstream `Match.selection`).
    pub(in crate::terminal) fn selection(&self) -> Selection {
        Selection::new(self.start, self.end, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::point;
    use crate::terminal::screen::Screen;
    use crate::terminal::size::CellCountInt;

    /// The pin for cell `i` on row 0 of `screen`.
    fn cell_pin(screen: &Screen, i: usize) -> Pin {
        screen
            .pages()
            .pin(point::Point::active(point::Coordinate::new(
                i as CellCountInt,
                0,
            )))
            .expect("cell pin")
    }

    /// A 40-col screen with `text` on row 0; the pins map one byte per cell.
    fn screen_with(text: &str) -> Screen {
        let mut screen = Screen::init(40, 5, None).unwrap();
        screen.set_text_lines_for_tests(&[text]);
        screen
    }

    /// A `StringMap` whose bytes are `text` and whose pins are `screen`'s row-0 cells.
    fn build_map(screen: &Screen, text: &str) -> StringMap {
        let bytes = text.as_bytes();
        let map: Vec<Pin> = (0..bytes.len()).map(|i| cell_pin(screen, i)).collect();
        StringMap::new(bytes.to_vec(), map)
    }

    #[test]
    fn search_iterator_finds_a_simple_match() {
        let text = "1ABCD2EFGH";
        let screen = screen_with(text);
        let sm = build_map(&screen, text);
        let re = Regex::new("[A-B]{2}").unwrap();
        let matches: Vec<Match> = sm.search_iterator(&re).collect();
        assert_eq!(matches.len(), 1);
        let sel = matches[0].selection();
        // "AB" is bytes 1..3 → the `A` (cell 1) and `B` (cell 2).
        assert_eq!(sel.start(), cell_pin(&screen, 1));
        assert_eq!(sel.end(), cell_pin(&screen, 2));
    }

    #[test]
    fn search_iterator_finds_multiple_and_advances() {
        let text = "ABxxAB";
        let screen = screen_with(text);
        let sm = build_map(&screen, text);
        let re = Regex::new("AB").unwrap();
        let starts: Vec<Pin> = sm
            .search_iterator(&re)
            .map(|m| m.selection().start())
            .collect();
        assert_eq!(starts.len(), 2);
        assert_eq!(starts[0], cell_pin(&screen, 0));
        assert_eq!(starts[1], cell_pin(&screen, 4));
    }

    #[test]
    fn search_iterator_no_match_is_empty() {
        let text = "12345";
        let screen = screen_with(text);
        let sm = build_map(&screen, text);
        let re = Regex::new("[A-B]{2}").unwrap();
        assert_eq!(sm.search_iterator(&re).count(), 0);
    }

    #[test]
    fn search_iterator_url_like() {
        let text = "go https://x.y z";
        let screen = screen_with(text);
        let sm = build_map(&screen, text);
        let re = Regex::new(r"https?://\S+").unwrap();
        let matches: Vec<Match> = sm.search_iterator(&re).collect();
        assert_eq!(matches.len(), 1);
        // "https://x.y" is bytes 3..=13 (after "go ", up to the last non-space byte before " z").
        assert_eq!(matches[0].selection().start(), cell_pin(&screen, 3));
        assert_eq!(matches[0].selection().end(), cell_pin(&screen, 13));
    }

    #[test]
    fn search_iterator_empty_match_terminates() {
        let text = "xyz";
        let screen = screen_with(text);
        let sm = build_map(&screen, text);
        // `a*` matches the empty string everywhere; the guard must make the iterator terminate with
        // no (invalid) selections rather than loop forever.
        let re = Regex::new("a*").unwrap();
        assert_eq!(sm.search_iterator(&re).count(), 0);
    }
}
