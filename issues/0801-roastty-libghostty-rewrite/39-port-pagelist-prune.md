# Experiment 39: Port PageList Prune Growth

## Description

Port the pruning branch of upstream PageList `grow`.

Experiment 38 implemented `grow` up to the prune boundary and returned
`GrowError::WouldPrune` when upstream would prune or recycle scrollback. This
experiment should replace that deferred error with upstream behavior:

- remove the first page when max-size pressure requires pruning;
- preserve the active area by backing out of prune when pruning would leave too
  few rows;
- update pinned viewport offset caches;
- remap tracked pins that pointed into the pruned page;
- reuse standard pages by resetting and appending them;
- drop non-standard pages and allocate a fresh page instead.

This is still PageList-local growth behavior. It must not implement erase,
reset, resize, reflow, dirty tracking, prompt scrolling, screen/parser
integration, or public ABI.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - the prune branch inside `grow`;
     - `PageList grow prune scrollback`;
     - `PageList grow prune scrollback with viewport pin not in pruned page`;
     - `PageList grow allows exceeding max size for active area`;
     - `PageList grow prune required with a single page`;
     - `PageList grow reuses non-standard page without leak`;
     - `PageList grow non-standard page prune protection`.
   - Do not modify `vendor/ghostty/`.

2. Replace `GrowError::WouldPrune`.
   - Remove the temporary deferred-prune return from Experiment 38.
   - `grow` should now complete the prune/reuse path when upstream would.
   - Keep `GrowError::PageAlloc` or equivalent for fresh allocation failure.

3. Port prune preconditions exactly.
   - Consider pruning only when:
     - there is more than one page;
     - the last page is full;
     - adding a standard page would exceed `max_size()`.
   - If removing the first page and adding one new row would leave fewer rows
     than `rows`, back out of pruning and append/allocate instead. This
     preserves the active area, matching upstream.

4. Remove the first page and adjust row accounting.
   - Remove the first `Box<Node>` from `pages` without dropping it immediately.
   - Subtract its `page.size_rows()` from `total_rows`.
   - If pruning must be backed out, put the same `Box<Node>` back at the front
     and restore `total_rows`.
   - Preserve `page_size` while the node is held for possible reuse.

5. Update viewport cache during prune.
   - If `viewport == Viewport::Pin` and `viewport_pin_row_offset` is present:
     - if the cached offset is inside the pruned page, set
       `viewport = Viewport::Top`;
     - otherwise subtract the pruned page's row count from the cached offset.
   - Keep the behavior aligned with upstream's cache update before tracked-pin
     remapping.

6. Remap tracked pins from the pruned page.
   - For every tracked pin whose `node` points to the pruned page:
     - change `node` to the new first page;
     - set `x = 0`;
     - set `y = 0`;
     - set `garbage = true`.
   - After the remapping loop, set `viewport_pin.garbage = false`, matching
     upstream.
   - Because Rust stores the viewport pin in a stable `Box<Pin>`, make sure the
     tracked-pins list still points at the viewport pin after remapping.

7. Reuse standard pages.
   - If the pruned page's backing length is at or below `standard_page_size()`,
     reset its page in place using the same backing memory. Do not call
     `Page::init` in a way that allocates a fresh backing buffer for the reused
     page.
   - Add or expose a narrow `Page` reset helper if needed. The helper must:
     - zero the existing backing memory;
     - rebuild page regions over the existing memory;
     - reset layout/capacity to `initial_capacity(cols)`;
     - reset dirty state and managed-memory maps/sets;
     - preserve the backing pointer for the reused page.
   - Set the reused page size to one row.
   - Append the same `Box<Node>` at the end of `pages`.
   - Set `page_serial_min = old_serial + 1`.
   - Set the reused node's `serial = page_serial`, then increment `page_serial`.
   - Do not change `page_size`, because the page memory is reused.
   - Return the reused node pointer from `grow`.

8. Drop non-standard pages.
   - If the pruned page's backing length is greater than `standard_page_size()`,
     subtract its backing length from `page_size` and drop the node.
   - Then fall through to the existing append-new-page allocation path.
   - Verify that the new last page took the fresh allocation path using page
     size and serial accounting. Do not use raw pointer inequality as proof:
     after the non-standard node is dropped, the allocator is allowed to reuse
     the same address for the fresh node.

9. Preserve integrity.
   - `verify_integrity` must continue to validate:
     - total rows;
     - page serial minimum;
     - tracked pin validity;
     - viewport pin validity and cache consistency.
   - Add or adjust tests if pruning reveals an integrity gap.

10. Add tests.
    - Standard prune/reuse:
      - fill page 1, allocate/fill page 2, then grow again under max-size
        pressure;
      - first page is removed and reused as the new last page;
      - the reused node pointer is the original first node pointer;
      - the reused page backing pointer is unchanged;
      - `page_size` is unchanged;
      - `page_serial_min` and `page_serial` update correctly;
      - tracked pins in the pruned page remap to the new first page and are
        marked garbage.
    - Viewport pin cache inside pruned page:
      - populate a pin viewport cache inside page 1;
      - prune page 1;
      - `viewport == Viewport::Top`;
      - scrollbar offset is zero.
    - Viewport pin not in pruned page:
      - populate a pin viewport cache in page 2;
      - prune page 1;
      - viewport pin still points to page 2;
      - cached scrollbar offset decreases by the pruned row count.
    - Active-area preservation:
      - set up max-size pressure where pruning would leave too few rows;
      - grow appends/allocates instead of pruning;
      - `total_rows >= rows` remains true.
    - Non-standard prune:
      - set up a non-standard first page;
      - prune it under max-size pressure;
      - `page_size` decreases for the dropped page and then increases for the
        fresh page;
      - the new last page has fresh-allocation size and serial accounting;
      - tracked pins from the dropped page remap to the new first page and are
        marked garbage.

11. Preserve scope.
    - Do not implement:
      - erase/reset/resize/reflow;
      - dirty tracking;
      - prompt scrolling;
      - screen/parser integration;
      - renderer or app integration;
      - public C ABI additions.
    - Do not add `ghostty` names except when citing upstream paths or test
      provenance.

12. Verify.
    - Run:

      ```bash
      cargo fmt
      cargo test -p roastty terminal::page_list
      cargo test -p roastty
      ```

    - `cargo fmt` output must be accepted as-is.

13. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - prune/reuse behavior implemented;
      - tracked-pin remapping behavior;
      - viewport cache behavior;
      - non-standard page behavior;
      - tests added;
      - verification command output summary.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `grow` no longer returns `GrowError::WouldPrune`;
- standard pages are pruned, reset, and reused under upstream max-size pressure;
- non-standard pruned pages are dropped and replaced with fresh appended pages;
- `page_size`, `page_serial`, `page_serial_min`, and `total_rows` remain
  consistent;
- tracked pins in pruned pages are remapped to the new first page and marked
  garbage;
- viewport pin cache updates match upstream for pins inside and after the pruned
  page;
- active-area preservation prevents destructive pruning;
- no erase/reset/resize/reflow, dirty tracking, prompt scrolling, screen/parser,
  renderer/app, or ABI work is introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- standard page reuse works, but non-standard page drop/reallocation requires a
  narrower follow-up because current Rust storage needs adjustment.

The experiment fails if:

- pruning can leave fewer rows than the active viewport requires;
- tracked pins point to dropped/reused pages incorrectly;
- cached viewport offsets become stale;
- page size or serial accounting becomes inconsistent;
- the implementation expands beyond PageList prune growth;
- tests or formatting fail.

## Result

**Result:** Pass

Experiment 39 implemented the PageList prune-growth path and removed the
temporary `GrowError::WouldPrune` branch from Experiment 38.

The implementation adds PageList-local pruning when max-size pressure requires
it and the list has more than one page. It removes the first page, preserves the
active area by backing out if pruning would leave too few rows, updates viewport
pin cache offsets, remaps tracked pins from the pruned page to the new first
page with `garbage = true`, and keeps the viewport pin itself non-garbage to
match upstream behavior.

Standard pages are now reset in place and appended as the new last page. The
reset path preserves the existing backing pointer, rebuilds the page regions
over the same memory, resets managed-memory maps and dirty state, sets the page
to one row, updates `page_serial_min`, assigns the reused node a fresh serial,
and leaves `page_size` unchanged. Non-standard pruned pages are dropped instead:
their backing length is subtracted from `page_size`, and growth falls through to
the existing fresh-page allocation path.

Tests now cover:

- standard page prune/reuse with stable node and backing pointers;
- tracked-pin remapping from the pruned page;
- cached viewport pins inside the pruned page moving to `Viewport::Top`;
- cached viewport pins after the pruned page decrementing by the pruned row
  count;
- active-area preservation backing out of prune and appending a fresh page;
- non-standard first-page drop and fresh appended page allocation.

Verification passed:

```bash
cargo fmt -- roastty/src/terminal/page.rs roastty/src/terminal/page_list.rs
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

The focused PageList suite passed with 77 tests. The full `roastty` package
passed with 358 unit tests plus the ABI harness test.

## Conclusion

PageList growth now covers both basic page append behavior and scrollback prune
behavior. This closes the deferred prune branch from Experiment 38 and brings
the current Rust PageList growth model closer to upstream Ghostty while keeping
the work scoped to PageList internals.

The next experiment should move to the next missing PageList behavior informed
by upstream ordering, likely reset/clear, resize, or another PageList-local
operation before screen/parser integration begins.
