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

# Experiment 279: Block Elements (U+2580–U+259F)

## Description

The Block Elements Unicode block — the half/eighth blocks, the shade glyphs
(`░▒▓`), the full block, and the quadrants. Upstream
`font/sprite/draw/block.zig` draws all 32 with plain rectangle fills
(`canvas.rect`/`canvas.box`/`fill`), which the already-ported `Canvas` and the
Experiment 278 `Fraction`/`fill` support. This experiment ports `Shade`,
`Alignment`, `Quads`, the `blockShade`/`block`/`fullBlockShade`/`quadrant`
helpers, and the `draw2580_259F` dispatch.

## Upstream behavior (`font/sprite/draw/block.zig`, `common.zig`)

- `Shade` (`enum(u8)`): `off=0x00`, `light=0x40`, `medium=0x80`, `dark=0xc0`,
  `on=0xff` — the value is the pixel alpha; a `Color` is `@enumFromInt` of it.
- `Alignment { horizontal: {left,right,center}=center, vertical: {top,bottom,middle}=middle }`,
  with presets `upper`(top), `lower`(bottom), `left`, `right`.
- `Quads { tl, tr, bl, br: bool }`.
- `blockShade(metrics, canvas, alignment, width: f64, height: f64, shade)`:
  `w = round(cell_width * width)`, `h = round(cell_height * height)`; `x` from
  the horizontal alignment (`left→0`, `right→cell_width-w`,
  `center→(cell_width-w)/2`), `y` from the vertical alignment (`top→0`,
  `bottom→cell_height-h`, `middle→(cell_height-h)/2`);
  `canvas.rect({x,y,w,h}, shade-as-color)`.
- `block(…)` = `blockShade(…, .on)`.
- `fullBlockShade(metrics, canvas, shade)`:
  `canvas.box(0, 0, cell_width, cell_height, shade-as-color)`.
- `quadrant(metrics, canvas, quads)`: for each set quad, `fill` the corner —
  `tl→fill(zero,half,zero,half)`, `tr→fill(half,full,zero,half)`,
  `bl→fill(zero,half,half,full)`, `br→fill(half,full,half,full)`.
- `draw2580_259F(cp, …)`: the 32-arm switch (`else => unreachable`).

The dispatch (utility fractions `1/8, 1/4, 3/8, 1/2, 5/8, 3/4, 7/8`):

| range                | call                                             |
| -------------------- | ------------------------------------------------ |
| `2580`               | `block(upper, 1, 1/2)`                           |
| `2581`–`2587`        | `block(lower, 1, {1/8,1/4,3/8,1/2,5/8,3/4,7/8})` |
| `2588`               | `fullBlockShade(on)`                             |
| `2589`–`258F`        | `block(left, {7/8,3/4,5/8,1/2,3/8,1/4,1/8}, 1)`  |
| `2590`               | `block(right, 1/2, 1)`                           |
| `2591`/`2592`/`2593` | `fullBlockShade(light/medium/dark)`              |
| `2594`               | `block(upper, 1, 1/8)`                           |
| `2595`               | `block(right, 1/8, 1)`                           |
| `2596`               | `quadrant(bl)`                                   |
| `2597`               | `quadrant(br)`                                   |
| `2598`               | `quadrant(tl)`                                   |
| `2599`               | `quadrant(tl, bl, br)`                           |
| `259A`               | `quadrant(tl, br)`                               |
| `259B`               | `quadrant(tl, tr, bl)`                           |
| `259C`               | `quadrant(tl, tr, br)`                           |
| `259D`               | `quadrant(tr)`                                   |
| `259E`               | `quadrant(tr, bl)`                               |
| `259F`               | `quadrant(tr, bl, br)`                           |

## Rust mapping (`roastty/src/font/sprite/draw.rs`)

The block family joins `draw.rs` so it can reuse the in-module
`Fraction`/`fill`, `Canvas`, and the test helpers
(`fixture_metrics`/`inked`/`row_spans`/ `col_spans`). The module doc is updated
to note it now covers box **and** block glyphs.

- `#[repr(u8)] enum Shade { Off=0x00, Light=0x40, Medium=0x80, Dark=0xc0, On=0xff }`
  with `fn color(self) -> Color { Color(self as u8) }`.
- `enum HAlign { Left, Right, Center }`, `enum VAlign { Top, Bottom, Middle }`,
  `struct Alignment { horizontal: HAlign, vertical: VAlign }` with
  `const UPPER/LOWER/LEFT/RIGHT` (and a `center()` default).
- `struct Quads { tl, tr, bl, br: bool }` (`Default`).
- `fn block_shade(metrics, canvas, align: Alignment, width: f64, height: f64, shade: Shade)`:
  the faithful port — `let w = (cell_width as f64 * width).round() as u32;` (and
  `h`), the alignment `x`/`y` (`u32`, matching upstream integer math), then
  `canvas.rect(Rect { x: x as i32, y: y as i32, width: w as i32, height: h as i32 }, shade.color())`.
- `fn block(…)` = `block_shade(…, Shade::On)`.
- `fn full_block_shade(metrics, canvas, shade)`:
  `canvas.r#box(0, 0, cell_width as i32, cell_height as i32, shade.color())`.
- `fn quadrant(metrics, canvas, quads)`: `fill` each set corner with the
  upstream `Fraction` pairs.
- `fn draw_block(cp: u32, metrics: &Metrics, canvas: &mut Canvas) -> bool`: the
  32-arm `match`, `_ => false`.

## Scope / faithfulness notes

- **Deferred**: the `z2d` anti-aliased primitives (arcs/diagonals), and the
  other sprite families (eighth-block octants, braille, powerline, legacy,
  geometric). `draw_block` is a sibling dispatch to
  `draw_box_lines`/`draw_box_dashes`; the unifying `has_codepoint`/draw entry
  point is a later step.
- `Shade`/`Alignment`/`Quads` are upstream `common.zig` types; they are
  introduced here (their first consumer) and may move to a shared module when a
  second family needs them.
- The `width`/`height` are runtime `f64` (upstream `comptime`), identical
  behavior.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/draw.rs`: add `Shade` (+ `color`),
   `HAlign`/`VAlign`/ `Alignment` (+ presets), `Quads`, `block_shade`, `block`,
   `full_block_shade`, `quadrant`, and `draw_block`; update the module doc.
2. Tests (deterministic, the fixture `Metrics` — `cell_width = 9`,
   `cell_height = 18`):
   - `block_upper_half` (`0x2580`): `w=9, h=9`, upper → `rect x[0,9) y[0,9)`;
     `y[9,18)` empty.
   - `block_lower_eighth` (`0x2581`): `h=round(2.25)=2`, lower →
     `rect x[0,9) y[16,18)`.
   - `block_lower_three_eighths` (`0x2583`): `h=round(6.75)=7`, lower →
     `y[11,18)`.
   - `block_left_half` (`0x258C`): `w=round(4.5)=5`, left → `x[0,5) y[0,18)`.
   - `block_right_eighth` (`0x2595`): `w=round(1.125)=1`, right → `x[8,9)`.
   - `full_block_on` (`0x2588`): every pixel alpha `0xFF`.
   - `full_block_shades` (`0x2591/2/3`): every pixel alpha `0x40`/`0x80`/`0xC0`
     respectively (checks `Shade::color`).
   - `quadrant_bl` (`0x2596`): `fill(zero,half,half,full)` → `x[0,5) y[9,18)`,
     other corners empty.
   - `quadrant_diagonal` (`0x259A`, tl+br): TL `x[0,5)y[0,9)` and BR
     `x[4,9)y[9,18)` inked; TR `x[5,9)y[0,9)` and BL `x[0,4)y[9,18)` empty.
   - `quadrant_three` (`0x259F`, tr+bl+br): TR, BL, BR inked; TL `x[0,4)y[0,9)`
     empty.
   - `draw_block_excludes`: `0x2500`, `0x257F`, `'M'` return `false`, draw
     nothing.
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

- `draw_block` dispatches all 32 codepoints to the correct helper/args, with the
  half/eighth blocks placed per alignment, the shades carrying the right alpha,
  and the quadrants filling the right corners; it returns `false` otherwise;
- `block_shade`'s own `round`/alignment math and the quadrant `fill` corners
  match upstream;
- the `z2d` primitives and other sprite families stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if some glyph needs a primitive beyond rect/`fill`
(none expected in this block).

The experiment **fails** if any block glyph's geometry/shade diverges from
upstream or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised one **Required**
finding: the dispatch table summarized `0x2596`–`0x259F` as "the ten quadrant
combinations" without the explicit per-codepoint `Quads`. Fixed by listing each
arm (`2596 bl`, `2597 br`, `2598 tl`, `2599 tl+bl+br`, `259A tl+br`,
`259B tl+tr+bl`, `259C tl+tr+br`, `259D tr`, `259E tr+bl`, `259F tr+bl+br`) —
matching upstream `block.zig` exactly. Codex confirmed everything else is
faithful: the `blockShade` math and alignment, `Shade → Color` alpha values, the
quadrant corner `fill` pairs, and the recomputed `9×18` test expectations.

Review artifacts:

- Prompt: `logs/codex-review/20260603-005735-609888-prompt.md`
- Result: `logs/codex-review/20260603-005735-609888-last-message.md`

## Result

**Result:** Pass

`roastty/src/font/sprite/draw.rs` gained `Shade` (+ `color`), `HAlign`/`VAlign`/
`Alignment` (+ `UPPER`/`LOWER`/`LEFT`/`RIGHT`/`center`), `Quads`, `block_shade`
(the `round` + alignment placement), `block`, `full_block_shade`, `quadrant`,
and the 32-arm `draw_block` dispatch. The module doc now covers box **and**
block glyphs; `Rect` was added to the imports.

Tests (deterministic, the fixture `Metrics`):

- `block_upper_half` (`0x2580`) → `x[0,9) y[0,9)`.
- `block_lower_eighth` (`0x2581`) → `y[16,18)`; `block_lower_three_eighths`
  (`0x2583`) → `y[11,18)`.
- `block_left_half` (`0x258C`) → `x[0,5)` full height; `block_right_eighth`
  (`0x2595`) → `x[8,9)`.
- `full_block_on` (`0x2588`) → all pixels alpha `0xFF`; `full_block_shades`
  (`0x2591/2/3`) → all pixels `0x40`/`0x80`/`0xC0`.
- `quadrant_bl` (`0x2596`) → `x[0,5) y[9,18)`; `quadrant_diagonal` (`0x259A`,
  tl+br) and `quadrant_three` (`0x259F`, tr+bl+br) confirm the correct corners
  inked and the complementary corners empty.
- `draw_block_excludes` → `0x2500`, `0x257F`, `'M'` return `false`, draw
  nothing.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty sprite` → 53 passed (11 new).
- `cargo test -p roastty` → 2479 passed, 0 failed (no regressions; +11).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The Block Elements (`U+2580`–`U+259F`) are ported and pixel-verified — the
half/eighth blocks placed per alignment, the three shade glyphs carrying the
right alpha, and the ten quadrant combinations filling the correct corners. The
rect-based sprite families are progressing well on the existing `Canvas`. The
next rect-based candidates are the eighth-block/octant and sextant families
(`symbols_for_legacy_computing*`) and the rect parts of the geometric shapes;
separately, the `z2d` anti-aliased-path port remains the prerequisite for the
arcs and diagonals. Wiring the box/dash/block dispatchers under one sprite
`has_codepoint`/draw entry point — which the resolver's deferred sprite render
arm needs — becomes worthwhile once a few more families land. Alongside the
sprite font remain the discovery consumer, the UCD emoji-presentation default,
codepoint overrides, the shaper, the Nerd Font attribute table, and SVG color
detection.

## Completion Review

Codex reviewed the completed implementation and result and found **no required
changes**. It verified `draw_block` matches upstream arm-for-arm across
`0x2580`–`0x259F` (including the `q(tl, tr, bl, br)` ordering and all ten
quadrant combinations), that `block_shade`/`block`/`full_block_shade`/
`Shade::color`/`Alignment`/`Quads`/`quadrant` match upstream semantics, and that
the tests assert the expected spans and shade alphas. It judged the verification
clean.

Review artifacts:

- Prompt: `logs/codex-review/20260603-010035-327081-prompt.md`
- Result: `logs/codex-review/20260603-010035-327081-last-message.md`
