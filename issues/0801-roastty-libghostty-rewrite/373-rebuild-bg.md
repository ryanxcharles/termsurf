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

# Experiment 373: the background-cell row

## Description

The foreground text path is complete (Experiments 358–372). The next renderer
subsystem is the **background cells**: each cell whose style has an explicit
background color paints a `CellBg` behind its glyph. `Contents` already stores
the per-cell backgrounds (`bg_cells`, written via `bg_cell_mut`);
`Style::bg_color` already resolves a cell's background to an `Rgb` — but it is
`pub(super)`. This experiment exposes a `pub(crate)` `resolve_bg` (mirroring
`resolve_fg`) and adds `rebuild_bg_row`, which writes each cell's background
into `Contents`.

## Upstream behavior

`rebuildCells` (`renderer/generic.zig`), per cell, resolves the background color
and writes the background cell. A cell with no explicit background
(`Color::None`) uses the screen's default background — which the renderer leaves
as the cleared/transparent slot (no per-cell overdraw), painting the default
background underneath separately. roastty's `Style::bg_color(palette)` returns
`Some(rgb)` for a `Palette`/`Rgb` background and `None` for `Color::None`; this
experiment writes a `CellBg` for the `Some` case and leaves `None` cells
transparent (`[0, 0, 0, 0]`, the `resize`/`clear` default).

## Rust mapping

`roastty/src/terminal/style.rs` — expose the resolution (mirrors `resolve_fg`):

```rust
/// Resolve this cell's background to an [`Rgb`], or `None` for the default
/// (`Color::None`). A `pub(crate)` wrapper over the (terminal-internal)
/// [`Self::bg_color`] so the renderer can resolve backgrounds.
pub(crate) fn resolve_bg(self, palette: &Palette) -> Option<Rgb> {
    self.bg_color(palette)
}
```

`roastty/src/renderer/cell.rs` — write a row's backgrounds:

```rust
/// Write one viewport row's background cells into `contents`. Each cell with an
/// explicit background (`Style::resolve_bg` → `Some`) paints a [`CellBg`] at its
/// column with `alpha`; cells with the default background (`None`) stay
/// transparent (the cleared slot). The background half of upstream `rebuildCells`'s
/// per-cell work.
pub(crate) fn rebuild_bg_row(
    contents: &mut Contents,
    y: u16,
    row_cells: &[RunCell],
    palette: &Palette,
    alpha: u8,
) {
    let row = usize::from(y);
    for (col, cell) in row_cells.iter().enumerate() {
        // Write every cell — a default (`None`) background is an active clear, so
        // a stale background from a prior rebuild does not linger.
        let bg = cell
            .style
            .resolve_bg(palette)
            .map(|rgb| CellBg([rgb.r, rgb.g, rgb.b, alpha]))
            .unwrap_or(CellBg([0, 0, 0, 0]));
        *contents.bg_cell_mut(row, col) = bg;
    }
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: a `pub(crate)` `resolve_bg` entry to `Style::bg_color`,
  and the per-row background write (`rebuild_bg_row`) — each cell's explicit
  background painted into `Contents.bg_cells`, the background half of upstream
  `rebuildCells`'s per-cell work.
- **Faithful**: `resolve_bg` delegates verbatim to the ported `Style::bg_color`
  (`Color::None → None`, `Palette(idx) → palette[idx]`, `Rgb(rgb) → rgb`); a
  `None` cell is **actively** written transparent (`[0, 0, 0, 0]`), so the
  screen's default background shows through with no overdraw — and a stale
  background from a prior rebuild cannot linger (the function rebuilds every
  cell of the row).
- **Faithful adaptation**: `rebuild_bg_row` writes `bg_cell_mut(row, col)` with
  `row = y` (the `Contents` background buffer is indexed by the raw row, unlike
  the foreground rows' cursor-offset `+ 1`); `alpha` is supplied by the renderer
  (background opacity is a deferred renderer concern).
- **Deferred**: the renderer-layer background adjustments — the
  default-background fill, background opacity, reverse-video (`inverse`) fg/bg
  swap, and selection background — and the outer loop that calls
  `rebuild_bg_row` per viewport row (alongside `rebuild_row`), plus the Metal
  upload. (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/terminal/style.rs`: add the `pub(crate) Style::resolve_bg`
   wrapper over `Style::bg_color`.
2. `roastty/src/renderer/cell.rs`: add the `rebuild_bg_row` function.
3. Tests:
   - `style.rs`: `resolve_bg` matches `bg_color` — `Color::None → None`,
     `Color::Palette(2) → palette[2]`, `Color::Rgb(x) → x`.
   - `cell.rs`: a 2×2 `Contents`, a row of two `RunCell`s — one with
     `bg_color = Color::Palette(1)`, one with the default (`None`) background.
     **Pre-seed** `bg_cell_mut(0, 1)` to a nonzero `CellBg` (a stale
     background), then call `rebuild_bg_row(y = 0, alpha = 255)`: assert
     `bg_cell(0, 0) == CellBg([palette[1].r, .g, .b, 255])` and
     `bg_cell(0, 1) == CellBg([0, 0, 0, 0])` — the default cell is actively
     cleared, proving "None means transparent" independent of a fresh
     `Contents`.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty resolve_bg
cargo test -p roastty rebuild_bg_row
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `resolve_bg` exposes `Style::bg_color` unchanged, and `rebuild_bg_row` writes
  each explicit-background cell into `Contents` while leaving default cells
  transparent — faithful to upstream `rebuildCells`'s per-cell background;
- the tests pass (resolve cases; the row write + transparent default), and the
  existing tests still pass;
- the background adjustments, the outer loop, and the Metal upload stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a default-background cell is painted (or an explicit
one skipped), the column/row index is wrong, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Required** finding, now addressed:

- **Required (addressed):** the original `rebuild_bg_row` only wrote the
  `Some`-background cells, leaving `None` cells untouched — so a stale `CellBg`
  from a prior rebuild would linger if a cell's background changed to default.
  Since the function rebuilds the row's backgrounds, the `None` branch now
  **actively** writes `CellBg([0, 0, 0, 0])` (via
  `resolve_bg(...).map(...).unwrap_or(transparent)`), and the test pre-seeds a
  nonzero background on the default cell before the call to prove it is cleared.

Codex confirmed the rest is sound: `resolve_bg` as a `pub(crate)` wrapper over
`Style::bg_color` is faithful; background indexing uses the raw row
(`bg_cell_mut(row, col)`, flat `row * columns + col`), not the foreground `+ 1`
cursor offset; and passing `alpha` is the right renderer-facing boundary while
the default-background fill, opacity policy, selection, and inverse handling
stay deferred.

Review artifacts:

- Prompt: `logs/codex-review/20260603-183624-528353-prompt.md` (design)
- Result: `logs/codex-review/20260603-183624-528353-last-message.md` (design)

## Result

**Result:** Pass

The background-cell row is in place — the first non-foreground renderer
subsystem.

- `roastty/src/terminal/style.rs`:
  `Style::resolve_bg(self, palette) -> Option<Rgb>` added as a `pub(crate)`
  wrapper over the (still `pub(super)`) `Style::bg_color` — a pure pass-through
  (`Color::None → None`, `Palette(idx) → palette[idx]`, `Rgb(rgb) → rgb`).
- `roastty/src/renderer/cell.rs`:
  `rebuild_bg_row(contents, y, row_cells, palette, alpha)` writes each column's
  background into `Contents.bg_cells` — `Some(rgb) → CellBg([r, g, b, alpha])`,
  and `None` **actively** written transparent (`[0, 0, 0, 0]`) so a stale
  background cannot linger. Uses the raw row index `bg_cell_mut(y, col)` (no
  foreground `+ 1` cursor offset).

Tests:

- `style.rs` `resolve_bg_delegates_to_bg_color` — `None`,
  `Palette(2) → DEFAULT_PALETTE[2]`, `Rgb(x) → x`.
- `cell.rs` `rebuild_bg_row_writes_and_clears` — a 2×2 `Contents` with column 1
  **pre-seeded** to `CellBg([1, 2, 3, 4])`; a row
  `[bg = Palette(1), bg = None]`; after `rebuild_bg_row(0, 255)`,
  `bg_cell(0, 0) == CellBg([p1.r, p1.g, p1.b, 255])` and
  `bg_cell(0, 1) == CellBg([0, 0, 0, 0])` — the stale background is cleared.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2824 passed, 0 failed (+2, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The renderer can now paint per-cell backgrounds: each row's explicit-background
cells become `CellBg`s in `Contents`, and default cells are cleared. With the
foreground text (Experiments 358–372) and the background row (this experiment),
`Contents` holds both the foreground glyphs and the background colors a viewport
needs.

The remaining renderer-bridge work: wire `rebuild_bg_row` into the viewport loop
(alongside `rebuild_row`); the **decorations** (underline/strikethrough/overline
cells); the **cursor** cell; the renderer-layer color adjustments
(reverse-video, selection, min-contrast, faint/dim alpha, default-background
fill, opacity); and the **Metal upload** of `Contents`.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed `resolve_bg` is a faithful wrapper (delegates
directly to `Style::bg_color` with no logic change), that `rebuild_bg_row`
correctly maps `Some(rgb)` to `CellBg([r, g, b, alpha])` and **actively** writes
transparent for `None` (so stale backgrounds cannot linger), and that the
indexing is correct (raw row/column via `bg_cell_mut(row, col)`, no foreground
`+ 1` offset). It confirmed the pre-seeded test proves the Required fix — a
stale background at `(0, 1)` is cleared when the cell resolves to
default/`None`. Nothing needed to change before the result commit.

Review artifacts:

- Result review: `logs/codex-review/20260603-183909-151437-last-message.md`
