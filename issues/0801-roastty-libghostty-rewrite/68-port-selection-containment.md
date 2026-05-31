# Experiment 68: Port Selection Containment

## Description

Port the `Selection.contains` behavior from upstream `Selection.zig`.

Upstream containment depends on `topLeft`, `bottomRight`, and
`pointFromPin(.screen, pin)`. Experiment 67 added the PageList-owned ordering
and normalization helpers that provide the equivalent local foundation. This
experiment should add only the next layer: determining whether a candidate pin
is inside a selection.

This experiment must not add `containedRow`, `containedRowCached`, selection
adjustment, word selection, line selection, formatting, selection gestures, C
ABI, Screen, search, renderer, parser, app, or terminal mutation behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/Selection.zig` for:
     - `contains`;
     - the upstream `Selection: contains` tests;
     - the upstream `Selection: contains, rectangle` tests.
   - Use existing Roastty code:
     - `roastty/src/terminal/selection.rs`;
     - `roastty/src/terminal/page_list.rs::selection_top_left`;
     - `roastty/src/terminal/page_list.rs::selection_bottom_right`;
     - `roastty/src/terminal/page_list.rs::point_from_pin`.
   - Do not modify `vendor/ghostty/`.

2. Add PageList-owned containment.
   - In `roastty/src/terminal/page_list.rs`, add a private helper such as:

     ```rust
     fn selection_contains(
         &self,
         selection: selection::Selection,
         pin: Pin,
     ) -> Option<bool>
     ```

   - Return `None` if:
     - the selection start cannot map to a valid screen point;
     - the selection end cannot map to a valid screen point;
     - the candidate pin cannot map to a valid screen point.
   - Use the Experiment 67 `selection_top_left` and `selection_bottom_right`
     helpers to normalize the selection.
   - Convert the normalized top-left, bottom-right, and candidate pins through
     `point_from_pin(point::Tag::Screen, pin)` before comparing coordinates.
   - Preserve upstream regular-selection containment semantics:
     - single-line selection contains only points on that row between the
       normalized x bounds, inclusive;
     - top row contains points with `x >= top_left.x`;
     - bottom row contains points with `x <= bottom_right.x`;
     - rows between top and bottom are fully contained.
   - Preserve upstream rectangle-selection containment semantics:
     - candidate row is between top and bottom, inclusive;
     - candidate column is between left and right, inclusive.

3. Preserve scope.
   - Do not add `Selection::contains`, because containment is coordinate-aware
     and belongs with `PageList` until the upstream-like `Screen` layer exists.
   - Do not add `containedRow` or `containedRowCached`; those need row slicing
     behavior and should be a separate experiment.
   - Do not add selection adjustment, word/line selection, formatting, gestures,
     C ABI, Screen, search, renderer, parser, app, tracking ownership, or
     terminal mutation behavior.

4. Add tests.
   - Port the upstream regular-selection containment cases:
     - forward multi-line selection;
     - reverse multi-line selection, with the same included top/bottom row
       points and excluded outside points as the upstream forward case after
       normalization;
     - single-line selection.
   - Port the upstream rectangle containment cases:
     - forward rectangle;
     - reverse rectangle;
     - single-line rectangle.
   - Add mirrored rectangle cases enabled by Experiment 67:
     - mirrored-forward rectangle contains the normalized rectangle interior and
       borders;
     - mirrored-reverse rectangle contains the normalized rectangle interior and
       borders.
   - Add cross-page containment to prove screen coordinates are used across page
     boundaries.
   - Add invalid/unmappable endpoint coverage:
     - invalid selection start returns `None`;
     - invalid selection end returns `None`;
     - invalid candidate pin returns `None`.
     - structurally valid candidate pin that cannot map to `point::Tag::Screen`
       returns `None`. This specifically catches the case where a pin is valid
       PageList storage but is before the current screen top, so `pin_is_valid`
       alone is not enough for the candidate.
   - Existing selection ordering, highlight, PageList, and full Roastty tests
     must continue passing.

5. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list::tests::selection
     cargo test -p roastty terminal::selection
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

6. Independent review.
   - Before implementation, get an independent agent review of this experiment
     design.
   - After implementation and verification, get an independent result review.
   - Fix all real findings before proceeding.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - containment helper added;
     - regular containment behavior;
     - rectangle containment behavior;
     - mirrored rectangle behavior;
     - invalid endpoint/candidate behavior;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `PageList` can determine whether a candidate pin is contained in a selection;
- regular containment matches upstream multi-line, reverse, and single-line
  behavior;
- rectangle containment matches upstream forward, reverse, and single-line
  behavior;
- mirrored rectangle containment uses normalized top-left/bottom-right bounds;
- containment uses screen coordinates across page boundaries;
- invalid selection endpoints and invalid candidate pins return `None`;
- no contained-row extraction, adjustment, word/line selection, formatting,
  gestures, C ABI, Screen, search, renderer, parser, app, tracking ownership, or
  terminal mutation behavior is introduced;
- `cargo fmt`, targeted selection/PageList tests, and full
  `cargo test -p roastty` pass;
- independent design and result reviews approve the experiment, or all real
  findings are fixed before proceeding.

The experiment is partial if:

- regular containment works, but rectangle or mirrored rectangle containment
  needs a follow-up.

The experiment fails if:

- containment diverges from upstream regular-selection row rules;
- rectangle containment uses line-selection rules instead of rectangular bounds;
- mirrored rectangles are not normalized before containment checks;
- cross-page containment uses page-local rows instead of screen coordinates;
- invalid pins panic instead of returning `None`;
- contained-row extraction, adjustment, formatting, gestures, ABI, Screen,
  search, renderer, parser, app, tracking ownership, or unrelated behavior is
  added prematurely;
- tests or formatting fail.
