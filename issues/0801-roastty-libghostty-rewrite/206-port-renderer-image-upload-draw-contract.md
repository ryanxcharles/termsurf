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

# Experiment 206: Port Renderer Image Upload and Draw Contract

## Description

Experiment 205 added the CPU-side renderer image-state foundation: image ids,
pending image data, replacement/unload intent, Kitty placement accumulation, and
z-layer split indices. Upstream's next renderer-independent boundary in
`vendor/ghostty/src/renderer/image.zig` is the public image state upload/draw
contract:

- `State.upload`
- `State.draw`
- `DrawPlacements`
- `Image.ready`
- `Image.replace`
- `Image.unload_ready`
- `Image.unload_replace`
- `Image.hasTexture`
- `Image.upload`
- `Image.convert`
- `Image.prepForUpload`

Do not add real Metal, shader, buffer, Swift, or app integration yet. Roastty
does not have the renderer API or texture types needed for concrete Metal image
rendering, and forcing those into this experiment would make correctness depend
on GPU/device state instead of unit tests.

This experiment should instead add a small internal renderer image backend
contract that mirrors the shape upstream expects: image state can upload pending
image bytes into an opaque texture type, remove unloading images, and emit draw
requests for the selected placement bucket. Tests should use a fake backend so
state transitions, upload failure behavior, RGBA conversion, placement bucket
selection, and draw skipping can be verified without macOS GUI permissions or a
Metal device.

All public names must use Roastty naming.

## Changes

1. Extend `roastty/src/renderer/image.rs` image states.

   Make `RendererImage` generic over an internal texture type, with a default
   texture type that keeps existing tests lightweight. Add states matching the
   upstream CPU/GPU boundary:
   - `Pending(PendingImage)`
   - `Replace { texture, pending }`
   - `Ready(texture)`
   - `UnloadPending(PendingImage)`
   - `UnloadReady(texture)`
   - `UnloadReplace { texture, pending }`

   Update `ImageState` to be generic over the same texture type. Existing
   Experiment 205 tests should continue to work with the default texture type.

2. Port renderer image helper behavior.

   Add or update methods equivalent to the upstream helper surface:
   - `RendererImage::is_pending`
   - `RendererImage::has_texture`
   - `RendererImage::is_unloading`
   - `RendererImage::pending_image`
   - `RendererImage::mark_for_unload`
   - `RendererImage::mark_for_replace`

   `mark_for_replace` must preserve an existing ready texture by moving into
   `Replace { texture, pending }`; if no texture exists, it becomes `Pending`.
   `mark_for_unload` must preserve whether the image is pending, ready, or
   replace so `upload()` can remove it consistently.

   Preserve Experiment 205's reappearing-image fix after texture-bearing states
   exist. If an uploaded image disappears from a frame, it should become
   `UnloadReady(texture)`. If the same unchanged image reappears before
   `upload()` removes unloading images, the unload mark must be canceled and the
   image must remain `Ready(texture)`, not become pending and not remain
   unloading. If the same image id reappears with changed dimensions, format, or
   bytes while in `UnloadReady(texture)`, the state must become
   `Replace { texture, pending }` so the old texture is retained for safe
   replacement/retry until a replacement upload succeeds. This preserves
   upstream state semantics; `draw()` still skips `Replace` images because
   upstream only draws `Ready` and `UnloadReady`.

3. Add CPU-side RGBA upload preparation.

   Port the renderer-independent portion of upstream `Image.convert` /
   `prepForUpload`:
   - grayscale becomes RGBA with duplicated gray channels and alpha 255;
   - grayscale-alpha becomes RGBA with duplicated gray channels and original
     alpha;
   - RGB becomes RGBA with alpha 255;
   - RGBA remains unchanged;
   - unsupported or length-mismatched pending data returns a testable error and
     leaves image state coherent.

   Do not add PNG decode here. Kitty PNG decode is already earlier in the
   terminal/Kitty pipeline.

4. Add an internal upload backend contract.

   Add a renderer-internal trait or equivalent abstraction, for example:

   ```rust
   pub(crate) trait ImageUploadBackend {
       type Texture;
       type Error;

       fn upload_image(&mut self, pending: &PendingImage) -> Result<Self::Texture, Self::Error>;
   }
   ```

   Add
   `ImageState::upload(&mut self, backend: &mut impl ImageUploadBackend) -> bool`
   matching upstream behavior:
   - `ImageState<T>::upload` only accepts upload backends whose associated
     `Texture` type is exactly `T`; do not introduce ad hoc texture conversion
     or wrapper paths in this experiment;
   - unloading images are removed from the image map;
   - pending images are prepared as RGBA and uploaded;
   - successful pending uploads become `Ready(texture)`;
   - successful replace uploads drop/supersede the previous texture and become
     `Ready(new_texture)`;
   - failed uploads return `false` overall but leave the image pending/replacing
     so a later upload can retry;
   - mixed success/failure returns `false` while still applying successful
     uploads.

5. Add an internal draw backend contract.

   Add `DrawPlacements` with upstream-equivalent buckets:
   - `KittyBelowBackground`
   - `KittyBelowText`
   - `KittyAboveText`
   - `Overlay`

   Add a draw request value carrying the ready texture reference and placement
   data. Add a renderer-internal draw backend trait or equivalent abstraction,
   for example:

   ```rust
   pub(crate) trait ImageDrawBackend<Texture> {
       type Error;

       fn draw_image(&mut self, texture: &Texture, placement: Placement) -> Result<(), Self::Error>;
   }
   ```

   Add `ImageState::draw(...)` matching upstream behavior:
   - select the placement slice by `DrawPlacements` and the already-computed
     `kitty_bg_end` / `kitty_text_end` split indices;
   - ignore placements whose image id is missing;
   - ignore placements whose image is not ready;
   - call the draw backend for ready images;
   - ignore individual draw errors and continue drawing later placements;
   - return a small testable summary, such as attempted/succeeded/skipped
     counts, so tests can prove behavior without reading logs.

   Add an `overlay_placements` vector if needed to preserve the upstream draw
   bucket shape, but do not implement overlay update or overlay rendering yet.

6. Add tests.

   Add Rust tests covering:
   - RGBA preparation for grayscale, grayscale-alpha, RGB, and no-op RGBA;
   - length mismatch during preparation reports failure and preserves retryable
     state;
   - `mark_for_replace` keeps an existing texture and pending replacement;
   - `mark_for_unload` preserves pending/ready/replace unload state;
   - `Ready` image absent from a frame becomes `UnloadReady`;
   - unchanged image reappearing from `UnloadReady` cancels unload and remains
     drawable with the existing texture;
   - changed image reappearing from `UnloadReady` becomes
     `Replace { texture, pending }`;
   - `upload()` removes unloading images;
   - successful pending upload becomes ready;
   - successful replacement upload becomes ready with the new texture;
   - failed upload returns `false` and leaves a pending image retryable;
   - failed replacement upload returns `false` and leaves both the old texture
     and pending replacement intact for retry;
   - mixed upload success/failure returns `false` but keeps successful uploads;
   - draw bucket selection uses `kitty_bg_end` and `kitty_text_end` correctly;
   - draw skips missing and non-ready images;
   - draw ignores backend errors and continues;
   - overlay draw bucket is present and empty/no-op until overlay update is
     ported.

   Use a fake upload/draw backend. Do not require a Metal device.

7. Verification commands.

   Run:

   ```bash
   cargo fmt -- roastty/src/lib.rs roastty/src/renderer/mod.rs roastty/src/renderer/image.rs
   cargo test -p roastty renderer::image
   cargo test -p roastty render_state
   cargo test -p roastty kitty_graphics_render_placement_c_abi
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
   git diff --check
   ```

   `cargo fmt` is required for Rust edits. Accept formatter output as-is.

## Non-Negotiable Invariants

- Do not expose `ghostty_*` symbols or comments in the Roastty public ABI.
- Do not modify vendored Ghostty source.
- Do not add real Metal, shader, buffer, Swift app, or C ABI behavior in this
  experiment.
- Do not add runtime dependencies on macOS GUI permissions, Screen Recording,
  Accessibility, a real Metal device, or app runtime state.
- Do not re-read terminal state from renderer image state.
- Do not recompute Kitty placement geometry.
- Keep backend contracts internal to Roastty unless a later experiment proves an
  ABI consumer needs them.

## Pass Criteria

- Renderer image state supports pending, ready, replace, and unload variants in
  a way that can later hold real Metal textures.
- Upload behavior is testable through a fake backend and matches upstream state
  transition semantics.
- Pending Kitty image data is prepared as RGBA before upload.
- Draw behavior is testable through a fake backend and uses the Kitty layer
  split indices from Experiment 205.
- Missing/non-ready images and draw backend errors do not abort drawing later
  placements.
- Existing Experiment 205 renderer image behavior, render-state behavior, and
  Kitty render placement behavior continue to pass.
- Full verification passes, including the public no-`ghostty` gate.

## Failure Criteria

- The experiment jumps directly into real Metal or Swift/app integration.
- Upload/draw behavior can only be verified visually or with a real GPU.
- Failed uploads destroy pending image data and prevent retry.
- Replacement uploads discard the old texture before a successful replacement is
  available.
- Draw bucket selection ignores `kitty_bg_end` / `kitty_text_end`.
- Existing renderer image, render-state, or Kitty render placement tests
  regress.

## Result

**Result:** Pass

Roastty now has an internal renderer image upload/draw contract layered on top
of the CPU image-state foundation from Experiment 205. `RendererImage` now
supports pending, ready, replace, and unload states with an internal generic
texture type, and `ImageState` can upload pending/replacement images through a
fake-testable backend contract. Unloading images are removed during upload,
successful uploads become ready textures, failed uploads leave retryable state
intact, and replacement failures preserve both the old texture and the pending
replacement.

The implementation also ports the renderer-independent RGBA preparation from
upstream `renderer/image.zig`: grayscale, grayscale-alpha, and RGB pending
images are converted to RGBA upload payloads, while stored source identity
remains in the original render-state format. That source-preservation detail
matters because render-state snapshots can keep arriving as RGB/gray even after
the renderer uploads an RGBA texture; a fresh identical frame must not look like
a changed image.

`ImageState::draw` now supports the upstream draw buckets:

- Kitty below background;
- Kitty below text;
- Kitty above text;
- overlay.

The draw path uses the existing `kitty_bg_end` and `kitty_text_end` split
indices, skips missing or non-ready images, ignores backend draw errors, and
returns a small testable summary so behavior is verifiable without logs or a
real Metal device. The overlay bucket exists but remains empty until overlay
update is ported in a later experiment.

Tests cover:

- RGBA preparation for grayscale, grayscale-alpha, RGB, and RGBA;
- length-mismatch preparation errors;
- replacement and unload state preservation;
- `Ready -> UnloadReady -> Ready` reappearance;
- `Ready -> UnloadReady -> Replace` changed reappearance;
- `Replace -> UnloadReplace -> Replace` reappearance;
- pending upload success/failure;
- replacement upload success/failure;
- mixed upload success/failure;
- draw bucket selection;
- missing and non-ready draw skips;
- draw error continuation;
- empty overlay draw bucket behavior.

Codex result review found two real bugs in the first implementation:

- uploaded non-RGBA images stored converted RGBA as their source identity, so a
  later identical RGB/gray snapshot would be treated as changed;
- `UnloadReplace` reappearance dropped the retained texture.

Both were fixed. Upload now converts a copy while preserving the original source
identity, and `UnloadReplace` reappearance restores
`Replace { texture, pending }`. Codex re-reviewed the corrected result and
approved it with no blockers.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/renderer/mod.rs roastty/src/renderer/image.rs
cargo test -p roastty renderer::image
cargo test -p roastty render_state
cargo test -p roastty kitty_graphics_render_placement_c_abi
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

## Conclusion

Experiment 206 ports the testable upload/draw state-machine boundary without
taking on real Metal. The renderer image state can now preserve images across
frames, prepare upload payloads, retain old textures during replacement, remove
unloading images, and emit draw calls by layer bucket through an internal
backend contract. The next experiment should continue toward the concrete macOS
renderer side, likely by porting the Metal texture/value definitions or the
smallest Metal-facing backend adapter that can satisfy this internal contract
without exposing public C ABI prematurely.
