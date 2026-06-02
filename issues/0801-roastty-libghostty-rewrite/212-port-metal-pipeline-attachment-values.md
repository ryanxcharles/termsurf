# Experiment 212: Port Metal Pipeline Attachment Values

## Description

Experiment 211 added value-level Metal vertex descriptor mapping. The remaining
value-only part of upstream `vendor/ghostty/src/renderer/metal/Pipeline.zig`
before real Objective-C pipeline creation is color attachment configuration.

Upstream pipeline options include one or more color attachments:

```zig
pub const Attachment = struct {
    pixel_format: mtl.MTLPixelFormat,
    blending_enabled: bool = true,
};
```

For each attachment, upstream sets:

- `pixelFormat`;
- `blendingEnabled`;
- when blending is enabled, premultiplied-alpha blend configuration:
  - `rgbBlendOperation = add`;
  - `alphaBlendOperation = add`;
  - `sourceRGBBlendFactor = one`;
  - `sourceAlphaBlendFactor = one`;
  - `destinationRGBBlendFactor = one_minus_source_alpha`;
  - `destinationAlphaBlendFactor = one_minus_source_alpha`.

Roastty should port this attachment configuration as internal value types and
tests. It should not create `MTLRenderPipelineDescriptor`, shader libraries, or
pipeline state yet.

All public names must use Roastty naming.

## Changes

1. Extend the Metal API value layer.

   In `roastty/src/renderer/metal/api.rs`, add the subset of Metal values needed
   for upstream attachment blending:

   ```rust
   pub(crate) enum MetalBlendFactor {
       One,
       OneMinusSourceAlpha,
   }

   pub(crate) enum MetalBlendOperation {
       Add,
   }
   ```

   Use raw values that match upstream `metal/api.zig` / Apple Metal:
   - `MetalBlendFactor::One = 1`;
   - `MetalBlendFactor::OneMinusSourceAlpha = 5`;
   - `MetalBlendOperation::Add = 0`.

   Add raw-value tests.

2. Add pipeline attachment value types.

   Extend `roastty/src/renderer/metal/pipeline.rs` with internal value types:

   ```rust
   pub(crate) struct MetalPipelineAttachmentOptions {
       pub(crate) pixel_format: MetalPixelFormat,
       pub(crate) blending_enabled: bool,
   }

   pub(crate) struct MetalPipelineAttachmentDescriptor {
       pub(crate) pixel_format: MetalPixelFormat,
       pub(crate) blending_enabled: bool,
       pub(crate) blend: Option<MetalBlendDescriptor>,
   }

   pub(crate) struct MetalBlendDescriptor {
       pub(crate) rgb_operation: MetalBlendOperation,
       pub(crate) alpha_operation: MetalBlendOperation,
       pub(crate) source_rgb_factor: MetalBlendFactor,
       pub(crate) source_alpha_factor: MetalBlendFactor,
       pub(crate) destination_rgb_factor: MetalBlendFactor,
       pub(crate) destination_alpha_factor: MetalBlendFactor,
   }
   ```

   Add a helper:

   ```rust
   pub(crate) fn pipeline_attachment_descriptor(
       options: MetalPipelineAttachmentOptions,
   ) -> MetalPipelineAttachmentDescriptor
   ```

   Upstream `Attachment.blending_enabled` defaults to `true`. This experiment
   may keep `MetalPipelineAttachmentOptions.blending_enabled` explicit because
   it is only a value-layer slice, but the later standard pipeline option
   builder must preserve the upstream default.

   Behavior:
   - always copy `pixel_format`;
   - always copy `blending_enabled`;
   - if `blending_enabled == true`, include the upstream premultiplied-alpha
     blend descriptor;
   - if `blending_enabled == false`, set `blend = None`.

3. Add tests.

   Add pure Rust tests proving:
   - blend factor raw values match upstream;
   - blend operation raw values match upstream;
   - enabled attachments produce the exact premultiplied-alpha blend descriptor;
   - disabled attachments preserve pixel format and disabled flag but have no
     blend descriptor;
   - multiple pixel formats can flow through unchanged, for example `Rgba8Unorm`
     and `Bgra8Unorm`.

   Do not create Metal devices, shader libraries, `MTLRenderPipelineDescriptor`,
   or pipeline state.

4. Verification commands.

   Run:

   ```bash
   cargo fmt -- roastty/src/renderer/metal/api.rs roastty/src/renderer/metal/pipeline.rs
   cargo test -p roastty renderer::metal::pipeline
   cargo test -p roastty renderer::metal::api
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
   git diff --check
   ```

   `cargo fmt` is required for Rust edits; accept formatter output as-is.

## Non-Negotiable Invariants

- Do not create Objective-C `MTLRenderPipelineDescriptor`,
  `MTLRenderPipelineState`, or `MTLVertexDescriptor` objects in this experiment.
- Do not compile shader libraries or add embedded shader source.
- Do not add render passes, command encoders, command buffers, IOSurface,
  CAMetalLayer, Swift integration, or public C ABI.
- Do not modify image upload, image draw, texture, buffer, or vertex descriptor
  semantics from Experiments 206-211.
- Do not modify vendored Ghostty source.
- Do not expose `ghostty_*` symbols or comments in the Roastty public ABI.

## Pass Criteria

- Roastty has internal pipeline attachment value types matching upstream
  `Pipeline.zig` attachment behavior.
- Enabled attachments produce the exact upstream premultiplied-alpha blend
  configuration.
- Disabled attachments do not carry blend settings.
- Tests cover raw Metal blend enum values and attachment descriptor behavior.
- Full verification passes, including the public no-`ghostty` gate.

## Failure Criteria

- The experiment creates real Metal pipeline objects instead of staying at the
  value layer.
- The enabled blend descriptor differs from upstream premultiplied-alpha
  settings.
- Disabled attachments retain stale blend settings.
- Existing renderer image, texture, buffer, or vertex descriptor tests regress.

## Result

Not run yet.

## Conclusion

Pending.
