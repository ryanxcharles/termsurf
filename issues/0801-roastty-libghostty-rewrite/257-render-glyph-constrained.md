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

# Experiment 257: Wire the constraint into render_glyph — RenderOptions + scaled draw

## Description

Experiment 255 gave `render_glyph` the unconstrained monochrome path; Experiment
256 ported the `constrain` geometry as a standalone module. This experiment
**joins them**: `render_glyph` takes a `RenderOptions` (grid metrics +
constraint + constraint width), adds the `cell_baseline` term, calls
`constrain`, re-centers within the cell (`dx`), and rasterizes at the
**constrained** size via a `scaleCTM` stretch. After this, `render_glyph`
faithfully matches the monochrome core of upstream `renderGlyph` —
constraint-aware cell placement, the piece the shaper needs for correct icon /
box-drawing / emoji / symbol layout.

### Upstream `renderGlyph` geometry being added (`font/face/coretext.zig`)

On top of the Experiment 255 core (`font/face/coretext.zig` lines 289–567):

- `cell_width = grid_metrics.cell_width` (as `f64`),
  `cell_baseline = grid_metrics.cell_baseline` (as `f64`).
- `glyph_size = constraint.constrain({ width: rect.w, height: rect.h, x: rect.origin.x, y: rect.origin.y + cell_baseline }, grid_metrics, constraint_width)`.
  The baseline is added to `y` **before** constraining because `constrain`
  operates on cell-relative positions, not baseline-relative ones.
- `x, y, width, height = glyph_size`.
- **Cell re-centering** (when `constraint.size != .stretch`):
  `dx = (cell_width - grid_metrics.face_width) / 2; x += dx; if dx < 0 { x -= dx.trunc() }`.
- Whole-pixel bearings and canvas from the **constrained** `x/y/width/height`:
  `px_x = floor(x)`, `px_y = floor(y)`, `frac_x = x - floor(x)`,
  `frac_y = y - floor(y)`, `px_width = ceil(width + frac_x)`,
  `px_height = ceil(height + frac_y)` (`canvas_padding = 0`, thicken deferred).
- Draw: `translateCTM(frac_x, frac_y)`, then
  `scaleCTM(width / rect.w, height / rect.h)` (the stretch that maps the raw
  outline to the constrained size), then `drawGlyphs` at the **raw** negated
  bearings `(-rect.origin.x, -rect.origin.y)`.
- `offset_x = px_x`, `offset_y = px_y + px_height`. Atlas write unchanged.

For an unconstrained (`.none`) glyph the stretch is identity (`width == rect.w`,
`height == rect.h`), but the baseline term and the `dx` re-center still apply —
so the new path is strictly more faithful than Experiment 255's (which omitted
both).

### Rust mapping (`roastty/src/font/face/coretext.rs`)

1. **`RenderOptions`** (new struct): `grid_metrics: Metrics`,
   `constraint: Constraint`, `constraint_width: u8`. (Upstream's
   `cell_width: ?u2`, `thicken`, and `thicken_strength` are deferred with the
   thicken/color branches.)
2. **Shared rasterization helper** to avoid duplicating the bitmap-context block
   between the unconstrained primitive and the constrained path:
   `fn draw_coverage(&self, glyph: u16, draw_x: f64, draw_y: f64, frac_x: f64, frac_y: f64, scale_x: f64, scale_y: f64, px_w: usize, px_h: usize) -> Option<Vec<u8>>`.
   It creates the `DeviceGray` bitmap context over a zeroed `px_w * px_h`
   buffer, sets antialiasing + white fill, applies
   `translate_ctm(frac_x, frac_y)` then `scale_ctm(scale_x, scale_y)`, draws the
   glyph at `(draw_x, draw_y)`, drops the context before returning `buf`.
   - `rasterize_glyph` (unchanged behavior) becomes a thin caller:
     `scale_x = scale_y = 1.0`, `draw = (-origin.x, -origin.y)`, `frac`/`px_*`
     from the raw rect — identical output to Experiment 255.
3. **`render_glyph`** gains an `opts: &RenderOptions` parameter and the geometry
   above:
   - raw `rect`; `< 0.25` guard → zero `Glyph`;
   - `constrained = opts.constraint.constrain(GlyphSize { width: rect.w, height: rect.h, x: rect.origin.x, y: rect.origin.y + cell_baseline }, &opts.grid_metrics, opts.constraint_width)`;
   - `dx` re-center unless `opts.constraint.size == Size::Stretch`;
   - `px_x/px_y/frac_x/frac_y/px_w/px_h` from the constrained values;
   - `bitmap = draw_coverage(glyph, -rect.origin.x, -rect.origin.y, frac_x, frac_y, width / rect.w, height / rect.h, px_w, px_h)?`;
   - reserve + set;
     `Glyph { width: px_w, height: px_h, offset_x: px_x, offset_y: px_y + px_h, atlas_x, atlas_y }`.
4. Imports: `Constraint`, `GlyphSize`, `Size` from `super::constraint`;
   `Metrics` from `crate::font::metrics`.

### Scope / faithfulness notes

- **Deferred** (later experiments): color/sbix (depth-4 P3 RGBA), synthetic bold
  (fill-stroke + size growth), thicken/font-smoothing (`canvas_padding`, the
  smoothing toggles, the `thicken_strength` gray fill), the explicit
  subpixel-quantization toggles, and the Nerd Font attribute table that supplies
  real per-codepoint constraints.
- `render_glyph` returns `Result<Glyph, RenderGlyphError>` (atlas-full or
  bitmap-context-creation failure, both propagated rather than panicked,
  matching upstream) and still `debug_assert!`s the grayscale atlas format.

### Tests (live CoreText, macOS)

The `.none` path leaves the stretch at `1.0`, so it cannot catch the
highest-risk bugs here (swapped `scale_ctm` numerator/denominator, missing
`scale_ctm`, wrong CTM order, or canvas/bearings from raw instead of constrained
size). A **constrained** test is therefore required:

- `render_glyph_none_*` (the updated Experiment 255 tests): a `.none`
  `RenderOptions` built from `Metrics::calc(face.get_metrics())` — `'M'` still
  lands inside the atlas with a positive top bearing; space → zero glyph. (These
  now exercise the baseline term and `dx`, which `.none` legitimately applies.)
- `render_glyph_stretch_fills_cell` (new, the key coverage): a `Size::Stretch`
  constraint (`align_horizontal: Start`, `align_vertical: Center1`, matching the
  Experiment 256 E0C0 fixture shape) with
  `grid_metrics = Metrics::calc(face.get_metrics())`, `constraint_width = 1`,
  rendering `'M'`. Because `stretch` maps any outline to exactly the cell, the
  resulting `Glyph` is deterministic regardless of the raw bbox:
  `width == grid_metrics.cell_width`, `height == grid_metrics.cell_height`,
  `offset_x == 0`, `offset_y == grid_metrics.cell_height as i32`. It then reads
  the reserved atlas region (via a new `Atlas::data(&self) -> &[u8]` accessor)
  and measures the **bounding box of the inked pixels**, asserting it spans most
  of the cell in _both_ axes (`ink_w >= 0.8 * g.width` and
  `ink_h >= 0.8 * g.height`). A stretched `'M'` fills nearly the whole cell, so
  this holds; but if `scale_ctm` were **omitted** the raw `'M'` would draw at
  its natural (smaller-than-cell, e.g. clipped in height) bbox and fail the
  spans-most-of-cell check, and if the ratio were **inverted** the glyph would
  shrink to a dot and fail both — closing the scale-direction gap without
  brittle pixel pinning. `Atlas::data` is a one-line getter the renderer will
  need regardless.

## Changes

1. `roastty/src/font/face/coretext.rs`:
   - Add `RenderOptions`.
   - Extract `draw_coverage`; reduce `rasterize_glyph` to call it (output
     unchanged).
   - Add the constraint geometry to `render_glyph` (new `opts` parameter).
   - New imports for `Constraint`/`GlyphSize`/`Size`/`Metrics`.
2. `roastty/src/font/atlas.rs`: add `pub(crate) fn data(&self) -> &[u8]`
   (returns the backing texture bytes; used by the new ink-coverage test and by
   the future renderer upload path).
3. Update the Experiment 255 `render_glyph` tests to pass a `.none`
   `RenderOptions`, and add `render_glyph_stretch_fills_cell` (above).
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

- `render_glyph` takes `RenderOptions`, adds the `cell_baseline` term, calls
  `constrain`, applies the `dx` re-center (except for `stretch`), and rasterizes
  at the constrained size via `scale_ctm(width/rect.w, height/rect.h)`, with the
  bearings and canvas computed from the constrained `x/y/width/height`;
- the shared `draw_coverage` helper backs both paths and `rasterize_glyph`'s
  output is unchanged (the Experiment 254 rasterization tests still pass);
- a `.none`-constraint `'M'` still lands inside the atlas with a positive top
  bearing and the space glyph is still a zero glyph;
- a `Size::Stretch` constraint renders `'M'` to exactly the cell
  (`width == cell_width`, `height == cell_height`, `offset_x == 0`,
  `offset_y == cell_height`), and the inked-pixel bounding box in the atlas
  region spans most of the cell in both axes (`>= 0.8 ×` the glyph dimensions) —
  proving the constrained size drives the canvas/bearings and that `scale_ctm`
  is present and the right way up;
- color, synthetic bold, and thicken remain cleanly deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the objc2 `scale_ctm` integration needs a
different call shape than expected.

The experiment **fails** if the constrained geometry diverges from upstream
(wrong scale denominator, missing baseline term, or `dx` applied to `stretch`),
or if any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation. Its first pass raised a
**High** finding: the original tests only exercised the `.none` path (scale
`1.0`), so they could not catch a swapped/missing `scale_ctm` or
canvas/bearings-from-raw bugs. The design added
`render_glyph_stretch_fills_cell` (a `Size::Stretch` constraint with
deterministic cell-sized `Glyph` geometry). A second pass refined the ink check:
a generic ink-fraction assertion could still pass with `scale_ctm` omitted, so
the test now measures the **inked-pixel bounding box** in the atlas region and
asserts it spans `>= 0.8 ×` the glyph dimensions in both axes (reading the
region via a new `Atlas::data` accessor). Codex's final pass found **no
remaining issues**, confirming the test now covers raw-canvas, omitted-scale,
and inverted-scale failure modes with a non-pixel-pinned threshold.

Review artifacts:

- Prompts: `logs/codex-review/20260602-205425-240790-prompt.md`,
  `…-205601-687623-prompt.md`, `…-205706-218703-prompt.md`
- Results: `logs/codex-review/20260602-205425-240790-last-message.md`,
  `…-205601-687623-last-message.md`, `…-205706-218703-last-message.md`

## Result

**Result:** Pass

`render_glyph` now takes `&RenderOptions`, adds the `cell_baseline` term, calls
`constrain`, applies the `dx` cell re-center (skipped for `Stretch`), and
rasterizes at the constrained size via
`scale_ctm(width / rect.w, height / rect.h)`. The bitmap-context block was
extracted into a shared `draw_coverage` helper (translate-then-scale-then-draw);
`rasterize_glyph` now calls it with scale `1.0` and is otherwise unchanged. A
`RenderOptions` struct (grid metrics, constraint, constraint width) was added.

A result-review **Medium** finding was fixed: the initial implementation used a
documented `.expect()` for the `draw_coverage` `None` case, but the CoreGraphics
steps (`CGColorSpace::new_device_gray`, `CGBitmapContextCreate`) are fallible by
API contract and upstream propagates bitmap-context-creation failure with `try`.
`render_glyph` now returns `Result<Glyph, RenderGlyphError>` — a small internal
enum (`AtlasFull` | `ContextCreationFailed`, with `From<AtlasError>`) — so the
FFI path never panics and the error type isn't misused.

Tests (live CoreText):

- `render_glyph_places_m_in_atlas` / `render_glyph_space_is_zero` (updated to a
  `.none` `RenderOptions` from `Metrics::calc(face.get_metrics())`) — still
  pass; they now also exercise the baseline term and `dx`.
- `render_glyph_stretch_fills_cell` (new) — a `Size::Stretch` constraint renders
  `'M'` to exactly the cell (`width == cell_width`, `height == cell_height`,
  `offset_x == 0`, `offset_y == cell_height`), and the inked-pixel bounding box
  read from the atlas spans `>= 0.8 ×` the cell in both axes, confirming
  `scale_ctm` is present and the right way up.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty face` → 21 passed, 0 failed (Experiment 254
  rasterization tests unchanged).
- `cargo test -p roastty` → 2372 passed, 0 failed (no regressions; +1).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

`render_glyph` now faithfully matches the monochrome core of upstream
`renderGlyph` end to end: bounding rect → baseline + constrain + `dx` re-center
→ scaled rasterization → atlas write → `Glyph`. The remaining `renderGlyph`
branches layer on in later experiments: the color/sbix depth-4 P3 RGBA path
(needs an RGBA atlas format and the premultiplied-first bitmap info), synthetic
bold (fill-stroke + size growth), and thicken/font-smoothing (`canvas_padding`,
the smoothing toggles, the `thicken_strength` gray fill). Beyond `renderGlyph`,
the font subsystem still needs the Collection/CodepointResolver and the shaper,
and the Nerd Font attribute table to supply real per-codepoint constraints.

## Completion Review

Codex reviewed the completed implementation. It raised one **Medium** finding —
`render_glyph` panicked via `.expect()` on a `draw_coverage` `None`, but the
CoreGraphics steps are fallible by API contract and upstream propagates
bitmap-context-creation failure. The fix introduced `RenderGlyphError`
(`AtlasFull` | `ContextCreationFailed`, with `From<AtlasError>`); `render_glyph`
now returns `Result<Glyph, RenderGlyphError>` and propagates the failure instead
of panicking. A follow-up review confirmed the finding is **fully resolved**
with no new issues — the geometry, CTM order, buffer lifetime, and stretch
coverage test remain sound. The full suite (2372) and all gates pass after the
fix.

Review artifacts:

- Prompts: `logs/codex-review/20260602-210139-973885-prompt.md`,
  `…-210335-765177-prompt.md`
- Results: `logs/codex-review/20260602-210139-973885-last-message.md`,
  `…-210335-765177-last-message.md`
