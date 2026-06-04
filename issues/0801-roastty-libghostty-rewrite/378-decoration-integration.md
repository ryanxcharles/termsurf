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

# Experiment 378: wiring decorations into the row pass

## Description

The decoration writers (`add_underline`/`add_strikethrough`/`add_overline`,
Experiments 375–376) and the color resolvers (Experiments 370/373/377) are all
ported, but nothing calls them during a rebuild. This experiment wires them into
`rebuild_row`: for each cell, it emits the underline, overline, and
strikethrough its style flags request, in upstream's draw order —
underlines/overlines **underneath** the text and strikethrough **on top**. This
completes the per-row foreground assembly (glyphs + decorations).

## Upstream behavior

`rebuildCells` walks each column and, per cell:

1. draws the **underline** (if any) — "We draw underlines first so that they
   layer underneath text", colored `style.underlineColor(palette) orelse fg`;
2. draws the **overline** (if set) — also underneath, colored `fg`;
3. draws the **glyph(s)**;
4. finally draws the **strikethrough** (if set) — on top, colored `fg`.

The decorations and glyphs share one foreground cell list, so insertion order is
draw order — upstream appends underline/overline, glyph(s), then strikethrough
for column `x` before moving to `x + 1`. roastty assembles glyphs per **run**
(`add_run`), so this experiment brackets the run loop with two per-cell
decoration passes — underline/overline before the runs (underneath),
strikethrough after (on top). This preserves the **same-column** layering
(within each column, underline/overline < glyph < strikethrough, as upstream)
but is a deliberate adaptation, not a strict reproduction of upstream's
interleaved column-walk: see the cross-column caveat below. (The upstream
link-underline override — a hyperlinked cell's underline becomes double — needs
hyperlink state the row pass does not carry yet, and is deferred.)

## Rust mapping (`roastty/src/renderer/cell.rs`)

`rebuild_row` gains two decoration passes around its existing run loop (the
`fg_colors`/`infos` derivation is unchanged):

```rust
// Decorations that layer UNDERNEATH the text: underline (its own color, else the
// foreground) and overline (the foreground).
for (col, cell) in row_cells.iter().enumerate() {
    let grid_pos = [u16::try_from(col).expect("column fits u16"), y];
    let rgba = fg_colors[col];
    let fg = [rgba[0], rgba[1], rgba[2]];
    let flags = cell.style.flags;
    if flags.underline != Underline::None {
        let underline_color = cell
            .style
            .resolve_underline_color(palette)
            .map(|rgb| [rgb.r, rgb.g, rgb.b])
            .unwrap_or(fg);
        add_underline(contents, grid, grid_pos, flags.underline, underline_color, alpha)?;
    }
    if flags.overline {
        add_overline(contents, grid, grid_pos, fg, alpha)?;
    }
}

for run in row_runs {
    add_run(/* …unchanged… */)?;
}

// Strikethrough layers ON TOP of the text.
for (col, cell) in row_cells.iter().enumerate() {
    if cell.style.flags.strikethrough {
        let grid_pos = [u16::try_from(col).expect("column fits u16"), y];
        let rgba = fg_colors[col];
        add_strikethrough(contents, grid, grid_pos, [rgba[0], rgba[1], rgba[2]], alpha)?;
    }
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: the per-cell decoration emission of upstream
  `rebuildCells` — for each cell, the underline/overline (underneath) and
  strikethrough (on top) its flags request, with the resolved colors.
- **Faithful**: within each column the draw order matches upstream
  (underline/overline < glyph < strikethrough); the underline color is
  `resolve_underline_color(palette).unwrap_or(fg)` (upstream's `orelse fg`);
  overline and strikethrough use the foreground; decorations are emitted per
  cell including wide-cell spacer columns (so a wide char's decoration spans
  both cells, as upstream draws per-column).
- **Faithful adaptation (with a documented caveat)**: roastty brackets the
  per-run glyph assembly with two per-cell decoration passes rather than
  upstream's single interleaved column-walk. This is **not strictly
  equivalent**: the global order is _all_ underlines/overlines, then _all_
  glyphs, then _all_ strikethroughs, whereas upstream interleaves per column.
  The two differ only for **cross-column overlap** — e.g. a wide or overhanging
  glyph in column `x` that extends into column `x + 1` where a neighbor has a
  decoration: upstream draws `x`'s glyph before `x + 1`'s underline, but the
  three-pass draws all underlines before all glyphs, so `x + 1`'s underline can
  land under `x`'s overhang. This is a rare visual edge case; a strict
  **column-ordered merge** (interleaving the decorations and the run glyphs per
  column) would reproduce upstream exactly and is a possible later refinement.
  The per-cell foreground color comes from `fg_colors[col]` (already resolved).
  The hyperlink double-underline override is deferred (needs hyperlink state).
- **Deferred**: the link-underline override; the cursor cell; the renderer-layer
  color adjustments (reverse-video, selection, min-contrast, faint/dim alpha);
  and the Metal upload. (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`: add the two per-cell decoration passes
   (underline/overline before the run loop, strikethrough after) to
   `rebuild_row`.
2. Test (in `cell.rs`): a 1×1 viewport row with one cell `'A'` whose style has
   `underline = Single` (with a distinct `underline_color = Rgb`), `overline`,
   and `strikethrough`, and a matching `ShapedRun`; after `rebuild_row`, assert
   `fg_rows[1]` holds, in order, the **underline** and **overline**
   (underneath), then the **glyph**, then the **strikethrough** (on top) —
   identifying each by a same-grid cache-identity render of its sprite/glyph and
   by color (the underline carries its own color, the others the foreground),
   proving the per-cell emission, the colors, and the same-column layering. (The
   cross-column overhang caveat is not exercised by this single-cell test — it
   is a documented adaptation, not asserted behavior.)
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

- `rebuild_row` emits each cell's underline/overline (underneath) and
  strikethrough (on top) per its flags, with the resolved colors — faithful to
  upstream's per-cell decoration draw order;
- the test passes (the decorations appear in the right order with the right
  colors alongside the glyph), and the existing tests still pass;
- the link-underline override, cursor, color adjustments, and Metal upload stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a decoration is emitted for the wrong cells, the
layering is wrong (strikethrough underneath / underline on top), the underline
color ignores its style, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Required** finding, now addressed:

- **Required (addressed):** the design overstated the three-pass approach as
  layering-_equivalent_ to upstream. Upstream interleaves per column
  (underline/overline, glyph, strikethrough for `x`, then `x + 1`), while the
  three-pass emits all underlines/overlines, then all glyphs, then all
  strikethroughs — which preserves same-column layering but differs for
  cross-column overlap (a wide/overhanging glyph in column `x` vs a decoration
  in column `x + 1`). The notes now document this as a **deliberate adaptation
  with a cross-column caveat** (not full equivalence) and name a strict
  column-ordered merge as the exact alternative, deferred.

Codex confirmed the rest is sound: iterating `row_cells` is the right source for
decoration flags; using spacer cells is correct if they carry the wide
character's style (upstream decorates per column); the underline color is
`resolve_underline_color(palette).unwrap_or(fg)` while overline/strikethrough
use the foreground; and the 1×1 test is good for same-cell emission, color
selection, and local layering (it cannot validate the cross-column caveat, which
is the documented adaptation, not asserted behavior).

Review artifacts:

- Prompt: `logs/codex-review/20260603-190328-962459-prompt.md` (design)
- Result: `logs/codex-review/20260603-190328-962459-last-message.md` (design)

## Result

**Result:** Pass

Decorations are now wired into the row pass — the per-row foreground assembly is
complete (glyphs + decorations).

- `roastty/src/renderer/cell.rs`: `rebuild_row` gains two per-cell decoration
  passes around the existing run loop. Before the runs (underneath), for each
  cell it emits an `add_underline` (colored
  `resolve_underline_color(palette) .unwrap_or(fg)`) when
  `flags.underline != None` and an `add_overline` (fg) when `flags.overline`;
  after the runs (on top), it emits an `add_strikethrough` (fg) when
  `flags.strikethrough`. The column is a checked `u16::try_from(col)`, the fg
  comes from the already-resolved `fg_colors[col]`.

Test (in `cell.rs`): `rebuild_row_emits_decorations_layered` builds a 1×1 row
with `'A'` carrying `underline = Single` (a distinct `underline_color`),
`overline`, and `strikethrough`, plus a matching `ShapedRun`; after
`rebuild_row`, `fg_rows[1]` has four cells in order — underline (its own color,
`Sprite::Underline`), overline (fg, `Sprite::Overline`), the glyph `'A'` (fg,
distinct), strikethrough (fg, `Sprite::Strikethrough`) — the sprites verified by
same-grid cache identity. This proves the per-cell emission, the colors, and the
same-column layering.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2830 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer) clean; `git diff --check` clean.

## Conclusion

`rebuild_row` now produces a cell's full foreground — its glyphs and its
underline/overline/strikethrough — and `rebuild_viewport` drives it (with
backgrounds) over the whole screen. From a terminal screen's `RunOptions`, the
renderer now fills `Contents` with backgrounds, glyphs, and all three
decorations, correctly layered within each column.

The remaining renderer-bridge work: a strict column-ordered merge (the deferred
cross-column layering refinement); the upstream link double-underline override;
the **cursor** cell; the renderer-layer **color adjustments** (reverse-video,
selection, min-contrast, faint/dim alpha, default-bg fill, opacity); and the
**Metal upload** of `Contents`.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed `rebuild_row` emits underline/overline before the
run glyphs and strikethrough after, with checked column conversion and clean
sequential borrows of `contents`/`grid`; that the color handling is faithful
(underline uses `resolve_underline_color(palette).unwrap_or(fg)`, overline and
strikethrough use the resolved foreground); that the test proves the same-column
layering and colors (`fg_rows[1]` ordered underline, overline, glyph,
strikethrough, with the underline's distinct color and the decoration sprites
verified by same-grid cache identity); and that the documented cross-column
ordering caveat covers the known non-equivalence with upstream's exact column
interleave. Nothing needed to change before the result commit.

Review artifacts:

- Result review: `logs/codex-review/20260603-190718-625631-last-message.md`
