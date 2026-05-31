# Experiment 12: Port Page Grapheme Storage

## Description

Port the first real managed-memory Page behavior: grapheme append, lookup,
clear, row flag updates, and count/capacity reporting.

Experiments 6, 10, and 11 supplied the prerequisites:

- `BitmapAllocator`
- page-aligned `Page` storage
- packed `Row` and `Cell` values
- offset-backed hash maps that can live inside Page storage

Upstream Ghostty stores extra codepoints for a grapheme separately from the
first cell codepoint. The cell keeps the first codepoint and a
`codepoint_grapheme` tag; the Page stores the extra codepoints in the grapheme
bitmap allocator and indexes them by cell offset in the grapheme offset map.

This experiment should port that behavior, not later Page features. Do not add
style sets, hyperlink behavior, clone/move/reflow, integrity scans, exact row
capacity, `PageList`, parser behavior, or terminal screen behavior.

## Changes

1. Inspect upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as source of truth.
   - Re-read:
     - `GraphemeAlloc`
     - `GraphemeMap`
     - `GraphemeError`
     - `setGraphemes`
     - `appendGrapheme`
     - `lookupGrapheme`
     - `clearGrapheme`
     - `updateRowGraphemeFlag`
     - `graphemeCount`
     - `graphemeCapacity`
   - Re-read upstream tests:
     - `Page appendGrapheme small`
     - `Page appendGrapheme larger than chunk`
     - `Page clearGrapheme not all cells`
   - Inspect later grapheme tests for future context, but do not port clone,
     move, scroll, or integrity behavior in this experiment.
   - Do not modify `vendor/ghostty/`.

2. Initialize real grapheme storage inside `Page`.
   - Add real `grapheme_alloc: GraphemeAlloc`.
   - Add real
     `grapheme_map: Option<OffsetHashMap<Offset<Cell>, OffsetSlice<u32>>>`.
   - Initialize both inside `Page::init` using the existing `PageLayout`
     offsets:
     - `layout.grapheme_alloc_start`
     - `layout.grapheme_alloc_layout`
     - `layout.grapheme_map_start`
     - `layout.grapheme_map_layout`
   - Do not allocate outside `PageMemory`.
   - Handle zero-capacity grapheme maps explicitly:
     - if `layout.grapheme_map_layout.capacity == 0`, store `None`;
     - appending a new grapheme in that state returns a grapheme map
       out-of-memory error before allocating codepoint storage;
     - lookup returns `None`;
     - count and capacity return `0`;
     - clear asserts/panics only if the caller tries to clear a cell marked as
       having graphemes without map storage, because that indicates corrupted
       Page state.
   - Do not initialize style, string, hyperlink, or hyperlink-set behavior.

3. Add safe Page-memory slice helpers as needed.
   - `OffsetHashMap::map` currently requires an exclusive mutable backing slice.
   - Extend the offset-map API with a read-only view before using it from Page:
     - `map_ref(&self, backing: &[u8]) -> MapRef`
     - read-only operations: `count`, `capacity`, `contains`, `get`, and
       iteration if needed by tests
     - mutable operations remain on the exclusive `map(&mut [u8]) -> Map` view.
   - Add narrow `PageMemory` helpers for:
     - immutable backing slices when lookup needs read-only access;
     - mutable backing slices when appending/clearing needs map mutation.
   - Keep helpers private to `page.rs` unless another terminal module actually
     needs them.
   - Preserve the unsafe policy: any pointer-to-slice conversion must have a
     short safety comment and stay inside `PageMemory` or Page internals.

4. Avoid Rust aliasing pitfalls from the upstream pointer API.
   - Upstream methods accept `*Page`, `*Row`, and `*Cell` together.
   - In Rust, a caller holding `&mut Row` / `&mut Cell` from `Page` cannot also
     safely call a `&mut Page` method that mutates managed memory.
   - Prefer coordinate-based Rust methods for this slice:
     - `append_grapheme_at(x, y, cp)`
     - `lookup_grapheme_at(x, y)`
     - `clear_grapheme_at(x, y)`
     - `update_row_grapheme_flag(row_index)`
   - The methods may use small internal raw-pointer sections to update the row,
     cell, allocator, and map under one exclusive `&mut Page` borrow.
   - Avoid live whole-Page mutable slices while mutating any other Page memory:
     - do map/allocator work inside a narrow scope with no live row/cell
       references;
     - do not call `BitmapAllocator::alloc` or `BitmapAllocator::free` while a
       full-Page mutable map view is live;
     - drop the map view before changing the cell tag, row flag, or allocator
       bitmap/chunk storage;
     - compute cell offsets from layout/x/y instead of from borrowed cells when
       possible;
     - use raw row/cell pointers only after the full backing-slice borrow has
       ended.
   - Do not expose an API that requires callers to hold row/cell references and
     then also borrow `Page` mutably.
   - Keep `get_row_and_cell_mut` for the existing basic tests; do not force it
     into the grapheme API.

5. Port grapheme append behavior.
   - Reject or assert invalid base cells the same way upstream does:
     - the cell must contain a non-zero codepoint before appending;
     - the first codepoint remains on the cell itself;
     - appended codepoints are extra grapheme codepoints only.
   - If the cell has no graphemes:
     - allocate one `u32` from `grapheme_alloc`;
     - write the appended codepoint;
     - insert an `OffsetSlice<u32>` into `grapheme_map`;
     - set the cell content tag to `CodepointGrapheme`;
     - set the row grapheme flag.
   - If the existing grapheme slice has spare capacity inside the current
     `GRAPHEME_CHUNK_LEN`, append in-place and increment the slice length.
   - If the slice is full:
     - allocate a larger slice;
     - copy the old codepoints;
     - append the new codepoint;
     - update the map value;
     - free the old slice.
   - Preserve rollback behavior:
     - if allocation fails before the map insert, leave cell and row flags
       unchanged;
     - if map insert fails after allocation, free the new slice and leave cell
       and row flags unchanged.
     - for growth of an existing grapheme slice, allocate/copy/update/free in an
       order that either leaves the old slice still mapped or completes with the
       new slice mapped; never leave the map pointing at freed storage.

6. Port lookup and clear behavior.
   - `lookup_grapheme_at(x, y)` returns the extra codepoints only, not the
     cell's first codepoint.
   - It may return an owned `Vec<u32>` initially if returning a borrowed slice
     would complicate safe aliasing. If an owned vector is used, document the
     divergence and keep the internal storage faithful.
   - `clear_grapheme_at(x, y)`:
     - finds and copies the map entry;
     - removes the map entry;
     - drops the map view;
     - frees the copied grapheme allocation;
     - changes the cell content tag back to `Codepoint`.
   - `update_row_grapheme_flag(row_index)` scans the row's cells and clears the
     row flag only when no cells in that row still have graphemes.

7. Add grapheme count/capacity helpers.
   - Add `grapheme_count`.
   - Add `grapheme_capacity`.
   - These count map entries, not byte size.

8. Translate tests.
   - Port upstream tests:
     - `Page appendGrapheme small`
     - `Page appendGrapheme larger than chunk`
     - `Page clearGrapheme not all cells`
   - Add Rust-specific tests for:
     - lookup returns only appended codepoints;
     - row grapheme flag remains true while another cell in the row still has a
       grapheme;
     - clearing the last grapheme in a row clears the row flag after
       `update_row_grapheme_flag`;
     - append growth past `GRAPHEME_CHUNK_LEN` preserves existing codepoints;
     - `grapheme_count` and `grapheme_capacity` report map entry count and map
       capacity;
     - zero-capacity grapheme map behavior:
       - append returns map out-of-memory;
       - lookup returns `None`;
       - count/capacity return `0`;
       - cell and row flags remain unchanged;
     - map out-of-memory on a new cell leaves no stale allocation and leaves the
       cell/row flags unchanged;
     - allocator out-of-memory on append/growth leaves existing grapheme data
       intact if that capacity can be constructed without changing production
       APIs;
     - clearing after growth frees the active allocation and removes the map
       entry.

9. Preserve scope.
   - Do not port:
     - `moveGrapheme`
     - `setGraphemes` unless it is strictly needed by the append tests;
     - clone/copy behavior;
     - row erasure;
     - scroll/reflow;
     - integrity checking;
     - style or hyperlink managed memory.
   - If `setGraphemes` proves necessary for clean implementation, keep it
     private and include only enough tests to prove it does not broaden scope.

10. Verify.
    - Run:

      ```bash
      cargo fmt
      cargo test -p roastty terminal::page
      cargo test -p roastty terminal::offset_hash_map
      cargo test -p roastty
      ```

    - `cargo fmt` output must be accepted as-is.

11. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - APIs added;
      - storage fields initialized;
      - borrow/aliasing shape chosen;
      - unsafe boundaries added;
      - upstream tests ported;
      - deferred upstream tests and why;
      - verification command output summary.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Page` initializes real grapheme allocator and map storage inside its backing
  memory;
- append, lookup, clear, row flag updates, count, and capacity work;
- the three upstream grapheme tests are ported and pass;
- growth past `GRAPHEME_CHUNK_LEN` preserves existing codepoints;
- map/allocator failure paths do not leave cell/row flags lying about stored
  graphemes;
- the implementation avoids exposing an aliasing-unsafe Rust API for Page +
  Row/Cell mutation;
- no style/hyperlink/clone/move/reflow/PageList behavior is introduced;
- `cargo fmt`, targeted Page tests, targeted offset-map tests, and full
  `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- append/lookup/clear work for normal cases, but failure rollback or borrowed
  lookup slices need one more focused experiment before later Page operations
  can rely on grapheme storage.

The experiment fails if:

- grapheme data is stored outside Page backing memory;
- lookup includes the cell's first codepoint instead of only appended
  codepoints;
- cell/row grapheme flags can become true without matching map data;
- clear frees the wrong allocation or leaves a stale map entry;
- the implementation requires callers to hold row/cell mutable references while
  also mutably borrowing `Page`.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the
slice.

## Result

**Result:** Pass

Experiment 12 ported Page grapheme storage into Roastty.

The implementation added:

- real `grapheme_alloc` initialization inside `Page`
- real optional `grapheme_map` initialization inside `Page`
- `GraphemeError`
- `append_grapheme_at`
- `lookup_grapheme_at`
- `clear_grapheme_at`
- `update_row_grapheme_flag`
- `grapheme_count`
- `grapheme_capacity`
- a read-only `OffsetHashMap::map_ref` / `MapRef` view
- `OffsetSlice::offset`, `OffsetSlice::len`, and `OffsetSlice::slice_mut`

### Borrowing Shape

The Rust API intentionally uses coordinates instead of upstream Zig's
`Page*`/`Row*`/`Cell*` pointer shape:

- `append_grapheme_at(x, y, cp)`
- `lookup_grapheme_at(x, y)`
- `clear_grapheme_at(x, y)`
- `update_row_grapheme_flag(row_index)`

This keeps the map, allocator, row, and cell mutations under one `&mut Page`
borrow. The implementation avoids holding a full-Page mutable map view while
mutating row/cell bytes or bitmap allocator storage.

Lookup uses the new read-only offset-map view and returns an owned `Vec<u32>`.
That is a small API-shape divergence from upstream's borrowed slice, but the
internal storage remains faithful: appended codepoints live in Page-managed
bitmap-allocated storage and are indexed by cell offset in the grapheme map.

### Storage and Rollback

`Page::init` now initializes grapheme storage from the existing `PageLayout`:

- `layout.grapheme_alloc_start`
- `layout.grapheme_alloc_layout`
- `layout.grapheme_map_start`
- `layout.grapheme_map_layout`

If the grapheme map capacity is zero, `Page` stores `None`. In that state,
append returns `GraphemeMapOutOfMemory` before allocating codepoint storage,
lookup returns `None`, and count/capacity return `0`.

The append paths preserve rollback:

- map out-of-memory after allocation frees the newly allocated slice and leaves
  cell/row flags unchanged;
- allocator out-of-memory during growth leaves the old mapped slice intact;
- growth updates the map to the new slice before freeing the old slice, so the
  map never points at freed storage.

### Tests Ported

The upstream tests ported are:

- `Page appendGrapheme small`
- `Page appendGrapheme larger than chunk`
- `Page clearGrapheme not all cells`

Additional Roastty tests cover:

- lookup excludes the cell's first codepoint;
- grapheme count and capacity;
- zero-capacity grapheme map behavior;
- map out-of-memory rollback;
- allocator out-of-memory preserving existing data;
- clear-after-growth freeing the active allocation and removing the map entry.

Deferred upstream grapheme tests are the later clone, move, scroll/reflow, and
integrity tests. Those depend on Page operations that are intentionally outside
this experiment's scope.

### Verification

The required verification passed:

```bash
cargo fmt
cargo test -p roastty terminal::page
cargo test -p roastty terminal::offset_hash_map
cargo test -p roastty
```

Observed results:

- `terminal::page`: 46 passed
- `terminal::offset_hash_map`: 23 passed
- full `roastty` suite: 132 Rust unit tests passed, C ABI harness passed, doc
  tests passed

## Conclusion

Roastty now has the first real managed-memory behavior in `Page`: grapheme
storage. The next experiment can build on this by porting the next Page
operation that depends on managed cell data, likely grapheme movement or the
next scoped Page mutation needed before clone/reflow/integrity can be ported.
