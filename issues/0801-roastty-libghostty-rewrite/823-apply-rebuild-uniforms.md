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

# Experiment 823: Apply Rebuild Uniforms

## Description

Connect `FrameRebuildPlan` to the uniform updates that upstream `rebuildCells`
performs while rebuilding CPU cell contents. When the cell grid changes,
upstream updates `uniforms.grid_size` immediately after resizing cells. When a
full rebuild happens, upstream resets `padding_extend` based on
`window-padding-color` before per-row refinement. Roastty already has
`MetalUniforms::update_grid_size` and `MetalUniforms::reset_padding_extend`, but
the prepared frame rebuild path currently leaves those calls to the caller and
later presentation only validates that `grid_size` happens to match.

This experiment remains prepared-input only. It does not collect live
configuration, run row background extension heuristics, update screen/cell-size
uniforms, upload buffers, draw frames, pace redraws, or add the live renderer
thread.

## Changes

- `roastty/src/renderer/frame_rebuild.rs`
  - Add `FrameRebuildUniformInput` with:
    - `padding_color: WindowPaddingColor`.
  - Add `FrameRebuildUniformValidationError` for invalid plan shapes that would
    make uniform updates ambiguous:
    - `resize_to` present but not equal to `effective_grid`, and
    - `resize_to` present while `full_rebuild` is false. Upstream and the
      planner both treat any grid resize as a full rebuild.
  - Add `FrameRebuildUniformApplication` recording:
    - whether `grid_size` was updated,
    - whether `padding_extend` was mutated, and
    - the applied effective grid.
  - Add
    `FrameRebuildPlan::apply_rebuild_uniforms(&self, uniforms: &mut MetalUniforms, input: FrameRebuildUniformInput) -> Result<FrameRebuildUniformApplication, FrameRebuildUniformValidationError>`.
  - If `resize_to` is present, call `MetalUniforms::update_grid_size` with the
    plan's effective grid. This valid path also has `full_rebuild == true`.
  - If `full_rebuild` is true, call `MetalUniforms::reset_padding_extend` with
    the prepared padding color. Report `padding_extend_mutated == true` only
    when the padding color can actually mutate the field (`Extend` or
    `ExtendAlways`); `Background` is a no-op and reports `false`.
  - If neither condition is true, leave uniforms unchanged.
  - Do not refine `padding_extend` per row in this experiment; that requires the
    row background `neverExtendBg` equivalent and belongs in a follow-up.
  - Add tests proving:
    - full rebuild without grid resize resets `padding_extend` but leaves
      `grid_size` untouched,
    - grid resize plus full rebuild applies both updates in upstream order,
    - clean/partial no-resize plans leave uniforms untouched,
    - `WindowPaddingColor::Background` keeps `padding_extend` unchanged and
      reports no padding mutation,
    - malformed resize/effective-grid plans reject before mutation, and
    - malformed resize-without-full-rebuild plans reject before mutation.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the renderer tracker to mention that prepared
    rebuild uniform inputs can sync grid-size and full-rebuild padding-extension
    uniforms, while live terminal-state collection, row padding-extension
    refinement, custom shader enablement/upload, pacing, and renderer-thread
    integration remain open.

## Verification

- Inspect:
  - `vendor/ghostty/src/renderer/generic.zig` `rebuildCells` resize/full-rebuild
    uniform section
  - `roastty/src/renderer/frame_rebuild.rs`
  - `roastty/src/renderer/metal/shaders.rs`
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty renderer::frame_rebuild -- --nocapture`
  - `cargo test -p roastty renderer::metal::shaders::tests::update_grid_size -- --nocapture`
  - `cargo test -p roastty renderer::metal::shaders::tests::reset_padding_extend -- --nocapture`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/823-apply-rebuild-uniforms.md`
- Run:
  - `git diff --check`

The experiment passes if prepared rebuild plans can update grid-size and
full-rebuild padding-extension uniforms through the existing Metal uniform
helpers without live renderer-state collection. It is Partial if one of the two
uniform updates lands and the other needs a follow-up. It fails if these updates
cannot be separated from live renderer-loop orchestration.

## Design Review

Codex reviewed the initial design and found one blocking invariant issue and one
metadata ambiguity. First, upstream and Roastty's planner both make grid resize
imply full rebuild, so a resize-only uniform update is a malformed plan shape
and must reject before mutation rather than be tested as a valid case. Second,
`WindowPaddingColor::Background` is intentionally a no-op in
`MetalUniforms::reset_padding_extend`, so application metadata must not report
that padding was mutated for that case.

The design was amended to reject resize-without-full-rebuild plans before
mutation, remove the resize-only valid-case test, require resize/full-rebuild
tests to apply both upstream uniform updates, and report padding mutation only
for `Extend` and `ExtendAlways`.

Codex re-reviewed the amended design and approved it for implementation with no
remaining blockers. The re-review confirmed that resize/full-rebuild invariants,
Background no-op reporting, valid resize behavior, and row padding refinement
deferral now match the intended scope.
