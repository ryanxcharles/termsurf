# Experiment 193: Port Kitty Tracked Placement Ownership

## Description

Experiment 192 connected Kitty graphics APC dispatch to terminal state, but
normal display placements still store a temporary raw cell location:

```rust
PlacementLocation::Cell { x, y }
```

That was enough to prove terminal dispatch, but it is not Ghostty's ownership
model. Upstream display execution tracks a page pin for normal placements:

- `vendor/ghostty/src/terminal/kitty/graphics_exec.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_storage.zig`

The tracked pin lets an image placement follow terminal storage through scroll,
split, reflow, page remapping, and reset rules. Upstream placement cleanup also
untracks the pin when the placement is replaced, deleted, evicted, or cleared.

The next coherent slice is to replace Roastty's temporary normal-display cell
location with tracked placement ownership. This is still not rendering. The goal
is the correct terminal-storage lifetime model that renderer-facing state can
safely consume later.

## Changes

1. Replace the temporary normal-display cell location with a tracked pin
   location.
   - Change `PlacementLocation::Cell { x, y }` to a tracked-pin variant such as:
     - `PlacementLocation::Pin(NonNull<Pin>)`
   - Keep `PlacementLocation::Virtual` for virtual placements.
   - Remove or quarantine raw cell placement usage from production code. Raw
     cell coordinates may remain only in tests if needed for transitional helper
     assertions, but normal terminal display must use tracked pins.
   - Add helpers on `Placement` / `PlacementLocation` for detecting and
     extracting a tracked pin without exposing pointer manipulation throughout
     the codebase.

2. Add safe placement cleanup plumbing.
   - Any path that removes a placement must make the removed placement available
     to its owner so the tracked pin can be untracked.
   - The implementation contract must be concrete:
     - placement insertion returns any replaced placement;
     - image insertion/eviction returns every placement removed by eviction;
     - limit changes / storage disable return every placement removed by clear;
     - clearing storage drains every placement before dropping the map.
   - `Screen` must immediately untrack pins from every removed placement
     returned by these operations.
   - Cover at least these removal paths:
     - external placement replacement by the same `(image_id, placement_id)`;
     - internal placement removal when an image is evicted;
     - all placements for an image when that image is evicted;
     - storage disable / `set_limit(0)`;
     - active screen reset.
   - Do not leave stale tracked pins in `PageList::tracked_pins` after any of
     those operations.
   - If an insertion fails after a new tracked pin was allocated, untrack the
     newly allocated pin before returning the error.

3. Add screen-level Kitty image/storage ownership methods.
   - Keep `ImageStorage` focused on image/placement metadata.
   - Add `Screen` helpers that coordinate image storage with page pin cleanup,
     for example:
     - `add_kitty_image(...)`;
     - `set_kitty_image_limit(...)`;
     - `add_kitty_placement(...)`;
     - `clear_kitty_images(...)`.
   - The exact names may differ, but the ownership rule is fixed: `Screen` owns
     both `ImageStorage` and `PageList`, so `Screen` is the layer that must
     untrack placement pins removed from storage.
   - Avoid making `ImageStorage` reach directly into `PageList`.
   - Restrict direct mutable access to `ImageStorage` after this experiment:
     - production terminal dispatch must use `Screen` ownership helpers for any
       operation that can remove or replace placements;
     - raw `kitty_images_mut()` access may remain only for tests or for tightly
       scoped internals that cannot remove placements;
     - no production caller may invoke a placement-removing `ImageStorage`
       method without participating in cleanup.

4. Update Kitty execution to use the ownership helpers.
   - Query/transmit execution must not keep calling bare
     `graphics_exec::execute(screen.kitty_images_mut(), command)` from terminal
     dispatch if that path can evict images or remove placements.
   - Route terminal query/transmit/display through screen-owned helpers, or
     update `graphics_exec` to return removed placement cleanup data that
     `Screen` consumes immediately.
   - This includes transmit-driven eviction. A transmit APC that evicts an image
     with placements must untrack those placements in the same command handling
     path.
   - For terminal display, track the active cursor pin through
     `Screen::track_pin(...)` and store the tracked pin in the placement.
   - If display creation fails after tracking a pin, untrack it before returning
     `EINVAL: failed to prepare terminal state`.
   - Preserve virtual placement behavior: virtual placements must not allocate
     tracked pins.
   - Keep `CursorMovement::After` deferred. Do not move the cursor in this
     experiment.

5. Preserve terminal dispatch behavior from Experiment 192.
   - Query and transmit APCs still execute and write responses.
   - Display APCs still create placements and write responses.
   - Quiet filtering remains identical to Experiment 192, including `q=2`
     suppressing all display responses.
   - Non-Kitty, malformed, and over-limit APC behavior remains unchanged.
   - Primary and alternate screens continue to have separate Kitty image
     storage.

6. Add focused tests for tracked placement ownership.
   - Add tests proving:
     - normal display stores a tracked pin location, not a raw cell coordinate;
     - the tracked placement resolves to the display cursor's grid cell;
     - virtual display does not allocate a tracked pin;
     - replacing an external placement untracks the old pin;
     - zero-placement-ID internal placements each track their own pin;
     - evicting an image untracks every placement for that image;
     - transmit-driven eviction through terminal APC dispatch untracks every
       evicted placement pin;
     - replacing an image with the same image ID preserves still-live placements
       and does not untrack their pins;
     - lowering/disabling storage clears placements and untracks pins;
     - failed display insertion untracks the newly allocated pin;
     - `Terminal::reset()` and RIS/full reset clear image storage and do not
       leave stale tracked placement pins;
     - alternate-screen image placement pins are independent from primary-screen
       image placement pins.
   - Add test-only tracked-pin count accessors if needed. Keep them
     `#[cfg(test)]`.

## Verification

Run:

```bash
cargo fmt -- roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs roastty/src/terminal/kitty/mod.rs roastty/src/terminal/kitty/graphics_command.rs roastty/src/terminal/kitty/graphics_exec.rs roastty/src/terminal/kitty/graphics_storage.rs
cargo test -p roastty kitty_graphics_exec
cargo test -p roastty kitty_graphics_storage
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty tracked_grid_ref
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when normal Kitty display placements own tracked page
pins, every storage path that removes such placements untracks them, and the
Experiment 192 terminal dispatch behavior still passes, while leaving rendering,
renderer-facing C ABI, deletion execution, animation execution, Unicode virtual
placement rendering, `CursorMovement::After`, and atomic transmit-and-display
semantics deferred.

## Non-Negotiable Invariants

- Do not expose any `ghostty_*` ABI names.
- Do not add public Kitty graphics C ABI in this experiment.
- Do not render images.
- Do not add renderer-facing placement iteration APIs.
- Do not mutate terminal cursor position for Kitty graphics commands.
- Do not implement `CursorMovement::After`.
- Do not add Unicode virtual placement rendering.
- Do not implement delete or animation execution.
- Do not implement atomic transmit-and-display semantics.
- Do not leave stale tracked pins after placement replacement, image eviction,
  storage disable, or screen reset.
- Do not reset `PageList` before deinitializing Kitty placements that own
  tracked pins. Screen reset/RIS must clear Kitty placements and untrack their
  pins before replacing image storage or resetting pages.
- Do not make `ImageStorage` directly own or mutate `PageList`; coordinate that
  through `Screen`.
- Do not leave production terminal dispatch using raw mutable `ImageStorage`
  APIs for commands that can remove placements.
- Do not break Experiment 189 query/transmit behavior.
- Do not break Experiment 190 placement storage and eviction behavior.
- Do not break Experiment 191 storage-side display validation behavior.
- Do not break Experiment 192 terminal dispatch behavior.
- Do not skip Codex design review. If the design review finds a real issue, fix
  it and re-review before committing this experiment design.
- Do not skip Codex result review after implementation.

## Result

**Result:** Pass

Normal Kitty graphics display placements now store tracked page pins instead of
raw cell coordinates. Terminal APC dispatch routes through a screen-owned
execution path, and `Screen` coordinates every placement-removing storage
operation with `PageList` pin cleanup.

The implementation added explicit removed/replaced placement return values for
storage mutation paths, screen-level ownership helpers, reset-time cleanup
before page reset, and focused tests for replacement, internal placement IDs,
transmit-driven eviction, storage disable, failed insertion cleanup, same-image
replacement, virtual placement behavior, reset/RIS cleanup, and alternate-screen
independence.

Verification passed:

```bash
cargo fmt -- roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs roastty/src/terminal/page_list.rs roastty/src/terminal/kitty/graphics_exec.rs roastty/src/terminal/kitty/graphics_storage.rs
cargo test -p roastty kitty_graphics_storage
cargo test -p roastty kitty_graphics_exec
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty tracked_grid_ref
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The full `cargo test -p roastty` run passed 1999 unit tests plus the ABI
harness. Codex result review also passed with no findings and called the
implementation pass-ready.

## Conclusion

Experiment 193 completes the Kitty placement ownership foundation needed before
renderer-facing image state can safely consume placements. Normal placements now
follow terminal storage through tracked pins, and every currently implemented
storage-removal path has a cleanup owner.

The next experiment should build on this by choosing the next coherent Kitty
graphics slice: either renderer-facing placement iteration/C ABI, delete command
execution, or cursor-after/atomic transmit-and-display semantics.
