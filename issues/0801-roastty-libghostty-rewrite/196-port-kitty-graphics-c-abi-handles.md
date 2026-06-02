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

# Experiment 196: Port Kitty Graphics C ABI Handles

## Description

Experiment 195 completed direct terminal execution for Kitty graphics transmit,
display, delete, transmit-display, and cursor-after behavior. The remaining
problem is that app/renderer code still cannot inspect the active screen's Kitty
graphics state through the public `libroastty` C ABI. `roastty_terminal_get`
currently returns `ROASTTY_NO_VALUE` for `ROASTTY_TERMINAL_DATA_KITTY_GRAPHICS`,
even though upstream Ghostty exposes a dedicated `terminal/c/kitty_graphics.zig`
API for image lookup and placement iteration.

This experiment ports the first renderer-facing Kitty graphics ABI slice:

- expose the active screen's Kitty graphics storage handle through
  `roastty_terminal_get(..., ROASTTY_TERMINAL_DATA_KITTY_GRAPHICS, ...)`;
- expose image handles and image metadata/data getters;
- expose placement iterator allocation, free, reset-from-storage, layer filter,
  next, placement data getters, and multi-get helpers.

This experiment deliberately does not port the geometry/render-info functions
yet. Upstream's placement rectangle, pixel size, grid size, viewport position,
source rectangle, and render-info helpers depend on viewport-relative placement
math and terminal sizing semantics. Those are the next coherent ABI slice after
handle/iterator access exists.

Use upstream Ghostty source as the behavior reference:

- `vendor/ghostty/src/terminal/c/kitty_graphics.zig`
- `vendor/ghostty/src/terminal/c/terminal.zig`
- `vendor/ghostty/src/terminal/c/main.zig`

All public names must be Roastty names. The upstream `GhosttyKittyGraphics*`
types and functions become `roastty_kitty_graphics_*` names and
`RoasttyKittyGraphics*` internal Rust structs/types where needed.

## Changes

1. Add public handle and enum declarations in `roastty/include/roastty.h`.

   Add opaque handle typedefs:

   ```c
   typedef void* roastty_kitty_graphics_t;
   typedef void* roastty_kitty_graphics_image_t;
   typedef void* roastty_kitty_graphics_placement_iterator_t;
   ```

   `roastty_kitty_graphics_t` is a borrowed active-screen storage handle. It is
   only valid while the terminal is alive and must not be stored by callers past
   terminal teardown.

   `roastty_kitty_graphics_image_t` is an owned image snapshot handle. This
   deliberately differs from upstream's borrowed image pointer so Roastty does
   not expose invalidatable Rust `HashMap` references across C calls. The handle
   owns a copy of the image metadata and payload bytes and remains valid until
   `roastty_kitty_graphics_image_free(...)`.

   Add Roastty-named enums mirroring upstream numeric values:
   - `roastty_kitty_graphics_data_e`
     - `ROASTTY_KITTY_GRAPHICS_DATA_INVALID = 0`
     - `ROASTTY_KITTY_GRAPHICS_DATA_PLACEMENT_ITERATOR = 1`
   - `roastty_kitty_graphics_placement_data_e`
     - `INVALID = 0`
     - `IMAGE_ID = 1`
     - `PLACEMENT_ID = 2`
     - `IS_VIRTUAL = 3`
     - `X_OFFSET = 4`
     - `Y_OFFSET = 5`
     - `SOURCE_X = 6`
     - `SOURCE_Y = 7`
     - `SOURCE_WIDTH = 8`
     - `SOURCE_HEIGHT = 9`
     - `COLUMNS = 10`
     - `ROWS = 11`
     - `Z = 12`
   - `roastty_kitty_placement_layer_e`
     - `ALL = 0`
     - `BELOW_BG = 1`
     - `BELOW_TEXT = 2`
     - `ABOVE_TEXT = 3`
   - `roastty_kitty_graphics_placement_iterator_option_e`
     - `LAYER = 0`
   - `roastty_kitty_image_format_e`
     - `ROASTTY_KITTY_IMAGE_FORMAT_RGB = 0`
     - `ROASTTY_KITTY_IMAGE_FORMAT_RGBA = 1`
     - `ROASTTY_KITTY_IMAGE_FORMAT_PNG = 2`
     - `ROASTTY_KITTY_IMAGE_FORMAT_GRAY_ALPHA = 3`
     - `ROASTTY_KITTY_IMAGE_FORMAT_GRAY = 4`
   - `roastty_kitty_image_compression_e`
     - `ROASTTY_KITTY_IMAGE_COMPRESSION_NONE = 0`
     - `ROASTTY_KITTY_IMAGE_COMPRESSION_ZLIB_DEFLATE = 1`
   - `roastty_kitty_graphics_image_data_e`
     - `INVALID = 0`
     - `ID = 1`
     - `NUMBER = 2`
     - `WIDTH = 3`
     - `HEIGHT = 4`
     - `FORMAT = 5`
     - `COMPRESSION = 6`
     - `DATA_PTR = 7`
     - `DATA_LEN = 8`

2. Add public C ABI functions in `roastty/include/roastty.h` and
   `roastty/src/lib.rs`.

   Port this first upstream function set with Roastty names:

   ```c
   roastty_result_e roastty_kitty_graphics_get(
       roastty_kitty_graphics_t,
       roastty_kitty_graphics_data_e,
       void*);

   roastty_kitty_graphics_image_t
   roastty_kitty_graphics_image(roastty_kitty_graphics_t, uint32_t image_id);

   void roastty_kitty_graphics_image_free(roastty_kitty_graphics_image_t);

   roastty_result_e roastty_kitty_graphics_image_get(
       roastty_kitty_graphics_image_t,
       roastty_kitty_graphics_image_data_e,
       void*);

   roastty_result_e roastty_kitty_graphics_image_get_multi(
       roastty_kitty_graphics_image_t,
       size_t count,
       const roastty_kitty_graphics_image_data_e* keys,
       void** values,
       size_t* out_written);

   roastty_result_e roastty_kitty_graphics_placement_iterator_new(
       roastty_kitty_graphics_placement_iterator_t* out);

   void roastty_kitty_graphics_placement_iterator_free(
       roastty_kitty_graphics_placement_iterator_t);

   roastty_result_e roastty_kitty_graphics_placement_iterator_set(
       roastty_kitty_graphics_placement_iterator_t,
       roastty_kitty_graphics_placement_iterator_option_e,
       const void*);

   bool roastty_kitty_graphics_placement_next(
       roastty_kitty_graphics_placement_iterator_t);

   roastty_result_e roastty_kitty_graphics_placement_get(
       roastty_kitty_graphics_placement_iterator_t,
       roastty_kitty_graphics_placement_data_e,
       void*);

   roastty_result_e roastty_kitty_graphics_placement_get_multi(
       roastty_kitty_graphics_placement_iterator_t,
       size_t count,
       const roastty_kitty_graphics_placement_data_e* keys,
       void** values,
       size_t* out_written);
   ```

   Use existing `ROASTTY_SUCCESS`, `ROASTTY_NO_VALUE`, `ROASTTY_INVALID_VALUE`,
   and `ROASTTY_OUT_OF_MEMORY` result codes.

3. Wire `ROASTTY_TERMINAL_DATA_KITTY_GRAPHICS`.

   Update `roastty_terminal_get` so `ROASTTY_TERMINAL_DATA_KITTY_GRAPHICS`
   writes a handle to the active screen's Kitty `ImageStorage` and returns
   `ROASTTY_SUCCESS`.

   Preserve the existing behavior for the deferred medium/limit values:
   `ROASTTY_TERMINAL_DATA_KITTY_IMAGE_STORAGE_LIMIT`,
   `ROASTTY_TERMINAL_DATA_KITTY_IMAGE_MEDIUM_FILE`,
   `ROASTTY_TERMINAL_DATA_KITTY_IMAGE_MEDIUM_TEMP_FILE`, and
   `ROASTTY_TERMINAL_DATA_KITTY_IMAGE_MEDIUM_SHARED_MEM` remain
   `ROASTTY_NO_VALUE` in this experiment.

4. Add internal ABI support types in `roastty/src/lib.rs`.

   Add a heap-owned image snapshot handle containing a copy of:
   - image ID;
   - image number;
   - width and height;
   - format and compression;
   - image bytes.

   `roastty_kitty_graphics_image(...)` returns null for missing images or
   allocation failure. `DATA_PTR` points into this owned snapshot and remains
   valid until `roastty_kitty_graphics_image_free(...)`. A later terminal Kitty
   mutation must not invalidate an existing image snapshot.

   Add a heap-owned placement iterator wrapper containing:
   - the current placement entries as owned `(PlacementKey, Placement)` snapshot
     values;
   - the current selected entry index;
   - the active layer filter.

   Do not store only placement keys. `roastty_kitty_graphics_placement_get(...)`
   receives only the iterator handle, not the graphics storage handle, and must
   continue returning the selected snapshot's values even if storage mutates
   later. A key-only list would either require unsafe storage re-lookup or would
   stop being a snapshot.

   `roastty_kitty_graphics_get(...PLACEMENT_ITERATOR...)` resets an already
   allocated iterator from the supplied graphics handle, preserving the
   iterator's allocator/ownership and current layer filter, matching upstream's
   "initialize this iterator from this storage" behavior.

5. Implement placement layer filtering.

   Match upstream layer semantics:
   - `ALL` includes every placement.
   - `BELOW_BG` includes `z < i32::MIN / 2`.
   - `BELOW_TEXT` includes `i32::MIN / 2 <= z < 0`.
   - `ABOVE_TEXT` includes `z >= 0`.

6. Add ABI tests in `roastty/src/lib.rs` and `roastty/tests/abi_harness.c`.

   Rust tests should cover:
   - public enum numeric values;
   - `roastty_terminal_get(...KITTY_GRAPHICS...)` succeeds and returns a
     non-null handle for a live terminal;
   - invalid/null handle validation;
   - image handle lookup returns null for missing image and non-null for stored
     image;
   - image getters return ID, number, dimensions, format, compression, data
     pointer, and data length;
   - image snapshots remain valid after later terminal Kitty graphics mutation;
   - image free accepts null and releases owned snapshot handles;
   - image multi-get stops at the first failure and reports `out_written`;
   - placement iterator allocation/free;
   - iterator reset through `roastty_kitty_graphics_get`;
   - placement `next` and placement getters for image ID, external placement ID,
     virtual status, offsets, source rectangle, columns, rows, and z;
   - internal placement IDs expose upstream-compatible `0` placement IDs where
     the public ID is absent;
   - layer filtering for below-bg, below-text, and above-text;
   - placement multi-get stops at the first failure and reports `out_written`;
   - iterator snapshots remain memory-safe if the terminal receives a later
     Kitty graphics mutation after the iterator is reset.

   The C ABI harness should cover:
   - typedef/function visibility from `roastty/include/roastty.h`;
   - enum numeric values;
   - a minimal terminal write that stores one image and one placement, then
     reads it through the C ABI.

7. Preserve public naming and formatting rules.

   Run:

   ```bash
   cargo fmt -- roastty/src/lib.rs roastty/src/terminal/kitty/graphics_storage.rs
   prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/196-port-kitty-graphics-c-abi-handles.md
   ```

   `graphics_storage.rs` only needs formatting if the implementation adds public
   crate-visible snapshot helpers there.

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/kitty/graphics_storage.rs
cargo test -p roastty kitty_graphics_c_abi
cargo test -p roastty terminal_get_abi
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when:

- `ROASTTY_TERMINAL_DATA_KITTY_GRAPHICS` returns a usable active-screen graphics
  handle;
- image handle/data getters expose owned image snapshots whose data pointers
  remain valid until image free;
- placement iterators enumerate the active placement state and layer filters
  match upstream;
- multi-get helpers validate inputs and report partial progress;
- iterator handles are memory-safe across terminal mutations because they do not
  hold borrowed Rust `HashMap` iterators across C calls;
- the C harness proves the public header exports the new ABI;
- all existing Kitty terminal execution tests still pass;
- the public no-`ghostty` naming gate passes.

## Non-Negotiable Invariants

- Do not expose any `ghostty_*` ABI names.
- Do not return borrowed image handles. Image handles must be owned snapshots
  and must have an explicit free function.
- Do not implement placement geometry/render-info helpers in this experiment:
  `placement_rect`, `placement_pixel_size`, `placement_grid_size`,
  `placement_viewport_pos`, `placement_source_rect`, and `placement_render_info`
  remain deferred.
- Do not implement Metal rendering.
- Do not render images.
- Do not decode PNG.
- Do not add non-direct image media support.
- Do not add animation execution.
- Do not add Unicode virtual placement rendering.
- Do not change Kitty transmit/display/delete/cursor-after execution semantics
  except as needed to expose existing state through the ABI.
- Do not return live Rust `HashMap` iterators or references that can be
  invalidated across C calls. Placement iterators must own
  `(PlacementKey, Placement)` snapshots.
- Do not weaken existing C ABI validation behavior for null handles, null output
  pointers, invalid selector values, or multi-get partial progress.
- Do not skip Codex design review. If the design review finds a real issue, fix
  it and re-review before committing this experiment design.
- Do not skip Codex result review after implementation.

## Result

**Result:** Pass

Experiment 196 implemented the first renderer-facing Kitty graphics C ABI slice.
`ROASTTY_TERMINAL_DATA_KITTY_GRAPHICS` now returns the active screen's Kitty
graphics storage handle, image handles are owned snapshots with
`roastty_kitty_graphics_image_free`, and placement iterators own snapshot
entries rather than borrowed Rust map iterators.

The implementation exposes:

- image snapshot lookup by image ID;
- image getters for ID, number, width, height, format, compression, data
  pointer, and data length;
- placement iterator allocation, reset from storage, layer filtering, next,
  single-get, multi-get, and free;
- placement getters for image ID, public placement ID, virtual flag, offsets,
  source rectangle, columns, rows, and z;
- stable enum values in both Rust tests and the C ABI harness.

Codex result review initially found missing verification for offset/source
placement fields, all layer filters, internal `p=0` placement IDs, and placement
multi-get partial progress. Those gaps were fixed, the suite was rerun, and
Codex re-reviewed the updated diff with no blocking findings.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs
cargo test -p roastty kitty_graphics_c_abi
cargo test -p roastty terminal_get_abi
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty --test abi_harness
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

## Conclusion

The C ABI can now see the Kitty graphics state produced by the terminal parser
and execution layer without exposing live image references or live map iterators
across the ABI boundary. The next experiment can build on this by exposing the
geometry/render-info helper slice that renderer code needs to turn placement
metadata into concrete draw commands.
