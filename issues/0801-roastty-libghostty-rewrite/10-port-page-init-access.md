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

# Experiment 10: Port Basic Page Allocation and Access

## Description

Port the first real `Page` owner: page-aligned zeroed backing memory, row
initialization, `Page::init`, drop/deallocation, and basic row/cell access.

Experiment 9 proved capacity and layout arithmetic without allocating a page.
The next upstream tests are:

- `Page init`
- `Page read and write cells`

This experiment should make those tests pass while keeping the scope tight. It
should not port grapheme append/lookup/clear, style-set behavior, hyperlink
behavior, clone/copy/move, integrity checking, exact-row-capacity, reflow, or
`PageList`.

## Changes

1. Inspect upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as source of truth.
   - Re-read:
     - `PageAlloc`
     - `Page.init`
     - `Page.initBuf`
     - `Page.deinit`
     - `Page.reinit` only for future context; do not port it yet
     - `getRow`
     - `getCells`
     - `getRowAndCell`
     - upstream tests `Page init` and `Page read and write cells`
   - Do not modify `vendor/ghostty/`.

2. Add a real `Page` struct to `roastty/src/terminal/page.rs`.
   - Include at minimum:
     - backing memory owner;
     - `rows: Offset<Row>`;
     - `cells: Offset<Cell>`;
     - `dirty: bool`;
     - `size: Size`;
     - `capacity: Capacity`;
     - layout-only or real fields for the already-portable pieces needed to
       preserve initialization shape.
   - Do not add public C ABI exposure for `Page` yet.
   - Do not add `PageList`.

3. Port page-aligned zeroed allocation.
   - Match upstream's macOS/POSIX allocation model as closely as practical.
   - Prefer `mmap`/`munmap` through the `libc` crate:
     - anonymous/private mapping;
     - read/write protection;
     - page-aligned;
     - zeroed by the OS;
     - freed exactly once in `Drop`.
   - Add `libc` as a direct `roastty` dependency if needed.
   - Do not add Linux/Windows branches. Roastty is macOS-only.
   - Keep all unsafe allocation/deallocation code inside a small helper type
     such as `PageMemory`.
   - Require `PageMemory::new(len) -> Result<PageMemory, PageAllocError>` or an
     equivalent fallible constructor:
     - `len` must be non-zero and `PAGE_SIZE_MIN` aligned;
     - call `libc::mmap`;
     - check explicitly for `libc::MAP_FAILED`, not only null;
     - preserve or expose the errno-derived allocation failure enough for
       debugging;
     - store only a successful non-null mapping and its exact length.
   - `Drop` must call `libc::munmap(ptr, len)` exactly once and only for a
     successful mapping.
   - Document the safety invariant: the memory length comes from `PageLayout`,
     is page-size aligned, remains live while offsets are dereferenced, and is
     unmapped exactly once.

4. Port `Page::init`.
   - Compute `PageLayout` from `Capacity`.
   - Assert `layout.total_size % PAGE_SIZE_MIN == 0`.
   - Allocate zeroed backing memory.
   - Initialize rows exactly like upstream `initBuf`: for each row, set the row
     cell offset to the start of that row's cells.
   - Initialize `size` to full capacity.
   - Initialize `dirty` to false.
   - Keep metadata sections zeroed/laid out, but do not implement real
     `StyleSet`, hash-map, or hyperlink-set behavior in this experiment unless
     required for the two upstream tests. Layout-only placeholders are
     acceptable if they do not expose fake behavior.

5. Port basic accessors.
   - Add immutable and mutable accessors deliberately, not accidentally:
     - `Page::get_row(&self, y) -> &Row`
     - `Page::get_row_mut(&mut self, y) -> &mut Row`
     - `Page::get_cells(&self, row) -> &[Cell]`
     - `Page::get_cells_mut(&mut self, row_index) -> &mut [Cell]`
     - `Page::get_row_and_cell_mut(&mut self, x, y) -> RowAndCellMut`
   - `Page::get_row_and_cell_mut` must take `&mut self` and return either:
     - a constrained wrapper containing `&mut Row` and `&mut Cell`, created by
       safely splitting the backing storage so Rust aliasing rules are upheld;
       or
     - a wrapper with methods that avoid simultaneously exposing overlapping
       mutable references.
   - Do not return mutable references from an `&self` method.
   - `get_cells` must define and check row provenance before converting a row's
     offset into a slice. Prefer taking `row_index` for mutable access so the
     row definitely comes from this page.
   - All offset-to-pointer logic must stay in a small unsafe boundary with clear
     safety comments.
   - Preserve upstream bounds checks:
     - `y < self.size.rows`
     - `x < self.size.cols`
   - Do not add erase/move/clone operations.

6. Translate tests.
   - Port upstream `Page init`.
   - Port upstream `Page read and write cells`.
   - Add direct tests for:
     - backing memory length equals `PageLayout.total_size`;
     - backing memory pointer is aligned to `PAGE_SIZE_MIN`;
     - `PageMemory` memory starts zeroed before `Page::init` writes row offsets
       (test this at the helper level or with a private/test-only construction
       path; do not broaden the production API just for the test);
     - every row's `cells` offset matches the expected cell range;
     - out-of-bounds row/cell access panics;
     - dropping a page does not double free in a simple create/drop loop.
   - Keep the existing layout/capacity tests green.

7. Preserve the unsafe policy.
   - Unsafe is expected in this experiment for allocation/deallocation and
     offset-to-reference conversion.
   - Unsafe blocks must be narrow and documented.
   - Safe public/internal methods should uphold their own bounds checks before
     creating references.
   - Do not expose unsafe requirements to callers unless a method is explicitly
     marked unsafe and justified.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - allocation strategy used;
     - unsafe boundaries added;
     - upstream tests ported;
     - upstream tests deferred and why;
     - verification command output summary.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Page::init` allocates page-aligned zeroed backing memory and initializes
  rows;
- `Drop` frees the backing memory exactly once;
- basic row/cell access works and preserves upstream bounds checks;
- upstream `Page init` and `Page read and write cells` tests are ported and
  pass;
- direct allocation/alignment/row-offset tests pass;
- no grapheme/style/hyperlink mutation, clone, move, integrity,
  exact-row-capacity, or `PageList` behavior is introduced;
- `cargo fmt`, targeted `cargo test -p roastty terminal::page`, and full
  `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- row/cell access works, but mmap/munmap cannot be made reliable in this slice.
  In that case, record the exact allocator issue and do not silently replace it
  with an unreviewed non-upstream allocation strategy.

The experiment fails if:

- it uses unaligned or non-zeroed backing memory;
- it leaks or double-frees backing memory;
- it exposes fake metadata-map behavior as real;
- it starts implementing later page mutation features;
- it cannot pass the upstream `Page init` and `Page read and write cells` tests.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the
slice.

## Result

**Result:** Pass

Experiment 10 ported the first real `Page` owner into
`roastty/src/terminal/page.rs`.

The implementation added:

- `Page`
- `PageMemory`
- `PageAllocError`
- `RowAndCellMut`
- `Page::init`
- `Page::get_row`
- `Page::get_row_mut`
- `Page::get_cells`
- `Page::get_cells_mut`
- `Page::get_row_and_cell_mut`

It also added `libc` as a direct `roastty` dependency so `PageMemory` can use
macOS/POSIX `mmap` and `munmap`.

### Allocation Strategy

`PageMemory` is the narrow unsafe allocation boundary:

- constructor requires non-zero, `PAGE_SIZE_MIN`-aligned lengths;
- uses `libc::mmap` with anonymous/private read-write pages;
- checks explicitly for `libc::MAP_FAILED`;
- preserves the OS allocation error through `io::Error::last_os_error`;
- stores only successful non-null mappings;
- calls `libc::munmap(ptr, len)` exactly once in `Drop`.

This matches the upstream model closely enough for the current macOS-only
Roastty port while keeping unsafe allocation/deallocation localized.

### Accessor Strategy

Mutable access is restricted to `&mut self` methods:

- `get_row_mut`
- `get_cells_mut`
- `get_row_and_cell_mut`

`get_row_and_cell_mut` returns a constrained `RowAndCellMut` wrapper containing
one row reference and one cell reference. The unsafe block is justified by the
layout invariant that row and cell arrays occupy disjoint regions of one live
page allocation.

`get_cells(&self, row)` performs an explicit row provenance check before turning
the row's cell offset into a slice.

### Upstream Tests Ported

The following upstream tests were ported:

- `Page init`
- `Page read and write cells`

Additional direct tests cover:

- `PageMemory` starts zeroed and is `PAGE_SIZE_MIN` aligned;
- backing memory length equals `PageLayout.total_size`;
- page backing pointer is `PAGE_SIZE_MIN` aligned;
- page starts not dirty;
- page size and capacity are initialized from capacity;
- every row's `cells` offset points to the expected cell range;
- `get_cells_mut`;
- corrupted row cell offsets panic before creating slices/references;
- simple create/drop loop;
- out-of-bounds row access panic;
- out-of-bounds cell access panic.

### Deferred Upstream Tests

The following upstream tests remain intentionally deferred:

| Deferred area                                 | Reason                                                    |
| --------------------------------------------- | --------------------------------------------------------- |
| `Page appendGrapheme ...` / grapheme clearing | Requires grapheme map allocation and mutation behavior.   |
| `Page clone ...` / `Page cloneFrom ...`       | Requires full metadata copy/rollback behavior.            |
| Style tests inside `Page`                     | Require `StyleSet` / `RefCountedSet` behavior.            |
| Hyperlink tests                               | Require hyperlink set/map behavior and string allocation. |
| `Page moveCells ...`                          | Requires managed-memory remapping semantics.              |
| Integrity and exact-row-capacity tests        | Require full grapheme/style/hyperlink metadata behavior.  |
| `PageList` tests                              | Require stable `Page` behavior first.                     |

### Verification

Ran and passed:

```bash
cargo fmt
cargo test -p roastty terminal::page
cargo test -p roastty
```

The targeted page run passed 37 tests. The full `cargo test -p roastty` run
passed 100 Rust unit tests, the C ABI harness, and doc tests.

## Conclusion

Experiment 10 succeeds. Roastty now has a real page backing allocation and basic
row/cell access sufficient for the upstream `Page init` and
`Page read and write cells` tests.

The next experiment should port the first managed-memory slice: grapheme map
layout plus append/lookup/clear behavior, enough to run the upstream
`Page appendGrapheme small`, `Page appendGrapheme larger than chunk`, and
`Page clearGrapheme not all cells` tests.
