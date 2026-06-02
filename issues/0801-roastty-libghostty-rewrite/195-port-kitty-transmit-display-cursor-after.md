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

# Experiment 195: Port Kitty Transmit-Display and Cursor-After

## Description

Experiments 186-194 built the internal Kitty graphics pipeline through parsing,
direct image loading, image storage, display placement storage, terminal APC
dispatch, tracked placement ownership, and delete execution. The remaining
unimplemented execution paths that are still core-terminal behavior, not
renderer behavior, are:

- atomic `TransmitAndDisplay`;
- display-time `CursorMovement::After`.

Upstream Ghostty implements both in
`vendor/ghostty/src/terminal/kitty/graphics_exec.zig`:

- `transmit(...)` handles both `.transmit` and `.transmit_and_display`;
- when the completed load includes a display command, it immediately calls the
  same display path with the loaded image ID;
- after a successful non-virtual display, `CursorMovement::After` moves the
  cursor using terminal-level `index()` and `setCursorPos(...)`.

Roastty currently leaves `TransmitAndDisplay` as an unimplemented response and
has an explicit regression test named
`terminal_stream_kitty_graphics_cursor_after_does_not_move_cursor_yet`. This
experiment removes those placeholders. It still does not render images, expose
renderer-facing placement iteration, implement animation, load
file/shared-memory media, or render Unicode virtual placeholders.

The architectural point is that cursor-after movement is not storage-owned and
not purely screen-owned. Upstream uses terminal-level `index()` so scrolling
regions and scrollback behavior are honored. The Roastty port must keep that
ownership: Kitty display execution may report that cursor-after movement is
needed, but terminal state must perform the movement.

## Changes

1. Add terminal-level Kitty execution result plumbing.
   - Keep storage-only `graphics_exec::execute(storage, command)` available for
     existing storage tests and for commands that do not need terminal cursor
     movement.
   - Update the terminal/screen-owned path so it can return both:
     - the optional Kitty graphics response;
     - an optional cursor-after movement request.
   - The movement request should contain only the information the terminal needs
     after display succeeds, for example:
     - original placement pin column;
     - computed placement grid columns;
     - computed placement grid rows.
   - Do not make `ImageStorage` move the cursor or call terminal methods.
   - Do not make `Screen` emulate terminal `index()` behavior.

2. Port `TransmitAndDisplay` execution.
   - Route `CommandControl::TransmitAndDisplay` through the same image loading
     path as `Transmit`.
   - Preserve chunked transmission quiet semantics from the existing transmit
     path:
     - intermediate chunks produce no response;
     - final chunk inherits or updates `q` exactly like transmit;
     - loading state is cleared on final success or final error.
   - When the final loaded image has an associated display command, display it
     immediately using the loaded image ID, not the original display ID.
   - Use the same screen-owned display helper as normal `Display`, so tracked
     pin ownership, placement replacement cleanup, virtual placement validation,
     quiet filtering, and placement field mapping are identical.
   - If the image stores successfully but display fails, preserve upstream
     behavior: the image remains stored and the response reports the display
     failure.
   - If the loaded image has an implicit auto-assigned ID, preserve upstream
     behavior: the display may still be stored, but the final response is empty.
   - Do not implement file, temporary-file, shared-memory, PNG decode,
     animation, or rendering in this experiment.

3. Port `CursorMovement::After`.
   - Apply cursor-after movement only after a successful non-virtual display.
   - `CursorMovement::None` must continue to leave the cursor unchanged.
   - Virtual placements must not move the cursor.
   - Use the placement's computed grid size with the current Kitty graphics cell
     metrics.
   - Match upstream Ghostty's movement shape:
     - call terminal `index()` once per occupied placement row;
     - then call a terminal-level `setCursorPos(...)` equivalent with the
       current active cursor row and `pin.x + grid_cols + 1`;
     - preserve upstream one-based/clamped `setCursorPos(...)` semantics,
       including origin-mode offsets and margin clamping.
   - Add or reuse a terminal helper for upstream-style `setCursorPos(...)` if
     the existing `cursor_position_basic(...)` helper is too low-level.
   - Keep pending-wrap clearing consistent with the existing cursor-position
     path.

4. Keep existing screen ownership invariants intact.
   - Normal display placements must still store tracked pins.
   - Replaced or evicted placements must still be untracked.
   - Transmit-driven eviction during a `TransmitAndDisplay` command must still
     untrack evicted placements before adding/displaying the new image.
   - Failed display after a newly tracked pin allocation must still untrack that
     newly allocated pin.
   - Primary and alternate screens keep separate Kitty image storage.

5. Replace placeholder tests with upstream-style behavior tests.
   - Update the storage-level
     `kitty_graphics_exec_transmit_and_display_is_unimplemented_without_store`
     test so storage-only execution remains intentionally unimplemented or is
     clearly marked as not the production terminal path.
   - Replace
     `terminal_stream_kitty_graphics_cursor_after_does_not_move_cursor_yet` with
     passing cursor-after behavior.
   - Add terminal tests covering:
     - `TransmitAndDisplay` stores the image and creates a tracked placement in
       one APC;
     - `TransmitAndDisplay` maps display fields into the stored placement;
     - `TransmitAndDisplay` by image number/auto-ID behavior matches existing
       transmit/display response rules;
     - chunked `TransmitAndDisplay` displays only on the final chunk;
     - display failure after image storage leaves the image stored and reports
       the display error;
     - `CursorMovement::After` moves right of a one-row placement;
     - `CursorMovement::After` indexes once per placement row;
     - cursor-after honors scrolling at the bottom row instead of only changing
       `y` arithmetically;
     - cursor-after honors origin mode and vertical scrolling margins;
     - cursor-after honors origin mode plus left/right margins when clamping the
       final column;
     - `CursorMovement::None` preserves the old cursor position;
     - virtual placements with `C=0` do not move the cursor;
     - failed display does not move the cursor.
     - `TransmitAndDisplay` with `C=0` stores/displays and moves the cursor
       through the same cursor-after path as standalone display.
     - chunked `TransmitAndDisplay` with `C=0` does not move the cursor on
       intermediate chunks and moves it only after the final successful display.
   - Keep any new helper APIs test-only unless production execution needs them.

## Verification

Run:

```bash
cargo fmt -- roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs roastty/src/terminal/kitty/graphics_exec.rs roastty/src/terminal/kitty/graphics_storage.rs
cargo test -p roastty kitty_graphics_exec
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty terminal_stream_kitty_graphics_delete
cargo test -p roastty terminal_stream_csi_scroll_up
cargo test -p roastty terminal_stream_split_feed_csi_scroll_up_and_down
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when terminal APC execution supports atomic
transmit-and-display, successful display still stores tracked placements with no
stale pins, cursor-after movement matches upstream terminal-level index/set
cursor behavior, and all previous Kitty transmit, display, delete, storage, and
dispatch tests still pass.

## Non-Negotiable Invariants

- Do not expose any `ghostty_*` ABI names.
- Do not add public Kitty graphics C ABI.
- Do not render images.
- Do not add renderer-facing placement iteration APIs.
- Do not implement animation frame storage or animation execution.
- Do not implement file, temporary-file, shared-memory, or PNG image loading.
- Do not implement Unicode virtual placement rendering.
- Do not move the cursor from storage-only code.
- Do not emulate terminal `index()` inside `ImageStorage` or plain `Screen`
  helpers.
- Do not leave stale tracked pins after transmit-driven eviction, display
  replacement, failed placement insertion, or delete.
- Do not make successful delete commands write OK responses.
- Do not break Experiment 189 query/transmit behavior.
- Do not break Experiment 190 placement storage and eviction behavior.
- Do not break Experiment 191 display storage validation behavior.
- Do not break Experiment 192 terminal dispatch behavior.
- Do not break Experiment 193 tracked placement ownership behavior.
- Do not break Experiment 194 delete execution behavior.
- Do not skip Codex design review. If the design review finds a real issue, fix
  it and re-review before committing this experiment design.
- Do not skip Codex result review after implementation.

## Result

**Result:** Pass

Implemented Kitty graphics `TransmitAndDisplay` (`a=T`) on the terminal/screen
execution path. A completed transmit-display command now stores the image,
creates the placement using the same placement logic as display, preserves the
normal Kitty response fields, and supports chunked direct transmissions. The
chunked path now shares quiet inheritance with plain transmit, so a quiet value
set on an earlier chunk is honored by the final `a=T` chunk.

Implemented `CursorMovement::After` (`C=0`) for screen-executed display and
transmit-display commands. Graphics execution returns a `CursorAfter` request,
and the terminal stream handler performs the actual movement with terminal-owned
index/set-cursor behavior. Storage-only execution still does not move the
cursor.

Added regression coverage for:

- explicit `a=T` image and placement storage;
- display field mapping through `a=T`;
- numbered `a=T` auto-ID response behavior;
- implicit `a=T` response suppression;
- chunked `a=T` display on the final chunk;
- chunked `a=T` quiet inheritance;
- failed `a=T` placement after successful image load;
- cursor-after movement for display and transmit-display;
- delayed cursor-after movement until the final chunk;
- cursor-after scroll/index behavior, origin-mode margins, virtual placement,
  and failed-display no-move behavior.

Verification passed:

```bash
cargo fmt -- roastty/src/terminal/terminal.rs roastty/src/terminal/kitty/graphics_exec.rs
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty kitty_graphics_exec
cargo test -p roastty terminal_stream_kitty_graphics_delete
cargo test -p roastty terminal_stream_csi_scroll_up
cargo test -p roastty terminal_stream_split_feed_csi_scroll_up_and_down
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The full `cargo test -p roastty` run passed 2023 Rust tests plus the C ABI
harness and doc tests.

Codex result review initially found one real issue: chunked `a=T` was not
sharing the plain transmit quiet-inheritance path, and tests for numbered and
implicit `a=T` were missing. Those findings were fixed. The follow-up Codex
review reported no blocking findings and marked the implementation pass-ready.

## Conclusion

Experiment 195 completes the direct in-terminal Kitty graphics execution slice
through transmit, display, delete, transmit-display, and cursor-after movement.
The remaining Kitty graphics work can proceed to the next coherent subsystem
without revisiting the terminal-owned cursor movement boundary established here.
