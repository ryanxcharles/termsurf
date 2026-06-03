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

# Experiment 304: z2d port — the closed-path stroke (`plotClosedJoined`)

## Description

Stroking a **closed** path (a `close_path` node) differs from an open one: there
are no caps; instead the two ends **join around the initial point**, wrapping
the contour shut, and the `outer` and `inner` contours are emitted as **two
separate closed polygons** (the stroke is the ring between them, filled
NonZero). This is `stroke_plotter.runClosePath` + `plotClosedJoined`. With the
full open-path stroke in place (Experiments 297–302), this experiment ports the
closed-path stroke — completing the stroke plotter and unblocking the
circle/ellipse and other closed geometric shapes.

## Upstream behavior (`runClosePath`, `plotClosedJoined`, `plotDotted`)

- `runClosePath` switches on the point count:
  - `0` → nothing;
  - `1` → `plotDotted(first)` — a zero-length closed path: round caps draw a
    full circle (all pen vertices fanned around the point), other caps draw
    nothing;
  - `2` → `plotSingle(head0, head1)` — a degenerate closed path is a single
    capped segment;
  - else → `plotClosedJoined(head0, head1, tail2, tail1)`;
  - then `points.reset()`.
- `plotClosedJoined(initial0, initial1, p1, p2)` records the closing join(s):
  - if `p2 != initial0` (normal): `join(p1, p2, initial0)` then
    `join(p2, initial0, initial1)` — the final segment's join, then the join
    wrapping the initial point;
  - if `p2 == initial0` (degenerate, the path already `line_to`'d the initial
    point): a single `join(p1, initial0, initial1)`;
  - both joins use the configured `join_mode`. Then it emits **both** the
    `outer` and the `inner` contour as closed polygons
    (`addEdgesFromContour(outer)` and `addEdgesFromContour(inner)` — no concat,
    no caps) and resets.

## Rust mapping (`roastty/src/font/sprite/raster.rs`)

- `run` dispatches `PathNode::ClosePath => self.run_close_path()` (replacing the
  `unreachable!`).
- `run_close_path(&mut self)` — the four-arm switch, then `points.reset()` and
  `reset_subpath()`.
- `plot_dotted(&mut self, point)` — if `cap_mode == Round`, fan all
  `pen.vertices` around `point` into `outer` and emit; else nothing. (The pen is
  built when `cap_mode == Round`.)
- `plot_closed_joined(&mut self, initial0, initial1, p1, p2)` — the closing
  join(s) (`join(self.join_mode, …)`), then
  `self.result.add_edges_from_contour(&self.outer)` and `… (&self.inner)` (both
  closed). `add_edges_from_contour` already closes each contour (last → first).

The `2`-point arm reuses `plot_single` (the existing capped single segment); the
final `run` `finish()` is a no-op after a close (`points.len == 0`).

## Scope / faithfulness notes

- **Ported**: the closed-path stroke (`plotClosedJoined`), `runClosePath` (with
  the dotted and single arms), emitting the outer and inner closed loops.
- **Deferred**: the circle/ellipse and geometric-shape glyphs that consume it (a
  later experiment builds those paths), dashes, and the sprite-kind dispatch.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/raster.rs`: wire `ClosePath` in `run`; add
   `run_close_path`, `plot_dotted`, `plot_closed_joined`.
2. Tests (deterministic):
   - `stroke_closed_square`: a closed square
     `move(0,0), line(10,0), line(10,10), line(0,10), close`, thickness 2 → the
     outer loop mitres to `[-1,11]×[-1,11]` and the inner loop to `[1,9]×[1,9]`,
     so `extent_left == -1`, `extent_right == 11`, `extent_top == -1`,
     `extent_bottom == 11`, with more edges than the same path left open (two
     closed loops vs one cap-assembled outline).
   - `stroke_close_no_panic`: a closed triangle strokes without panic into a
     non-empty polygon (the `ClosePath` arm is reachable).
   - `canvas_closed_square_ring`: via `Canvas::stroke_path` (NonZero), a closed
     square inks its **border** but leaves the **center hole** empty — the ring
     fill that distinguishes a closed stroke from a filled shape.
   - `stroke_dotted_close`: a `move + close` zero-length closed path — with
     `CapMode::Round` it emits a circle (a non-empty polygon, the pen fan around
     the point); with `CapMode::Butt` it emits nothing (an empty polygon) — per
     the design review, the only direct `plot_dotted` coverage.
3. Format and test (`cargo fmt`, accept output).

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

- `plot_closed_joined`/`run_close_path` reproduce z2d's closed-path stroke (the
  wrap-around closing join(s), the two separate closed loops, the dotted/single
  arms);
- the closed-square, no-panic, and ring tests confirm the geometry and the
  NonZero ring fill;
- the geometric-shape glyphs, dashes, and the sprite dispatch stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the closed stroke needs winding/fill-rule
handling the open-path scope did not cover (it should not — NonZero already
fills the ring).

The experiment **fails** if the closed outline diverges from z2d, or any public
C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
changes**. It confirmed: `run_close_path` matches upstream's point-count switch,
with `points.reset()` + `reset_subpath()` consistent with the existing reset
structure; `plot_closed_joined` is correct (the closing join(s) then emitting
`outer` and `inner` as separate closed contours, no concat, no caps);
`plot_dotted` is right (only `CapMode::Round` draws, fanning all pen vertices
around the point); the two oppositely-wound closed loops produce the ring under
`NonZero`; and the trailing `finish()` no-ops on `points.len == 0` after a
close, matching multi-subpath behavior. One **Optional** suggestion — add a
direct `plot_dotted` test (`move + close`: `Round` draws a circle, `Butt` draws
nothing) — folded into the plan as `stroke_dotted_close`.

Review artifacts:

- Prompt: `logs/codex-review/20260603-081120-962507-prompt.md`
- Result: `logs/codex-review/20260603-081120-962507-last-message.md`

## Result

**Result:** Pass

`roastty/src/font/sprite/raster.rs` gained the closed-path stroke:

- `run` dispatches `PathNode::ClosePath => self.run_close_path()` (the
  `unreachable!` removed).
- `run_close_path` — the point-count switch (`0` nothing / `1` `plot_dotted` /
  `2` `plot_single` / else `plot_closed_joined(head0, head1, tail2, tail1)`),
  then `points.reset()` + `reset_subpath()`.
- `plot_dotted(point)` — `CapMode::Round` fans all `pen.vertices` around the
  point into `outer` and emits a circle; other caps draw nothing.
- `plot_closed_joined(initial0, initial1, p1, p2)` — the closing join(s)
  (`p2 != initial0`: `join(p1,p2,initial0)` then `join(p2,initial0,initial1)`;
  else the single `join(p1,initial0,initial1)`), then `outer` and `inner` each
  emitted as a separate closed polygon (no concat, no caps).

Tests:

- `stroke_closed_square` — the closed square mitres to `[-1,11]×[-1,11]` (two
  concentric loops); its edges differ from the same path left open (capped).
- `stroke_close_no_panic` — a closed triangle strokes into a non-empty polygon
  (the `ClosePath` arm is reachable).
- `stroke_dotted_close` — `move + close`: `Round` draws a circle (non-empty),
  `Butt` draws nothing (empty).
- `canvas_closed_square_ring` — via `Canvas::stroke_path` (NonZero), a closed
  square inks its four border arms and leaves the center hole empty (the ring
  fill), confirmed against the hollow-square render.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2610 passed, 0 failed (+4, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The closed-path stroke completes the z2d stroke plotter: `plotClosedJoined`
wraps the contour shut around the initial point and emits the outer and inner
loops, which `NonZero` fills as a ring. The stroke subsystem now handles open
**and** closed `move_to`/`line_to`/`curve_to` paths with miter/round/bevel joins
and butt/round/square caps — the full z2d stroke.

The next sprite glyphs are the **circle/ellipse and geometric shapes**
(`geometric_shapes.zig` — filled and stroked circles/ellipses, the closed paths
this unblocks) and the remaining rect-based **special sprites** (plain/double/
dotted/dashed underlines, strikethrough, overline, cursors). The larger
remaining integration is the unifying sprite `has_codepoint`/draw and
sprite-kind dispatch (filling the resolver's deferred `SpriteUnavailable` arm),
then the discovery consumer, the UCD emoji-presentation default, codepoint
overrides, the shaper, the Nerd Font attribute table, and SVG color detection.
(Dashes remain the one deferred stroke feature, used by dashed box/underline
glyphs.)

## Completion Review

Codex reviewed the completed implementation and result and found **no Required
changes**. It confirmed `run_close_path` follows the upstream point-count switch
then clears `points` and resets subpath state (so the trailing `finish()`
no-ops); `plot_closed_joined` records the correct closing-join sequence, handles
the `p2 == initial0` degenerate case, and emits `outer` and `inner` as two
separate closed contours (no concat, no caps); `plot_dotted` matches upstream
(`Round` draws the pen circle, non-round caps draw nothing); and the ring
behavior through `Canvas::stroke_path` + `NonZero` is validated by the
hollow-square test. The added dotted-close test covers the design-review branch.
No Optional findings.

Review artifacts:

- Result review: `logs/codex-review/20260603-081445-487184-last-message.md`
