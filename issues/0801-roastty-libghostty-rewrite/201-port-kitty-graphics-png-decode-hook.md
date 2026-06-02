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

# Experiment 201: Port Kitty Graphics PNG Decode Hook

## Description

Experiment 200 completed Kitty graphics loading for non-PNG media: direct data,
regular files, temporary files, and POSIX shared memory. The next loading-layer
gap is PNG.

Ghostty's library build does not own a built-in PNG decoder. Instead, the
terminal package exposes a runtime sys hook:

- `vendor/ghostty/src/terminal/sys.zig`
- `vendor/ghostty/src/terminal/c/sys.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_image.zig`

Roastty already exposes the corresponding public C ABI shape:

- `ROASTTY_SYS_OPT_DECODE_PNG`
- `roastty_sys_decode_png_fn`
- `roastty_sys_image_s`
- `roastty_alloc`
- `roastty_free`

This experiment wires that existing sys callback into Kitty graphics image
completion. It does not add a PNG decoder crate, rendering, Metal, image
placement drawing, or app UI. The embedder remains responsible for installing a
decoder through `roastty_sys_set(ROASTTY_SYS_OPT_DECODE_PNG, ...)`.

## Changes

1. Add crate-private sys helper functions in `roastty/src/lib.rs`.

   Add a small internal bridge that Kitty graphics can call without exposing new
   public ABI:
   - `pub(crate) fn sys_has_decode_png() -> bool`
   - `pub(crate) fn sys_decode_png(data: &[u8]) -> Result<DecodedPng, ImageLoadError>`

   `DecodedPng` should contain:
   - decoded width;
   - decoded height;
   - owned RGBA bytes.

   The helper must:
   - copy the current `userdata` and callback out of `SYS_STATE` under the
     mutex;
   - return `UnsupportedFormat` when no callback is installed;
   - pass an allocator pointer to the callback;
   - require callback success;
   - reject a null output data pointer regardless of `data_len`, matching
     Ghostty's C sys wrapper;
   - reject `data_len > MAX_IMAGE_SIZE` before copying callback-owned bytes into
     a Rust `Vec`;
   - copy the callback-owned bytes into a Rust `Vec<u8>`;
   - free callback-owned bytes with the same allocator after copying;
   - free callback-owned bytes on malformed-output and oversized-output failure
     paths after the callback has returned ownership;
   - map allocation failure to `OutOfMemory`;
   - map callback failure or malformed callback output to `InvalidData`.

   Prefer a small static Roastty allocator vtable that backs
   `roastty_alloc`/`roastty_free` semantics, rather than passing a null
   allocator pointer. The callback should be able to allocate through the
   allocator pointer it receives, matching Ghostty's C wrapper pattern.

2. Decode PNG in `roastty/src/terminal/kitty/graphics_image.rs`.

   Update `LoadingImage::complete()` to match upstream order:
   - decompress first;
   - if `self.image.format == TransmissionFormat::Png`, call the sys PNG decode
     helper;
   - reject decoded data larger than `MAX_IMAGE_SIZE`;
   - replace `self.data` with decoded RGBA bytes;
   - update `self.image.width` and `self.image.height` from the decoder result;
   - set `self.image.format = TransmissionFormat::Rgba` so existing byte-length
     validation uses 4 bytes per pixel;
   - then run the existing dimension and data-length validation.

   Direct PNG without an installed decoder must still fail with
   `UnsupportedFormat`.

3. Allow non-direct PNG only when a decoder is installed.

   Update the file, temporary-file, and shared-memory PNG short-circuit from
   Experiments 199-200:
   - if PNG decode callback is not installed, non-direct PNG still returns
     `UnsupportedMedium` before opening files or shared-memory objects;
   - if the callback is installed, non-direct PNG may load bytes through the
     selected medium and decode during completion.

   This matches upstream's "save buffering work when PNG decoding is
   unavailable" behavior.

4. Preserve non-PNG media behavior.

   Do not change raw RGB/RGBA/gray/gray-alpha direct, file, temporary-file, or
   shared-memory loading semantics except where common helper code is needed for
   PNG.

5. Add tests.

   Add image-loader tests for:
   - direct PNG without a callback still returns `UnsupportedFormat`;
   - direct PNG with a callback decodes to RGBA, updates dimensions, and stores
     decoded bytes;
   - callback failure maps to `InvalidData`;
   - callback output with null data and zero length maps to `InvalidData`;
   - callback output with null data and nonzero length maps to `InvalidData`;
   - callback output exceeding `MAX_IMAGE_SIZE` maps to `InvalidData` before
     Roastty attempts to allocate or copy that output;
   - decoded dimensions of zero still fail the existing dimension validation;
   - decoded byte length that does not match width × height × 4 fails
     `InvalidData`;
   - file, temporary-file, and shared-memory PNG remain `UnsupportedMedium`
     without a callback and do not touch OS resources;
   - file, temporary-file, and shared-memory PNG load and decode when the
     callback is installed.

   Add terminal-stream coverage proving a PNG transmit can pass through parser,
   image execution, active-screen storage, and image snapshot inspection when a
   decode callback is installed.

   Because the sys decode callback is global process state, tests that install
   or clear it must use a test-only lock/guard that saves the prior state,
   installs the test callback, runs the assertion, and restores the prior state.
   Any existing tests that assert PNG is unsupported must either avoid global
   sys state or run under the same guard with the callback explicitly cleared.

6. Preserve formatting and review rules.

   Run:

   ```bash
   cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/kitty/graphics_image.rs roastty/src/terminal/kitty/graphics_exec.rs
   prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/201-port-kitty-graphics-png-decode-hook.md
   ```

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/kitty/graphics_image.rs roastty/src/terminal/kitty/graphics_exec.rs
cargo test -p roastty kitty_graphics_image
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty sys_c_abi
cargo test -p roastty kitty_graphics_c_abi
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when:

- PNG remains unsupported when no decode callback is installed;
- direct PNG decodes through the installed callback and stores as RGBA;
- file, temporary-file, and shared-memory PNG remain blocked before OS access
  when no callback is installed;
- file, temporary-file, and shared-memory PNG load through their existing media
  paths and decode when a callback is installed;
- malformed decoder output and oversized decoded output fail without panics or
  leaks;
- non-PNG Kitty graphics behavior from Experiments 187-200 remains unchanged;
- Codex approves the experiment design before implementation and approves the
  result before the experiment is recorded.

## Non-Negotiable Invariants

- Do not add a built-in PNG decoder dependency in this experiment.
- Do not render images.
- Do not add Metal or any platform renderer.
- Do not change the public sys callback ABI shape unless implementation proves
  the existing shape is unusable; if so, stop and redesign before editing the
  ABI.
- Do not leave callback-owned decoded buffers unfreed.
- Do not allow global sys-state test races.
- Do not weaken Experiment 199's file/temp-file cleanup guarantees.
- Do not weaken Experiment 200's shared-memory cleanup guarantees.
- Do not expose any `ghostty_*` ABI names.
- Do not skip Codex design review or Codex result review.

## Result

**Result:** Pass

Implemented the Kitty graphics PNG decode hook through the existing Roastty sys
callback ABI. `LoadingImage::complete()` now decompresses first, decodes PNG
through `ROASTTY_SYS_OPT_DECODE_PNG` when installed, normalizes decoded PNGs to
RGBA, updates image dimensions from the callback result, and then runs the
existing dimension and byte-length validation.

The sys bridge in `roastty/src/lib.rs` now:

- passes a real Roastty allocator pointer to the callback;
- maps missing callbacks to `UnsupportedFormat`;
- maps callback failure and malformed callback output to `InvalidData`;
- rejects null decoded data unconditionally;
- rejects oversized decoded output before copying callback-owned bytes;
- copies callback-owned bytes into Rust-owned memory;
- frees callback-owned bytes with the same allocator after success and on
  oversized/allocation-failure paths.

Non-direct PNG now follows upstream's capability gate:

- without a decoder callback, file, temporary-file, and shared-memory PNG still
  return `UnsupportedMedium` before touching OS resources;
- with a decoder callback, the existing file, temporary-file, and shared-memory
  loaders read the PNG bytes and decode during completion.

Added test coverage for:

- direct PNG unsupported behavior without a callback;
- direct PNG decode through the sys callback;
- callback failure;
- null output with zero and nonzero lengths;
- oversized callback output using a small test-only limit so the pre-copy guard
  is exercised without allocating hundreds of megabytes;
- decoded zero dimensions and decoded byte-length mismatch;
- non-direct PNG blocked before OS access without a callback;
- file, temporary-file, and shared-memory PNG decode with a callback;
- terminal-stream PNG storage through parser/executor/active-screen state.

Because the sys callback table is global process state, PNG tests and the
existing sys callback ABI test now use `SYS_TEST_LOCK` to serialize callback
mutation and avoid parallel test races.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/kitty/graphics_image.rs roastty/src/terminal/kitty/graphics_exec.rs
cargo test -p roastty kitty_graphics_image
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty sys_c_abi
cargo test -p roastty kitty_graphics_c_abi
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Codex result review found no blocking issues and approved recording Experiment
201 as Pass.

## Conclusion

Roastty's Kitty graphics loading layer now supports PNG as an embedder-supplied
decode capability rather than a built-in dependency. Direct, file,
temporary-file, and shared-memory PNG transmissions can all reach image storage
when the sys callback is installed, while no-decoder behavior remains
predictably unsupported before unnecessary OS work.

The remaining Kitty graphics work can now move beyond image loading into the
next missing subsystem slice, such as animation/frame behavior or renderer-side
image presentation.
