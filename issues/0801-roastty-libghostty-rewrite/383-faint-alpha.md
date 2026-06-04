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

# Experiment 383: faint foreground alpha

## Description

A cell with the **faint** (dim) flag draws its foreground at a reduced opacity.
Upstream computes a per-cell foreground
`alpha = style.flags.faint ? faint_opacity : 255` and uses it for the glyph
**and** all decorations. roastty's `rebuild_row` currently uses one uniform
`alpha` for every cell's foreground. This experiment makes the foreground alpha
per-cell (faint-aware) and threads a `faint_opacity` config through
`rebuild_viewport`. (The minimum-contrast adjustment, by contrast, is a
GPU-shader concern — the CPU only sets the already-ported `no_min_contrast` flag
— so it is not a CPU experiment.)

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`):

```zig
// Foreground alpha for this cell.
const alpha: u8 = if (style.flags.faint) self.config.faint_opacity else 255;
```

This per-cell `alpha` is then passed to `addGlyph`, `addUnderline`,
`addOverline`, and `addStrikethrough` — so the glyph and every foreground
decoration of a faint cell are drawn at `faint_opacity`. The **background**
alpha is a separate computation (transparency/opacity), unaffected by faint.

## Rust mapping (`roastty/src/renderer/cell.rs`)

`rebuild_row` gains a `faint_opacity: u8` parameter. The per-column `fg_colors`
already carry an alpha channel; it becomes the faint-aware alpha, and the
decoration writers use that per-cell alpha (`fg_colors[col][3]`) instead of the
uniform `alpha`:

```rust
let fg_colors: Vec<[u8; 4]> = row_cells
    .iter()
    .map(|cell| {
        let fg = cell_colors(cell.style, cell.codepoint, default_fg, default_bg, palette, bold).fg;
        let a = if cell.style.flags.faint { faint_opacity } else { alpha };
        [fg.r, fg.g, fg.b, a]
    })
    .collect();

// …in the decoration passes, the alpha is the cell's faint-aware foreground
// alpha (`fg_colors[col][3]`), not the uniform `alpha`:
add_underline(contents, grid, grid_pos, flags.underline, underline_color, rgba[3])?;
if flags.overline { add_overline(contents, grid, grid_pos, fg, rgba[3])?; }
// …and the strikethrough pass:
add_strikethrough(contents, grid, grid_pos, [rgba[0], rgba[1], rgba[2]], rgba[3])?;
```

`add_run` already reads `fg_colors[col]` (including its alpha), so the glyph
already uses the per-cell alpha. `rebuild_viewport` gains a `faint_opacity`
parameter and passes it to `rebuild_row`.

## Scope / faithfulness notes

- **Ported (bridged)**: the per-cell faint foreground alpha — a faint cell's
  glyph and decorations draw at `faint_opacity`, a non-faint cell's at the base
  `alpha` (255), matching upstream's `alpha = faint ? faint_opacity : 255`.
- **Faithful**: the alpha is `faint_opacity` when `style.flags.faint`, else the
  base `alpha`; it is used for the glyph (via `fg_colors[col]`, already consumed
  by `add_run`) and for all three decoration writers (now `fg_colors[col][3]`),
  as upstream uses the one `alpha` for `addGlyph`/`addUnderline`/`addOverline`/
  `addStrikethrough`; the background alpha (`rebuild_bg_row`) is unchanged
  (faint is foreground-only).
- **Faithful adaptation**: `faint_opacity` is the renderer config (upstream's
  `@ceil(config.faint-opacity * 255)`), threaded through `rebuild_viewport`; the
  decoration passes switch from the uniform `alpha` to the per-cell
  `fg_colors[col][3]` so they share the glyph's faint alpha.
- **Deferred**: the background-alpha (transparency/opacity) computation; the
  selection/search colors; the lock-cursor glyph; the column-ordered decoration
  merge and link double-underline; and the Metal upload. (Consumed by tests
  now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`:
   - `rebuild_row`: add a `faint_opacity: u8` param; make `fg_colors`'s alpha
     `faint ? faint_opacity : alpha`; the decoration passes use
     `fg_colors[col] [3]` for the decoration alpha.
   - `rebuild_viewport`: add a `faint_opacity` param; pass it to `rebuild_row`.
   - Update the existing `rebuild_row`/`rebuild_viewport` test call sites.
2. Test (in `cell.rs`): a 1×1 row with one cell `'A'` that is **faint** and has
   **underline + overline + strikethrough**, plus a matching `ShapedRun`; after
   `rebuild_row` (with `alpha = 255`, `faint_opacity = 128`), assert **all
   four** `fg_rows[1]` cells — the underline, overline, glyph, and strikethrough
   — carry alpha `128` (`color[3] == 128`), so every decoration writer (not just
   underline) uses the faint-aware alpha; and a **non-faint** cell (separate)
   carries alpha `255` — proving the faint alpha reaches the glyph and all three
   decorations, and a non-faint cell is unaffected.
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

- `rebuild_row` draws a faint cell's glyph and decorations at `faint_opacity`
  and a non-faint cell's at the base `alpha` — faithful to upstream's per-cell
  foreground alpha;
- the test passes (faint glyph + underline at `faint_opacity`; non-faint at the
  base alpha), and the existing tests still pass (updated for the new
  signatures);
- the background-alpha, selection, and Metal upload stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the faint alpha is mis-applied (wrong cells, the
decorations keep the uniform alpha, the background alpha changes), or any public
C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Required** finding, now addressed:

- **Required (addressed):** the test covered only the underline, so it would
  pass even if `add_overline`/`add_strikethrough` accidentally kept the uniform
  alpha. The test now uses one faint cell with **all three** decorations and
  asserts all four `fg_rows[1]` vertices (underline, overline, glyph,
  strikethrough) have alpha `128`, plus a non-faint cell at `255`.

Codex confirmed the design is otherwise faithful: upstream computes one per-cell
foreground alpha and passes it to `addGlyph`/`addUnderline`/`addOverline`/
`addStrikethrough`, and using `fg_colors[col][3]` gives roastty the same
behavior while keeping `add_run` unchanged; leaving `rebuild_bg_row` alone is
correct (faint is foreground-only and the background alpha is a separate
upstream computation); and the min-contrast note is correct (it is a
shader/uniform concern, not a CPU experiment — the CPU already sets
`no_min_contrast`).

Review artifacts:

- Prompt: `logs/codex-review/20260603-193919-603610-prompt.md` (design)
- Result: `logs/codex-review/20260603-193919-603610-last-message.md` (design)
