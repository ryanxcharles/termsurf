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

# Experiment 299: z2d port — round joins in the stroke plotter

## Description

With the `Pen` (Experiment 298) in place, this experiment wires it into the
multi-segment open-path stroke (Experiment 297) to support **round joins** —
where two segments meet in a rounded arc instead of a miter point or a bevel
cut. It also makes the join **mode-driven** (`miter` / `round` / `bevel`),
faithful to z2d's `JoinMode`: the existing miter-within-limit-else-bevel
behavior becomes the `miter` arm, a new `bevel` mode always bevels, and the new
`round` arm walks the pen's vertex arc between the two face slopes.

Still **line-only open paths**: `CurveTo`/`ClosePath` stay `unreachable!`, and
**round caps** stay deferred (the ends are still butt-capped). The curve stroke
and the box-drawing arcs build on this in later experiments.

## Upstream behavior (`stroke_plotter.join`, the `.round` arm)

The join (`vendor/z2d/src/internal/tess/stroke_plotter.zig`) is identical to the
ported miter/bevel join up to the `switch (join_mode)`:

- `.miter, .bevel`: if `join_mode == .miter` **and** the miter is within the
  limit (`Slope.compare_for_miter_limit`), the outer gets the single
  `in.intersect(out, join_clockwise)` point; otherwise (bevel mode, or miter
  over the limit) the outer gets the two face ends (`in.p1_*` then `out.p0_*`).
- `.round`: with the pen non-null,
  `vit = pen.vertexIteratorFor(in.dev_slope, out.dev_slope, join_clockwise)`;
  the outer gets `in.p1_ccw`/`in.p1_cw` (by direction), **then each pen vertex
  offset by `p1`** (`{ p1.x + v.point.x, p1.y + v.point.y }`), **then**
  `out.p0_ccw`/`out.p0_cw`. The arc of pen vertices fans the rounded corner from
  the inbound outer end to the outbound outer end.

The **inner** join (after the switch) is unchanged across all modes:
`in.p1_cw`/`in.p1_ccw`, the shared `p1`, then `out.p0_cw`/`out.p0_ccw`.

The pen is **lazily initialized** when `join_mode == .round` (or, later, the cap
mode is round); its radius is `thickness / 2`, sized to `tolerance`.

## Rust mapping (`roastty/src/font/sprite/raster.rs`)

- `pub(crate) enum JoinMode { Miter, Round, Bevel }` (the upstream order).
- `StrokePlotter` gains `join_mode: JoinMode`, `tolerance: f64`, and
  `pen: Option<Pen>`.
  `StrokePlotter::new(thickness, scale, miter_limit, tolerance, join_mode)`
  builds the pen (`Pen::init(thickness, tolerance)`) when `join_mode` is
  `Round`, else `None`.
- `join` gains the mode switch:
  - the existing outer miter/bevel block runs for `Miter`/`Bevel`, guarded by
    `join_mode == Miter && compare_for_miter_limit(...)` for the miter point (so
    `Bevel` always bevels);
  - a new `Round` block plots the inbound outer end, then
    `self.pen.as_ref().unwrap().vertex_iterator_for(in.dev_slope, out.dev_slope, join_clockwise)`
    offset by `p1` via `plot_outer(direction_switched, …)`, then the outbound
    outer end. The co-linear early-out and the inner join are unchanged.
- `stroke_path(nodes, thickness, scale, miter_limit, tolerance, join_mode) -> Polygon`
  — the entry point gains `tolerance` and `join_mode`. The six existing
  `stroke_path` tests pass `0.01, JoinMode::Miter` (unchanged geometry: the pen
  is `None`, the miter/bevel path is byte-identical).

`JoinMode`/the new fields are `pub(crate)`; `font/mod.rs`'s
`#![allow(dead_code)]` covers anything not yet called by `Canvas`.

## Scope / faithfulness notes

- **Ported**: the mode-driven join (`miter`/`round`/`bevel`) for line-only open
  paths, with the pen-arc round join offset by the shared corner `p1`.
- **Deferred**: round/square **caps** (the ends stay butt), the cubic-curve
  stroke (`runCurveTo`), the closed-path stroke, dashes, and the box-drawing
  arcs/circle glyphs.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/raster.rs`: add `JoinMode`; extend `StrokePlotter`
   (`join_mode`/`tolerance`/`pen`, the round arm, the mode-guarded miter/bevel);
   extend `stroke_path`'s signature.
2. Update the six existing `stroke_path` tests to pass `0.01, JoinMode::Miter`
   (geometry unchanged — assert the same edges/extents as before).
3. New tests (deterministic):
   - `stroke_path_round_l`: the L-path `move(0,0), line(10,0), line(10,10)`,
     thickness 2 (radius 1), `JoinMode::Round` → the convex corner is a pen arc:
     far more outer edges than the miter L's 4, the right/top extents bounded by
     the radius around `(10,0)` (`extent_right` in `(10, 11]`, `extent_top` in
     `[-1, 0)`, `extent_left == 0`, `extent_bottom == 10`).
   - `stroke_path_round_vs_miter`: the same zigzag under `Round` has strictly
     more edges than under `Miter` (the arcs add vertices).
   - `stroke_path_bevel_l`: the L-path under `JoinMode::Bevel` always bevels —
     the outer gets the two face ends `(10,-1)` then `(11,0)` instead of the
     single miter apex `(11,-1)`. The **extents match** the miter L
     (`left == 0`, `right == 11`, `top == -1`, `bottom == 10` —
     `out.p0_ccw = (11,0)` still reaches `x = 11` and `in.p1_ccw = (10,-1)`
     still reaches `y = -1`), so the discriminator is the **outer edge count**:
     the bevel's extra face-end vertex adds a diagonal edge, giving one more
     edge than the miter L's 4 (the exact count is locked from the
     implementation). This proves the bevel omits the miter apex while keeping
     the same bounding box.
4. Format and test (`cargo fmt`, accept output).

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

- the round join reproduces z2d's `.round` arm (inbound outer end, the pen-arc
  vertices offset by the corner `p1`, the outbound outer end) and the mode
  switch makes `miter`/`bevel` faithful (the miter point only for `miter` within
  the limit);
- the round-L, round-vs-miter, and bevel-L tests confirm the geometry, and the
  six pre-existing `stroke_path` tests are unchanged under `Miter`;
- round/square caps, the curve stroke, the closed-path stroke, dashes, and the
  arcs/circle glyphs stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the round join needs cap-mode interplay that
the line-only scope does not cover (it should not — caps are independent).

The experiment **fails** if the round/bevel outline diverges from z2d, or any
public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised one **Required**
finding: the original `stroke_path_bevel_l` asserted `extent_right == 10`, which
is wrong — the bevel still plots `out.p0_ccw = (11, 0)`, so the bevel L's
bounding box is identical to the miter L's (`right == 11`, `top == -1`). The
bevel differs by omitting the miter apex `(11, -1)` and using the two face ends
`(10, -1)` then `(11, 0)`. Fixed: the bevel test now asserts the **same
extents** as the miter L and discriminates on the **outer edge count** (the
bevel's extra face-end vertex adds a diagonal edge — one more edge than the
miter L's 4, locked from the implementation). Codex confirmed the rest is sound:
the `.round` arm description (inbound outer end → pen-iterator vertices offset
by `p1` → outbound outer end, all through the direction-switched outer joiner,
inner join unchanged); the `join_mode == Miter && compare_for_miter_limit(...)`
guard (so `Bevel` always bevels and existing `Miter` behavior is unchanged); the
`Round`-only lazy pen init for this line-only butt-cap scope; and threading
`tolerance`/`JoinMode` through `stroke_path`.

Review artifacts:

- Prompt: `logs/codex-review/20260603-073403-052736-prompt.md`
- Result: `logs/codex-review/20260603-073403-052736-last-message.md`

## Result

**Result:** Pass

`roastty/src/font/sprite/raster.rs` gained mode-driven joins:

- `JoinMode { Miter, Round, Bevel }` (the upstream order).
- `StrokePlotter` gained `join_mode`, `tolerance` (via `new`), and
  `pen: Option<Pen>` built (`Pen::init(thickness, tolerance)`) only when
  `join_mode` is `Round`.
- The outer join is now a mode switch: the `Miter | Bevel` arm plots the single
  miter apex only when `join_mode == Miter && compare_for_miter_limit(...)`,
  else the two outer face ends (so `Bevel` always bevels); the new `Round` arm
  plots the inbound outer end, then each pen vertex from
  `vertex_iterator_for(in.dev_slope, out.dev_slope, join_clockwise)` offset by
  the shared corner `p1`, then the outbound outer end — all through
  `plot_outer(direction_switched, …)`. The co-linear early-out and the inner
  join are unchanged.
- `stroke_path` gained `tolerance` and `join_mode` parameters.

Tests:

- The six existing `stroke_path` tests pass `0.01, JoinMode::Miter` — geometry
  byte-identical (the pen is `None`, the miter path unchanged).
- `stroke_path_round_l` — the L under `Round`: the same bounding box as the
  miter L (`0/11/-1/10`) but 16 outer edges (the pen arc) vs the miter L's 4.
- `stroke_path_round_vs_miter` — the zigzag under `Round` has 30 edges vs the
  miter's 6 (each corner becomes an arc).
- `stroke_path_bevel_l` — the L under `Bevel`: the same bounding box as the
  miter L, 5 edges (the extra face-end vertex adds a diagonal) vs the miter's 4,
  proving the bevel omits the apex while keeping the box.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2592 passed, 0 failed (+3 net, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

Round joins render faithfully: the mode-driven outer join (`miter`/`round`/
`bevel`) reproduces z2d's `join` for line-only open paths, with the pen-arc
round corner offset by the shared point. The miter/bevel split is now
mode-correct (`Bevel` always bevels), and the pen is built only when needed.

The next z2d-dependent step is the **cubic-curve stroke** (`runCurveTo`):
flatten each cubic with the existing `Spline` decompose (Experiment 296), then
join the flattened points with **round** joins (the reason `CurveTo` was
deferred — it always round-joins regardless of the outer join mode). That
unlocks the box-drawing **arcs** (`U+256D`–`U+2570`), which stroke a single
quarter-circle cubic, and then the circle/ellipse pieces. After the stroke
families: round/ square **caps**, the closed-path stroke, dashes, and then the
unifying sprite `has_codepoint`/draw entry point, the discovery consumer, the
UCD emoji-presentation default, codepoint overrides, the shaper, the Nerd Font
attribute table, and SVG color detection.

## Completion Review

Codex reviewed the completed implementation and result and found **no Required
changes**. It confirmed the `JoinMode`/`StrokePlotter` wiring (pen present only
for `Round`, faithful while caps remain butt); the `Miter | Bevel` arm matching
upstream (the miter apex only for `JoinMode::Miter && compare_for_miter_limit`,
else bevel face ends, existing miter behavior preserved); the `Round` arm
matching the z2d arm (inbound outer end → pen iterator over
`in.dev_slope → out.dev_slope` offset by `p1` → outbound outer end, all through
`plot_outer(direction_switched, …)`, inner join unchanged); that the
`Vec<Point>` borrow break is implementation-local and does not change the
emitted order; and that the corrected bevel test uses the right discriminator
(same bbox, the extra bevel edge). No Optional findings.

Review artifacts:

- Result review: `logs/codex-review/20260603-073837-390913-last-message.md`
