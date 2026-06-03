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

# Experiment 288: z2d port — the multisample rasterizer run

## Description

The capstone of the z2d fill rasterizer: the multisample-4× **`run`**
(`vendor/z2d/src/internal/raster/multisample.zig`). It ties together the
already-ported `Polygon` (284), `WorkingEdgeSet` (285), `SparseCoverageBuffer`
(286), and `add_supersampled_span` (287): for each device pixel row it drives
the `WorkingEdgeSet` over the four sub-scanlines, records the filtered spans
into the coverage buffer, then writes each coverage run as an anti-aliased alpha
into the surface. This experiment ports `run` **specialized to the sprite case**
— an alpha8 surface, an opaque `.on` source, and the `src_over` operator (which
is _bounded_, so the unbounded-clear branches drop out) — which is all the
sprite `Canvas` path methods need.

## Upstream behavior (`multisample.run`, sprite-specialized)

- `coverage_full = scale * scale = 16`; `alpha_scale = 256 / 16 = 16`.
- Skip if `!polygon.in_box(scale, width, height)`.
- Scanline range: `start = clamp(floor(extent_top/scale), 0, height-1)`,
  `end = clamp(ceil(extent_bottom/scale), start, height-1)`. Column range:
  `scanline_start_x = clamp(floor(extent_left/scale), 0, width-1)`,
  `scanline_end_x = clamp(ceil(extent_right/scale), scanline_start_x, width)`;
  `draw_width = end_x - start_x`; the coverage buffer has `draw_width` capacity;
  `start_x_scaled = start_x * scale`, `draw_width_scaled = draw_width * scale`.
- `WorkingEdgeSet` + `breakpoints()`; the initial breakpoint index is the
  **saturating predecessor** (`idx -| 1`) of the first breakpoint that is
  `>= start_scanline` — so if the first qualifying breakpoint is at index `0`
  (e.g. it equals or exceeds `start_scanline`), the index is `0`, not "none".
  `run` is a no-op **only** when no breakpoint is `>= start_scanline`.
- For each pixel row `y` in `start..=end` (resetting the coverage buffer each
  row): for `y_offset in 0..4`, `y_scanline_scaled = y*scale + y_offset`; if it
  reached the current breakpoint, `rescan(y_scanline_scaled)` and advance the
  breakpoint index; then `inc(y_scanline_scaled)`, `sort()`,
  `filtered = filter(fill_rule)`. Walk the filtered crossings in pairs with an
  `x_min` guard: `start_x = max(x_min, filtered[2i] - start_x_scaled)` (break if
  `>= draw_width_scaled`),
  `end_x = clamp(filtered[2i+1] - start_x_scaled, start_x, draw_width_scaled)`,
  `fill_len = end_x - start_x`; if `> 0`,
  `add_supersampled_span(coverage_buffer, start_x, fill_len)`; `x_min = end_x`.
- Write-out: walk the coverage runs; for each, `x = cov_x + scanline_start_x`,
  `coverage_val = clamp(raw, 0, 16)`, `coverage_len = min(raw_len, width - x)`;
  `0` → skip; `16` → opaque (set the alpha8 stride to the source `.on` = 255);
  else → `alpha = clamp(coverage_val * 16 - 1, 0, 255)` composited `src_over`
  over each destination pixel.
- `src_over` for alpha8 (z2d's integer `mul(a,b) = trunc(a*b/255)`):
  `out = alpha + dst - trunc(alpha * dst / 255)`. (For an opaque `.on` source
  the full-coverage path reduces to a plain overwrite to 255.)

## Rust mapping (`roastty/src/font/sprite/raster.rs`)

- `fn fill_polygon(buf: &mut [u8], width: i32, height: i32, polygon: &Polygon, fill_rule: FillRule)`:
  the faithful sprite-specialized port of `run`. The surface is the alpha8 `buf`
  (`width * height`, row-major); fills with the opaque `.on` source (255) using
  `src_over`.
- A small `fn src_over_alpha8(dst: u8, alpha: u8) -> u8` helper
  (`alpha + dst - (alpha as u32 * dst as u32 / 255) as u8`).

## Scope / faithfulness notes

- **Deferred**: the general operator/pattern/precision machinery (the unbounded
  operators' clear branches, gradients, dithers, non-`src_over` blend modes,
  non-alpha8 surfaces) — the sprite `Canvas` only ever fills an alpha8 surface
  with an opaque `.on` source under `src_over`. The fill/stroke plotters and
  `Canvas::line`/`fill`/`stroke` are later slices.
- The composite uses z2d's truncating integer `mul`; full coverage is the opaque
  overwrite fast-path.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/raster.rs`: add `fill_polygon` (the `run` port) and
   `src_over_alpha8`.
2. Tests (deterministic; build the polygon with `scale = MSAA_SCALE`, add
   device- coordinate corners, then `fill_polygon` into a small zeroed `buf`):
   - `fill_square_crisp`: a square `(1,1)-(5,5)` into a `6×6` buffer → pixels
     `x∈[1,5), y∈[1,5)` are `255` (axis-aligned, crisp), all others `0`.
   - `fill_partial_row_aa`: a rectangle `(1,1)-(3, 2.5)` into a `6×6` buffer →
     row `y=1` cols `1,2` are `255`; row `y=2` cols `1,2` are `127` (the bottom
     pixel row is half-covered: `2/4` sub-scanlines → coverage `8` →
     `8*16-1 = 127`); everything else `0`.
   - `fill_outside_noop`: a polygon entirely outside the buffer draws nothing.
   - `src_over_math`: `src_over_alpha8(0, 127) == 127`,
     `src_over_alpha8(255, x) == 255`, and an overlap value
     (`src_over_alpha8(127, 127) == 127 + 127 - 127*127/255 = 191`).
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

- `fill_polygon` reproduces z2d's multisample `run` scanline/sub-scanline loop,
  the coverage-buffer span recording, and the coverage→alpha source-over
  write-out for the sprite (alpha8, `.on`, `src_over`) case;
- the crisp square, the deterministic `127` half-pixel AA row, and the
  out-of-box no-op verify the pipeline end to end;
- the general compositor, the plotters, and `Canvas` path methods stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the scanline/breakpoint orchestration needs a
different shape than upstream to match exactly.

The experiment **fails** if the rasterized coverage diverges from z2d or any
public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised one **Required**
finding: the breakpoint-index description was imprecise — upstream initializes
to the **saturating predecessor** (`idx -| 1`) of the first breakpoint
`>= start_scanline` (so the index is `0`, not "none", when the first qualifying
breakpoint is at index 0), and `run` is a no-op only when _no_ breakpoint is
`>= start_scanline`. Fixed in the design. Codex confirmed everything else is
faithful for the sprite-specialized scope: the floor/ceil/clamp scanline/column
ranges, the per-row/per-sub-scanline loop order (breakpoint rescan, index
advance, `inc`, `sort`, `filter`, the `x_min`-guarded pair walk, the
supersampled span add), the write-out (coverage clamp, length crop, zero-skip,
full-coverage `255`, partial `coverage*16-1`), the `src_over_alpha8` integer
math, the bounded `src_over`/`.on`/alpha8 specialization, and the deterministic
AA math (coverage `8` → alpha `127`; `src_over_alpha8(127,127) = 191`).

Review artifacts:

- Prompt: `logs/codex-review/20260603-061058-578728-prompt.md`
- Result: `logs/codex-review/20260603-061058-578728-last-message.md`

## Result

**Result:** Pass

`roastty/src/font/sprite/raster.rs` gained `src_over_alpha8` (the truncating
integer `src_over`) and `fill_polygon` — the faithful sprite-specialized port of
z2d's `multisample.run`: the scanline/column range computation, the
saturating-predecessor breakpoint-index init, the per-row/per-sub-scanline loop
(breakpoint rescan + index advance, `inc`, `sort`, `filter`, the `x_min`-guarded
pair walk feeding `add_supersampled_span`), and the coverage write-out
(clamp/crop, zero-skip, full-coverage opaque `255`, partial
`alpha = clamp(cov*16-1, 0, 255)` source-over).

Tests (deterministic; polygons built at `scale = MSAA_SCALE`):

- `src_over_math` — `src_over_alpha8(0,127)=127`, `(255,100)=255`,
  `(127,127)=191`.
- `fill_square_crisp` — a `(1,1)-(5,5)` square into a `6×6` buffer: exactly the
  interior `x∈[1,5), y∈[1,5)` is `255`, every other pixel `0` (full 36-pixel
  check).
- `fill_partial_row_aa` — a `(1,1)-(3,2.5)` rectangle: row `y=1` cols `1,2` are
  `255`, the half-covered row `y=2` cols `1,2` are exactly `127`, the rest `0` —
  the AA value verified end-to-end through the whole pipeline.
- `fill_outside_noop` — an out-of-box polygon draws nothing.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty raster` → 36 passed (4 new).
- `cargo test -p roastty` → 2537 passed, 0 failed (no regressions; +4).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The z2d fill rasterizer is **complete and verified end to end**: an arbitrary
`Polygon` rasterizes into an alpha8 surface with 4× multisample anti-aliasing,
the half-pixel AA value (`127`) confirmed through the whole `Polygon` →
`WorkingEdgeSet` → `SparseCoverageBuffer` → `add_supersampled_span` → composite
pipeline. What remains to reach the box-drawing diagonals/arcs is the **path
front-end**: the `fill_plotter` (a path's line/curve nodes → flattened `Polygon`
contours) and the `stroke_plotter` (a stroked path → outline `Polygon`, with
butt caps and the join/`Pen` machinery), then a
`Canvas::fill_path`/`line`/`stroke` that builds the polygon and calls
`fill_polygon` on the (padded) `Canvas` buffer. The simplest first consumer is
`Canvas::line` (a 2-node path, butt cap) → the three box-drawing diagonals
(`0x2571`–`0x2573`). Alongside the sprite font remain the discovery consumer,
the UCD emoji-presentation default, codepoint overrides, the shaper, the Nerd
Font attribute table, and SVG color detection.

## Completion Review

Codex reviewed the completed implementation and result and found **no required
changes**. It confirmed `fill_polygon` is faithful for the sprite-specialized
`multisample.run` port — the range computation, breakpoint init, sub-scanline
loop, span recording, write-out behavior, and `src_over_alpha8` math all match
z2d under the alpha8/opaque-`.on`/bounded-`src_over` scope — and that the tests
are deterministic and cover the key outcomes (crisp full coverage, the half-row
AA `127`, the outside no-op, and `src_over_alpha8(127,127) == 191`). It judged
the gates clean.

Review artifacts:

- Prompt: `logs/codex-review/20260603-061505-087270-prompt.md`
- Result: `logs/codex-review/20260603-061505-087270-last-message.md`
