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

# Experiment 820: Apply Cursor Uniforms

## Description

Connect the prepared frame rebuild path to the existing Metal cursor-uniform
helpers. Experiment 819 can clear stale cursor glyphs, suppress cursor drawing
when preedit is active, and emit cursor/preedit overlay glyphs into `Contents`.
Upstream `rebuildCells` also clears the shader cursor position every frame and,
for visible block cursors only, sets `cursor_pos`, `cursor_wide`, and
`cursor_color` so covered text is recolored under the block cursor.

This experiment keeps the input prepared. It does not collect live terminal
render state, resolve cursor style from `RenderStateScalar`, compute
under-cursor terminal style/color from live cells, update custom shader cursor
animation uniforms, upload GPU buffers, submit draw calls, pace redraws, or add
the live renderer thread.

## Changes

- `roastty/src/renderer/frame_rebuild.rs`
  - Add `FrameCursorUniformInput` with prepared cursor-uniform state:
    - `preedit_active: bool`, and
    - `block_cursor: Option<FrameBlockCursorUniform>`.
  - Add `FrameBlockCursorUniform` with:
    - `grid_pos: [u16; 2]`,
    - `wide: font::run::Wide`, and
    - `color: Rgb`.
  - Add `FrameCursorUniformValidationError` for block cursor positions outside
    the plan's effective grid and `Wide::Wide` block cursors whose second cell
    would extend past the effective grid. `Wide::SpacerTail` keeps the existing
    upstream-compatible backstep behavior in
    `MetalUniforms::update_block_cursor`.
  - Add `FrameCursorUniformApplication` recording whether cursor uniforms were
    cleared and whether block cursor uniforms were set.
  - Add
    `FrameRebuildPlan::apply_cursor_uniforms(&self, uniforms: &mut MetalUniforms, input: FrameCursorUniformInput) -> Result<FrameCursorUniformApplication, FrameCursorUniformValidationError>`.
  - Validate prepared cursor uniform inputs before mutation.
  - After validation, always call `MetalUniforms::clear_cursor()`, matching
    upstream's per-frame cursor-position clear.
  - If `input.preedit_active` is true, leave the cursor uniform cleared even
    when a block cursor input is present.
  - If preedit is inactive and `block_cursor` is present, call
    `MetalUniforms::update_block_cursor` with the prepared position, wide kind,
    and resolved cursor text color.
  - If preedit is inactive and `block_cursor` is absent, leave the cursor
    uniform cleared. Non-block cursor styles intentionally do not set block
    cursor uniforms.
  - Add tests proving:
    - no cursor input clears only `cursor_pos` and leaves the previous
      `cursor_color`/`cursor_wide` values untouched,
    - active preedit clears `cursor_pos` and suppresses a prepared block cursor,
      without updating the stale `cursor_color` or `cursor_wide` fields,
    - a block cursor applies position, spacer-tail backstep/wide handling, and
      opaque cursor color through `MetalUniforms::update_block_cursor`,
    - invalid cursor positions reject before mutating uniforms, and
    - `Wide::Wide` at the last column rejects before mutating uniforms while
      `Wide::SpacerTail` at column zero still uses the helper's saturating
      backstep,
    - non-block cursor uniform state is represented by `block_cursor: None` and
      leaves the cursor position cleared.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the renderer tracker to mention that prepared
    block cursor uniforms can be cleared/applied after cursor/preedit overlay
    emission, while live terminal-state collection, custom shader cursor
    animation updates, glyph upload/draw calls, pacing, and renderer-thread
    integration remain open.

## Verification

- Inspect:
  - `vendor/ghostty/src/renderer/generic.zig` `rebuildCells` cursor uniform
    section
  - `roastty/src/renderer/frame_rebuild.rs`
  - `roastty/src/renderer/metal/shaders.rs`
  - `roastty/src/renderer/cell.rs`
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty renderer::frame_rebuild -- --nocapture`
  - `cargo test -p roastty renderer::metal::shaders::tests::update_block_cursor -- --nocapture`
  - `cargo test -p roastty renderer::metal::shaders::tests::clear_cursor -- --nocapture`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/820-apply-cursor-uniforms.md`
- Run:
  - `git diff --check`

The experiment passes if prepared block cursor uniform inputs can be validated
and applied after text overlays using the existing Metal uniform helpers,
without duplicating live cursor-style or color resolution. It is Partial if the
driver lands but needs a follow-up to reconcile a missing prepared input. It
fails if cursor uniform updates cannot be separated cleanly from live terminal
render-state collection.

## Design Review

Codex reviewed the initial design and approved the prepared-input scope, but
found one required validation fix. A `Wide::Wide` block cursor anchored at the
last column is inside the grid by anchor position but would extend past the
effective grid, so the uniform driver must reject it before mutating
`MetalUniforms`. The design keeps `Wide::SpacerTail` on the existing helper path
because its saturating backstep matches upstream. Codex also asked the active
preedit suppression test to assert that stale `cursor_color` and `cursor_wide`
values are not updated when the prepared block cursor is suppressed.

The design was amended to add the `Wide::Wide` extent validation and explicit
tests for suppressed-block non-position fields, `Wide::Wide` rejection, and
`Wide::SpacerTail` helper behavior.

Codex re-reviewed the amended design and approved it for implementation with no
remaining blockers. The re-review confirmed that the `Wide::Wide` last-column
validation, `Wide::SpacerTail` helper path, and active-preedit stale-field
assertions resolve the prior findings.

## Result

**Result:** Pass

Roastty can now apply prepared block cursor uniforms after text overlays:

- `roastty/src/renderer/frame_rebuild.rs` adds `FrameBlockCursorUniform`,
  `FrameCursorUniformInput`, `FrameCursorUniformValidationError`, and
  `FrameCursorUniformApplication`.
- `FrameRebuildPlan::apply_cursor_uniforms` validates prepared block cursor
  uniform input before mutation, always clears `MetalUniforms.cursor_pos` after
  validation, suppresses block cursor uniforms while preedit is active, and
  applies visible block cursor uniforms through
  `MetalUniforms::update_block_cursor`.
- Validation rejects cursor anchors outside the effective grid and rejects
  `Wide::Wide` cursors whose second cell would extend past the grid.
- `Wide::SpacerTail` remains on the existing helper path, preserving the
  upstream-compatible saturating backstep behavior.
- Tests cover clear-only frames, active-preedit block suppression while stale
  color/wide fields remain untouched, spacer-tail block cursor application,
  spacer-tail-at-zero backstep behavior, out-of-bounds rejection before
  mutation, and wide-last-column rejection before mutation.

Verification:

- Inspected `vendor/ghostty/src/renderer/generic.zig` `rebuildCells` cursor
  uniform section.
- Inspected `roastty/src/renderer/frame_rebuild.rs`.
- Inspected `roastty/src/renderer/metal/shaders.rs`.
- Inspected `roastty/src/renderer/cell.rs`.
- `cargo fmt -p roastty` — passed.
- `cargo test -p roastty renderer::frame_rebuild -- --nocapture` — passed, 56
  tests.
- `cargo test -p roastty renderer::metal::shaders::tests::update_block_cursor -- --nocapture`
  — passed, 1 test.
- `cargo test -p roastty renderer::metal::shaders::tests::clear_cursor -- --nocapture`
  — passed, 1 test.
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/820-apply-cursor-uniforms.md`
  — passed.
- `git diff --check` — passed.

## Conclusion

Experiment 820 connects prepared block cursor uniform state to the existing
Metal uniforms. The frame rebuild path can now clear cursor uniforms every frame
and apply block cursor position, width, and color only when preedit is inactive
and a prepared block cursor is present. The remaining render-loop work still
needs live terminal-state collection, custom shader cursor animation updates,
cell/glyph upload, draw-call submission, pacing, and renderer-thread
integration.

## Completion Review

Codex reviewed the completed implementation and found no blocking issues. The
review confirmed that validation happens before `MetalUniforms::clear_cursor`,
out-of-bounds anchors and `Wide::Wide` last-column overflow reject before
mutation, preedit suppression clears only `cursor_pos` without applying stale
block cursor color or width, and the tests cover invalid-input no-mutation,
`Wide::Wide` rejection, and `Wide::SpacerTail` backstep behavior including
column zero.

Codex also re-ran the focused `frame_rebuild`, `update_block_cursor`, and
`clear_cursor` test filters successfully and approved the experiment for the
result commit.
