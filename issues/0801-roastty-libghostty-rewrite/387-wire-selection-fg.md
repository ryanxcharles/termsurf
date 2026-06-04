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

# Experiment 387: wire the selected state into the foreground pass

## Description

Experiment 386 recolored a selected cell's **background**, but its glyph and
decorations still use the SGR foreground. Upstream uses one selection-aware
foreground for a selected cell's glyph **and** every decoration. This experiment
wires the `selected` state into the **foreground** pass (`rebuild_row`): a
selected cell's `fg_colors` come from `selection_colors(...).fg` (not
`cell_colors(...).fg`), so the glyph, underline, overline, and strikethrough all
draw with the selection foreground. This completes the `.selection` recolor
(both halves); the search-highlight arms remain deferred.

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`), the per-cell foreground `fg` is the
`switch (selected)` result (Experiment 385's `.selection` arm for a selected
cell), and that one `fg` feeds the glyph and the decorations:

```zig
const fg = fg: { … switch (selected) { .selection => …, .false => …, … } };
// glyph:        addGlyph(…, fg, alpha);
// underline:    addUnderline(…, style.underlineColor(palette) orelse fg, alpha);
// overline:     addOverline(…, fg, alpha);
// strikethrough:addStrikethrough(…, fg, alpha);
```

So a selected cell's glyph and decorations all use the selection foreground, and
the underline color is the cell's explicit SGR underline color **or** (falling
back to) that selection foreground.

## Rust mapping (`roastty/src/renderer/cell.rs`)

`rebuild_row` already builds a per-column `fg_colors` that the glyph (`add_run`)
and all three decoration passes read. Making `fg_colors` selection-aware
therefore recolors the entire foreground in one place. `rebuild_row` gains
`selection: Option<[u16; 2]>` and `selection_config: &SelectionConfig`; the
`fg_colors` builder enumerates and picks the color source per cell:

```rust
let fg_colors: Vec<[u8; 4]> = row_cells
    .iter()
    .enumerate()
    .map(|(col, cell)| {
        let x = u16::try_from(col).expect("viewport column fits u16");
        let selected = is_selected(selection, x, cell.wide);
        let fg = if selected {
            selection_colors(
                cell.style, default_fg, default_bg, palette, bold,
                selection_config.background, selection_config.foreground,
            )
            .fg
        } else {
            cell_colors(cell.style, cell.codepoint, default_fg, default_bg, palette, bold).fg
        };
        // A faint cell's foreground draws at the reduced faint opacity.
        let a = if cell.style.flags.faint { faint_opacity } else { alpha };
        [fg.r, fg.g, fg.b, a]
    })
    .collect();
```

The decoration passes and `add_run` are unchanged — they already read
`fg_colors[col]`, and the underline-color fallback
(`resolve_underline_color( palette).unwrap_or(fg)`) now falls back to the
selection-aware foreground, matching upstream's
`underlineColor(palette) orelse fg`. `rebuild_viewport` passes each row's
`opts.selection` and the `selection_config` to `rebuild_row` (it already has
both for the background pass).

## Scope / faithfulness notes

- **Ported (bridged)**: the use of the selection foreground in the
  **foreground** pass — a selected cell's glyph and all decorations draw with
  `selection_colors(...).fg` (Experiment 385), completing the `.selection`
  recolor that Experiment 386 began on the background.
- **Faithful**: a selected cell's `fg_colors` come from
  `selection_colors(...).fg` and a non-selected cell's from
  `cell_colors(...).fg`; that one per-cell color feeds the glyph and every
  decoration (via `fg_colors[col]`), and the underline-color fallback uses it —
  upstream uses one `fg` for the glyph and the
  `addUnderline`/`addOverline`/`addStrikethrough` decorations, with the
  underline color `underlineColor(palette) orelse fg`. The faint alpha is
  unchanged (still `faint ? faint_opacity : alpha`, independent of selection, as
  upstream).
- **Faithful adaptation**: roastty recolors in one place (the `fg_colors`
  builder) rather than at each decoration call, because the passes already share
  `fg_colors`; the result is identical. The selection source (`is_selected` +
  `SelectionConfig`) is the same as the background pass (Experiment 386).
- **Deferred**: the `.search`/`.search_selected` highlight arms; the lock-cursor
  glyph + under-cursor text recolor; the column-ordered decoration merge + link
  double-underline; the Metal upload. (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`:
   - `rebuild_row`: add `selection`/`selection_config` params; build `fg_colors`
     selection-aware (`enumerate`, `selection_colors(...).fg` for a selected
     cell). Update its doc comment (foreground now selection-aware).
   - `rebuild_viewport`: pass `opts.selection` + `selection_config` to
     `rebuild_row`.
   - Update the existing `rebuild_row` test call sites (`None` bounds,
     `&SelectionConfig::default()`).
2. Tests (in `cell.rs`):
   - a small row with one cell carrying a glyph plus an underline, an overline,
     and a strikethrough, **selected**, default selection config: assert all
     four `fg_rows[1]` vertices (underline, overline, glyph, strikethrough)
     carry the selection foreground (the default background, a plain reverse),
     proving every foreground element uses the selection fg — and a separate
     **unselected** cell keeps its `cell_colors` foreground;
   - a **selected** cell with an **explicit SGR underline color**: its underline
     keeps that explicit color (not the selection fg), while its glyph uses the
     selection fg — proving the underline-color precedence (`… orelse fg`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty rebuild_row
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `rebuild_row` draws a selected cell's glyph and all decorations with the
  selection foreground (`selection_colors(...).fg`) and a non-selected cell's
  with `cell_colors(...).fg`, with the underline-color fallback using the
  selection-aware foreground — faithful to upstream's single per-cell
  foreground;
- the tests pass (a selected cell's four foreground vertices at the selection
  fg; an unselected cell unchanged; the explicit-underline-color precedence),
  and the existing tests still pass (updated for the new signatures);
- the `.search` arms, the lock-cursor recolor, and the Metal upload stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a selected cell's foreground is wrong (the glyph or
a decoration keeps the SGR fg, the underline-color precedence inverted), a
non-selected cell changes, the faint alpha is disturbed, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed that recoloring in the `fg_colors` builder is faithful
— upstream computes one per-cell `fg` from the selected-state switch and passes
that same `fg` to the glyph, the underline fallback, the overline, and the
strikethrough; roastty's glyph and decoration paths already share
`fg_colors[col]`, so selecting the color there is the right single integration
point. It confirmed the underline-color precedence is correct
(`resolve_underline_color(palette). unwrap_or(fg)` — an explicit SGR underline
color wins, else the selection-aware foreground, matching upstream's
`underlineColor(palette) orelse fg`), and that leaving the faint alpha
independent of selection is faithful (upstream computes the foreground alpha
separately from color selection, so `faint ? faint_opacity : alpha` stays
unchanged). It agreed that using the same `is_selected`/ `SelectionConfig`
inputs as the background pass keeps both passes consistent, and that the
proposed tests are sufficient (a selected cell's glyph + underline + overline +
strikethrough all at the selection foreground, an unselected cell unchanged, and
the explicit underline color preserved).

Review artifacts:

- Prompt: `logs/codex-review/20260603-201035-582739-prompt.md` (design)
- Result: `logs/codex-review/20260603-201035-582739-last-message.md` (design)

## Result

**Result:** Pass

A selected cell's foreground now draws with the selection color.

- `roastty/src/renderer/cell.rs`:
  - `rebuild_row` (new `selection: Option<[u16; 2]>` and
    `selection_config: &SelectionConfig` params): the `fg_colors` builder
    enumerates and, per cell, uses `selection_colors(...).fg` when
    `is_selected(selection, col as u16, cell.wide)` else `cell_colors(...).fg`.
    The faint alpha (`faint ? faint_opacity : alpha`) is unchanged. The
    decoration passes and `add_run` are untouched — they read `fg_colors[col]`,
    so the glyph, underline, overline, and strikethrough all inherit the
    selection foreground, and the underline-color fallback
    (`resolve_underline_color(palette).unwrap_or(fg)`) now falls back to the
    selection-aware foreground (upstream's `underlineColor(palette) orelse fg`).
    Doc comment updated.
  - `rebuild_viewport`: passes each row's `opts.selection` and the
    `selection_config` to `rebuild_row` (it already passed them to
    `rebuild_bg_row`), so both passes share the same selection state. The
    existing `rebuild_row` test call sites are updated for the new signatures.

Tests (in `cell.rs`):

- `rebuild_row_recolors_selected_foreground` — a cell `'A'` with underline +
  overline + strikethrough and no explicit colors (`default_fg = (200,200,200)`,
  `default_bg = (9,8,7)`): selected (default config) → all four `fg_rows[1]`
  vertices carry the selection foreground `(9,8,7)` (a plain reverse = the
  default background); unselected → all four carry the SGR foreground
  `(200,200,200)`.
- `rebuild_row_selected_underline_keeps_explicit_color` — a selected cell `'A'`
  with an explicit SGR underline color `(1,2,3)` and an underline: the underline
  vertex (emitted first, underneath) keeps `(1,2,3)` while the glyph vertex uses
  the selection foreground `(9,8,7)` — the `… orelse fg` precedence.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2844 passed, 0 failed (+2, no regressions; existing
  `rebuild_row` tests preserved with updated signatures).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer) clean; `git diff --check` clean.

## Conclusion

The `.selection` recolor is now complete on **both** halves of the rebuild: a
selected cell draws the selection background (Experiment 386) **and** its glyph
and every decoration draw with the selection foreground, with an explicit SGR
underline color still taking precedence — all from the same `is_selected` /
`SelectionConfig` state shared by the two passes. A default-config selection is
a faithful plain reverse; a configured
`selection-background`/`selection-foreground` flows through unchanged.

The remaining renderer-bridge work: the `.search`/`.search_selected` highlight
arms (which extend the `selected` bool to the full enum and reuse
`selection_colors`); the lock-cursor glyph + under-cursor text recolor; the
column-ordered decoration merge + link double-underline; and the **Metal
upload** of `Contents`.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation matches the approved design:
`rebuild_row` takes `selection` and `&SelectionConfig`, computes
`selected = is_selected(selection, col, cell.wide)`, and chooses
`selection_colors(...).fg` for a selected cell or `cell_colors(...).fg`
otherwise, with the faint-alpha path unchanged; because `fg_colors[col]` remains
the single source consumed by `add_run` and all decoration passes, the glyph,
underline fallback, overline, and strikethrough all inherit the selection
foreground, and the underline fallback stays faithful (explicit underline color
wins, else the selection-aware `fg`); `rebuild_viewport` passes `opts.selection`
and `selection_config` into both the background and foreground passes, so the
selection state is consistent across the two, and the existing call-site updates
(`None`/`SelectionConfig::default()`) preserve prior behavior. It confirmed the
new tests cover the selected foreground across all four foreground vertices, the
unselected-foreground preservation, and the explicit-underline-color precedence,
with the diff internal Rust only (no public C ABI/header change). Nothing needed
to change before the result commit.

Review artifacts:

- Result review: `logs/codex-review/20260603-201341-588883-last-message.md`
