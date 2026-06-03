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

# Experiment 300: z2d port — the cubic-curve stroke (`runCurveTo`)

## Description

A stroked path may contain cubic Bézier segments (`curve_to`). z2d strokes a
curve by **flattening** it into line segments (the `Spline.decompose` ported in
Experiment 296) and feeding each flattened point back through the line-stroke
machinery — but with a **round** join between the segments, regardless of the
path's configured join mode. This is why `curve_to` was deferred until the `Pen`
(298) and round joins (299) existed.

This experiment ports `runCurveTo`: lazily build the pen, decompose the cubic,
and run each flattened point as a round-joined `line_to`. It removes the
`CurveTo` `unreachable!` from the stroke walk. The box-drawing **arcs**
(`U+256D`–`U+2570`, which `path.curveTo(...)` then stroke) build on this in the
next experiment. `ClosePath` (the closed-path stroke), round/square caps, and
dashes stay deferred.

## Upstream behavior (`stroke_plotter.runCurveTo`)

- `_runLineTo(join_mode, node)` (the parameterized line walk): consume a
  degenerate point, else append it and
  `join(join_mode, tail(3), tail(2), tail(1))` once 3+ points exist. `runLineTo`
  passes the configured `opts.join_mode`; the curve's flattened points pass
  `.round`.
- `runCurveTo(node)`: read the current point; **lazy-init the pen** if null
  (`Pen.init(thickness, tolerance, ctm)`); build a
  `Spline { a = current, b = p1, c = p2, d = p3, tolerance }`;
  `spline.decompose()` with a plotter whose `line_to` calls
  `_runLineTo(.round, point)`. So every flattened segment joins with a round
  corner, independent of the outer join mode.
- `Spline.decompose` emits the **intermediate** flattened points and the final
  `d` (not the start `a`, which is already the current point) — exactly the
  ported `Spline::decompose(out)` (Experiment 296).

## Rust mapping (`roastty/src/font/sprite/raster.rs`)

- `StrokePlotter` gains a `tolerance: f64` field (already threaded into `new`;
  now stored for the lazy pen and the spline).
- `StrokePlotter::ensure_pen(&mut self)` — build
  `Pen::init(self.thickness, self.tolerance)` into `self.pen` if it is `None`.
  (The `Round`-join eager build in `new` is preserved; `ensure_pen` covers the
  curve case for any outer mode.)
- `run_line_to` and `join` gain a `join_mode: JoinMode` parameter (upstream's
  `_runLineTo`/`join` take it). The `LineTo` walk passes `self.join_mode`; the
  curve passes `JoinMode::Round`. `join` uses the passed mode for its outer
  switch (no longer `self.join_mode`).
- `run_curve_to(&mut self, p1, p2, p3)` — read the current point, `ensure_pen`,
  build the `Spline`, `decompose` into a `Vec<Point>`, and
  `run_line_to(Round, p)` for each. Replaces the `CurveTo` `unreachable!` in
  `run`.
- `ClosePath` stays `unreachable!` (closed-path stroke deferred).

No public entry-point signature change: `stroke_path` already carries
`tolerance` and `join_mode`.

## Scope / faithfulness notes

- **Ported**: the cubic-curve stroke — flatten via `Spline::decompose`, round-
  join the flattened segments — for open paths.
- **Deferred**: the box-drawing arcs/circle glyphs (the next experiment builds
  the arc paths and a `Canvas` curve entry point), round/square **caps**, the
  closed-path stroke (`ClosePath`), and dashes.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/raster.rs`: store `tolerance`; add `ensure_pen`;
   parameterize `run_line_to`/`join` by `join_mode`; add `run_curve_to`; wire
   `CurveTo` in `run`.
2. Tests (deterministic):
   - `stroke_path_curve_degenerate_line`: a `curve_to` whose cubic degenerates
     to a straight line (`a == b`, `c == d`, `d = (10,0)`) strokes to the **same
     polygon** as the single `line_to` segment (`decompose` emits just `d`, so
     the curve adds one round-joined point equivalent to a line).
   - `stroke_path_curve_quarter`: a genuinely curved cubic (`move(0,0)` then
     `curve_to((0,5.523), (4.477,10), (10,10))`, a quarter-circle
     approximation), thickness 2, `tolerance 0.1` → a non-empty stroke whose
     extents enclose the endpoints `±` the half-width, with **many** edges (the
     flattened segments each round-joined), strictly more than the 2-edge single
     bar.
   - `stroke_path_curve_uses_round`: the same curved cubic strokes identically
     whether the path's `join_mode` is `Miter` or `Round` (the curve always
     round-joins) — assert the **full polygon** is equal (the same `edges` and
     extents), not just the edge count (per the design review).
   - `stroke_path_line_then_curve`: a `line_to` followed by a `curve_to`
     (`move(0,0), line(5,0), curve_to(...)`) strokes without panic into a
     non-empty polygon — directly exercising the transition into the curve and
     the round join at the line→curve seam (per the design review).
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

- `run_curve_to` reproduces z2d's `runCurveTo` — the lazy pen, the
  `Spline::decompose` flattening, and the round-joined flattened segments
  (independent of the outer join mode);
- the degenerate-line, quarter-curve, and uses-round tests confirm the geometry;
- the arcs/circle glyphs, round/square caps, the closed-path stroke, and dashes
  stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the curve stroke needs cap or closed-path
behavior the open-path scope does not cover (it should not — the arcs are open
paths capped at the ends).

The experiment **fails** if the curve stroke diverges from z2d, or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
changes**. It confirmed the faithfulness of: parameterizing `run_line_to`/`join`
by `JoinMode` (normal `LineTo` uses the configured mode, curve-flattened points
use `Round`), matching upstream `_runLineTo`/`join`; `ensure_pen` (only needed
for `Miter`/`Bevel` paths that hit a curve, since `Round` paths already have the
pen from Experiment 299); the `Spline` mapping (`a` is the current buffered
point, not re-added; `decompose` emits the flattened points through `d`; each
feeds `run_line_to(Round, p)`); `ClosePath` staying deferred while `CurveTo`
becomes reachable; and that the degenerate-cubic test equals a single line
stroke (one emitted point, no join fires before `plot_single`). Two **Optional**
suggestions, both folded in: `stroke_path_curve_uses_round` now asserts **full
polygon equality** (not just the edge count) between outer `Miter` and `Round`
for a curve-only path; and a new `stroke_path_line_then_curve` test exercises
the `line_to → curve_to` transition (the round join at the seam).

Review artifacts:

- Prompt: `logs/codex-review/20260603-074137-472576-prompt.md`
- Result: `logs/codex-review/20260603-074137-472576-last-message.md`

## Result

**Result:** Pass

`roastty/src/font/sprite/raster.rs` gained the cubic-curve stroke:

- `StrokePlotter` stores `tolerance`; `ensure_pen` builds
  `Pen::init(thickness, tolerance)` only when `pen` is `None` (the eager `Round`
  build from Experiment 299 is preserved).
- `run_line_to` and `join` gained a `join_mode: JoinMode` parameter. The
  `LineTo` walk passes `self.join_mode`; `join`'s outer switch uses the passed
  mode (existing miter/round/bevel behavior for line paths unchanged).
- `run_curve_to(p1, p2, p3)` reads the current point, `ensure_pen`, builds a
  `Spline { a = current, b = p1, c = p2, d = p3, tolerance }`, `decompose`s into
  a `Vec`, and runs each flattened point as `run_line_to(JoinMode::Round, p)`.
- `CurveTo` is now wired in `run`; `ClosePath` stays `unreachable!`.

Tests:

- `stroke_path_curve_degenerate_line` — a cubic with `a == b`, `c == d` flattens
  to just `d`, so the curve stroke equals the single `line_to` polygon (same
  edges and extents).
- `stroke_path_curve_quarter` — a quarter-circle cubic from `(0,0)` to
  `(10,10)`: 45 round-joined edges (`> 10`), the box bulging past each endpoint
  by the half-width (`left < 0`, `right > 10`, `top < 0`, `bottom > 10`).
- `stroke_path_curve_uses_round` — a curve-only path strokes to an identical
  polygon (edges + extents) under `Miter` and `Round`, proving the curve always
  round-joins.
- `stroke_path_line_then_curve` — a `line_to → curve_to` path strokes without
  panic into a non-empty polygon (the round join at the seam).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2596 passed, 0 failed (+4, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The cubic-curve stroke renders faithfully: `runCurveTo` flattens via the
Experiment 296 `Spline::decompose` and round-joins the flattened segments,
independent of the path's outer join mode. The stroke pipeline now handles
`move_to`/`line_to`/`curve_to` open paths with miter/round/bevel joins and butt
caps.

The next step is the **box-drawing arcs** (`U+256D`–`U+2570`): a `Canvas` curve
entry point (`Canvas::stroke_path` over a `move_to`/`curve_to`/`line_to` path,
or extending `Canvas::line`'s machinery) plus the `arc` geometry from `box.zig`
(each arc strokes a quarter-circle cubic from the cell edge to the center). That
makes the arcs the first curved sprite glyphs. After the arcs: round/square
**caps**, the circle/ellipse pieces, the closed-path stroke, dashes, then the
unifying sprite `has_codepoint`/draw entry point, the discovery consumer, the
UCD emoji-presentation default, codepoint overrides, the shaper, the Nerd Font
attribute table, and SVG color detection.

## Completion Review

Codex reviewed the completed implementation and result and found **no Required
changes**. It confirmed the implementation is faithful to upstream `runCurveTo`
for the open-path scope: `tolerance` stored and `ensure_pen` lazily building the
pen only when needed (eager `Round` build intact); `_runLineTo(join_mode, …)`
correctly modeled (normal `LineTo` uses `self.join_mode`, curve-flattened points
use `JoinMode::Round`); `join` using the passed mode (existing line-path
behavior preserved while curves force round joins); `run_curve_to` using the
current point as `Spline.a` without re-adding it, decomposing, and feeding each
flattened point through round-joined `run_line_to`; and `CurveTo` reachable with
`ClosePath` still deferred. It judged the four tests well-targeted. No Optional
findings.

Review artifacts:

- Result review: `logs/codex-review/20260603-074532-608319-last-message.md`
