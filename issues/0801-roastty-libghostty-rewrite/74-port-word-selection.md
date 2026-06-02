+++
[implementer]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 74: Port Word Selection

## Description

Port upstream word-selection helpers from `Screen.zig` into Roastty's current
PageList-centered terminal model.

Experiment 73 added cell-granular drag selection. Upstream word-drag behavior
depends on `Screen.selectWord` and `Screen.selectWordBetween`, but Roastty does
not have `Screen` yet. The already-ported selection, PageList, pin navigation,
cell iterator, and default selection codepoint tables are enough to port the
pure word-boundary selection logic now.

This experiment should add only word-selection helpers and tests. It must not
add full `SelectionGesture`, press/release state, line selection, output
selection, Screen, Terminal, public C ABI, renderer, parser, app, platform
input, autoscroll, deep-press, or mouse event behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/Screen.zig` for:
     - `selectWord`;
     - `selectWordBetween`;
     - `test "Screen: selectWord"`;
     - `test "Screen: selectWord across soft-wrap"`;
     - `test "Screen: selectWord whitespace across soft-wrap"`;
     - `test "Screen: selectWord with character boundary"`.
   - Use existing Roastty code:
     - `roastty/src/terminal/page_list.rs`;
     - `roastty/src/terminal/selection.rs`;
     - `roastty/src/terminal/selection_codepoints.rs`.
   - Do not modify `vendor/ghostty/`.

2. Add PageList-local word-selection helpers.
   - Add private methods on `PageList`, likely near the existing selection
     helpers:

     ```rust
     fn select_word(
         &self,
         pin: Pin,
         boundary_codepoints: &[u32],
     ) -> Option<selection::Selection>

     fn select_word_between(
         &self,
         start: Pin,
         end: Pin,
         boundary_codepoints: &[u32],
     ) -> Option<selection::Selection>
     ```

   - Keep both helpers private to the terminal module for now. Do not expose
     them through public API or C ABI.
   - Reject invalid or garbage pins with `None`.
   - Return `None` when the starting cell has no text.
   - Use `Cell::codepoint()` and `Cell::has_text()` for the same boundary
     classification upstream uses.
   - Treat any cell whose codepoint is in `boundary_codepoints` as a boundary
     character. Boundary runs are selected as their own words, matching upstream
     behavior for spaces and punctuation.
   - Scan right/down to find the end of the current word.
   - Scan left/up to find the start of the current word.
   - Stop cross-row word selection at hard row boundaries. A word may continue
     across a row only when the previous row is soft-wrapped, matching upstream
     `row.wrap` behavior.
   - Return an untracked `Selection::new(start_pin, end_pin, false)` on success.

3. Preserve `selectWordBetween` semantics.
   - Determine direction with the existing PageList pin-ordering helper: forward
     when `start` is before `end`, otherwise backward.
   - Iterate from `start` toward `end`, inclusive.
   - Return the first non-null `select_word` result encountered.
   - Stop and return `None` once the iterator moves past `end`.
   - Preserve upstream's "nearest to start" behavior. Do not normalize
     `start`/`end` before iterating.

4. Add upstream-equivalent tests.
   - Port the upstream `selectWord` cases using PageList fixtures:
     - selecting the same word when clicked at the start, middle, and end;
     - selecting boundary whitespace runs;
     - selecting single-character whitespace runs;
     - selecting at the end of written screen content;
     - returning `None` for empty/unwritten cells.
   - Port soft-wrap cases:
     - non-boundary word selection across a soft-wrapped row;
     - boundary whitespace selection across a soft-wrapped row;
     - hard row boundaries stopping selection even when adjacent cells are
       non-boundary text.
   - Port character-boundary cases for every upstream default boundary
     character:
     - clicking inside `abc` selects only `abc`;
     - clicking on the boundary character selects the surrounding boundary run,
       preserving upstream's current punctuation behavior.
   - Add focused `select_word_between` tests:
     - forward scan returns the first selectable word nearest `start`;
     - backward scan returns the first selectable word nearest `start`;
     - equal `start` and `end` still checks that one inclusive cell and returns
       the word under `start` when selectable;
     - scans over empty cells;
     - returns `None` when no selectable word exists before the inclusive end;
     - invalid or garbage start/end pins return `None`.
   - Add focused `select_word` tests proving invalid and garbage pins return
     `None`.

5. Keep scope narrow.
   - Do not add line selection, output selection, semantic prompt boundaries,
     `SelectionGesture.dragSelectionWord`, or `SelectionGesture.pressSelection`
     yet.
   - Do not add full write/parser support merely to build the tests. Reuse the
     existing PageList test helpers such as `set_screen_text_lines` and add
     small row-wrap test helpers if needed.
   - Do not add public API or ABI exposure.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty select_word
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

7. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real findings before proceeding.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - helper names and location;
     - upstream test coverage;
     - Roastty-specific edge tests;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has private PageList helpers equivalent to upstream `selectWord` and
  `selectWordBetween`;
- word selection matches upstream behavior for ordinary text, whitespace,
  punctuation boundaries, empty cells, and screen end cases;
- word selection crosses soft-wrapped rows but stops at hard row boundaries;
- `select_word_between` returns the nearest selectable word in scan direction
  and treats `end` as inclusive;
- invalid or garbage pins return `None`;
- all new selections are untracked non-rectangular selections;
- no full `SelectionGesture`, press/release state, line selection, output
  selection, Screen, Terminal, public ABI, renderer, parser, app, platform
  input, autoscroll, deep-press, or mouse event behavior is added;
- `cargo fmt`, targeted word-selection tests, PageList tests, and full
  `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- `select_word` matches upstream but `select_word_between` exposes a dependency
  on future gesture or Screen state that should be split into the next
  experiment.

The experiment fails if:

- word selection cannot be implemented without adding Screen, Terminal, parser,
  ABI, renderer, app, or platform input behavior;
- the helper treats punctuation boundaries differently from upstream without a
  documented reason;
- soft-wrapped rows do not behave differently from hard row boundaries;
- empty cells or invalid pins panic instead of returning `None`;
- tests or formatting fail.

## Design Review

Codex reviewed the initial design and requested two test-plan additions before
implementation:

- focused `select_word` tests for invalid and garbage pins returning `None`;
- a `select_word_between(start, start, ...)` test proving equal start/end still
  checks the inclusive cell and returns the word under `start` when selectable.

Those requirements are incorporated above. Follow-up Codex review approved the
updated design for implementation with no remaining blockers.

## Result

**Result:** Pass

Roastty now has private PageList-local word-selection helpers in
`roastty/src/terminal/page_list.rs`:

- `PageList::select_word`
- `PageList::select_word_between`

Both helpers return untracked, non-rectangular `selection::Selection` values and
remain private to the terminal implementation. No full `SelectionGesture`,
press/release state, line selection, output selection, Screen, Terminal, public
C ABI, renderer, parser, app, platform input, autoscroll, deep-press, or mouse
event behavior was added.

Coverage added:

- upstream `selectWord` basic cases for start/middle/end clicks inside a word;
- upstream whitespace and single-whitespace boundary-run cases;
- empty/unwritten cell cases returning `None`;
- end-of-written-screen content selection;
- soft-wrapped non-boundary word selection across rows;
- soft-wrapped whitespace boundary-run selection across rows;
- hard row-boundary stops;
- all upstream default punctuation boundary characters, including the current
  upstream behavior where clicking the boundary character selects the
  surrounding boundary run;
- focused `select_word` invalid and garbage pin checks;
- focused `select_word_between` forward, backward, equal start/end inclusive,
  empty-cell scan, no-match, invalid-pin, and garbage-pin checks.

Verification passed:

```bash
cargo fmt
cargo test -p roastty select_word
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

The targeted word-selection run passed 8 tests. The PageList run passed 354
tests. The full Roastty run passed 647 unit tests, the ABI harness, and
doctests.

Codex result review found no code blockers. Its only findings were that this
result/conclusion and the README status still needed to be recorded, which this
section and the README update complete.

## Conclusion

Experiment 74 successfully ports upstream word selection into Roastty's current
PageList model. Word selection can now classify boundary and non-boundary text,
cross soft-wrapped rows, stop at hard row boundaries, skip empty cells when
searching between two pins, and preserve upstream's nearest-to-start
`selectWordBetween` behavior.

The next experiment can continue the selection stack by porting the next
content-aware selection slice, most likely line selection, before higher-level
word/line drag gesture behavior is wired together.
