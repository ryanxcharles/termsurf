+++
[implementer]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 180: Port Render State Row Cells Basic C ABI

## Description

Port the first row-cell iterator slice from upstream:

- `vendor/ghostty/src/terminal/c/render.zig`;
- `vendor/ghostty/src/terminal/render.zig`.

Experiment 179 made render-state row iteration real and deliberately left
`ROASTTY_RENDER_STATE_ROW_DATA_CELLS` as `ROASTTY_NO_VALUE`. This experiment
turns that selector into a real binding operation for an existing
`roastty_render_state_row_cells_t` handle.

Scope for this experiment:

- add row-cell snapshot storage to render-state row snapshots;
- add row-cells lifecycle, `next`, and `select`;
- make row `CELLS` bind an existing row-cells handle to the currently selected
  row's owned snapshot;
- add row-cells data selectors with upstream numeric values and Roastty names;
- implement row-cells `RAW`, `SELECTED`, and `HAS_STYLING`;
- keep richer row-cell selectors as explicit `ROASTTY_NO_VALUE` until the next
  slice ports style snapshots, grapheme snapshots, and color resolution.

This keeps the row-cell C ABI moving without pretending that style, grapheme, or
resolved-color data exists before the render-state snapshot actually owns it.

Public names must use Roastty naming only:

- `roastty_render_state_row_cells_t`;
- `roastty_render_state_row_cells_data_e`;
- `roastty_render_state_row_cells_*`.

Upstream names may appear only in this issue document as source citations.

## Changes

1. Re-read upstream and current Roastty source:
   - `vendor/ghostty/src/terminal/c/render.zig`;
   - `vendor/ghostty/src/terminal/render.zig`;
   - `roastty/src/terminal/page.rs`;
   - `roastty/src/terminal/page_list.rs`;
   - `roastty/src/terminal/screen.rs`;
   - `roastty/src/terminal/terminal.rs`;
   - existing `roastty_cell_t` C ABI from Experiment 176;
   - existing render-state row iterator C ABI from Experiment 179.

2. Extend render-state row snapshots.

   Add row-cell snapshot storage to each `RenderStateRowSnapshot`. Each cell
   snapshot in this slice must contain:
   - raw `roastty_cell_t`;
   - enough reserved/internal structure for a future style/grapheme/color
     snapshot without changing the public `roastty_render_state_row_cells_t`
     handle contract.

   The render-state update path should fill exactly `cols` cell snapshots for
   each viewport row snapshot after a successful terminal update.

   If there is no clean internal terminal accessor for active row cells, add a
   narrow internal snapshot method beside the Experiment 179 row snapshot path.
   Do not expose raw page pointers, row pointers, cell slices, node pointers, or
   live page storage to C.

3. Add public row-cells C ABI types and selectors.

   `roastty_render_state_row_cells_t` already exists as an opaque handle from
   Experiment 179. Add `roastty_render_state_row_cells_data_e` in
   `roastty/include/roastty.h` and mirrored Rust constants in
   `roastty/src/lib.rs`.

   Selector values must match upstream numeric order with Roastty names:

   | Name                                                 | Value | Output type / behavior in this slice |
   | ---------------------------------------------------- | ----- | ------------------------------------ |
   | `ROASTTY_RENDER_STATE_ROW_CELLS_DATA_INVALID`        | 0     | none; always invalid                 |
   | `ROASTTY_RENDER_STATE_ROW_CELLS_DATA_RAW`            | 1     | `roastty_cell_t*`                    |
   | `ROASTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE`          | 2     | deferred; `ROASTTY_NO_VALUE`         |
   | `ROASTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN`  | 3     | deferred; `ROASTTY_NO_VALUE`         |
   | `ROASTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF`  | 4     | deferred; `ROASTTY_NO_VALUE`         |
   | `ROASTTY_RENDER_STATE_ROW_CELLS_DATA_BG_COLOR`       | 5     | deferred; `ROASTTY_NO_VALUE`         |
   | `ROASTTY_RENDER_STATE_ROW_CELLS_DATA_FG_COLOR`       | 6     | deferred; `ROASTTY_NO_VALUE`         |
   | `ROASTTY_RENDER_STATE_ROW_CELLS_DATA_SELECTED`       | 7     | `bool*`                              |
   | `ROASTTY_RENDER_STATE_ROW_CELLS_DATA_HAS_STYLING`    | 8     | `bool*`                              |
   | `ROASTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_UTF8` | 9     | deferred; `ROASTTY_NO_VALUE`         |

   The deferred selectors must not allocate, synthesize defaults, or fake
   values. A future experiment will make them real when the render-state cell
   snapshot owns the necessary style/grapheme/color inputs.

4. Add public row-cells lifecycle and navigation functions:

   ```c
   ROASTTY_API roastty_result_e
   roastty_render_state_row_cells_new(roastty_render_state_row_cells_t*);

   ROASTTY_API void
   roastty_render_state_row_cells_free(roastty_render_state_row_cells_t);

   ROASTTY_API bool
   roastty_render_state_row_cells_next(roastty_render_state_row_cells_t);

   ROASTTY_API roastty_result_e
   roastty_render_state_row_cells_select(roastty_render_state_row_cells_t,
                                         uint16_t);
   ```

   Behavior:
   - `new(NULL)` returns `ROASTTY_INVALID_VALUE`;
   - `new(out)` allocates row cells in an unbound state;
   - `free(NULL)` is a no-op;
   - `next(NULL)` returns `false`;
   - `next(cells)` returns `false` while unbound;
   - after binding, first `next` selects x `0`, subsequent calls advance by one,
     and `next` returns `false` after the last cell;
   - `select(NULL, x)` returns `ROASTTY_INVALID_VALUE`;
   - `select(cells, x)` returns `ROASTTY_INVALID_VALUE` while unbound or when
     `x >= cols`;
   - successful `select` makes that cell the current selected cell.

5. Change row `CELLS`.

   Experiment 179 returns `ROASTTY_NO_VALUE` for
   `ROASTTY_RENDER_STATE_ROW_DATA_CELLS`. This experiment replaces that with:
   - output type is `roastty_render_state_row_cells_t*`;
   - null output returns `ROASTTY_INVALID_VALUE`;
   - the pointed-to row-cells handle must already be non-null;
   - if the pointed-to handle is null, return `ROASTTY_INVALID_VALUE`;
   - the row iterator must be bound and must have selected a row;
   - on success, bind the row-cells handle to the current row snapshot, clone
     that row's cell snapshot vector into the row-cells handle, copy the row's
     optional selection range into the handle, and reset the row-cells x
     position to before-first-cell.

   Binding must preserve the Experiment 179 lifetime contract:
   - terminal mutation after binding does not affect the row-cells iterator;
   - `roastty_render_state_update(state, terminal)` after binding does not
     affect the row-cells iterator;
   - freeing the row iterator after binding does not invalidate the row-cells
     iterator;
   - freeing the render state after binding does not invalidate the row-cells
     iterator;
   - rebinding replaces the row-cells handle's owned snapshot and resets x to
     before-first-cell.

6. Add row-cells getters:

   ```c
   ROASTTY_API roastty_result_e
   roastty_render_state_row_cells_get(roastty_render_state_row_cells_t,
                                      roastty_render_state_row_cells_data_e,
                                      void*);

   ROASTTY_API roastty_result_e
   roastty_render_state_row_cells_get_multi(
       roastty_render_state_row_cells_t,
       size_t,
       const roastty_render_state_row_cells_data_e*,
       void**,
       size_t*);
   ```

   Behavior:
   - raw enum values are accepted as `int`/`c_int` and validated before
     matching;
   - null cells handle returns `ROASTTY_INVALID_VALUE`;
   - invalid selector returns `ROASTTY_INVALID_VALUE`;
   - null output pointer returns `ROASTTY_INVALID_VALUE`;
   - getters before a cell is selected return `ROASTTY_INVALID_VALUE`;
   - `RAW` returns the current cell's raw `roastty_cell_t`;
   - `SELECTED` returns true iff the current x is inside the row's inclusive
     selection range;
   - `HAS_STYLING` returns true iff the raw cell's packed style id field is
     non-zero, matching the existing `ROASTTY_CELL_DATA_HAS_STYLING` ABI helper
     and the internal default style id `0`;
   - deferred selectors return `ROASTTY_NO_VALUE`;
   - `row_cells_get_multi` with `count == 0` succeeds and writes `0` to
     `out_written` if provided;
   - `row_cells_get_multi` null keys or values returns `ROASTTY_INVALID_VALUE`
     when `count > 0`;
   - `row_cells_get_multi` accepts null `out_written`;
   - `row_cells_get_multi` writes the number of completed entries on success or
     first failure.

7. Keep scope narrow.

   Do not implement the deferred row-cells selectors in this slice:
   - `STYLE`;
   - `GRAPHEMES_LEN`;
   - `GRAPHEMES_BUF`;
   - `BG_COLOR`;
   - `FG_COLOR`;
   - `GRAPHEMES_UTF8`.

   Do not add formatter objects, Metal renderer, Swift integration, hyperlink
   expansion, highlight expansion, Kitty graphics, browser overlay behavior, or
   `ghostty_*` compatibility aliases.

   Do not make row-cells handles borrow from render-state rows, row iterators,
   terminal pages, or page-list cell slices. Row-cells handles own their cloned
   cell snapshots after binding.

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/page_list.rs roastty/src/terminal/screen.rs roastty/src/terminal/terminal.rs
cargo test -p roastty render_state_row_cells_c_abi
cargo test -p roastty render_state_row_c_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Pass criteria:

- row-cells handle lifecycle works and frees cleanly;
- `ROASTTY_RENDER_STATE_ROW_DATA_CELLS` binds an existing non-null row-cells
  handle and resets x before the first cell;
- row-cells iteration visits exactly the render state's column count in
  left-to-right order for the selected row;
- `select(x)` selects valid x values and rejects unbound/out-of-range values;
- row-cells data selector numeric values match upstream order with Roastty
  names;
- `RAW`, `SELECTED`, and `HAS_STYLING` behave as specified;
- `HAS_STYLING` is tested on both a default cell that returns false and an
  SGR-styled printed cell that returns true;
- deferred row-cells selectors return `ROASTTY_NO_VALUE`;
- row `CELLS` before row selection returns `ROASTTY_INVALID_VALUE`;
- row `CELLS` with a null pointed-to row-cells handle returns
  `ROASTTY_INVALID_VALUE`;
- row-cells getters and `select` while unbound return `ROASTTY_INVALID_VALUE`;
- every deferred row-cells selector returns `ROASTTY_NO_VALUE` with a valid
  output pointer and does not allocate fake data;
- `row_cells_get_multi(count == 0, NULL, NULL, out_written)` succeeds and writes
  `0`;
- bound row-cells handles remain valid and stable after terminal mutation,
  render-state update, render-state free, and row iterator free;
- rebinding replaces the prior row-cells snapshot and resets x;
- `row_cells_get_multi` reports success and partial progress correctly;
- C header, Rust ABI tests, C harness, full `roastty` tests, no-Ghostty diff
  check, and `git diff --check` all pass;
- Codex reviews and approves both the design and completed result.

Partial criteria:

- row-cells lifecycle and raw cell iteration pass, but selection or
  `HAS_STYLING` cannot be derived cleanly without a focused follow-up.

Failure criteria:

- the API exposes raw page/node pointers, row pointers, cell slices, or live
  terminal storage;
- row-cells handles borrow from render-state rows or row iterators instead of
  owning a cloned snapshot;
- raw C enum values are represented as Rust enums before validation;
- deferred selectors synthesize fake default values instead of returning
  `ROASTTY_NO_VALUE`;
- the row `CELLS` selector still returns `ROASTTY_NO_VALUE`;
- the implementation expands into style snapshots, grapheme buffers, color
  resolution, formatter objects, renderer backend code, Swift integration, or
  browser overlay behavior;
- the API exposes `ghostty_*` symbols or compatibility aliases.

## Review

This design must be reviewed with the Codex review skill before implementation.
All real findings must be fixed, and the design must be re-reviewed until Codex
approves it.

After implementation and result recording, the completed result must also be
reviewed with Codex and approved before the result commit.

## Result

**Result:** Pass

Experiment 180 implemented the basic row-cells C ABI slice:

- added `roastty_render_state_row_cells_data_e` selector values to the public
  header and mirrored Rust constants;
- added row-cells lifecycle, `next`, `select`, `get`, and `get_multi`;
- changed row `CELLS` from a deferred selector into a binding operation for an
  existing non-null row-cells handle;
- extended render-state row snapshots so each row owns cloned cell snapshots;
- implemented row-cells `RAW`, `SELECTED`, and `HAS_STYLING`;
- kept `STYLE`, grapheme, and resolved-color selectors as explicit
  `ROASTTY_NO_VALUE`;
- covered the new ABI in Rust tests and the C harness.

Verification run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/page_list.rs roastty/src/terminal/screen.rs roastty/src/terminal/terminal.rs
cargo test -p roastty render_state_row_cells_c_abi
cargo test -p roastty render_state_row_c_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Observed results:

- focused row-cells ABI tests: 3 passed;
- row iterator ABI tests: 4 passed;
- C harness link test: passed;
- full `roastty` suite: 1866 Rust tests passed, 1 C harness test passed, 0
  doctests;
- strict no-`ghostty` check on public ABI/code files: passed;
- `git diff --check`: passed.
- Codex completed-result review: approved after fixing deferred-selector
  validation precedence for null, unbound, and unselected row-cells handles.

## Conclusion

The render-state row path now exposes a usable row-cells iterator without
leaking live page storage into the C ABI. Binding clones the selected row's
snapshot into the row-cells handle, so the handle remains stable after terminal
mutation, render-state update, row iterator free, and render-state free.

The richer row-cell data remains intentionally deferred. The next render-state
slice should make style, grapheme, and resolved-color cell selectors real by
expanding the snapshot data they need, not by synthesizing placeholders.
