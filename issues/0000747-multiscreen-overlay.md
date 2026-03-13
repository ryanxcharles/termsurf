# Issue 747: Overlay doesn't reposition on split (second screen)

## Goal

When a pane is split, the webview overlay must reposition immediately — not just
resize. This must work on all screens, not just the primary display.

## Background

Issue 746 replaced the overlay's broken positioning system with one that
piggybacks on the terminal's own render pass. The old system
(`reposition_all_overlays`, `get_pane_cell_position`, global `metrics` atomics)
computed overlay coordinates independently and was wrong in multiple ways. The
new system returns `(left_pixel_x, top_pixel_y)` from `paint_pane()` and passes
those coordinates directly to `set_overlay_frame()`, which converts backing
pixels to logical points (`dpi / 72.0`) and sets the CALayer frame inside a
`CATransaction` (to suppress animation).

Issue 746 also added a `MuxNotification::WindowInvalidated` notification after
the async split completes (spawn.rs), ensuring the GUI repaints with the updated
split tree.

This works on the primary screen. On a secondary screen, it doesn't.

### What works

- Single window, single screen: open, split, resize, tab switch — all correct.
- Scale correct on Retina (`dpi / 72.0` = 2.0).
- No animation artifacts (CATransaction suppresses implicit animations).
- Split triggers immediate overlay reposition via `WindowInvalidated`.

### The bug

Steps to reproduce:

1. Open a new window.
2. Move the window to the second screen.
3. Open a webview. It positions correctly.
4. Split pane to the left.
5. The webview **resizes** (width shrinks correctly) but does **not reposition**
   (x stays at 0, left edge, instead of moving to the right pane).
6. Press any key — the overlay snaps to the correct position.

This does not happen on the primary screen. The same window on the primary
screen repositions correctly on split. The bug is specific to a window on a
secondary display.

### What this tells us

The overlay's width and x position are both set in the same
`set_overlay_frame()` call from `paint_pass()`. The width comes from
`pane.pixel_width` (updated by the TUI's protocol resize message after
SIGWINCH). The x comes from `pane_pixel_x` (returned by `paint_pane()`, which
computes `padding_left + border.left + pos.left * cell_width`).

Since the width updates but x doesn't, the paint pass is running — but with
`pos.left` still at 0 (the pre-split value). Then a keypress triggers another
paint which sees the correct `pos.left`.

The `WindowInvalidated` notification travels through a 4-hop async chain:

1. `mux.notify()` → subscriber calls `spawn_into_main_thread`
2. `mux_pane_output_event_callback` → `window.notify()` →
   `Connection::with_window_inner` → `spawn_into_main_thread`
3. Event handler dispatches `WindowInvalidated` → `window.invalidate()` →
   `Connection::with_window_inner` → `spawn_into_main_thread`
4. Inner invalidate runs `setNeedsDisplay: true`

On macOS, the spawn queue processes one task per `CFRunLoopObserver` invocation.
Each display has its own `CVDisplayLink` at its own phase. The hypothesis is
that on the second screen, display timing causes a repaint between the tree
update and the `setNeedsDisplay: true` arriving — so the repaint sees old
positions but new sizes (since the TUI's resize message updates
`pane.pixel_width` through a different path).

However, `get_panes_to_render()` reads directly from `tab.iter_panes()`, which
walks the live split tree. The tree is updated before the notification is sent.
So any paint after the split should see the correct `pos.left`. This contradicts
the observed behavior and suggests the root cause may be elsewhere.

### Key files

- `wezboard/wezboard-gui/src/termwindow/render/paint.rs` — `paint_pass()`,
  overlay positioning block (lines 263–288)
- `wezboard/wezboard-gui/src/termwindow/render/pane.rs` — `paint_pane()`,
  returns `(left_pixel_x, top_pixel_y)`
- `wezboard/wezboard-gui/src/termsurf/conn.rs` — `set_overlay_frame()`,
  `update_ca_layer_frame()` (initial placement)
- `wezboard/wezboard-gui/src/spawn.rs` — `WindowInvalidated` notification after
  split
- `wezboard/wezboard-gui/src/termwindow/mod.rs` — `WindowInvalidated` handler
  (line 1328), notification filter (line 1552)
- `wezboard/window/src/os/macos/window.rs` — `invalidate()` →
  `setNeedsDisplay: true`
- `wezboard/window/src/os/macos/connection.rs` — `with_window_inner` →
  `spawn_into_main_thread`

### Root cause

The initial analysis in the "What this tells us" section above was wrong. It
assumed the resize and reposition both come from `set_overlay_frame` in
`paint_pass`. They don't. The resize is visible because **Chromium itself
renders smaller content**, not because the CALayer frame changed.

There are **two independent code paths** that set the CALayer frame:

1. **`set_overlay_frame()`** (conn.rs:1372) — called from `paint_pass()` every
   frame. Uses `paint_pane()` coordinates which include `pos.left` from the
   split tree. **Correct.**

2. **`update_ca_layer_frame()`** (conn.rs:1331) — called from
   `handle_ca_context()` whenever Chromium sends a `CaContext` message. Uses
   global `metrics` atomics and `pane.col` — has **no knowledge of `pos.left`**.
   Always positions the overlay as if the pane is at the left edge. **Wrong
   after a split.**

The `on_ca_context_id` callback fires on **every GPU frame swap** (traced
through Chromium: `DidSwapBuffersComplete` → `DidReceiveCALayerParams` →
`AcceleratedWidgetCALayerParamsUpdated` → `ca_layer_params_callback_` →
`TsNotifyCAContextId`). This means every time Chromium renders a frame —
including after a resize — it sends a `CaContext` message, which triggers
`update_ca_layer_frame`.

The sequence on split:

1. Split happens. Tree updated. `pos.left` is now correct.
2. `paint_pass` runs (terminal panes reposition correctly). `set_overlay_frame`
   sets the overlay to the **correct** position and size.
3. TUI sends SetOverlay resize → Roamium sends Resize to Chromium.
4. Chromium resizes, renders a new frame at the smaller size. The user sees
   smaller content because Chromium's CALayerHost displays the new smaller
   frame.
5. Chromium fires `on_ca_context_id` → Roamium sends `CaContext` to GUI.
6. `handle_ca_context` → `update_ca_layer_frame` → **overwrites** the frame with
   `origin_x + border_left + pane.col * cell_w`. No `pos.left`. The overlay
   snaps back to the left edge with the correct width.

Step 6 clobbers step 2. The user sees the overlay at the left edge with the
correct (smaller) size.

Pressing ESC triggers another `paint_pass` → `set_overlay_frame` with the
correct `pos.left`. No `CaContext` follows (Chromium isn't resizing), so the
correct position sticks.

On the primary screen, the timing works out so that another `paint_pass` runs
after step 6 (from PaneOutput or other activity), correcting it before the user
notices. On the secondary screen, the display link phase means no additional
`paint_pass` runs until the user interacts.

## Experiments

### Experiment 1: Remove update_ca_layer_frame

#### Description

`update_ca_layer_frame` is the sole cause of the bug. It runs on every
`CaContext` message (every Chromium frame swap) and overwrites the correct
position set by `set_overlay_frame` with a position that has no split tree
awareness.

The fix: delete `update_ca_layer_frame` and its call site. The render pass
already calls `set_overlay_frame` every frame in `paint_pass`, which computes
the correct position from `paint_pane()`. The first frame after the CALayerHost
is created will position the overlay correctly — a one-frame delay that is
imperceptible.

The `handle_ca_context` function still needs to create the layer hierarchy
(flipped layer, positioning layer, CALayerHost) and swap CALayerHost on context
ID changes. It just shouldn't set the frame position.

#### Changes

**`wezboard-gui/src/termsurf/conn.rs`:**

1. Delete the `update_ca_layer_frame` function (lines 1330–1369, both the
   `#[cfg(target_os = "macos")]` implementation and any non-macOS stub).

2. In `handle_ca_context`, remove the call to `update_ca_layer_frame` (line
   1307: `update_ca_layer_frame(pane, root_layer);`). The `root_layer` variable
   is still needed for `get_or_create_overlay` and
   `msg_send![root_layer, bounds]` / `msg_send![root_layer, addSublayer]` in the
   layer creation block. No other code references `update_ca_layer_frame`.

No other files change.

#### Verification

Build:

- [ ] `cd wezboard && cargo build` — compiles without errors.

Edge case checklist (test on both screens):

- [ ] Open a webview — positioned and sized correctly.
- [ ] Split pane to the left — webview repositions to the right immediately.
- [ ] Same test on second screen — webview repositions immediately (the bug).
- [ ] Open a new tab — webview disappears.
- [ ] Switch back — webview at correct position.
- [ ] Resize the window — webview tracks pane position.
- [ ] Close the split pane — webview expands to fill.
- [ ] Open a second window on a second screen, open webview — correct position.
