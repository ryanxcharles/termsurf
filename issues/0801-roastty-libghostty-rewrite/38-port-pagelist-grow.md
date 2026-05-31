# Experiment 38: Port PageList Basic Growth

## Description

Port the basic non-pruning PageList row growth behavior from upstream Ghostty:

- `maxSize`
- `grow`
- `growRows`

Experiment 37 ported viewport scrolling over already-existing rows. The tests
still need to manually simulate history by mutating page sizes. This experiment
should replace that artificial setup for future tests by allowing PageList to
grow rows the way upstream does when there is enough capacity in the last page
or when a new page can be appended without pruning old scrollback.

This experiment must not implement pruning, recycled-page reuse, tracked-pin
remapping during pruning, erase/reset/resize/reflow, or screen/parser
integration.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `maxSize`;
     - `grow`;
     - `growRows`;
     - upstream tests `PageList grow fit in capacity` and
       `PageList grow allocate`.
   - Inspect prune tests only to understand what this experiment is deliberately
     deferring.
   - Do not modify `vendor/ghostty/`.

2. Add `max_size`.
   - Shape it as:

     ```rust
     fn max_size(&self) -> usize
     ```

   - Return `max(explicit_max_size, min_max_size)`, matching upstream.
   - Add tests for explicit max below/above `min_max_size`.

3. Add page creation helper.
   - Add an internal helper for creating a fresh page node from a `Capacity`.
   - Use existing `Page::init(capacity)` and `Node { page, serial }`.
   - Immediately set the fresh page's row size to zero before returning it.
     `Page::init` currently initializes a full-size page, while upstream
     `createPageExt` creates an empty page and lets callers set the row count.
   - Increment `page_serial` after assigning the serial to the new node.
   - Update `page_size` by the new page's backing length.
   - Preserve `page_serial_min`.
   - Do not add page-pool behavior or recycled-page reuse.

4. Port the fast growth path.
   - If the last page has unused row capacity, increment `last.page.size_rows`
     by one.
   - Increment `total_rows` by one.
   - Return `None`, matching upstream's "no newly allocated page" result.
   - Preserve PageList integrity.

5. Port the append-new-page path.
   - If the last page is full and appending a page would not require pruning,
     create a new page with `initial_capacity(cols)`.
   - Set the new page size to one row.
   - Push it onto `pages`.
   - Increment `total_rows`.
   - Return the new page pointer/handle. In Rust, this can be
     `Option<NonNull<Node>>` or another narrow internal representation that lets
     tests verify a new last node was appended.
   - Preserve PageList integrity.

6. Defer pruning explicitly.
   - Match upstream's prune boundary before deferring:
     - pruning is considered only when there is more than one page and adding a
       standard page would exceed `max_size()`;
     - if pruning the first page and then adding one row would leave fewer rows
       than the active viewport needs, upstream skips pruning and allocates
       anyway to preserve the active area. This experiment must allow that
       allocation.
   - Prefer returning a narrow internal error such as `GrowError::WouldPrune`,
     or panic only in tests if a fallible result would be excessive for this
     temporary internal API, only when upstream would actually enter the
     prune/reuse/remap branch.
   - The important requirement is that future pruning work is not accidentally
     papered over by ignoring `max_size`, while upstream-allowed active-area
     preservation growth still works.

7. Add `grow_rows`.
   - Shape it as:

     ```rust
     fn grow_rows(&mut self, rows: usize) -> Result<(), GrowError>
     ```

     or the matching fallible shape chosen for `grow`.

   - Implement by calling `grow` repeatedly.
   - Keep it internal/test-oriented, matching upstream's comment that `growRows`
     is only used for testing and is not optimized.

8. Add tests.
   - `max_size` returns `min_max_size` when explicit max is smaller.
   - `max_size` returns explicit max when it is larger.
   - `grow` within the last page's capacity:
     - returns no new node;
     - increments the last page size by one;
     - increments `total_rows`;
     - moves active top-left down by one row;
     - preserves integrity.
   - Active top-left assertions must use `get_top_left(point::Tag::Active)` /
     `point_from_pin` or equivalent viewport/cell mapping, not the existing
     `active_top_left()` helper. That helper currently exposes the stored
     viewport pin, not the computed active top-left.
   - `grow_rows` can create a small scrollback history without manual page-size
     mutation, and existing scroll tests can use it in at least one new test.
   - `grow` when the last page is full appends a new page:
     - returns the new node handle;
     - makes that node the last page;
     - sets the new page size to one row;
     - increments `total_rows`;
     - increments `page_size`;
     - increments `page_serial`;
     - preserves integrity.
   - A fresh page helper or append-path test proves newly created pages start
     empty and appended grow pages end with exactly one row.
   - A single-page or active-area-preservation setup that exceeds `max_size`
     still allocates when upstream would skip pruning to preserve the active
     area.
   - A true would-prune setup returns the chosen deferred-prune error instead of
     silently ignoring max size or implementing prune.

9. Preserve scope.
   - Do not implement:
     - pruning;
     - page recycling;
     - tracked-pin remapping during prune;
     - viewport fixups during prune;
     - dirty marking;
     - erase/reset/resize/reflow;
     - screen/parser integration;
     - public C ABI additions.
   - Do not add `ghostty` names except when citing upstream paths or test
     provenance.

10. Verify.
    - Run:

      ```bash
      cargo fmt
      cargo test -p roastty terminal::page_list
      cargo test -p roastty
      ```

    - `cargo fmt` output must be accepted as-is.

11. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - `grow` return/error shape;
      - page creation/accounting behavior;
      - pruning deferral behavior;
      - tests added;
      - verification command output summary.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `max_size` matches upstream `max(explicit_max_size, min_max_size)` semantics;
- `grow` can extend the last page when capacity remains;
- `grow` can append a new page when the last page is full and pruning is not
  required;
- `grow_rows` builds history by repeated `grow` calls;
- row counts, page size accounting, page serials, active viewport position, and
  integrity are updated correctly;
- upstream-allowed max-size exceedance for active-area preservation still works;
- true prune-required cases are explicit deferred-prune results, not silent
  success;
- no pruning, page recycling, tracked-pin remapping, erase/reset/resize/reflow,
  parser/screen integration, or public ABI is introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- fast growth works, but append-new-page behavior exposes a missing Page or
  PageList primitive that must be ported first.

The experiment fails if:

- `grow` creates or removes rows incorrectly;
- `page_size` or serial accounting becomes inconsistent;
- growth can silently exceed a prune-required max-size boundary;
- the implementation expands into pruning/recycling/remapping or unrelated
  PageList mutation behavior;
- tests or formatting fail.

## Result

**Result:** Pass

Implemented basic non-pruning PageList growth in
`roastty/src/terminal/page_list.rs`.

Added:

- `GrowError`;
- `max_size`;
- `create_page`;
- `grow`;
- `grow_rows`.

`grow` returns:

- `Ok(None)` when growth fits inside the last page's existing capacity;
- `Ok(Some(NonNull<Node>))` when a new page is appended;
- `Err(GrowError::WouldPrune)` when upstream would enter the prune/reuse/remap
  branch, which is explicitly deferred to a later experiment;
- `Err(GrowError::PageAlloc)` if page allocation fails.

Fresh page creation follows upstream's empty-page behavior: `Page::init` creates
a full-size page in current Roastty, so `create_page` immediately sets the page
row count to zero before returning the node. The append growth path then sets
the new page to exactly one row.

Accounting implemented:

- `page_size` increases when a fresh page is created;
- `page_serial` increments when a fresh node is assigned a serial;
- `total_rows` increments on every successful grow;
- fast growth leaves `page_size` and `page_serial` unchanged;
- append growth updates `page_size`, `page_serial`, `pages`, and `total_rows`.

Pruning remains deferred, but the deferral now matches upstream's boundary:

- single-page growth can append even when the configured explicit max is small;
- growth that would exceed max size but must preserve the active area is not
  treated as a prune error;
- true prune-required cases return `GrowError::WouldPrune` rather than silently
  ignoring max size or implementing partial pruning.

Added tests for:

- `max_size` using `min_max_size` when explicit max is smaller;
- `max_size` using explicit max when it is larger;
- `create_page` producing a zero-row page and updating accounting;
- fast in-page grow;
- `grow_rows` building history without manual page-size mutation;
- append-new-page grow;
- single-page max-size exceedance;
- true prune-required deferral.

Verification passed:

```bash
cargo fmt
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

The targeted PageList suite reported 73 passing tests. The full `roastty` suite
reported 354 unit tests, the ABI harness, and doc tests passing.

## Conclusion

Roastty PageList can now grow real history rows without test-only manual row
size mutation. The implementation covers upstream's non-pruning fast path and
append-new-page path, preserves accounting and integrity, and deliberately stops
at the prune boundary. The next growth-related experiment should port pruning,
page reuse/destruction behavior, tracked-pin remapping, and viewport cache
fixups.
