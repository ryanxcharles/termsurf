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

# Experiment 386: wire the selected state into the background pass

## Description

`selection_colors` (Experiment 385) computes a selected cell's colors, but
nothing yet decides _which_ cells are selected during a rebuild. Upstream
derives a per-cell `selected` state from the row's selection range and, for a
selected cell, uses the selection colors and forces the background **opaque**.
This experiment ports the `selected` derivation (the `.selection` half) as an
`is_selected` predicate and wires it into the **background** pass
(`rebuild_bg_row`): a selected cell's background comes from `selection_colors`
(not `cell_colors`) and its `bg_alpha` is opaque. The **foreground** pass
(`rebuild_row`) is wired in a follow-up (Experiment 387); the search-highlight
arms remain deferred. `RunOptions` already carries the per-row `selection`
bounds.

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`), the per-cell `selected` state (the
`.selection` part) is:

```zig
const x_compare = if (wide == .spacer_tail) x -| 1 else x;
if (selection) |sel| {
    if (x_compare >= sel[0] and x_compare <= sel[1]) break :selected .selection;
}
// (search highlights follow; else .false)
```

So a cell is `.selection` when its column (adjusted: a wide cell's **spacer
tail** compares one column to the left, with saturating subtraction) falls
within the row's `[start, end]` selection bounds. The background switch then
takes its `.selection` arm (Experiment 385's `selection_colors`), and the
`bg_alpha` computation makes any selected cell opaque
(`if (selected != .false) break :bg_alpha default`), checked **before** the
inverse/explicit-bg branches.

## Rust mapping (`roastty/src/renderer/cell.rs`)

A `SelectionConfig` bundles the two selection color config values, and an
`is_selected` predicate ports the derivation:

```rust
/// The `selection-background`/`selection-foreground` config (upstream's two
/// `?TerminalColor`s). `Default` (both `None`) is a plain reverse.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SelectionConfig {
    pub background: Option<SelectionColor>,
    pub foreground: Option<SelectionColor>,
}

/// Whether column `x` of a row is selected, given the row's `[start, end]`
/// selection bounds. A wide cell's spacer tail compares one column to the left
/// (saturating), faithful to upstream's `x_compare`.
fn is_selected(selection: Option<[u16; 2]>, x: u16, wide: Wide) -> bool {
    let Some([start, end]) = selection else {
        return false;
    };
    let x_compare = if matches!(wide, Wide::SpacerTail) {
        x.saturating_sub(1)
    } else {
        x
    };
    x_compare >= start && x_compare <= end
}
```

`rebuild_bg_row` gains `selection: Option<[u16; 2]>` and
`selection_config: &SelectionConfig`; per cell it picks the color source and the
alpha:

```rust
for (col, cell) in row_cells.iter().enumerate() {
    let x = u16::try_from(col).expect("viewport column fits u16");
    let selected = is_selected(selection, x, cell.wide);
    let colors = if selected {
        selection_colors(
            cell.style, default_fg, default_bg, palette, bold,
            selection_config.background, selection_config.foreground,
        )
    } else {
        cell_colors(cell.style, cell.codepoint, default_fg, default_bg, palette, bold)
    };
    // A selected cell is opaque (upstream's `bg_alpha` selection branch, checked
    // before inverse/explicit-bg).
    let has_explicit_bg = !matches!(cell.style.bg_color, Color::None);
    let bg_alpha = if selected || cell.style.flags.inverse || has_explicit_bg {
        alpha
    } else {
        0
    };
    let rgb = colors.bg.unwrap_or(default_bg);
    *contents.bg_cell_mut(row, col) = CellBg([rgb.r, rgb.g, rgb.b, bg_alpha]);
}
```

`rebuild_viewport` gains a `selection_config: &SelectionConfig` param and passes
each row's `opts.selection` (already on `RunOptions`) and the config to
`rebuild_bg_row`. `rebuild_row` (the foreground) is unchanged this experiment.

## Scope / faithfulness notes

- **Ported (bridged)**: the per-cell `selected` derivation (the `.selection`
  part) and its use in the **background** pass — a cell inside the row's
  selection bounds draws the selection background and is opaque.
- **Faithful**: `is_selected` matches upstream's `x_compare` (the spacer-tail
  one-column-left saturating adjustment) and the inclusive `[start, end]` range
  test; a selected cell's background is `selection_colors(...)` (Experiment 385,
  itself faithful) and its `bg_alpha` is opaque, the selection branch evaluated
  before inverse/explicit-bg exactly as upstream (`selected` first); a
  non-selected cell is unchanged (`cell_colors` + the Experiment 384 alpha).
- **Faithful adaptation**: the selection config is bundled into
  `SelectionConfig` (the two `?TerminalColor`s) and threaded through
  `rebuild_viewport`; the per-row bounds come from `RunOptions.selection`. The
  `selected` enum is reduced to a `bool` here (only `.selection` vs `.false`)
  because the search arms are deferred — when search lands, this becomes the
  full enum.
- **Deferred**: the foreground pass (`rebuild_row`) selection recolor
  (Experiment 387); the `.search`/`.search_selected` highlight arms; the
  `background_opacity_cells` scaling; the Metal upload. (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`:
   - add the `SelectionConfig` struct and the `is_selected` predicate;
   - `rebuild_bg_row`: add `selection`/`selection_config` params; per cell pick
     `selection_colors` vs `cell_colors` and force `bg_alpha` opaque when
     selected; update its doc comment (selection now honored).
   - `rebuild_viewport`: add a `selection_config` param; pass `opts.selection` +
     the config to `rebuild_bg_row`.
   - Update the existing `rebuild_bg_row`/`rebuild_viewport` test call sites
     (`None` bounds, `&SelectionConfig::default()`).
2. Tests (in `cell.rs`):
   - `is_selected`: a small table — outside/inside/at the bounds, `None` bounds
     (never selected), and a **spacer tail** at `end + 1` selected (its
     `x_compare = end`), where a narrow cell at the same column is not;
   - `rebuild_bg_row` with a selection: a cell inside the range with **no
     explicit bg** and **default selection config** → opaque
     `CellBg([default_fg …, alpha])` (the selection background, made opaque —
     both the recolor and the selection → opaque alpha, which the Experiment-384
     path would have left transparent); a cell **outside** the range is
     unchanged.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty is_selected
cargo test -p roastty rebuild_bg_row
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `is_selected` matches upstream's `x_compare`/range derivation (inclusive
  bounds, spacer-tail one-column-left saturating), and `rebuild_bg_row` uses
  `selection_colors` and an opaque `bg_alpha` for a selected cell while leaving
  non-selected cells unchanged;
- the tests pass (`is_selected` table incl. the spacer-tail case; the selected
  cell opaque-recolored, the unselected cell unchanged), and the existing tests
  still pass (updated for the new signatures);
- the foreground-pass recolor, the `.search` arms, and the Metal upload stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the selection derivation is wrong (exclusive bounds,
the spacer-tail adjustment missing or mis-signed), a selected cell is not opaque
or not recolored, a non-selected cell changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed `is_selected` is faithful (`None` → false, inclusive
bounds, `Wide::SpacerTail` compares with `x.saturating_sub(1)` matching
upstream's `x -| 1`), and that using `col` as `x` is correct because
`RunOptions.selection` is the per-row column bounds over the same cell vector.
It agreed that reducing upstream's `selected` enum to a bool is fine here (only
`.selection`/`.false` in scope, search deferred), and that for the current
no-opacity-config subset `selected || inverse || has_explicit_bg` is equivalent
to upstream's ordered opaque branches because all three yield the same alpha —
noting the branch order will matter again when `background_opacity_cells` lands.
It confirmed wiring only `rebuild_bg_row` now is a sound incremental step (the
renderer path is still being assembled and the upload is deferred; the
visually-incomplete intermediate is documented and the foreground recolor is
scheduled for Experiment 387), and that the proposed tests are sufficient (the
`is_selected` table covers the bounds and the spacer-tail adjustment; the
background-pass test proves a selected cell uses `selection_colors` plus the
opaque alpha while an unselected cell keeps the Experiment-384 path).

Review artifacts:

- Prompt: `logs/codex-review/20260603-200331-617770-prompt.md` (design)
- Result: `logs/codex-review/20260603-200331-617770-last-message.md` (design)

## Result

**Result:** Pass

The selected state now drives the background pass.

- `roastty/src/renderer/cell.rs`:
  - a `SelectionConfig` struct (the two `selection-background`/
    `selection-foreground` `Option<SelectionColor>` values, `Default` = a plain
    reverse) and an `is_selected(selection, x, wide)` predicate (inclusive
    `[start, end]`, `Wide::SpacerTail` compares `x.saturating_sub(1)`, `None` →
    false — upstream's `x_compare` derivation, the `.selection` part).
  - `rebuild_bg_row` (new `selection: Option<[u16; 2]>` and
    `selection_config: &SelectionConfig` params): per cell,
    `selected = is_selected(selection, col as u16, cell.wide)`; a selected
    cell's background is `selection_colors(...)` (else `cell_colors(...)`), and
    `bg_alpha = (selected || inverse || has_explicit_bg) ? alpha : 0` — the
    selection → opaque branch, faithful to upstream (all three branches yield
    the base `alpha`). The RGB still falls back to `default_bg`. Doc comment
    updated.
  - `rebuild_viewport` (new `selection_config` param): passes each row's
    `opts.selection` and the config to `rebuild_bg_row`. `rebuild_row` (the
    foreground) is untouched (Experiment 387). The existing `rebuild_bg_row`/
    `rebuild_viewport` test call sites are updated for the new signatures.

Tests (in `cell.rs`):

- `is_selected_matches_the_x_compare_derivation` — `None` → never selected;
  inclusive `[1, 3]` for a narrow cell (before/at-start/inside/at-end/after); a
  spacer tail at `end + 1 = 4` selected (its `x_compare = 3`) where a narrow
  cell at column 4 is not; the saturating edge (a spacer tail at column 0
  compares 0).
- `rebuild_bg_row_recolors_selected_cells_opaque` — two no-explicit-bg cells
  (transparent under the Exp 384 path); selecting only column 0 (default
  config): column 0 → opaque `CellBg([default_fg …, 255])` (the selection
  background, made opaque), column 1 → unchanged transparent `[0, 0, 0, 0]`.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2842 passed, 0 failed (+2, no regressions; existing
  rebuild tests preserved with updated signatures).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer) clean; `git diff --check` clean.

## Conclusion

A selection range now recolors the rebuild's **backgrounds**: a cell inside the
row's `selection` bounds draws the selection background (a plain reverse by
default, or the configured color) and is forced opaque, while non-selected cells
keep the Experiment-384 path. The `is_selected` predicate (faithful to
upstream's `x_compare`, including the spacer-tail adjustment) and
`SelectionConfig` are now in place for the foreground pass.

The remaining renderer-bridge work: the **foreground** pass (`rebuild_row`)
selection recolor (Experiment 387 — so a selected cell's glyph and decorations
take the selection foreground); the `.search`/`.search_selected` highlight arms;
the lock-cursor glyph + under-cursor text recolor; the column-ordered decoration
merge + link double-underline; and the **Metal upload** of `Contents`.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation matches the approved design:
`is_selected` returns false for `None`, uses inclusive `[start, end]`, and
applies `x.saturating_sub(1)` only for `Wide::SpacerTail` (upstream's `x -| 1`);
`rebuild_bg_row` computes `selected` from the row bounds and column, uses
`selection_colors` for selected cells and `cell_colors` otherwise, and computes
`bg_alpha` opaque for `selected || inverse || has_explicit_bg` (equivalent to
upstream's ordered branches for the current deferred-config subset, since all
included branches yield the same alpha), preserving
`rgb = colors.bg.unwrap_or( default_bg)`; `rebuild_viewport` threads
`opts.selection` and `&SelectionConfig` correctly while `rebuild_row` stays
untouched for Experiment 387. It confirmed the tests cover the selection
predicate, the spacer-tail adjustment including saturation, the selected opaque
recolor, and the unchanged unselected behavior, with the diff internal Rust only
(no public C ABI/header change). Nothing needed to change before the result
commit.

Review artifacts:

- Result review: `logs/codex-review/20260603-200731-505483-last-message.md`
