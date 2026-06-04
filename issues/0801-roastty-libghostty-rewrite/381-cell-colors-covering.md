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

# Experiment 381: the full-block twist in cell colors

## Description

`cell_colors` (Experiment 380) applies the reverse-video swap but deferred the
**`isCovering` full-block twist**: upstream swaps the **background** on
`inverse != isCovering(cp)` (not just `inverse`), so a full-block glyph (U+2588)
fills its cell via the background even without `inverse`. This experiment
completes `cell_colors` — it takes the cell's codepoint and applies that twist —
so the function is the faithful base per-cell color computation (sans
selection/search/ min-contrast). `is_covering` is already ported in `cell.rs`.

## Upstream behavior

The base (non-selected) per-cell color computation in `rebuildCells`:

```zig
// background:
.false => if (style.flags.inverse != isCovering(cell.codepoint())) fg_style else bg_style,
// foreground:
const final_bg = bg_style orelse state.colors.background;
.false => if (style.flags.inverse) final_bg else fg_style,
```

The **foreground** swaps on `inverse` alone
(`fg = inverse ? final_bg : fg_style`). The **background** swaps on
`inverse != isCovering(cp)`: for a full-block char (`isCovering` true), the
background becomes `fg_style` even when not inverse (so the solid block is
painted via the cell background, no glyph), and under inverse it stays
`bg_style`. For a non-covering char, the background swap is just `inverse` (the
Experiment 380 behavior).

## Rust mapping (`roastty/src/renderer/cell.rs`)

`cell_colors` gains a `codepoint` parameter and the `is_covering` twist on the
background:

```rust
pub(crate) fn cell_colors(
    style: TermStyle,
    codepoint: u32,
    default_fg: Rgb,
    default_bg: Rgb,
    palette: &Palette,
    bold: Option<BoldColor>,
) -> CellColors {
    let fg_style = style.resolve_fg(default_fg, palette, bold);
    let bg_style = style.resolve_bg(palette);
    let inverse = style.flags.inverse;

    // The foreground swaps to the (default-filled) background under inverse.
    let fg = if inverse {
        bg_style.unwrap_or(default_bg)
    } else {
        fg_style
    };
    // The background swaps to the foreground on `inverse != is_covering`: a full
    // block (U+2588) paints its cell via the background even without inverse.
    let bg = if inverse != is_covering(codepoint) {
        Some(fg_style)
    } else {
        bg_style
    };
    CellColors { fg, bg }
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: the `isCovering` full-block twist on the background swap
  — `cell_colors` now takes the codepoint and computes `bg` on
  `inverse != is_covering(cp)`, completing the base per-cell color computation.
- **Faithful**: the foreground is
  `inverse ? bg_style.unwrap_or(default_bg) : fg_style` and the background is
  `(inverse != is_covering(cp)) ? Some(fg_style) : bg_style` — upstream's exact
  base formulas; for a non-covering codepoint the background swap reduces to
  `inverse` (Experiment 380's behavior, unchanged); for a full block it flips so
  the block paints via the background. `is_covering` is the already-ported
  U+2588 predicate.
- **Faithful adaptation**: the added `codepoint` parameter threads the cell's
  primary codepoint (the row pass has it on each `RunCell`); no other change.
- **Deferred**: the selection/search colors, the minimum-contrast adjustment,
  the faint/dim alpha, and the integration of `cell_colors` into `rebuild_row`/
  `rebuild_bg_row` (a follow-up — now unblocked, since the twist is handled
  here). (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`: add the `codepoint` parameter to
   `cell_colors` and the `is_covering` twist on the background.
2. Tests (in `cell.rs`): extend `cell_colors_applies_reverse_video` (pass a
   non-covering codepoint, e.g. `'A'`, so its cases are unchanged) and add
   full-block (`U+2588`) cases:
   - **non-inverse full block** (`fg = Rgb(a)`, `bg = Rgb(b)`):
     `{ fg: a, bg: Some(a) }` — the block paints via the background with the
     foreground color (the twist), even without inverse;
   - **inverse full block**: `{ fg: b, bg: Some(b) }` — under inverse the
     full-block twist cancels, so it swaps to the explicit background.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty cell_colors
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `cell_colors` computes the background on `inverse != is_covering(codepoint)`
  and the foreground on `inverse`, matching upstream's base formulas;
  non-covering cells keep Experiment 380's behavior and full-block cells get the
  twist;
- the tests pass (non-covering unchanged; full block paints via the background;
  inverse full block swaps to the explicit bg), and the existing tests still
  pass;
- the selection/search/min-contrast and the integration stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the background twist is wrong (e.g. swaps on
`inverse` alone, ignoring `is_covering`, or uses the wrong combinator), the
foreground changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the design is faithful to upstream — the background
uses the XOR `style.flags.inverse != is_covering(codepoint)` while the
foreground stays controlled by `inverse` alone with
`bg_style.unwrap_or(default_bg)`, matching upstream's base non-selected
formulas, and the local `is_covering` predicate is exactly U+2588. It confirmed
the full-block expectations are correct (non-inverse U+2588 → `fg = a`,
`bg = Some(a)` because the block paints through the background with the
foreground color; inverse U+2588 → `fg = b`, `bg = Some(b)` because inverse and
covering cancel for the background while the foreground still swaps), that
adding `codepoint: u32` is the right shape (the row pass has
`RunCell. codepoint`, keeping `cell_colors` the single base color computation),
and that the test plan is sufficient (non-covering cases preserve Experiment
380, the new full-block cases prove the XOR twist and the inverse-only
foreground rule).

Review artifacts:

- Prompt: `logs/codex-review/20260603-192355-624819-prompt.md` (design)
- Result: `logs/codex-review/20260603-192355-624819-last-message.md` (design)
