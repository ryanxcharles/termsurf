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

# Experiment 67: Port Selection Ordering

## Description

Port the ordering and top-left/bottom-right normalization portion of upstream
`Selection.zig`.

Upstream `Selection.order`, `topLeft`, `bottomRight`, and `ordered` depend on
`Screen` because they must convert pins to screen coordinates through
`s.pages.pointFromPin(.screen, pin)`. Roastty does not have the upstream
`Screen` layer yet, but `PageList` already owns pin-to-screen coordinate
conversion. Therefore this experiment should add a PageList-owned Rust
equivalent:

- determine selection order;
- compute top-left and bottom-right pins;
- return an untracked selection in a requested order.

This experiment must not add containment, contained-row extraction, adjustment,
word selection, line selection, formatting, selection gestures, C ABI, Screen,
search, renderer, parser, app, or terminal mutation behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/Selection.zig` for:
     - `Order`;
     - `order`;
     - `topLeft`;
     - `bottomRight`;
     - `ordered`.
   - Use existing Roastty code:
     - `roastty/src/terminal/selection.rs`;
     - `roastty/src/terminal/page_list.rs::point_from_pin`;
     - existing PageList screen-point tests and split/cross-page tests.
   - Do not modify `vendor/ghostty/`.

2. Add selection order type.
   - In `roastty/src/terminal/selection.rs`, add a terminal-internal enum:

     ```rust
     pub(super) enum Order {
         Forward,
         Reverse,
         MirroredForward,
         MirroredReverse,
     }
     ```

   - Derive `Debug`, `Clone`, `Copy`, `PartialEq`, and `Eq`.
   - Do not expose `Order` through crate-public API or C ABI.

3. Add PageList-owned selection ordering helpers.
   - In `roastty/src/terminal/page_list.rs`, add private helpers such as:

     ```rust
     fn selection_order(&self, selection: selection::Selection) -> Option<selection::Order>
     fn selection_top_left(&self, selection: selection::Selection) -> Option<Pin>
     fn selection_bottom_right(&self, selection: selection::Selection) -> Option<Pin>
     fn selection_ordered(
         &self,
         selection: selection::Selection,
         desired: selection::Order,
     ) -> Option<selection::Selection>
     ```

   - Return `None` if either selection endpoint cannot be mapped to a screen
     point.
   - Use `point_from_pin(point::Tag::Screen, pin)` as the screen-coordinate
     source of truth.
   - Preserve upstream regular-selection order semantics:
     - start row before end row is `Forward`;
     - start row after end row is `Reverse`;
     - same row with `start.x <= end.x` is `Forward`;
     - same row with `start.x > end.x` is `Reverse`.
   - Preserve upstream rectangle-selection order semantics:
     - bottom-right to top-left is `Reverse`;
     - top-right to bottom-left is `MirroredForward`;
     - bottom-left to top-right is `MirroredReverse`;
     - all other ordered rectangle selections are `Forward`;
     - single-row, single-column, and single-cell rectangle edge cases match
       upstream.
   - `selection_top_left` and `selection_bottom_right` should mirror upstream
     pin construction:
     - `Forward`: start/end;
     - `Reverse`: end/start;
     - `MirroredForward`: top-left uses start's row and end's x; bottom-right
       uses end's row and start's x;
     - `MirroredReverse`: top-left uses end's row and start's x; bottom-right
       uses start's row and end's x.
   - `selection_ordered` should return a new untracked `Selection`:
     - first, if the selection is already in the desired order, return a new
       untracked selection with the original start/end and rectangle state,
       matching upstream's early return;
     - `Forward`: top-left to bottom-right;
     - `Reverse`: bottom-right to top-left;
     - any mirrored desired order that did not hit the early return acts like
       `Forward`, matching upstream.

4. Preserve scope and ownership.
   - Do not add methods to `Selection` that require `PageList` or `Screen`.
   - Do not make `Selection` own or untrack pins.
   - Do not call PageList tracking/untracking helpers.
   - Do not add `Selection::deinit`, `Selection::track`, containment,
     adjustment, formatting, gestures, C ABI, Screen, search, renderer, parser,
     app, or terminal mutation behavior.

5. Add tests.
   - Add focused PageList tests porting upstream behavior:
     - regular selection order:
       - forward multi-line;
       - reverse multi-line;
       - forward same-line;
       - forward single-cell;
       - reverse same-line;
     - rectangle selection order:
       - forward top-left to bottom-right;
       - reverse bottom-right to top-left;
       - mirrored forward top-right to bottom-left;
       - mirrored reverse bottom-left to top-right;
       - forward single-row left-to-right;
       - reverse single-row right-to-left;
       - forward single-column top-to-bottom;
       - reverse single-column bottom-to-top;
       - forward single-cell;
     - top-left normalization for forward, reverse, mirrored-forward, and
       mirrored-reverse rectangle selections;
     - bottom-right normalization for forward, reverse, mirrored-forward, and
       mirrored-reverse selections;
     - ordered selection conversion for forward, reverse, mirrored-forward, and
       mirrored-reverse inputs;
     - cross-page selection ordering uses screen coordinates, not page-local row
       values.
     - invalid/unmappable endpoint pins return `None` for each helper:
       - `selection_order`;
       - `selection_top_left`;
       - `selection_bottom_right`;
       - `selection_ordered`.
   - Existing selection value, selection-codepoint, highlight, and PageList
     tests must continue passing.

6. Keep scope narrow.
   - Do not add selection containment.
   - Do not add contained-row helpers.
   - Do not add selection adjustment.
   - Do not add word or line selection.
   - Do not add selection formatting.
   - Do not add selection gestures.
   - Do not add C ABI.
   - Do not add Screen.
   - Do not add search, renderer, parser, app, or terminal mutation behavior.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list::tests::selection
     cargo test -p roastty terminal::selection
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Independent review.
   - Before implementation, get an independent agent review of this experiment
     design.
   - After implementation and verification, get an independent result review.
   - Fix all real findings before proceeding.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - order enum added;
     - PageList-owned ordering helpers;
     - top-left/bottom-right behavior;
     - ordered conversion behavior;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `selection::Order` exists with the four upstream order variants;
- `PageList` can compute selection order from screen coordinates;
- regular and rectangle ordering match upstream semantics;
- top-left and bottom-right normalized pins match upstream behavior for forward,
  reverse, mirrored-forward, and mirrored-reverse selections;
- ordered conversion returns new untracked selections in requested forward or
  reverse order;
- ordered conversion preserves original start/end when the selection is already
  in the desired order, including mirrored desired orders;
- ordered conversion treats mirrored desired orders that do not match the
  current order as forward;
- cross-page ordering uses screen coordinates;
- invalid/unmappable endpoint pins return `None`;
- no containment, contained-row, adjustment, word/line selection, formatting,
  gestures, C ABI, Screen, search, renderer, parser, app, tracking ownership, or
  terminal mutation behavior is introduced;
- `cargo fmt`, targeted selection/PageList tests, and full
  `cargo test -p roastty` pass;
- independent design and result reviews approve the experiment, or all real
  findings are fixed before proceeding.

The experiment is partial if:

- regular ordering works, but rectangle mirrored ordering or ordered conversion
  needs a follow-up.

The experiment fails if:

- regular or rectangle ordering diverges from upstream;
- top-left or bottom-right normalization swaps the wrong x/y basis;
- ordered conversion returns tracked selections or mutates the original
  selection;
- cross-page order uses page-local row values instead of screen coordinates;
- invalid endpoint pins panic instead of returning `None`;
- containment, adjustment, formatting, gestures, ABI, Screen, search, renderer,
  parser, app, tracking ownership, or unrelated behavior is added prematurely;
- tests or formatting fail.

## Result

**Result:** Pass

Implemented the selection ordering slice in `roastty/src/terminal/selection.rs`
and `roastty/src/terminal/page_list.rs`.

The change adds the terminal-internal `selection::Order` enum with the four
upstream variants: `Forward`, `Reverse`, `MirroredForward`, and
`MirroredReverse`.

`PageList` now owns private selection helpers for the coordinate-dependent
behavior:

- `selection_order`;
- `selection_top_left`;
- `selection_bottom_right`;
- `selection_ordered`.

The helpers use `point_from_pin(point::Tag::Screen, pin)` as the screen
coordinate source of truth and guard endpoints with `pin_is_valid`, so invalid
or unmappable endpoints return `None` instead of panicking.

The implementation mirrors upstream `Selection.zig` behavior:

- regular selections order by screen row, then screen column;
- rectangle selections include forward, reverse, mirrored-forward, and
  mirrored-reverse cases;
- top-left and bottom-right normalization preserve upstream's x/y pin
  reconstruction;
- `selection_ordered` returns a new untracked selection;
- exact desired-order matches preserve original start/end, including mirrored
  desired orders;
- nonmatching mirrored desired orders fall back to forward ordering.

Added PageList tests for:

- regular selection ordering;
- rectangle ordering, including single-row, single-column, and single-cell edge
  cases;
- top-left and bottom-right normalization;
- ordered forward/reverse conversion;
- mirrored desired-order early return;
- nonmatching mirrored desired-order fallback to forward;
- cross-page ordering using screen coordinates;
- invalid start and end pins returning `None` for each helper.

Verification:

```bash
cargo fmt
cargo test -p roastty terminal::page_list::tests::selection
cargo test -p roastty terminal::selection
cargo test -p roastty
```

Results:

- `cargo fmt` completed successfully.
- `cargo test -p roastty terminal::page_list::tests::selection`: 10 passed.
- `cargo test -p roastty terminal::selection`: 12 passed.
- `cargo test -p roastty`: 587 unit tests passed, ABI harness passed, doctests
  passed.

Independent result review approved the implementation as a Pass. The reviewer
found no blocking issues, confirmed upstream ordering semantics, confirmed the
mirrored `ordered` early-return behavior, confirmed the invalid endpoint
coverage, and found no scope drift.

## Conclusion

Roastty now has the upstream selection ordering, top-left, bottom-right, and
ordered-conversion behavior needed by later selection containment and text
extraction work. The behavior remains PageList-owned until a higher-level Screen
layer exists, which keeps this port aligned with the current Roastty terminal
architecture without adding premature selection features.
