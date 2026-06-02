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

# Experiment 188: Port Kitty Image Storage Foundation

## Description

Experiment 187 added direct Kitty image loading. The next useful Kitty graphics
layer is the image-storage foundation from:

- `vendor/ghostty/src/terminal/kitty/graphics_storage.zig`

The full upstream storage module also owns placements, grid pins, deletion by
geometry, and renderer dirty signaling. Those parts require terminal/screen
integration and should not be bundled into the first storage experiment. This
experiment ports only the image map, storage limit, lookup, replacement, and
eviction foundation. It gives later experiments a real place to store validated
`Image` values without introducing placement tracking or terminal mutation yet.

## Changes

1. Add an internal Kitty storage module.
   - Add `roastty/src/terminal/kitty/graphics_storage.rs`.
   - Add `pub(crate) mod graphics_storage;` from
     `roastty/src/terminal/kitty/mod.rs`.
   - Keep all types internal to Roastty. Do not add public C ABI.

2. Port the foundational `ImageStorage` fields.
   - Add `ImageStorage` with:
     - `dirty`;
     - `next_image_id`;
     - `loading`;
     - `image_limits`;
     - `total_bytes`;
     - `total_limit`;
     - the image map keyed by image ID.
   - Preserve upstream defaults where applicable:
     - `next_image_id = 2147483647`;
     - `total_limit = 320 * 1000 * 1000`;
     - `image_limits = LoadingImageLimits::DIRECT`;
     - storage disabled when `total_limit == 0`.
   - Do not add placements, placement keys, grid pins, terminal pointers, or
     deletion geometry in this experiment.

3. Port image-storage behavior that does not require placements.
   - `enabled()` returns whether the Kitty graphics protocol is enabled for the
     storage.
   - `set_limit(limit)` updates the total byte limit.
     - Setting the limit to zero clears all stored images and resets byte
       accounting while preserving image-loading limits.
     - Lowering the limit evicts enough existing images when possible.
   - `add_image(image)` consumes a validated `Image`.
     - Reject images larger than the storage limit.
     - Evict existing images as needed before insertion.
     - Replace any existing image with the same ID and update byte accounting.
       Replacement capacity math must subtract the old image's bytes before
       deciding whether eviction is required, so same-ID replacements do not
       over-evict or fail when the final storage size fits.
     - Mark storage dirty after successful mutation.
   - `image_by_id(id)` returns a borrowed image reference.
   - `image_by_number(number)` returns the newest image with that image number,
     using transmit time ordering and ID tie-breaking compatible with upstream.
   - `evict_image(required_bytes)` evicts oldest images first. Because placement
     tracking is deferred, all images are currently treated as unused.
     - Eviction succeeds when `evicted_bytes >= required_bytes`. Exact-fit
       eviction is valid and must not require evicting more than necessary.

4. Keep storage ownership explicit.
   - Do not clone image payloads during lookup, replacement, or eviction.
   - Prefer borrowed lookups and move ownership on insertion.
   - If a metadata-only copy is needed for tests or debug helpers, use
     `Image::without_data()` from Experiment 187.

5. Port focused storage tests.
   - Add Rust tests covering:
     - default enabled state and defaults;
     - disabling with `set_limit(0)` clears images and preserves loading limits;
     - adding an image updates `total_bytes`, lookup, and dirty state;
     - replacing an image with the same ID fixes byte accounting for same-size,
       smaller, and larger-but-still-fitting replacements;
     - same-ID replacement does not over-evict before subtracting the old image
       bytes;
     - images larger than the limit are rejected without mutation;
     - lowering the limit evicts oldest images;
     - lowering the limit with exact-fit eviction succeeds;
     - adding an image evicts enough old images to fit;
     - adding an image with exact-fit eviction succeeds;
     - `image_by_id` borrows the stored image;
     - `image_by_number` picks the newest matching number and uses ID as a
       deterministic tie-breaker;
     - eviction never clones image payloads.

## Verification

Run:

```bash
cargo fmt -- roastty/src/terminal/kitty/mod.rs roastty/src/terminal/kitty/graphics_image.rs roastty/src/terminal/kitty/graphics_storage.rs
cargo test -p roastty kitty_graphics_storage
cargo test -p roastty kitty_graphics_image
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when image storage can own, replace, look up, limit, and
evict direct-loaded images under focused tests, while placements, terminal
execution, deletion geometry, rendering, and C ABI remain untouched.

## Non-Negotiable Invariants

- Do not expose any `ghostty_*` ABI names.
- Do not add public Kitty graphics C ABI in this experiment.
- Do not mutate terminal state from image storage.
- Do not add placement tracking, grid pins, deletion geometry, renderer
  integration, Unicode virtual placement, or terminal APC execution.
- Do not implement file, temporary-file, or shared-memory reads.
- Do not implement PNG decoding.
- Do not clone image payloads for lookup, eviction, or replacement.
- Do not skip Codex design review. If the design review finds a real issue, fix
  it and re-review before committing this experiment design.
- Do not skip Codex result review after implementation.

## Result

**Result:** Pass

The experiment added `roastty/src/terminal/kitty/graphics_storage.rs` and wired
it through `roastty/src/terminal/kitty/mod.rs`. The new internal storage module
ports the Kitty image-storage foundation without adding placements, terminal
mutation, renderer integration, PNG decoding, non-direct media, or public C ABI.

The implementation includes:

- `ImageStorage` with upstream-style defaults;
- image map ownership keyed by image ID;
- borrowed `image_by_id` and `image_by_number` lookups;
- `enabled()` and `set_limit()` behavior;
- `add_image()` ownership transfer, replacement, byte accounting, and dirty
  marking;
- eviction of oldest currently-unused images;
- replacement accounting that subtracts the existing image before deciding
  whether eviction is needed;
- exact-fit eviction using `evicted_bytes >= required_bytes`.

Verification passed:

```bash
cargo fmt -- roastty/src/terminal/kitty/mod.rs roastty/src/terminal/kitty/graphics_image.rs roastty/src/terminal/kitty/graphics_storage.rs
cargo test -p roastty kitty_graphics_storage
cargo test -p roastty kitty_graphics_image
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The full `roastty` suite passed with 1936 Rust tests plus the C harness.

Codex result review found one real issue: deriving `Default` for `ImageStorage`
gave Rust's zero defaults instead of upstream storage defaults. The
implementation was fixed so `ImageStorage::default()` delegates to
`ImageStorage::new()`, and the defaults test now covers both paths. A follow-up
Codex review found no blocking issues and approved recording the experiment as
Pass.

## Conclusion

Roastty now has a real internal storage foundation for Kitty graphics images.
Validated `Image` values from Experiment 187 can be owned, replaced, looked up,
limited, and evicted without cloning payloads. The remaining Kitty graphics work
can now proceed toward placements and execution on top of this storage layer,
rather than inventing storage behavior inside those later experiments.
