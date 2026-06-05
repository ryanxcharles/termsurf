+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 616: terminal StringMap + the regex crate

## Description

Per the chosen direction (resolve a blocked area by **adding a dependency
crate**), this experiment unblocks the regex/oniguruma area by adding the Rust
`regex` crate and porting `terminal/StringMap.zig` — the terminal-core
"regex-search the screen" primitive used for link detection and regex selection.

`StringMap` is a flattened screen-text string plus a per-byte mapping back to
screen `Pin`s; its `searchIterator(regex)` yields each regex match as a
`Selection` over the matched pins. Upstream compiles the pattern with oniguruma;
roastty uses `regex::bytes::Regex` (`regex` 1.x — already vendored in the
workspace by `wezboard`).

This is the first slice of the regex-area work: the `StringMap` struct +
`search_iterator` + `Match::selection`. The screen-side producer (building a
`StringMap` from a selection) already exists in roastty as
`PageStringWithPinMap`; wiring `StringMap` to consume it, and the `input/Link` /
`config/url` URL-detection regex, are follow-ups (the URL pattern may need
`regex`-crate-compatible syntax, a separate concern).

## Upstream behavior (`StringMap.zig`)

```zig
string: [:0]const u8,   // flattened, NUL-terminated screen text
map: []Pin,             // one Pin per byte of `string`

fn searchIterator(self, regex: oni.Regex) SearchIterator { .{ .map=self, .regex=regex } }

const SearchIterator = struct {
    map: StringMap, regex: oni.Regex, offset: usize = 0,
    fn next(self) !?Match {
        if (offset >= string.len) return null;
        region = regex.searchWithParam(string[offset..], ...) catch return null; // mismatch/budget → done
        const end_idx = region.ends()[0];
        defer self.offset += end_idx;          // advance past this match
        return .{ .map=map, .offset=offset, .region=region };
    }
};

const Match = struct {
    map, offset, region,
    fn selection(self) Selection {
        const s = region.starts()[0]; const e = region.ends()[0] - 1;
        return .init(map.map[offset + s], map.map[offset + e], false);
    }
};
```

## Rust mapping (`roastty/src/terminal/string_map.rs`, new file)

`regex::bytes::Regex` operates on `&[u8]` (the screen text is bytes; no NUL
needed). `find_at(haystack, at)` finds the leftmost match starting at/after
`at`, returning absolute byte offsets — so the offset bookkeeping uses absolute
indices (simpler than upstream's slice-relative `region` + `offset` addition).
`Pin` is `Copy`; `Match` holds the resolved start/end pins, so it carries no
borrow.

```rust
//! A flattened screen-text string with a per-byte map back to screen `Pin`s, plus regex search over
//! it (port of upstream `terminal/StringMap`). Uses the `regex` crate in place of oniguruma.

use regex::bytes::Regex;
use super::page_list::Pin;
use super::selection::Selection;

/// A flattened string and the screen `Pin` for each of its bytes (upstream `StringMap`).
pub(in crate::terminal) struct StringMap {
    string: Vec<u8>,
    map: Vec<Pin>, // `map.len() == string.len()`: one pin per byte
}

impl StringMap {
    pub(in crate::terminal) fn new(string: Vec<u8>, map: Vec<Pin>) -> StringMap {
        // Hard assert (not `debug_assert`) so a violation fails at construction, not later indexing.
        assert_eq!(string.len(), map.len(), "one pin per byte");
        StringMap { string, map }
    }

    /// Iterate the regex matches of the string (upstream `searchIterator`).
    pub(in crate::terminal) fn search_iterator<'a>(&'a self, regex: &'a Regex) -> SearchIterator<'a> {
        SearchIterator { map: self, regex, offset: 0 }
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
            let m = self.regex.find_at(&self.map.string, self.offset)?; // no match → done
            let (s, e) = (m.start(), m.end());
            if e > s {
                self.offset = e; // advance past the match (non-overlapping)
                return Some(Match {
                    start: self.map.map[s],
                    end: self.map.map[e - 1],
                });
            }
            // Empty match: advance one byte to make progress, then keep searching.
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
```

Registered in `terminal/mod.rs` as `#[allow(dead_code)] mod string_map;`.

Cargo: add `regex = "1"` to `roastty/Cargo.toml` `[dependencies]` (already in
the workspace lock via `wezboard`).

### Notes / deviations

- **oniguruma → `regex::bytes::Regex`** (the chosen crate). The caller compiles
  the pattern (`Regex::new(pattern)`) and passes `&Regex` to `search_iterator`.
  The oniguruma retry-budget / `MatchParam` machinery has no `regex`-crate
  analog (the Rust regex engine is linear-time, so no backtracking-budget is
  needed); it drops.
- **Absolute offsets**: `find_at` returns absolute byte offsets, so `Match` uses
  `map[s]` / `map[e-1]` directly (vs upstream's `map[offset + …]`). The
  empty-match guard (advance one byte) avoids an infinite loop — oniguruma's URL
  patterns never match empty, but the `regex` crate can.
- **`map.len() == string.len()`** (one pin per byte) is the StringMap invariant,
  matching upstream's `[]Pin` per byte.
- The screen-side producer (`StringMap::from` a `PageStringWithPinMap`) and the
  URL-detection regex (`config/url`) are follow-ups; tests here build a
  `StringMap` directly from a test screen's per-cell pins.

## Verification

- `cargo build -p roastty` — no warnings; `regex` resolves.
- `cargo test -p roastty` — no regressions; new tests (build a `StringMap` from
  a test `Screen`'s pins — one ASCII cell per byte):
  - `search_iterator_finds_a_simple_match` — text `"1ABCD2EFGH"`, regex
    `[A-B]{2}` → one match `"AB"`; `Match::selection()` spans the pins for bytes
    1–2 (the `A` and `B` cells).
  - `search_iterator_finds_multiple_and_advances` — text with two matches; the
    iterator yields both, non-overlapping, then `None`.
  - `search_iterator_no_match_is_empty` — a pattern with no match yields `None`.
  - `search_iterator_url_like` — a simple `https?://\S+` over
    `"go https://x.y z"` selects the URL bytes.
  - `search_iterator_empty_match_terminates` — a can-match-empty pattern (`a*`)
    over non-empty text terminates (the empty-match guard) and yields no invalid
    selection.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on touched source — clean.
- `git diff --check` — clean.

Pass = `StringMap::search_iterator` yields each regex match (via the `regex`
crate) as a `Selection` over the correct screen pins, non-overlapping, with the
`regex` crate added as a roastty dependency.

## Design Review

Codex reviewed the design and **APPROVED** it with **no Required findings**,
confirming the mapping is faithful: `regex::bytes::Regex::find_at` gives
absolute byte offsets so `map[start]` / `map[end-1]` equals upstream's
`map[offset + …]`; advancing `offset = match.end()` preserves non-overlapping
iteration; the empty-match guard is the right Rust addition (the `regex` crate
can match empty where oniguruma's URL patterns don't); dropping the oniguruma
retry-budget is sound with the Rust engine's linear-time guarantee; `Match`
holding resolved `Pin`s (Copy) is a clean simplification; and `regex::bytes`
(not `fancy-regex`) is the right dependency for flattened terminal bytes. Both
Optionals adopted:

- **Optional (adopted)**: `StringMap::new` uses `assert_eq!` (not
  `debug_assert_eq!`) for the one-pin-per-byte invariant, failing at
  construction.
- **Optional (adopted)**: an empty-match test (`a*` over non-empty text
  terminates with no invalid selection).

Review artifacts:

- Prompt: `logs/codex-review/20260605-d616-prompt.md`
- Result: `logs/codex-review/20260605-d616-last-message.md`

## Result

**Result:** Pass

Added `regex = "1"` to `roastty/Cargo.toml` and implemented
`roastty/src/terminal/string_map.rs`, porting upstream `terminal/StringMap`:
`StringMap { string: Vec<u8>, map: Vec<Pin> }` (one pin per byte, hard
`assert_eq!`), `search_iterator(&Regex)` over `regex::bytes::Regex`, the
`SearchIterator` (`find_at` absolute offsets → advance `offset = match.end()`;
empty-match guard), and `Match::selection()`
(`Selection::new(start, end, false)`). The oniguruma retry-budget machinery
drops (the Rust engine is linear-time). The screen-side producer
(`PageStringWithPinMap`) wiring and the URL-detection regex (`config/url`)
remain follow-ups.

Five tests build a `StringMap` from one screen's per-cell pins and compare match
selections against pins from the same screen (so node-identity comparisons are
meaningful): a simple `[A-B]{2}` match, two non-overlapping matches, a no-match
case, a `https?://\S+` URL-like match, and the empty-match-terminates guard.
Gates: `cargo fmt --check` clean, `cargo build -p roastty` no warnings (`regex`
resolves), `cargo test -p roastty` **3390 passed / 0 failed** (3385 → 3390, +5),
no-ghostty grep clean, `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **APPROVED** it with **no Required,
Optional, or Nit findings**, confirming the port is faithful: the
one-byte-per-pin invariant (hard `assert_eq!`); `find_at` absolute offsets
correctly replacing the slice-relative region; non-overlapping advancement;
`Match` resolving to copied `Pin`s; `Match::selection()` building an untracked
non-rectangle `Selection`; the empty-match guard (tested); and the sound
dropping of the oniguruma retry budget. The test approach (all pins from the
same screen) and the minimal dependency addition are confirmed sound.

Review artifacts:

- Prompt: `logs/codex-review/20260605-d616-prompt.md` / `…-r616-prompt.md`
- Result: `logs/codex-review/20260605-d616-last-message.md` /
  `…-r616-last-message.md`

## Conclusion

The `regex` crate is now a roastty dependency and the terminal-core `StringMap`
regex primitive is ported — the first slice of the regex-area work (resolving
the oniguruma block by depending, per the chosen direction). The natural
follow-ups: wire `StringMap` to the existing `PageStringWithPinMap` producer
(build a map from a real screen selection), and port `config/url`'s
URL-detection regex + `input/Link`'s matching (the URL pattern may need
`regex`-crate-compatible syntax — a `fancy-regex` fallback is also available in
the workspace if look-around is required). Issue 801 stays open and broad.
