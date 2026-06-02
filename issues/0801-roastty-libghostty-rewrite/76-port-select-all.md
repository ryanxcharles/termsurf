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

# Experiment 76: Port Select All

## Description

Port upstream `Screen.selectAll` into Roastty's current PageList-centered
terminal model.

Experiment 75 added line selection. Upstream `selectAll` is the next selection
primitive in `vendor/ghostty/src/terminal/Screen.zig`, and it is small enough to
port before the larger `selectionString`, `ScreenFormatter`, `LineIterator`, or
gesture-state layers. The helper should select all written screen content while
trimming surrounding blank, space, and tab cells, matching upstream behavior.

This experiment should add only the PageList-local select-all calculation and
tests. It must not add `Screen`, `Terminal`, `ScreenFormatter`,
`selectionString`, string-map support, `LineIterator`, selection gesture state,
public ABI, renderer, parser, app, platform input, mouse event behavior,
clipboard behavior, or UI wiring.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/Screen.zig` for:
     - `selectAll`;
     - `test "Screen: selectAll"`.
   - Use existing Roastty code:
     - `roastty/src/terminal/page_list.rs`;
     - `roastty/src/terminal/selection.rs`;
     - existing PageList cell iterator, screen-point, active/screen coordinate,
       and text fixture helpers.
   - Do not modify `vendor/ghostty/`.

2. Add a private PageList helper.
   - Preferred shape:

     ```rust
     fn select_all(&self) -> Option<selection::Selection>
     ```

   - Match upstream trimming behavior:
     - scan forward from the top-left screen cell to find the first cell with
       text whose codepoint is not `0`, space, or tab;
     - scan backward from the bottom-right screen cell to find the last cell
       with text whose codepoint is not `0`, space, or tab;
     - return `None` if no such cell exists;
     - return an untracked non-rectangular selection from the first
       non-whitespace text cell through the last non-whitespace text cell.
   - Use the PageList `Screen` coordinate domain, not only the active viewport.
     If history/scrollback rows exist, select-all should include written
     scrollback rows because upstream starts from `.screen = .{}` and scans the
     full screen-backed PageList.
   - Keep the whitespace table local to this helper unless an existing constant
     already exactly matches upstream's `[0, ' ', '\t']` table. Do not reuse the
     line-whitespace table unless it is proven identical for this specific
     upstream helper.

3. Add upstream-equivalent tests.
   - Port the two upstream `Screen: selectAll` cases:
     - `ABC  DEF`, ` 123`, `456` selects `(0, 0)..(2, 2)`;
     - a later write with leading blank/space rows and trailing content selects
       the same trimmed span as upstream.
   - Add Roastty-specific coverage:
     - an empty PageList returns `None`;
     - rows containing only spaces, tabs, or unwritten cells return `None`;
     - leading and trailing spaces/tabs are trimmed, but internal spaces are
       preserved by the selected span;
     - select-all uses screen coordinates across scrollback, not just active
       coordinates;
     - returned selections are untracked and non-rectangular.
   - Reuse existing local test helpers. Add only small helpers if they make the
     select-all tests clearer.

4. Keep scope narrow.
   - Do not add selection string extraction. This experiment proves the selected
     `Pin` bounds, not copied text.
   - Do not add public API or C ABI exposure.
   - Do not add gesture, keyboard shortcut, clipboard, app, terminal, or UI
     behavior.

5. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty select_all
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

6. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real findings before proceeding.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - helper name and location;
     - upstream test coverage;
     - Roastty-specific edge tests;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private PageList helper equivalent to upstream `selectAll`;
- the selected start/end pins match upstream's trimmed first and last
  non-whitespace text cells;
- empty, unwritten, and all-whitespace screen content returns `None`;
- screen-coordinate selection includes scrollback/history rows when present;
- returned selections are untracked and non-rectangular;
- no `Screen`, `Terminal`, `ScreenFormatter`, `selectionString`, string-map
  support, `LineIterator`, selection gesture state, public ABI, renderer,
  parser, app, platform input, mouse event behavior, clipboard behavior, or UI
  wiring is added;
- `cargo fmt`, targeted select-all tests, PageList tests, and full
  `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- active-screen select-all works, but the current PageList test fixtures expose
  an unimplemented screen-vs-active coordinate behavior that must be split into
  its own prerequisite experiment.

The experiment fails if:

- select-all cannot be implemented without adding Screen, Terminal, formatter,
  ABI, renderer, parser, app, or platform input behavior;
- selection starts from the active viewport instead of the screen domain;
- all-whitespace content is selected instead of returning `None`;
- surrounding whitespace is not trimmed;
- tests or formatting fail.

## Design Review

Codex reviewed the design and found no blockers. It approved the scope, upstream
`Screen.selectAll` semantic match, screen-domain/scrollback distinction,
verification plan, and result-recording requirements as good enough to commit
before implementation.

## Result

**Result:** Pass

Implemented private PageList select-all calculation in
`roastty/src/terminal/page_list.rs`:

- `SELECT_ALL_WHITESPACE` stores upstream's exact select-all trimming table:
  `0`, space, and tab.
- `PageList::select_all()` scans forward from the top-left screen cell for the
  first non-whitespace text cell, scans backward from the bottom-right screen
  cell for the last non-whitespace text cell, and returns an untracked
  non-rectangular `Selection`.
- The helper returns `None` for empty, unwritten, or all-whitespace screen
  content.

The implementation stays private to PageList. It does not add `Screen`,
`Terminal`, `ScreenFormatter`, `selectionString`, string-map support,
`LineIterator`, gesture state, public ABI, renderer, parser, app, platform
input, mouse behavior, clipboard behavior, or UI wiring.

The tests cover:

- both upstream `Screen: selectAll` cases;
- empty PageLists;
- all-space, all-tab, and unwritten content returning `None`;
- trimming leading/trailing spaces and tabs while preserving the internal
  selected span;
- selecting across the full screen coordinate domain when scrollback/history
  rows exist;
- untracked non-rectangular selection shape.

Verification passed:

```bash
cargo fmt
cargo test -p roastty select_all
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Observed results:

- `cargo test -p roastty select_all`: 4 passed.
- `cargo test -p roastty terminal::page_list`: 373 passed.
- `cargo test -p roastty`: 666 unit tests passed, ABI harness passed, and
  doctests passed.

Codex reviewed the implementation and found no implementation blockers. Its only
finding was to record this result and update the README status.

## Conclusion

Experiment 76 successfully ports upstream `selectAll` into Roastty's
PageList-centered terminal layer. Roastty can now compute a private
screen-domain select-all range with upstream trimming behavior and scrollback
coverage, without adding string extraction, formatter, gesture, or public API
surface.

The next experiment can continue the selection stack with the next primitive
that depends on the completed selection helpers.
