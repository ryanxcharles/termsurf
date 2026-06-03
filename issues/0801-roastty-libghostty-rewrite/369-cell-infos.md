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

# Experiment 369: deriving the CellInfo row

## Description

`render_options` and `add_run` (Experiments 367–368) read a row's **`CellInfo`**
slice (each column's codepoint and grid width) to compute the constraint and
constraint width. The terminal already decodes a row into `font::run::RunCell`s
(Experiment 358, via `Terminal::shape_run_options`). This experiment adds the
small bridge that maps a row's `RunCell`s to the `CellInfo` slice the render
options need — so the future `rebuildCells` can feed `add_run` directly from the
shaping input. It is the `CellInfo` half of the per-row inputs (the per-column
`fg_colors` is a separate, color-resolution concern).

## Upstream behavior

Upstream's `addGlyph`/`constraintWidth` read `[]const terminal.page.Cell`
directly: each cell's `codepoint()` and `gridWidth()`. roastty's
`constraint_width`/`render_options` take a `CellInfo` view (codepoint + grid
width) instead — the adaptation established in the `constraint_width` port. The
grid width is upstream `Cell.gridWidth()`:

```zig
return switch (self.wide) {
    .narrow, .spacer_head, .spacer_tail => 1,
    .wide => 2,
};
```

A `RunCell` already carries the codepoint and the `Wide` kind, so the `CellInfo`
for a column is exactly `{ codepoint, grid_width = gridWidth(wide) }`.

## Rust mapping (`roastty/src/renderer/cell.rs`)

```rust
use crate::font::run::{RunCell, Wide};

/// The grid width of a cell from its [`Wide`] kind — upstream `Cell.gridWidth()`:
/// a wide cell spans two columns, everything else (narrow, spacer head/tail) one.
fn grid_width(wide: Wide) -> u8 {
    match wide {
        Wide::Wide => 2,
        Wide::Narrow | Wide::SpacerHead | Wide::SpacerTail => 1,
    }
}

/// Map a row's decoded [`RunCell`]s to the [`CellInfo`] slice the render options
/// read (each column's codepoint and grid width). The `CellInfo` half of the
/// per-row inputs the future `rebuildCells` feeds to [`add_run`].
pub(crate) fn cell_infos(cells: &[RunCell]) -> Vec<CellInfo> {
    cells
        .iter()
        .map(|cell| CellInfo {
            codepoint: cell.codepoint,
            grid_width: grid_width(cell.wide),
        })
        .collect()
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: the per-column `CellInfo` view (codepoint + grid width)
  for a row, derived from the row's `RunCell`s — the input `constraint_width`/
  `render_options` read, which upstream reads off the `terminal.page.Cell`
  slice.
- **Faithful**: `grid_width` is upstream `Cell.gridWidth()` exactly (`Wide → 2`,
  `Narrow`/`SpacerHead`/`SpacerTail → 1`); the codepoint is the cell's primary
  codepoint (`0` for an empty cell, which `constraint_width` already treats as a
  whitespace boundary).
- **Faithful adaptation**: roastty maps `RunCell` (the shaping input already
  decoded from the terminal page) → `CellInfo`, rather than re-reading
  `terminal.page.Cell` — the same `CellInfo` adaptation `constraint_width`/
  `render_options` already use, and the `RunCell`s are already produced by
  `Terminal::shape_run_options`.
- **Deferred**: the per-column `fg_colors` derivation (resolving each cell's
  style foreground to RGBA — a terminal color concern), the outer `rebuildCells`
  loop, the background/decoration/cursor cells, and the Metal upload. (Consumed
  by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`: add the `grid_width` helper and the
   `cell_infos` function; import `font::run::{RunCell, Wide}`.
2. Tests (in `cell.rs`): build a row of `RunCell`s — a narrow `'A'`, a wide
   `'W'` followed by its `SpacerTail`, a `SpacerHead`, and an empty cell — and
   assert `cell_infos`:
   - the codepoints round-trip (`'A'`, `'W'`, `0` for the spacers/empty as
     decoded);
   - `grid_width` is `1` for narrow, `2` for the wide cell, and `1` for **both**
     spacer kinds (`SpacerTail` and `SpacerHead`) and the empty cell — guarding
     the key faithfulness point that spacers are `1`, not `2`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty cell_infos
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `cell_infos` maps a row's `RunCell`s to the `CellInfo` slice with the
  codepoint and upstream `gridWidth` grid width — faithful to what
  `constraint_width`/`render_options` read;
- the test passes (narrow/wide/spacer/empty grid widths and codepoints), and the
  existing tests still pass;
- the `fg_colors` derivation, the outer loop, and the Metal upload stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the grid width diverges from upstream `gridWidth`
(e.g. a wide spacer mapped to 2), a codepoint is dropped, or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Required** finding, now addressed:

- **Required (addressed):** the test covered only `SpacerTail`, but the function
  maps both spacer kinds to width `1` and the key faithfulness point is "spacer
  head/tail are `1`, not `2`". The test now includes a `SpacerHead` cell too and
  asserts both spacer kinds map to grid width `1`.

Codex confirmed the rest is sound: `grid_width` exactly matches upstream
`Cell.gridWidth()` (`Wide::Wide → 2`, everything else → `1`); passing
`RunCell.codepoint` through directly is right (including `0` for empty/spacer
cells, which `constraint_width` already treats as a blank boundary); and
deferring the `fg_colors` derivation and the outer row loop is a clean scope
boundary.

Review artifacts:

- Prompt: `logs/codex-review/20260603-181341-382142-prompt.md` (design)
- Result: `logs/codex-review/20260603-181341-382142-last-message.md` (design)
