# Experiment 6: Port Bitmap Allocator

## Description

Port Ghostty's `terminal/bitmap_allocator.zig` into Roastty.

Experiment 5 identified `bitmap_allocator.zig` as the next implementation slice
because it is the smallest direct `Page` storage dependency that materially
advances the page model. It uses the `terminal::size` offset primitives already
ported in Experiment 4 and has a focused upstream test suite.

This experiment should port the allocator itself and its tests. It must not port
`Page`, `hash_map`, `ref_counted_set`, style, hyperlink, or PageList behavior.

## Changes

1. Add the module.
   - Create `roastty/src/terminal/bitmap_allocator.rs`.
   - Wire it from `roastty/src/terminal/mod.rs`.
   - Keep it internal for now. Do not expose new C ABI.

2. Port allocator shape.
   - Implement a const-generic `BitmapAllocator<const CHUNK_SIZE: usize>`.
   - Preserve:
     - `base_align = align_of::<u64>()`
     - `bitmap_bit_size = 64`
     - `bitmap: Offset<u64>`
     - `bitmap_count: usize`
     - `chunks: Offset<u8>`
   - Assert `CHUNK_SIZE` is a power of two.

3. Port layout and helpers.
   - Implement a `Layout` struct with:
     - `total_size`
     - `bitmap_count`
     - `bitmap_start`
     - `chunks_start`
   - Port:
     - `init(buf, layout)`
     - `layout(cap)`
     - `bytes_required<T>(n)`
     - `capacity_bytes`
     - `used_bytes`
     - test-only `is_allocated`
   - `init` must:
     - assert the `OffsetBuf` start pointer satisfies `base_align`;
     - initialize all bitmap words to `u64::MAX`;
     - derive `bitmap` and `chunks` offsets from `OffsetBuf::member`.
   - Preserve upstream layout math exactly:
     - `aligned_cap = align_forward(cap, CHUNK_SIZE)`;
     - `chunk_count = aligned_cap / CHUNK_SIZE`;
     - `aligned_chunk_count = align_forward(chunk_count, 64)`;
     - `bitmap_count = aligned_chunk_count / 64`;
     - `bitmap_start = 0`;
     - `bitmap_end = size_of::<u64>() * bitmap_count`;
     - `chunks_start = align_forward(bitmap_end, align_of::<u8>())`;
     - `chunks_end = chunks_start + (aligned_cap * CHUNK_SIZE)`;
     - `total_size = chunks_end`.
   - Add direct parity assertions for `Layout` fields in the layout test.

4. Port allocation and free behavior.
   - Implement allocation of `n` typed values from a caller-provided backing
     buffer.
   - Preserve:
     - `OutOfMemory` when no contiguous span exists;
     - assertion that `CHUNK_SIZE % align_of::<T>() == 0`;
     - assertion that `n > 0`;
     - marking found chunks as used;
     - `free` marking chunks free again from a typed slice.
   - Use `terminal::size::{Offset, OffsetBuf}` for backing-buffer-relative
     storage.
   - Preserve overflow behavior: byte-count and chunk-count overflow during
     allocation returns `OutOfMemory`; it must not wrap and should not panic
     accidentally.

5. Define the unsafe boundary.
   - Raw pointer/slice creation is expected here.
   - Keep unsafe code local to `bitmap_allocator.rs`.
   - Make typed allocation/free APIs `unsafe fn` unless the implementation uses
     a safe API shape whose lifetimes enforce exclusive backing-buffer access
     and same-allocator freeing.
   - Document the caller invariant for allocation/free:
     - backing buffer must be valid for the allocator layout;
     - backing buffer must outlive returned slices;
     - returned slices must be freed only once and through the same allocator
       and backing buffer;
     - typed allocation requires `CHUNK_SIZE` to satisfy alignment.
   - Do not expose safe APIs that can create arbitrary references unless their
     invariants are enforced locally.

6. Port `findFreeChunks`.
   - Preserve upstream behavior: find `n` sequential free chunks and mark them
     used, returning the starting chunk index.
   - Port all upstream tests for this function.

7. Port upstream tests.
   - Port all tests from `bitmap_allocator.zig`:
     - all `findFreeChunks...` tests;
     - `BitmapAllocator layout`;
     - allocation tests for byte and non-byte element types;
     - large allocation;
     - allocation/free fragmentation cases;
     - `BitmapAllocator bytesRequired`.
   - Preserve test intent and edge cases, even if Rust names are adjusted.

8. Format and test.
   - Run `cargo fmt` after Rust edits and accept its output.
   - Run:

     ```bash
     cargo test -p roastty terminal::bitmap_allocator
     cargo test -p roastty
     ```

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `terminal::bitmap_allocator` is implemented in Roastty with no C ABI changes;
- all upstream `bitmap_allocator.zig` tests are ported or have documented
  equivalents;
- unsafe pointer/slice creation is local and documented;
- allocation, free, capacity, used bytes, and fragmentation behavior match
  upstream tests;
- `cargo fmt` is run and accepted;
- `cargo test -p roastty terminal::bitmap_allocator` passes;
- `cargo test -p roastty` passes;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- `findFreeChunks` and layout behavior are ported, but typed allocation/free
  needs a follow-up unsafe-boundary redesign before it can be used by `Page`.

The experiment fails if:

- it starts porting `Page` or unrelated metadata structures;
- it changes public ABI;
- it leaves unsafe allocation invariants undocumented;
- it silently changes upstream allocation/free behavior;
- it cannot pass the targeted Roastty tests.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the port.
