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

# Experiment 46: Port PageList Split

## Description

Port upstream PageList `split`.

Experiment 45 completed PageList compaction. The next source-order PageList
operation is `split`, which divides one page into two pages at a pin: rows
before the split point remain in the original page, and rows at/after the split
point move into a newly inserted page with the same capacity as the original
page.

This experiment should add PageList splitting only. It must not implement
erase/eraseRow, eraseHistory, eraseActive, resize/reflow, scrollClear,
row/cell/prompt iterators, parser retry loops, renderer dirty-region delivery,
or public ABI.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `PageList.split`;
     - `SplitError`;
     - split tests from `PageList split at middle row` through
       `PageList split preserves hyperlinks`.
   - Do not modify `vendor/ghostty/`.

2. Add Rust split error type.
   - Add a narrow `SplitError` enum near the existing PageList error types.
   - Include:
     - `OutOfMemory` for allocation failure;
     - `OutOfSpace` for single-row pages and unexpected clone failure.
   - Implement only the conversions needed by this experiment.

3. Add `PageList::split`.
   - Input should be a `Pin`.
   - Validate the pin with existing PageList pin validation behavior. If the pin
     is invalid, return `SplitError::OutOfSpace` or assert consistently with
     current PageList internal-method style; do not add public API.
   - Copy `pin.node` to a local `original_node` before mutation, matching the
     upstream aliasing guard.
   - If the target page has one row or fewer, return
     `Err(SplitError::OutOfSpace)`.
   - If `pin.y == 0`, return `Ok(())` without mutating the list.
   - Allocate a new page with the same capacity as the original page.
   - Set the new page row count to `old_rows - pin.y` and the same column count
     as the original page.
   - Clone rows `pin.y..old_rows` from the original page into the new page.
   - If cloning unexpectedly fails, restore `page_size`, `page_serial`, page
     order, total rows, pins, `viewport_pin`, and original page contents to
     their pre-allocation state, then return `Err(SplitError::OutOfSpace)`.
   - Move tracked pins whose node is the original page and whose `y >= pin.y` to
     the new page with `y -= pin.y`.
   - Move `viewport_pin` the same way when it points into the split region.
   - Clear the moved rows from the original page with Page's existing cell
     cleanup path before shrinking the original page's row count. This must
     release styles, graphemes, hyperlinks, and string-backed data from the
     original page.
   - Shrink the original page row count by the number of moved rows.
   - Insert the new page immediately after the original page.
   - Preserve `page_size` accounting, serials, total rows, and page order.
   - Verify PageList integrity after real mutation.

4. Add tests.
   - Port the upstream split test set in Rust form:
     - split at middle row;
     - split at row 0 no-op;
     - split at last row;
     - single-row page returns `OutOfSpace`;
     - tracked pin after split moves;
     - tracked pin before split remains;
     - tracked pin at split point moves;
     - multiple tracked pins across both regions;
     - `viewport_pin` in split region moves;
     - splitting a middle page preserves order;
     - splitting the last page makes the inserted page last;
     - splitting the first page keeps the original first;
     - wrap flags survive on moved rows;
     - styled cells move to the new page and are released from the original;
     - grapheme clusters move to the new page and are released from the
       original;
     - hyperlinks move to the new page and are released from the original.
   - Add assertions for:
     - `page_size` before/after split;
     - total rows before/after split;
     - original page backing length unchanged;
     - new page capacity equals original page capacity;
     - row-level dirty bits on moved rows survive in the new page;
     - row-level dirty bits before the split remain in the original page;
     - page-level dirty behavior matches upstream: the original page-level dirty
       flag remains on the original page, and the new page keeps the default
       page-level dirty state unless existing Page clone behavior deliberately
       changes it;
     - no-op split leaves node identity, `page_size`, `page_serial`, and
       integrity unchanged.

5. Preserve scope.
   - Do not implement:
     - erase/eraseRow;
     - eraseHistory or eraseActive;
     - resize/reflow;
     - scrollClear;
     - row/cell/prompt iterators;
     - parser retry loops;
     - renderer or app integration;
     - public C ABI additions.
   - Do not add `ghostty` names except when citing upstream paths or test
     provenance.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - split behavior implemented;
     - no-op and single-row behavior;
     - pin and viewport remapping behavior;
     - page-order behavior;
     - managed-memory release/preservation behavior;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- splitting at row 0 is a no-op;
- splitting a single-row page returns `OutOfSpace`;
- splitting at middle and last rows preserves row data and produces the expected
  row counts;
- the new page is inserted immediately after the original page;
- first/middle/last page split ordering remains correct;
- tracked pins and `viewport_pin` move to the new page only when their row is at
  or after the split point;
- moved rows preserve wrap flags, styled cells, graphemes, hyperlinks, row-level
  dirty state, and visible content;
- page-level dirty behavior is explicitly asserted to match upstream rather than
  accidentally copied to the new page;
- moved managed memory is released from the original page and present in the new
  page;
- `page_size`, serial accounting, total rows, and integrity remain valid;
- allocation or unexpected clone failure before insertion does not leave a
  dangling page, corrupt accounting, mutate page order, mutate total rows, or
  move pins;
- no erase, resize/reflow, scrollClear, iterator, parser, renderer, app, or ABI
  work is introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- basic row splitting works, but managed-memory release from the original page
  exposes a Page cleanup bug that needs a separate Page-level follow-up.

The experiment fails if:

- split loses row data, dirty state, wrap flags, or managed memory;
- split leaves tracked pins or viewport pins pointing to the wrong node or row;
- split corrupts page order, `page_size`, serials, or total row accounting;
- row-0 no-op mutates the PageList;
- single-row split succeeds instead of returning `OutOfSpace`;
- the implementation expands into unrelated PageList operations;
- tests or formatting fail.

## Result

**Result:** Pass

Implemented `PageList::split` in `roastty/src/terminal/page_list.rs` and widened
`Page::clear_cells` to `pub(super)` in `roastty/src/terminal/page.rs` so
PageList can reuse Page's existing managed-memory cleanup path.

Split now:

- returns `OutOfSpace` for invalid pins and single-row pages;
- treats row-0 split as a no-op;
- allocates a same-capacity replacement page for rows at and after the split
  point;
- clones moved rows into the new page;
- restores `page_size` and `page_serial` if cloning unexpectedly fails before
  insertion;
- remaps tracked pins and `viewport_pin` whose rows move to the new page;
- clears moved rows from the original page before shrinking it, releasing
  styles, graphemes, hyperlinks, and string-backed data from the original page;
- inserts the new page immediately after the original page; and
- preserves total rows, page order, serial accounting, and PageList integrity.

The tests ported the upstream split coverage in Rust form, including middle,
row-0, last-row, and single-row behavior; tracked-pin and viewport-pin
remapping; first/middle/last page-order cases; wrap flags; row-level and
page-level dirty semantics; style, grapheme, and hyperlink preservation/release;
and accounting checks.

Verification:

```bash
cargo fmt && cargo test -p roastty terminal::page_list
```

Result: 143 PageList tests passed.

```bash
cargo test -p roastty
```

Result: 424 unit tests passed, plus the ABI harness passed.

Independent result review: Codex reviewer approved recording Experiment 46 as
Pass with no findings. The reviewer specifically confirmed the split semantics,
clone-failure rollback, dirty-state coverage, managed-memory coverage, page
order coverage, and the narrow `Page::clear_cells` visibility change.

## Conclusion

PageList splitting is now ported for the currently implemented PageList surface.
This fills another core PageList mutation path needed by later erase,
resize/reflow, and parser retry behavior while staying scoped to internal
terminal data structures.

The next experiment should continue with the next upstream PageList operation in
source order.
