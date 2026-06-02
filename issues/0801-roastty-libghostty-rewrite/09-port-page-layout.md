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

# Experiment 9: Port Page Capacity and Layout Arithmetic

## Description

Port the layout-only part of upstream `Page`: `std_capacity`, `Size`,
`Capacity`, `Capacity::adjust`, `Capacity::max_cols`, and the minimal
`Page::layout` arithmetic needed to pass upstream layout/capacity tests.

Experiment 8 ported packed `Row` and `Cell` values. The next upstream page tests
are layout/capacity tests:

- `Page.layout can take a maxed capacity`
- `Page capacity adjust cols down`
- `Page capacity adjust cols down to 1`
- `Page capacity adjust cols up`
- `Page capacity adjust cols sweep`
- `Page capacity adjust cols too high`
- `Capacity maxCols basic`
- `Capacity maxCols preserves total size`
- `Capacity maxCols with 1 row exactly`

This experiment should port only the arithmetic and layout metadata needed for
those tests. It should not allocate a real page, initialize row offsets, expose
cell slices, port hash maps/ref-counted sets behavior, or implement grapheme,
style, hyperlink, clone, move, integrity, or exact-row-capacity behavior.

## Changes

1. Inspect upstream source.
   - Use these files as source of truth:
     - `vendor/ghostty/src/terminal/page.zig`
     - `vendor/ghostty/src/terminal/bitmap_allocator.zig`
     - `vendor/ghostty/src/terminal/hash_map.zig`
     - `vendor/ghostty/src/terminal/ref_counted_set.zig`
     - `vendor/ghostty/src/terminal/style.zig`
     - `vendor/ghostty/src/terminal/hyperlink.zig`
   - Do not modify `vendor/ghostty/`.

2. Add layout-only types to `roastty/src/terminal/page.rs`.
   - Add `Size`.
   - Add `Capacity`.
   - Add `CapacityAdjustment`.
   - Add `STD_CAPACITY`.
   - Add a layout-only `PageLayout` struct matching the upstream layout fields
     needed by tests:
     - `total_size`
     - `rows_start`
     - `rows_size`
     - `cells_start`
     - `cells_size`
     - `styles_start`
     - `styles_layout`
     - `grapheme_alloc_start`
     - `grapheme_alloc_layout`
     - `grapheme_map_start`
     - `grapheme_map_layout`
     - `string_alloc_start`
     - `string_alloc_layout`
     - `hyperlink_map_start`
     - `hyperlink_map_layout`
     - `hyperlink_set_start`
     - `hyperlink_set_layout`
     - `capacity`
   - Do not add a runtime `Page` allocation/owner type yet. If a namespace is
     useful, use a zero-sized layout namespace or free functions rather than a
     fake `Page`.

3. Port layout-only dependency sizing.
   - Reuse `BitmapAllocator::<CHUNK_SIZE>::layout` for grapheme and string
     allocators.
   - Add constants matching upstream:
     - `grapheme_chunk_len = 4`
     - `grapheme_chunk = grapheme_chunk_len * size_of::<u32-compatible codepoint>`
     - `grapheme_bytes_default`
     - `string_chunk_len = 32`
     - `string_chunk`
     - `string_bytes_default`
     - `hyperlink_count_default = 4`
     - `hyperlink_cell_multiplier = 16`
   - Add layout-only structs/functions for deferred dependencies:
     - style-set layout equivalent to `RefCountedSet<Style, Id, ...>::Layout`
     - grapheme offset-map layout equivalent to
       `AutoOffsetHashMap<Offset<Cell>, Offset<u21>.Slice>::layout`
     - hyperlink map layout equivalent to
       `AutoOffsetHashMap<Offset<Cell>, hyperlink.Id>::layout`
     - hyperlink set layout equivalent to `hyperlink.Set.Layout`
   - These must be explicitly marked layout-only. Do not implement insertion,
     lookup, ref-counting, hashing, or deletion behavior.

4. Preserve upstream sizing constants and alignment.
   - Use Rust `size_of` / `align_of` for actual local value types.
   - Preserve upstream page-size alignment exactly. Upstream aligns
     `Page.layout.total_size` to Zig `std.heap.page_size_min`, not to the
     runtime host page size. Before implementing, verify the upstream
     `std.heap.page_size_min` value with a temporary local Zig probe or another
     authoritative Zig source and encode that value as a documented constant. Do
     not use `libc::getpagesize`, `sysconf`, or a runtime system page-size query
     for this experiment.
   - Preserve upstream `u21` storage size for graphemes. Ghostty stores `u21` in
     4 bytes; Roastty should use a 4-byte codepoint storage size for layout
     math.
   - Do not assume Roastty's safe Rust `Style`, future hyperlink structs, or
     future map structs have the same layout as upstream Zig values. The
     layout-only dependency sizing in this experiment must use explicit
     upstream-equivalent surrogate sizes/alignments.
   - Before implementation, record or test the upstream size/alignment values
     used for:
     - `StyleSet` item value and metadata;
     - ref-counted-set table ID;
     - hash-map header;
     - hash-map metadata byte;
     - grapheme-map key/value;
     - hyperlink-map key/value;
     - hyperlink set `PageEntry` item value and metadata;
     - `PageEntry.Id` / offset-slice representation.
   - Verify uncertain size/alignment values with a temporary local Zig probe. Do
     not leave the probe in the repo.
   - Preserve hash-map layout rules from upstream:
     - capacity is already power-of-two for `layoutForCapacity`;
     - metadata is one byte per slot;
     - header stores two `Offset` values and two `u32` values;
     - keys and values are aligned independently;
     - total size is aligned to the max base alignment.
   - Preserve ref-counted set layout rules from upstream:
     - zero capacity has zero size;
     - table capacity rounds up to a power of two;
     - load factor is `0.8125`;
     - ID 0 is reserved;
     - table stores IDs;
     - items store value plus metadata.

5. Port `Capacity` behavior.
   - `Capacity` fields:
     - `cols`
     - `rows`
     - `styles`
     - `hyperlink_bytes`
     - `grapheme_bytes`
     - `string_bytes`
   - Defaults must match upstream values.
   - `STD_CAPACITY` must match upstream test-mode capacity:
     - `cols = 215`
     - `rows = 215`
     - `styles = 128`
     - `grapheme_bytes = 512` in tests
   - Implement `max_cols`.
   - Implement `adjust`, returning an error instead of panicking when the
     requested column count cannot fit one row.
   - Implement the same `available_bits_for_grid` calculation used upstream.

6. Port `layout`.
   - Port upstream `Page.layout(cap)` arithmetic exactly:
     - rows first;
     - cells aligned after rows;
     - styles after cells;
     - grapheme allocator;
     - grapheme map;
     - string allocator;
     - hyperlink set;
     - hyperlink map;
     - final total size aligned to page size.
   - Use checked arithmetic where Rust would otherwise wrap. A maxed capacity
     must not overflow in debug builds.
   - If an upstream expression intentionally saturates or clamps (for example
     hyperlink map count cast overflow), preserve that behavior.

7. Translate upstream tests.
   - Port these upstream tests:
     - `Page.layout can take a maxed capacity`
     - `Page capacity adjust cols down`
     - `Page capacity adjust cols down to 1`
     - `Page capacity adjust cols up`
     - `Page capacity adjust cols sweep`
     - `Page capacity adjust cols too high`
     - `Capacity maxCols basic`
     - `Capacity maxCols preserves total size`
     - `Capacity maxCols with 1 row exactly`
   - Add direct layout tests for:
     - `STD_CAPACITY` total size
     - layout field monotonicity/order
     - zero metadata capacities where applicable
     - dependency layout parity for representative and zero-capacity style-set
       layouts
     - dependency layout parity for representative and zero-capacity grapheme
       map layouts
     - dependency layout parity for representative and zero-capacity hyperlink
       map layouts
     - dependency layout parity for representative and zero-capacity hyperlink
       set layouts
     - bitmap allocator layout values used by page layout
   - If an expected raw layout value is uncertain, verify it with a temporary
     local Zig probe. Do not leave the probe in the repo.

8. Preserve the unsafe policy.
   - This experiment should be safe Rust only.
   - Do not add raw allocation or pointer access.
   - Do not introduce `unsafe` to mimic layout arithmetic.

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
      - layout-only dependency structs added;
      - upstream tests ported;
      - upstream tests deferred and why;
      - any divergence from upstream constants;
      - verification command output summary.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Size`, `Capacity`, `CapacityAdjustment`, `STD_CAPACITY`, and `PageLayout`
  exist;
- layout-only metadata sizing exists for style set, grapheme map, string
  allocator, hyperlink map, and hyperlink set;
- no real `Page` allocation, row/cell access, hash-map behavior, ref-counted-set
  behavior, grapheme/style/hyperlink behavior, clone, move, integrity, or
  exact-row-capacity behavior is introduced;
- all listed layout/capacity upstream tests are ported and pass;
- direct layout sanity tests and dependency layout parity tests pass;
- `cargo fmt`, targeted `cargo test -p roastty terminal::page`, and full
  `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- `Capacity` and `PageLayout` are mostly ported, but one deferred layout-only
  dependency cannot be faithfully sized without first porting more of that
  dependency. In that case, record the exact dependency and make it the next
  experiment.

The experiment fails if:

- it uses placeholder sizes that are not tied to upstream layout rules;
- it starts implementing real page allocation or metadata-map behavior;
- maxed-capacity layout overflows;
- it cannot pass the upstream capacity/layout tests.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the
slice.

## Result

**Result:** Pass

Experiment 9 ported the layout-only page capacity layer into
`roastty/src/terminal/page.rs`.

The implementation added:

- `Size`
- `Capacity`
- `CapacityAdjustment`
- `CapacityAdjustError`
- `STD_CAPACITY`
- `PageLayout`
- layout-only `StyleSetLayout`
- layout-only `GraphemeMapLayout`
- layout-only `HyperlinkMapLayout`
- layout-only `HyperlinkSetLayout`

The experiment also made `BitmapAllocator::Layout` fields visible within the
terminal module so page layout can compose allocator layout data without
duplicating it.

No real `Page` allocation, page backing memory, row/cell pointer access,
hash-map behavior, ref-counted-set behavior, grapheme/style/hyperlink behavior,
clone, move, integrity, or exact-row-capacity behavior was added.

### Layout Constants

The implementation uses upstream-equivalent layout constants rather than
assuming Roastty's safe Rust types match Zig's in-memory layout.

Temporary local Zig probes confirmed:

| Value                               | Size | Align |
| ----------------------------------- | ---: | ----: |
| `std.heap.page_size_min`            |      | 16384 |
| Zig `RGB` packed value              |    4 |     4 |
| Zig `Style.Color`                   |    8 |     4 |
| Zig `Style.Flags`                   |    2 |     2 |
| Zig `Style`                         |   28 |     4 |
| Zig `Offset(u8).Slice` equivalent   |   16 |     8 |
| Zig `hyperlink.PageEntry.Id`        |   24 |     8 |
| Zig `hyperlink.PageEntry`           |   40 |     8 |
| Zig `RefCountedSet.Item<Style>`     |   36 |     4 |
| Zig `RefCountedSet.Item<PageEntry>` |   48 |     8 |
| Zig hash-map header                 |   16 |     4 |
| Zig hash-map metadata byte          |    1 |     1 |

`PAGE_SIZE_MIN` is encoded as `16_384`, matching Zig `std.heap.page_size_min`.
The implementation intentionally does not query the runtime macOS page size.

### Upstream Tests Ported

The following upstream page layout/capacity tests were ported:

- `Page.layout can take a maxed capacity`
- `Page capacity adjust cols down`
- `Page capacity adjust cols down to 1`
- `Page capacity adjust cols up`
- `Page capacity adjust cols sweep`
- `Page capacity adjust cols too high`
- `Capacity maxCols basic`
- `Capacity maxCols preserves total size`
- `Capacity maxCols with 1 row exactly`

Additional direct tests cover:

- upstream surrogate size/alignment constants
- representative and zero-capacity style-set layouts
- representative and zero-capacity grapheme-map layouts
- representative and zero-capacity hyperlink-map layouts
- representative and zero-capacity hyperlink-set layouts
- bitmap allocator layout values used by page layout
- `STD_CAPACITY` full layout offsets and total size
- layout field ordering and final page-size alignment

The test-mode `STD_CAPACITY` layout total is `458_752` bytes with the current
upstream bitmap allocator sizing formula and the probed Zig layout constants.

### Deferred Upstream Tests

The following upstream tests remain intentionally deferred:

| Deferred area                                  | Reason                                                    |
| ---------------------------------------------- | --------------------------------------------------------- |
| `Page init` / `Page read and write cells`      | Requires page allocation and row/cell pointer access.     |
| Grapheme tests                                 | Require offset hash maps and grapheme storage behavior.   |
| Style tests inside `Page`                      | Require `StyleSet` / `RefCountedSet` behavior.            |
| Hyperlink tests                                | Require hyperlink set/map behavior and string allocation. |
| Clone/copy/move/integrity/exact-capacity tests | Require full page storage and metadata mutation behavior. |

### Verification

Ran and passed:

```bash
cargo fmt
cargo test -p roastty terminal::page
cargo test -p roastty
```

The targeted page run passed 28 tests. The full `cargo test -p roastty` run
passed 91 Rust unit tests, the C ABI harness, and doc tests.

## Conclusion

Experiment 9 succeeds. Roastty now has `Capacity`, `STD_CAPACITY`, layout-only
metadata sizing, and `PageLayout` arithmetic sufficient to pass the upstream
page layout/capacity tests without allocating a real page.

The next experiment should port basic `Page` allocation and row/cell access:
page-aligned zeroed backing memory, row initialization, `Page::init`/drop, and
the upstream `Page init` plus `Page read and write cells` tests.
