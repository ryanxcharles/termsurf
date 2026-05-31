# Experiment 44: Port PageList Increase Capacity

## Description

Port upstream PageList `increaseCapacity`.

Several later PageList operations need a way to replace one page node with a
larger-capacity clone while preserving row data and tracked pins. Upstream uses
this path when managed-memory capacity is exhausted and later compact/split
tests depend on it. Roastty already has the Page-level pieces:

- `Page::capacity`;
- `Page::clone_rows_from`;
- `Page::set_size_rows`;
- `Page::set_dirty`;
- `Page::backing_len`;
- Page exact-capacity and managed-memory tests from earlier experiments.

This experiment should add the PageList-level node replacement path only. It
must not implement erase, resize/reflow, split, compact, row/cell/prompt
iterators, parser retry loops, or public ABI.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `IncreaseCapacity`;
     - `IncreaseCapacityError`;
     - `PageList.increaseCapacity`;
     - `PageList increaseCapacity to increase styles`;
     - `PageList increaseCapacity to increase graphemes`;
     - `PageList increaseCapacity to increase hyperlinks`;
     - `PageList increaseCapacity to increase string_bytes`;
     - `PageList increaseCapacity tracked pins`;
     - `PageList increaseCapacity returns OutOfSpace at max capacity`;
     - `PageList increaseCapacity multi-page`;
     - `PageList increaseCapacity preserves dirty flag`.
   - Treat `PageList increaseCapacity after col shrink` as deferred unless a
     small local column-shrink helper already exists, because full resize is not
     in scope.
   - Do not modify `vendor/ghostty/`.

2. Add capacity adjustment types.
   - Add a private `IncreaseCapacity` enum with variants for:
     - `Styles`;
     - `GraphemeBytes`;
     - `HyperlinkBytes`;
     - `StringBytes`.
   - Add a private `IncreaseCapacityError` with at least:
     - allocation failure;
     - out-of-space / max-capacity failure;
     - unexpected clone failure only if Rust error typing requires it.
   - Keep both private to PageList for now.

3. Implement checked capacity growth.
   - Start from the source page's current `Capacity`.
   - Double the requested field, matching upstream.
   - Use checked arithmetic and saturate to that field's max value only for the
     final overflow step, matching upstream's "use all bits before OutOfSpace"
     behavior.
   - If the field is already at its max value, return `OutOfSpace`.
   - If the resulting `page_layout(capacity).total_size()` exceeds the maximum
     supported page size used by Page/PageList, return `OutOfSpace`.
   - If Roastty does not yet expose a named `max_page_size`, add the smallest
     private helper or constant needed to match the Page layout limit. Do not
     invent a broader memory policy.

4. Implement `PageList::increase_capacity`.
   - Input should identify the target node and optional adjustment.
   - If adjustment is `None`, recreate/reclone the page at the same capacity.
     This mirrors upstream's future rehash path and should be covered by a small
     test even if no current caller uses it.
   - Create a replacement page with the new capacity using existing PageList
     allocation/accounting helpers.
   - If allocation succeeds but clone/copy fails before the replacement is
     installed, the PageList must remain unchanged, including `page_size`,
     tracked pins, viewport pin, page order, and old page contents. Either roll
     back the temporary replacement node/accounting before returning the error,
     or prove clone failure is unreachable for this path and panic before any
     persistent PageList mutation.
   - Preserve:
     - page size rows/cols;
     - page-level dirty bit;
     - row-level dirty bits through `clone_rows_from`;
     - text/style/grapheme/hyperlink data through `clone_rows_from`;
     - page order;
     - page serial accounting semantics used by `create_page`.
   - Update `page_size` by subtracting the removed page's backing length after
     the new page is inserted/accounted.
   - Update every tracked pin whose node points at the old node to point at the
     replacement node.
   - Preserve viewport state. If `viewport_pin` points at the old node, it must
     point at the replacement node after replacement.
   - Remove/drop the old node only after all no-fail bookkeeping is complete.
   - Verify PageList integrity after success.

5. Add tests.
   - Increase styles:
     - use a small PageList;
     - write visible cell data;
     - increase styles capacity;
     - verify capacity increased and data survived.
   - Increase grapheme bytes:
     - same shape for `grapheme_bytes`.
   - Increase hyperlink bytes:
     - same shape for `hyperlink_bytes`.
   - Increase string bytes:
     - same shape for `string_bytes`.
   - Reclone with no adjustment:
     - call `increase_capacity` with `None`;
     - verify a replacement node is returned, capacity is unchanged, and data
       survived.
   - Tracked pins:
     - track a pin on the replaced page;
     - increase capacity;
     - verify the tracked pin points at the replacement node with unchanged x/y.
   - Viewport pin:
     - replace the active page containing `viewport_pin`;
     - verify `viewport_pin` points at the replacement node and integrity
       passes.
   - OutOfSpace:
     - construct a page with the adjusted field already at that field's max;
     - verify `increase_capacity` returns `OutOfSpace` without changing the
       PageList.
   - Final overflow-to-max:
     - repeatedly increase styles capacity until the final successful increase
       reaches the `style::Id`/style-capacity maximum;
     - verify the next increase returns `OutOfSpace`;
     - verify the failed next increase leaves the PageList unchanged.
   - Multi-page:
     - create at least two pages;
     - increase the first page only;
     - verify the second page remains unchanged and page order is preserved.
   - Dirty preservation:
     - set page-level dirty and row-level dirty bits;
     - increase capacity;
     - verify both survive.

6. Preserve scope.
   - Do not implement:
     - erase;
     - resize/reflow;
     - split;
     - compact;
     - row/cell/prompt iterators;
     - parser retry loops for capacity exhaustion;
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
     - capacity variants implemented;
     - replacement-node behavior;
     - pin and viewport remapping behavior;
     - dirty preservation behavior;
     - deferred column-shrink test note, if still deferred;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- PageList can replace a target page with a larger-capacity clone for styles,
  grapheme bytes, hyperlink bytes, and string bytes;
- `None` adjustment reclones the page without changing capacity;
- data, managed memory, page-level dirty state, and row-level dirty state are
  preserved;
- tracked pins and the viewport pin are remapped from the old node to the
  replacement node;
- page order, `page_size`, serial accounting, and integrity remain valid;
- any pre-install allocation/clone failure leaves `page_size`, pins, viewport,
  page order, and old page contents unchanged;
- checked doubling performs the final overflow-to-max successful increase before
  returning `OutOfSpace` on the next request;
- `OutOfSpace` leaves the PageList unchanged;
- no erase, resize/reflow, split, compact, iterator, parser, renderer, app, or
  ABI work is introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- basic replacement works, but a specific managed-memory field requires a
  narrower Page helper follow-up.

The experiment fails if:

- replacement loses row data or managed memory;
- replacement leaves tracked pins or viewport pins pointing at the dropped old
  node;
- replacement corrupts page order, `page_size`, serials, or total row
  accounting;
- out-of-space mutates the PageList;
- the implementation expands into unrelated PageList operations;
- tests or formatting fail.

## Result

**Result:** Pass

Experiment 44 ported the PageList capacity-growth replacement path:

- private `IncreaseCapacity` variants for styles, grapheme bytes, hyperlink
  bytes, and string bytes;
- private `IncreaseCapacityError`;
- checked doubling with the upstream final overflow-to-max behavior;
- max page-size guard using Roastty's existing `MAX_PAGE_SIZE`;
- `PageList::increase_capacity`, which replaces a target node with a cloned
  replacement page.

The replacement path preserves:

- page row/column size;
- text cells;
- style, grapheme, hyperlink, and string-backed managed memory;
- page-level dirty state;
- row-level dirty state;
- page order;
- page-size accounting;
- serial accounting;
- tracked pins;
- viewport pin state.

The clone-failure path rolls back temporary `page_size` and `page_serial`
accounting before returning an error, and it does not install the replacement
node. Out-of-space errors are detected before mutation and leave the PageList
unchanged.

Tests added:

- increase styles capacity and preserve visible cells;
- increase grapheme bytes capacity and preserve visible cells;
- increase hyperlink bytes capacity and preserve visible cells;
- increase string bytes capacity and preserve visible cells;
- reclone with no adjustment and unchanged capacity;
- preserve style/grapheme/hyperlink managed memory;
- remap tracked pins;
- remap the viewport pin;
- return `OutOfSpace` without mutation when the adjusted field is already maxed;
- perform the final overflow-to-max successful increase before `OutOfSpace`;
- preserve multi-page order and leave non-target pages unchanged;
- preserve page-level and row-level dirty flags.

The upstream `PageList increaseCapacity after col shrink` test remains deferred
because resize/column-shrink behavior is explicitly out of scope for this
experiment.

Verification:

```bash
cargo fmt
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Observed result:

- `cargo test -p roastty terminal::page_list`: 123 passed;
- `cargo test -p roastty`: 404 unit tests passed, plus 1 ABI harness test
  passed.

Independent review:

- Design review required explicit rollback/accounting behavior for pre-install
  clone failures and a final overflow-to-max test; both were added before the
  design commit.
- Result review found no correctness issues and approved Experiment 44 as ready
  to record as `Pass`.

## Conclusion

PageList can now grow a page's managed-memory capacity by replacing the page
node with a larger cloned node while preserving data, dirty state, pins,
viewport state, accounting, and page order. This unlocks later PageList work
that needs capacity recovery, especially compact/split and future parser retry
paths for managed-memory exhaustion.

The next experiment should continue with the next PageList-local operation that
depends on this replacement path, without pulling in full resize/reflow unless
that dependency becomes unavoidable.
