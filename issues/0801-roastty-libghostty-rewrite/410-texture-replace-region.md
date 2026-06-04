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

# Experiment 410: the texture region upload (replace_region)

## Description

`drawFrame`'s per-frame sync has three parts: the uniforms, the cells (done —
`FrameCells`, Experiment 408), and the **font atlas textures**. The atlas sync
(`syncAtlasTexture`) re-uploads the atlas pixels into the GPU texture each time
the atlas changes, reallocating the texture only when the atlas outgrows it. Its
core GPU operation is a texture **region replace** — writing a rectangle of
pixels into an existing texture. roastty's `MetalTexture` performs this
internally on creation (in `new`, over the full region) but exposes no method to
re-upload afterward. This experiment adds that primitive,
`MetalTexture::replace_region`, so the atlas sync (a later slice) can refresh
the texture; the grow-and-reallocate wrapper and the font `Atlas` type are
deferred.

## Upstream behavior

In `syncAtlasTexture` (`renderer/generic.zig`), after a possible reallocation,
the atlas pixels are uploaded with a full-region replace:

```zig
try texture.replaceRegion(0, 0, atlas.size, atlas.size, atlas.data);
```

`Texture.replaceRegion(x, y, width, height, data)` maps to Metal's
`replaceRegion:mipmapLevel:withBytes:bytesPerRow:` — it writes `data` into the
texture's `[x, x+width) × [y, y+height)` region, with
`bytesPerRow = width × bytes_per_pixel`. The atlas always replaces the whole
square (`0, 0, size, size`), but the underlying operation is a general
rectangular region write.

## Rust mapping (`roastty/src/renderer/metal/texture.rs`)

`MetalTexture::new` already issues this call for the initial full-region upload:

```rust
texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
    full_region(width, height), 0, bytes, width * bytes_per_pixel,
);
```

`replace_region` exposes the same operation for an arbitrary sub-region, with
bounds and length validation:

```rust
pub(crate) fn replace_region(
    &self,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    data: &[u8],
) -> Result<(), MetalTextureError> {
    // The region must fit inside the texture. Subtraction-based to avoid the
    // overflow an `x + width` check would risk.
    if x > self.width
        || width > self.width - x
        || y > self.height
        || height > self.height - y
    {
        return Err(MetalTextureError::RegionOutOfBounds {
            x, y, width, height,
            texture_width: self.width,
            texture_height: self.height,
        });
    }
    // The data must be exactly `width × height × bytes_per_pixel`.
    let expected = texture_byte_len(width, height, self.bytes_per_pixel)?;
    if data.len() != expected {
        return Err(MetalTextureError::ByteLengthMismatch { expected, actual: data.len() });
    }
    if !data.is_empty() {
        let region = MTLRegion {
            origin: MTLOrigin { x, y, z: 0 },
            size: MTLSize { width, height, depth: 1 },
        };
        let bytes = NonNull::new(data.as_ptr().cast_mut().cast()).expect("non-empty data");
        unsafe {
            self.texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                region, 0, bytes, width * self.bytes_per_pixel,
            );
        }
    }
    Ok(())
}
```

`bytes_per_row` is the **source** stride, `width × bytes_per_pixel`, matching
`new`'s full-region upload. A new `MetalTextureError::RegionOutOfBounds` variant
reports a region that exceeds the texture. The full-region case
(`replace_region(0, 0, w, h, data)`) is exactly upstream's
`replaceRegion(0, 0, size, size, data)`.

## Scope / faithfulness notes

- **Ported (bridged)**: `MetalTexture::replace_region` — the texture region
  upload primitive (`replaceRegion:mipmapLevel:withBytes:bytesPerRow:`), the
  core GPU operation behind upstream's `syncAtlasTexture`.
- **Faithful**: writes `data` into the `[x, x+width) × [y, y+height)` region
  with `bytesPerRow = width × bytes_per_pixel`, the same call `new` issues for
  the initial upload; the full-region form matches upstream's
  `replaceRegion(0, 0, size, size, data)`.
- **Faithful adaptation**: roastty validates the region (in bounds) and the data
  length (`width × height × bytes_per_pixel`) before the unsafe write —
  defensive bookkeeping roastty already does in `new` (the byte-length check)
  and `read_bytes`; upstream relies on the caller. The method generalizes
  upstream's always-full square to an arbitrary sub-region (the underlying Metal
  operation), which subsumes the atlas case.
- **Deferred**: `syncAtlasTexture` itself (the grow-and-reallocate wrapper that
  calls `replace_region` after reallocating when the atlas outgrows the
  texture), the font `Atlas` type it reads from, and the per-frame atlas sync
  call site. (Consumed by a later slice; this experiment lands and tests the
  primitive.)
- No C ABI/header/ABI-inventory change (internal Rust); the Metal texture module
  is part of the renderer layer consumed by later slices.

## Changes

1. `roastty/src/renderer/metal/texture.rs`:
   - add a
     `MetalTextureError::RegionOutOfBounds { x, y, width, height, texture_width, texture_height }`
     variant;
   - add
     `MetalTexture::replace_region(&self, x, y, width, height, data) -> Result<(), MetalTextureError>`
     with the bounds + length validation and the region write described above.
2. Tests (in `texture.rs`, live Metal device, grayscale `Gray` format — 1
   byte/pixel):
   - a **full-region** replace: a 2×2 texture initialized to `[1, 2, 3, 4]`,
     `replace_region(0, 0, 2, 2, &[10, 20, 30, 40])` → `read_bytes()` is
     `[10, 20, 30, 40]`;
   - a **sub-region** replace: a 4×4 texture initialized to zero,
     `replace_region(1, 1, 2, 2, &[1, 2, 3, 4])` → `read_bytes()` has the block
     at the `(1, 1)` origin and zero elsewhere
     (`[0,0,0,0, 0,1,2,0, 0,3,4,0, 0,0,0,0]`), proving the x/y offset and the
     source `bytesPerRow`;
   - **validation**: a wrong-length `data` → `ByteLengthMismatch`; an
     out-of-bounds region (e.g. `replace_region(3, 3, 2, 2, …)` on a 4×4) →
     `RegionOutOfBounds`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty replace_region
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `replace_region` writes the data into the given region with the correct source
  `bytesPerRow`, validates the bounds and data length, and the full-region form
  matches upstream's `replaceRegion(0, 0, size, size, data)`;
- the tests pass (the full-region replace; the sub-region offset; the length and
  bounds validation), and the existing texture tests still pass;
- `syncAtlasTexture`, the font `Atlas` type, and the per-frame atlas sync stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if `replace_region` writes the wrong region or stride,
skips the validation, the sub-region offset is wrong, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it after one
**Required** finding was addressed:

- **Required (addressed):** the bounds check must not use unchecked `x + width`
  / `y + height` — the addition can overflow before the comparison (panicking in
  debug, wrapping in release and wrongly accepting an out-of-bounds region,
  which is exactly what the check guards against). The design now uses the
  subtraction-based form
  `x > self.width || width > self.width - x || y > self.height || height > self.height - y`,
  which cannot overflow.

Codex confirmed the rest is sound: `bytesPerRow = width * bytes_per_pixel` is
the correct **source** stride for tightly packed sub-region data (not
`self.width * bpp`); the full-region form matches upstream's atlas upload; the
defensive exact-length validation is reasonable; the planned sub-region test has
the correct row-major expected layout; and generalizing beyond the full-square
atlas use is acceptable for this GPU primitive.

Review artifacts:

- Prompt: `logs/codex-review/20260604-073624-d410-prompt.md` (design)
- Result: `logs/codex-review/20260604-073624-d410-last-message.md` (design)
