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

# Experiment 32: Port PageList Sizing

## Description

Start the upstream `PageList` port with the smallest non-mutating foundation:
viewport tags and sizing arithmetic.

Upstream `PageList.zig` builds all list allocation and scrolling behavior on two
small helpers:

- `initialCapacity(cols)`
- `minMaxSize(cols, rows)`

These helpers decide whether a PageList can use standard pooled pages or must
start with a wider non-standard page. They also compute the minimum max-size
budget that can hold the active area plus the extra page needed by later grow,
split, and reflow algorithms.

This experiment should add those helpers in a new `terminal::page_list` module
and test them before introducing linked nodes, pools, pins, PageList
initialization, scrolling, selection, or screen integration.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `Viewport`
     - `minMaxSize`
     - `initialCapacity`
     - the beginning of the `PageList init` tests that explain the intended
       sizing behavior
   - Use `vendor/ghostty/src/terminal/page.zig` for `Capacity.adjust` and
     `Capacity.maxCols` semantics already ported to Roastty.
   - Do not modify `vendor/ghostty/`.

2. Add `roastty/src/terminal/page_list.rs`.
   - Define an internal `Viewport` enum with upstream variants:
     - `Active`
     - `Top`
     - `Pin`
   - Add internal constants derived from the Page module:
     - standard capacity (`STD_CAPACITY`)
     - standard page byte size (`page_layout(STD_CAPACITY).total_size`)
   - Add `initial_capacity(cols: CellCountInt) -> Capacity`.
   - Add `min_max_size(cols: CellCountInt, rows: CellCountInt) -> usize`.
   - Keep the module internal; do not add public ABI or app-facing APIs.

3. Preserve upstream `initialCapacity` semantics.
   - Try to adjust `STD_CAPACITY` to the requested column count first.
   - If adjustment succeeds, return the adjusted standard-sized capacity.
   - If adjustment fails because the requested columns cannot fit while
     preserving standard page size, return a non-standard capacity with
     `cols = requested cols` and the rest of `STD_CAPACITY` unchanged.
   - Preserve the invariant that `CellCountInt::MAX` columns can still layout
     with at least one row when using the non-standard path.

4. Preserve upstream `minMaxSize` semantics.
   - Compute capacity with `initial_capacity(cols)`.
   - If that capacity can hold `rows`, require one page for the active area;
     otherwise use ceil-div by `capacity.rows`.
   - Add one extra page beyond the active-area exact count.
   - Return `standard_page_size * pages`.
   - Keep the returned size tied to the standard page byte size, matching
     upstream `PagePool.item_size`, even when `initial_capacity` returns a
     non-standard wider capacity.

5. Add internal Page capacity access only as needed.
   - If `page_list.rs` needs to read or construct `Capacity` values, expose
     narrow `pub(super)` accessors or fields in `page.rs`.
   - Do not make Page capacity public outside the terminal module.
   - Do not rewrite existing Page capacity tests except for mechanical fallout
     from visibility changes.

6. Add tests.
   - `initial_capacity` returns a standard-size-adjusted capacity for normal
     widths such as 80 columns.
   - The adjusted standard capacity keeps `page_layout(cap).total_size` equal to
     `page_layout(STD_CAPACITY).total_size`.
   - `initial_capacity(STD_CAPACITY.max_cols().unwrap() + 1)` returns a
     non-standard capacity with the requested column count and total size larger
     than the standard page size.
   - `initial_capacity(CellCountInt::MAX)` lays out successfully with at least
     one row.
   - `min_max_size(80, 24)` returns exactly two standard page sizes.
   - `min_max_size` for a row count that exceeds one initial page uses
     `ceil(rows / cap.rows) + 1` standard page sizes.
   - `Viewport` variants compare as expected.

7. Preserve scope.
   - Do not implement:
     - PageList node storage or intrusive lists;
     - memory pools;
     - `PageList::init`;
     - pins or tracked pins;
     - scrollbar, scrolling, erase, resize, split, compact, selection, or
       iterators;
     - screen/parser integration;
     - public C ABI additions.
   - Do not add `ghostty` names except when citing upstream paths or test
     provenance.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - APIs added;
     - any Page capacity visibility changes;
     - tests added;
     - verification command output summary.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has an internal PageList sizing module with upstream `Viewport`,
  `initialCapacity`, and `minMaxSize` semantics;
- normal-width capacities preserve standard page byte size;
- too-wide capacities take the non-standard path;
- `min_max_size` adds the required extra standard page;
- no node/pin/scroll/resize/screen behavior or public ABI is introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- Codex reviews the experiment design and completed result and approves them, or
  all real findings are fixed.

The experiment is partial if:

- sizing helpers are correct, but a Page capacity accessor shape needs a small
  follow-up before PageList initialization;
- one test needs to be rewritten after the next PageList allocation slice gives
  a better public-internal entry point.

The experiment fails if:

- `initial_capacity` returns standard-sized capacity for a column count that
  cannot fit in the standard page size;
- `min_max_size` omits the extra page required by upstream algorithms;
- standard page size is accidentally recomputed from non-standard capacity;
- linked PageList mutation, pins, scrolling, resize, screen/parser behavior, or
  public ABI is introduced prematurely;
- tests or formatting fail.

## Result

**Result:** Pass

Implemented `roastty/src/terminal/page_list.rs` and wired it into the internal
terminal module tree.

The new module adds:

- `Viewport` with upstream PageList variants: active, top, and pin;
- `initial_capacity(cols)`;
- `min_max_size(cols, rows)`;
- internal standard-page-size calculation based on
  `page_layout(STD_CAPACITY).total_size()`.

The sizing behavior matches upstream:

- normal widths first adjust `STD_CAPACITY` while preserving the standard page
  byte size;
- widths beyond `STD_CAPACITY.max_cols()` take the non-standard capacity path by
  changing only `cols`;
- `min_max_size` uses the initial capacity to count the pages needed for the
  active area, then adds one extra standard page, matching upstream
  `PagePool.item_size * pages`.

Added narrow internal Page access:

- `Capacity::cols()`;
- `Capacity::rows()`;
- `Capacity::with_cols()`;
- `CapacityAdjustment::cols()`;
- `PageLayout::total_size()`.

No Page capacity fields were made public outside the terminal module, and no
PageList node storage, pools, pins, scrolling, resize, screen/parser behavior,
or public ABI were added.

Added tests for:

- `Viewport` variant comparisons;
- normal-width initial capacity preserving standard page size;
- maximum standard-width capacity preserving standard page size;
- too-wide initial capacity using a non-standard page;
- maximum-column initial capacity laying out successfully;
- normal `min_max_size` returning two standard pages;
- multi-page active-area `min_max_size` using ceil-div plus one extra page.

Verification passed:

```bash
cargo fmt
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

The targeted PageList sizing suite reported 7 passing tests. The full `roastty`
suite reported 288 unit tests, the ABI harness, and doc tests passing.

## Conclusion

Roastty now has the first upstream PageList foundation: sizing arithmetic that
decides standard pooled capacity versus wider non-standard capacity and computes
the minimum active-area budget with the extra page required by later PageList
algorithms. This prepares the next slice to add PageList allocation and initial
node construction without rediscovering the sizing model.
