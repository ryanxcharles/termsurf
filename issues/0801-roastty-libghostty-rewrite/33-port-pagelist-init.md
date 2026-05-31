# Experiment 33: Port PageList Init

## Description

Port the first usable `PageList` lifecycle slice: initial page construction,
viewport metadata, a pre-tracked viewport pin, and basic integrity checks.

Experiment 32 ported the sizing arithmetic that decides standard versus
non-standard page capacity. This experiment should use that arithmetic to build
the initial list of pages for an active terminal area, matching upstream
`PageList.init` / `initPages` behavior closely enough to support the first
upstream initialization tests.

This experiment should not port scrolling, resizing, grow/prune, reset, erase,
iterators, selection, highlighting, or screen/parser integration.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `PageList.init`;
     - `initPages`;
     - `deinit` at the semantic level only;
     - `IntegrityError`;
     - `verifyIntegrity` rows/serial/tracked-pin basics;
     - upstream tests:
       - `PageList`;
       - `PageList init rows across two pages`;
       - `PageList init more than max cols`.
   - Do not modify `vendor/ghostty/`.

2. Add initial PageList storage.
   - Add an internal `PageList` struct in `roastty/src/terminal/page_list.rs`.
   - Store pages in a Rust-owned stable-node collection suitable for later pin
     work. Prefer `Vec<Box<Node>>` for this slice:
     - each `Node` owns one `Page`;
     - each node has a monotonically assigned `serial`;
     - the `Box<Node>` gives stable node addresses even if the vector
       reallocates later;
     - later experiments can add prev/next links, split/erase, and pin
       navigation on top of the same node shape.
   - Add a `Pin` struct with:
     - a stable node pointer or equivalent handle;
     - `x`;
     - `y`;
     - `garbage`.
   - Keep `Pin` internal.
   - The viewport pin handle itself must also be stable. Do not store tracked
     pin references to an inline `viewport_pin` field that would move if the
     `PageList` value moves. Use a boxed/arena-owned pin, or an index/generation
     handle, so tracked pins can safely identify the viewport pin after the
     `PageList` struct is moved.
   - Keep the implementation macOS/Rust-only; do not port Zig memory-pool
     machinery literally unless a later experiment proves a pool is needed.

3. Port `PageList::init`.
   - Shape it as:

     ```rust
     fn init(cols: CellCountInt, rows: CellCountInt, max_size: Option<usize>) -> Result<Self, PageListAllocError>
     ```

   - Build pages with an `init_pages` helper:
     - compute `cap = initial_capacity(cols)`;
     - allocate as many pages as needed to cover `rows`;
     - set each page's active `size.rows` to `min(remaining_rows, cap.rows)`;
     - increment serial for every node;
     - return pages and total allocated page bytes.
   - Set:
     - `cols`;
     - `rows`;
     - `page_serial`;
     - `page_serial_min = 0`;
     - `page_size`;
     - `explicit_max_size = max_size.unwrap_or(usize::MAX)`;
     - `min_max_size = min_max_size(cols, rows)`;
     - `total_rows = rows`;
     - `viewport = Viewport::Active`;
     - `viewport_pin` pointing at the first page, x/y 0;
     - tracked pins containing the viewport pin.

4. Add narrow Page helpers only as needed.
   - Add `Page::set_size_rows` or an equivalent internal helper to set the
     active row count after initialization.
   - Add `Page::size_rows`, `Page::size_cols`, `Page::capacity_rows`, or
     `Page::memory_len` only if needed for PageList tests/integrity.
   - Keep all helpers `pub(super)` and do not expose Page internals outside the
     terminal module.

5. Add basic PageList integrity checks.
   - Add an internal `IntegrityError` enum for this slice.
   - Verify:
     - every node serial is at least `page_serial_min`;
     - sum of page active rows equals cached `total_rows`;
     - every tracked pin points to a live node;
     - every tracked pin coordinate is inside its target page's active size
       (`x < page.size.cols`, `y < page.size.rows`), matching upstream
       `pinIsValid`;
     - `viewport_pin` points to a live node, has in-bounds coordinates, and is
       not garbage.
   - Do not implement viewport-pin row-offset checks yet, because scrolling to a
     pinned viewport is out of scope.

6. Add tests.
   - Port the relevant parts of upstream `PageList`:
     - `PageList::init(80, 24, None)` creates an active viewport;
     - at least one page exists;
     - total rows equals requested rows;
     - `total_rows` cache equals requested rows;
     - viewport pin points at the first page;
     - active top-left is first page at x/y 0 if a helper exists in this slice.
   - Port `PageList init rows across two pages`:
     - choose columns that make the initial capacity hold fewer rows than the
       requested active area;
     - verify more than one page is created;
     - verify the sum of page active rows equals requested rows.
   - Port `PageList init more than max cols`:
     - initialize with `STD_CAPACITY.max_cols().unwrap() + 1`;
     - verify the first page uses non-standard memory/layout larger than the
       standard page size;
     - verify total rows/cache/viewport state.
   - Add Rust-specific integrity tests:
     - corrupt cached `total_rows` and verify integrity reports a mismatch;
     - corrupt `page_serial_min` beyond a node serial and verify integrity
       reports invalid serial;
     - mark the viewport pin garbage and verify integrity rejects it.
     - corrupt `viewport_pin.x` or `viewport_pin.y` beyond the target page's
       active size and verify integrity rejects it.

7. Preserve scope.
   - Do not implement:
     - scrolling or viewport pin offsets;
     - `scrollbar`;
     - `getTopLeft` / `getBottomRight` beyond a tiny active top-left helper if
       needed by tests;
     - reset, grow, resize, split, compact, erase, clone, or iterators;
     - selection or highlighting;
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
     - storage shape chosen;
     - Page helpers added;
     - tests added;
     - verification command output summary.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `PageList::init` creates enough pages to cover the active row count;
- normal, multi-page, and non-standard-width initialization cases pass;
- viewport starts as active and the viewport pin points at the first page;
- basic integrity catches total-row, serial, viewport-pin garbage, and
  out-of-bounds tracked-pin coordinates;
- no scroll/resize/grow/reset/erase/screen/parser/public ABI behavior is added;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- initialization works, but the node/pin handle shape needs one follow-up before
  scroll or erase work can safely build on it;
- one upstream init assertion is deferred because it depends on scrollbar or
  top-left conversion behavior outside this slice.

The experiment fails if:

- pages do not cover the requested active row count;
- node addresses/handles are not stable enough for later pins;
- non-standard-width initialization incorrectly uses standard page capacity;
- integrity accepts corrupted total rows, stale serials, a garbage viewport pin,
  or out-of-bounds tracked-pin coordinates;
- linked-list mutation, scrolling, resize, grow, reset, screen/parser behavior,
  or public ABI is introduced prematurely;
- tests or formatting fail.

## Result

**Result:** Pass

Implemented the first usable PageList lifecycle slice in
`roastty/src/terminal/page_list.rs`.

The storage shape is:

- `PageList` owns `Vec<Box<Node>>`;
- each `Node` owns one `Page` and a monotonically assigned serial;
- boxed nodes provide stable node addresses for later pin work;
- `Pin` stores a stable `NonNull<Node>` plus x/y coordinates and a garbage flag;
- the viewport pin is itself boxed, and `tracked_pins` stores the stable pin
  address, avoiding references to a movable inline field.

Implemented:

- `PageList::init(cols, rows, max_size)`;
- `init_pages`;
- active viewport metadata;
- cached page size, min/max size, explicit max size, total rows, and page
  serials;
- a pre-tracked viewport pin pointing at the first page;
- basic PageList integrity checks for serials, cached total rows, live tracked
  pins, in-bounds pin coordinates, and garbage viewport pins.

Added narrow internal Page helpers:

- `Page::size_cols()`;
- `Page::size_rows()`;
- `Page::set_size_rows()`.

No scrolling, viewport pin offsets, scrollbar, reset, grow, resize, split,
compact, erase, iterators, screen/parser behavior, or public C ABI was added.

Added tests for:

- normal `PageList::init(80, 24, None)`;
- explicit max-size metadata;
- initialization across multiple pages;
- initialization with more columns than fit in the standard page capacity;
- integrity rejection for cached total-row mismatch;
- integrity rejection for stale serials;
- integrity rejection for a garbage viewport pin;
- integrity rejection for out-of-bounds viewport pin x/y coordinates.

Verification passed:

```bash
cargo fmt
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

The targeted PageList suite reported 16 passing tests. The full `roastty` suite
reported 297 unit tests, the ABI harness, and doc tests passing.

## Conclusion

Roastty now has a real PageList initialization foundation. It can allocate the
active area across one or more pages, handle non-standard-width initial pages,
track the active viewport pin in a stable Rust-owned shape, and reject the basic
corrupt states that upstream checks before higher-level scrolling and mutation
logic starts building on the list.
