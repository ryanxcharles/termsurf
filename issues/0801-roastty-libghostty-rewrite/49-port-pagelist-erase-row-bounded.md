# Experiment 49: Port PageList Erase Row Bounded

## Description

Port upstream PageList `eraseRowBounded`.

Experiment 48 added the unbounded one-row erase primitive: erase one row, shift
every following row in the PageList up by one, and clear the final row. The next
source-order mutation is the bounded variant. It erases one target row but only
shifts a bounded number of following rows, filling the vacated row at the
boundary with blank cells.

This experiment should add `erase_row_bounded` only. It must not implement
`eraseHistory`, `eraseActive`, `eraseRows`, page deletion, active regrowth,
resize/reflow, scrollClear, row/cell/prompt iterators, parser retry loops,
renderer dirty-region delivery, or public ABI.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `PageList.eraseRowBounded`;
     - the viewport offset cache tests for `eraseRowBounded`;
     - the functional tests:
       - `PageList eraseRowBounded less than full row`;
       - `PageList eraseRowBounded with pin at top`;
       - `PageList eraseRowBounded full rows single page`;
       - `PageList eraseRowBounded full rows two pages`.
   - Do not modify `vendor/ghostty/`.

2. Add only narrow Page helpers if needed.
   - Prefer the helpers introduced in Experiment 48:
     - `Page::rotate_rows_left`;
     - `Page::clone_row_from`;
     - `Page::clear_cells`.
   - If a new helper is required, keep it `pub(super)` and specific to row
     metadata movement or managed-memory-aware row cleanup.
   - Do not expose raw page memory more broadly than `erase_row_bounded`
     requires.

3. Add `PageList::erase_row_bounded`.
   - Input should be a `point::Point` and a `limit: usize`.
   - Match upstream semantics: `limit` is exclusive of the erased row. A limit
     of 1 erases the target row, shifts the next row into the target row, and
     leaves a blank row one row below.
   - Resolve the target through `PageList::pin`; return a narrow error if the
     point does not resolve.
   - If the bounded region ends inside the target page:
     - clear the target row through Page's managed-memory cleanup path;
     - rotate rows left from the target row through `limit + 1` rows;
     - mark the page dirty;
     - update pinned viewport cache only when upstream does;
     - update tracked pins in the shifted region exactly like upstream.
   - If the bounded region crosses page boundaries:
     - rotate the target page from the erased row through the end of the page;
     - mark the target page dirty;
     - update target-page tracked pins and viewport cache using upstream
       conditions;
     - for each following full-shift page, clone its top row into the previous
       page's final row, rotate the following page, mark it dirty, update
       viewport cache, and remap tracked pins;
     - when the boundary falls inside a following page, clone that page's top
       row into the previous page's final row, clear the following page's top
       row, rotate only the bounded prefix, mark the page dirty, update viewport
       cache, update tracked pins, and return;
     - if the PageList ends before the limit is satisfied, clear the final row
       of the final touched page through `Page::clear_cells`.
   - Preserve page row counts, page order, total row count, page_size, and page
     serials.
   - Verify PageList integrity after successful mutation.

4. Define error behavior.
   - Reuse `EraseRowError` if it still fits, or add a narrow
     `EraseRowBoundedError`.
   - Cross-page row cloning can fail if the destination page lacks
     managed-memory capacity. Return the clone error rather than pretending the
     erase succeeded.
   - Match upstream by not adding broad rollback after earlier rotations or pin
     updates. The result must document that choice.
   - The error path must not leave dangling style, grapheme, hyperlink, string,
     or pin references. If this failure path is not practically reachable with
     current capacity helpers, document why and rely on existing Page clone
     failure tests for the lower-level managed-memory rollback guarantee.

5. Add tests.
   - Bounded single-page prefix:
     - fill all rows with distinct visible data;
     - erase a row with a small limit that ends inside the same page;
     - verify only the bounded rows shift, the boundary row is blank, rows
       beyond the boundary are unchanged, row counts/accounting are unchanged,
       and the page is dirty.
   - Pin at top:
     - reproduce upstream's `eraseRowBounded with pin at top`;
     - verify tracked pin behavior when the erased row is page row 0.
   - Single-page full-row span:
     - erase with a limit that reaches the end of the page but not another page;
     - verify shifted rows, blank final row, and tracked pins inside/outside the
       shifted range.
   - Exact page-boundary span:
     - create at least two pages;
     - erase a row where `limit` exactly reaches the end of the target page
       while a following page exists;
     - verify this follows upstream's cross-page path, not the same-page path:
       the next page's row 0 is cloned into the previous page's final row, the
       following page clears/rotates a one-row prefix, row-0 pins remap to the
       previous page's final row, managed memory from the following page's top
       row is cleaned up there after the move, dirty flags are preserved, and
       accounting/order are unchanged.
   - Two-page span:
     - create an active area that straddles two pages;
     - erase a bounded region crossing the page boundary;
     - verify the previous page receives the next page's top row, only the
       bounded prefix of the following page rotates, and rows beyond the limit
       remain unchanged.
   - Viewport cache:
     - port the upstream `eraseRowBounded invalidates viewport offset cache`
       cases for single-page, multi-page, full-page shift, and exhausted-page
       limit where practical with current helpers;
     - verify `scrollbar()` and `viewport_pin_row_offset` match upstream
       expectations.
   - Managed memory:
     - include style, grapheme, hyperlink, and string-backed data in the erased
       row and verify it is released;
     - include managed-memory data in shifted rows and verify it survives;
     - include managed-memory data in the blanked boundary row and final cleared
       row and verify it is released;
     - include managed-memory data in a cross-page boundary row and verify it is
       cloned and cleaned up correctly.
   - Dirty state:
     - verify page-level dirty flags on every touched page;
     - verify row-level dirty flags survive row metadata movement and boundary
       cloning.
   - Accounting:
     - verify page row counts, total rows, page_size, page serials, and page
       order are unchanged.

6. Preserve scope.
   - Do not implement:
     - `eraseHistory`, `eraseActive`, or `eraseRows`;
     - page deletion or active regrowth;
     - resize/reflow;
     - scrollClear;
     - row/cell/prompt iterators;
     - parser retry loops;
     - renderer or app integration;
     - public C ABI additions.
   - Do not add `ghostty` names except when citing upstream paths or test
     provenance.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - bounded erase behavior implemented;
     - single-page bounded behavior;
     - cross-page bounded behavior;
     - tracked-pin behavior;
     - viewport cache behavior;
     - managed-memory behavior;
     - dirty-state behavior;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- bounded erasure physically removes the target row and shifts only the bounded
  number of following rows;
- the row left at the bounded boundary is blank and releases managed memory;
- rows beyond the bounded region are unchanged;
- cross-page bounded shifting copies and rotates only the rows upstream would
  move;
- page row counts, page order, total rows, page_size, and page serials are
  unchanged;
- touched pages are marked dirty;
- row-level dirty flags survive moved/cloned rows;
- tracked pins update exactly like upstream for target-page, boundary, and
  following-page cases;
- pinned viewport cached offset behavior matches upstream;
- style, grapheme, hyperlink, and string-backed data survive movement and are
  released from blanked rows;
- no history/active erase, eraseRows, page deletion, active regrowth,
  resize/reflow, scrollClear, iterator, parser, renderer, app, or ABI work is
  introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- simple same-page bounded erasure works, but cross-page bounded erasure exposes
  a missing lower-level Page primitive that should be designed separately.

The experiment fails if:

- it clears rows instead of physically shifting the bounded region;
- it shifts beyond the requested bound;
- row data, dirty state, or managed memory is lost during shifting;
- tracked pins or viewport state diverge from upstream semantics;
- row counts, total rows, page_size, serials, or page order change;
- the implementation expands into history/active erase, eraseRows, page
  deletion, resize/reflow, parser, renderer, app, or ABI work;
- tests or formatting fail.
