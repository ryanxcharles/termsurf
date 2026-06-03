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

# Experiment 367: per-cell RenderOptions derivation

## Description

`add_glyph` (Experiment 366) takes a pre-built `RenderOptions`. Upstream builds
that `RenderOptions` **inside** `addGlyph`, per cell, from the codepoint and the
cell row: the grid metrics, the thicken config, the cell's grid width, the
constraint (Nerd Font lookup, else symbol-fit, else none), and the constraint
width. This experiment ports that derivation as `render_options` — the function
the future `rebuildCells` calls to produce each glyph's `RenderOptions`. All the
inputs already exist in roastty (`get_constraint`, `is_symbol`,
`constraint_width`); this assembles them exactly as upstream does.

## Upstream behavior

Inside `addGlyph` (`renderer/generic.zig`), the `RenderOptions` is:

```zig
.{
    .grid_metrics = self.grid_metrics,
    .thicken = self.config.font_thicken,
    .thicken_strength = self.config.font_thicken_strength,
    .cell_width = cell.gridWidth(),
    .constraint = getConstraint(cp) orelse
        if (cellpkg.isSymbol(cp)) .{ .size = .fit } else .none,
    .constraint_width = constraintWidth(cell_raws, x, cols),
}
```

- `cell_width` is the cell's grid width (1 or 2);
- `constraint` is the Nerd Font constraint for the codepoint if one exists;
  else, for a "symbol-like" codepoint, a `.fit` constraint (scale down to fit
  the cell, no alignment change); else `.none` (no constraint);
- `constraint_width` is the symbol-aware cell span (`constraintWidth`).

## Rust mapping (`roastty/src/renderer/cell.rs`)

`is_symbol` and `constraint_width` already live in `cell.rs`; `get_constraint`
is `font::face::nerd_font_attributes`. The builder reads the cell row's
`CellInfo` at `x` (the same `CellInfo` view `constraint_width` already takes):

```rust
use crate::font::face::constraint::{Constraint, Size};
use crate::font::face::nerd_font_attributes::get_constraint;
use crate::font::metrics::Metrics;

/// Build the [`RenderOptions`] for the glyph at column `x`, exactly as upstream
/// `addGlyph` does: the grid metrics and thicken config, the cell's grid width,
/// the constraint (Nerd Font lookup → else symbol `Fit` → else none), and the
/// symbol-aware `constraint_width`. The caller (the future `rebuildCells`)
/// supplies the row's `CellInfo` slice and the grid/thicken config.
pub(crate) fn render_options(
    grid_metrics: Metrics,
    raw_slice: &[CellInfo],
    x: usize,
    cols: usize,
    thicken: bool,
    thicken_strength: u8,
) -> RenderOptions {
    let cell = raw_slice[x];
    let cp = cell.codepoint;

    // Nerd Font constraint, else a symbol fits its cell, else no constraint.
    let constraint = get_constraint(cp).unwrap_or_else(|| {
        if is_symbol(cp) {
            Constraint {
                size: Size::Fit,
                ..Constraint::default()
            }
        } else {
            Constraint::default() // `.none`
        }
    });

    RenderOptions {
        grid_metrics,
        cell_width: Some(cell.grid_width),
        constraint,
        constraint_width: constraint_width(raw_slice, x, cols),
        thicken,
        thicken_strength,
    }
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: the per-cell `RenderOptions` derivation — the
  grid-metrics/thicken passthrough, `cell_width` from the cell's grid width, the
  Nerd-Font/symbol-fit/none constraint, and `constraint_width`. This is the
  opts-construction block inside upstream `addGlyph`.
- **Faithful**: the constraint is `get_constraint(cp)` if present, else
  `Size::Fit` (rest default) for a symbol, else `Constraint::default()`
  (`.none`) — matching upstream's
  `getConstraint(cp) orelse if (isSymbol(cp)) .{ .size = .fit } else .none`;
  `cell_width` is `Some(grid_width)` (upstream's `cell.gridWidth()`);
  `constraint_width` reuses the already-ported `constraint_width`;
  `grid_metrics`/`thicken`/`thicken_strength` pass through.
- **Faithful adaptation**: `Constraint::default()` is roastty's `.none` (the
  no-op constraint already used as the no-constraint case in the render tests);
  `Size::Fit` with default fields is upstream's `.{ .size = .fit }`. The builder
  takes a `CellInfo` slice (the same view `constraint_width` uses) rather than a
  `terminal.page.Cell` slice, the adaptation already established for
  `constraint_width`.
- **Deferred**: the `rebuildCells` loop that calls this (and `add_glyph`) for
  every shaped cell, deriving the `CellInfo` slice from the terminal page and
  the thicken values from the renderer config; the cursor/decoration cells; and
  the Metal upload. (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`: add the `render_options` function; import
   `Constraint`/`Size`, `get_constraint`, and `Metrics`.
2. Tests (in `cell.rs`):
   - a plain letter (`'a'`): `constraint == Constraint::default()` (none),
     `cell_width == Some(1)`, `constraint_width == 1`, and the metrics/thicken
     pass through;
   - a "symbol-like" codepoint with **no** Nerd Font entry:
     `constraint.size == Size::Fit`;
   - a codepoint **with** a Nerd Font entry (one from `get_constraint`'s table):
     `constraint == get_constraint(cp).unwrap()`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty render_options
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `render_options` assembles the `RenderOptions` exactly as upstream `addGlyph`
  (grid metrics/thicken passthrough, `cell_width` from grid width, the
  Nerd-Font/symbol-fit/none constraint, and `constraint_width`);
- the tests pass (plain → none, symbol → fit, Nerd Font → its constraint), and
  the existing tests still pass;
- the `rebuildCells` loop and the Metal upload stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the constraint precedence is wrong (symbol-fit
shadowing a Nerd Font entry, or vice versa), `cell_width`/`constraint_width` are
mis-derived, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed `render_options` matches upstream's option block:
`get_constraint(cp)` has priority, the symbol-like fallback uses
`Constraint { size: Size::Fit, ..Default::default() }`, and the non-symbol
fallback is `Constraint::default()` (`Size::None`) — preserving the correct
precedence, including Nerd Font entries overriding the generic symbol-fit path;
that `cell_width: Some(cell.grid_width)` is the right mapping for upstream's
`cell.gridWidth()` (`Some(1)` is the single-cell case, `Some(2)` carries the
wide-cell span for sprite rendering); that reusing the `CellInfo` slice is the
right adaptation because `constraint_width` already depends on that codepoint +
grid-width view; and that the test plan is sufficient (plain letter covers
`.none` plus passthrough, symbol-without-entry covers fallback `Fit`, a known
Nerd Font entry covers precedence over symbol-fit).

Review artifacts:

- Prompt: `logs/codex-review/20260603-180242-084832-prompt.md` (design)
- Result: `logs/codex-review/20260603-180242-084832-last-message.md` (design)

## Result

**Result:** Pass

The per-cell `RenderOptions` derivation is in place — the input `add_glyph`
needs, built exactly as upstream `addGlyph` builds it.

- `roastty/src/renderer/cell.rs`:
  `render_options(grid_metrics, raw_slice, x, cols, thicken, thicken_strength)`
  reads the cell's codepoint and grid width at `x` and assembles the
  `RenderOptions`: `cell_width = Some(grid_width)`,
  `constraint = get_constraint(cp)` else `Size::Fit` for a symbol else
  `Constraint::default()` (`.none`), `constraint_width = constraint_width(...)`,
  and the grid-metrics/thicken passthrough. Imported `Constraint`/`Size`,
  `get_constraint` (Nerd Font), and `Metrics`.

Tests (in `cell.rs`):

- `render_options_plain_letter_has_no_constraint` — `'a'`:
  `constraint == Constraint::default()`, `cell_width == Some(1)`,
  `constraint_width == 1`, and the `grid_metrics`/`thicken`/`thicken_strength`
  pass through;
- `render_options_symbol_without_nerd_entry_fits` — `0x1F600` (symbol, no Nerd
  entry; asserts `get_constraint == None`): `constraint.size == Size::Fit`;
- `render_options_nerd_entry_overrides_symbol_fit` — `0x2630` (a Nerd glyph that
  is also symbol-like): `constraint == get_constraint(0x2630).unwrap()` and
  `size != Fit`, proving the Nerd Font entry wins over the symbol-fit fallback.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2817 passed, 0 failed (+3, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer) clean; `git diff --check` clean.

## Conclusion

The two halves of upstream's `addGlyph` are now both ported: `render_options`
(Experiment 367) builds the per-cell `RenderOptions`, and `add_glyph`
(Experiment 366) renders and emits the cell. A `rebuildCells` loop can now, for
each shaped cell, call `render_options` then `add_glyph` — only the loop and its
terminal/config inputs (the `CellInfo` slice per row, the per-cell color/alpha,
`no_min_contrast`, the thicken config) remain.

The remaining renderer-bridge work is the **`rebuildCells` loop**: iterate the
viewport's `ShapedRun`s and their terminal cells, build each row's `CellInfo`
slice, derive color/alpha and `no_min_contrast(cp)`, call `render_options` +
`add_glyph`, and handle the background/decoration/cursor cells — then the Metal
upload of `Contents`.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation matches upstream's `addGlyph`
option construction (`get_constraint` wins first, then symbol-like codepoints
fall back to `Size::Fit`, otherwise `Constraint::default()`;
`cell_width = Some(grid_width)`, `constraint_width(...)`, and the
metrics/thicken passthrough all correct), and that the three tests cover the
important cases (plain letter → no constraint plus passthrough, `0x1F600` →
symbol fallback without a Nerd Font entry, `0x2630` → a Nerd Font entry
overriding the symbol fallback). Nothing needed to change before the result
commit.

Review artifacts:

- Result review: `logs/codex-review/20260603-180458-119364-last-message.md`
