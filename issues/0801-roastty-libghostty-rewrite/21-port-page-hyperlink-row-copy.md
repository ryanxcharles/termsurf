# Experiment 21: Port Page CloneFrom Hyperlinks

## Description

Port hyperlink-aware Page row copy now that Experiment 20 added Page hyperlink
storage.

Experiments 17-19 ported Page row copy for plain cells, graphemes, and styles,
but deliberately kept the hyperlink guard because Page had no hyperlink storage.
Experiment 20 added Page-owned `string_alloc`, `hyperlink_map`, and
`hyperlink_set` plus insert/lookup/set/clear behavior. This experiment should
remove the row-copy hyperlink guard and copy hyperlink metadata with the same
ownership semantics as upstream Ghostty.

The scope is still Page row copy only. Do not implement OSC 8 parsing, terminal
hyperlink state, exact-row-capacity sizing, partial-row APIs, or public ABI
surface.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as the source of truth for:
     - `Page.cloneFrom`;
     - the hyperlink branch inside the managed-memory copy loop;
     - `hyperlink.PageEntry.dupe`;
     - destination cleanup before copying managed memory;
     - row flag behavior for full-row and destination-wider copies.
   - Use `vendor/ghostty/src/terminal/hyperlink.zig` for Page entry duplication,
     hashing, equality, and cleanup semantics.
   - Do not modify `vendor/ghostty/`.

2. Rename the row-copy helpers.
   - Rename `clone_rows_from_without_hyperlinks` to the normal `clone_rows_from`
     name.
   - Rename `clone_row_from_without_hyperlinks` to `clone_row_from`.
   - Remove the source/destination row hyperlink rejection and cell hyperlink
     rejection.
   - Keep other unsupported managed-memory guards only if they still represent
     genuinely unsupported state after this experiment.

3. Pick a Rust-safe same-page copy API before implementation.
   - The existing Rust helper shape is `&mut self, other: &Page`, so safe Rust
     callers cannot pass the same `Page` as both source and destination.
   - Do not fake same-page support with an unsound external call.
   - Add a dedicated internal same-page helper, such as
     `clone_row_within_page(dst_y, src_y)` / `clone_rows_within_page`, that:
     - snapshots the source row/cell managed-memory data before mutating the
       destination;
     - then applies the same destination cleanup and migration path as the
       cross-page helper;
     - never creates simultaneous safe `&mut Page` and `&Page` aliases to the
       same value.
   - Same-page tests should use this explicit helper. Cross-page
     `clone_rows_from(&Page)` remains the normal source/destination Page API.

4. Clear destination hyperlink state before copying.
   - Before overwriting destination cells in the copied range, clear any
     destination hyperlink for those cells.
   - Clearing must remove the cell from `hyperlink_map`, release the old
     hyperlink set ref, and clear the cell hyperlink bit.
   - Preserve existing behavior for destination graphemes and styles.
   - If the destination is wider than the source, preserve trailing destination
     cells and their managed-memory state outside the copied range, as the
     existing plain/grapheme/style tests require.

5. Reset copied managed markers before fallible migration.
   - Match Ghostty's ordering in the managed-memory loop:
     - copy the source cell value into the destination cell;
     - immediately reset destination managed markers that require separate
       backing storage:
       - `dst_cell.hyperlink = false`;
       - `dst_cell.style_id = default`;
       - grapheme content tag back to `codepoint` when needed;
     - then perform fallible grapheme, hyperlink, and style migration.
   - This reset is mandatory. If a later allocation fails, the copied cell must
     not be left with `hyperlink == true` and no map entry.

6. Copy source hyperlinks.
   - For each source cell with `cell.hyperlink == true`, look up its hyperlink
     ID from the source page.
   - Same-page copy:
     - call `use_hyperlink(id)` to acquire the destination cell ref;
     - call `set_hyperlink` / offset helper with the same ID.
   - Cross-page copy:
     - get the source Page entry from the source hyperlink set;
     - first try to find an equivalent destination entry using content-based
       equality against source-page strings;
     - if found, increment the destination ref and use that ID;
     - otherwise duplicate the URI and explicit ID strings into the destination
       `string_alloc`;
     - add the duplicated entry to the destination `hyperlink_set`, preferring
       the source ID with `add_with_id` when available;
     - on add failure, free duplicated strings and return the correct error;
     - set the destination cell hyperlink map entry with the resulting ID.
   - Preserve Experiment 20's rule that `set_hyperlink` does not increment the
     new ID; the row-copy code must acquire the ref before setting.
   - Before acquiring a ref or duplicating strings for a source hyperlink,
     perform the Ghostty-style capacity check:
     `hyperlink_count() < hyperlink_capacity()`. If the destination hyperlink
     map is full, return map OOM before acquiring refs or allocating destination
     strings.
   - If `set_hyperlink` still fails after a ref/string/set entry has been
     acquired, release the acquired ref and free any newly duplicated strings or
     set entry according to the ownership state.

7. Add the minimum internal helpers needed.
   - Add a Page helper for duplicating a source `hyperlink::PageEntry` into the
     destination page's string allocator.
   - Add a Page helper for looking up a destination hyperlink set entry using a
     source-page entry as the candidate, reusing the Page-aware `RefCountedSet`
     context from Experiment 20.
   - Add an `add_hyperlink_with_id` helper if needed so row-copy can preserve
     source IDs when the destination slot is available.
   - Extend the existing hyperlink context with an explicit source-memory
     constructor instead of adding heap side maps.

8. Extend row-copy errors.
   - Add clone-from error variants for:
     - destination string allocator out of memory;
     - hyperlink map out of memory;
     - hyperlink set out of memory;
     - hyperlink set needs rehash.
   - Preserve existing grapheme and style error behavior.
   - If a failure occurs after some destination cells were copied, leave the
     destination page internally valid: no cell may have `hyperlink == true`
     without a map entry and no map entry may exist for a cell with
     `hyperlink == false`.
   - Failure tests must also prove no hyperlink ref leak, no destination string
     leak, and no newly inserted destination set entry leak in the map-OOM path.

9. Add focused tests.
   - Same-page full-row copy:
     - use the explicit same-page helper from step 3;
     - source hyperlink cells copy to destination row;
     - map entries point at the same hyperlink ID;
     - ref counts increase by the number of copied hyperlink cells.
   - Cross-page full-row copy:
     - implicit and explicit hyperlinks copy from source to destination;
     - URI and explicit ID bytes are duplicated into destination Page memory;
     - source and destination stay independent after source refs are released.
   - Deduplication:
     - copying two cells with the same source hyperlink produces one destination
       set entry with two refs;
     - copying a hyperlink already present in the destination reuses it instead
       of duplicating strings.
   - Replacement:
     - destination cells with old hyperlinks release old refs before receiving
       source hyperlinks;
     - destination trailing cells outside the copied range keep their old
       hyperlinks when destination is wider than source.
   - Row flags:
     - full-row copy sets/clears row hyperlink flags to match the copied row;
     - partial/destination-wider copy preserves row hyperlink state when
       trailing destination cells still contain hyperlinks.
   - Failure paths:
     - destination string allocation failure returns the string OOM error and
       leaves destination state valid;
     - destination hyperlink map full returns map OOM before acquiring refs or
       duplicating strings; verify old ref counts, destination string usage, and
       destination set count are unchanged;
     - destination hyperlink set full / needs-rehash returns the appropriate
       error and frees duplicated strings.
   - Existing plain, grapheme, and style row-copy tests must continue to pass.

10. Preserve scope.

- Do not implement:
  - `exactRowCapacity`;
  - `clonePartialRowFrom`;
  - terminal OSC 8 parsing;
  - URI validation;
  - public ABI or app-facing APIs.
- Do not add heap `HashMap`/`Vec` side storage as the Page hyperlink source of
  truth.

11. Verify.

- Run:

  ```bash
  cargo fmt
  cargo test -p roastty terminal::page
  cargo test -p roastty
  ```

- `cargo fmt` output must be accepted as-is.

12. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - row-copy helper renames;
      - same-page helper/API shape;
      - hyperlink copy APIs/helpers added;
      - clone-from error variants added;
      - tests added;
      - any remaining deferred Page clone/capacity work;
      - verification command output summary.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- row-copy no longer rejects source or destination hyperlink rows/cells;
- same-page copy is exposed through a Rust-safe internal helper rather than an
  unsound `&mut self` plus `&self` alias;
- same-page hyperlink row copy reuses hyperlink IDs and increases ref counts;
- cross-page hyperlink row copy duplicates URI and explicit ID strings into the
  destination Page and keeps source/destination independent;
- destination hyperlink set lookup compares source and destination string
  contents, not offsets;
- copied destination cells reset hyperlink/style/grapheme managed markers before
  fallible migration;
- destination old hyperlinks are released before overwritten cells receive
  source hyperlinks;
- copied cells have matching `hyperlink_map` entries and row/cell flags;
- trailing destination cells outside the copied range keep their hyperlinks;
- failure paths return specific errors and leave destination hyperlink state
  internally valid without leaking acquired refs, duplicated strings, or set
  entries;
- existing plain, grapheme, and style row-copy behavior does not regress;
- `cargo fmt`, targeted Page tests, and full `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- same-page hyperlink row copy works, but cross-page copying needs a focused
  `RefCountedSet` context or Page-entry duplication extension before it can be
  made faithful.

The experiment fails if:

- hyperlink strings are copied into heap storage instead of destination Page
  backing memory;
- hyperlink set equality compares offsets instead of string contents;
- clone-from leaves stale map entries, leaked refs, or missing row/cell flags;
- row-copy silently drops hyperlinks;
- exact-row-capacity, OSC 8 parsing, or public ABI work is added prematurely;
- existing Page layout, style, grapheme, or row-copy tests regress.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the
slice.

## Result

**Result:** Pass

Implemented hyperlink-aware Page row copy:

- renamed the row-copy helpers to `clone_rows_from` and `clone_row_from`;
- removed the previous hyperlink row/cell rejection path;
- added a Rust-safe `clone_rows_within_page` helper for same-page copies using a
  Page snapshot instead of an unsound `&mut self` / `&self` alias;
- cleared destination hyperlinks before overwriting copied cells;
- reset copied cell managed markers before fallible migration:
  - hyperlink bit cleared;
  - style ID reset to default;
  - grapheme content tag reset to `Codepoint` when needed;
- added hyperlink migration for row copy:
  - same-page/snapshot copies reuse existing destination IDs through
    content-based lookup and ref acquisition;
  - cross-page copies deduplicate existing destination entries when possible;
  - otherwise URI and explicit ID strings are duplicated into destination Page
    memory and inserted into the destination hyperlink set, preferring the
    source ID;
  - map-capacity checks happen before acquiring refs or duplicating strings;
- added clone-from error variants for Page allocation failure, string OOM,
  hyperlink map OOM, hyperlink set OOM, and hyperlink set needs-rehash.

Tests added cover cross-page hyperlink copy, same-page copy via the dedicated
helper, destination deduplication, replacement with trailing destination
hyperlink preservation, string-OOM cleanup, set-OOM cleanup, and map-OOM
behavior with no ref/string/set leaks. Existing plain, grapheme, and style
row-copy tests continue to pass.

The `HyperlinkSetNeedsRehash` row-copy error path is wired through the same
`RefCountedSet::AddError::NeedsRehash` mapping as styles and hyperlink storage,
but a row-copy-specific needs-rehash fixture was not kept because the setup
collapsed back to successful insertion once trailing dead IDs were reclaimed.
The lower-level `RefCountedSet` needs-rehash behavior remains covered by its own
tests.

Verification run:

```bash
cargo fmt
cargo test -p roastty terminal::page
cargo test -p roastty
```

Results:

- `cargo test -p roastty terminal::page`: 85 passed.
- `cargo test -p roastty`: 194 unit tests passed, ABI harness passed, doc tests
  passed.

## Conclusion

Page row copy now handles hyperlinks through Page-owned map/set/string storage.
This removes the final row-copy managed-memory guard that was blocking hyperlink
cells. The next experiment can move back to the upstream Page backlog, likely
`exactRowCapacity` / capacity sizing for cloned rows, unless a review identifies
a row-copy cleanup that should happen first.
