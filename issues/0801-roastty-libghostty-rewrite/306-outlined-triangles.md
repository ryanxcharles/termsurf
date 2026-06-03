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

# Experiment 306: Canvas::inner_stroke_path + the outlined corner triangles (U+25F8–25FA, 25FF)

## Description

The outlined corner triangles `◸ U+25F8`, `◹ U+25F9`, `◺ U+25FA`, `◿ U+25FF`
complete the corner-triangle family (the filled ones landed in Experiment 305).
Upstream `geometric_shapes.zig`'s `cornerTriangleOutline` strokes a **closed**
triangle with an **inner** stroke (`innerStrokePath`): the stroke is clipped to
the shape's interior so the outline never spills past the cell.
`innerStrokePath` is a fill-mask × double-width-stroke multiply: it fills the
closed shape as a mask, strokes the path at **double** the width, multiplies the
two (keeping only the stroke that lies inside the shape), and composites the
result. This experiment ports `Canvas::inner_stroke_path` and
`draw_corner_triangle_outline` — a capstone that exercises both the closed-path
stroke (Experiment 304) and the fill (Experiment 305).

## Upstream behavior (`Canvas.innerStrokePath`, `cornerTriangleOutline`)

- `innerStrokePath(path, opts, color)`:
  - allocate two alpha8 surfaces (`fill_sfc`, `stroke_sfc`), the size of the
    main surface;
  - **fill mask**: close the path and `fill` it into `fill_sfc` with alpha 255
    (the solid interior);
  - **double-width stroke**: `mut_opts.line_width *= 2`; `stroke` the path into
    `stroke_sfc` with the color;
  - **multiply** (manual, per byte):
    `fill_sfc[i] = round(255 · (stroke_sfc[i] / 255) · (fill_sfc[i] / 255))` —
    keeps only the part of the (double-width) stroke inside the filled shape,
    i.e. the inner half of the stroke;
  - **composite**: `surface.composite(fill_sfc, .src_over)`.
- `cornerTriangleOutline(metrics, canvas, corner)`:
  `float_thick = Thickness.light.height(box_thickness)`; the same per-corner
  triangle vertices as the filled version (`.tl (0,0)(0,h)(w,0)`,
  `.tr (0,0)(w,h)(w,0)`, `.bl (0,0)(0,h)(w,h)`, `.br (0,h)(w,h)(w,0)`); the path
  `move/line/line/close`; then
  `innerStrokePath(path, butt + line_width = float_thick, .on)`.
- The codepoint→corner dispatch: `0x25F8 → .tl`, `0x25F9 → .tr`, `0x25FA → .bl`,
  `0x25FF → .br`.

## Rust mapping (`roastty/src/font/sprite/raster.rs`, `canvas.rs`, `draw.rs`)

- `roastty/src/font/sprite/raster.rs`: make `src_over_alpha8` `pub(crate)` (the
  composite step needs it). Signature `src_over_alpha8(dst, alpha)`.
- `roastty/src/font/sprite/canvas.rs`:
  `pub(crate) fn inner_stroke_path(&mut self, nodes: &[raster::PathNode], thickness: f64)`
  — translate the nodes by the padding (once), then:
  - **fill mask** — build `mask_nodes` = the translated nodes with a `ClosePath`
    appended if not already closed (matching upstream's `closed_path.close()`,
    which closes only the **fill copy**, so the primitive stays faithful for
    open inputs too); `mask = vec![0u8; self.buf.len()]`; `fill_polygon` of
    `fill_plot(mask_nodes, MSAA_SCALE, 0.1)` into `mask` (the solid interior);
  - **double-width stroke** — `stroke_buf = vec![0u8; self.buf.len()]`;
    `fill_polygon` of
    `stroke_path(translated /* the original */, 2·thickness, MSAA_SCALE, 10.0, 0.1, Miter, Butt)`
    into `stroke_buf` (for the already-closed triangles, a closed-path ring);
  - multiply `mask[i] = round(255 · (stroke_buf[i]/255) · (mask[i]/255))`;
  - composite `self.buf[i] = src_over_alpha8(self.buf[i], mask[i])`.
- `roastty/src/font/sprite/draw.rs`:
  `pub(crate) fn draw_corner_triangle_outline(cp: u32, metrics: &Metrics, canvas: &mut Canvas) -> bool`
  — map `0x25F8/0x25F9/0x25FA/0x25FF` to a `Corner`, build the same per-corner
  triangle node list as `draw_corner_triangle` (`move/line/line/close`),
  `let thick = Thickness::Light.height(box_thickness) as f64`, and
  `canvas.inner_stroke_path(&nodes, thick)`; `_ => false`. Update the module
  doc.

## Scope / faithfulness notes

- **Ported**: `Canvas::inner_stroke_path` (the fill-mask × double-stroke
  multiply composite) and the four outlined corner triangles.
- **Deferred**: shaded fills, the circle/ellipse shapes, and the sprite
  dispatch.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/raster.rs`: `pub(crate)` on `src_over_alpha8`.
2. `roastty/src/font/sprite/canvas.rs`: add `Canvas::inner_stroke_path`.
3. `roastty/src/font/sprite/draw.rs`: add `draw_corner_triangle_outline`; update
   the module doc.
4. Tests (deterministic — the fixture `9×18` cell; each outlined triangle inks
   its three sides but leaves both its **interior** and the **opposite corner**
   empty, confirmed against the render):
   - `corner_outline_25f8_tl` (`◸`): a point on the top-left **edge** is inked;
     the triangle **interior** (just inside the hypotenuse) and the opposite
     **bottom-right** corner are empty.
   - `corner_outline_25f9_tr` (`◹`), `corner_outline_25fa_bl` (`◺`),
     `corner_outline_25ff_br` (`◿`): the analogous edge-inked / interior-and-
     opposite-empty checks per corner.
   - `inner_stroke_hollow`: directly via `Canvas::inner_stroke_path`, a closed
     square's border is inked but its center hole is empty **and** the stroke
     stays inside the square (no spill past the outer edge) — distinguishing the
     inner stroke from the plain closed stroke.
   - `draw_corner_triangle_outline_excludes`: `0x2500`, `0x25E2` (a filled
     triangle), `'M'` return `false` and draw nothing.
   - (The exact inked/empty pixels are confirmed against the render during
     implementation.)
5. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty sprite
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Canvas::inner_stroke_path` reproduces z2d's `innerStrokePath` (the fill mask,
  the double-width stroke, the per-byte multiply, the `src_over` composite), and
  `draw_corner_triangle_outline` renders the four outlined triangles with the
  inner-clipped outline, returning `false` otherwise;
- the four outline tests, the hollow inner-stroke test, and the exclusion test
  confirm the rendering;
- the shaded fills, the circle/ellipse shapes, and the sprite dispatch stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the inner stroke needs a compositing nuance the
single-channel alpha8 multiply does not capture.

The experiment **fails** if the outlined-triangle rendering or the
`inner_stroke_path` algorithm diverges from z2d, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and raised one **Required**
finding: `inner_stroke_path` must close a **mask copy** of the input for the
fill step (matching upstream's `closed_path.close()`) while stroking the
**original** path — otherwise the general primitive diverges as soon as a caller
passes an **open** path (the triangles are already closed, so the rendered
result is equivalent, but the primitive must be faithful). Fixed: the design now
builds `mask_nodes` = the translated nodes with a `ClosePath` appended if not
already closed (used for `fill_plot`), and strokes the original translated nodes
at 2×. Codex confirmed the rest is sound: the temp zeroed buffers are equivalent
to fresh alpha8 surfaces for this opaque path; the multiply formula and the
`src_over_alpha8(self.buf[i], mask[i])` ordering are correct; the outlined-
triangle vertex sets and the dispatch (`0x25F8 → tl`, `0x25F9 → tr`,
`0x25FA → bl`, `0x25FF → br`) are faithful. Test note (folded into the plan):
for triangles whose legs lie on `x=0`/`y=0`, the inner stroke is intentionally
clipped to the interior, so assert "edge" pixels just inside the shape.

Review artifacts:

- Prompt: `logs/codex-review/20260603-082347-320397-prompt.md`
- Result: `logs/codex-review/20260603-082347-320397-last-message.md`
