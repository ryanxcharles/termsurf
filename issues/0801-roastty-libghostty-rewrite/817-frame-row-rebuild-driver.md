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

# Experiment 817: Frame Row Rebuild Driver

## Description

Add the per-row rebuild driver that restores upstream `rebuildCells` sequencing
after Experiments 815-816 split planning and batched application. Upstream does
not merely apply all clears and dirty-flag cleanup as a batch; inside the row
loop it clears a row when needed, marks that row clean, then rebuilds that row.
Roastty already has a frame rebuild plan, a no-formatting applier, and
row-formatting helpers in `cell.rs`. The missing bridge is a tested driver that
performs resize/full-reset setup, then invokes row rebuild work one planned row
at a time in upstream order.

This experiment does not yet wire real terminal `RunOptions`, search/link
highlight state, cursor/preedit emission, GPU upload, Metal draw submission,
pacing, or the renderer thread. It adds the sequencing boundary that later
integration can use to call the existing row-formatting helpers for each planned
row.

## Changes

- `roastty/src/renderer/frame_rebuild.rs`
  - Add `FrameRowRebuildValidationError` for invalid driver inputs:
    - the same plan/input validation errors already covered by
      `FrameRebuildApplyError`,
    - a `rows_to_rebuild` row outside the effective grid, and
    - duplicate rows in any row set, and
    - `clear_rows` or `rows_to_mark_clean` rows that are not present in
      `rows_to_rebuild`.
  - Add `FrameRowRebuildApplication<E>` metadata containing:
    - `resized_to`,
    - `reset_contents`,
    - `cleared_rows`,
    - `marked_clean_rows`, and
    - `rebuilt_rows` for rows whose callback succeeded,
    - `failed_rows`, with each failed row and callback error.
  - Add
    `FrameRebuildPlan::drive_row_rebuilds(&self, contents: &mut Contents, row_dirty: &mut [bool], rebuild_row: impl FnMut(&mut Contents, Unit) -> Result<(), E>) -> Result<FrameRowRebuildApplication<E>, FrameRowRebuildValidationError>`.
  - Validate all plan indexes and row-set consistency before mutation, including
    `rows_to_rebuild`, so stale/manual plans cannot partially mutate `Contents`.
  - Apply setup in upstream order:
    - resize `Contents` first when `resize_to` is present,
    - reset all contents when `reset_contents` is true,
    - for each row in `rows_to_rebuild`:
      - clear the row if it appears in `clear_rows`,
      - mark the row dirty flag clean when it appears in `rows_to_mark_clean`,
      - call the row rebuild callback for that row.
  - Match upstream row-error recovery for this callback boundary: if the
    callback returns an error, clear that row again, record the error in
    `failed_rows`, and continue with later rows. The driver returns validation
    failures as `Err`, but row-formatting callback failures are part of
    successful application metadata.
  - Keep `apply_to_contents` as the no-formatting batched helper from Experiment
    816; the new driver is the path future row-formatting integration should
    use.
  - Add tests proving:
    - partial rows execute as `clear -> mark clean -> rebuild` for each row,
    - full rebuild resets once, does not row-clear, marks each row clean, and
      rebuilds every planned row,
    - rows are rebuilt in plan order,
    - callback errors clear the failed row, record the row/error, and continue
      with later rows,
    - validation catches out-of-bounds `rows_to_rebuild` without mutation,
    - validation catches duplicate row-set entries and `clear_rows` /
      `rows_to_mark_clean` entries outside `rows_to_rebuild`,
    - validation catches dirty-slice/grid mismatches without mutation, including
      snapshots of unchanged row-dirty flags,
    - resize happens before any callback and callbacks observe the post-resize
      `Contents::size()`,
    - applying an empty clean plan invokes no callbacks and returns empty
      metadata, and
    - returned metadata matches the actions actually performed.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the renderer tracker to mention that the
    rebuild plan now has a per-row driver with upstream clear/mark/rebuild
    sequencing, while terminal row formatting, glyph upload/draw calls, pacing,
    and live renderer integration remain open.

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
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/817-frame-row-rebuild-driver.md`
- Run:
  - `git diff --check`

The experiment passes if a `FrameRebuildPlan` can drive row rebuild callbacks in
the same per-row success and error-recovery order as upstream's row loop, while
preserving the invalid-plan no-partial-mutation guarantees from Experiment 816.
It is Partial if the driver lands but needs a follow-up to match an upstream
side effect. It fails if the per-row sequencing cannot be separated cleanly from
terminal row formatting.

## Design Review

Codex reviewed the initial design and found two blockers before implementation.
First, the plan claimed upstream row-loop equivalence while callback errors
would stop before later rows; upstream catches a failed `rebuildRow`, clears
that row, and continues. Second, the driver would have ignored stale/manual
plans where `clear_rows` or `rows_to_mark_clean` contained rows not present in
`rows_to_rebuild`. Codex also requested tests for duplicate/stray row sets,
post-resize callback observations, empty clean plans, and dirty-slice
no-mutation snapshots.

The design was amended so callback errors clear the failed row, are recorded in
application metadata, and do not stop later rows. Validation now rejects
out-of-bounds, duplicate, and non-subset row-set entries before mutation. The
planned tests were expanded to cover the requested edge cases.

Codex re-reviewed the amended design and approved it with no blocking findings.
The follow-up review confirmed that callback failure handling and row-set
validation now match the scoped upstream behavior. The only clarification was
that `rebuilt_rows` should mean rows whose callback succeeded, with failed
attempts recorded only in `failed_rows`; the plan was updated accordingly before
the plan commit.

## Result

**Result:** Pass

Roastty now has a per-row frame rebuild driver:

- `roastty/src/renderer/frame_rebuild.rs` adds `FrameRowRebuildValidationError`,
  `FrameRowRebuildFailure`, and `FrameRowRebuildApplication`.
- `FrameRebuildPlan::drive_row_rebuilds` validates plan/input invariants,
  resizes first, resets full-rebuild contents, then visits each
  `rows_to_rebuild` row in order.
- For each row, the driver clears rows listed in `clear_rows`, marks rows listed
  in `rows_to_mark_clean` clean, then invokes the rebuild callback.
- Callback failures match upstream recovery for this boundary: the failed row is
  cleared again, recorded in `failed_rows`, and later rows continue.
- Validation rejects stale/manual row-set inconsistencies before mutation:
  out-of-bounds rebuild rows, duplicate row entries, `clear_rows` outside
  `rows_to_rebuild`, `rows_to_mark_clean` outside `rows_to_rebuild`, and the
  existing grid/dirty-slice validation errors.
- `rebuilt_rows` contains only rows whose callback succeeded.

Verification:

- Inspected `vendor/ghostty/src/renderer/generic.zig` `rebuildCells`.
- Inspected `roastty/src/renderer/frame_rebuild.rs`.
- Inspected `roastty/src/renderer/cell.rs`.
- `cargo fmt -p roastty` — passed.
- `cargo test -p roastty renderer::frame_rebuild -- --nocapture` — passed, 32
  tests.
- `cargo test -p roastty renderer::cell::tests::contents_resize -- --nocapture`
  — passed, 4 tests.
- `cargo test -p roastty renderer::cell::tests::clear -- --nocapture` — passed,
  2 tests.
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/817-frame-row-rebuild-driver.md`
  — passed.
- `git diff --check` — passed.

## Conclusion

Experiment 817 restores the per-row sequencing boundary that future row
formatting should use: resize/reset setup first, then per planned row
`clear -> mark clean -> rebuild callback`, with upstream-style
clear-and-continue recovery when a row rebuild fails. The next renderer slice
can connect this driver to the existing `cell.rs` row-formatting helpers and
real per-row `RunOptions` inputs, while leaving GPU upload, Metal draw
submission, pacing, and renderer-thread integration for later.

## Completion Review

Codex reviewed the completed implementation and found no driver correctness
bugs. The review confirmed that the driver validates before mutation, resizes,
resets, executes per-row clear/mark-clean/callback sequencing, and clears failed
callback rows while continuing to later rows. It found two issues before the
result commit: duplicate `clear_rows` and duplicate `rows_to_mark_clean` needed
explicit tests because they were part of the amended blocker resolution, and the
result record omitted the successful Prettier and `git diff --check` commands.

Both findings were fixed before the result commit. The duplicate row-set tests
were added, `renderer::frame_rebuild` increased from 30 to 32 passing tests, and
the verification record now includes Prettier and `git diff --check`.
