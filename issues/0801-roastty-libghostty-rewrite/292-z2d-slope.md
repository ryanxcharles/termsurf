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

# Experiment 292: z2d port — Slope

## Description

The last z2d sub-area for the box-drawing glyphs is the **stroke path** (a
stroked path → outline `Polygon`), which the diagonals reach via `Canvas::line`.
Its foundation is **`Slope`** (`vendor/z2d/src/internal/tess/Slope.zig`, derived
from Cairo): a segment's direction vector, with the comparison and miter-limit
helpers the stroke joins use. It is self-contained (depends only on `Point` and
`std` math) and testable in isolation. This experiment ports it; `Face`, `Pen`,
and the stroke plotter follow.

## Upstream behavior (`Slope.zig`)

- `Slope { dx, dy }`; `init(a, b)` is the vector `b - a`.
- `equal(other)`: exact `dx`/`dy` equality. `calculate()`: `dy / dx`.
- `compare(a, b) -> i32`: an angular comparison done multiplicatively on the
  vectors (`< 0` when `a < b`, `0` equal, `> 0` when `a > b`). It snaps `b`'s
  components to `a`'s when within one `f64` epsilon, takes
  `sign(a.dy·bdx - bdy·a.dx)`, and returns that if non-zero; else applies
  tie-breakers (zero vectors compare equal and "greater" than non-zero;
  opposite/`pi` directions use the sign-difference rule).
- `compare_for_miter_limit(in, out, miter_limit) -> bool`: normalizes both
  slopes, takes their dot product, and returns `2 <= miter_limit² · (1 + dot)`
  (the Cairo miter test).
- `normalize() -> f64`: asserts the slope is non-zero, sets it to the unit
  vector (with exact axis cases for `dx == 0` / `dy == 0`, else `hypot`), and
  returns the pre-normalization magnitude.

## Rust mapping (`roastty/src/font/sprite/raster.rs`)

- `struct Slope { dx: f64, dy: f64 }` with `init`/`equal`/`calculate`/`compare`/
  `compare_for_miter_limit`/`normalize`.
- A `fn sign(x: f64) -> f64` helper returning `1.0`/`-1.0`/`0.0` (matching Zig's
  `math.sign`, which is `0` at `0` — _not_ `f64::signum`, which is `±1` at
  `±0`).
- `math.floatEps(f64)` → `f64::EPSILON`; `math.hypot` → `f64::hypot`;
  `@intFromFloat` of the non-zero sign → `as i32`.

## Scope / faithfulness notes

- **Deferred**: `Face`, `Pen`, the `stroke_plotter`, the `Dasher`, the `Path`
  builder, and `Canvas::line`/`fill`/`stroke` — later stroke-path slices.
- The custom `sign` (zero at zero) is required for faithfulness; `f64::signum`
  would differ at `±0`.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/raster.rs`: add `Slope` (+ methods) and the `sign`
   helper.
2. Tests (deterministic):
   - `slope_init`: `Slope::init((1,2),(4,6)) == {dx:3, dy:4}`.
   - `slope_calculate`: `{dx:2, dy:6}.calculate() == 3.0`.
   - `slope_normalize`: `{dx:3, dy:4}.normalize()` returns `5.0`, leaving
     `{0.6, 0.8}`; `{dx:0, dy:5}` → `5.0`, `{0,1}`; `{dx:-4, dy:0}` → `4.0`,
     `{-1,0}`.
   - `slope_compare`: `compare(init((0,0),(1,0)), init((0,0),(0,1))) == -1`
     (`+x` before `+y`); parallel same-direction → `0`
     (`compare({1,1},{2,2}) == 0`); opposite (`pi`) directions →
     `compare({1,0},{-1,0}) == -1`.
   - `slope_miter_limit`: for a right-angle turn (`in={1,0}, out={0,1}`,
     `dot == 0`), `compare_for_miter_limit(in, out, 2.0) == true` (`4 >= 2`) and
     `… 1.0 == false` (`1 < 2`).
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

- `Slope` reproduces z2d's vector, the `compare` cross-product/tie-breaker logic
  (with the custom zero-at-zero `sign`), the miter-limit test, and the
  `normalize` axis/`hypot` cases;
- the deterministic init/calculate/normalize/compare/miter tests confirm
  faithfulness;
- `Face`, `Pen`, the stroke plotter, and `Canvas` path methods stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a helper needs a different shape to serve the
(next) `Face`/stroke faithfully.

The experiment **fails** if the slope math diverges from z2d/Cairo or any public
C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**. It confirmed `init`/`equal`/`calculate` match upstream, that
`compare` captures the epsilon snap, the cross-product `sign`, and all
tie-breakers in the correct order (zero-vector and opposite-direction cases),
that a custom zero-at-zero `sign` is required (`f64::signum` would be unfaithful
at `±0.0`), and that `compare_for_miter_limit` and `normalize` match `Slope.zig`
(the pre-normalization magnitude return, the axis fast paths, the nonzero
assertion, the `hypot` fallback). It recomputed all the deterministic checks
(init `{3,4}`, normalize cases, `compare` `+x,+y = -1`, parallel `0`, opposite
`-1`, miter `ml=2 true`/`ml=1 false`).

Review artifacts:

- Prompt: `logs/codex-review/20260603-063303-355711-prompt.md`
- Result: `logs/codex-review/20260603-063303-355711-last-message.md`
