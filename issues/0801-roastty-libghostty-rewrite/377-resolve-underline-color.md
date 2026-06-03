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

# Experiment 377: exposing underline-color resolution

## Description

The underline decoration (`add_underline`, Experiment 375) takes a color.
Upstream colors an underline with the cell's **underline color** if set, else
the foreground (`style.underlineColor(palette) orelse fg`).
`Style::underline_color` already resolves it — but it is `pub(super)`. This
experiment exposes a `pub(crate)` `resolve_underline_color` wrapper (mirroring
`resolve_fg`/`resolve_bg`) so the future decoration integration can resolve the
underline color and fall back to the foreground.

## Upstream behavior

`rebuildCells` colors the underline with
`style.underlineColor(&palette) orelse fg`. roastty's
`Style::underline_color(palette)` returns `Some(rgb)` for a `Palette`/`Rgb`
underline color and `None` for `Color::None` (use the foreground). Only the
_visibility_ blocks the renderer — this experiment opens it, exactly as
Experiment 373 did for the background.

## Rust mapping (`roastty/src/terminal/style.rs`)

```rust
/// Resolve this cell's underline color to an [`Rgb`], or `None` for the default
/// (`Color::None`, meaning "use the foreground"). A `pub(crate)` wrapper over the
/// (terminal-internal) [`Self::underline_color`] so the renderer can color
/// underlines.
pub(crate) fn resolve_underline_color(self, palette: &Palette) -> Option<Rgb> {
    self.underline_color(palette)
}
```

The caller colors the underline with
`resolve_underline_color(palette) .unwrap_or(fg)`.

## Scope / faithfulness notes

- **Ported (bridged)**: a `pub(crate)` `resolve_underline_color` entry to the
  existing `Style::underline_color` resolution — the renderer can resolve a
  cell's underline color (or `None` ⇒ use the foreground), the input the
  decoration integration colors underlines with.
- **Faithful**: `resolve_underline_color` delegates verbatim to the ported
  `Style::underline_color` (`Color::None → None`, `Palette(idx) → palette[idx]`,
  `Rgb(rgb) → rgb`); the `None ⇒ foreground` fallback is the caller's
  (`unwrap_or(fg)`), matching upstream's `orelse fg`.
- **Faithful adaptation**: a thin one-line wrapper, exactly the shape of
  `resolve_fg` (Experiment 370) and `resolve_bg` (Experiment 373). It returns
  the base resolved color; strikethrough/overline use the foreground directly
  (no separate color), so they need no wrapper.
- **Deferred**: the decoration integration that calls
  `resolve_underline_color`/the decoration writers per decorated cell; the
  cursor cell; the renderer-layer color adjustments; and the Metal upload.
  (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/terminal/style.rs`: add the
   `pub(crate) Style::resolve_underline_color` wrapper over
   `Style::underline_color`.
2. Test (in `style.rs`): `resolve_underline_color` matches `underline_color` —
   `Color::None → None`, `Color::Palette(3) → palette[3]`, `Color::Rgb(x) → x`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty resolve_underline_color
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `resolve_underline_color` exposes `Style::underline_color` unchanged,
  returning the resolved underline color or `None` (use foreground) — faithful
  to upstream's `underlineColor(palette) orelse fg`;
- the test passes (none → None, palette → palette color, rgb → rgb), and the
  existing tests still pass;
- the decoration integration, cursor, color adjustments, and Metal upload stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if `resolve_underline_color` diverges from
`underline_color`, the visibility change leaks more than intended, or any public
C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed `resolve_underline_color` is the right thin exposure
(delegating directly to `Style::underline_color`, matching the existing
`resolve_bg` wrapper pattern); that returning `Option<Rgb>` is faithful (`None`
means "use foreground", and the caller does
`resolve_underline_color(palette).unwrap_or(fg)`, exactly like upstream's
`underlineColor(...) orelse fg`); and that the test is sufficient (`None`,
palette lookup, RGB passthrough cover the resolver's full behavior), with no
visibility issue beyond the intended `pub(crate)` method on the
already-`pub(crate)` `Style`.

Review artifacts:

- Prompt: `logs/codex-review/20260603-185851-606770-prompt.md` (design)
- Result: `logs/codex-review/20260603-185851-606770-last-message.md` (design)
