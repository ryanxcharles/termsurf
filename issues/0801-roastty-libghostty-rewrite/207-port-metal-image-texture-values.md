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

# Experiment 207: Port Metal Image Texture Values

## Description

Experiment 206 added an internal image upload/draw contract with a fake-testable
backend. The next step toward a real macOS renderer is to give that contract a
Metal-shaped value layer without taking on Objective-C runtime calls, live Metal
devices, IOSurface targets, shaders, buffers, or Swift/app integration yet.

Upstream's relevant source is:

- `vendor/ghostty/src/renderer/Metal.zig`
- `vendor/ghostty/src/renderer/metal/api.zig`
- `vendor/ghostty/src/renderer/metal/Texture.zig`

This experiment should port the renderer-independent Metal texture value
definitions that are needed for image uploads:

- Metal pixel format constants used by image textures;
- resource option storage/cache modes used by image textures;
- texture usage flags for shader-read images;
- image texture format mapping from logical image formats to Metal pixel
  formats;
- bytes-per-pixel calculation for the pixel formats Roastty will use.

Do not add real Metal object creation yet. In upstream, `Texture.init` creates
an `MTLTextureDescriptor`, asks an `MTLDevice` for a texture, and writes bytes
with `replaceRegion:mipmapLevel:withBytes:bytesPerRow:`. That requires an
Objective-C runtime layer and live Metal device, so it belongs in a later
experiment after the value layer is stable and tested.

All public names must use Roastty naming.

## Changes

1. Add an internal Metal renderer module.

   Add:

   ```text
   roastty/src/renderer/metal/mod.rs
   roastty/src/renderer/metal/api.rs
   roastty/src/renderer/metal/texture.rs
   ```

   Wire it from `roastty/src/renderer/mod.rs` with:

   ```rust
   pub(crate) mod metal;
   ```

   Keep the module internal. Do not add public C ABI.

2. Port the Metal API value definitions needed for image textures.

   In `roastty/src/renderer/metal/api.rs`, add strongly typed Rust
   representations for the upstream values needed by image textures:
   - `MetalPixelFormat`
   - `MetalCpuCacheMode`
   - `MetalStorageMode`
   - `MetalHazardTrackingMode`
   - `MetalResourceOptions`
   - `MetalTextureUsage`

   Include only the values needed by this renderer slice:
   - invalid;
   - `r8unorm`;
   - `r8unorm_srgb`;
   - `rgba8unorm`;
   - `rgba8unorm_srgb`;
   - `bgra8unorm`;
   - `bgra8unorm_srgb`;
   - CPU cache mode default/write-combined;
   - storage mode shared/managed/private/memoryless;
   - hazard tracking default/untracked/tracked;
   - texture usage shader-read.

   Preserve the numeric values from upstream `metal/api.zig` because later
   Objective-C calls must pass the exact Metal constants.

3. Port image texture option mapping.

   In `roastty/src/renderer/metal/texture.rs`, add:
   - `ImageTextureFormat` with `Gray`, `Rgba`, and `Bgra`;
   - `ImageTextureOptions` carrying pixel format, resource options, and usage;
   - a function equivalent to upstream `Metal.imageTextureOptions(format, srgb)`
     that returns shader-read image texture options for a provided storage mode.

   Required mapping:
   - `Gray`, `srgb=false` -> `r8unorm`;
   - `Gray`, `srgb=true` -> `r8unorm_srgb`;
   - `Rgba`, `srgb=false` -> `rgba8unorm`;
   - `Rgba`, `srgb=true` -> `rgba8unorm_srgb`;
   - `Bgra`, `srgb=false` -> `bgra8unorm`;
   - `Bgra`, `srgb=true` -> `bgra8unorm_srgb`.

   Resource options must match upstream image textures:
   - CPU cache mode `write_combined`;
   - caller-provided storage mode;
   - default hazard tracking;
   - usage `shader_read = true`.

4. Port bytes-per-pixel behavior for the supported formats.

   Add a `bytes_per_pixel` method for `MetalPixelFormat` or an equivalent helper
   in `texture.rs`. For this slice:
   - `r8unorm` and `r8unorm_srgb` are 1 byte per pixel;
   - `rgba8unorm`, `rgba8unorm_srgb`, `bgra8unorm`, and `bgra8unorm_srgb` are 4
     bytes per pixel;
   - `invalid` returns an error or `None`, not a panic.

   Do not port the entire upstream `bppOf` table yet. Limiting the table to the
   formats Roastty can produce keeps this experiment focused and avoids dead
   unsupported constants.

5. Connect the value layer to the Experiment 206 upload contract shape.

   Add a small internal helper that maps `crate::renderer::image::PixelFormat`
   upload payloads to `ImageTextureFormat`. Because Experiment 206 prepares all
   uploaded images as RGBA before calling the backend, the required current
   mapping is:
   - `PixelFormat::Rgba` -> `ImageTextureFormat::Rgba`;
   - other `PixelFormat` values return an error or `None`.

   Do not modify `ImageState::upload` yet. This helper is the value-level bridge
   the later real Metal backend will use.

6. Add tests.

   Add Rust tests covering:
   - Metal pixel format numeric values match upstream;
   - resource option raw integer bit packing matches upstream's packed `c_ulong`
     layout exactly:
     - `write_combined/shared/default == 0x1`;
     - managed storage shifts into bits 4-7 as `0x10`;
     - private storage shifts into bits 4-7 as `0x20`;
     - memoryless storage shifts into bits 4-7 as `0x30`;
     - untracked hazard mode shifts into bits 8-9 as `0x100`;
     - tracked hazard mode shifts into bits 8-9 as `0x200`;
   - texture usage raw integer bit packing matches upstream exactly:
     - shader-read-only usage is `0x1`;
     - render-target usage, if represented for tests, is `0x4`;
   - image texture format to pixel format mapping for all Gray/RGBA/BGRA + sRGB
     combinations;
   - image texture options use write-combined CPU cache mode, caller-provided
     storage mode, default hazard tracking, and shader-read usage;
   - bytes-per-pixel results for all supported formats;
   - invalid pixel format returns no byte size instead of panicking;
   - `PixelFormat::Rgba` maps to `ImageTextureFormat::Rgba`;
   - non-RGBA renderer image pixel formats are rejected by the bridge because
     Experiment 206 converts upload payloads to RGBA first.

7. Verification commands.

   Run:

   ```bash
   cargo fmt -- roastty/src/renderer/mod.rs roastty/src/renderer/metal/mod.rs roastty/src/renderer/metal/api.rs roastty/src/renderer/metal/texture.rs
   cargo test -p roastty renderer::metal
   cargo test -p roastty renderer::image
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
   git diff --check
   ```

   `cargo fmt` is required for Rust edits. Accept formatter output as-is.

## Non-Negotiable Invariants

- Do not expose `ghostty_*` symbols or comments in the Roastty public ABI.
- Do not modify vendored Ghostty source.
- Do not add Objective-C runtime calls, Metal device creation, texture
  descriptor allocation, IOSurface targets, shaders, buffers, Swift app, or C
  ABI behavior in this experiment.
- Do not add a new dependency unless implementation proves a value-only module
  cannot be represented without it. If a dependency becomes necessary, stop and
  redesign before adding it.
- Do not change the Experiment 206 image upload/draw state machine.
- Keep all new Metal code internal to Roastty.

## Pass Criteria

- Roastty has an internal Metal value module with the image texture constants
  and option mapping needed by a future real Metal backend.
- Numeric Metal constants used by image textures match upstream.
- Image texture option construction is covered by tests and matches upstream
  intent.
- Bytes-per-pixel behavior is covered for every supported image texture format.
- The bridge from renderer image pixel format to Metal image texture format is
  explicit and rejects non-RGBA upload payloads.
- Existing renderer image tests and the full Roastty suite continue to pass.
- Full verification passes, including the public no-`ghostty` gate.

## Failure Criteria

- The experiment adds live Metal/Objective-C calls or depends on a real GPU.
- The experiment adds public C ABI.
- Metal numeric constants are guessed instead of copied from upstream
  `metal/api.zig`.
- Unsupported pixel formats panic in normal helper paths.
- The Experiment 206 upload/draw state machine regresses.

## Result

**Result:** Pass

Roastty now has an internal `renderer::metal` value module for the image texture
constants and option mapping needed by a future real Metal backend. The
implementation added:

- `renderer::metal::api` for the supported Metal pixel format constants,
  resource option bit packing, storage/cache/hazard modes, texture usage, and
  supported bytes-per-pixel behavior;
- `renderer::metal::texture` for image texture format mapping, image texture
  option construction, and the bridge from prepared renderer image pixel format
  to Metal image texture format;
- scoped `dead_code` allowance on the internal Metal module because this
  foundation is intentionally consumed by later renderer slices.

The implementation stays value-only. It adds no Objective-C runtime calls, no
live Metal device or texture descriptor allocation, no IOSurface target, no
shader/buffer work, no dependencies, no C ABI, and no changes to Experiment
206's upload/draw state machine.

Tests cover:

- Metal pixel format raw values copied from upstream;
- `MetalResourceOptions` raw bit packing for CPU cache, storage mode, and hazard
  tracking;
- `MetalTextureUsage` raw bit packing for shader-read and render-target flags;
- image texture format mapping for Gray/RGBA/BGRA with and without sRGB;
- image texture option construction with write-combined CPU cache,
  caller-provided storage mode, default hazard tracking, and shader-read usage;
- bytes-per-pixel results for supported image texture formats;
- invalid pixel format returning no byte size instead of panicking;
- the upload bridge accepting only already-prepared RGBA payloads.

Codex reviewed the implementation and approved it with no blockers. The only
process note was to ensure the new `roastty/src/renderer/metal/` files are
included in the result commit.

Verification passed:

```bash
cargo fmt -- roastty/src/renderer/mod.rs roastty/src/renderer/metal/mod.rs roastty/src/renderer/metal/api.rs roastty/src/renderer/metal/texture.rs
cargo test -p roastty renderer::metal
cargo test -p roastty renderer::image
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

## Conclusion

Experiment 207 establishes the Metal image texture value layer without taking on
runtime Metal integration. The next experiment can now build on known-good
constants and option mapping, likely by adding the Objective-C/Metal dependency
decision and the smallest real texture wrapper that can implement Experiment
206's `ImageUploadBackend` for RGBA image uploads.
