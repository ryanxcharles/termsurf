# Experiment 221: Port Offscreen Metal Cell Text Color Readback

## Description

Experiment 220 proved the production Metal `cell_text` grayscale atlas path by
drawing synthetic grayscale glyph masks into an offscreen target and reading
back exact pixels.

The next renderer proof is the other production `cell_text_fragment` branch:
`ATLAS_COLOR`. This branch samples `textureColor` at Metal texture index `1`,
uses pixel-coordinate sampling, ignores the per-vertex text color, and returns
the sampled premultiplied color glyph data. When `use_linear_blending = false`,
the shader unlinearizes the sampled color before returning it.

This experiment should prove color-atlas sampling with deterministic endpoint
pixels. It should not add font loading, glyph rasterization, atlas allocation,
emoji shaping, color emoji selection, min-contrast correction, cursor glyph
behavior, presentation, Swift/app integration, or public C ABI. It is still a
renderer proof only.

All public names must use Roastty naming.

## Changes

1. Extend cell-text test helpers for color atlas vertices.

   In `roastty/src/renderer/metal/render_pass.rs`, reuse the helpers from
   Experiment 220 where possible:
   - `cell_text_uniforms(...)`;
   - `cell_text_vertex_buffer(...)`;
   - `grayscale_atlas_texture(...)`;
   - `image_texture(...)`;
   - `cell_bg_buffer(...)`.

   Add or adjust a small helper so tests can create `CellTextVertex` values with
   `atlas = CellTextAtlas::Color`. Do not remove or weaken the grayscale helper
   behavior from Experiment 220.

   The draw path must remain the production render-pass path:
   - text vertex buffer through `buffers[0]`, mapping to Metal buffer `0`;
   - uniforms through Metal buffer `1`;
   - `CellBg` grid through `buffers[1]`, mapping to Metal buffer `2`;
   - dummy grayscale atlas through `textures[0]`;
   - color atlas through `textures[1]`;
   - `pipelines.cell_text`;
   - `MetalPrimitiveType::TriangleStrip`;
   - `vertex_count = 4`.

2. Add production `cell_text` color-atlas read-back tests.

   Add tests in `roastty/src/renderer/metal/render_pass.rs`.

   Required tests:
   - `cell_text_color_render_pass_draws_color_atlas_pixels`:
     - create `MetalStandardPipelines`;
     - create a 1x1 dummy grayscale atlas texture;
     - create a 2x2 RGBA color atlas texture with endpoint-only opaque texels,
       for example red, green, blue, and white;
     - create a 2x2 BGRA8 unorm shared render target, cleared transparent;
     - create uniforms for a 2x2 screen with:
       - `screen_size = [2, 2]`;
       - `grid_size = [1, 1]`;
       - `cell_size = [2.0, 2.0]`;
       - zero `grid_padding`;
       - `padding_extend = 0`;
       - `bg_color = [0, 0, 0, 0]`;
       - `min_contrast = 0.0`;
       - `use_display_p3 = true`;
       - `use_linear_blending = false`;
       - `use_linear_correction = false`;
       - `cursor_pos` outside the tested grid;
     - create a one-cell transparent `CellBg` grid;
     - create one `CellTextVertex`:
       - `glyph_pos = [0, 0]`;
       - `glyph_size = [2, 2]`;
       - `bearings = [0, 2]`;
       - `grid_pos = [0, 0]`;
       - `color` set to a value that is not the expected output, proving the
         color branch does not use vertex text color;
       - `atlas = CellTextAtlas::Color`;
       - `flags = CellTextFlags::new(false, false)`;
       - zero padding;
     - draw `pipelines.cell_text`;
     - read back the 2x2 target after `commit_and_wait(...)`;
     - verify exact BGRA output matching the color atlas texels.

   - `cell_text_color_uses_glyph_pos_and_ignores_grayscale_mask`:
     - create a 1x1 grayscale atlas with byte `[0]`;
     - create a 2x1 RGBA color atlas with two opaque endpoint texels, for
       example red then blue;
     - create a 1x1 transparent render target;
     - create uniforms for a 1x1 screen with `grid_size = [1, 1]` and
       `cell_size = [1.0, 1.0]`, with `cursor_pos` outside the tested grid;
     - create one `CellTextVertex`:
       - `glyph_pos = [1, 0]`;
       - `glyph_size = [1, 1]`;
       - `bearings = [0, 1]`;
       - `grid_pos = [0, 0]`;
       - `color = [255, 0, 0, 255]`;
       - `atlas = CellTextAtlas::Color`;
     - verify the output is the second color-atlas texel, not the vertex color
       and not transparent from the zero grayscale mask.

   These tests do not need another zero-instance case; Experiment 220 already
   covers `cell_text` zero-instance behavior through the same render-pass path.

3. Keep color expectations deterministic.

   Use endpoint-only opaque color atlas texels (`0` / `255`, alpha `255`). Avoid
   partially transparent color glyph data in this experiment because the
   production branch divides by alpha before unlinearizing when
   `use_linear_blending = false`; partial alpha and zero alpha need their own
   focused experiment if they become important.

   Keep:
   - `min_contrast = 0.0`;
   - `use_linear_correction = false`;
   - `cursor_pos` outside the tested grid;
   - transparent `CellBg`;
   - transparent global background.

   If exact bytes differ, do not loosen the tests silently. Record the observed
   bytes and either adjust the deterministic setup or close the experiment as
   Partial/Fail.

4. Verification commands.

   Run:

   ```bash
   cargo fmt -- roastty/src/renderer/metal/render_pass.rs
   cargo test -p roastty renderer::metal::render_pass
   cargo test -p roastty renderer::metal::shaders
   cargo test -p roastty renderer::metal::pipeline
   cargo test -p roastty renderer::shader
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/metal; then exit 1; else exit 0; fi
   git diff --check
   ```

   `cargo fmt` is required for Rust edits; accept formatter output as-is.

## Non-Negotiable Invariants

- Do not add font loading, font discovery, shaping, glyph rasterization, atlas
  allocation, or emoji selection.
- Do not add min-contrast, cursor-glyph, linear-correction, ligature, or
  multi-cell text behavior.
- Do not add sampler binding; the production shader uses a `constexpr sampler`.
- Do not add IOSurface, CAMetalLayer, Swift integration, presentation, frame
  callbacks, draw threads, renderer health, or public C ABI.
- Do not change production shader source.
- Do not modify vendored Ghostty source.
- Do not weaken Experiments 216-220 render-pass read-back tests.
- Do not introduce public or app-facing `ghostty_*` names in Roastty.

## Pass Criteria

- The production `cell_text` shader draws color-atlas texels into an offscreen
  render target.
- Read-back proves exact BGRA output for endpoint-only opaque color atlas data.
- Read-back proves `glyph_pos` selects from `textureColor` and does not use the
  grayscale mask.
- Read-back proves the color branch ignores per-vertex text color.
- The tests bind vertex buffer `0`, uniforms `1`, background colors `2`, dummy
  grayscale texture `0`, and color texture `1` through the existing render-pass
  mapping.
- Full verification passes, including both no-`ghostty` gates.

## Failure Criteria

- The experiment changes shader source or uses a fake fragment shader instead of
  production `cell_text`.
- The color atlas texture or background color buffer is bound through an ad hoc
  path instead of the existing render-pass mappings.
- The test only checks command encoding and does not read pixels back.
- The experiment grows into font loading, glyph rasterization, atlas allocation,
  color emoji selection, min contrast, cursor rendering, presentation, Swift/app
  integration, or public C ABI.
- Existing Metal `bg_color`, `cell_bg`, `image`, `bg_image`, `cell_text`
  grayscale, shader-library, pipeline, texture, buffer, image, or full Roastty
  tests regress.

## Result

**Result:** Pass

Implemented production `cell_text` color-atlas read-back tests in
`roastty/src/renderer/metal/render_pass.rs`.

The implementation added a small test helper,
`cell_text_vertex_with_atlas(...)`, so tests can select `CellTextAtlas::Color`
while preserving the grayscale helper from Experiment 220.

The new tests prove:

- a 2x2 opaque endpoint-only RGBA color atlas renders exact BGRA output through
  `pipelines.cell_text`;
- color atlas sampling uses `textureColor` at texture index `1`;
- `glyph_pos = [1, 0]` selects the second color-atlas texel;
- a zero grayscale atlas mask does not affect the color-atlas branch;
- the color-atlas branch ignores the per-vertex text color.

The render-pass step binds text vertex data through `buffers[0]`, uniforms
through Metal buffer `1`, the `CellBg` grid through `buffers[1]` so the existing
mapping sends it to Metal buffer `2`, the dummy grayscale atlas through
`textures[0]`, and the color atlas through `textures[1]`.

Verification passed:

```bash
cargo fmt -- roastty/src/renderer/metal/render_pass.rs
cargo test -p roastty renderer::metal::render_pass
cargo test -p roastty renderer::metal::shaders
cargo test -p roastty renderer::metal::pipeline
cargo test -p roastty renderer::shader
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/metal; then exit 1; else exit 0; fi
git diff --check
```

Observed results:

- `renderer::metal::render_pass`: 19 passed.
- `renderer::metal::shaders`: 8 passed.
- `renderer::metal::pipeline`: 17 passed.
- `renderer::shader`: 9 passed.
- Full `roastty`: 2203 unit tests passed, plus the C ABI harness passed.
- Both no-`ghostty` gates passed.
- `git diff --check` passed.

Codex reviewed the completed implementation and found no blocking issues. It
confirmed the resource binding path, the load-bearing color atlas tests, the
grayscale-mask independence proof, and the deterministic endpoint-only color
setup.

## Conclusion

Roastty now has automated production-shader read-back coverage for both
`cell_text` atlas branches. Experiment 220 proved grayscale glyph masks;
Experiment 221 proved color atlas sampling at texture index `1`, glyph-position
selection, grayscale-mask independence, and vertex-color independence.

This is still renderer proof work. It intentionally leaves font loading, glyph
rasterization, real atlas allocation, color emoji selection, partial-alpha color
glyphs, min-contrast behavior, cursor glyph behavior, presentation, and public C
ABI integration for later experiments.
