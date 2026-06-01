# Experiment 69: Port Selection Contained Row

## Description

Port the `Selection.containedRow` and `Selection.containedRowCached` behavior
from upstream `Selection.zig`.

Upstream row extraction depends on `topLeft`, `bottomRight`, and
`pointFromPin(.screen, pin)`. Experiments 67 and 68 added the PageList-owned
ordering, normalization, and containment helpers that provide the equivalent
local foundation. This experiment should add only the next layer: returning the
portion of a selection that applies to one screen row.

This experiment must not add selection adjustment, word selection, line
selection, formatting, selection gestures, C ABI, Screen, search, renderer,
parser, app, or terminal mutation behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/Selection.zig` for:
     - `containedRow`;
     - `containedRowCached`;
     - the upstream `Selection: containedRow` test.
   - Use existing Roastty code:
     - `roastty/src/terminal/selection.rs`;
     - `roastty/src/terminal/page_list.rs::selection_top_left`;
     - `roastty/src/terminal/page_list.rs::selection_bottom_right`;
     - `roastty/src/terminal/page_list.rs::selection_pin_screen_point`;
     - `roastty/src/terminal/page_list.rs::selection_contains`.
   - Do not modify `vendor/ghostty/`.

2. Add PageList-owned row extraction.
   - In `roastty/src/terminal/page_list.rs`, add private helpers such as:

     ```rust
     fn selection_contained_row(
         &self,
         selection: selection::Selection,
         pin: Pin,
     ) -> Option<selection::Selection>

     fn selection_contained_row_cached(
         &self,
         selection: selection::Selection,
         top_left_pin: Pin,
         bottom_right_pin: Pin,
         pin: Pin,
         top_left: Coordinate,
         bottom_right: Coordinate,
         point: Coordinate,
     ) -> Option<selection::Selection>
     ```

   - `selection_contained_row` should compute normalized top-left and
     bottom-right pins, convert top-left, bottom-right, and the candidate pin to
     screen coordinates, then call the cached helper.
   - `selection_contained_row_cached` should mirror upstream
     `containedRowCached` semantics:
     - if the candidate row is above top-left or below bottom-right, return
       `None`;
     - for rectangle selections, return a new untracked rectangle selection on
       the candidate row from `top_left.x` to `bottom_right.x`;
     - for regular single-line selections, return a new untracked selection from
       `top_left_pin` to `bottom_right_pin`;
     - for the top row of a multi-line regular selection, return from
       `top_left_pin` to the last column of the candidate row;
     - for the bottom row of a multi-line regular selection, return from column
       zero of the candidate row to `bottom_right_pin`;
     - for middle rows of a multi-line regular selection, return the full row
       from column zero to `self.cols - 1`;
     - preserve the input selection's rectangle flag for rectangle rows and use
       `false` for regular rows.

3. Preserve Rust invalid-pin behavior.
   - Upstream assumes `pointFromPin(.screen, pin).?` succeeds. Roastty's
     PageList-owned helpers should not panic for invalid pins.
   - `selection_contained_row` should return `None` if:
     - the selection start cannot map to a valid screen point;
     - the selection end cannot map to a valid screen point;
     - the candidate pin cannot map to a valid screen point.
   - If the row is valid but not inside the selection's vertical range, return
     `None`, matching upstream's null result.
   - This means `None` represents both "not contained" and "invalid input" at
     this private helper boundary. Do not add a public error type or API surface
     in this experiment.

4. Preserve scope.
   - Do not add `Selection::contained_row` or `Selection::contained_row_cached`;
     row extraction is coordinate-aware and belongs with `PageList` until the
     upstream-like `Screen` layer exists.
   - Do not add selection adjustment, word/line selection, formatting, gestures,
     C ABI, Screen, search, renderer, parser, app, tracking ownership, or
     terminal mutation behavior.

5. Add tests.
   - Port the upstream regular contained-row cases:
     - row outside the selection returns `None`;
     - top row returns from top-left to last column;
     - bottom row returns from first column to bottom-right;
     - middle row returns the full row;
     - single-line selection returns the normalized original row selection.
   - Port the upstream rectangle contained-row cases:
     - row outside the rectangle returns `None`;
     - top row returns rectangle x bounds on the candidate row;
     - bottom row returns rectangle x bounds on the candidate row;
     - middle row returns rectangle x bounds on the candidate row.
   - Add reverse regular and reverse rectangle cases to prove normalization
     before row extraction.
   - Add mirrored rectangle cases enabled by Experiment 67:
     - mirrored-forward rectangle extracts the normalized x bounds on the
       candidate row;
     - mirrored-reverse rectangle extracts the normalized x bounds on the
       candidate row.
   - Add cross-page row extraction to prove screen coordinates are used across
     page boundaries.
   - Add invalid input coverage:
     - invalid selection start returns `None`;
     - invalid selection end returns `None`;
     - invalid candidate pin returns `None`;
     - garbage candidate pin returns `None`.
   - Existing selection ordering, containment, highlight, PageList, and full
     Roastty tests must continue passing.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list::tests::selection
     cargo test -p roastty terminal::selection
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

7. Independent review.
   - Before implementation, get an independent agent review of this experiment
     design.
   - After implementation and verification, get an independent result review.
   - Fix all real findings before proceeding.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - contained-row helpers added;
     - regular contained-row behavior;
     - rectangle contained-row behavior;
     - reverse and mirrored behavior;
     - invalid input behavior;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `PageList` can return the row-specific selection segment for a candidate row;
- regular contained-row behavior matches upstream top-row, bottom-row,
  middle-row, outside-row, and single-line behavior;
- rectangle contained-row behavior matches upstream top-row, bottom-row,
  middle-row, and outside-row behavior;
- reverse regular and reverse rectangle selections normalize before extraction;
- mirrored rectangle selections normalize before extraction;
- cross-page row extraction uses screen coordinates;
- invalid selection endpoints and invalid candidate pins return `None`;
- no adjustment, word/line selection, formatting, gestures, C ABI, Screen,
  search, renderer, parser, app, tracking ownership, or terminal mutation
  behavior is introduced;
- `cargo fmt`, targeted selection/PageList tests, and full
  `cargo test -p roastty` pass;
- independent design and result reviews approve the experiment, or all real
  findings are fixed before proceeding.

The experiment is partial if:

- regular row extraction works, but rectangle, reverse, or mirrored row
  extraction needs a follow-up.

The experiment fails if:

- row extraction diverges from upstream regular-selection row rules;
- rectangle row extraction returns full rows instead of rectangle x bounds;
- reverse or mirrored selections are not normalized before extraction;
- cross-page extraction uses page-local rows instead of screen coordinates;
- invalid pins panic instead of returning `None`;
- adjustment, word/line selection, formatting, gestures, ABI, Screen, search,
  renderer, parser, app, tracking ownership, or unrelated behavior is added
  prematurely;
- tests or formatting fail.

## Result

**Result:** Pass

Implemented row-specific selection extraction in
`roastty/src/terminal/page_list.rs`.

The change adds two private PageList helpers:

- `selection_contained_row`;
- `selection_contained_row_cached`.

The uncached helper computes normalized top-left and bottom-right pins, converts
top-left, bottom-right, and candidate pins through the existing screen-point
conversion path, then delegates to the cached helper. The cached helper mirrors
upstream `containedRowCached` behavior:

- rows outside the selection's vertical bounds return `None`;
- rectangle rows return a new untracked rectangle selection on the candidate row
  using the normalized rectangle x bounds;
- regular single-line selections return the normalized original row segment;
- regular top rows return from normalized top-left to the last column;
- regular bottom rows return from column zero to normalized bottom-right;
- regular middle rows return the full row.

The local Rust helper returns `None` both for rows outside the selection and for
invalid input pins. That preserves the private PageList helper pattern from
Experiments 67 and 68 without adding a public error type or API surface.

Added tests for:

- upstream regular top-row, bottom-row, middle-row, and outside-row behavior;
- reverse regular normalization;
- upstream rectangle top-row, bottom-row, middle-row, and outside-row behavior;
- reverse and mirrored rectangle normalization;
- regular single-line normalization;
- cross-page row extraction using screen rows;
- invalid selection start, invalid selection end, invalid candidate, and garbage
  candidate returning `None`.

Verification:

```bash
cargo fmt
cargo test -p roastty terminal::page_list::tests::selection
cargo test -p roastty terminal::selection
cargo test -p roastty
```

Results:

- `cargo fmt` completed successfully.
- `cargo test -p roastty terminal::page_list::tests::selection`: 24 passed.
- `cargo test -p roastty terminal::selection`: 12 passed.
- `cargo test -p roastty`: 601 unit tests passed, ABI harness passed, doctests
  passed.

Independent result review approved the implementation as a Pass. The reviewer
found no blocking issues, confirmed upstream `containedRow` /
`containedRowCached` semantics, confirmed test coverage, and found no scope
drift.

## Conclusion

Roastty now has the upstream row-extraction behavior needed to slice a selection
into the portion that applies to a single screen row. This completes the next
selection layer above ordering and containment while still leaving adjustment,
word/line selection, formatting, and higher-level selection features for later
experiments.
