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

# Experiment 222: Port Offscreen Metal Cell Text Cursor Readback

## Description

Experiments 220 and 221 proved the production Metal `cell_text` atlas sampling
paths for grayscale glyph masks and color glyph textures. The next production
`cell_text` behavior is the cursor color override performed in
`cell_text_vertex`.

The shader changes a non-cursor glyph's text color to `uniforms.cursor_color`
when the glyph's grid position matches `uniforms.cursor_pos`. It also supports
wide cursors: when `uniforms.cursor_wide` is true, the cell immediately to the
right of `cursor_pos` is treated as part of the cursor. Glyphs marked with
`CellTextFlags::new(_, true)` are cursor glyphs and must not receive this
override.

This experiment should prove those production cursor-color rules with
deterministic offscreen read-back tests. It should not add font loading, glyph
rasterization, atlas allocation, cursor shape rendering, selection rendering,
presentation, Swift/app integration, or public C ABI. It is still a renderer
proof only.

All public names must use Roastty naming.

## Changes

1. Extend cell-text test helpers for cursor-specific uniforms and flags.

   In `roastty/src/renderer/metal/render_pass.rs`, reuse the helpers from
   Experiments 220 and 221 where possible:
   - `cell_text_uniforms(...)`;
   - `cell_text_vertex_buffer(...)`;
   - `cell_text_vertex_with_atlas(...)`;
   - `grayscale_atlas_texture(...)`;
   - `dummy_color_atlas_texture(...)`;
   - `cell_bg_buffer(...)`.

   Add a small helper or direct setup in tests that can:
   - set `uniforms.cursor_pos`;
   - set `uniforms.cursor_color`;
   - set `uniforms.bools.cursor_wide`;
   - create a `CellTextVertex` with `CellTextFlags::new(false, true)` for the
     cursor-glyph exemption test.

   The draw path must remain the production render-pass path:
   - text vertex buffer through `buffers[0]`, mapping to Metal buffer `0`;
   - uniforms through Metal buffer `1`;
   - `CellBg` grid through `buffers[1]`, mapping to Metal buffer `2`;
   - grayscale atlas through `textures[0]`;
   - dummy color atlas through `textures[1]`;
   - `pipelines.cell_text`;
   - `MetalPrimitiveType::TriangleStrip`;
   - `vertex_count = 4`.

2. Add production `cell_text` cursor override read-back tests.

   Add tests in `roastty/src/renderer/metal/render_pass.rs`.

   Required tests:
   - `cell_text_cursor_pos_overrides_non_cursor_glyph_color`:
     - create `MetalStandardPipelines`;
     - create a 1x1 grayscale atlas texture with byte `[255]`;
     - create a dummy 1x1 RGBA color atlas texture;
     - create a 1x1 BGRA8 unorm shared render target, cleared transparent;
     - create uniforms for a 1x1 screen with:
       - `screen_size = [1, 1]`;
       - `grid_size = [1, 1]`;
       - `cell_size = [1.0, 1.0]`;
       - transparent `bg_color`;
       - `min_contrast = 0.0`;
       - `cursor_pos = [0, 0]`;
       - `cursor_color = [0, 255, 0, 255]`;
       - `cursor_wide = false`;
       - `use_display_p3 = true`;
       - `use_linear_blending = false`;
       - `use_linear_correction = false`;
     - create one transparent `CellBg`;
     - create one non-cursor grayscale `CellTextVertex` at `grid_pos = [0, 0]`
       with red vertex color;
     - draw `pipelines.cell_text`;
     - read back the target after `commit_and_wait(...)`;
     - verify the output is green, not red.

   - `cell_text_cursor_glyph_flag_preserves_vertex_color`:
     - use the same 1x1 setup;
     - set `cursor_pos = [0, 0]` and green `cursor_color`;
     - create one grayscale `CellTextVertex` at `grid_pos = [0, 0]` with red
       vertex color and `flags = CellTextFlags::new(false, true)`;
     - verify the output is red, proving cursor glyphs do not receive the
       non-cursor override.

   - `cell_text_wide_cursor_overrides_second_cell`:
     - create a 2x1 screen with `grid_size = [2, 1]` and
       `cell_size = [1.0, 1.0]`;
     - create `CellBg` entries for both cells;
     - set `cursor_pos = [0, 0]`, `cursor_wide = true`, and green
       `cursor_color`;
     - draw one non-cursor grayscale glyph at `grid_pos = [1, 0]` with red
       vertex color;
     - verify the second pixel is green and the first pixel remains transparent.

   Use endpoint-only colors and a full-alpha grayscale mask so exact BGRA output
   remains deterministic.

3. Keep scope narrow.

   This experiment proves color override rules only. It does not need to prove
   cursor shape, cursor blink, cursor rendering order, reverse video, selection,
   min-contrast correction, or any text layout behavior. Those can be separate
   renderer or presentation experiments.

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
  allocation, or cursor shape rendering.
- Do not add min-contrast, selection rendering, reverse-video presentation,
  ligature, or multi-cell text layout behavior beyond the wide-cursor cell match
  needed by this experiment.
- Do not add sampler binding; the production shader uses a `constexpr sampler`.
- Do not add IOSurface, CAMetalLayer, Swift integration, presentation, frame
  callbacks, draw threads, renderer health, or public C ABI.
- Do not change production shader source.
- Do not modify vendored Ghostty source.
- Do not weaken Experiments 216-221 render-pass read-back tests.
- Do not introduce public or app-facing `ghostty_*` names in Roastty.

## Pass Criteria

- Read-back proves a non-cursor grayscale glyph at `cursor_pos` uses
  `cursor_color` instead of vertex text color.
- Read-back proves `CellTextFlags::new(false, true)` preserves the cursor
  glyph's own vertex color at `cursor_pos`.
- Read-back proves `cursor_wide = true` applies the cursor override to the cell
  immediately to the right of `cursor_pos`.
- The tests bind vertex buffer `0`, uniforms `1`, background colors `2`,
  grayscale texture `0`, and dummy color texture `1` through the existing
  render-pass mapping.
- Full verification passes, including both no-`ghostty` gates.

## Failure Criteria

- The experiment changes shader source or uses a fake fragment shader instead of
  production `cell_text`.
- The cursor behavior is tested by inspecting encoded commands only instead of
  reading pixels back.
- The glyph textures or background color buffer are bound through an ad hoc path
  instead of the existing render-pass mappings.
- The experiment grows into cursor shape rendering, font loading, glyph
  rasterization, atlas allocation, min contrast, selection rendering,
  presentation, Swift/app integration, or public C ABI.
- Existing Metal `bg_color`, `cell_bg`, `image`, `bg_image`, `cell_text`
  grayscale/color, shader-library, pipeline, texture, buffer, image, or full
  Roastty tests regress.

## Result

**Result:** Pass

Implemented production `cell_text` cursor-color read-back tests in
`roastty/src/renderer/metal/render_pass.rs`.

The implementation added a test-only `cell_text_cursor_uniforms(...)` helper
that starts from the existing cell-text uniforms and sets:

- `cursor_pos`;
- `cursor_color`;
- `cursor_wide`.

The new tests prove:

- a non-cursor grayscale glyph at `cursor_pos` renders with `cursor_color`
  instead of its red vertex text color;
- a glyph marked with `CellTextFlags::new(false, true)` preserves its own red
  vertex color at `cursor_pos`;
- `cursor_wide = true` applies the cursor-color override to the cell immediately
  to the right of `cursor_pos`;
- `cursor_wide = false` does not override the cell immediately to the right of
  `cursor_pos`.

The render-pass step binds text vertex data through `buffers[0]`, uniforms
through Metal buffer `1`, the `CellBg` grid through `buffers[1]` so the existing
mapping sends it to Metal buffer `2`, the grayscale atlas through `textures[0]`,
and the dummy color atlas through `textures[1]`.

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

- `renderer::metal::render_pass`: 23 passed.
- `renderer::metal::shaders`: 8 passed.
- `renderer::metal::pipeline`: 17 passed.
- `renderer::shader`: 9 passed.
- Full `roastty`: 2207 unit tests passed, plus the C ABI harness passed.
- Both no-`ghostty` gates passed.
- `git diff --check` passed.

Codex reviewed the completed implementation, identified one real low-risk
coverage gap, and the implementation was updated with a `cursor_wide = false`
second-cell negative test. Codex reviewed the updated diff again and found no
remaining issues.

## Conclusion

Roastty now has automated production-shader read-back coverage for the
`cell_text` cursor-color override rules. The renderer proof covers ordinary
cursor-position override, cursor-glyph exemption, wide-cursor second-cell
override, and the corresponding non-wide second-cell non-override.

This remains renderer proof work. It intentionally leaves cursor shape
rendering, cursor blink, render ordering with selection/reverse-video behavior,
font loading, glyph rasterization, real atlas allocation, presentation, and
public C ABI integration for later experiments.
