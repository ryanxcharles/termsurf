+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 406: the foreground-cell buffer upload (sync_from_array_lists)

## Description

The cell-assembly pass (Experiments 384–405) fills `Contents`: a flat `bg_cells`
slice and `fg_rows` — a **list of per-row vertex lists**
(`Vec<Vec<CellTextVertex>>`). Uploading the background to the GPU is a plain
`MetalBuffer::sync` of the flat slice (already ported). Uploading the
**foreground** is different: upstream concatenates the per-row array lists into
one contiguous GPU buffer in a single pass. This experiment ports that
primitive, `syncFromArrayLists`, into roastty's `MetalBuffer` as
`sync_from_array_lists`, so the foreground vertices can be uploaded the way
upstream's frame draw does
(`frame.cells.syncFromArrayLists(self.cells.fg_rows.lists)`).

## Upstream behavior

In `renderer/metal/buffer.zig`, `syncFromArrayLists` mirrors `sync` but reads
from an array of array lists, returning the number of items synced:

```zig
pub fn syncFromArrayLists(self: *Self, lists: []const std.ArrayListUnmanaged(T)) !usize {
    var total_len: usize = 0;
    for (lists) |list| total_len += list.items.len;

    const req_bytes = total_len * @sizeOf(T);
    const avail_bytes = self.buffer.getProperty(c_ulong, "length");
    if (req_bytes > avail_bytes) {
        self.buffer.msgSend(void, objc.sel("release"), .{});
        const size = req_bytes * 2;
        self.buffer = self.opts.device.msgSend(/* newBufferWithLength:options: */ …);
    }

    const dst = /* self.buffer contents */ ptr[0..req_bytes];

    var i: usize = 0;
    for (lists) |list| {
        const ptr = @as([*]const u8, @ptrCast(list.items.ptr));
        @memcpy(dst[i..][0 .. list.items.len * @sizeOf(T)], ptr);
        i += list.items.len * @sizeOf(T);
    }

    // managed: didModifyRange(0, req_bytes)
    return total_len; // (the count is returned at the end)
}
```

So it sums the per-list lengths, reallocates if the total exceeds the buffer
(doubling), copies each list's bytes contiguously in order, signals
`didModifyRange` for managed storage, and returns the total item count. Empty
lists contribute nothing (a zero-length copy).

## Rust mapping (`roastty/src/renderer/metal/buffer.rs`)

roastty's existing `sync` already adapts upstream's reallocation idiom — it uses
`byte_len::<T>` (overflow-checked), doubles the **item** count, and tracks
`capacity_items` / `capacity_bytes`. `sync_from_array_lists` mirrors `sync`
exactly, but sums the per-list lengths and copies each list at a running byte
offset:

```rust
pub(crate) fn sync_from_array_lists(
    &mut self,
    options: MetalBufferOptions<'_>,
    lists: &[Vec<T>],
) -> Result<usize, MetalBufferError> {
    let total_len: usize = lists.iter().map(Vec::len).sum();
    let required_bytes = byte_len::<T>(total_len)?;
    if required_bytes > self.capacity_bytes {
        let new_capacity_items = total_len
            .checked_mul(2)
            .ok_or(MetalBufferError::ByteLengthOverflow)?;
        let new_capacity_bytes = byte_len::<T>(new_capacity_items)?;
        let new_buffer = options
            .device
            .newBufferWithLength_options(new_capacity_bytes, options.resource_options.to_objc())
            .ok_or(MetalBufferError::BufferCreationFailed)?;
        self.buffer = new_buffer;
        self.resource_options = options.resource_options;
        self.capacity_items = new_capacity_items;
        self.capacity_bytes = new_capacity_bytes;
    }

    if required_bytes > 0 {
        let dst = self.buffer.contents().as_ptr().cast::<u8>();
        let mut offset = 0usize;
        for list in lists {
            if list.is_empty() {
                continue;
            }
            let src = data_as_bytes(list.as_slice());
            unsafe {
                std::ptr::copy_nonoverlapping(src.as_ptr(), dst.add(offset), src.len());
            }
            offset += src.len();
        }
        if requires_did_modify(self.resource_options, required_bytes) {
            self.buffer.didModifyRange(NSRange::new(0, required_bytes));
        }
    }

    Ok(total_len)
}
```

It returns `total_len` (upstream's "number of items synced"). When the total is
zero (all lists empty), `required_bytes` is `0`, the copy and `didModifyRange`
are skipped, and it returns `0` without touching the buffer — the same
empty-data behavior as `sync`.

## Scope / faithfulness notes

- **Ported (bridged)**: `sync_from_array_lists` — the contiguous upload of a
  list of vertex lists into one GPU buffer, returning the total item count. This
  is the foreground-cell upload primitive (`fg_rows.lists` → the cell-text
  buffer).
- **Faithful**: sums the per-list lengths; reallocates (doubling) only when the
  total exceeds the buffer; copies each list's bytes contiguously in list order;
  signals `didModifyRange` once over `[0, required_bytes)` for managed storage;
  returns the total item count; empty lists contribute nothing. Matches
  upstream's `syncFromArrayLists`.
- **Faithful adaptation**: `lists: &[Vec<T>]` is the Rust shape of upstream's
  `[]const ArrayListUnmanaged(T)`. The reallocation reuses roastty's
  overflow-checked `byte_len` and `capacity_items` / `capacity_bytes`
  bookkeeping (exactly as `sync` does) — which already corrects upstream's
  `size * @sizeOf(T)` double-scaling quirk; keeping the two paths identical is
  the faithful choice. The empty-total path returns `0` without touching the
  buffer (mirroring `sync` on empty data).
- **Deferred**: the frame-draw wiring that calls `sync_from_array_lists` on the
  cell-text buffer from `fg_rows.lists` (the live render loop / `drawFrame`);
  the rest of the Metal upload (atlas textures, custom-shader uniforms).
  (Consumed by a later slice; this experiment lands and tests the primitive.)
- No C ABI/header/ABI-inventory change (internal Rust); the Metal buffer module
  is already `#![allow(dead_code)]` ("consumed by later renderer slices").

## Changes

1. `roastty/src/renderer/metal/buffer.rs`:
   - add
     `sync_from_array_lists(&mut self, options, lists: &[Vec<T>]) -> Result<usize, MetalBufferError>`
     mirroring `sync` but summing per-list lengths and copying each list at a
     running byte offset; returns the total item count.
2. Tests (in `buffer.rs`, live Metal device, `u32` element):
   - `sync_from_array_lists` over `[[1, 2], [], [3, 4, 5]]` into a buffer with
     capacity `5` → no reallocation (`capacity_items == 5`), `read_bytes(5)` is
     the contiguous concatenation `[1, 2, 3, 4, 5]`, and the return is `5`
     (proves the interspersed empty list is skipped and the order is preserved);
   - reallocation: a buffer of capacity `1`, lists totaling `5` items →
     `capacity_items == 10`, `capacity_bytes == 40`, `read_bytes(5)` is
     `[4, 5, 6, 7, 8]`, return `5`;
   - all-empty (`[[], []]`) → returns `0` and leaves the capacity unchanged.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty sync_from_array_lists
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `sync_from_array_lists` concatenates the per-list items into the buffer in
  order, reallocates (doubling) only when the total exceeds capacity, signals
  `didModifyRange` for managed storage, and returns the total item count —
  faithful to upstream's `syncFromArrayLists`;
- the tests pass (the contiguous concatenation with an interspersed empty list;
  the reallocation to double the total; the all-empty zero return), and the
  existing buffer tests still pass;
- the frame-draw wiring and the rest of the Metal upload stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the items are concatenated out of order or with
gaps, the reallocation does not double (or drops data), the return count is
wrong, the all-empty case touches the buffer, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the design is faithful to upstream's
`syncFromArrayLists` and consistent with roastty's existing `MetalBuffer::sync`
adaptation: `lists: &[Vec<T>]` matches the current `fg_rows` shape, the total
item count is the correct return value, and the running byte-offset copy
preserves upstream's contiguous row-list concatenation. The reallocation plan
correctly follows roastty's established item-count doubling and checked
`byte_len::<T>` bookkeeping rather than copying upstream's byte-size quirk
literally; the zero-total path, the single `didModifyRange(0, required_bytes)`
for managed non-empty writes, and leaving the buffer intact for all-empty input
all match the existing `sync` semantics. It judged the planned tests sufficient
(ordered concatenation with an empty middle list, reallocation capacity/data/
return, and the all-empty no-op).

Review artifacts:

- Prompt: `logs/codex-review/20260604-070634-d406-prompt.md` (design)
- Result: `logs/codex-review/20260604-070634-d406-last-message.md` (design)

## Result

**Result:** Pass

The foreground-cell buffer upload primitive is now live.

- `roastty/src/renderer/metal/buffer.rs`: a new
  `MetalBuffer::sync_from_array_lists(&mut self, options, lists: &[Vec<T>]) -> Result<usize, MetalBufferError>`
  mirroring `sync` — it sums the per-list item counts, reallocates (doubling the
  item count, via the overflow-checked `byte_len`) only when the total exceeds
  capacity, copies each non-empty list's bytes contiguously at a running byte
  offset, signals `didModifyRange(0, required_bytes)` once for managed non-empty
  writes, and returns the total item count. The all-empty (zero-total) path
  returns `0` without touching the buffer.

Tests (in `buffer.rs`, live Metal device, `u32` element):

- `sync_from_array_lists_concatenates_in_order_skipping_empty` —
  `[[1, 2], [], [3, 4, 5]]` into a capacity-5 buffer → `count == 5`,
  `capacity_items == 5` (no realloc), `read_bytes(5) == [1, 2, 3, 4, 5]` (the
  interspersed empty list is skipped, order preserved).
- `sync_from_array_lists_reallocates_to_double_total` — `init_fill([0])`
  (capacity 1), `[[4, 5], [6], [7, 8]]` → `count == 5`, `capacity_items == 10`,
  `capacity_bytes == 40`, `read_bytes(5) == [4, 5, 6, 7, 8]`.
- `sync_from_array_lists_all_empty_returns_zero_without_realloc` — `[[], []]` →
  `count == 0`, capacity unchanged (`3` items / `12` bytes).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2871 passed, 0 failed (+3, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + `lib.rs`/header/`abi_harness.c`)
  clean; `git diff --check` clean.

## Conclusion

roastty's `MetalBuffer` now has both cell-upload primitives: `sync` (the flat
`bg_cells` background upload) and `sync_from_array_lists` (the `fg_rows.lists`
foreground upload), each faithful to upstream. The next renderer-bridge slice is
the frame-draw wiring that consumes them — calling `sync` on the background
buffer and `sync_from_array_lists` on the cell-text buffer from an assembled
`Contents`, as upstream's `drawFrame` does — which depends on the live frame /
render-pass plumbing; plus the remaining Metal upload (atlas textures,
custom-shader uniforms) and the `rebuild_viewport` cursor/preedit assembly.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation matches the approved design and
is faithful to upstream `syncFromArrayLists`: it sums the per-list item counts,
computes the checked byte length, reallocates only when the required bytes
exceed capacity (doubling by item count in the same style as `sync`), copies
each non-empty list contiguously at a running byte offset, calls
`didModifyRange(0, required_bytes)` only for non-empty managed writes, and
returns the total item count. It judged the offset copy sound under the computed
`required_bytes` (each `src.len()` is one list's byte length, offsets advance by
exactly those lengths, and reallocation guarantees the destination capacity
before copying), and the tests sufficient (ordered concatenation with an empty
middle list, reallocation and capacity bookkeeping, the all-empty no-op).
Internal Rust only, no public C ABI/header impact — nothing needed to change
before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-070835-r406-prompt.md` (result)
- Result: `logs/codex-review/20260604-070835-r406-last-message.md` (result)
