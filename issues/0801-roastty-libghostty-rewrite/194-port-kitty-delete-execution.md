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

# Experiment 194: Port Kitty Delete Execution

## Description

Experiment 193 gave normal Kitty graphics placements the same tracked-pin
ownership model as upstream Ghostty. The next coherent Kitty graphics slice is
delete execution.

Roastty already parses Kitty delete commands from Experiment 186, but execution
still returns placeholder "unimplemented" responses. Upstream Ghostty handles
delete in:

- `vendor/ghostty/src/terminal/kitty/graphics_exec.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_storage.zig`

Delete execution is the first protocol feature that heavily exercises the
tracked-placement cleanup model:

- delete all placements;
- delete placements by image ID / placement ID;
- delete newest numbered image placements;
- delete placements intersecting cursor, cell, row, column, or z-index;
- optionally delete now-unused images;
- never respond on successful delete;
- preserve dirty tracking and image byte accounting.

This experiment ports delete execution without rendering, without animation
support, and without adding public C ABI.

## Changes

1. Add storage-side delete result plumbing.
   - Add a delete result type that returns every removed placement and every
     removed image/accounting mutation needed by `Screen`.
   - Keep `ImageStorage` responsible for image and placement metadata only.
   - Keep `Screen` responsible for untracking placement pins returned by delete
     operations.
   - Preserve the Experiment 193 invariant: no placement-removing storage path
     may silently drop a tracked pin.

2. Port upstream delete selectors.
   - Implement these delete variants from `Delete`:
     - `All { delete_images }`;
     - `Id { delete, image_id, placement_id }`;
     - `Newest { delete, image_number, placement_id }`;
     - `IntersectCursor { delete }`;
     - `IntersectCell { delete, x, y }`;
     - `IntersectCellZ { delete, x, y, z }`;
     - `Column { delete, x }`;
     - `Row { delete, y }`;
     - `Z { delete, z }`;
     - `Range { delete, first, last }`.
   - Keep `AnimationFrames` as a successful no-op for now, matching upstream's
     "animation frames are successfully deleted" placeholder.
   - Preserve upstream behavior that virtual placements are skipped for
     all-placement deletes and z deletes.
   - Preserve upstream one-based external coordinates for cell, row, and column
     delete selectors.
   - Port upstream's range behavior and tests exactly in this experiment, even
     where the implementation looks broader than the name suggests. If this is
     later judged to be an upstream bug, fix it in a separate experiment with a
     dedicated compatibility note.

3. Add placement rectangle helpers.
   - Add a private terminal-owned Kitty graphics metrics source used only by
     internal Kitty placement geometry.
   - The metrics source should produce
     `CellMetrics { columns, rows, width_px, height_px }`.
   - Until a later surface/renderer experiment wires real pixel dimensions into
     terminal state, initialize and resize this metric as one pixel per grid
     cell (`width_px = cols`, `height_px = rows`). This is explicit fallback
     behavior, not an accidental approximation.
   - Add test-only setters for the metrics so upstream-style delete geometry
     tests can use the same 100x100 grid / 100x100 pixel setup used by Ghostty's
     tests.
   - Do not call the size-report callback merely to compute delete geometry.
     Size-report callbacks answer terminal queries; they are not owned terminal
     state and should not become a hidden dependency of APC execution.
   - Add a Rust equivalent of upstream `Placement.rect(...)`.
   - The helper should:
     - return `None` for virtual placements;
     - resolve the tracked pin through `Screen::tracked_pin_value(...)`;
     - use existing `Placement::grid_size(...)`;
     - clamp the bottom-right x to the terminal width;
     - follow page-list pin movement for vertical extent.
   - Keep this helper internal. Do not expose renderer-facing placement
     iteration or C ABI in this experiment.

4. Add screen-level delete helper.
   - Add a `Screen` method that executes a Kitty delete command against its
     storage and immediately untracks returned placements.
   - This method must also handle optional image deletion for now-unused images
     while preserving total byte accounting.
   - `graphics_exec::execute_screen(...)` must call this helper for
     `CommandControl::Delete`.
   - Successful delete commands must produce no terminal response.
   - Malformed/unsupported delete forms should preserve existing parser
     behavior; execution should not invent new response messages for successful
     no-op cases.

5. Preserve existing deferred work.
   - Do not render images.
   - Do not add renderer-facing C ABI.
   - Do not implement animation frame storage.
   - Do not implement `CursorMovement::After`.
   - Do not implement atomic transmit-and-display semantics.
   - Do not implement Unicode virtual placement rendering.

6. Port upstream-style tests.
   - Add storage/screen/terminal tests covering:
     - delete all placements and images;
     - delete all placements and images preserves `total_limit`;
     - delete all placements without images;
     - delete by image ID;
     - delete by image ID and remove unused image;
     - delete one external placement by placement ID;
     - delete newest image number placements;
     - delete intersecting cursor, including a multi-hit case;
     - delete intersecting explicit cell;
     - delete intersecting explicit cell with z filter;
     - delete by column;
     - delete by row;
     - delete by z;
     - delete by range, matching upstream's current tests;
     - uppercase delete variants remove now-unused images when applicable;
     - virtual placements are skipped by all-placement and z deletes;
     - successful delete writes no response;
     - `q` quiet handling does not matter because successful delete is silent;
     - delete selectors mark storage dirty;
     - all removed tracked placements are untracked;
     - image byte accounting is updated when images are removed.
   - Keep any new test-only helpers `#[cfg(test)]`.

## Verification

Run:

```bash
cargo fmt -- roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs roastty/src/terminal/page_list.rs roastty/src/terminal/kitty/graphics_command.rs roastty/src/terminal/kitty/graphics_exec.rs roastty/src/terminal/kitty/graphics_storage.rs
cargo test -p roastty kitty_graphics_command_delete
cargo test -p roastty kitty_graphics_storage_delete
cargo test -p roastty kitty_graphics_exec
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when parsed Kitty delete commands execute against the
active screen's Kitty image storage, successful deletes are silent, every
removed tracked placement is untracked, optional unused-image deletion updates
byte accounting, and all existing Kitty query/transmit/display behavior remains
unchanged.

## Non-Negotiable Invariants

- Do not expose any `ghostty_*` ABI names.
- Do not add public Kitty graphics C ABI.
- Do not render images.
- Do not add renderer-facing placement iteration APIs.
- Do not implement animation frame storage.
- Do not implement `CursorMovement::After`.
- Do not implement atomic transmit-and-display semantics.
- Do not implement Unicode virtual placement rendering.
- Do not leave stale tracked pins after any delete selector.
- Do not make `ImageStorage` directly own or mutate `PageList`; deleted
  placements must be returned to `Screen` for cleanup.
- Do not make successful delete commands write OK responses. Upstream delete is
  silent on success.
- Do not break Experiment 189 query/transmit behavior.
- Do not break Experiment 190 placement storage and eviction behavior.
- Do not break Experiment 191 display storage validation behavior.
- Do not break Experiment 192 terminal dispatch behavior.
- Do not break Experiment 193 tracked placement ownership behavior.
- Do not skip Codex design review. If the design review finds a real issue, fix
  it and re-review before committing this experiment design.
- Do not skip Codex result review after implementation.

## Result

**Result:** Pass

Kitty delete execution now runs on the screen-owned terminal path. Parsed delete
commands are dispatched through `graphics_exec::execute_screen(...)` into
`Screen::delete_kitty(...)`, and successful deletes are silent. Removed
placements are returned to `Screen`, so tracked pins are untracked from the
page-list owner instead of being deleted inside storage alone.

The implementation covers delete-all, image ID selectors, newest-image
selectors, cursor/cell/z intersections, row, column, z, range, unused-image
cleanup, dirty tracking, and byte accounting. Storage-only delete execution
remains unimplemented by design because it cannot safely clean up screen-owned
tracked pins.

Verification passed:

```bash
cargo fmt -- roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs roastty/src/terminal/page_list.rs roastty/src/terminal/kitty/graphics_exec.rs roastty/src/terminal/kitty/graphics_storage.rs
cargo test -p roastty kitty_graphics_storage_delete
cargo test -p roastty terminal_stream_kitty_graphics_delete
cargo test -p roastty kitty_graphics_command_delete
cargo test -p roastty kitty_graphics_exec
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The full suite passed with `2008` unit tests plus the ABI harness.

Codex result review initially found one real blocker: oversized placements that
extended past the page-list rows could be skipped by intersection deletes. The
fix added `PageList::pin_down_or_end(...)`, switched Kitty placement rectangle
calculation to clamp to the page-list end, and added
`terminal_stream_kitty_graphics_delete_intersect_clamps_oversized_placement`.
After that fix, Codex re-reviewed the diff and reported no blocking findings.

## Conclusion

Experiment 194 completes the first screen-owned Kitty graphics mutation path.
Roastty can now remove stored Kitty placements and images while keeping
page-list pin ownership coherent. The next experiment can move to the next Kitty
graphics subsystem without leaving stale tracked placement state behind.
