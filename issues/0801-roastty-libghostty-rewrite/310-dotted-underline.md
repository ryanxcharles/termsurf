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

# Experiment 310: the arc primitive + the dotted underline

## Description

The dotted underline is the last underline decoration — and the first consumer
of the **arc** primitive (z2d's cubic-Bézier circle approximation, `arc.zig`,
Cairo-derived). Upstream `special.zig`'s `underline_dotted` lays a row of dots
across the cell, each a filled circle built from `ctx.arc(x, y, radius, 0, τ)`
and filled with `ctx.fill()`. This experiment ports the arc primitive
(`raster::arc`, emitting `move_to`/`curve_to` path nodes for an arc) and
`draw_underline_dotted`. The arc primitive is also the basis for the
circle/ellipse geometric shapes.

## Upstream behavior

### `arc.zig` (the cubic arc approximation)

- `arc_max_angle_for_tolerance(tolerance)`: the largest arc angle a single cubic
  can approximate within `tolerance` — a table lookup (11 entries, `π/1 … π/11`)
  falling back to a `π/i` search. The comparisons must be preserved exactly: the
  table uses `err < tolerance`, the fallback uses `err <= tolerance`.
- `arc_segments_needed(angle, radius, tolerance)`:
  `ceil(|angle| / arc_max_angle_for_tolerance(tolerance / major_axis))`, with
  `major_axis = radius` under the translation-only CTM.
- `arc_segment(xc, yc, radius, A, B)`: one cubic for the arc `A → B`, with
  `h = 4/3 · tan((B − A) / 4)` and control points
  `(xc + r·cosA − h·r·sinA, yc + r·sinA + h·r·cosA)`,
  `(xc + r·cosB + h·r·sinB, yc + r·sinB − h·r·cosB)`, and end
  `(xc + r·cosB, yc + r·sinB)`.
- `arc_in_direction(xc, yc, radius, amin, amax, dir, tolerance)` (forward only
  needed): if `amax − amin > π`, recurse on the two halves; else emit a
  `line_to` to the start point, then `segments` `arc_segment`s stepping by
  `(amax − amin) / segments`. (A full circle `0 → τ` recurses into two `π`
  halves.) The first `line_to` (with no current point) acts as a `move_to`.

### `special.zig` `underline_dotted`

- `float_thick = underline_thickness`; `radius = (1/√2) · float_thick` (a bit
  fatter so dots don't look anemic).
- `padding = canvas.padding_y`;
  `y = min(float_pos + 0.5·float_thick, float_height + padding − ceil(radius))`.
- `dot_count = max(min(ceil(width / (4·radius)), floor(width / (3·radius)), floor(width / (2·radius + 1))), 1)`.
- `x = (width / dot_count) / 2`; for each of `dot_count` dots:
  `arc(x, y, radius, 0, τ)` then `close_path`; `x += width / dot_count`; finally
  `fill()`.

## Rust mapping (`roastty/src/font/sprite/raster.rs`, `draw.rs`)

- `roastty/src/font/sprite/raster.rs`:
  - `arc_max_angle_for_tolerance(tolerance: f64) -> f64` (the table + fallback).
  - `arc_segments_needed(angle: f64, radius: f64, tolerance: f64) -> i32`
    (`major_axis = radius`).
  - `pub(crate) fn arc(cx, cy, radius, angle_min, angle_max, tolerance) -> Vec<PathNode>`
    — the forward recursion, emitting a `MoveTo` for the first point of the arc
    then `LineTo`/`CurveTo` (a private recursive helper threads a `first` flag
    so the leading `line_to` becomes `MoveTo`; `arc_segment` pushes a
    `CurveTo`). Reverse direction and the `max_full_circles` wrap are deferred
    (a single `0 → τ` circle exercises neither).
- `roastty/src/font/sprite/draw.rs`:
  `pub(crate) fn draw_underline_dotted(canvas: &mut Canvas, width: u32, height: u32, metrics: &Metrics)`
  — compute `radius`/`y`/`dot_count`, build the node list (per dot:
  `arc(x, y, radius, 0, τ)` nodes then `PathNode::ClosePath`; advance `x`), and
  `canvas.fill_path(&nodes)`. Update the module doc.

`std::f64::consts::TAU` for τ, `FRAC_1_SQRT_2` for `1/√2`, `PI` for π.

## Scope / faithfulness notes

- **Ported**: the forward `arc` primitive (the tolerance-driven cubic
  approximation) and the dotted underline.
- **Deferred**: the reverse arc direction, the `max_full_circles` wrap, the
  circle/ellipse geometric shapes (later consumers of `arc`), and the sprite
  dispatch.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/raster.rs`: add `arc_max_angle_for_tolerance`,
   `arc_segments_needed`, `arc` (+ the recursive helper and `arc_segment`).
2. `roastty/src/font/sprite/draw.rs`: add `draw_underline_dotted`; note it in
   the module doc.
3. Tests (deterministic):
   - `arc_full_circle_nodes`: `arc(0, 0, 10, 0, τ, 0.1)` emits a `MoveTo` at
     `(10, 0)` followed by **4** `CurveTo`s (per the upstream table:
     `tolerance / radius = 0.01` → `max_angle = π/2` → 2 segments per `π` half →
     4 total). Only the **on-circle** points are checked: the `MoveTo` point and
     **each `CurveTo.p3` endpoint** are at distance ≈ 10 from the center (the
     control points `p1`/`p2` intentionally sit off the circle and are not
     asserted), and the endpoints cover the full circle (a `p3` near `(−10, 0)`
     and the last back at `(10, 0)`).
   - `arc_fill_disc`: filling `arc(5, 5, 4, 0, τ, 0.1) + ClosePath` via
     `Canvas::fill_path` inks a disc — the center `(5, 5)` is inked and a point
     well outside the radius (`(0, 0)`) is not.
   - `underline_dotted_dots`: `draw_underline_dotted` on the fixture cell inks
     dots along the underline band (at least one inked pixel near the underline
     `y`) with gaps between them (a clear column between two dots), and the
     upper cell empty — confirming the dotted layout.
   - (The exact pixels are confirmed against the render during implementation.)
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty raster
cargo test -p roastty sprite
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `raster::arc` reproduces z2d's forward cubic arc approximation (the
  tolerance-driven segment count, the `arc_segment` control points, the
  recursion for arcs `> π`), and `draw_underline_dotted` lays the dot row
  faithfully (the radius, the clamp, the `dot_count`, the per-dot circle fill);
- the arc-nodes, disc-fill, and dotted-row tests confirm the behavior;
- the reverse arc, the circle/ellipse shapes, and the sprite dispatch stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the dotted underline needs an arc feature
(reverse, wrap) the forward circle does not exercise.

The experiment **fails** if the arc approximation or the dotted-underline
geometry diverges from z2d, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised one **Required**
finding: the `arc_full_circle_nodes` test must **not** assert that all emitted
path points lie on the circle — the cubic control points (`p1`/`p2`)
intentionally sit off the circle; only the initial `MoveTo` point and each
`CurveTo.p3` endpoint are on-circle. Fixed: the test now checks only the
`MoveTo` and the `p3` endpoints against the radius (and pins the segment count).
Two **Optional** suggestions, both folded in: assert the expected full-circle
segment count (`r = 10`, `tolerance = 0.1` → 4 cubics under the upstream table),
and preserve the exact `arc_max_angle_for_tolerance` comparisons (table
`err < tolerance`, fallback `err <= tolerance`). Codex confirmed the rest is
sound: the arc emission strategy is faithful (the first synthetic `lineTo`
becomes `MoveTo`, later half-arc starts become harmless zero-length `LineTo`s);
the `arc_segment` control points and `h`; the recursion; the dotted-underline
geometry, dot-count math, and multi-subpath NonZero disc fill; and deferring the
reverse direction, the `max_full_circles` wrap, and the dispatch.

Review artifacts:

- Prompt: `logs/codex-review/20260603-084806-637909-prompt.md`
- Result: `logs/codex-review/20260603-084806-637909-last-message.md`
