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

# Experiment 191: Port Kitty Display Storage Execution

## Description

Experiment 190 added placement storage, but `graphics_exec.rs` still returns
`ERROR: unimplemented action` for display commands. The next coherent slice is
the storage-side display execution from:

- `vendor/ghostty/src/terminal/kitty/graphics_exec.zig`

Upstream display execution has two responsibilities:

1. validate and resolve the target image;
2. create a placement and optionally move the terminal cursor.

Roastty can now implement the first part and the storage insertion half of the
second part. It must still defer terminal cursor movement and tracked page-pin
ownership because the current Kitty executor only receives `ImageStorage`, not a
full `Terminal`.

This experiment should add an internal display execution helper that takes an
explicit `PlacementLocation`. Later terminal integration can call this helper
with the current cursor's tracked location. Do not fake cursor ownership inside
`execute(storage, command)`.

## Changes

1. Add a storage-side display helper in `graphics_exec.rs`.
   - Add an internal function such as:
     - `display_with_location(storage, display, location) -> Response<'static>`
   - The helper receives:
     - mutable `ImageStorage`;
     - the parsed `Display`;
     - an explicit `PlacementLocation`.
   - The helper must not require `Terminal`, mutate cursor state, render, or
     expose C ABI.

2. Preserve `execute(storage, command)` scope.
   - Keep `CommandControl::Display` returning `ERROR: unimplemented action`
     through `execute(storage, command)` until terminal integration can supply a
     real location.
   - Do not silently place non-virtual display commands at `(0, 0)` from the
     generic executor.
   - Do not make `execute()` accept a fake terminal or optional cursor. If that
     becomes necessary, stop and design the terminal-integration experiment.

3. Implement display validation and lookup.
   - If both `image_id` and `image_number` are zero, return:
     - `EINVAL: image ID or number required`.
   - If `image_id` is non-zero, resolve by ID.
   - If `image_id` is zero and `image_number` is non-zero, resolve the newest
     matching image number via `ImageStorage::image_by_number`.
   - If no image is found, return:
     - `ENOENT: image not found`.
   - If lookup by image number succeeds, set the response `id` to the resolved
     image ID, matching upstream.

4. Implement virtual-placement validation.
   - For `Display { virtual_placement: true, parent_id > 0, ... }`, return:
     - `EINVAL: virtual placement cannot refer to a parent`.
   - For `virtual_placement: true` without a parent, force
     `PlacementLocation::Virtual` regardless of the helper's location argument.
   - For non-virtual display, use the explicit helper-provided location.
   - Do not implement Unicode virtual placement rendering; this only stores the
     virtual placement metadata.

5. Insert placement metadata.
   - Convert `Display` fields into a `Placement`:
     - `x_offset = display.x_offset`;
     - `y_offset = display.y_offset`;
     - `source_x = display.x`;
     - `source_y = display.y`;
     - `source_width = display.width`;
     - `source_height = display.height`;
     - `columns = display.columns`;
     - `rows = display.rows`;
     - `z = display.z`;
     - `location` from step 4.
   - Insert through `ImageStorage::add_placement`.
   - Preserve `placement_id` response behavior:
     - response uses the external `display.placement_id`;
     - storage generates an internal key when the caller placement ID is zero.
   - If insertion fails unexpectedly, return:
     - `EINVAL: failed to prepare terminal state`.

6. Keep cursor movement deferred.
   - Do not implement `CursorMovement::After` in this experiment.
   - Do not call terminal index/scroll-region logic.
   - Add tests proving the display helper stores placement metadata regardless
     of `cursor_movement`, but does not expose any cursor side effects.

7. Add focused display execution tests.
   - Port or create equivalent Rust tests for:
     - display requires image ID or image number;
     - missing image returns `ENOENT: image not found`;
     - display by ID inserts a placement;
     - display by image number resolves newest image and response ID;
     - placement metadata maps all display fields correctly;
     - zero placement ID creates an internal placement key but response
       placement ID remains zero;
     - non-zero placement ID creates/replaces an external placement key;
     - virtual placement stores `PlacementLocation::Virtual`;
     - virtual placement with parent ID returns the upstream validation error
       without mutation;
     - `CursorMovement::After` does not move a cursor or require terminal state;
     - quiet filtering preserves display success/failure behavior through the
       helper or a small wrapper if one is added.

## Verification

Run:

```bash
cargo fmt -- roastty/src/terminal/kitty/mod.rs roastty/src/terminal/kitty/graphics_exec.rs roastty/src/terminal/kitty/graphics_storage.rs roastty/src/terminal/kitty/graphics_image.rs
cargo test -p roastty kitty_graphics_exec
cargo test -p roastty kitty_graphics_storage
cargo test -p roastty kitty_graphics_image
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when internal display storage execution validates image
targets, resolves image numbers, inserts placement metadata, and preserves
upstream response semantics while leaving generic `execute(storage, command)`
display dispatch, terminal cursor movement, tracked page pins, rendering,
Unicode virtual placement rendering, deletion execution, animation execution,
and public C ABI deferred.

## Non-Negotiable Invariants

- Do not expose any `ghostty_*` ABI names.
- Do not add public Kitty graphics C ABI in this experiment.
- Do not render images.
- Do not mutate terminal cursor state.
- Do not add tracked terminal pin ownership.
- Do not add Unicode virtual placement rendering.
- Do not make generic `execute(storage, command)` fake a cursor location for
  display commands.
- Do not implement delete or animation execution.
- Do not break Experiment 189 query/transmit behavior.
- Do not break Experiment 190 placement storage or eviction behavior.
- Do not skip Codex design review. If the design review finds a real issue, fix
  it and re-review before committing this experiment design.
- Do not skip Codex result review after implementation.

## Result

**Result:** Pass

Implemented storage-side Kitty display execution in
`roastty/src/terminal/kitty/graphics_exec.rs`.

The implementation adds an internal `display_with_location()` helper that:

- validates that display commands specify either image ID or image number;
- resolves images by ID;
- resolves images by newest matching image number;
- sets the response ID to the resolved image ID for number lookups;
- returns `ENOENT: image not found` for missing images;
- rejects parented virtual placements with
  `EINVAL: virtual placement cannot refer to a parent`;
- stores unparented virtual placements as `PlacementLocation::Virtual`;
- maps display source rectangle, offsets, rows, columns, z, and placement ID
  into `Placement`;
- inserts placements through `ImageStorage::add_placement`;
- preserves zero placement ID response behavior while storage creates an
  internal placement key;
- supports external placement replacement through non-zero placement IDs;
- lets quiet filtering suppress successes and preserve/report failures as
  expected.

The generic `execute(storage, command)` path still returns
`ERROR: unimplemented action` for display commands. This preserves the
experiment's scope: terminal cursor movement, tracked page pins, renderer
integration, Unicode virtual rendering, deletion execution, animation execution,
and public C ABI remain deferred.

Codex result review found no blocking issues and approved the result as pass
ready.

Verification passed:

```bash
cargo fmt -- roastty/src/terminal/kitty/mod.rs roastty/src/terminal/kitty/graphics_exec.rs roastty/src/terminal/kitty/graphics_storage.rs roastty/src/terminal/kitty/graphics_image.rs
cargo test -p roastty kitty_graphics_exec
cargo test -p roastty kitty_graphics_storage
cargo test -p roastty kitty_graphics_image
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The focused exec suite passed with 33 tests. The full Roastty suite passed with
1,980 Rust tests plus the C harness.

## Conclusion

Roastty now has a storage-side display helper that can validate and create Kitty
image placements once terminal integration supplies a real location. The next
experiment should integrate Kitty display execution with terminal state so
non-virtual display commands can use the current cursor/tracked location and
`CursorMovement::After` can be implemented without faking state in
`ImageStorage` alone.
