# Experiment 210: Port Metal Buffer Wrapper

## Description

Experiment 209 added the backend-agnostic image draw command contract. Upstream
`vendor/ghostty/src/renderer/image.zig` turns each image placement into one
`GraphicsAPI.shaders.Image` value, then calls
`GraphicsAPI.Buffer(GraphicsAPI.shaders.Image).initFill(...)` before adding the
render-pass step.

The next coherent renderer slice is therefore the Metal buffer wrapper from
`vendor/ghostty/src/renderer/metal/buffer.zig`.

Upstream's generic Metal buffer wrapper provides:

- `init(opts, len)`: allocate an `MTLBuffer` with capacity for `len` typed
  items;
- `initFill(opts, data)`: allocate and initialize from typed data;
- `sync(data)`: copy replacement bytes into the existing buffer, reallocating to
  a larger buffer when the new data no longer fits;
- managed-storage synchronization through `didModifyRange` after CPU writes.

Roastty should port this as an internal typed buffer wrapper using
`objc2`/`objc2-metal`. The experiment should prove the wrapper with live
headless Metal tests and byte readback from `MTLBuffer::contents`.

This experiment must not add render passes, command encoders, pipelines,
shaders, IOSurface, CAMetalLayer, Swift integration, or public C ABI.

All public names must use Roastty naming.

## Changes

1. Add the narrow Metal buffer dependencies.

   Update `roastty/Cargo.toml` only as needed:
   - add `objc2-metal` feature `MTLBuffer`;
   - add `objc2-foundation` only if needed for `NSRange::new` in the
     `didModifyRange` path, using the narrowest feature set that compiles.

   Do not enable `objc2-metal`'s broad default feature set.

2. Add a typed Metal buffer wrapper.

   Add `roastty/src/renderer/metal/buffer.rs` and wire it from
   `roastty/src/renderer/metal/mod.rs`.

   Required internal types:

   ```rust
   pub(crate) struct MetalBufferOptions<'a> {
       pub(crate) device: &'a objc2::runtime::ProtocolObject<dyn objc2_metal::MTLDevice>,
       pub(crate) resource_options: MetalResourceOptions,
   }

   pub(crate) struct MetalBuffer<T> {
       buffer: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_metal::MTLBuffer>>,
       resource_options: MetalResourceOptions,
       capacity_items: usize,
       capacity_bytes: usize,
       _marker: std::marker::PhantomData<T>,
   }
   ```

   Add a small internal unsafe marker trait for buffer element types:

   ```rust
   pub(crate) unsafe trait MetalBufferElement: Copy {}
   ```

   Safety contract: implementors must have a stable initialized byte
   representation suitable for direct GPU upload. `T: Copy` alone is not enough
   because some `Copy` types may contain padding bytes. Implement the trait only
   for types this experiment needs:
   - `ImageVertex`;
   - test primitives such as `u32`.

   Do not expose this trait publicly.

3. Implement allocation and fill.

   Implement:

   ```rust
   impl<T: MetalBufferElement> MetalBuffer<T> {
       pub(crate) fn new(options: MetalBufferOptions<'_>, len: usize) -> Result<Self, MetalBufferError>;
       pub(crate) fn init_fill(options: MetalBufferOptions<'_>, data: &[T]) -> Result<Self, MetalBufferError>;
       pub(crate) fn sync(&mut self, options: MetalBufferOptions<'_>, data: &[T]) -> Result<(), MetalBufferError>;
       pub(crate) fn capacity_items(&self) -> usize;
       pub(crate) fn capacity_bytes(&self) -> usize;
   }
   ```

   Behavior:
   - byte length is `len * size_of::<T>()` with checked arithmetic;
   - `new` calls `newBufferWithLength:options:`;
   - `init_fill` calls `newBufferWithBytes:length:options:`;
   - both return a testable error if Metal returns no buffer;
   - zero-length buffers are allowed only if Metal accepts them in the live
     test; otherwise record the actual behavior and reject zero-length buffers
     explicitly with a testable error.

4. Implement sync and reallocation.

   `sync` should mirror upstream:
   - compute required byte length for `data`;
   - if required bytes exceed `capacity_bytes`, allocate a replacement buffer
     with capacity for double the required item count;
   - copy the new bytes into `buffer.contents()`;
   - preserve upstream's capacity-oriented length semantics: the wrapper tracks
     allocated item capacity, not logical synced item count;
   - shorter syncs do not shrink `capacity_items` or `capacity_bytes`, and bytes
     past the newly-written data are left untouched;
   - larger syncs update `capacity_items` and `capacity_bytes` to the new
     allocated capacity;
   - if reallocation fails, return an error and leave the existing buffer,
     `capacity_items`, and `capacity_bytes` unchanged;
   - call `didModifyRange(NSRange::new(0, required_bytes))` when
     `resource_options.storage_mode == MetalStorageMode::Managed`.

   Upstream's reallocation code appears to double `req_bytes` and then multiply
   by `@sizeOf(T)` again when calling Metal. Roastty should follow the intended
   behavior, not the apparent double-multiply artifact: allocate capacity for
   `data.len() * 2` items, with byte length `capacity_items * size_of::<T>()`.

   For live tests, use `MetalStorageMode::Shared` so contents can be read back
   without command-buffer synchronization. Add a small unit test for the managed
   branch's decision logic if the direct `didModifyRange` side effect cannot be
   inspected.

5. Add readback-only test helpers.

   Add `#[cfg(test)]` helpers on `MetalBuffer<T>` to read the initialized bytes
   or typed values from `contents`. These helpers are for verification only; do
   not expose them as public API.

6. Add automated tests.

   Add tests that do not require GUI permissions:
   - live `MetalBuffer<ImageVertex>::init_fill` creates a one-item buffer and
     reads back the exact `ImageVertex` bytes or field values;
   - live `MetalBuffer<u32>::new` creates capacity for N values and records
     `capacity_items` and `capacity_bytes` correctly;
   - `sync` with data that fits updates bytes without reallocating;
   - shorter `sync` leaves trailing capacity and trailing bytes intact;
   - larger `sync` reallocates to at least double the required byte capacity and
     reads back the new data;
   - failed reallocation preserves the previous buffer and capacity if this can
     be simulated with a fake allocator/backend hook; if not, document why the
     live Metal path cannot force this failure deterministically;
   - byte-length overflow returns a testable error;
   - `ImageVertex` from Experiment 209 can be uploaded through the new buffer
     wrapper.

   Keep tests headless. Do not create render passes, command buffers, windows,
   or drawable targets.

7. Verification commands.

   Run:

   ```bash
   cargo fmt -- roastty/src/renderer/metal/buffer.rs roastty/src/renderer/metal/mod.rs roastty/src/renderer/metal/api.rs roastty/src/renderer/shader.rs
   cargo test -p roastty renderer::metal::buffer
   cargo test -p roastty renderer::image
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
   git diff --check
   ```

   If `api.rs` or `shader.rs` is not edited, it may still be listed in
   `cargo fmt`; `cargo fmt` accepts unchanged files. `cargo fmt` is required for
   Rust edits; accept formatter output as-is.

## Non-Negotiable Invariants

- Do not expose public C ABI for Metal buffers in this experiment.
- Do not add render passes, command encoders, pipelines, shader compilation,
  IOSurface, CAMetalLayer, Swift integration, or app-window behavior.
- Do not modify image upload or image draw semantics from Experiments 206-209.
- Do not modify vendored Ghostty source.
- Do not expose `ghostty_*` symbols or comments in the Roastty public ABI.
- Keep the wrapper internal to Roastty.

## Pass Criteria

- Roastty has an internal typed Metal buffer wrapper matching upstream
  allocation, fill, and sync behavior.
- Live headless tests prove `MTLBuffer` allocation, initial fill, contents
  readback, in-place sync, and reallocation.
- `ImageVertex` can be uploaded through the buffer wrapper.
- Full verification passes, including the public no-`ghostty` gate.

## Failure Criteria

- The experiment only adds compile-only wrappers without proving live buffer
  allocation and readback.
- The wrapper silently overflows byte-length calculations.
- `sync` changes upstream semantics for shorter updates or failed reallocations.
- The experiment drifts into render-pass, pipeline, command-buffer, or public
  ABI work.

## Result

**Result:** Pass

Experiment 210 added the internal typed Metal buffer wrapper. The
implementation:

- added the narrow `objc2-metal` `MTLBuffer` feature and direct
  `objc2-foundation` `NSRange` dependency;
- added `renderer::metal::buffer`;
- added `MetalBufferOptions`, `MetalBuffer<T>`, `MetalBufferError`, and the
  internal unsafe `MetalBufferElement` marker trait;
- implemented `MetalBufferElement` for `ImageVertex`;
- implemented `new`, `init_fill`, `sync`, `capacity_items`, and
  `capacity_bytes`;
- used checked byte-length arithmetic and explicit errors for overflow,
  zero-sized element types, zero-length buffers, and Metal buffer creation
  failure;
- preserved capacity-oriented buffer semantics: shorter syncs do not shrink
  capacity and leave trailing bytes untouched;
- implemented safe reallocation ordering, allocating the replacement before
  replacing the old buffer so failed reallocation leaves the old buffer intact;
- added a testable `requires_did_modify` predicate for managed-storage sync
  decisions.

The first implementation tried to allow zero-length buffers, but the live Metal
test proved `newBufferWithLength:options:` returns no buffer for length 0 on the
test machine. Per the experiment design, the implementation now rejects
zero-length buffers explicitly with `MetalBufferError::ZeroLengthBuffer`.

Verification passed:

```bash
cargo fmt -- roastty/src/renderer/metal/buffer.rs roastty/src/renderer/metal/mod.rs roastty/src/renderer/metal/api.rs roastty/src/renderer/shader.rs
cargo test -p roastty renderer::metal::buffer
cargo test -p roastty renderer::image
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Observed results:

- `cargo test -p roastty renderer::metal::buffer`: 9 passed.
- `cargo test -p roastty renderer::image`: 30 passed.
- `cargo test -p roastty`: 2136 library tests plus 1 ABI harness test passed.
- The public no-`ghostty` gate and `git diff --check` both exited 0.

Codex result review initially found two blocking test gaps: zero-length buffer
behavior was allowed but unproven, and the managed-storage `didModifyRange`
branch was untested. Both were fixed before recording the result. The follow-up
Codex result review approved the experiment as Pass.

## Conclusion

Roastty now has the Metal storage primitive needed by the image draw path:
stable-layout `ImageVertex` values can be uploaded into live `MTLBuffer`
objects, synchronized in place, reallocated when needed, and read back in
headless tests.

The next experiment can use this buffer wrapper to move toward real Metal render
steps: either port the render-pass step value/binding layer or the minimal image
draw backend that turns Experiment 209 `ImageDrawCall`s into one-item
`MetalBuffer<ImageVertex>` instances for later encoder submission.
