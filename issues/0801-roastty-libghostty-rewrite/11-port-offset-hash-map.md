# Experiment 11: Port Offset Hash Map Storage

## Description

Port Ghostty's offset-backed terminal hash map into Roastty as the next Page
storage prerequisite.

Experiment 10 made `Page` allocate and expose rows/cells, but the next upstream
Page behavior (`appendGrapheme`, `lookupGrapheme`, `clearGrapheme`, and later
hyperlinks) depends on `AutoOffsetHashMap`. That map is not a normal `HashMap`:
it stores metadata, keys, and values inside caller-provided backing memory and
keeps only offsets so the whole Page allocation can be copied or rebased without
invalidating the map.

This experiment should port the hash-map storage substrate before adding
grapheme behavior. Do not do another broad Zig-to-Rust pattern pass. The general
policy from Experiment 2 is sufficient; this experiment should solve the
specific unsafe/storage pattern needed by the current Page implementation.

## Changes

1. Inspect upstream source.
   - Use `vendor/ghostty/src/terminal/hash_map.zig` as source of truth.
   - Re-read:
     - `AutoOffsetHashMap`
     - `OffsetHashMap`
     - `HashMapUnmanaged`
     - `Metadata`
     - `Header`
     - `init`
     - `layoutForCapacity`
     - `get`, `getPtr`, `getEntry`
     - `putNoClobber`, `putAssumeCapacity`, `putAssumeCapacityNoClobber`
     - `fetchPutAssumeCapacity`
     - `remove`, `removeByPtr`
     - `clearRetainingCapacity`
     - `count`, `capacity`, and iterators
     - upstream `HashMap` tests at the bottom of the file
   - Re-read the Page grapheme call sites in
     `vendor/ghostty/src/terminal/page.zig` so the API is shaped for the real
     next consumer.
   - Do not modify `vendor/ghostty/`.

2. Add a Roastty offset hash map module.
   - Add `roastty/src/terminal/offset_hash_map.rs`.
   - Wire it from `roastty/src/terminal/mod.rs`.
   - Keep the module internal to the terminal implementation.
   - Preserve Roastty naming in code and docs. Upstream Ghostty names are
     allowed only when citing source provenance.

3. Port the backing-memory layout.
   - Represent the upstream header explicitly with `#[repr(C)]`.
   - Represent metadata as one byte, matching upstream:
     - free slot: byte value `0`
     - tombstone slot: byte value `1`
     - used slots: high seven-bit fingerprint plus used bit
   - Represent key/value slot storage so Rust never creates references to
     uninitialized keys or values.
     - Prefer `MaybeUninit<K>` and `MaybeUninit<V>` for backing arrays.
     - Only create `&K`, `&V`, or `&mut V` for slots whose metadata is `used`.
     - New, tombstone, and free slots must remain raw `MaybeUninit` storage.
     - Removal must mark metadata tombstone before dropping or forgetting slot
       contents, and must not read the removed slot afterward.
     - Because Page's first consumers use `Copy` key/value types, this
       experiment may require `K: Copy` and `V: Copy` and avoid destructor
       management. If non-`Copy` support is needed later, add it in a separate
       reviewed expansion with explicit drop semantics.
   - Add a `Layout` equivalent with:
     - `total_size`
     - `keys_start`
     - `vals_start`
     - `capacity`
   - The layout calculation must match the layout already used by `PageLayout`
     for the same key/value sizes and alignments.
   - Preserve upstream's metadata-relative layout convention:
     - the header lives immediately before the metadata pointer;
     - `keys_start` and `vals_start` are offsets relative to the metadata
       pointer, not the beginning of the whole backing buffer;
     - the public layout's `total_size` still covers header, metadata, keys, and
       values.
   - Keep layout constants and `size_of` / `align_of` assertions in tests.

4. Port initialization from caller-provided storage.
   - Initialize the map inside an `OffsetBuf`/Page backing range.
   - Store an `Offset<Metadata>` or equivalent offset-based handle, not a raw
     permanent pointer.
   - On access, derive the raw metadata/header/key/value pointers from the
     current base pointer.
   - Zero/initialize metadata during `init`.
   - Set header capacity and size exactly once during initialization.
   - The backing buffer must not allocate heap memory outside the caller's Page
     storage.

5. Port the fixed-capacity operations needed by Page.
   - Implement, at minimum:
     - `count`
     - `capacity`
     - `ensure_total_capacity`
     - `ensure_unused_capacity`
     - `clear_retaining_capacity`
     - `contains`
     - `get`
     - `get_mut` or `get_ptr` equivalent
     - `get_entry`
     - `put_no_clobber`
     - `put_assume_capacity`
     - `put_assume_capacity_no_clobber`
     - `fetch_put_assume_capacity`
     - `remove`
     - `remove_by_ptr` or a safe equivalent that supports Page's move/clear use
       cases
     - iteration over used key/value entries
   - The map must remain fixed-capacity. If inserting would exceed capacity,
     return a local `OutOfMemory` error; do not grow or allocate.
   - It is acceptable to use Rust trait bounds such as `K: Copy + Eq + Hash` and
     `V: Copy` for the initial port, as long as the Page key/value types needed
     by graphemes and hyperlinks fit those bounds.
   - Hash slot order is not yet externally observable by Page tests. Use a
     deterministic, non-random, 64-bit hash path in this experiment, and
     document the choice.
     - Do not use `DefaultHasher` or another process-randomized hash path.
     - Extract metadata fingerprints from the high seven bits of the 64-bit
       hash, matching upstream's metadata model.
     - If later serialization or byte-for-byte map slot parity requires
       Ghostty's exact Wyhash placement, design that as a follow-up instead of
       silently changing this experiment's scope.

6. Preserve unsafe boundaries.
   - Unsafe is expected for:
     - deriving the header pointer before metadata;
     - converting offsets into metadata/key/value `MaybeUninit` slices;
     - returning mutable entry references from one backing allocation.
   - Keep unsafe code inside `offset_hash_map.rs`.
   - Each unsafe block must include a short safety comment.
   - Safe methods must check:
     - non-zero/power-of-two capacity where required;
     - backing buffer alignment;
     - key/value region ranges before creating slices;
     - insertion capacity before writing a new entry.
   - Safe methods must not produce references to uninitialized key/value slots.
   - Do not expose unsafe requirements to Page callers.

7. Connect layout names without changing Page behavior.
   - Either replace the private hash-map layout helper in `page.rs` with the new
     module's layout function, or add tests proving both calculations stay
     identical.
   - Do not implement `Page::append_grapheme`, `lookup_grapheme`,
     `clear_grapheme`, hyperlink maps, styles, clone, move, reflow, integrity
     checks, or `PageList` in this experiment.
   - Do not change `Page` semantics except where needed to compile the new
     module and share layout helpers.

8. Translate upstream tests.
   - Account for every upstream test in
     `vendor/ghostty/src/terminal/hash_map.zig`.
   - Port the directly relevant tests:
     - `HashMap basic usage`
     - `HashMap ensureTotalCapacity`
     - `HashMap ensureUnusedCapacity with tombstones`
     - `HashMap clearRetainingCapacity`
     - `HashMap ensureTotalCapacity with existing elements`
     - `HashMap remove`
     - `HashMap reverse removes`
     - `HashMap multiple removes on same metadata`
     - `HashMap put and remove loop in random order`
     - `HashMap put`
     - `HashMap put full load`
     - `HashMap putAssumeCapacity`
     - `HashMap repeat putAssumeCapacity/remove`
     - `HashMap getOrPut`
     - `HashMap basic hash map usage`
     - `HashMap ensureUnusedCapacity`
     - `HashMap removeByPtr`
     - `HashMap repeat fetchRemove`
     - `OffsetHashMap basic usage`
     - `OffsetHashMap remake map`
     - `layoutForCapacity no overflow for large capacity`
   - The zero-sized-key upstream test may be deferred if the initial Rust port
     requires non-zero-sized `K`; if deferred, record why Page does not need it
     and what would be required to support it later.
   - Add Roastty-specific tests for:
     - layout parity with the current `GraphemeMapLayout` and
       `HyperlinkMapLayout` values;
     - offset rebasing: initialize a map in one buffer, copy the bytes to a
       second buffer at a different address, construct/use the offset map with
       the second base pointer, and confirm lookups/removals still work;
     - tombstone reuse;
     - capacity-full insertion failure without corrupting existing entries;
     - no references are created for free/tombstone slots; where direct proof is
       not possible, test through APIs that iterate, get, remove, and reuse
       tombstones under Miri-compatible safe surfaces;
     - `remove_by_ptr` or the chosen safe equivalent.

9. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::offset_hash_map
     cargo test -p roastty terminal::page
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

10. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - hash/layout strategy used;
      - unsafe boundaries added;
      - upstream tests ported;
      - tests deferred and why;
      - whether Page layout now uses the shared map layout function or is still
        proven equivalent by tests;
      - verification command output summary.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has an internal offset-backed hash map module;
- its backing layout matches the Page map layout expectations already ported in
  Experiment 9;
- map storage lives entirely inside caller-provided backing memory;
- the map can be rebased by copying backing bytes to a different address;
- fixed-capacity insertion, lookup, mutation, removal, tombstones, iteration,
  and capacity failure behavior work;
- the listed upstream hash-map tests or equivalent Rust tests pass;
- `Page` tests remain green;
- no grapheme/hyperlink/style behavior is added prematurely;
- `cargo fmt`, targeted hash-map tests, targeted Page tests, and full
  `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- the offset map works for normal fixed-capacity use, but rebase behavior or
  `remove_by_ptr` equivalent needs one more focused slice before Page can use it
  safely.

The experiment fails if:

- it stores permanent raw pointers instead of offsets;
- it allocates heap storage outside the caller-provided buffer;
- it changes Page behavior while trying to port the map;
- it implements grapheme or hyperlink behavior before the map substrate is
  proven;
- it cannot pass the upstream hash-map behavior tests.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the
slice.
