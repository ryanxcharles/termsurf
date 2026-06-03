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

# Experiment 359: assembling RunOptions for the viewport

## Description

Experiment 358 decodes one page row into `RunCell`s. This experiment assembles
the full **`RunOptions`** the shaper needs per viewport row — the row's
`RunCell`s plus its **selection** column range and the **cursor** column — for
every visible row. It mirrors the existing `PageList::render_rows_snapshot`
(which already iterates the viewport rows, navigates each row's page, and
computes the per-row selection range), producing `font::run::RunOptions` instead
of a render snapshot. This is the last terminal-side piece before the draw path.

## Upstream behavior

Upstream's renderer builds, per visible row, a
`shape.RunOptions { grid, cells, selection, cursor_x }` from the screen's
`RenderState` and feeds it to a `RunIterator`. The selection is the
`[start, end]` column range of the selection within the row; `cursor_x` is the
cursor's column when the cursor is on that row. roastty already computes the
per-row selection range in `render_rows_snapshot`; this experiment reuses that
computation and pairs it with the `RunCell` decode (Experiment 358) and the
cursor column.

## Rust mapping (`roastty/src/terminal/page_list.rs`)

```rust
use crate::font::run::RunOptions;

impl PageList {
    /// Assemble a [`RunOptions`] per visible row: the row's decoded `RunCell`s
    /// (Experiment 358), its selection column range (`[start_x, end_x]`, the same
    /// computation as `render_rows_snapshot`), and the cursor column when the
    /// cursor is on that row. The shaper's `RunIterator` consumes each
    /// `RunOptions`. (`grid` is omitted — roastty passes the resolver separately.)
    pub(super) fn shape_run_options(
        &self,
        selection: Option<selection::Selection>,
        cursor: Option<(CellCountInt, CellCountInt)>, // active (x, y), or none
    ) -> Vec<RunOptions> {
        let mut out = Vec::with_capacity(self.rows as usize);
        let last_col = self.cols.saturating_sub(1);
        for y in 0..self.rows {
            let Some(pin) = self.pin(point::Point::active(Coordinate::new(0, y.into()))) else {
                continue;
            };
            let Some(node) = self.node_for_pin(&pin) else {
                continue;
            };
            let cells = node.page.shape_run_cells(pin.y as usize);
            let selection = selection.and_then(|selection| {
                let selection = self.selection_contained_row(selection, pin)?;
                let start_x = selection.start().x.min(selection.end().x).min(last_col);
                let end_x = selection.start().x.max(selection.end().x).min(last_col);
                Some([start_x, end_x])
            });
            let cursor_x = cursor.and_then(|(cx, cy)| (cy == y).then_some(cx));
            out.push(RunOptions {
                cells,
                selection,
                cursor_x,
            });
        }
        out
    }
}
```

This is the `render_rows_snapshot` loop with the cell decode swapped to
`shape_run_cells` and the snapshot fields swapped to `RunOptions` (selection
range, cursor column).

## Scope / faithfulness notes

- **Ported (bridged)**: the per-row `RunOptions` assembly — the decoded
  `RunCell`s, the selection column range, and the cursor column — for every
  viewport row, the input a `RunIterator` consumes.
- **Faithful**: the viewport-row iteration, the pin→page navigation, and the
  selection-range computation (`selection_contained_row` → `[min, max]` clamped
  to the last column) match `render_rows_snapshot` (and upstream's per-row
  selection); `cursor_x` is set only on the cursor's row; `RunOptions.selection`
  is `None` when the row has no selection.
- **Faithful adaptation**: `RunOptions` omits the `grid` pointer (roastty passes
  the `CodepointResolver` to the iterator separately); rows whose pin/page
  lookup fails are skipped (as in `render_rows_snapshot`); "viewport" here means
  the **active visible rows** (`Point::active`, as `render_rows_snapshot` uses)
  — scrollback-pinned viewport modes are out of scope (as there).
- **Raw selection range (matches upstream's downstream guard)**: the assembly
  emits the selection's raw `[start_x, end_x]` columns, including `[0, N]`. The
  `RunIterator`'s selection break guards `bounds[0] > 0` (so it does **not**
  break before column 0) and breaks at `bounds[1] + 1` — this is upstream's
  behavior, so a selection whose start is column 0 is not isolated before it.
  The assembly is faithful (it passes the true range); the guard is the
  iterator's concern. A test documents this (`shape_run_options` emits
  `[0, end]` for a column-0 selection).
- **Deferred**: the `Terminal`/`Screen`-facing `pub(crate)` entry the renderer
  calls (since `PageList`/`Screen` are `pub(super)`) and the draw-path wiring
  (running the `RunIterator` over these `RunOptions` and routing the shaped
  glyphs to the renderer). (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/terminal/page_list.rs`: add `PageList::shape_run_options`;
   import `font::run::RunOptions`.
2. Tests (in `page_list.rs`): build a `PageList`, write a few active cells
   (`write_basic_active_cell`), then assert `shape_run_options`:
   - one `RunOptions` per visible row;
   - a row's `cells` decode (`'A'`/`'B'` at the written columns, empty
     elsewhere);
   - `cursor_x` is `Some(col)` only on the cursor's row, `None` elsewhere;
   - `selection` is `None` with no selection (and, if a `Selection` is easily
     constructed, `Some([start, end])` for a selected row — otherwise the
     selection range reuses the `render_rows_snapshot` path verbatim and is
     noted).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty shape_run_options
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `PageList::shape_run_options` assembles a `RunOptions` per viewport row with
  the decoded cells, the selection range, and the cursor column — mirroring
  `render_rows_snapshot` and faithful to upstream's per-row `RunOptions`;
- the assembly tests pass, and the existing tests still pass;
- the `Terminal`/`Screen`-facing entry and the draw-path wiring stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the assembly diverges from `render_rows_snapshot`'s
row/selection handling, the cursor column is wrong, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it (one
Required documentation finding addressed, no code change required):

- **Required (addressed):** the `RunIterator`'s selection break guards
  `bounds[0] > 0` (it does not break before column 0) and breaks at
  `bounds[1] + 1` — upstream's behavior — so a selection whose start is column 0
  is not isolated before it. The assembly should still emit the raw
  `[start, end]` range (matching `render_rows_snapshot`); a scope note + a
  `[0, end]`-emitting test now document this downstream behavior.

Codex confirmed: the row loop, `pin`/`node_for_pin` navigation, and the
`selection_contained_row` + `min`/`max` + `last_col` clamp are faithful to
`render_rows_snapshot`; `cursor_x` is correct for active `(x, y)` coords
(`Some(cx)` only when `cy == y`); skipping rows on failed pin/node lookup
matches `render_rows_snapshot` (the output can be shorter than `self.rows`);
`CellCountInt` is compatible with `RunOptions.selection: Option<[u16; 2]>`; and
the `pub(super)` placement + `terminal → font::run` coupling are acceptable
(same direction as Exp 358). Its naming note — that `Point::active` covers the
active visible rows, not scrollback-pinned viewport modes — is now stated in the
scope.

Review artifacts:

- Prompt: `logs/codex-review/20260603-170342-865560-prompt.md` (design)
- Result: `logs/codex-review/20260603-170342-865560-last-message.md` (design)

## Result

**Result:** Pass

The viewport-side `RunOptions` assembly is in place — the last terminal-side
piece before the draw path.

- `roastty/src/terminal/page_list.rs`:
  `PageList::shape_run_options(selection, cursor)` builds one
  `font::run::RunOptions` per active visible row: the row's decoded `RunCell`s
  (`Page::shape_run_cells`, Experiment 358), the per-row selection column range
  (`selection_contained_row` → `[min, max]` clamped to `last_col`, the same
  computation as `render_rows_snapshot`), and `cursor_x` set only on the
  cursor's row. Rows whose pin/page lookup fails are skipped (as in
  `render_rows_snapshot`). Imported `crate::font::run::RunOptions`.

Tests (in `page_list.rs`):

- `shape_run_options_assembles_rows` — a 4×2 page with `'A'`/`'B'` on row 0,
  cursor at `(1, 0)`, no selection: asserts one `RunOptions` per visible row,
  row 0's cells decode (`'A'`/`'B'` then empty), `cursor_x == Some(1)` on row 0
  and `None` on row 1, and `selection == None` on both.
- `shape_run_options_emits_column_zero_selection` — a single row `'A'`/`'B'`/
  `'C'` with a selection spanning columns 0..=2: asserts the assembly emits the
  raw `Some([0, 2])` range (it does not pre-clamp the column-0 start; the
  `RunIterator`'s `bounds[0] > 0` guard is the iterator's concern).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2806 passed, 0 failed (+2, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The terminal viewport is now fully assemblable into shaper input: a caller can
turn the active screen — with its selection and cursor — into a
`Vec<RunOptions>`, one per row, each ready for a `RunIterator` to group into
`TextRun`s and `Face::shape_run` to shape into positioned glyphs. The
terminal→font bridge is complete from packed page cell to shaper-ready run
options.

The remaining renderer↔font work:

1. **`Terminal`/`Screen`-facing entry** — a `pub(crate)` method the renderer can
   call (since `PageList`/`Screen` are `pub(super)`), threading the active
   screen's selection and cursor into `shape_run_options`.
2. **Draw-path wiring** — run the `RunIterator` over these `RunOptions` and
   route the shaped glyphs into the Metal renderer's cell/draw path.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed `shape_run_options` is faithful to
`render_rows_snapshot` (same active-row loop, same `pin`/`node_for_pin`
handling, same `selection_contained_row` path, same `[min, max]` range clamped
to `last_col`), and that swapping the snapshot fields for
`RunOptions { cells, selection, cursor_x }` matches the approved design and
upstream mapping. It confirmed the selection behavior is correct (raw `[0, end]`
is the right assembly output; the iterator's `bounds[0] > 0` guard is
downstream), that `cursor_x` is set only when `cy == y` in active-row
coordinates, and that the two tests cover the core bridge behavior (decoded row
cells, one option per active visible row, no-selection rows, cursor-row
filtering, raw column-zero selection). Reverse/rectangle/clamp cases remain
covered by the existing `selection_contained_row` tests, which it deemed
acceptable since the helper reuses that exact computation. No correctness or
regression issue blocks the result commit.

Review artifacts:

- Result review: `logs/codex-review/20260603-170946-771961-last-message.md`
