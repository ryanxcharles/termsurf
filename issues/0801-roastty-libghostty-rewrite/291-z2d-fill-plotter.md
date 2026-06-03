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

# Experiment 291: z2d port — the fill plotter

## Description

The **fill plotter** (`vendor/z2d/src/internal/tess/fill_plotter.zig`) turns a
path's nodes into a `Polygon` (the input to the rasterizer): it walks the
move/line/curve/close nodes, flattens each `curve_to` via the (Experiment 290)
`Spline`, and `add_edge`s each segment. It builds on `PathNode` (289), `Spline`
(290), and `Polygon` (284), and ships two upstream tests. It also needs the
small **`PointBuffer`** (`point_buffer.zig`) the plotters use to track the
subpath's first point and its recent points.

## Upstream behavior

- `PointBuffer(split, len)`: a fixed buffer of `len` points. `add` appends until
  full, then keeps the first `split` items and FIFO-rotates the rest (so with
  `split = 1`, `first()` stays pinned to the subpath's initial point while
  `last()` follows the most recent; `len` caps at the buffer size). `reset`
  empties it; `first()`/`last()`/`head(n)`/`tail(n)` index it (null when out of
  range). Fill uses `PointBuffer(1, 3)`.
- `fill_plotter.plot(nodes, scale, tolerance) -> Polygon`:
  - `move_to`: if it is the **last** node (the auto-added move after a
    `close_path`), stop; else `reset` the buffer and `add` the point.
  - `line_to`: if there is a current point and it differs from the target,
    `add_edge(last, target)` and `add` the target (else, no current point is an
    invalid state).
  - `curve_to`: flatten the cubic from the current point through `p1`/`p2` to
    `p3` with `Spline`; for each flattened point, the same "differs from last →
    `add_edge` + `add`" logic.
  - `close_path`: only if `len >= 3` (else a degenerate line, cleared on the
    next `move_to`); if the current point equals the first, no-op; else
    `add_edge(last, first)` and `add(first)`.

## Rust mapping (`roastty/src/font/sprite/raster.rs`)

- `struct PointBuffer<const SPLIT: usize, const LEN: usize> { items: [Point; LEN], len: usize }`
  with `new`/`add`/`reset`/`first`/`last`/`head`/`tail` — the faithful port (the
  `add` FIFO-rotate keeping the first `SPLIT`).
- `fn fill_plot(nodes: &[PathNode], scale: f64, tolerance: f64) -> Polygon` —
  the faithful port using `PointBuffer<1, 3>`. The upstream `InternalError`
  invalid-state branches (a `line_to`/`curve_to` with no current point — only
  reachable from a malformed path) become `unreachable!`/`panic!`, since the
  `Canvas` only emits well-formed paths.

## Scope / faithfulness notes

- **Deferred**: the `stroke_plotter` (the `Pen`/join/cap outline machinery), the
  `Path`/`StaticPath` builder, and `Canvas::line`/`fill`/`stroke` — later z2d
  slices.
- `PointBuffer` is ported generically (it serves the stroke plotter too); fill
  uses `<1, 3>`.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/raster.rs`: add `PointBuffer` and `fill_plot`.
2. Tests:
   - `point_buffer_split_one`: a `PointBuffer<1, 3>` after adding 4 points keeps
     the first (`first()` is point 0) and FIFO-rotates the tail (`last()` is
     point 3, `len == 3`); `reset` empties it.
   - `fill_degenerate_line_to` (the upstream test): nodes
     `move(5,0), line(10,10), line(10,10), line(0,10), close, move(5,0)` →
     exactly two edges `{y0:0, y1:10, x_start:5, x_inc:0.5}` and
     `{y0:10, y1:0, x_start:5, x_inc:-0.5}` (the duplicate `line_to` skipped,
     the horizontal `(10,10)→(0,10)` filtered, the close edge added).
   - `fill_degenerate_close_move_line` (the upstream test):
     `move, line, close, move` → one edge `{y0:0, y1:10, x_start:5, x_inc:0.5}`
     (the `close` is a no-op since `len < 3`).
   - `fill_degenerate_double_close` (the upstream test):
     `move, line, line, close, move, close, move` → the same two edges as the
     first test.
   - `fill_curve`: a path with a `curve_to` produces multiple edges (the cubic
     flattened), with the polygon extents inside the control bounding box.
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

- `PointBuffer` reproduces the split/FIFO behavior and `fill_plot` reproduces
  the node-walk, the curve flattening, and the degenerate-line/close handling,
  verified by the ported upstream tests;
- the stroke plotter, `Path` builder, and `Canvas` path methods stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the plotter needs buffer behavior beyond
first/last/len for fill.

The experiment **fails** if the plotted polygon diverges from z2d or any public
C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**. It confirmed `PointBuffer` matches the split-preserving FIFO
behavior, reset, and the nullable `head`/`tail`/`first`/`last` indexing; that
`fill_plot` matches `fill_plotter.zig` (the last-`move_to` stop, the `line_to`
duplicate skip, the `curve_to` flatten reusing the same last-differs edge/add
logic, and the `close_path` `len >= 3` / `last == first` handling); that
treating malformed invalid-state branches as `unreachable!`/`panic!` is
acceptable for the well-formed Canvas-path scope; and that the upstream test
expectations recompute correctly (`degenerate_line_to` → the two edges
`{0,10,5,0.5}`/`{10,0,5,-0.5}`, `move,line,close,move` → one edge,
`double_close` → the same two edges).

Review artifacts:

- Prompt: `logs/codex-review/20260603-062727-751303-prompt.md`
- Result: `logs/codex-review/20260603-062727-751303-last-message.md`

## Result

**Result:** Pass

`roastty/src/font/sprite/raster.rs` gained the generic `PointBuffer<SPLIT, LEN>`
(`new`/`add`/`reset`/`first`/`last`/`head`/`tail`, with the split-preserving
FIFO `add`) and `fill_plot` — the faithful node walk: `MoveTo` (stop on the
trailing auto-move, else reset+add), `LineTo` (skip duplicates, else add
edge+point), `CurveTo` (flatten via `Spline`, each point through the same
edge/add logic), `ClosePath` (the `len >= 3` / `last != first` close).
Malformed-path branches are `unreachable!`.

Tests:

- `point_buffer_split_one` — `<1,3>` keeps `first` pinned and FIFO-rotates the
  tail; `reset` empties.
- `fill_degenerate_line_to` (upstream) → two edges `{0,10,5,0.5}` /
  `{10,0,5, -0.5}` (duplicate skipped, horizontal filtered, close edge added).
- `fill_degenerate_close_move_line` (upstream) → one edge (`len < 3` close is a
  no-op).
- `fill_degenerate_double_close` (upstream) → the same two edges.
- `fill_curve` — a `CurveTo` flattens to many edges, all within the control
  bounding box.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty raster` → 54 passed (5 new).
- `cargo test -p roastty` → 2555 passed, 0 failed (no regressions; +5).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The fill plotter completes the **fill path** of the z2d port: a path's nodes →
`fill_plot` → `Polygon` → `fill_polygon` (the rasterizer) → AA alpha8 surface. A
filled vector path can now be rendered end to end. The one remaining z2d
sub-area for the box-drawing glyphs is the **`stroke_plotter`** — a stroked path
→ outline `Polygon` with the `Pen`/join/cap machinery (the box-drawing diagonals
call `Canvas::line`, which strokes a 2-node butt-cap path). After that, a
`Canvas::fill_path`/`line`/`stroke` wires the plotters + `fill_polygon` to the
(padded) `Canvas` buffer, unblocking the diagonals (`0x2571`–`0x2573`), the
arcs, the circle/ellipse pieces, and the geometric curves. Alongside the sprite
font remain the discovery consumer, the UCD emoji-presentation default,
codepoint overrides, the shaper, the Nerd Font attribute table, and SVG color
detection.

## Completion Review

Codex reviewed the completed implementation and result and found **no required
changes**. It confirmed `PointBuffer` matches `point_buffer.zig` (the
split-preserving FIFO rotation, the nullable `head`/`tail`/`first`/`last`, the
`tail(0)` assertion), and `fill_plot` matches `fill_plotter.zig` (the trailing
`MoveTo` break, the shared duplicate-skip + `add_edge` + buffer-add for line and
curve, the point tracking through `add_edge`'s horizontal filter, and the
`ClosePath` `len >= 3` / `last != first` logic), that the `unreachable!`
branches are acceptable for the well-formed Canvas-path scope, and that the
ported upstream and added tests are correct. It judged the gates clean.

Review artifacts:

- Prompt: `logs/codex-review/20260603-063012-751816-prompt.md`
- Result: `logs/codex-review/20260603-063012-751816-last-message.md`
