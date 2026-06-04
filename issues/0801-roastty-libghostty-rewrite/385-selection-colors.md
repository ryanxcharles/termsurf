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

# Experiment 385: the selection color computation

## Description

A **selected** cell draws with selection colors, not its SGR colors. Upstream
computes a per-cell `selected` state (`false` / `selection` / `search` /
`search_selected`) and, for a selected cell, takes the matching arm of the
background and foreground switches — driven by the `selection-background` /
`selection-foreground` config. This experiment ports the **`.selection`** arms
(the most common case) as a dedicated `selection_colors` function, plus the
`SelectionColor` config enum (upstream's `TerminalColor`). Search highlighting
(`.search` / `.search_selected`) and the plumbing of real selection ranges into
the row passes are deferred to follow-ups; this experiment is the color
computation itself, unit-tested in isolation.

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`), the background and foreground are
`switch (selected)`. The `.selection` arms (with `fg_style`/`bg_style` the
resolved SGR colors and `final_bg = bg_style orelse default_bg`):

```zig
// background, .selection arm:
.selection => if (self.config.selection_background) |v| switch (v) {
    .color => |color| color.toTerminalRGB(),
    .@"cell-foreground" => if (style.flags.inverse) bg_style else fg_style,
    .@"cell-background" => if (style.flags.inverse) fg_style else bg_style,
} else state.colors.foreground,

// foreground, .selection arm:
.selection => if (self.config.selection_foreground) |v| switch (v) {
    .color => |color| color.toTerminalRGB(),
    .@"cell-foreground" => if (style.flags.inverse) final_bg else fg_style,
    .@"cell-background" => if (style.flags.inverse) fg_style else final_bg,
} else state.colors.background,
```

Key points: (1) when no `selection-background`/`selection-foreground` is
configured, the selection background is the **default foreground**
(`state.colors.background` for the selection foreground) — i.e. a plain reverse;
(2) the `cell-foreground`/`cell-background` options pick the cell's own resolved
colors, swapping under `inverse`; (3) the selection background can be `null`
(`bg_style` under `cell-foreground` + `inverse`), falling back to the default
background like any other `bg`; (4) the **covering (full-block) twist does not
apply** to a selected cell — the `.selection` arm never consults
`isCovering(codepoint)`, so `selection_colors` needs no codepoint.

## Rust mapping (`roastty/src/renderer/cell.rs`)

A new `SelectionColor` enum (upstream `TerminalColor`) and a `selection_colors`
function returning the existing `CellColors`:

```rust
/// A selection/search color configuration value (upstream `TerminalColor`):
/// either an explicit color, or the cell's own resolved foreground/background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionColor {
    Color(Rgb),
    CellForeground,
    CellBackground,
}

/// Compute a *selected* cell's final colors — upstream's `.selection` arms of
/// the per-cell background/foreground switches. `background`/`foreground` are the
/// `selection-background`/`selection-foreground` config (`None` → the default
/// selection colors: the default foreground for the background, the default
/// background for the foreground — a plain reverse). The covering (full-block)
/// twist does not apply to a selected cell. Search highlighting is deferred.
#[allow(clippy::too_many_arguments)]
pub(crate) fn selection_colors(
    style: TermStyle,
    default_fg: Rgb,
    default_bg: Rgb,
    palette: &Palette,
    bold: Option<BoldColor>,
    background: Option<SelectionColor>,
    foreground: Option<SelectionColor>,
) -> CellColors {
    let fg_style = style.resolve_fg(default_fg, palette, bold);
    let bg_style = style.resolve_bg(palette);
    let inverse = style.flags.inverse;
    let final_bg = bg_style.unwrap_or(default_bg);

    // Background: `None` → the default foreground (a plain reverse). The
    // `CellForeground`/`CellBackground` options can yield `bg_style` (possibly
    // `None`, i.e. the default background), faithful to upstream.
    let bg = match background {
        None => Some(default_fg),
        Some(SelectionColor::Color(c)) => Some(c),
        Some(SelectionColor::CellForeground) => {
            if inverse {
                bg_style
            } else {
                Some(fg_style)
            }
        }
        Some(SelectionColor::CellBackground) => {
            if inverse {
                Some(fg_style)
            } else {
                bg_style
            }
        }
    };

    // Foreground: `None` → the default background (a plain reverse). The
    // cell-color options use `final_bg` (the default-filled background).
    let fg = match foreground {
        None => default_bg,
        Some(SelectionColor::Color(c)) => c,
        Some(SelectionColor::CellForeground) => {
            if inverse {
                final_bg
            } else {
                fg_style
            }
        }
        Some(SelectionColor::CellBackground) => {
            if inverse {
                fg_style
            } else {
                final_bg
            }
        }
    };

    CellColors { fg, bg }
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: the `.selection` arms of the per-cell background and
  foreground switches, and the `SelectionColor` config (upstream
  `TerminalColor`) — given the selection config, a selected cell's colors match
  upstream exactly.
- **Faithful**: the `None` config defaults are the **default foreground**
  (background) and **default background** (foreground), a plain reverse; the
  `Color` option is the explicit color; the `CellForeground`/`CellBackground`
  options pick the cell's resolved colors and swap under `inverse`, using
  `bg_style` (Option) for the background and
  `final_bg = bg_style.unwrap_or( default_bg)` for the foreground — upstream's
  exact arms; the covering twist is not consulted (upstream's `.selection` arm
  never calls `isCovering`).
- **Faithful adaptation**: roastty keeps the selection computation in a separate
  `selection_colors` function rather than as arms inside `cell_colors`'s switch
  — the same split already used for the foreground/background passes; the result
  type is the shared `CellColors`, so the row passes can later pick
  `selection_colors` vs `cell_colors` per cell from the `selected` state.
- **Deferred**: the `.search` / `.search_selected` arms (need the search
  highlight list); the per-cell `selected` state and the plumbing of real
  selection ranges into the row passes (a follow-up — `RunOptions` already
  carries a `selection` field); the `bg_alpha` selection → opaque branch (wired
  when selection reaches `rebuild_bg_row`); the Metal upload. (Consumed by tests
  now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`: add the `SelectionColor` enum and the
   `selection_colors` function (the `.selection` bg/fg arms). `cell_colors` and
   its callers are untouched.
2. Tests (in `cell.rs`): a `selection_colors_*` test covering, for a cell with
   an explicit SGR `fg = a` / `bg = b`:
   - **default config** (`None`/`None`): `bg = Some(default_fg)`,
     `fg = default_bg` (a plain reverse);
   - **explicit `Color`**: `bg = Some(c1)`, `fg = Some(c2)` colors used
     verbatim;
   - **`CellForeground`/`CellBackground`, non-inverse**: `bg` from
     `CellForeground = Some(a)` and `CellBackground = Some(b)`; `fg` from
     `CellForeground = a` and `CellBackground = b`;
   - **`CellForeground`/`CellBackground`, inverse**: the swap — `bg` from
     `CellForeground = Some(b)` and `CellBackground = Some(a)`; `fg` from
     `CellForeground = final_bg (= b)` and `CellBackground = a`;
   - a **no-explicit-bg** case proving `CellForeground` background under inverse
     yields `bg = None` (falls back to the default background) and the
     foreground `CellBackground` non-inverse yields `final_bg = default_bg`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty selection_colors
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `selection_colors` computes a selected cell's background and foreground per
  the `.selection` arms — `None` → a plain reverse (default fg/bg), `Color` →
  verbatim, `CellForeground`/`CellBackground` → the cell's resolved colors
  swapped under `inverse`, with the background an Option (default-fallback) —
  and the covering twist is not consulted;
- the tests pass (the config × inverse matrix above, plus the no-bg
  default-fallback case), and the existing tests still pass;
- the `.search` arms, the selection-range plumbing, and the Metal upload stay
  deferred; `cell_colors` is unchanged;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a selection color is wrong (the `None` defaults
crossed, the inverse swap inverted, the background made non-optional, or the
covering twist wrongly applied), or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the mapping is faithful to upstream's `.selection`
arms: the `None` defaults are the correct plain-reverse colors (background → the
default foreground, foreground → the default background); the
`CellForeground`/`CellBackground` options swap under `inverse` exactly as
upstream; keeping the selected background as `Option<Rgb>` is correct
(upstream's background can remain null before the later `bg orelse default_bg`
fallback and the selection-opaque alpha), while the foreground is a plain `Rgb`
using `final_bg = bg_style orelse default_bg`. It confirmed `selection_colors`
correctly takes **no codepoint** — the full-block `isCovering` twist lives only
in the non-selected background arm, never the `.selection` arm. It agreed that
keeping this as a separate function is a clean adaptation for the current split
row passes (giving follow-up plumbing a simple per-cell choice between
`cell_colors` and `selection_colors`), that deferring
`.search`/`.search_selected` and the selection-range integration is reasonable,
and that the proposed tests (default config, explicit colors, the
cell-fg/cell-bg matrix under inverse and non-inverse, plus the no-explicit-bg
fallback) are sufficient.

Review artifacts:

- Prompt: `logs/codex-review/20260603-195744-901792-prompt.md` (design)
- Result: `logs/codex-review/20260603-195744-901792-last-message.md` (design)
