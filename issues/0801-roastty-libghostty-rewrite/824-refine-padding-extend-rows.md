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

# Experiment 824: Refine Padding Extend Rows

## Description

Connect `FrameRebuildPlan` to the per-row `padding_extend` refinement that
upstream performs inside `rebuildRow`. Experiment 823 resets
`MetalUniforms.padding_extend` on full rebuilds. Upstream then refines the top
and bottom padding-extension edges while rebuilding rows in
`window-padding-color = extend` mode: row `0` can disable the `up` edge and the
last row can disable the `down` edge when `rowNeverExtendBg` says that row's
background should not extend.

Roastty already has the low-level `MetalUniforms::refine_padding_extend` helper.
This experiment adds the frame-level driver that applies it for rebuilt boundary
rows using prepared per-row `never_extend` booleans. It deliberately does not
derive those booleans from terminal row/cell/style data yet; the actual
`rowNeverExtendBg` equivalent remains a later live row-state collection slice.

This experiment does not collect live terminal state, mutate `Contents`, format
rows, update grid-size/full-rebuild uniforms, upload buffers, draw frames, pace
redraws, or add renderer-thread integration.

## Changes

- `roastty/src/renderer/frame_rebuild.rs`
  - Add `FramePaddingExtendInput<'a>` with:
    - `padding_color: WindowPaddingColor`, and
    - `row_never_extend: &'a [bool]`, indexed by viewport row.
  - Add `FramePaddingExtendValidationError` for malformed prepared inputs:
    - duplicate `rows_to_rebuild`,
    - rebuilt row out of bounds, and
    - missing `row_never_extend[row]` for a rebuilt boundary row when
      `padding_color == WindowPaddingColor::Extend`.
  - Add `FramePaddingExtendApplication` recording:
    - which boundary rows were refined, and
    - whether `padding_extend` actually changed, computed by snapshotting the
      field before refinement and comparing it with the value after all
      refinement calls.
  - Add
    `FrameRebuildPlan::refine_padding_extend_rows(&self, uniforms: &mut MetalUniforms, input: FramePaddingExtendInput<'_>) -> Result<FramePaddingExtendApplication, FramePaddingExtendValidationError>`.
  - Validate all required prepared inputs before mutating uniforms.
  - In `WindowPaddingColor::Extend` mode, iterate `rows_to_rebuild` in plan
    order. For rebuilt row `0`, call `MetalUniforms::refine_padding_extend` with
    `is_first_row = true`; for rebuilt row `effective_grid.rows - 1`, call it
    with `is_last_row = true`. Middle rows do not refine padding. On a one-row
    grid, preserve upstream's `if y == 0 ... else if last` behavior by refining
    only the `up` edge.
  - Treat zero-row grids as a no-op when `rows_to_rebuild` is empty: do not read
    `row_never_extend`, do not mutate uniforms, and report no refined rows. If a
    malformed zero-row plan has any rebuilt row, reject it as out of bounds
    before mutation.
  - In `Background` and `ExtendAlways` modes, leave uniforms unchanged and do
    not require `row_never_extend` entries, matching upstream's no-op row
    refinement for those modes.
  - Add tests proving:
    - top-row refinement can clear `EXTEND_UP` when the prepared row says never
      extend,
    - bottom-row refinement can clear `EXTEND_DOWN`,
    - middle rows and clean plans are no-ops,
    - a one-row grid refines only the upstream top-row branch,
    - an empty zero-row plan is a no-op and malformed nonempty zero-row plans
      reject before mutation,
    - `Background` and `ExtendAlways` skip row refinement and do not require row
      inputs,
    - a boundary row can be refined while leaving `padding_extend` unchanged and
      reporting `padding_extend_mutated == false`,
    - missing required boundary row input rejects before mutation, and
    - duplicate/out-of-bounds rebuilt rows reject before mutation.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, update the renderer tracker to mention that prepared
    row-level padding-extension refinement is available while live
    `rowNeverExtendBg` derivation remains open.

## Verification

- Inspect:
  - `vendor/ghostty/src/renderer/generic.zig` `rebuildRow` padding-extension
    branch.
  - `vendor/ghostty/src/renderer/row.zig` `neverExtendBg`.
  - `roastty/src/renderer/metal/shaders.rs`
    `MetalUniforms::refine_padding_extend`.
  - `roastty/src/renderer/frame_rebuild.rs`.
- Run Rust formatting:
  - `cargo fmt -p roastty`
- Run targeted tests:
  - `cargo test -p roastty renderer::frame_rebuild -- --nocapture`
  - `cargo test -p roastty renderer::metal::shaders::tests::refine_padding_extend -- --nocapture`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/824-refine-padding-extend-rows.md`
- Run:
  - `git diff --check`

The experiment passes if a prepared frame plan can refine Metal `padding_extend`
for rebuilt top/bottom rows in `Extend` mode without requiring live terminal row
analysis. It is Partial if row refinement lands but the prepared input shape
needs follow-up changes before live integration. It fails if the row refinement
cannot be separated from the full `rowNeverExtendBg` terminal-state derivation.

## Design Review

Codex reviewed the initial design and found two blocking gaps. First, zero-row
grids were not specified even though `FrameRebuildPlan` permits them; the design
now requires empty zero-row plans to be no-ops and malformed nonempty zero-row
plans to reject as out of bounds before mutation. Second, the
`padding_extend_mutated` metadata was ambiguous; the design now requires a
before/after comparison of the uniform field and a test where a boundary row is
refined but the field does not change.

Codex re-reviewed the amended design and approved it for implementation with no
remaining blockers. The re-review confirmed that zero-row behavior, malformed
zero-row rejection, before/after mutation metadata, one-row top-branch behavior,
`Background`/`ExtendAlways` no-op handling, and prepared `row_never_extend`
scope now match the intended experiment.
