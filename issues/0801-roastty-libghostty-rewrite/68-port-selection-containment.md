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
     - if current PageList behavior permits constructing a structurally valid
       candidate pin that cannot map to `point::Tag::Screen`, add that case too.
       If not, document that `Tag::Screen` starts at the first stored node, so a
       PageList-valid owned pin maps to screen coordinates by construction.
       Still ensure the helper calls `point_from_pin(point::Tag::Screen, pin)`
       for the candidate instead of relying on `pin_is_valid` alone.
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

## Result

**Result:** Pass

Implemented selection containment in `roastty/src/terminal/page_list.rs`.

The change adds a private `selection_pin_screen_point` helper and a private
`selection_contains(selection, pin) -> Option<bool>` helper on `PageList`.
Containment remains PageList-owned because it depends on converting pins to
screen coordinates; no `Selection::contains` method was added.

The containment helper:

- normalizes selections with the Experiment 67 `selection_top_left` and
  `selection_bottom_right` helpers;
- converts normalized top-left, bottom-right, and candidate pins through
  `point_from_pin(point::Tag::Screen, pin)`;
- matches upstream regular-selection rules for single-line, top-row, bottom-row,
  and middle-row containment;
- matches upstream rectangle-selection rules for inclusive rectangular bounds;
- handles mirrored rectangles through normalized top-left/bottom-right pins;
- returns `None` for invalid selection endpoints, invalid candidate pins,
  missing-node pins, garbage pins, or any unmappable screen conversion.

During implementation, the design was corrected and re-reviewed: current
PageList behavior cannot construct a PageList-owned structurally valid pin that
fails `point::Tag::Screen` mapping, because `Tag::Screen` starts at the first
stored node. That failure mode applies to narrower coordinate spaces such as
Active or Viewport. The implementation still calls `point_from_pin` for the
candidate before comparing coordinates.

Added tests for:

- regular forward and reverse multi-line containment using the upstream included
  and excluded points;
- regular single-line containment;
- rectangle forward and reverse containment using the upstream interior, border,
  and excluded points;
- rectangle single-line containment;
- mirrored-forward and mirrored-reverse rectangle containment after
  normalization;
- cross-page containment using screen rows;
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
- `cargo test -p roastty terminal::page_list::tests::selection`: 17 passed.
- `cargo test -p roastty terminal::selection`: 12 passed.
- `cargo test -p roastty`: 594 unit tests passed, ABI harness passed, doctests
  passed.

Independent result review approved the implementation as a Pass. The reviewer
found no blocking issues, confirmed upstream containment semantics, confirmed
the `Option<bool>` invalid-pin behavior, confirmed the test coverage, and found
no scope drift.

## Conclusion

Roastty now has the upstream `Selection.contains` behavior represented as a
PageList-owned helper. This completes the containment layer that sits directly
above selection ordering and normalization, while leaving row extraction,
adjustment, and higher-level selection features for later experiments.
