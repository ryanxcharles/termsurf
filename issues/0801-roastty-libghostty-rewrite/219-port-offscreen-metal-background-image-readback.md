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

# Experiment 219: Port Offscreen Metal Background Image Readback

## Description

Experiment 218 added render-pass texture binding and proved the production
`image` shader by offscreen read-back. That test proved fragment-stage texture
sampling and structurally required vertex-stage texture binding, but the
production `image_vertex` function declares `texture(0)` without reading it.

The next production shader, `bg_image`, does read the texture in the vertex
stage:

```metal
float2 tex_size = float2(image.get_width(), image.get_height());
```

and then uses that texture size to compute the destination rectangle,
positioning, scaling, and fragment texture coordinates. This makes `bg_image`
the first automated proof that Roastty's render-pass texture binding works for
both vertex and fragment stages.

This experiment should not add any new generic render-pass resource class.
Experiment 218 already added texture binding; `bg_image` uses a `constexpr`
sampler just like `image`, so sampler-state plumbing is still out of scope.

This remains offscreen and automated. It should not add text rendering, glyph
atlas textures, explicit sampler binding, custom shaders, IOSurface,
CAMetalLayer, Swift integration, presentation, frame callbacks, draw threads,
renderer health, or public C ABI.

All public names must use Roastty naming.

## Changes

1. Add background-image draw helpers for tests.

   In `roastty/src/renderer/metal/render_pass.rs`, add test helpers that use the
   existing production types:
   - `BgImageVertex` from `roastty/src/renderer/shader.rs`;
   - `BgImageInfo`;
   - `BgImagePosition`;
   - `BgImageFit`;
   - `MetalStandardPipelines::bg_image`;
   - `MetalRenderPassStep.textures`;
   - `MetalPrimitiveType::Triangle`.

   The helper should upload one `BgImageVertex` with
   `MetalBuffer::init_fill(...)` using `MetalStorageMode::Shared`, bind it as
   `buffers[0]`, bind the uploaded image as `textures[0]`, bind uniforms at
   Metal buffer index `1`, and draw the full-screen triangle with
   `vertex_count = 3`.

   Reuse the offscreen render-target and uniform helpers from Experiment 218
   where possible. `bg_image_vertex` emits its own full-screen clip-space
   triangle and does not use `projection_matrix`; these tests should depend on
   `screen_size`, `bg_color`, `BgImageVertex`, and texture binding, not on
   projection behavior. Do not duplicate large test setup blocks if a small
   shared helper keeps the tests clearer.

2. Add live production `bg_image` read-back tests.

   Add tests in `roastty/src/renderer/metal/render_pass.rs`.

   Required tests:
   - `bg_image_render_pass_draws_texture_over_background`:
     - create `MetalStandardPipelines`;
     - create a 2x2 `ImageTextureFormat::Rgba` image texture with endpoint-only
       opaque RGBA texels;
     - create a 2x2 BGRA8 unorm shared render target;
     - create uniforms for a 2x2 screen with `bg_color = [0, 0, 0, 255]`,
       `use_display_p3 = true`, and `use_linear_blending = false`;
     - create one `BgImageVertex` with:
       - `opacity = 1.0`;
       - `info = BgImageInfo::new(BgImagePosition::TopLeft, BgImageFit::Stretch, false)`;
     - draw `pipelines.bg_image` with `MetalPrimitiveType::Triangle`,
       `vertex_count = 3`, and `instance_count = 1`;
     - read back the 2x2 target after `commit_and_wait(...)`;
     - verify exact BGRA byte output for the texture over the black background.

   - `bg_image_none_fit_uses_vertex_texture_size_for_placement`:
     - create a 2x2 opaque endpoint-only RGBA image texture;
     - create a 4x4 BGRA8 unorm shared render target;
     - create uniforms for a 4x4 screen with opaque black `bg_color`;
     - create one `BgImageVertex` with:
       - `opacity = 1.0`;
       - `info = BgImageInfo::new(BgImagePosition::MiddleCenter, BgImageFit::None, false)`;
     - draw `pipelines.bg_image`;
     - verify the image occupies only the centered 2x2 rectangle and the
       surrounding pixels are opaque black.

     This is the load-bearing test for vertex-stage texture use: `BgImageFit`
     `None` keeps the destination size equal to `image.get_width()` /
     `image.get_height()`, and `MiddleCenter` positions that texture-size-based
     rectangle inside the larger screen. If the texture is not bound in the
     vertex stage, this placement should not produce the expected centered 2x2
     result.

   - `bg_image_zero_instance_step_does_not_bind_or_draw`:
     - clear the render target to one color;
     - provide a valid `BgImageVertex` buffer and image texture with a different
       endpoint-only color;
     - draw `pipelines.bg_image` with `instance_count = 0`;
     - verify the clear color remains.

   Use `MetalPixelFormat::Bgra8Unorm` and `MetalStorageMode::Shared` for all
   render-target read-back tests.

3. Keep color expectations deterministic.

   Use `ImageTextureFormat::Rgba` without sRGB for uploaded test textures and
   `MetalPixelFormat::Bgra8Unorm` for render targets. Keep
   `use_display_p3 = true` and `use_linear_blending = false`.

   The production `bg_image_fragment` calls `unlinearize(rgba)` when
   `use_linear_blending = false`, then premultiplies and composites over a fully
   opaque version of `uniforms.bg_color`. Therefore:
   - use only endpoint texture channel values `0` and `255`;
   - keep uploaded texture alpha at `255` for pixels expected to show image
     color;
   - use opaque black `bg_color = [0, 0, 0, 255]` for placement tests so
     out-of-image pixels are deterministic opaque black.

   Do not loosen exact byte equality silently. If exact read-back differs,
   record the observed bytes and either adjust the test data/layout to avoid the
   ambiguity or close the experiment as Partial/Fail.

4. Verification commands.

   Run:

   ```bash
   cargo fmt -- roastty/src/renderer/metal/render_pass.rs
   cargo test -p roastty renderer::metal::render_pass
   cargo test -p roastty renderer::metal::shaders
   cargo test -p roastty renderer::metal::pipeline
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/metal; then exit 1; else exit 0; fi
   git diff --check
   ```

   `cargo fmt` is required for Rust edits; accept formatter output as-is.

## Non-Negotiable Invariants

- Do not add sampler binding.
- Do not add text rendering, glyph atlas textures, custom shaders, postprocess
  shaders, IOSurface, CAMetalLayer, Swift integration, presentation, frame
  callbacks, draw threads, renderer health, or public C ABI.
- Do not change production shader source.
- Do not modify vendored Ghostty source.
- Do not weaken Experiment 216's `bg_color`, Experiment 217's `cell_bg`, or
  Experiment 218's `image` tests.
- Do not introduce public or app-facing `ghostty_*` names in Roastty.

## Pass Criteria

- The production `bg_image` shader draws an uploaded texture into an offscreen
  render target.
- Read-back proves exact BGRA output for a full-screen/stretch background-image
  draw.
- Read-back proves `BgImageFit::None` plus `BgImagePosition::MiddleCenter` uses
  the texture dimensions in the vertex stage to place a smaller image inside a
  larger target.
- Zero-instance behavior remains no-draw.
- Full verification passes, including both no-`ghostty` gates.

## Failure Criteria

- The experiment changes shader source or uses a fake fragment shader instead of
  production `bg_image`.
- The background image texture is bound through an ad hoc one-off path instead
  of the render-pass texture mapping from Experiment 218.
- The test only checks command encoding and does not read pixels back.
- The experiment adds sampler binding, text/glyph rendering, presentation,
  Swift/app integration, or public C ABI.
- Existing Metal `bg_color`, `cell_bg`, `image`, shader-library, pipeline,
  texture, buffer, image, or full Roastty tests regress.

## Result

**Result:** Pass

Experiment 219 added production `bg_image` offscreen read-back tests using the
render-pass texture binding from Experiment 218.

Implementation details:

- added test helpers for `BgImageVertex` buffers and opaque-background uniforms;
- drew `pipelines.bg_image` with `MetalPrimitiveType::Triangle` and
  `vertex_count = 3`;
- bound `BgImageVertex` as `buffers[0]`;
- bound uploaded RGBA textures as `textures[0]`;
- kept uniforms at Metal buffer index `1`;
- used endpoint-only opaque RGBA texture colors and opaque black background
  colors for deterministic exact-byte read-back.

The new tests cover:

- full-screen/stretch background-image read-back over an opaque black
  background;
- `BgImageFit::None` plus `BgImagePosition::MiddleCenter`, proving that
  vertex-stage texture size from `image.get_width()` / `image.get_height()`
  affects placement;
- zero-instance no-draw behavior for `bg_image`.

Verification passed:

```bash
cargo fmt -- roastty/src/renderer/metal/render_pass.rs
cargo test -p roastty renderer::metal::render_pass
cargo test -p roastty renderer::metal::shaders
cargo test -p roastty renderer::metal::pipeline
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/metal; then exit 1; else exit 0; fi
git diff --check
```

Observed results:

- `renderer::metal::render_pass`: 14 passed.
- `renderer::metal::shaders`: 8 passed.
- `renderer::metal::pipeline`: 17 passed.
- Full `roastty`: 2198 library tests passed, plus the C ABI harness passed.
- Both no-`ghostty` gates passed.
- `git diff --check` passed.

Codex reviewed the completed implementation and found no blocking issues. It
confirmed that the `bg_image` tests use the production pipeline, bind texture
resources through the shared render-pass mapping, prove vertex-stage texture
size use through the centered `Fit::None` case, and keep exact BGRA expectations
sound for endpoint-only opaque inputs.

Residual scope: partial alpha, opacity less than `1.0`, repeat, contain, and
cover behavior are not covered in this experiment. That is acceptable because
the target here was production `bg_image` offscreen read-back plus vertex-stage
texture-size proof, not the full background-image feature matrix.

## Conclusion

Roastty now has automated production-shader read-back coverage for background
images. This completes the immediate proof that the render-pass texture mapping
from Experiment 218 reaches both shader stages: `image` proved fragment-stage
sampling, and `bg_image` proved vertex-stage texture-size use.

The next renderer slice can move toward terminal text rendering and glyph atlas
resources, or it can broaden background-image coverage for opacity, repeat,
contain, and cover if that feature matrix becomes more important before text.
