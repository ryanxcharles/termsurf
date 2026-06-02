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

# Experiment 215: Port Standard Metal Shader Library

## Description

Experiment 214 proved that Roastty can convert tested Metal pipeline values into
real Objective-C descriptors and create live `MTLRenderPipelineState` objects
from a small test-only shader source.

The next upstream layer is the standard Metal shader library from
`vendor/ghostty/src/renderer/metal/shaders.zig` and
`vendor/ghostty/src/renderer/shaders/shaders.metal`. Upstream embeds a compiled
`ghostty_metallib` and loads it with `newLibraryWithData:error:`. Roastty does
not yet have a Metal build-artifact pipeline, so this experiment should port the
standard shader source as Roastty source and compile it with
`newLibraryWithSource_options_error(...)`.

This is intentionally the standard shader-library slice only. It should build
the five standard pipelines from `STANDARD_PIPELINE_DESCRIPTIONS` using the real
production shader source. It should not add custom postprocess shaders,
Shadertoy conversion, render passes, command encoders, frames, CAMetalLayer, or
a compiled `.metallib` build step.

All public names must use Roastty naming. The production shader source may be a
faithful adaptation of the upstream shader source, but any app-facing comments,
file names, Rust symbols, and module names in Roastty must say Roastty unless
they are explicitly citing the original project in an issue document.

## Changes

1. Add the adapted production Metal shader source.

   Add `roastty/src/renderer/metal/shaders.metal` as a faithful Roastty
   adaptation of `vendor/ghostty/src/renderer/shaders/shaders.metal`.

   Requirements:
   - preserve the shader function names used by
     `STANDARD_PIPELINE_DESCRIPTIONS`:
     - `full_screen_vertex`;
     - `bg_color_fragment`;
     - `cell_bg_fragment`;
     - `cell_text_vertex`;
     - `cell_text_fragment`;
     - `image_vertex`;
     - `image_fragment`;
     - `bg_image_vertex`;
     - `bg_image_fragment`;
   - preserve the shader input/output struct layouts and uniform semantics;
   - remove or adapt app-facing upstream naming in comments so Roastty source
     does not casually refer to itself by the old name;
   - do not modify the vendored upstream source.

   This file is source material for runtime compilation in this experiment. Do
   not add a compiled `.metallib` artifact yet.

2. Add a standard shader-library wrapper.

   Add `roastty/src/renderer/metal/shaders.rs` and export it from
   `roastty/src/renderer/metal/mod.rs`.

   The module should contain:

   ```rust
   pub(crate) const STANDARD_METAL_SHADER_SOURCE: &str = include_str!("shaders.metal");

   pub(crate) struct MetalShaderLibrary {
       library: objc2::rc::Retained<
           objc2::runtime::ProtocolObject<dyn objc2_metal::MTLLibrary>,
       >,
   }

   pub(crate) enum MetalShaderLibraryError {
       CompileFailed(String),
   }

   impl MetalShaderLibrary {
       pub(crate) fn compile(
           device: &objc2::runtime::ProtocolObject<dyn objc2_metal::MTLDevice>,
       ) -> Result<Self, MetalShaderLibraryError>;

       pub(crate) fn library(
           &self,
       ) -> &objc2::runtime::ProtocolObject<dyn objc2_metal::MTLLibrary>;
   }
   ```

   Behavior:
   - add a private helper that compiles arbitrary source:

     ```rust
     fn compile_source(
         device: &objc2::runtime::ProtocolObject<dyn objc2_metal::MTLDevice>,
         source: &str,
     ) -> Result<
         objc2::rc::Retained<
             objc2::runtime::ProtocolObject<dyn objc2_metal::MTLLibrary>,
         >,
         MetalShaderLibraryError,
     >
     ```

   - have `MetalShaderLibrary::compile(...)` call `compile_source(...)` with
     `STANDARD_METAL_SHADER_SOURCE`;
   - compile source with `device.newLibraryWithSource_options_error(...)`;
   - return a retained library on success;
   - return `CompileFailed(...)` with the Metal compiler's error string on
     failure;
   - do not panic on compiler errors.

3. Add a standard pipeline collection.

   In `roastty/src/renderer/metal/shaders.rs`, add:

   ```rust
   pub(crate) struct MetalStandardPipelines {
       pub(crate) bg_color: MetalPipeline,
       pub(crate) cell_bg: MetalPipeline,
       pub(crate) cell_text: MetalPipeline,
       pub(crate) image: MetalPipeline,
       pub(crate) bg_image: MetalPipeline,
   }

   pub(crate) enum MetalStandardPipelinesError {
       ShaderLibrary(MetalShaderLibraryError),
       MissingStandardPipeline(&'static str),
       Pipeline {
           name: &'static str,
           error: MetalPipelineError,
       },
   }

   impl MetalStandardPipelines {
       pub(crate) fn new(
           device: &objc2::runtime::ProtocolObject<dyn objc2_metal::MTLDevice>,
           pixel_format: MetalPixelFormat,
       ) -> Result<Self, MetalStandardPipelinesError>;
   }
   ```

   Behavior:
   - compile the standard shader library once;
   - build every entry in `STANDARD_PIPELINE_DESCRIPTIONS`;
   - use the same library for vertex and fragment functions;
   - route each pipeline through `standard_pipeline_build_values(...)` and
     `MetalPipeline::new(...)`;
   - preserve every pipeline name in errors so failures are diagnosable;
   - keep the collection strongly typed with one field per standard pipeline,
     matching upstream's named pipeline collection shape.

   Add a private/testable helper that builds the collection from an already
   compiled library:

   ```rust
   fn build_from_library(
       device: &objc2::runtime::ProtocolObject<dyn objc2_metal::MTLDevice>,
       library: &objc2::runtime::ProtocolObject<dyn objc2_metal::MTLLibrary>,
       pixel_format: MetalPixelFormat,
   ) -> Result<MetalStandardPipelines, MetalStandardPipelinesError>
   ```

   `MetalStandardPipelines::new(...)` should compile the standard library and
   then call `build_from_library(...)`. Tests can use the helper with a
   deliberately incompatible library to prove named pipeline failures. Do not
   rely on `MetalPixelFormat::Invalid` for this path; Experiment 214 showed
   Metal may accept that value on this device.

   If the implementation needs a helper to look up descriptions by name, add it
   in `pipeline.rs` or `shaders.rs`, but do not change the existing descriptor
   table semantics.

4. Add live Metal tests for the production standard shader source.

   Add tests in `roastty/src/renderer/metal/shaders.rs`.

   Required tests:
   - `STANDARD_METAL_SHADER_SOURCE` contains every function name required by
     `STANDARD_PIPELINE_DESCRIPTIONS`;
   - the standard shader library compiles on the system default Metal device;
   - every required shader function resolves from the compiled production
     library;
   - `MetalStandardPipelines::new(...)` creates all five standard pipelines for
     a BGRA8 sRGB pixel format;
   - an intentionally invalid shader source returns `CompileFailed(...)` with a
     non-empty message through `compile_source(...)`, proving the same helper
     used by the production compile path;
   - a deliberately incompatible test library passed to
     `build_from_library(...)` reports the failing standard pipeline name
     instead of a nameless `MetalPipelineError`. The incompatible library should
     resolve the relevant function names but make at least one
     descriptor/function interface mismatch deterministic.

   Do not weaken these to Rust-only tests. If Metal shader compilation fails,
   record the exact compiler error in the experiment result.

   This experiment proves source compilation, function resolution, and pipeline
   compatibility. It does not prove draw-time shader semantics, visual output,
   color correctness, glyph correctness, or texture sampling correctness. Those
   remain for later render-pass/readback experiments.

5. Verification commands.

   Run:

   ```bash
   cargo fmt -- roastty/src/renderer/metal/mod.rs roastty/src/renderer/metal/shaders.rs
   cargo test -p roastty renderer::metal::shaders
   cargo test -p roastty renderer::metal::pipeline
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/metal; then exit 1; else exit 0; fi
   git diff --check
   ```

   `cargo fmt` is required for Rust edits; accept formatter output as-is.

## Non-Negotiable Invariants

- Do not add render passes, command encoders, command buffers, frames,
  IOSurface, CAMetalLayer, Swift integration, or public C ABI.
- Do not add custom postprocess shader support.
- Do not add Shadertoy conversion.
- Do not add a compiled `.metallib` build step in this experiment.
- Do not modify vendored Ghostty source.
- Do not change shader payload, vertex descriptor, attachment, standard table,
  or pipeline-builder semantics except where needed to wire the standard shader
  library into the already-tested builder.
- Do not introduce public or app-facing `ghostty_*` names in Roastty.

## Pass Criteria

- Roastty contains an adapted production Metal shader source file.
- Roastty can compile the adapted production shader source into a real
  `MTLLibrary`.
- Every standard pipeline shader function resolves from that real production
  library.
- Roastty can create all five standard `MTLRenderPipelineState` objects from
  `STANDARD_PIPELINE_DESCRIPTIONS`.
- Compile and pipeline failures are explicit, named, and non-panicking.
- Full verification passes, including the public no-`ghostty` gate and the
  renderer Metal no-`ghostty` gate.

## Failure Criteria

- The experiment keeps using the small test-only shader source from Experiment
  214 for standard pipeline verification.
- The experiment only proves library compilation and does not build all five
  standard pipelines.
- Pipeline failures do not identify the failing standard pipeline by name.
- The experiment grows into render passes, command encoders, custom shaders, or
  `.metallib` build artifacts.
- Existing Metal pipeline, descriptor, texture, buffer, image, or full Roastty
  tests regress.

## Result

**Result:** Pass

Experiment 215 ported the standard Metal shader-library layer for Roastty.

The implementation added:

- `roastty/src/renderer/metal/shaders.metal`, a faithful Roastty adaptation of
  the upstream production Metal shader source;
- `roastty/src/renderer/metal/shaders.rs`;
- `STANDARD_METAL_SHADER_SOURCE`;
- `MetalShaderLibrary`, which compiles the production source into a retained
  `MTLLibrary`;
- a private `compile_source(...)` helper used by both production compilation and
  invalid-source testing;
- `MetalStandardPipelines`, a strongly typed collection for the five standard
  pipelines;
- a private `build_from_library(...)` helper used by both production standard
  pipeline creation and deterministic named-failure testing.

The live tests prove:

- the production source contains every shader function required by
  `STANDARD_PIPELINE_DESCRIPTIONS`;
- the production source compiles with the system default Metal device;
- every required vertex and fragment function resolves from the compiled
  production library;
- all five standard pipelines create live `MTLRenderPipelineState` objects with
  a BGRA8 sRGB pixel format;
- invalid shader source returns `CompileFailed(...)` with a non-empty message;
- a deliberately incompatible compiled library returns a named standard-pipeline
  error before the underlying `MetalPipelineError`.

Verification passed:

```bash
cargo fmt -- roastty/src/renderer/metal/mod.rs roastty/src/renderer/metal/shaders.rs
cargo test -p roastty renderer::metal::shaders
cargo test -p roastty renderer::metal::pipeline
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/metal; then exit 1; else exit 0; fi
git diff --check
```

Observed test results:

- `cargo test -p roastty renderer::metal::shaders`: 6 passed, 0 failed;
- `cargo test -p roastty renderer::metal::pipeline`: 17 passed, 0 failed;
- `cargo test -p roastty`: 2176 library tests passed, 1 ABI harness test passed,
  0 doc tests.

Codex reviewed the implementation result and reported no blocking findings. The
review explicitly approved recording Experiment 215 as Pass.

## Conclusion

Roastty now has the production standard Metal shader source wired into a real
runtime `MTLLibrary`, and every standard pipeline description can create a live
`MTLRenderPipelineState` from that library.

This proves source compilation, function resolution, and pipeline compatibility.
It still does not prove draw-time shader semantics, visual output, color
correctness, glyph correctness, or texture sampling correctness. The next
renderer slice should move from pipeline creation into render-pass or command
encoding work with read-back tests, so the standard pipelines are exercised by
actual draw commands rather than only pipeline construction.
