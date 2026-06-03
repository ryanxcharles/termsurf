+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 261: Colored glyph render — depth-4 P3 RGBA (sbix)

## Description

The final `renderGlyph` branch: rendering **color** (sbix/emoji) glyphs into a
depth-4 RGBA atlas. With color detection in place (Experiment 260),
`render_glyph` now routes a color glyph through a Display-P3,
premultiplied-first, 4-bytes-per-pixel bitmap context and writes it to a `Bgra`
atlas, with the sbix-specific whole-pixel quantization and the synthetic-bold /
thicken suppression that bitmap glyphs need.

### Upstream behavior (`font/face/coretext.zig`)

- The color descriptor (lines 416–432): non-color → `depth = 1`, `linearGray`,
  `ImageAlphaInfo.only`; color → `depth = 4`, `displayP3`,
  `byte_order_32_little | premultiplied_first`.
- Atlas-format check (lines 436–442): `atlas.format.depth() != color.depth` →
  `error.InvalidAtlasFormat`.
- sbix suppression: synthetic-bold rect growth is `if (!sbix)` (line 315);
  `canvas_padding` is `if (opts.thicken and !sbix)` (line 396).
- sbix whole-pixel quantization (lines 385–390), after the `dx` re-center:
  `width = cell_width - round(cell_width - width - x) - round(x)`,
  `height = cell_height - round(cell_height - height - y) - round(y)`,
  `x = round(x)`, `y = round(y)`.
- Color fill/stroke (lines 501–503): `setRGBFillColor(1,1,1,1)`,
  `setRGBStrokeColor(1,1,1,1)`.

Since Experiment 260 detects **only** sbix color (SVG deferred), every color
glyph here is an sbix glyph — so `is_color ⟺ sbix` in this port.

### Mono colorspace note (deliberate, documented)

Upstream's **mono** descriptor uses `linearGray` + `ImageAlphaInfo.only`; the
existing `draw_coverage` uses `device_gray` + `alphaNone`. Both yield valid
single-channel coverage, and at the default `thicken_strength = 255` they are
equivalent. Per the issue's "rasterization matches by construction" principle
(exact rasterizer bytes are not a fidelity-fixture requirement), this experiment
**keeps the mono path unchanged** and only adds the structurally-required color
path. Matching upstream's exact mono colorspace/alpha-info (and reconciling the
`thicken_strength` semantics under `alphaOnly`) is noted as a possible future
refinement, out of scope here.

### Rust mapping (`roastty/src/font/face/coretext.rs`)

1. **`draw_coverage`** gains a `color: bool` parameter and branches the context
   setup:
   - color:
     `colorspace = CGColorSpace::new_with_name(Some(kCGColorSpaceDisplayP3))`,
     `bytes_per_row = px_w * 4`,
     `bitmap_info = CGImageByteOrderInfo::Order32Little.0 | CGImageAlphaInfo::PremultipliedFirst.0`
     (`= 8194`), `buf = vec![0; px_w*px_h*4]`, fill+stroke via
     `set_rgb_fill_color(1,1,1,1)` / `set_rgb_stroke_color(1,1,1,1)`;
   - mono: unchanged (`device_gray`, `bitmap_info = 0`, `bytes_per_row = px_w`,
     `buf = px_w*px_h`, gray fill/stroke from `fill_gray`).
   - The antialiasing / subpixel / smoothing toggles and the CTM translate+scale
     are shared.
2. **`render_glyph`**:
   - `let is_color = self.is_color_glyph(glyph); let sbix = is_color;`
   - the synthetic-bold rect growth is gated on `!sbix`;
   - after the `dx` re-center, if `sbix` apply the whole-pixel quantization
     (using `cell_width`/`cell_height` from the grid metrics);
   - `canvas_padding = if opts.thicken && !sbix { 1 } else { 0 }`;
   - the required atlas format is
     `if is_color { Format::Bgra } else { Format::Grayscale }`; if
     `atlas.format() != required`, **return**
     `Err(RenderGlyphError::InvalidAtlasFormat)` **before** any
     allocation/`reserve`/`set` (a real runtime error, faithful to upstream's
     `InvalidAtlasFormat` — not a `debug_assert`, which would let a release
     build silently copy wrong-depth rows or panic in `Atlas::set`). This
     replaces the current grayscale `debug_assert` for the mono path too.
   - `draw_coverage(..., color: is_color)`; the atlas write is unchanged
     (`atlas.set` already copies `format.depth()` bytes per pixel).
3. **`RenderGlyphError`** gains an `InvalidAtlasFormat` variant.
4. Imports: `CGImageAlphaInfo`, `CGImageByteOrderInfo` (for the bitmap-info
   value), and the `kCGColorSpaceDisplayP3` static +
   `CGColorSpace::new_with_name`.

### Scope / faithfulness notes

- **Deferred**: SVG-only color fonts (`opentype::SVG`), and the mono
  colorspace/alpha-info exact match (see the note above).
- The color glyph is premultiplied-first BGRA in CoreGraphics' native order; the
  `Bgra` atlas stores it as-is (the renderer interprets the channel order).
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/face/coretext.rs`:
   - `draw_coverage`: add `color: bool`, branch colorspace/depth/bitmap-info/
     buffer/fill.
   - `render_glyph`: `is_color`/`sbix` routing — gate synthetic bold and
     `canvas_padding` on `!sbix`, apply sbix quantization, **return**
     `RenderGlyphError::InvalidAtlasFormat` when the atlas format doesn't match
     the depth, pass `color` to `draw_coverage`.
   - Add `RenderGlyphError::InvalidAtlasFormat`.
   - Imports for the P3 colorspace + bitmap-info constants.
2. New tests (live CoreText, macOS):
   - `render_color_glyph_into_bgra_atlas`: render the `U+1F600` emoji glyph
     (`Face::new("Apple Color Emoji", 32.0)`, surrogate-pair resolved) into an
     `Atlas::new(1024, Format::Bgra)` with a `.none` `RenderOptions`. Assert
     `g.width > 0`, `g.height > 0`, the region fits, and the reserved region has
     non-zero RGBA bytes (the emoji rendered in color — at least one pixel has a
     non-zero color channel, i.e. not just alpha).
   - `mono_glyph_still_renders`: a regression check that `'M'` from Menlo still
     renders into a `Grayscale` atlas unchanged (size > 0, ink present).
   - `wrong_atlas_format_errors`: rendering a color glyph into a `Grayscale`
     atlas returns `Err(RenderGlyphError::InvalidAtlasFormat)` (and,
     symmetrically, a mono glyph into a `Bgra` atlas).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty face
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- a color (sbix) glyph renders through a Display-P3, premultiplied-first,
  depth-4 context into a `Bgra` atlas, with the sbix whole-pixel quantization
  and the synthetic-bold / `canvas_padding` suppression;
- the mono path is unchanged and its existing tests still pass;
- the atlas-format/depth invariant is asserted;
- a live emoji renders to a non-empty colored region and a live `'M'` still
  renders to a grayscale region;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the P3 colorspace or the bitmap-info value
needs a different shape than expected, or if Apple Color Emoji can't be loaded
in the test environment.

The experiment **fails** if the color geometry (sbix quantization, depth
branching) diverges from upstream or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation. Its first pass raised a
**High** finding: the atlas-format check was a `debug_assert`, which in a
release build would let a color glyph silently copy wrong-depth rows into a
grayscale atlas (or panic in `Atlas::set` for the mono-into-BGRA case) —
upstream returns `InvalidAtlasFormat` at runtime. The design was revised to add
a real `RenderGlyphError::InvalidAtlasFormat` returned before any
`reserve`/`draw_coverage`/`set` when the atlas format doesn't match the depth
(replacing the mono grayscale `debug_assert` too), plus a symmetric
`wrong_atlas_format_errors` test. Codex's re-review confirmed the finding is
**fully resolved** and approved the design, with no other findings (the sbix
quantization placement/formula, `!sbix` gating, Display-P3 +
`Order32Little | PremultipliedFirst` BGRA path, and tests are sound; the mono
`device_gray` deviation is an accepted documented scoped deviation).

Review artifacts:

- Prompts: `logs/codex-review/20260602-213405-968557-prompt.md`,
  `…-213557-627770-prompt.md`
- Results: `logs/codex-review/20260602-213405-968557-last-message.md`,
  `…-213557-627770-last-message.md`

## Result

**Result:** Pass

`draw_coverage` gained a `color: bool` that switches to a Display-P3
(`CGColorSpace::with_name(kCGColorSpaceDisplayP3)`), premultiplied-first
(`Order32Little | PremultipliedFirst`), depth-4 BGRA context with an opaque
white RGBA fill/stroke; the mono path is unchanged. `render_glyph` computes
`is_color`/`sbix`, returns `RenderGlyphError::InvalidAtlasFormat` (new variant)
before any work when the atlas format doesn't match the depth, gates the
synthetic-bold rect growth and `canvas_padding` on `!sbix`, and applies the sbix
whole-pixel quantization after the `dx` re-center.

Tests (live CoreText):

- `render_color_glyph_into_bgra_atlas` — the `U+1F600` emoji renders into a
  `1024×1024` `Bgra` atlas with a non-empty region carrying real color (a pixel
  with a non-zero B/G/R channel, not just alpha).
- `mono_glyph_still_renders` — `'M'` still renders into a `Grayscale` atlas with
  ink (the mono path is unaffected).
- `wrong_atlas_format_errors` — a color glyph into a `Grayscale` atlas and a
  mono glyph into a `Bgra` atlas both return `Err(InvalidAtlasFormat)`.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty face` → 31 passed, 0 failed.
- `cargo test -p roastty` → 2382 passed, 0 failed (no regressions; +3).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

**Upstream `renderGlyph` is now fully ported** — the monochrome path
(constraints, sub-pixel positioning, thicken, synthetic bold) and the color/sbix
path (Display-P3 depth-4 RGBA, sbix quantization, bold/thicken suppression, the
atlas-format guard). `Face::render_glyph` faithfully matches upstream for both
text and emoji.

The font subsystem's remaining work is **above** `renderGlyph`: SVG-color
detection (`opentype::SVG` for non-sbix color fonts), the
**Collection/CodepointResolver** (font fallback and glyph-to-face resolution),
the **shaper** (text shaping — the harfbuzz replacement), and the **Nerd Font
attribute table** (the `getConstraint` lookup deferred from Experiment 256). The
Collection/CodepointResolver is the natural next sub-area, since it sits
directly above the now-complete face/`render_glyph` layer.

## Completion Review

Codex reviewed the completed implementation and result and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-213950-891898-prompt.md`
- Result: `logs/codex-review/20260602-213950-891898-last-message.md`

Codex confirmed the implementation matches the intended sbix color render path:
the P3 BGRA context setup, depth/stride/buffer sizing, opaque RGB fill/stroke,
the sbix quantization after re-centering, the `!sbix` geometry/padding gates,
and the runtime `InvalidAtlasFormat` guard before drawing or atlas writes are
all in place, and the tests cover the color render, the mono regression, and
both format-mismatch directions.
