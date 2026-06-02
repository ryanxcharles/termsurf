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

# Experiment 218: Port Offscreen Metal Image Texture Readback

## Description

Experiment 217 proved the production `cell_bg` shader with a real terminal
content buffer bound through the render-pass step. The next production resource
class is a texture.

Upstream `vendor/ghostty/src/renderer/metal/RenderPass.zig` supports textures on
each render-pass step:

```zig
textures: []const ?Texture = &.{},
```

and binds every present texture to both vertex and fragment texture indices:

```zig
for (s.textures, 0..) |t, i| if (t) |tex| {
    setVertexTexture(..., i);
    setFragmentTexture(..., i);
};
```

Roastty already has the production `image` pipeline description, `ImageVertex`
layout, `MetalTexture` upload path, and offscreen render-pass read-back. This
experiment connects those pieces by adding the supported texture-binding subset
to `MetalRenderPassStep`, adding the upstream `triangle_strip` primitive needed
for image quads, and proving the production `image` shader by drawing an
uploaded texture into an offscreen render target.

This remains offscreen and automated. It should not add samplers, background
images, text rendering, glyph atlases, custom shaders, IOSurface, CAMetalLayer,
Swift integration, presentation, frame callbacks, draw threads, renderer health,
or public C ABI.

All public names must use Roastty naming.

## Changes

1. Add the `triangle_strip` primitive.

   In `roastty/src/renderer/metal/api.rs`, extend `MetalPrimitiveType` with:

   ```rust
   TriangleStrip = 4,
   ```

   matching `vendor/ghostty/src/renderer/metal/api.zig` and Apple Metal's
   `MTLPrimitiveTypeTriangleStrip` value. Update `raw()` / `to_objc()` coverage
   and tests.

   Do not add unrelated primitive types in this experiment.

2. Extend render-pass texture binding.

   In `roastty/src/renderer/metal/render_pass.rs`, extend `MetalRenderPassStep`
   with:

   ```rust
   pub(crate) textures: &'a [Option<&'a MetalTexture>],
   ```

   Behavior must match the supported upstream shape:
   - preserve the zero-instance early return before any pipeline, buffer,
     texture, uniform, or draw work;
   - bind every present texture to both vertex and fragment texture index `i`;
   - preserve Experiment 217's buffer mapping and uniform binding;
   - preserve the current `bg_color` and `cell_bg` behavior when `textures` is
     empty.

   Add a small helper if needed to keep texture index mapping easy to read. Do
   not add sampler binding in this experiment. The production `image` shader
   uses a `constexpr sampler`, so proving image texture binding does not require
   sampler-state plumbing yet.

3. Add an image draw helper for tests.

   Use the existing `ImageVertex` shader payload type from
   `roastty/src/renderer/shader.rs`.

   The test helper should:
   - upload one `ImageVertex` with `MetalBuffer::init_fill(...)` using
     `MetalStorageMode::Shared`;
   - bind that vertex buffer as `buffers[0]`, so the render-pass mapping binds
     it to Metal buffer index `0`;
   - bind the uploaded image texture as `textures[0]`;
   - bind uniforms at Metal buffer index `1`;
   - draw with `MetalPrimitiveType::TriangleStrip`, `vertex_count = 4`, and
     `instance_count = 1`.

4. Add live production `image` read-back tests.

   Add tests in `roastty/src/renderer/metal/render_pass.rs`.

   Required tests:
   - `image_render_pass_draws_uploaded_texture_pixels`:
     - create `MetalStandardPipelines`;
     - create a 2x2 `ImageTextureFormat::Rgba` image texture with
       `MetalStorageMode::Shared`;
     - use distinct opaque RGBA texels whose channels are only endpoint values
       `0` or `255`, for example red, green, blue, and white;
     - create a 2x2 BGRA8 unorm shared render target;
     - create `MetalUniforms::test_with_grid(...)` with:
       - `screen_size = [2, 2]`;
       - `grid_size = [2, 2]`;
       - `cell_size = [1.0, 1.0]`;
       - zero `grid_padding`;
       - `padding_extend = 0`;
       - `bg_color = [0, 0, 0, 0]`;
       - `use_display_p3 = true`;
       - `use_linear_blending = false`;
     - create one `ImageVertex`:
       - `grid_pos = [0.0, 0.0]`;
       - `cell_offset = [0.0, 0.0]`;
       - `source_rect = [0.0, 0.0, 2.0, 2.0]`;
       - `dest_size = [2.0, 2.0]`;
     - draw `pipelines.image` with a triangle strip;
     - read back the 2x2 target after `commit_and_wait(...)`;
     - verify exact BGRA byte output for each texel.

   - `image_render_pass_respects_cell_offset_and_dest_size`:
     - create a 1x1 opaque RGBA image texture using endpoint-only channel
       values;
     - create a 4x4 transparent render target;
     - use uniforms for a 4x4 screen and 1x1 cells;
     - create one `ImageVertex` with:
       - `grid_pos = [0.0, 0.0]`;
       - `cell_offset = [1.0, 1.0]`;
       - `source_rect = [0.0, 0.0, 1.0, 1.0]`;
       - `dest_size = [2.0, 2.0]`;
     - draw `pipelines.image`;
     - verify only the 2x2 rectangle at pixels `(1..3, 1..3)` is colored and all
       surrounding pixels remain transparent.

   - `image_zero_instance_step_does_not_bind_or_draw`:
     - clear the render target to one color;
     - provide a valid vertex buffer and texture with a different endpoint-only
       color;
     - draw `pipelines.image` with `instance_count = 0`;
     - verify the clear color remains.

   Use `MetalPixelFormat::Bgra8Unorm` and `MetalStorageMode::Shared` for all
   render-target read-back tests.

5. Keep color expectations deterministic.

   Use `ImageTextureFormat::Rgba` without sRGB for uploaded test textures and
   `MetalPixelFormat::Bgra8Unorm` for render targets. Keep
   `use_display_p3 = true` and `use_linear_blending = false`, matching the
   existing read-back tests' deterministic color-mode shape.

   The production `image_fragment` calls `unlinearize(rgba)` when
   `use_linear_blending = false`. Mid-range channel values therefore do not
   round-trip as uploaded bytes. The exact-byte tests must use only endpoint
   channel values `0` and `255`, where `unlinearize` is byte-stable. Do not use
   mid-range values unless the expected gamma conversion and rounding are
   explicitly computed and documented.

   If exact byte equality fails because the production shader's sampling path
   produces unavoidable interpolation or conversion differences, do not loosen
   the test silently. Record the exact observed bytes, explain why, and either:
   - adjust the test data/layout to avoid interpolation ambiguity; or
   - close the experiment as Partial/Fail and design the next experiment around
     the proven source of nondeterminism.

6. Be precise about what the read-back proves.

   The render-pass texture mapping must bind textures to both vertex and
   fragment stages because that is the upstream `RenderPass.zig` contract.
   However, the production `image` shader's vertex function declares
   `texture(0)` but does not currently read from it; the fragment function is
   what samples `texture(0)`.

   Therefore this experiment's read-back tests prove fragment-stage texture
   binding through production `image` rendering and structurally require
   vertex-stage binding in the render-pass helper. Do not claim the read-back
   itself proves vertex-stage texture consumption. The production `bg_image`
   shader will be the first later slice that can observe vertex-stage texture
   binding because its vertex function reads `image.get_width()` /
   `image.get_height()`.

7. Verification commands.

   Run:

   ```bash
   cargo fmt -- roastty/src/renderer/metal/api.rs roastty/src/renderer/metal/render_pass.rs
   cargo test -p roastty renderer::metal::api
   cargo test -p roastty renderer::metal::render_pass
   cargo test -p roastty renderer::metal::texture
   cargo test -p roastty renderer::metal::pipeline
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/metal; then exit 1; else exit 0; fi
   git diff --check
   ```

   `cargo fmt` is required for Rust edits; accept formatter output as-is.

## Non-Negotiable Invariants

- Do not add sampler binding.
- Do not add text rendering, glyph atlas textures, background-image rendering,
  custom shaders, postprocess shaders, IOSurface, CAMetalLayer, Swift
  integration, presentation, frame callbacks, draw threads, renderer health, or
  public C ABI.
- Do not change production shader source.
- Do not modify vendored Ghostty source.
- Do not weaken Experiment 216's `bg_color` tests or Experiment 217's `cell_bg`
  tests.
- Do not introduce public or app-facing `ghostty_*` names in Roastty.

## Pass Criteria

- `MetalPrimitiveType` includes the upstream-compatible `TriangleStrip` value.
- `MetalRenderPassStep` implements upstream-compatible texture index binding for
  the supported subset.
- The production `image` shader draws uploaded texture data into an offscreen
  render target.
- Read-back proves exact BGRA output for the full-texture draw.
- Read-back proves `cell_offset` / `dest_size` placement behavior.
- Zero-instance behavior remains no-draw.
- Full verification passes, including both no-`ghostty` gates.

## Failure Criteria

- The experiment changes shader source or uses a fake fragment shader instead of
  production `image`.
- The image texture is bound through an ad hoc one-off path instead of the
  render-pass texture mapping.
- The test only checks command encoding and does not read pixels back.
- The experiment adds sampler binding, background-image rendering, text/glyph
  rendering, presentation, Swift/app integration, or public C ABI.
- Existing Metal `bg_color`, `cell_bg`, shader-library, pipeline, texture,
  buffer, image, or full Roastty tests regress.

## Result

**Result:** Pass

Experiment 218 added the upstream-compatible `TriangleStrip` primitive and
extended `MetalRenderPassStep` with the supported texture-binding subset.

Implementation details:

- `MetalPrimitiveType::TriangleStrip` now maps to raw value `4` and
  `objc2_metal::MTLPrimitiveType::TriangleStrip`;
- each present render-pass texture binds to both vertex and fragment texture
  index `i`;
- the zero-instance early return remains before pipeline, buffer, texture,
  uniform, or draw work;
- existing `bg_color` and `cell_bg` steps now pass empty texture slices and keep
  their prior behavior.

The production `image` shader now has automated offscreen read-back coverage.
The tests upload RGBA textures, upload an `ImageVertex` buffer at `buffers[0]`,
bind the texture at `textures[0]`, bind uniforms at Metal buffer index `1`, draw
`pipelines.image` with `TriangleStrip`, and verify BGRA target pixels after
`commit_and_wait(...)`.

The image tests use endpoint-only texture channels (`0` / `255`) so exact byte
expectations are deterministic through the production shader's
`use_linear_blending = false` / `unlinearize(...)` path.

Verification passed:

```bash
cargo fmt -- roastty/src/renderer/metal/api.rs roastty/src/renderer/metal/render_pass.rs
cargo test -p roastty renderer::metal::api
cargo test -p roastty renderer::metal::render_pass
cargo test -p roastty renderer::metal::texture
cargo test -p roastty renderer::metal::pipeline
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/metal; then exit 1; else exit 0; fi
git diff --check
```

Observed results:

- `renderer::metal::api`: 20 passed.
- `renderer::metal::render_pass`: 11 passed.
- `renderer::metal::texture`: 10 passed.
- `renderer::metal::pipeline`: 17 passed.
- Full `roastty`: 2195 library tests passed, plus the C ABI harness passed.
- Both no-`ghostty` gates passed.
- `git diff --check` passed.

Codex reviewed the completed implementation and found no blocking issues. It
confirmed that the `TriangleStrip` value, texture binding, image read-back
tests, placement test, zero-instance behavior, and scope boundaries satisfy the
experiment. Codex also confirmed the intended proof boundary: this experiment's
read-back tests prove fragment-stage texture sampling through production
`image`; vertex-stage texture binding is structurally implemented but will be
observably tested by a later `bg_image` slice.

## Conclusion

Roastty now has the render-pass texture-binding primitive needed for production
image rendering. Together with Experiment 217's buffer mapping, the Metal
offscreen path can now bind vertex buffers, uniforms, content buffers, and
textures, then verify production shader output by exact pixel read-back.

The next renderer experiment can either exercise the production `bg_image`
shader, which will make vertex-stage texture consumption observable, or move
toward the next resource class needed for terminal text rendering. The
`bg_image` path is the natural continuation if we want to finish the generic
texture-binding proof before adding glyph atlas complexity.
