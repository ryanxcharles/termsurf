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

# Experiment 307: the rect-based special sprites (underline, double underline, strikethrough, overline)

## Description

Alongside the curly underline (Experiment 303), the special-sprite family
includes the straight line decorations: the plain **underline**, the **double**
underline, the **strikethrough**, and the **overline**. Unlike the undercurl,
these are simple opaque rectangles (`canvas.rect`), positioned from the metrics
with saturating clamps so they stay within the drawable area. This experiment
ports them as standalone draw functions (the special sprites are keyed by a
sprite kind, not a Unicode codepoint — the unifying dispatch is a later
experiment, so they are ported standalone, like `draw_underline_curly`).

## Upstream behavior (`special.zig`)

With `width`/`height` the glyph dimensions and `metrics` the cell metrics
(`underline_position`/`underline_thickness`/`strikethrough_*`/`overline_*`):

- `underline`: `y = min(underline_position, (height + padding_y) -| thickness)`
  (saturating); a full-width rect at `y`, height `underline_thickness`.
- `underline_double`:
  `y = min(underline_position, (height + padding_y) -| 2 · thickness)`; **two**
  rects — one at `y -| thickness` and one at `y + thickness` (each height
  `underline_thickness`), creating a gap where the single underline would sit.
- `strikethrough`: a full-width rect at `strikethrough_position`, height
  `strikethrough_thickness` (no clamp).
- `overline`: `y = max(overline_position, -padding_y)`; a full-width rect at
  `y`, height `overline_thickness`.

All paint the opaque `.on` source.

## Rust mapping (`roastty/src/font/sprite/draw.rs`)

Four `pub(crate)` functions, each
`(canvas: &mut Canvas, width: u32, height: u32, metrics: &Metrics)`, using the
existing `hline(canvas, 0, width as i32, y, thickness)` helper (which boxes
`0..width × y..y+thickness` with `.on`):

All `u32` arithmetic uses **saturating** ops at every stage to match Zig's
`+|`/`-|` (a plain Rust `+` could overflow before the later `saturating_sub`):

- `draw_underline`:
  `let limit = height.saturating_add(canvas.padding_y()).saturating_sub(metrics.underline_thickness);`
  `let y = metrics.underline_position.min(limit);`
  `hline(canvas, 0, width as i32, y as i32, metrics.underline_thickness)`.
- `draw_underline_double`: `let thick = metrics.underline_thickness;`
  `let limit = height.saturating_add(canvas.padding_y()).saturating_sub(thick.saturating_mul(2));`
  `let y = metrics.underline_position.min(limit);` `hline` at
  `y.saturating_sub(thick) as i32` and at `y.saturating_add(thick) as i32` (the
  lower line is upstream's `y +| thickness`), each height `thick`.
- `draw_strikethrough`:
  `hline(canvas, 0, width as i32, metrics.strikethrough_position as i32, metrics.strikethrough_thickness)`.
- `draw_overline`:
  `let y = metrics.overline_position.max(-(canvas.padding_y() as i32));`
  `hline(canvas, 0, width as i32, y, metrics.overline_thickness)` (negative `y`
  draws into the top padding, clipped by `pixel()`).

(`underline_position`/`underline_thickness`/`strikethrough_*` are `u32`;
`overline_position` is `i32`. The `u32` clamps saturate at every stage; the
`overline` `max` is on `i32`.)

## Scope / faithfulness notes

- **Ported**: the four rect-based special-sprite decorations.
- **Deferred**: the dotted/dashed underlines (which need the dash/dot stroke),
  the cursors, and the sprite-kind dispatch.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/draw.rs`: add `draw_underline`,
   `draw_underline_double`, `draw_strikethrough`, `draw_overline`; note them in
   the module doc.
2. Tests (deterministic — the fixture `9×18` cell, unpadded;
   `underline_position 15`, `underline_thickness 1`, `strikethrough_position 9`,
   `overline_position 0`, each thickness 1):
   - `underline_row`: `draw_underline` inks the full width at `y = 15` and the
     rows above/below are clear of it.
   - `underline_double_gap`: `draw_underline_double` inks two rows (at `14` and
     `16`) with a clear gap row between them (`15`).
   - `strikethrough_row`: `draw_strikethrough` inks the full width at `y = 9`.
   - `overline_row`: `draw_overline` inks the full width at `y = 0` (the top
     row).
   - `underline_clamp`: a metrics variant with a large `underline_position`
     (past the cell) clamps the underline to the saturating limit
     (`height + padding_y − thickness`) instead of drawing off the bottom —
     exercising the saturating arithmetic (per the design review).
   - `overline_negative`: with `overline_position = -1` and a padded canvas, the
     overline draws into the top padding (`y = -1`, read back as inked above the
     cell) rather than clamping to `0` — exercising the `max(pos, −padding_y)`
     and the negative-`y` path (per the design review).
   - (The exact rows are confirmed against the render during implementation.)
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

- the four functions reproduce z2d's `underline`/`underline_double`/
  `strikethrough`/`overline` (the saturating-clamped positions, the full-width
  rects, the double underline's gap);
- the row tests confirm the rendering;
- the dotted/dashed underlines, the cursors, and the sprite dispatch stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a decoration needs clamp/position handling the
fixture does not exercise.

The experiment **fails** if a decoration's geometry diverges from z2d, or any
public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised two **Required**
findings, both fixed: (1) the underline clamp must saturate at **every** stage
(`height.saturating_add(padding_y).saturating_sub(thickness)`) — a plain Rust
`+` could overflow before the `saturating_sub`, whereas upstream
`height +| padding_y -| thickness` saturates both; (2) `underline_double` must
likewise use `saturating_add` for `height + padding_y` (and `saturating_mul` for
`2 · thickness`), and the **lower** line uses `y.saturating_add(thick)`
(upstream's `y +| thickness`), not a plain `y + thick`. The Rust mapping now
uses saturating ops throughout. Codex confirmed the rest is sound:
`strikethrough` has no clamp upstream; `overline`'s
`max(overline_position, −padding_y)` with a negative `y` drawing into the top
padding (via `pixel()`) is correct; and porting these four standalone (deferring
dotted/dashed/cursors/dispatch) is disciplined. Two **Optional** suggestions,
both folded in: a clamp-focused test (large `underline_position`) and an
`overline_position < 0` + padding test.

Review artifacts:

- Prompt: `logs/codex-review/20260603-083112-077555-prompt.md`
- Result: `logs/codex-review/20260603-083112-077555-last-message.md`

## Result

**Result:** Pass

`roastty/src/font/sprite/draw.rs` gained four rect-based special-sprite
decorations, each `(canvas, width, height, metrics)` over the `hline` helper:

- `draw_underline` — a full-width rect at `underline_position`, clamped by
  `height.saturating_add(padding_y()).saturating_sub(thickness)`.
- `draw_underline_double` — two rects at `y.saturating_sub(thick)` and
  `y.saturating_add(thick)` (the gap where the single underline would sit), `y`
  clamped with `saturating_add`/`saturating_mul(2)`.
- `draw_strikethrough` — an unclamped full-width rect at
  `strikethrough_position`.
- `draw_overline` — a full-width rect at
  `overline_position.max(-(padding_y() as i32))` (negative `y` draws into the
  top padding).

Tests (the fixture `9×18` cell):

- `underline_row` (`y = 15`, rows 14/16 clear), `underline_double_gap` (rows
  14 + 16, gap row 15 clear), `strikethrough_row` (`y = 9`), `overline_row`
  (`y = 0`) — the normal full-width rows.
- `underline_clamp` — a large `underline_position` (100) clamps to row 17 (the
  saturating limit) instead of drawing off the bottom.
- `overline_negative` — `overline_position = -1` with `padding_y = 2` draws at
  cell `y = -1` (into the top padding), leaving the cell's top row clear.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2627 passed, 0 failed (+6, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The straight line decorations (underline, double underline, strikethrough,
overline) render faithfully — the rect-based half of the special-sprite family,
alongside the curly underline (Experiment 303). The saturating-clamped positions
match upstream's `+|`/`-|` exactly.

The remaining special sprites are the **dotted** and **dashed** underlines
(which need the dash/dot stroke — `painter` dashes, the one deferred stroke
feature) and the **cursors** (rect/bar/underline/block variants). The larger
remaining integration is the unifying sprite `has_codepoint`/draw and
**sprite-kind dispatch** (the special sprites are keyed by a sprite kind, so the
dispatch must map a `Sprite` enum to these standalone draw functions), then the
resolver's deferred `SpriteUnavailable` arm, the discovery consumer, the UCD
emoji-presentation default, codepoint overrides, the shaper, the Nerd Font
attribute table, and SVG color detection.

## Completion Review

Codex reviewed the completed implementation and result and found **no Required
changes**. It confirmed `draw_underline` mirrors
`height +| padding_y -| thickness`; `draw_underline_double` uses the right clamp
and the two saturated offsets around the gap; `draw_strikethrough` draws
unclamped at the metric position; and `draw_overline` clamps upward to
`-padding_y`, allowing negative cell coordinates into the top padding. It judged
the standalone scope disciplined and the tests good coverage (the normal rows
plus the underline clamp and the negative-overline behavior). No Optional
findings.

Review artifacts:

- Result review: `logs/codex-review/20260603-083459-101216-last-message.md`
