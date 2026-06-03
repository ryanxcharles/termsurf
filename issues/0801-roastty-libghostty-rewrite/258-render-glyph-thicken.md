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

# Experiment 258: Faithful drawing context + thicken (font smoothing)

## Description

The Experiment 255–257 `draw_coverage` sets only antialiasing. Upstream
`renderGlyph` configures the drawing context more fully before drawing
(`font/face/coretext.zig` lines 478–508):

- **Sub-pixel positioning ON, quantization OFF** (unconditional) — so CoreText
  honors the exact sub-pixel position we computed via the fractional CTM
  translate instead of snapping to the pixel grid. These four calls are
  **currently missing** from `draw_coverage` (a fidelity gap).
- **Font smoothing ("thicken")** — `setAllowsFontSmoothing(true)` always, and
  `setShouldSmoothFonts(opts.thicken)`. When thicken is on, the glyph is drawn
  slightly heavier (closer to native macOS text). Thicken also adds a
  one-pixel-per-edge **`canvas_padding`** so the heavier ink isn't clipped.
- **Gray fill from `thicken_strength`** — the non-color fill is
  `gray = thicken_strength / 255` (default `255` → `1.0`, i.e. white), not a
  hardcoded `1.0`.

This experiment ports that full context configuration and the `thicken`
canvas-padding geometry. It does **not** add color/sbix (the
`setShouldSmoothFonts` sbix exemption and the RGBA branch) or synthetic bold
(the stroke path) — those stay deferred; here
`canvas_padding = if thicken { 1 } else { 0 }`.

### Upstream geometry with `canvas_padding` (`font/face/coretext.zig`)

With `canvas_padding` (lines 396–525):

- `px_x = floor(x) - canvas_padding`, `px_y = floor(y) - canvas_padding`.
- `frac_x = x - floor(x)`, `frac_y = y - floor(y)` (unchanged).
- `px_width = ceil(width + frac_x) + 2 * canvas_padding`,
  `px_height = ceil(height + frac_y) + 2 * canvas_padding`.
- `translateCTM(frac_x + canvas_padding, frac_y + canvas_padding)` (the padding
  shifts the draw inward so the glyph is centered in the padded canvas).
- `scaleCTM` and the draw position are unchanged.
- `offset_x = px_x`, `offset_y = px_y + px_height` (unchanged formula, new
  padded values).

So for a given glyph, turning thicken on grows the canvas by `2` in each axis,
moves `offset_x` by `-1`, and moves `offset_y` by `+1` — a deterministic,
testable shift.

### Rust mapping (`roastty/src/font/face/coretext.rs`)

1. **`RenderOptions`** gains `thicken: bool` and `thicken_strength: u8`
   (upstream defaults `false` / `255`).
2. **`draw_coverage`** takes the translate offsets directly (`tx`, `ty` — the
   caller folds in `canvas_padding`) plus `thicken: bool` and `fill_gray: f64`,
   and adds the full faithful context block before the draw:
   - `set_allows_font_smoothing(true)`, `set_should_smooth_fonts(thicken)`;
   - `set_allows_font_subpixel_positioning(true)`,
     `set_should_subpixel_position_fonts(true)`;
   - `set_allows_font_subpixel_quantization(false)`,
     `set_should_subpixel_quantize_fonts(false)`;
   - `set_allows_antialiasing(true)`, `set_should_antialias(true)` (already
     present);
   - `set_gray_fill_color(Some(&ctx), fill_gray, 1.0)`;
   - `translate_ctm(tx, ty)` then `scale_ctm(scale_x, scale_y)`.
   - `rasterize_glyph` (the unconstrained primitive) calls it with
     `tx = frac_x`, `ty = frac_y`, `thicken = false`, `fill_gray = 1.0` — output
     unchanged except for the now-present subpixel/smoothing toggles, which only
     refine sub-pixel fidelity and don't change the existing tests' assertions
     (size / ink presence).
3. **`render_glyph`**: `canvas_padding = if opts.thicken { 1 } else { 0 }`;
   `px_x/px_y` subtract it, `px_w/px_h` add `2 * canvas_padding`, the translate
   passes `frac + canvas_padding`; the gray fill is
   `opts.thicken_strength as f64 / 255.0`, and `set_should_smooth_fonts` gets
   `opts.thicken`.

### Scope / faithfulness notes

- **Deferred** (later experiments): color/sbix (the depth-4 RGBA path, the
  `setShouldSmoothFonts` sbix exemption, the sbix pixel quantization) and
  synthetic bold (rect growth + `set_text_drawing_mode(fill_stroke)` +
  `set_line_width`). With those deferred, `canvas_padding` depends only on
  `thicken`.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/face/coretext.rs`:
   - Add `thicken` / `thicken_strength` to `RenderOptions`.
   - Extend `draw_coverage` (faithful context block; `tx`/`ty`/`thicken`/
     `fill_gray` params).
   - Add the `canvas_padding` geometry and the `thicken`/`thicken_strength`
     wiring to `render_glyph`.
   - Update `rasterize_glyph`'s call (`thicken = false`, `fill_gray = 1.0`,
     `tx/ty = frac`).
2. Update the existing `render_glyph` test option builders to set
   `thicken: false`, `thicken_strength: 255`.
3. New tests (live CoreText, macOS):
   - `render_glyph_thicken_pads_canvas`: render `'M'` with `.none` +
     `thicken = false` and again with `thicken = true`; assert
     `thick.width == plain.width + 2`, `thick.height == plain.height + 2`,
     `thick.offset_x == plain.offset_x - 1`,
     `thick.offset_y == plain.offset_y + 1`, and both glyphs have ink.
   - `render_glyph_strength_dims_fill`: render `'M'` with `.none`,
     `thicken = false`, `thicken_strength = 255` vs `64`; read the atlas regions
     and assert the max pixel value of the `255` render exceeds that of the `64`
     render (the grayer fill caps coverage lower).
4. Format and test (`cargo fmt`, accept output).

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

- `draw_coverage` applies the full faithful context block (sub-pixel positioning
  on, quantization off, font smoothing allowed and gated on `thicken`,
  antialiasing on, gray fill from `fill_gray`);
- `render_glyph` adds the `canvas_padding` geometry (the `±` shifts above) and
  wires `thicken`/`thicken_strength`;
- the `thicken` canvas-padding shifts are exactly `+2`/`+2`/`-1`/`+1` and a
  lower `thicken_strength` caps the fill darker;
- `rasterize_glyph`'s existing tests still pass (output unchanged but for
  refined sub-pixel fidelity);
- color and synthetic bold stay cleanly deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if an objc2 context setter needs a different shape
than expected.

The experiment **fails** if the `canvas_padding` geometry diverges from upstream
or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-210807-534248-prompt.md`
- Result: `logs/codex-review/20260602-210807-534248-last-message.md`

Codex confirmed the `canvas_padding` geometry matches upstream (bearings
subtract padding, canvas adds `2 * padding`, translate adds padding,
`offset_y = px_y + px_height` yields the net `+1` top-bearing shift), that the
deterministic thicken shift (`+2/+2/-1/+1`) is correct because `frac_x/frac_y`
are unchanged, that the unconditional `thicken_strength / 255` fill matches
upstream even when `thicken = false` and the max-pixel test direction is sound,
and that the newly-added subpixel toggles refine sub-pixel fidelity without
changing the existing size/ink assertions.

## Result

**Result:** Pass

`draw_coverage` now applies the full faithful context block — font smoothing
(allowed always, `set_should_smooth_fonts(thicken)`), sub-pixel positioning on,
sub-pixel quantization off, antialiasing on, and the gray fill from `fill_gray`
— with `tx`/`ty`/`thicken`/`fill_gray` parameters. `RenderOptions` gained
`thicken: bool` and `thicken_strength: u8`. `render_glyph` computes
`canvas_padding = if opts.thicken { 1 } else { 0 }` and folds it into the
bearings (`- padding`), canvas (`+ 2 * padding`), and translate (`+ padding`),
with the gray fill `opts.thicken_strength / 255`. `rasterize_glyph` calls
`draw_coverage` with `thicken = false`, `fill_gray = 1.0`.

Tests (live CoreText):

- `render_glyph_thicken_pads_canvas` — turning thicken on shifts a fixed `'M'`
  by exactly `width + 2`, `height + 2`, `offset_x - 1`, `offset_y + 1`, with ink
  in both renders. The `canvas_padding` geometry is pinned deterministically.
- `render_glyph_strength_dims_fill` — `thicken_strength = 255` reaches a
  brighter peak pixel than `64`, confirming the `thicken_strength / 255` fill.
- The Experiment 254–257 tests still pass unchanged (the new subpixel toggles
  refine fidelity without altering size/ink/geometry assertions).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty face` → 23 passed, 0 failed.
- `cargo test -p roastty` → 2374 passed, 0 failed (no regressions; +2).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The glyph drawing context is now fully faithful for the monochrome path, and
thicken/font-smoothing is wired with its `canvas_padding` geometry. The two
remaining `renderGlyph` branches are **synthetic bold** (rect growth by the line
width before constraining, `set_text_drawing_mode(fill_stroke)` +
`set_line_width`, with the gray stroke color) and the **color/sbix** path (the
`isColorGlyph`/`ColorState` detection, the depth-4 P3 RGBA atlas + bitmap-info,
and the sbix pixel quantization). Color is the larger sub-area (it needs font
traits + sbix/SVG detection and an RGBA atlas write); synthetic bold is the
smaller, self-contained next step.

## Completion Review

Codex reviewed the completed implementation and result and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-211230-803022-prompt.md`
- Result: `logs/codex-review/20260602-211230-803022-last-message.md`

Codex confirmed the canvas-padding math is faithful (`px_x/px_y` subtract
padding, `px_w/px_h` add two per axis, translate gets `frac + padding`,
`offset_x = px_x` / `offset_y = px_y + px_h` preserve the net `-1`/`+1` shifts),
that the casts are sound because `canvas_padding ∈ {0, 1}`, that
`rasterize_glyph` keeps equivalent geometry with the new toggles not affecting
its assertions, and that both thicken tests are valid and non-flaky.
