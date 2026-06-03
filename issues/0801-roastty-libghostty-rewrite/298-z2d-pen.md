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

# Experiment 298: z2d port — the Pen (round-join/cap vertex set)

## Description

Round joins and round caps in z2d are drawn by a **Pen**: a circle of radius
`thickness / 2`, approximated by evenly-spaced vertices, with the angular
density chosen so the chord error stays within `tolerance`. A join or cap walks
a contiguous arc of those vertices between the two face slopes. The Pen
(`vendor/z2d/src/internal/tess/Pen.zig`, adapted from Cairo's `cairo-pen.c`) is
the missing primitive for the box-drawing **arcs** (`U+256D`–`U+2570`) and the
circle/ellipse pieces — and for round joins/caps generally.

This experiment ports the **Pen primitive itself**: the vertex/slope
construction (`init`) and the face-to-face vertex-range selection
(`vertexIteratorFor` + its iterator). It does **not** wire the Pen into the
stroke plotter yet — that (round joins in `join`, round caps, the curve stroke)
is a later experiment, as is the `arc` cubic-flattening that feeds the curve
stroke.

## Translation-only CTM specialization

The sprite `Canvas`'s CTM is **translation-only** (the linear part is identity),
so the Pen's CTM-dependent steps collapse (consistent with Experiments 288/293):

- `arc.transformed_circle_major_axis(ctm, radius)` → `radius` (unity scale: the
  determinant is 1 and the off-diagonal is zero, so Cairo's `has_unity_scale`
  fast-path returns the radius).
- `reflect = ctm.determinant() < 0` → `false` (the identity linear part has
  determinant 1), so `theta` is **not** negated.
- `ctm.userToDeviceDistance(&dx, &dy)` → a no-op for the identity linear part,
  so a vertex is exactly `(radius·cos θ, radius·sin θ)`.

So the ported Pen takes `thickness` and `tolerance` (no CTM argument), with
`radius = thickness / 2` standing in for the major axis. This matches the
existing `Face`/`Canvas` specialization and the sprite-only scope (`roastty` is
macOS-only, the sprite Canvas is the only Pen consumer).

## Upstream behavior (`Pen.init`, `vertexIteratorFor`)

- **Vertex count** (`num_vertices`), with `M = radius` the major axis:
  - `tolerance >= M * 4` → `1` (degenerate pen);
  - else `tolerance >= M` → `4` (the minimum, fast-path);
  - else `delta = acos(1 - tolerance / M)`; if `delta == 0` → `4`; else
    `n = ceil(2π / delta)`; if `n < 4` → `4`; if `n` is odd → `n + 1`; else `n`.
- **Vertices** (a first pass): for `i` in `0..num_vertices`,
  `theta = 2π·i / num_vertices` (negated if `reflect`; here never), the point is
  `(radius·cos θ, radius·sin θ)`. Centered on `(0,0)`, evenly distributed.
- **Slopes** (a second pass, so each is relative to its neighbors): with
  `prev`/`next` wrapping the ring, `slope_cw = Slope(prev.point, point)` and
  `slope_ccw = Slope(point, next.point)`.
- **`vertexIteratorFor(from_slope, to_slope, clockwise)`**: a binary search over
  the ring for the contiguous vertex range between the inbound face's outer
  point and the outbound face's outer point, returned as a `start`/`end`/`idx`
  iterator that steps forward (clockwise) or backward (counter-clockwise),
  wrapping the ring, stopping when `idx == end`. Faithful port of the two
  (cw/ccw) binary-search branches and the `next()` stepping.

## Rust mapping (`roastty/src/font/sprite/raster.rs`)

- `struct PenVertex { point: Point, slope_cw: Slope, slope_ccw: Slope }`.
- `struct Pen { vertices: Vec<PenVertex> }` with
  `Pen::init(thickness: f64, tolerance: f64) -> Pen` (the count formula, the
  vertex pass, the slope pass — `radius` for the major axis, no reflection, no
  device-distance transform).
- `fn vertex_count(radius: f64, tolerance: f64) -> usize` — the count formula,
  factored for direct testing (returns `1` for the degenerate pen).
- `Pen::vertex_iterator_for(&self, from_slope: Slope, to_slope: Slope, clockwise: bool) -> PenVertexIterator`
  — the two binary-search branches, ported with `i32` index arithmetic to match
  upstream's signed wrap math, returning `{ start, end, clockwise }`.
- `struct PenVertexIterator<'a> { pen: &'a Pen, end: usize, idx: usize, clockwise: bool }`
  implementing `Iterator<Item = PenVertex>` (`next` mirrors upstream: forward
  with wrap-to-0 / backward with wrap-to-len, stopping at `idx == end`).

`PenVertex`/`Pen`/the iterator are `pub(crate)` and currently unused by any
caller (the stroke wiring is a later experiment); `font/mod.rs`'s
`#![allow(dead_code)]` covers them, so the build stays warning-free.

## Scope / faithfulness notes

- **Ported**: the Pen vertex/slope construction and the vertex-range iterator,
  specialized to the translation-only CTM.
- **Deferred**: wiring the Pen into the stroke plotter (round joins in `join`,
  round caps in `finish`/`plotSingle`), the cubic-curve stroke (`runCurveTo`
  with round joins), the `arc` spline flattening, and the box-drawing arcs /
  circle pieces that consume all of the above.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/raster.rs`: add `PenVertex`, `Pen` (`init`,
   `vertex_count`, `vertex_iterator_for`), and `PenVertexIterator`.
2. Tests (deterministic):
   - `pen_vertex_count_degenerate`: `tolerance >= 4·radius` → `1`.
   - `pen_vertex_count_minimum`: `radius <= tolerance < 4·radius` → `4`.
   - `pen_vertex_count_even`: a small tolerance gives an **even** count `>= 4`
     equal to `ceil(2π / acos(1 - tolerance/radius))` rounded up to even.
   - `pen_vertices_on_circle`: every vertex lies on the radius-`r` circle
     (`hypot(x,y) ≈ r`), `vertex[0]` is `(r, 0)`, and the angular step between
     consecutive vertices is `2π / n` (no reflection — angles increase).
   - `pen_vertex_slopes`: `slope_cw`/`slope_ccw` equal `Slope(prev,p)` /
     `Slope(p,next)` for a representative vertex, with ring wrap.
   - `pen_vertex_iterator_clockwise`: for a pen and two slopes bracketing a
     known arc, the clockwise iterator yields the expected contiguous, in-order
     vertex range (by index), and the counter-clockwise call yields the
     reverse-stepped range — asserting the start/end selection and the wrap
     stepping.
   - `pen_vertex_iterator_wrap`: a clockwise range whose indices cross the
     `len - 1 → 0` boundary yields the correct wrapped, contiguous sequence (per
     the design-review suggestion — the wrap is where such bugs hide).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty raster
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Pen::init` reproduces z2d's vertex count (degenerate/minimum/even-rounded),
  the evenly-spaced circle vertices (no reflection, no device transform under
  the translation-only CTM), and the neighbor-relative `slope_cw`/`slope_ccw`;
- `vertex_iterator_for` reproduces the cw/ccw binary-search range selection and
  the wrapping `next()` stepping;
- the count/geometry/slope/iterator tests confirm the behavior;
- the stroke wiring, the curve stroke, the `arc` flattening, and the arcs/circle
  glyphs stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the Pen needs a non-identity-CTM term that the
sprite path actually exercises (it should not — the CTM is translation-only).

The experiment **fails** if the vertex set or the iterator range diverges from
z2d, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
changes**. It confirmed the translation-only CTM reduction (identity linear part
→ `major_axis = radius`, `reflect = false`, `userToDeviceDistance` a no-op, so
vertices are `(r·cos θ, r·sin θ)` with increasing angles), the vertex-count
formula (the degenerate `1`, minimum `4`, `delta == 0`, lower-bound, and
even-rounding cases all match upstream), and that porting `vertexIteratorFor`
with signed `i32` search indices is the right approach — preserving the exact
comparison directions and converting to `usize` only at ring indexing/return. It
judged the Pen testable standalone (count, geometry, slopes, iterator ranges),
the deferred scope sound, and translation-only valid for the sprite Pen
consumers. One **Optional** suggestion: add an iterator test that crosses the
`len - 1 → 0` wrap boundary (where such code tends to hide bugs) — folded into
the test plan as `pen_vertex_iterator_wrap`.

Review artifacts:

- Prompt: `logs/codex-review/20260603-072457-391730-prompt.md`
- Result: `logs/codex-review/20260603-072457-391730-last-message.md`
