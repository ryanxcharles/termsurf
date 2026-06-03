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

# Experiment 297: z2d port — the multi-segment open-path stroke

## Description

A multi-segment **open** path (several `line_to`s) strokes by building two
contours — `outer` (the convex side, appended forward) and `inner` (the concave
side, prepended) — with a **join** between each consecutive segment pair, then
capping both ends and concatenating into the result. z2d's stroke plotter
(`vendor/z2d/src/internal/tess/stroke_plotter.zig`) does this. With `Slope`
(292), `Face`/`intersect` (293), and `Contour` (294/295) in place, this
experiment ports the **line-only** open-path stroke (butt caps, **miter/bevel**
joins) as `stroke_path(nodes, thickness, scale, miter_limit) -> Polygon` — the
join machinery for straight segments.

`CurveTo` is **deferred**: upstream `runCurveTo` flattens the spline and joins
the flattened points with **round** joins (needing `Pen`), regardless of the
outer join mode — so the box-drawing **arcs** (which contain a `curve_to`) need
the `Pen`/round-join path, a later experiment. Round caps, closed paths, dashes,
and dotted strokes are likewise deferred.

## Upstream behavior (`stroke_plotter`, open path, butt caps, miter/bevel)

- The plotter walks the path nodes into a `PointBuffer(2, 5)`: `move_to`
  finishes any prior subpath and starts a new one; `line_to` appends the point
  and, once 3+ points exist, `join`s the last three. (`curve_to` — the
  round-joined flattened spline — is deferred with `Pen`.)
- `join(p0, p1, p2)`: builds faces `in = Face(p0, p1)`, `out = Face(p1, p2)`;
  `join_clockwise = in.dev_slope.compare(out.dev_slope) < 0`; the polygon
  clockwise direction `poly_clockwise` is fixed on the first join. If the join
  direction differs from the polygon's, the outer/inner plotters are swapped
  (`direction_switched`) to avoid twisting. Then:
  - co-linear (`compare == 0`): plot only the inbound face end (outer gets
    `in.p1_ccw`/`in.p1_cw` by direction; inner the opposite);
  - **miter** within the limit (`Slope::compare_for_miter_limit`): outer gets
    the `in.intersect(out, join_clockwise)` miter point; else
    (**bevel**/over-limit) outer gets the two face ends `in.p1_*` then
    `out.p0_*`;
  - inner always gets `in.p1_*`, the shared `p1`, then `out.p0_*` (by
    direction).
  - the outer plots append (`before = null`); the inner plots prepend
    (`plotReverse`).
- `finish` (open, ≥3 points): `plotOpenJoined` caps the start (`cap_p0`,
  inserted before the original first `outer` node — i.e. an **order-preserving**
  prefix insert, not a per-point reverse) and the end (`cap_p1`, appended),
  `outer.concat(inner)`, then `addEdgesFromContour`.

## Rust mapping (`roastty/src/font/sprite/raster.rs`)

- `struct StrokePlotter { thickness, scale, miter_limit, points: PointBuffer<2,5>, clockwise: Option<bool>, result: Polygon, outer: Contour, inner: Contour }`
  with `run(nodes)` (the `move_to`/`line_to` walk; a `curve_to` is an
  `unreachable!` for now — the line-only scope), `join(p0, p1, p2)` (the
  miter/bevel + direction-switch logic), `finish`, `plot_open_joined`, and the
  single-segment `plot_single` (reusing the Experiment 295 stroke).
- `fn stroke_path(nodes: &[PathNode], thickness: f64, scale: f64, miter_limit: f64) -> Polygon`
  — the entry point (`StrokePlotter::run` then `finish`, returning `result`).
- Contour insertion: the walk-time joins append to `outer` (and `plot_reverse`
  to `inner`); the start cap is collected into a temp `Vec` and prepended to
  `outer` **preserving order** (insert at indices `0, 1, …` or splice at the
  front). No arbitrary mid-list insert is needed.

## Scope / faithfulness notes

- **Deferred**: `CurveTo` and the **round** joins/caps (`Pen`) — so the
  box-drawing **arcs** (which flatten a cubic with round joins) are a later
  experiment that adds `Pen`; the closed-path stroke (`plotClosedJoined`); the
  dotted/dashed strokes; and the arbitrary mid-list `Contour` insert. This
  experiment is line-only open paths, butt caps, miter/bevel joins.
- The `PointBuffer<2, 5>` (split 2, len 5) keeps the initial 2 points and the
  recent 3 — what the join (`tail(3)`/`tail(2)`/`tail(1)`) needs.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/raster.rs`: add `StrokePlotter` (+ `run`/`join`/
   `finish`/`plot_open_joined`) and `stroke_path`.
2. Tests (deterministic):
   - `stroke_path_single`: a 2-node path (`move,line`) → the same polygon as
     `stroke_line` (the single-segment fallback).
   - `stroke_path_l_miter`: an L-shaped path
     `move(0,0), line(10,0), line(10,10)`, thickness 2, scale 1, miter_limit 10
     → a polygon whose outer corner is the miter point `(9 or 11, ±1)` (computed
     via `Face::intersect`), enclosing the bend; assert the edge count and that
     the miter point appears among the edge coordinates.
   - `stroke_path_collinear`: `move(0,0), line(5,0), line(10,0)` (a straight
     line through a redundant point) → the same 2-edge bar as a single segment
     (the co-linear join plots only the inbound end).
   - `stroke_path_zigzag`: a 3-segment path
     `move(0,0), line(10,0), line(10,10), line(0,10)` → a closed-looking outline
     with the expected edge count and extents enclosing all the points (two
     miter joins).
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

- `stroke_path` reproduces z2d's line-only open-path multi-segment stroke — the
  `move_to`/`line_to` node walk, the miter/bevel/co-linear joins with the
  direction-switch logic, and the butt-cap `plotOpenJoined` assembly (with the
  order-preserving start-cap prefix insert);
- the single/L-miter/collinear/zigzag tests confirm the geometry;
- `CurveTo`, the round joins/caps (`Pen`), the closed-path stroke, and the
  dashes stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the arc's specific stroke needs join behavior
beyond miter/bevel/co-linear.

The experiment **fails** if the stroke outline diverges from z2d or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised two **Required**
findings: (1) the start-cap insert could not be a naive per-point `plot_reverse`
— it must preserve the emitted cap order; (2) `runCurveTo` joins the flattened
spline with **round** joins (`Pen`), so a miter-only `stroke_path` would diverge
for cubic arcs. Both fixed: the experiment is **narrowed to line-only** open
paths (`CurveTo` is `unreachable!`, with `Pen`/round-joins/arcs deferred to a
later experiment), and the start cap is collected into a temp `Vec` and
prepended to `outer` preserving order (splice at the front). A follow-up cleaned
the pass criteria (line-only, the zigzag test). Codex confirmed the join
assembly, direction switching, miter/bevel/co-linear behavior, inner prepend,
`plotSingle` fallback, and open `finish` all align with upstream for the
line-only scope — with no remaining design issues.

Review artifacts:

- Prompt: `logs/codex-review/20260603-071004-664426-prompt.md`
- Result: `logs/codex-review/20260603-071004-664426-last-message.md`
- Follow-up: `logs/codex-review/20260603-071213-846307-last-message.md`
