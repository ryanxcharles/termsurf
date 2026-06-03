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

# Experiment 370: exposing foreground color resolution

## Description

The other per-row input the outer `rebuildCells` loop needs is each column's
**foreground color** (RGBA). `Style::fg` already resolves a cell's foreground to
an `Rgb` (palette lookup, default fallback, bold-brightening) — but it is
`pub(super)` and takes the `pub(super)` `Fg` options struct, so the renderer
cannot call it. This experiment exposes a `pub(crate)` `resolve_fg` wrapper (and
widens `BoldColor` to `pub(crate)`) so the renderer can resolve each cell's
foreground from its style, the default color, the palette, and the bold config —
the `fg_colors` primitive the outer loop will map over a row's cells.

## Upstream behavior

Upstream's renderer resolves a cell's foreground with
`style.fg(.{ .default = …, .palette = …, .bold = config.bold_color })` — exactly
roastty's already-ported `Style::fg(Fg { default, palette, bold })`. The
renderer holds the `default` foreground, the `palette`, and a `?BoldColor` from
its config and passes them in. roastty's resolution is faithful
(`Color::None → default` or the bold color; `Color::Palette(idx)` →
`palette[idx]`, or the bright variant `palette[idx + 8]` when bold and
`idx < 8`; `Color::Rgb(rgb)` → `rgb`, or the bold color when the rgb equals the
default and bold is set). Only the _visibility_ blocks the renderer — this
experiment opens it.

## Rust mapping (`roastty/src/terminal/style.rs`)

```rust
// `BoldColor` becomes pub(crate) so the renderer can express the bold config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoldColor {
    Color(Rgb),
    Bright,
}

impl Style {
    /// Resolve this cell's foreground to an [`Rgb`], given the renderer's default
    /// foreground, the active `palette`, and the bold-color config. A `pub(crate)`
    /// wrapper over the (terminal-internal) [`Self::fg`] so the renderer can
    /// resolve colors without the `pub(super)` [`Fg`] options struct.
    pub(crate) fn resolve_fg(
        self,
        default: Rgb,
        palette: &Palette,
        bold: Option<BoldColor>,
    ) -> Rgb {
        self.fg(Fg { default, palette, bold })
    }
}
```

`Fg` stays `pub(super)` (terminal-internal); `resolve_fg` builds it from the
renderer-friendly parameters.

## Scope / faithfulness notes

- **Ported (bridged)**: a `pub(crate)` entry point to the existing `Style::fg`
  resolution — the renderer can now resolve a cell's foreground to an `Rgb` from
  the style + default + palette + bold config, the input the future
  `rebuildCells` maps into `add_run`'s `fg_colors`.
- **Faithful**: `resolve_fg` delegates verbatim to the already-ported
  `Style::fg` (no new color logic) — the palette/default/bold-bright resolution
  is unchanged; `BoldColor` widened to `pub(crate)` mirrors upstream's
  `config.bold_color` crossing into the renderer.
- **Faithful adaptation**: `Fg` stays internal; `resolve_fg` is the thin
  renderer-facing wrapper (the alternative — exposing `Fg` — would leak a
  lifetime-bearing options struct). `resolve_fg` returns the base resolved
  `Rgb`; alpha is the renderer's (a separate channel).
- **Deferred**: the renderer-layer color adjustments upstream applies _after_
  the base resolution — the reverse-video (`inverse`) fg/bg swap, selection
  colors, faint/dim, and minimum-contrast — plus the row mapping
  (`cells → fg_colors`) and the outer `rebuildCells` loop. (Consumed by tests
  now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/terminal/style.rs`: widen `BoldColor` to `pub(crate)`; add the
   `pub(crate) Style::resolve_fg` wrapper over `Style::fg`.
2. Tests (in `style.rs`): assert `resolve_fg` matches `fg` for the key cases:
   - a default (`Color::None`, not bold) style → the `default` color;
   - `Color::Palette(1)` + bold + `Some(BoldColor::Bright)` → `palette[9]` (the
     bright variant);
   - `Color::Rgb(x)` → `x`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty resolve_fg
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `resolve_fg` exposes the foreground resolution to the renderer, delegating to
  `Style::fg` (palette/default/bold-bright) unchanged, with `BoldColor`
  `pub(crate)`;
- the test passes (none → default, palette+bold → bright, rgb → rgb), and the
  existing tests still pass;
- the reverse-video/selection/min-contrast adjustments and the outer loop stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if `resolve_fg` diverges from `Style::fg`, the
visibility change leaks more than intended, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed `resolve_fg` is the right minimal bridge — a
`pub(crate)` method on the already-`pub(crate)` `Style` that keeps the
lifetime-bearing `Fg` options struct internal — and that widening `BoldColor` to
`pub(crate)` is justified (the renderer must express the bold-color config,
which `Style::fg` already consumes); that the wrapper is faithful (it delegates
directly to `self.fg(Fg { default, palette, bold })` with no new logic); that
returning only the base resolved foreground is the correct scope (inverse,
selection, dim/faint, alpha, and min-contrast are renderer-layer adjustments
that stay deferred); and that the tests are sufficient for this thin exposure
(default fallback, palette brightening, RGB passthrough cover the wrapper's
parameters, with the existing `foreground_bold_behavior` test exercising the
deeper `BoldColor` behavior).

Review artifacts:

- Prompt: `logs/codex-review/20260603-182050-764343-prompt.md` (design)
- Result: `logs/codex-review/20260603-182050-764343-last-message.md` (design)
