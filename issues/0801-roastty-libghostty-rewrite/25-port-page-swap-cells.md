# Experiment 25: Port Page Swap Cells

## Description

Port upstream `Page.swapCells` to Roastty.

`swapCells` swaps two cells within one Page without allocation. Unlike
`moveCells`, it does not clear either side and it must preserve style refcounts
while swapping map-backed grapheme and hyperlink entries keyed by cell offset.
Upstream uses this as a low-level Page primitive adjacent to `moveCells`.

This experiment should add internal Page cell-swapping support only. It should
not add terminal edit commands, parser/screen integration, reflow, scrollback,
public ABI, or app-facing APIs.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as the source of truth for:
     - `Page.swapCells`;
     - `moveGrapheme`;
     - `moveHyperlink`;
     - the surrounding `moveCells` ownership rules.
   - Upstream does not currently have focused `Page swapCells` tests, so create
     Roastty tests that directly prove the upstream behavior.
   - Do not modify `vendor/ghostty/`.

2. Add an internal swap API.
   - Add an internal method such as:

     ```rust
     fn swap_cells(&mut self, src_y: usize, src_x: usize, dst_y: usize, dst_x: usize)
     ```

   - Keep the operation internal to Page tests and future terminal internals.
   - Assert that both coordinates are inside the current Page size.
   - If the source and destination coordinates are identical, return without
     mutation. This is a Rust-friendly no-op equivalent of swapping a cell with
     itself.
   - The operation must not allocate and must not return an allocation error.

3. Swap grapheme map entries.
   - Graphemes are keyed by cell offset, so a cell metadata swap alone is not
     enough.
   - Match upstream cases:
     - if only source has a grapheme, move the map entry source -> destination;
     - if only destination has a grapheme, move destination -> source;
     - if both have graphemes, swap the two map values in place;
     - if neither has graphemes, do nothing.
   - Do not allocate or clone grapheme slices.
   - Grapheme count and allocator used bytes must remain unchanged.

4. Swap hyperlink map entries.
   - Hyperlinks are keyed by cell offset, so they require the same map-entry
     handling as graphemes:
     - one-sided hyperlink moves the map entry;
     - two-sided hyperlinks swap map values;
     - neither does nothing.
   - Do not change hyperlink set refcounts. The same number of cells still
     reference the same link IDs; only their offsets change.
   - Hyperlink count must remain unchanged.

5. Swap cell payloads after map handling.
   - After the map entries are moved/swapped, swap the two `Cell` values.
   - Styles require no refcount changes because the styled cells are swapped,
     not cloned or cleared.
   - Preserve all other cell metadata bits exactly as part of the cell swap.

6. Update row flags if needed.
   - Upstream preserves row state because the swap is within the same row.
   - Roastty's method accepts row coordinates; if source and destination rows
     differ, recompute `grapheme`, `hyperlink`, and `styled` flags for both
     affected rows after the swap.
   - If both cells are in the same row, row flags should remain correct, but
     recomputing that row is acceptable for clarity and safety.

7. Add focused tests.
   - Plain cell swap:
     - codepoints and metadata bits move to the opposite coordinates.
   - Grapheme cases:
     - source-only grapheme moves to destination;
     - destination-only grapheme moves to source;
     - two grapheme cells swap their grapheme slices;
     - grapheme count and used bytes remain unchanged.
   - Hyperlink cases:
     - source-only hyperlink moves to destination;
     - destination-only hyperlink moves to source;
     - two hyperlink cells swap hyperlink IDs;
     - hyperlink map count and link refcounts remain unchanged.
   - Style case:
     - styled cells swap style IDs without changing style refcounts.
   - Cross-row flag case:
     - swapping a managed-memory cell with a plain cell across rows updates both
       row flags correctly.
   - Self-swap case:
     - swapping the same coordinate is a no-op and preserves data/refcounts.

8. Preserve scope.
   - Do not implement:
     - terminal edit commands;
     - parser/screen integration;
     - reflow or scrollback behavior;
     - public ABI or app-facing APIs.
   - Do not change Page layout constants or allocator semantics.
   - Do not change `move_cells` unless a bug is discovered that directly affects
     `swap_cells`; if that happens, document it in the result.

9. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

10. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - swap API added;
      - grapheme and hyperlink map-entry strategy;
      - refcount behavior for styles and hyperlinks;
      - tests added;
      - any deferred upstream Page methods;
      - verification command output summary.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Page::swap_cells` swaps two valid cells without allocating;
- identical-coordinate swaps are no-ops;
- one-sided grapheme swaps move the grapheme map entry to the new cell offset;
- two-sided grapheme swaps exchange map values in place;
- grapheme count and allocator used bytes remain stable;
- one-sided hyperlink swaps move the hyperlink map entry to the new cell offset;
- two-sided hyperlink swaps exchange hyperlink IDs in the map;
- hyperlink map count and hyperlink set refcounts remain stable;
- style IDs move with cell payloads and style refcounts remain stable;
- affected row flags are correct after same-row and cross-row swaps;
- existing clone, partial clone, move, exact capacity, style, grapheme, and
  hyperlink tests do not regress;
- `cargo fmt`, targeted Page tests, and full `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- plain and style swaps work, but a map-backed managed-memory type needs a
  focused follow-up;
- same-row swaps work, but cross-row row-flag handling exposes a helper gap that
  needs a smaller preparatory experiment.

The experiment fails if:

- map-backed graphemes or hyperlinks remain keyed to the old offsets after the
  cell payload swap;
- a swap changes style or hyperlink refcounts;
- grapheme allocator usage changes;
- row flags become inconsistent with row contents;
- the implementation expands into parser/screen integration, reflow, scrollback,
  public ABI, or unrelated behavior.

## Result

**Result:** Pass

Roastty now has internal Page cell-swapping support.

Implementation details:

- added `Page::swap_cells(src_y, src_x, dst_y, dst_x)`;
- identical-coordinate swaps return as a no-op;
- grapheme map entries are moved for one-sided grapheme swaps and exchanged for
  two-sided grapheme swaps before cell payloads are swapped;
- hyperlink map entries follow the same one-sided move / two-sided exchange
  pattern before cell payloads are swapped;
- style IDs move as part of the `Cell` payload, so style refcounts remain
  stable;
- hyperlink set refcounts remain stable because hyperlink cells are swapped, not
  cloned or cleared;
- affected row `grapheme`, `hyperlink`, and `styled` flags are recomputed after
  the swap.

Tests added:

- plain cell swap, including non-text metadata bits;
- source-only grapheme swap;
- destination-only grapheme swap;
- two-sided grapheme swap;
- source-only hyperlink swap;
- destination-only hyperlink swap;
- two-sided hyperlink swap;
- style swap preserving style refcounts;
- cross-row managed-memory swap updating row flags;
- identical-coordinate self-swap no-op preserving data and refcounts.

The experiment did not implement terminal edit commands, parser/screen
integration, reflow, scrollback, public ABI, or app-facing APIs.

Verification run:

```bash
cargo fmt
cargo test -p roastty terminal::page
cargo test -p roastty
```

Results:

- `cargo test -p roastty terminal::page`: 122 passed.
- `cargo test -p roastty`: 231 unit tests passed; ABI harness passed; doc tests
  passed.

## Conclusion

Experiment 25 successfully ports upstream `Page.swapCells` semantics into
Roastty's Page storage model. The operation remains internal, no-allocation, and
keeps map-backed grapheme/hyperlink data aligned with the swapped cell offsets.

The next experiment should continue through the remaining upstream Page
primitives after re-reading current upstream call sites rather than assuming a
larger terminal edit operation is ready.
