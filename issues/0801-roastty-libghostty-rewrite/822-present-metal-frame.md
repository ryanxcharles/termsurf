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

# Experiment 822: Present Metal Frame

## Description

Connect the prepared frame rebuild output to Roastty's existing Metal frame
compositor. Experiments 815-821 can plan/rebuild contents, draw cursor/preedit
overlays, prepare Metal cursor uniforms, and update custom shader frame uniforms
from prepared inputs. The remaining prepared renderer step before live pacing
and thread integration is to hand the prepared `Contents`, `MetalUniforms`, and
font atlases to `MetalFrameCompositor::draw_frame` and report the presentation
metadata.

This experiment keeps presentation inputs prepared. It does not collect live
terminal render state, decide whether a frame should be drawn, pace redraws,
schedule swap-chain ticks, load or enable custom shaders, manage the renderer
thread, or integrate with the macOS surface lifecycle.

## Changes

- `roastty/src/renderer/frame_rebuild.rs`
  - Add `FrameMetalPresentationInput<'a>` with prepared presentation data:
    - `width: usize`,
    - `height: usize`,
    - `contents_scale: f64`,
    - `uniforms: &'a MetalUniforms`,
    - `contents: &'a Contents`,
    - `grayscale_atlas: &'a Atlas`, and
    - `color_atlas: &'a Atlas`.
  - Add `FrameMetalPresentationValidationError` for:
    - zero width/height,
    - `Contents` grid mismatches against the plan's effective grid, and
    - `MetalUniforms.grid_size` mismatches against the plan's effective grid.
      Leave `contents_scale` validation to `MetalFrameCompositor`, which already
      rejects non-finite or non-positive scales before target allocation.
  - Add `FrameMetalPresentationError` wrapping validation errors and
    `MetalFrameCompositorError`.
  - Add `FrameMetalPresentationApplication` recording:
    - the compositor's `MetalFramePresentation`,
    - whether foreground cells were uploaded/drawn (`fg_count > 0`), and
    - whether the target was reallocated.
  - Add
    `FrameRebuildPlan::present_metal_frame(&self, compositor: &mut MetalFrameCompositor, input: FrameMetalPresentationInput<'_>) -> Result<FrameMetalPresentationApplication, FrameMetalPresentationError>`.
  - Validate prepared dimensions, `Contents` grid, and uniform grid before
    calling the compositor.
  - Construct `MetalFrameInput` from the prepared input and call
    `MetalFrameCompositor::draw_frame`.
  - Do not duplicate frame sync, render-pass, command-buffer, target resize, or
    IOSurface presentation behavior from the compositor.
  - Add tests proving:
    - zero width/height reject before calling the compositor, including when
      `contents_scale` is also invalid so bridge validation wins first,
    - `Contents` grid mismatches reject before calling the compositor,
    - `MetalUniforms.grid_size` mismatches reject before calling the compositor,
    - invalid `contents_scale` is propagated from the compositor,
    - a background-only prepared frame presents successfully and reports
      `fg_count == 0`,
    - a prepared frame with a foreground glyph reports nonzero foreground count,
      and
    - target reallocation is reported when the prepared frame size changes.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the renderer tracker to mention that prepared
    Metal frame presentation can sync contents/atlases and submit draw calls
    through the compositor, while live terminal-state collection, custom shader
    enablement/upload, pacing, and renderer-thread integration remain open.

## Verification

- Inspect:
  - `vendor/ghostty/src/renderer/generic.zig` `drawFrame`
  - `roastty/src/renderer/frame_rebuild.rs`
  - `roastty/src/renderer/metal/compositor.rs`
  - `roastty/src/renderer/metal/frame.rs`
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty renderer::frame_rebuild -- --nocapture`
  - `cargo test -p roastty renderer::metal::compositor -- --nocapture`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/822-present-metal-frame.md`
- Run:
  - `git diff --check`

The experiment passes if prepared renderer frame outputs can be handed to the
Metal compositor and produce presentation metadata without duplicating
compositor internals or introducing live renderer-loop decisions. It is Partial
if the driver lands but a missing prepared input must be split into a follow-up.
It fails if frame presentation cannot be separated from live pacing/thread
integration.

## Design Review

Codex reviewed the initial design and found that prepared grid validation was
missing. Because the bridge is a `FrameRebuildPlan` method, it must reject
`Contents` whose grid does not match the plan's effective grid and
`MetalUniforms.grid_size` that does not match that same grid before calling the
compositor. Otherwise stale prepared state could reach GPU buffer sync and draw
submission. Codex also asked the tests to prove bridge validation runs before
compositor validation by combining a bridge error with an invalid
`contents_scale`.

The design was amended to add `Contents` grid and uniform grid validation before
compositor calls, plus tests for those no-compositor validation paths and for
bridge-validation precedence over compositor `contents_scale` validation.

Codex re-reviewed the amended design and approved it for implementation with no
remaining blockers. The re-review confirmed that contents grid validation,
uniform grid validation, bridge dimension validation, and
bridge-before-compositor validation tests resolve the prior findings while
preserving prepared-input scope and compositor delegation.
