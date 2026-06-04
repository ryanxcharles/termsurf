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

# Experiment 382: wiring cell_colors into the row passes

## Description

`cell_colors` (Experiments 380–381) is the complete base per-cell color
computation (reverse-video + full-block twist), but the row passes still color
cells directly from `resolve_fg`/`resolve_bg`, so `inverse` is ignored during a
rebuild. This experiment wires `cell_colors` into both passes: `rebuild_row`'s
`fg_colors` use `cell_colors(...).fg`, and `rebuild_bg_row` uses
`cell_colors(...).bg`. `default_bg` (and the `default_fg`/`bold` config for the
background pass) are threaded through `rebuild_viewport`.

## Upstream behavior

`rebuildCells` computes each cell's final `fg`/`bg` once (via the base color
computation now in `cell_colors`) and uses that `fg` for the glyph and the
decorations and that `bg` for the background cell. roastty splits the foreground
(`rebuild_row`) and background (`rebuild_bg_row`) into separate per-row passes;
this experiment has each call `cell_colors` for its half so both honor `inverse`
and the full-block twist, matching upstream.

## Rust mapping (`roastty/src/renderer/cell.rs`)

`rebuild_bg_row` gains the color config and uses `cell_colors(...).bg`:

```rust
pub(crate) fn rebuild_bg_row(
    contents: &mut Contents,
    y: u16,
    row_cells: &[RunCell],
    default_fg: Rgb,
    default_bg: Rgb,
    palette: &Palette,
    bold: Option<BoldColor>,
    alpha: u8,
) {
    let row = usize::from(y);
    for (col, cell) in row_cells.iter().enumerate() {
        let bg = cell_colors(cell.style, cell.codepoint, default_fg, default_bg, palette, bold).bg;
        let cell_bg = bg
            .map(|rgb| CellBg([rgb.r, rgb.g, rgb.b, alpha]))
            .unwrap_or(CellBg([0, 0, 0, 0]));
        *contents.bg_cell_mut(row, col) = cell_bg;
    }
}
```

`rebuild_row` gains a `default_bg` parameter and builds `fg_colors` from
`cell_colors(...).fg` (the rest of `rebuild_row` is unchanged — the decorations
and `add_run` already read `fg_colors`):

```rust
let fg_colors: Vec<[u8; 4]> = row_cells
    .iter()
    .map(|cell| {
        let fg = cell_colors(cell.style, cell.codepoint, default_fg, default_bg, palette, bold).fg;
        [fg.r, fg.g, fg.b, alpha]
    })
    .collect();
```

`rebuild_viewport` gains a `default_bg` parameter and passes the color config to
both `rebuild_bg_row` (now `default_fg`, `default_bg`, `bold`) and `rebuild_row`
(now `default_bg`).

## Scope / faithfulness notes

- **Ported (bridged)**: the integration of the base per-cell color computation
  into the rebuild — both the foreground (`rebuild_row`) and the background
  (`rebuild_bg_row`) now derive their colors from `cell_colors`, so `inverse`
  and the full-block twist are honored during a rebuild.
- **Faithful**: each cell's foreground (for the glyph and the decorations) is
  `cell_colors(...).fg` and its background is `cell_colors(...).bg` — upstream's
  single per-cell `fg`/`bg`; the decoration colors (which read `fg_colors`) and
  the underline-color fallback now use the inverse-aware foreground, as
  upstream; the non-inverse, non-covering common case is unchanged
  (`cell_colors` reduces to `resolve_fg`/`resolve_bg` there), so existing
  behavior is preserved.
- **Faithful adaptation**: roastty computes `cell_colors` once per cell **per
  pass** (the foreground pass takes `.fg`, the background pass takes `.bg`) — a
  small duplication versus upstream's single computation, because the two passes
  are separate functions; the colors are identical. `default_bg` and `bold` are
  threaded through `rebuild_viewport` to the background pass.
- **Deferred**: the selection/search colors, the minimum-contrast adjustment,
  the faint/dim alpha; the lock-cursor glyph; the column-ordered decoration
  merge and link double-underline; and the Metal upload. (Consumed by tests
  now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`:
   - `rebuild_bg_row`: add `default_fg`/`default_bg`/`bold` params; use
     `cell_colors(...).bg`. Update its doc comment (now `cell_colors`, not
     `resolve_bg`).
   - `rebuild_row`: add a `default_bg` param; build `fg_colors` from
     `cell_colors(...).fg`. Update its doc comment.
   - `rebuild_viewport`: add a `default_bg` param; pass the color config to
     both.
   - Update the existing `rebuild_row`/`rebuild_bg_row`/`rebuild_viewport` test
     call sites for the new signatures.
2. Tests (in `cell.rs`):
   - **inverse** (through `rebuild_viewport`): a 1×1 row with one cell `'A'`,
     `fg = Color::Rgb(a)`, `bg = Color::Rgb(b)`, **`inverse`**, and a matching
     `ShapedRun`; assert the **glyph** color is `b` (the swapped foreground) and
     the **background** cell is `a` (the swapped background) — the inverse swap
     flows through both passes;
   - **full block** (through `rebuild_bg_row`): a cell with
     `codepoint = 0x2588`, `fg = Color::Rgb(a)`, `bg = Color::Rgb(b)`,
     **non-inverse**; assert `bg_cell(0, 0)` is `a` (the full-block twist paints
     the bg with the fg color) — proving `RunCell.codepoint` is threaded into
     `cell_colors` (a constant non-covering codepoint would give `b`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty rebuild
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `rebuild_row`/`rebuild_bg_row` derive their colors from `cell_colors`, so a
  rebuild honors `inverse` and the full-block twist — faithful to upstream's
  per-cell color usage;
- the new test passes (an inverse cell's glyph uses the swapped fg and its
  background uses the swapped bg), and the existing tests still pass (updated
  for the new signatures);
- the selection/search/min-contrast and the Metal upload stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the colors are mis-wired (fg/bg crossed, inverse not
applied, the wrong codepoint), the non-inverse case changes, or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Required** and one **Low** finding, both now addressed:

- **Required (addressed):** the inverse `'A'` test proves
  `cell_colors(...) .fg/.bg` are used, but would still pass if the row passes
  fed a constant non-covering codepoint into `cell_colors`. A **full-block**
  test is added (a `codepoint = 0x2588`, non-inverse cell through
  `rebuild_bg_row` — `bg_cell` becomes the foreground color `a`, not `b`),
  directly proving `RunCell.codepoint` is threaded into `cell_colors`.
- **Low (addressed):** the `rebuild_row`/`rebuild_bg_row` doc comments still
  describe direct `resolve_fg`/`resolve_bg`; they are updated to describe the
  `cell_colors` (inverse-aware) color path.

Codex confirmed the wiring is sound (foreground from `cell_colors(...).fg` feeds
glyphs and decorations, backgrounds from `.bg` feed `CellBg`, and
non-inverse/non-covering cells reduce to the existing `resolve_fg`/`resolve_bg`
behavior), that computing `cell_colors` once per pass is acceptable for the
split design (sharing would be an optimization, not a correctness requirement),
and that threading `default_bg` through `rebuild_viewport` and
`default_fg`/`bold` into `rebuild_bg_row` is correct (inverse/full-block
backgrounds can come from the resolved foreground).

Review artifacts:

- Prompt: `logs/codex-review/20260603-193022-886775-prompt.md` (design)
- Result: `logs/codex-review/20260603-193022-886775-last-message.md` (design)

## Result

**Result:** Pass

`cell_colors` is now wired into the row passes — a rebuild honors reverse-video
and the full-block twist.

- `roastty/src/renderer/cell.rs`:
  - `rebuild_row` (new `default_bg` param): builds `fg_colors` from
    `cell_colors(cell.style, cell.codepoint, default_fg, default_bg, palette, bold).fg`,
    so the glyphs and decorations inherit the inverse-aware foreground.
  - `rebuild_bg_row` (new `default_fg`/`default_bg`/`bold` params): writes each
    background from `cell_colors(...).bg` (active transparent clear for `None`).
  - `rebuild_viewport` (new `default_bg` param): threads the color config to
    both.
  - The `rebuild_row`/`rebuild_bg_row` doc comments now describe the
    `cell_colors` color path; the existing test call sites are updated for the
    new signatures.

Tests (in `cell.rs`):

- `rebuild_viewport_applies_inverse` — a 1×1 inverse cell `'A'` (`fg = a`,
  `bg = b`): the glyph color is `b` and `bg_cell(0, 0)` is `a` (the swap flows
  through both passes).
- `rebuild_bg_row_applies_full_block_twist` — a non-inverse `U+2588` cell
  (`fg = a`, `bg = b`): `bg_cell(0, 0)` is `a` (the twist paints the bg with the
  foreground), proving `RunCell.codepoint` is threaded into `cell_colors`.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2836 passed, 0 failed (+2, no regressions; existing
  rebuild tests preserved with updated signatures).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer) clean; `git diff --check` clean.

## Conclusion

The base per-cell color computation is now live in the rebuild: from a terminal
screen, `rebuild_viewport` fills `Contents` with backgrounds and foreground
(glyphs + decorations) whose colors honor reverse-video and the full-block
twist.

The remaining renderer-bridge work: the **selection/search** colors, the
**minimum-contrast** adjustment, and **faint/dim alpha**; the lock-cursor
glyph + under-cursor text recolor; the column-ordered decoration merge + link
double-underline; and the **Metal upload** of `Contents`.

## Completion Review

Codex reviewed the completed implementation and result and **approved**. It
confirmed the wiring is correct (`rebuild_row` builds `fg_colors` from
`cell_colors(...).fg` so glyphs and decorations inherit the inverse-aware
foreground; `rebuild_bg_row` writes from `.bg` with active transparent clears
and threads `cell.codepoint`/`default_fg`/`default_bg`/`bold` correctly), and
that the two new tests cover the important behavior (the viewport inverse test
proves both passes consume swapped colors; the full-block background test proves
the codepoint is threaded into `cell_colors`), with the existing call-site
updates scoped and the gates covering regressions. Its one **Low** finding —
that the `rebuild_row` doc comment overstated the full-block twist (which
affects only `.bg`) — was fixed: the comment now says the foreground is
inverse-aware, leaving the full-block wording to `rebuild_bg_row`.

Review artifacts:

- Result review: `logs/codex-review/20260603-193521-504294-last-message.md`
