+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "default"
reasoning = "medium"

[review.result]
agent = "codex"
model = "default"
reasoning = "medium"
+++

# Experiment 816: Apply Frame Rebuild Plan

## Description

Apply Experiment 815's frame rebuild plan to renderer cell storage. The planner
now decides the upstream `rebuildCells` front-half actions: resize, full reset,
row clears, and row dirty-flag cleanup. The next missing piece is a tested
mutation boundary that performs those actions against `Contents` and the
render-state row dirty flags before future row formatting repopulates rebuilt
rows.

This experiment does not format terminal rows, shape glyphs, draw cursors,
handle preedit glyph emission, upload GPU buffers, run Metal draws, or add the
renderer thread. It only turns a reviewed `FrameRebuildPlan` into the CPU-side
storage mutations that upstream performs immediately before/inside its row
rebuild loop.

## Changes

- `roastty/src/renderer/cell.rs`
  - Add a `Contents::size(&self) -> GridSize` getter so frame integration can
    inspect the current cell-storage grid without reaching into private fields.
- `roastty/src/renderer/frame_rebuild.rs`
  - Add `FrameRebuildApplyError` for invalid plans or inputs that would make
    application unsafe:
    - contents grid does not match `effective_grid` when no resize is planned,
    - `resize_to` and `effective_grid` disagree,
    - a `clear_rows` index is outside the effective post-resize contents grid,
      or
    - the row-dirty slice is too short for `rows_to_mark_clean`.
  - Add `FrameRebuildApplication` result metadata containing:
    - `resized_to`,
    - `reset_contents`,
    - `cleared_rows`, and
    - `marked_clean_rows`.
  - Add
    `FrameRebuildPlan::apply_to_contents(&self, contents: &mut Contents, row_dirty: &mut [bool]) -> Result<FrameRebuildApplication, FrameRebuildApplyError>`.
  - Apply actions in the no-formatting order needed for this slice:
    - resize `Contents` first when `resize_to` is present,
    - reset all contents when `reset_contents` is true,
    - clear each row in `clear_rows`,
    - set each `rows_to_mark_clean` row dirty flag to `false`.
  - Validate every mutation index and grid invariant before mutation so the
    caller never gets a partially applied plan when a stale or manually
    constructed plan is invalid.
  - Treat the clear/mark-clean operations as a batched application equivalent to
    upstream only for this no-formatting slice. Upstream clears, marks clean,
    and rebuilds each row inside the loop; the later row-formatting experiment
    must restore that per-row clear/mark/rebuild sequencing when it writes
    cells.
  - Keep row formatting separate: rows are cleared/reset and marked clean, but
    no new cells are written in this experiment.
  - Add tests proving:
    - resize happens before reset/clear and leaves `Contents` at the target grid
      size,
    - `Contents::size()` reports the post-apply grid size,
    - full rebuild resets all existing contents and marks every planned row
      clean, including cursor-reserved foreground lists,
    - partial/clean dirty-row plans clear only planned rows and preserve other
      row contents,
    - pre-validation prevents partial mutation on short row-dirty slices,
      out-of-bounds `clear_rows`, and grid mismatches,
    - applying an empty clean plan is a no-op, and
    - application metadata matches the performed actions.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the renderer tracker to mention that the frame
    rebuild plan can be applied to `Contents`, while terminal row formatting,
    glyph upload/draw calls, pacing, and live renderer integration remain open.

## Verification

- Inspect:
  - `vendor/ghostty/src/renderer/generic.zig` `rebuildCells`
  - `roastty/src/renderer/frame_rebuild.rs`
  - `roastty/src/renderer/cell.rs`
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty renderer::frame_rebuild -- --nocapture`
  - `cargo test -p roastty renderer::cell::tests::contents_resize -- --nocapture`
  - `cargo test -p roastty renderer::cell::tests::clear -- --nocapture`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/816-apply-frame-rebuild-plan.md`
- Run:
  - `git diff --check`

The experiment passes if a `FrameRebuildPlan` can be applied deterministically
to `Contents` and row dirty flags in a batched order equivalent to upstream for
this no-formatting slice, with tests proving resize, reset, row clear,
mark-clean, no-op, and invalid-plan behavior. It is Partial if the applier lands
but needs follow-up to match an upstream side effect. It fails if applying the
plan cannot be separated cleanly from terminal row formatting.

## Design Review

Codex reviewed the initial design and found one blocker before implementation:
the planned pre-validation only covered short row-dirty slices, but
`apply_to_contents` would also call `Contents::clear`, which asserts that the
row is in bounds. Because `FrameRebuildPlan` fields are visible within the
crate, a stale or manually constructed plan could resize/reset and then panic on
an invalid clear row, contradicting the no-partial-mutation guarantee. Codex
also noted that the plan should describe its clear/mark-clean ordering as
batched equivalent for this no-formatting slice, not exact upstream loop order,
and should add invalid-plan tests for out-of-bounds clear rows and grid
mismatches.

The design was amended to validate all mutation indexes and grid invariants
before any mutation, add error cases for stale/mismatched plans, document the
batched-order limitation, and expand tests for out-of-bounds clear rows,
row-dirty max-index validation, full reset of cursor-reserved lists, and
post-apply `Contents::size()`.

Codex re-reviewed the amended design and approved it with no remaining blocking
findings. The follow-up review confirmed that the grid/mutation pre-validation
resolves the no-partial-mutation blocker, that the batched-order caveat is clear
for this no-formatting slice, and that the planned tests cover the previously
missing cases.

## Result

**Result:** Pass

Roastty can now apply frame rebuild plans to CPU-side renderer cell storage:

- `roastty/src/renderer/cell.rs` adds `Contents::size`.
- `roastty/src/renderer/frame_rebuild.rs` adds `FrameRebuildApplyError` and
  `FrameRebuildApplication`.
- `FrameRebuildPlan::apply_to_contents` pre-validates plan/input invariants,
  resizes `Contents` first when needed, resets full-rebuild contents, clears
  planned rows, marks planned dirty rows clean, and returns metadata describing
  the applied actions.
- Pre-validation covers no-resize grid mismatches, `resize_to`/`effective_grid`
  disagreement, out-of-bounds `clear_rows`, and short row-dirty slices for
  `rows_to_mark_clean`, so invalid plans do not partially mutate `Contents`.
- Tests cover resize-before-reset, post-apply size, full reset including cursor
  reserved lists, partial row clear preservation, no-op application, application
  metadata, and invalid-plan no-mutation cases.

Verification:

- Inspected `vendor/ghostty/src/renderer/generic.zig` `rebuildCells`.
- Inspected `roastty/src/renderer/frame_rebuild.rs`.
- Inspected `roastty/src/renderer/cell.rs`.
- `cargo fmt -p roastty` — passed.
- `cargo test -p roastty renderer::frame_rebuild -- --nocapture` — passed, 21
  tests.
- `cargo test -p roastty renderer::cell::tests::contents_resize -- --nocapture`
  — passed, 4 tests.
- `cargo test -p roastty renderer::cell::tests::clear -- --nocapture` — passed,
  2 tests.
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/816-apply-frame-rebuild-plan.md`
  — passed.
- `git diff --check` — passed.

## Conclusion

Experiment 816 completes the mutation boundary for the value-level frame rebuild
plan: future renderer integration can now plan and apply resize/reset/clear and
row-dirty cleanup before repopulating rows. The next renderer slice should
restore the per-row upstream sequencing: clear the row, mark it clean, then
rebuild that row into `Contents`, while leaving GPU upload, Metal draw
submission, pacing, and renderer-thread integration for later.

## Completion Review

Codex reviewed the completed implementation and approved it with no blocking
code findings. The review confirmed that `apply_to_contents` validates before
mutation, resizes first, resets full-rebuild contents, clears planned rows,
marks planned rows clean, and resolves the design-review validation blocker by
checking grid mismatches, resize/effective-grid mismatches, clear-row bounds,
and dirty-slice max-index requirements. The only findings were documentation
fixes: record the successful Prettier and `git diff --check` commands, and
correct the future row-formatting order to `clear -> mark clean -> rebuild row`.
Both were fixed before the result commit.
