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

# Experiment 309: the dashed underline

## Description

The dashed underline is the last **rect-based** underline decoration. Upstream
`special.zig`'s `underline_dashed` draws a row of dashes — alternating
full-thickness rects across the width — at the (clamped) underline position. It
needs no new infrastructure (unlike the dotted underline, which fills circles
via an arc primitive). This experiment ports it as a standalone
`draw_underline_dashed`.

## Upstream behavior (`special.zig` `underline_dashed`)

With `width`/`height` the glyph dimensions and `metrics` the cell metrics:

- `y = min(underline_position, (height + padding_y) -| underline_thickness)`
  (the underline clamp, saturating).
- `dash_width = width / 3 + 1` (integer); `dash_count = width / dash_width + 1`.
- for `i` stepping by **2** while `i < dash_count`: a rect at
  `x = i · dash_width`, `y`, width `dash_width`, height `underline_thickness`,
  `.on`.

So dashes are drawn at even-index slots (`0, 2, 4, …`), leaving gaps at the odd
slots.

## Rust mapping (`roastty/src/font/sprite/draw.rs`)

`pub(crate) fn draw_underline_dashed(canvas: &mut Canvas, width: u32, height: u32, metrics: &Metrics)`:

- `let thick = metrics.underline_thickness;`
- `let limit = height.saturating_add(canvas.padding_y()).saturating_sub(thick);`
- `let y = metrics.underline_position.min(limit);`
- `let dash_width = width / 3 + 1;` (always `≥ 1`, so no division by zero);
  `let dash_count = width / dash_width + 1;`
- `let mut i = 0; while i < dash_count { canvas.rect(Rect { x: (i · dash_width) as i32, y: y as i32, width: dash_width as i32, height: thick as i32 }, Color::ON); i += 2; }`.

(`width`/`underline_*` are `u32`; the clamp saturates.
`dash_width = width / 3 + 1` matches upstream's plain `+ 1` — `width / 3` cannot
overflow on `+ 1`.)

## Scope / faithfulness notes

- **Ported**: the dashed underline.
- **Deferred**: the dotted underline (fills circles — needs an arc/circle
  primitive), and the sprite-kind dispatch.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/draw.rs`: add `draw_underline_dashed`; note it in
   the module doc.
2. Tests (deterministic — the fixture `9×18` cell: `width 9` gives
   `dash_width = width / 3 + 1 = 4` and
   `dash_count = width / dash_width + 1 = 3`; dashes at `i = 0` (cols 0–3) and
   `i = 2` (col 8, clipped), with a gap at cols 4–7; `underline_position 15`):
   - `underline_dashed_dashes`: at `y = 15`, the first dash (cols 0–3) is inked,
     the gap (cols 4–7) is empty, and the third slot (col 8) is inked —
     confirming the alternating dashes and the even-index stepping.
   - `underline_dashed_clamp`: a large `underline_position` clamps the dashes to
     the saturating limit (row 17) instead of drawing off the bottom.
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

- `draw_underline_dashed` reproduces z2d's `underline_dashed` (the clamped
  position, the `dash_width`/`dash_count` computation, the even-index dashes);
- the dash and clamp tests confirm the rendering;
- the dotted underline and the sprite dispatch stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the dash layout needs handling the fixture does
not exercise.

The experiment **fails** if the dashed-underline geometry diverges from z2d, or
any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
changes**. It confirmed: the underline clamp uses the right saturating
`height +| padding_y -| underline_thickness` equivalent;
`dash_width = width / 3 + 1` and `dash_count = width / dash_width + 1` preserve
integer-division behavior; the `i += 2` loop draws exactly the even-index slots;
the fixture math is right for `width = 9` (dash width 4, count 3, dashes at cols
0–3 and the clipped col 8, gap 4–7); `i · dash_width` is bounded by `width` so
practical overflow is not a concern; and deferring the dotted underline (needs
an arc/circle fill) and the sprite dispatch is appropriately scoped. No Optional
findings.

Review artifacts:

- Prompt: `logs/codex-review/20260603-084234-539319-prompt.md`
- Result: `logs/codex-review/20260603-084234-539319-last-message.md`

## Result

**Result:** Pass

`roastty/src/font/sprite/draw.rs` gained `draw_underline_dashed`: the saturating
underline clamp (`height.saturating_add(padding_y()).saturating_sub(thick)`),
`dash_width = width / 3 + 1`, `dash_count = width / dash_width + 1`, and the
even- index loop (`i += 2`) drawing a `dash_width × underline_thickness` rect at
`x = i · dash_width`, `.on`.

Tests (the fixture `9×18` cell — `dash_width 4`, `dash_count 3`):

- `underline_dashed_dashes` — at `y = 15`, the first dash (cols 0–3) is inked,
  the gap (cols 4–7) is empty, and the third slot (col 8, clipped) is inked.
- `underline_dashed_clamp` — a large `underline_position` clamps the dashes to
  row 17.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2634 passed, 0 failed (+2, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The dashed underline renders faithfully — the last rect-based underline
decoration. The sprite font's special sprites now cover the underlines (plain,
double, dashed, curly), strikethrough, overline, and the four cursors; only the
**dotted** underline remains, which fills circles via an arc/circle primitive.

The next sprite-font steps are the **dotted** underline (needs a circle/arc fill
— the `arc.zig` cubic approximation feeding `Canvas::fill_path`, or a pen-based
circle polygon) and then the unifying sprite `has_codepoint`/draw and
**sprite-kind dispatch** (mapping a `Sprite` enum and the box/braille/etc.
codepoint tables to all the standalone `draw_*` functions, filling the
resolver's deferred `SpriteUnavailable` arm). After the sprite font: the
discovery consumer, the UCD emoji-presentation default, codepoint overrides, the
shaper, the Nerd Font attribute table, and SVG color detection.

## Completion Review

Codex reviewed the completed implementation and result and found **no Required
changes**. It confirmed `draw_underline_dashed` faithfully matches upstream —
the saturating underline clamp, the integer dash-width/count math, the even-slot
loop, and the rect geometry are all transcribed correctly; the fixture test pins
the `width = 9` layout including the clipped final dash at column 8; and the
clamp test covers the low-position case. No Optional findings.

Review artifacts:

- Result review: `logs/codex-review/20260603-084408-034352-last-message.md`
