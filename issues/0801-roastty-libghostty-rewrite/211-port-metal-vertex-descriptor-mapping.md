# Experiment 211: Port Metal Vertex Descriptor Mapping

## Description

Experiment 210 added the typed Metal buffer wrapper. The next upstream renderer
dependency is the vertex-descriptor half of
`vendor/ghostty/src/renderer/metal/Pipeline.zig`.

Upstream `Pipeline.zig` has an `autoAttribute(T, attrs)` helper that maps a
shader input struct's fields to Metal vertex descriptor attributes:

- each field gets a Metal vertex format based on its type;
- each field gets `offset = @offsetOf(T, field.name)`;
- each field gets `bufferIndex = 0`;
- the vertex layout gets `stride = @sizeOf(T)`;
- the vertex layout gets the configured step function, usually `.per_vertex`.

Roastty already has `ImageVertex` from Experiment 209 with a stable
upstream-compatible layout. This experiment should port the descriptor mapping
logic as an internal, testable value layer first. It should not create a real
`MTLRenderPipelineDescriptor`, compile shader libraries, or create pipeline
state yet.

All public names must use Roastty naming.

## Changes

1. Extend the Metal API value layer.

   In `roastty/src/renderer/metal/api.rs`, add the small subset of Metal values
   needed by upstream vertex descriptor mapping:

   ```rust
   pub(crate) enum MetalVertexFormat {
       UChar,
       UChar4,
       Char,
       Short2,
       UShort2,
       Float,
       Float2,
       Float4,
       Int,
       Int2,
       UInt,
       UInt2,
       UInt4,
   }

   pub(crate) enum MetalVertexStepFunction {
       PerVertex,
       PerInstance,
   }
   ```

   Use raw values that match Apple Metal / upstream Ghostty's `mtl` values. Add
   tests for every raw value included in the subset. If a value is not needed by
   `ImageVertex` but is included because upstream's mapping supports it, test it
   anyway.

2. Add vertex descriptor value types.

   Add an internal module, for example `roastty/src/renderer/metal/pipeline.rs`,
   and wire it from `roastty/src/renderer/metal/mod.rs`.

   Required internal types:

   ```rust
   pub(crate) struct MetalVertexAttribute {
       pub(crate) format: MetalVertexFormat,
       pub(crate) offset: usize,
       pub(crate) buffer_index: usize,
   }

   pub(crate) struct MetalVertexLayout {
       pub(crate) stride: usize,
       pub(crate) step_function: MetalVertexStepFunction,
   }

   pub(crate) struct MetalVertexDescriptor {
       pub(crate) attributes: Vec<MetalVertexAttribute>,
       pub(crate) layout: MetalVertexLayout,
   }
   ```

   These are value types only. Do not allocate Objective-C `MTLVertexDescriptor`
   objects in this experiment.

3. Add a mapping trait or explicit mapping function.

   Add the smallest Rust mechanism that maps known shader input types to the
   descriptor value types. A simple trait is acceptable:

   ```rust
   pub(crate) trait MetalVertexInput {
       fn vertex_descriptor(step_function: MetalVertexStepFunction) -> MetalVertexDescriptor;
   }
   ```

   Implement it for `ImageVertex`.

   `ImageVertex` must map to:
   - attribute 0: `grid_pos`, `Float2`, offset of `grid_pos`, buffer index 0;
   - attribute 1: `cell_offset`, `Float2`, offset of `cell_offset`, buffer index
     0;
   - attribute 2: `source_rect`, `Float4`, offset of `source_rect`, buffer index
     0;
   - attribute 3: `dest_size`, `Float2`, offset of `dest_size`, buffer index 0;
   - layout stride `size_of::<ImageVertex>()`;
   - caller-provided step function.

   Use `std::mem::offset_of!(ImageVertex, field)` or an equivalent structured
   offset helper for offsets. Do not use unstructured numeric offset literals.

   Do not try to build a general reflection system for arbitrary Rust structs in
   this experiment. Upstream Zig uses comptime reflection, but the Rust port can
   implement explicit mappings one shader input at a time until a real
   abstraction is justified.

4. Add automated tests.

   Add tests that prove:
   - raw `MetalVertexFormat` values match the upstream/Metal values for the
     included subset;
   - raw `MetalVertexStepFunction` values match upstream/Metal values;
   - `ImageVertex::vertex_descriptor(PerVertex)` produces exactly four
     attributes with the expected formats, offsets, and buffer indices;
   - `ImageVertex::vertex_descriptor(PerInstance)` preserves the same attributes
     but changes the layout step function;
   - layout stride equals `size_of::<ImageVertex>()`.

   Tests should be pure Rust. Do not create a Metal device, shader library,
   pipeline descriptor, or pipeline state.

5. Verification commands.

   Run:

   ```bash
   cargo fmt -- roastty/src/renderer/metal/api.rs roastty/src/renderer/metal/mod.rs roastty/src/renderer/metal/pipeline.rs roastty/src/renderer/shader.rs
   cargo test -p roastty renderer::metal::pipeline
   cargo test -p roastty renderer::metal::api
   cargo test -p roastty renderer::shader
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
   git diff --check
   ```

   If `shader.rs` is not edited, it may still be listed in `cargo fmt`;
   `cargo fmt` accepts unchanged files. `cargo fmt` is required for Rust edits;
   accept formatter output as-is.

## Non-Negotiable Invariants

- Do not create Objective-C `MTLVertexDescriptor`,
  `MTLRenderPipelineDescriptor`, or `MTLRenderPipelineState` objects in this
  experiment.
- Do not compile shader libraries or add embedded shader source.
- Do not add render passes, command encoders, command buffers, IOSurface,
  CAMetalLayer, Swift integration, or public C ABI.
- Do not modify image upload, image draw, texture, or buffer semantics from
  Experiments 206-210.
- Do not modify vendored Ghostty source.
- Do not expose `ghostty_*` symbols or comments in the Roastty public ABI.

## Pass Criteria

- Roastty has internal Metal vertex descriptor value types.
- Roastty has an `ImageVertex` descriptor mapping that matches upstream
  `Pipeline.zig` field formats, offsets, buffer index, stride, and step function
  behavior.
- Tests cover raw Metal vertex enum values and the full `ImageVertex` mapping.
- Full verification passes, including the public no-`ghostty` gate.

## Failure Criteria

- The experiment creates real Metal pipeline objects or shader libraries instead
  of staying at the descriptor value layer.
- The mapping ignores field offsets or relies on hard-coded offsets without
  tests.
- The mapping uses a generic reflection abstraction that obscures per-shader
  correctness.
- Existing renderer image, texture, or buffer tests regress.

## Result

**Result:** Pass

Experiment 211 added the internal Metal vertex descriptor value layer. The
implementation:

- added `MetalVertexFormat` and `MetalVertexStepFunction` raw-value enums for
  the subset used by upstream `Pipeline.zig`;
- added tests proving the raw values match upstream/Metal values;
- added internal `renderer::metal::pipeline` value types:
  `MetalVertexAttribute`, `MetalVertexLayout`, and `MetalVertexDescriptor`;
- added the `MetalVertexInput` trait;
- implemented `MetalVertexInput` for `ImageVertex`;
- mapped `ImageVertex` fields with structured `std::mem::offset_of!` offsets,
  buffer index `0`, `Float2`/`Float4` formats, `size_of::<ImageVertex>()`
  stride, and caller-provided step function;
- kept the implementation value-only: no Objective-C `MTLVertexDescriptor`,
  `MTLRenderPipelineDescriptor`, shader library, or pipeline state is created.

Verification passed:

```bash
cargo fmt -- roastty/src/renderer/metal/api.rs roastty/src/renderer/metal/mod.rs roastty/src/renderer/metal/pipeline.rs roastty/src/renderer/shader.rs
cargo test -p roastty renderer::metal::pipeline
cargo test -p roastty renderer::metal::api
cargo test -p roastty renderer::shader
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Observed results:

- `cargo test -p roastty renderer::metal::pipeline`: 2 passed.
- `cargo test -p roastty renderer::metal::api`: 9 passed.
- `cargo test -p roastty renderer::shader`: 1 passed.
- `cargo test -p roastty`: 2140 library tests plus 1 ABI harness test passed.
- The public no-`ghostty` gate and `git diff --check` both exited 0.

Codex result review approved the experiment as Pass. Its only note was to ensure
the new `roastty/src/renderer/metal/pipeline.rs` file is included in the result
commit.

## Conclusion

Roastty now has the value-level vertex descriptor mapping needed before real
Metal pipeline creation. The image shader input layout, field formats, offsets,
buffer index, stride, and step function are all represented and tested without
pulling in shader library or pipeline-state work prematurely.

The next experiment can move from descriptor values toward the real
Objective-C-backed pipeline wrapper, or it can first port the small remaining
pipeline value pieces such as color attachment blend configuration if that keeps
the next slice easier to verify.
