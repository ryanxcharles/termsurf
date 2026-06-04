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

# Experiment 394: the under-cursor text recolor

## Description

A **block** cursor covers its cell, so the text underneath is recolored to stay
legible — upstream sets a `uniforms.cursor_color` that the shader uses to redraw
the covered glyph. This experiment ports that color computation (upstream's
block-cursor `uniform_color`) as `cursor_text_color`: given the under-cursor
cell's style and the `cursor-text` config, it produces the recolor — an explicit
color, the cell's resolved foreground/background swapped under `inverse`, or the
default background. Its `cell-foreground`/`cell-background` resolution is
**identical** to the selection foreground arm (Experiment 385), so it reuses
`selection_colors` and takes `.fg`. This is the CPU color computation; the
uniform that carries it to the shader is part of the deferred Metal upload.

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`), for a **block** cursor the
under-cursor text color (`uniform_color`) is:

```zig
const uniform_color = if (self.config.cursor_text) |txt| blk: {
    if (txt == .color) break :blk txt.color.toTerminalRGB();
    const fg_style = cursor_style.fg(.{ .default = state.colors.foreground,
        .palette = &state.colors.palette, .bold = self.config.bold_color });
    const bg_style = cursor_style.bg(&state.cursor.cell, &state.colors.palette)
        orelse state.colors.background;
    break :blk switch (txt) {
        .@"cell-foreground" => if (cursor_style.flags.inverse) bg_style else fg_style,
        .@"cell-background" => if (cursor_style.flags.inverse) fg_style else bg_style,
        else => unreachable,
    };
} else state.colors.background;
self.uniforms.cursor_color = .{ uniform_color.r, …, 255 };
```

So: the `cursor-text` config (a `?TerminalColor`) drives it — an explicit
`.color`, or the under-cursor cell's resolved `fg_style`/`bg_style` (`bg_style`
defaulting to the default background) swapped under `inverse`, and `None` → the
default background. This is exactly the **selection foreground** resolution
(`.color`/`.cell-foreground`/`.cell-background`, `None` → default background)
that `selection_colors` already computes for its `.fg`.

## Rust mapping (`roastty/src/renderer/cell.rs`)

`cursor_text_color` delegates to `selection_colors` (Experiment 385) and takes
`.fg` — the cursor-text resolution is the selection foreground arm applied to
the under-cursor cell's style with the cursor-text config:

```rust
/// Compute the under-cursor text recolor — the color a **block** cursor's covered
/// text is redrawn with (upstream's block-cursor `uniforms.cursor_color`). Given
/// the under-cursor cell's `cursor_style` and the `cursor-text` config: an
/// explicit color, or the cell's resolved foreground/background swapped under
/// `inverse`, defaulting to the default background. Its resolution is identical to
/// the selection foreground arm (the shared `TerminalColor` foreground
/// resolution), so it reuses [`selection_colors`] and takes `.fg`.
pub(crate) fn cursor_text_color(
    cursor_style: TermStyle,
    cursor_text: Option<SelectionColor>,
    default_fg: Rgb,
    default_bg: Rgb,
    palette: &Palette,
    bold: Option<BoldColor>,
) -> Rgb {
    // The selection background config is unused — only `.fg` is taken.
    selection_colors(
        cursor_style,
        default_fg,
        default_bg,
        palette,
        bold,
        None,
        cursor_text,
    )
    .fg
}
```

`SelectionColor` (Experiment 385) is the `TerminalColor` config. The `None`
background config passed to `selection_colors` is irrelevant since only `.fg` is
used. This is the CPU-side color; only the block cursor uses it, and the uniform
that holds it is deferred to the Metal upload.

## Scope / faithfulness notes

- **Ported (bridged)**: the under-cursor text recolor color (upstream's
  block-cursor `uniform_color`) as `cursor_text_color` — the `cursor-text`
  resolution against the under-cursor cell's style.
- **Faithful**: `None` → the default background; `Color(c)` → the explicit
  color; `CellForeground`/`CellBackground` → the cell's resolved
  foreground/background (the background defaulting to the default background)
  swapped under `inverse` — upstream's exact `uniform_color` arms. Because that
  resolution equals the selection foreground arm, `cursor_text_color` reuses
  `selection_colors(...).fg`, guaranteeing the two stay consistent.
- **Faithful adaptation**: the cursor-text config is `Option<SelectionColor>`
  (upstream's `?TerminalColor`); the function delegates to `selection_colors`
  and discards the (unused) background it computes — a small computation to keep
  one source of truth for the `TerminalColor` foreground resolution.
- **Deferred**: the uniform that carries this color to the shader (and the
  `cursor_pos`/`cursor_wide` block-cursor uniforms), part of the Metal upload;
  the cursor's own color (`cursor_color` from OSC 12 / config); the
  only-for-block gating (this computes the color; the shader applies it under a
  block cursor); the column-ordered decoration merge + link double-underline.
  (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`: add the `cursor_text_color` function.
2. Tests (in `cell.rs`): a `cursor_text_color_*` test over a cell with explicit
   SGR `fg = a` / `bg = b`:
   - **`None` config** → `default_bg`;
   - **`Color(c)`** → `c`;
   - **`CellForeground`** → `a` (non-inverse) / `b` (inverse);
   - **`CellBackground`** → `b` (non-inverse) / `a` (inverse);
   - a **no-explicit-bg** cell with `CellBackground` non-inverse → `default_bg`
     (the background defaults to the default background).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty cursor_text_color
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `cursor_text_color` computes the under-cursor text recolor per the
  `cursor-text` config — `None` → the default background, `Color` → verbatim,
  `CellForeground`/`CellBackground` → the cell's resolved colors swapped under
  `inverse` — faithful to upstream's block-cursor `uniform_color`, reusing the
  selection foreground resolution;
- the tests pass (the config matrix incl. inverse and the no-bg default), and
  the existing tests still pass;
- the cursor uniforms, the cursor's own color, and the Metal upload stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a recolor is wrong (the `None` default crossed, the
inverse swap inverted, the background not defaulting to the default background),
or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the byte-identical claim: upstream's block-cursor
`uniform_color` resolution is the same as the already-ported selection
foreground arm — `None → default_bg`, explicit `Color(c) → c`,
`CellForeground → inverse ? final_bg : fg_style`,
`CellBackground → inverse ? fg_style : final_bg`, with
`final_bg = bg_style.unwrap_or(default_bg)`. It judged delegating to
`selection_colors(..., None, cursor_text).fg` an acceptable DRY adaptation (the
computed background is unused but harmless, keeping the shared `TerminalColor`
foreground resolution in one place), and `Option<SelectionColor>` the right
shape for upstream's `?TerminalColor`. It agreed that deferring the actual
uniform/Metal plumbing, the cursor's own-color handling, and the block-only
gating is reasonable for this CPU-side color slice, and that the tests cover the
important matrix (including the inverse swaps and the no-explicit-bg
defaulting).

Review artifacts:

- Prompt: `logs/codex-review/20260603-210118-011959-prompt.md` (design)
- Result: `logs/codex-review/20260603-210118-011959-last-message.md` (design)

## Result

**Result:** Pass

The under-cursor text recolor color is now computable.

- `roastty/src/renderer/cell.rs`:
  `cursor_text_color(cursor_style, cursor_text, default_fg, default_bg, palette, bold) -> Rgb`
  delegates to
  `selection_colors(cursor_style, default_fg, default_bg, palette, bold, None, cursor_text).fg`
  — the cursor-text resolution is the selection foreground arm (Experiment 385)
  applied to the under-cursor cell's style with the cursor-text config; the
  (unused) selection background is discarded. `pub(crate)` and not yet called in
  production (the uniform/Metal upload is deferred), but reachable in the
  library crate, so no dead-code warning.

Test (in `cell.rs`): `cursor_text_color_resolves_the_cursor_text_config` — over
a cell with explicit SGR `fg = a` / `bg = b`: `None` → `default_bg`; `Color(c1)`
→ `c1`; `CellForeground` → `a` (non-inverse) / `b` (inverse); `CellBackground` →
`b` (non-inverse) / `a` (inverse); and a no-explicit-bg `CellBackground`
non-inverse → `default_bg` (the background defaults to the default background).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2853 passed, 0 failed (+1, no regressions; the
  `selection_colors` tests guard the shared resolution).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer) clean; `git diff --check` clean.

## Conclusion

The under-cursor text recolor — the color a block cursor's covered glyph is
redrawn with — is now ported faithfully as `cursor_text_color`, reusing the
selection foreground resolution so the two stay consistent. Like `cell_colors` /
`selection_colors` / `cursor_text_color`, this is the CPU-side color
computation; the uniform that carries it (and the `cursor_pos`/`cursor_wide`
block-cursor uniforms) is part of the deferred Metal upload.

The remaining renderer-bridge work: the cursor's own color (`cursor_color` from
OSC 12 / config) and the block-cursor uniforms (with the Metal upload); the
column-ordered decoration merge + link double-underline; and the **Metal
upload** of `Contents`.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation matches the approved design:
`cursor_text_color` delegates exactly to
`selection_colors(cursor_style, default_fg, default_bg, palette, bold, None, cursor_text).fg`,
inheriting the already-reviewed selection foreground arm (`None → default_bg`,
the explicit color verbatim, and `CellForeground`/`CellBackground` swapped under
inverse with `final_bg = bg_style.unwrap_or(default_bg)`). It confirmed the test
covers the full intended matrix including the no-explicit-bg fallback to
`default_bg`, that the function is internal Rust only (unused until the cursor
uniform/Metal upload work lands), and that the deferred scope is reasonable,
with no public C ABI/header change. Nothing needed to change before the result
commit.

Review artifacts:

- Result review: `logs/codex-review/20260603-210309-581385-last-message.md`
