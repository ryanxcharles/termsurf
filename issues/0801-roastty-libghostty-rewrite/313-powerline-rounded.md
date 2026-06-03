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

# Experiment 313: the rounded powerline separators (E0B4–E0B7)

## Description

The rounded powerline separators — the filled `E0B4`/`E0B6` and the outlined
`E0B5`/`E0B7` — are a rectangle with a rounded right (or, flipped, left) side.
Upstream `powerline.zig` builds the rounded path with explicit `curveTo` nodes
(a quarter-circle approximation, coefficient `c = (√2 − 1)·4/3`) and either
`fillPath`s it (`E0B4`) or `innerStrokePath`s it (`E0B5`); the `E0B6`/`E0B7`
variants flip horizontally. This experiment ports `draw_powerline_rounded` over
the already-wired `Canvas::fill_path`, `Canvas::inner_stroke_path`, and
`Canvas::flip_horizontal` — no new primitives.

## Upstream behavior

With `w`/`h` the glyph dimensions, `c = (√2 − 1)·4/3`, and `r = min(w, h/2)`,
the rounded path is:

- `move(0, 0)`;
- `curve((r·c, 0), (r, r − r·c), (r, r))` — the rounded top-right corner;
- `line(r, h − r)`;
- `curve((r, h − r + r·c), (r·c, h), (0, h))` — the rounded bottom-right corner.

Then per codepoint:

- `E0B4`: `close` the path and `fillPath(.on)` — the filled separator.
- `E0B5`: `innerStrokePath` the **open** path (`line_width = box_thickness`,
  butt caps, `.on`) — the outlined separator.
- `E0B6`: `drawE0B4` then `flipHorizontal`.
- `E0B7`: `drawE0B5` then `flipHorizontal`.

## Rust mapping (`roastty/src/font/sprite/draw.rs`)

`pub(crate) fn draw_powerline_rounded(cp: u32, width: u32, height: u32, metrics: &Metrics, canvas: &mut Canvas) -> bool`
— map the codepoint to `(outlined, flip)`: `E0B4 → (false, false)`,
`E0B5 → (true, false)`, `E0B6 → (false, true)`, `E0B7 → (true, true)`;
`_ => false`. Build the open rounded node list (`MoveTo`, `CurveTo`, `LineTo`,
`CurveTo` with the control points above,
`r = (width as f64).min(height as f64 / 2.0)`,
`c = (2.0_f64.sqrt() − 1.0) · 4.0 / 3.0`). Then:

- if `outlined`:
  `canvas.inner_stroke_path(&nodes, metrics.box_thickness as f64)`;
- else: push `PathNode::ClosePath` and `canvas.fill_path(&nodes)`.

Finally, if `flip`: `canvas.flip_horizontal()`. Update the module doc.

## Scope / faithfulness notes

- **Ported**: the four rounded powerline separators (filled + outlined, each and
  its horizontal flip).
- **Deferred**: the inner-stroke powerline arrows (`E0B9`/`E0BB`/`E0BD`/`E0BF`),
  the flames (`E0D2`/`E0D4`), and the sprite dispatch.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/draw.rs`: add `draw_powerline_rounded`; update the
   module doc.
2. Tests (deterministic — the fixture `9×18` cell, `box_thickness 2`;
   `r = min(9, 9) = 9`):
   - `powerline_e0b4_filled`: the filled rounded separator inks the left side
     (`(0, 9)` inked) and the interior, and leaves the far-right beyond the
     curve empty (`(8, 0)` — the top-right corner outside the rounded arc).
   - `powerline_e0b5_outlined`: the outlined separator strokes the rounded edge
     (a point on the right curve inked) but leaves the **interior hollow** (a
     point well inside the shape, that the filled version inks, is empty).
   - `powerline_e0b6_flipped`: `E0B6` is `E0B4` mirrored — the filled body is
     now on the **right** (`(8, 9)` inked, `(0, 0)` corner empty).
   - `powerline_e0b7_outlined_flipped`: `E0B7` is `E0B5` mirrored — the outlined
     separator's rounded edge is on the **left** (a point on the left curve
     inked) with the interior hollow (per the design review — direct `E0B7`
     coverage to catch a bad `(outlined, flip)` mapping).
   - `powerline_rounded_radius`: a non-fixture dimension (`width 8`, `height 6`
     → `r = min(8, 3) = 3`) on a larger canvas confirms the radius uses
     `min(w, h/2)` (the curve fits in a `3`-radius corner, not `8` or `6`) — per
     the design review.
   - `draw_powerline_rounded_excludes`: `0x2500`, `0xE0B0`, `'M'` return `false`
     and draw nothing.
   - (The exact pixels are confirmed against the render during implementation.)
3. Format and test (`cargo fmt`, accept output).

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

- `draw_powerline_rounded` reproduces z2d's rounded separators (the curve
  control points, the filled vs inner-stroked variants, and the horizontal
  flips), returning `false` otherwise;
- the filled, outlined, flipped, and exclusion tests confirm the rendering;
- the inner-stroke arrows, the flames, and the sprite dispatch stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a separator needs a curve/stroke nuance the
fill or inner-stroke does not capture.

The experiment **fails** if a separator's geometry diverges from z2d, or any
public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised one **Required**
finding: the listed tests exercised `E0B4`/`E0B5`/`E0B6` but not `E0B7`, so a
bad `E0B7 → (outlined, flip)` mapping could slip through. Fixed: added
`powerline_e0b7_outlined_flipped` (the mirrored outlined separator — the rounded
edge on the left, interior hollow). One **Optional** suggestion — a non-fixture
dimension test (`width ≠ height/2`) since the `9×18` fixture makes
`r = min(9, 9)` and would not distinguish `min(w, h/2)` from using only `w` or
only `h/2` — folded in as `powerline_rounded_radius` (`width 8`, `height 6` →
`r = 3`). Codex confirmed the geometry and wiring are otherwise sound: the
rounded path/control points match upstream; `E0B4` uses the closed fill with no
metric thickness; `E0B5` correctly inner-strokes the original open path with
`box_thickness` and butt caps; and `E0B6`/`E0B7` as post-draw horizontal flips
match upstream.

Review artifacts:

- Prompt: `logs/codex-review/20260603-090752-168111-prompt.md`
- Result: `logs/codex-review/20260603-090752-168111-last-message.md`

## Result

**Result:** Pass

`roastty/src/font/sprite/draw.rs` gained
`draw_powerline_rounded(cp, width, height, metrics, canvas)`: the
`(outlined, flip)` dispatch (`E0B4 → (f, f)`, `E0B5 → (t, f)`, `E0B6 → (f, t)`,
`E0B7 → (t, t)`), the open rounded-right path (`move(0,0)`, the two `curve_to`s
with `c = (√2 − 1)·4/3` and `r = min(w, h/2)`, the `line(r, h − r)`), then
either `inner_stroke_path(box_thickness)` (outlined) or `ClosePath` +
`fill_path` (filled), then `flip_horizontal` when flipped.

Tests (the fixture `9×18` cell, `r = 9`), confirmed against the render:

- `powerline_e0b4_filled` — left side `(0,9)` + interior `(4,9)` filled,
  top-right `(8,0)` empty.
- `powerline_e0b5_outlined` — the right curve `(8,9)` stroked, interior `(4,9)`
  hollow.
- `powerline_e0b6_flipped` — the filled body on the right (`(8,9)` inked,
  `(0,0)` empty).
- `powerline_e0b7_outlined_flipped` — the left curve `(0,9)` stroked, interior
  `(4,9)` hollow.
- `powerline_rounded_radius` — `width 8`, `height 6` → `r = 3`: `(1,3)` inked,
  `(6,3)` empty (proving `min(w, h/2)`).
- `draw_powerline_rounded_excludes` — `0x2500`, `0xE0B0`, `'M'` return `false`
  and draw nothing.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2652 passed, 0 failed (+6, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The rounded powerline separators render faithfully — the powerline family now
covers the six solid triangles (311), the two outlined chevrons (312), and these
four rounded separators (filled + outlined, each flipped). They reuse the fill,
inner-stroke, and flip primitives with cubic quarter-circle corners — no new
infrastructure.

The remaining powerline glyphs are the **inner-stroke arrows**
(`E0B9`/`E0BB`/`E0BD`/`E0BF`) and the **flames** (`E0D2`/`E0D4`). The larger
remaining integration is the unifying sprite `has_codepoint`/draw and
**sprite-kind dispatch** (mapping the codepoint tables and a `Sprite` enum to
all the standalone `draw_*` functions, filling the resolver's deferred
`SpriteUnavailable` arm). After the sprite font: the discovery consumer, the UCD
emoji-presentation default, codepoint overrides, the shaper, the Nerd Font
attribute table, and SVG color detection.

## Completion Review

Codex reviewed the completed implementation and result and found **no Required
changes**. It confirmed the rounded path, the cubic coefficient, the radius
formula, and the dispatch all match upstream; that the filled variants close and
fill the path while the outlined variants correctly pass the original open path
to `inner_stroke_path` with `box_thickness`, then flip where upstream flips; and
that the added `E0B7` and non-`9×18` radius tests close the design gaps. No
Optional findings.

Review artifacts:

- Result review: `logs/codex-review/20260603-091047-542543-last-message.md`
