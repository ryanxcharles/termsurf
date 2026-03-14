# Issue 749: Initial overlay flash on wrong side of split

## Goal

When opening a browser overlay in a split pane, the webview appears at its
correct position immediately — no flash on the wrong side.

## Background

### Two code paths set the overlay frame

There are two functions that set the CALayer frame for browser overlays:

1. **`update_ca_layer_frame()`** (conn.rs ~1331) — runs once when the
   CALayerHost is first created inside `handle_ca_context()`. Computes position
   using `origin_x + border_left + pane.col * cell_w`. This formula has no
   knowledge of the split tree — it doesn't know `pos.left`, so it always
   positions relative to the window's left edge.

2. **`set_overlay_frame()`** (conn.rs ~1372) — runs every frame from
   `paint_pass()`. Receives coordinates from `paint_pane()` which includes
   `pos.left` and `pos.top` from the live split tree. This is the correct,
   authoritative position.

### The flash

When opening `web google.com` in a right-side split pane:

1. Chromium starts rendering and sends a `CaContext` message
2. `handle_ca_context()` creates the CALayerHost and calls
   `update_ca_layer_frame()`, which places the overlay at the LEFT side of the
   window (no split tree awareness)
3. On the next frame, `paint_pass()` calls `set_overlay_frame()` with the
   correct right-side coordinates
4. The overlay jumps from left to right — visible as a brief flash

### Prior work

Issue 746 established the render-pass-based positioning system
(`set_overlay_frame()` called from `paint_pass()`). Issue 747 fixed a bug where
`update_ca_layer_frame()` was being called on EVERY `CaContext` message (not
just first creation), which caused overlays to snap back to the wrong position
after splits on secondary screens. The fix moved the call inside the
first-creation branch only.

Issue 747's fix was correct — `update_ca_layer_frame()` should not run on every
frame swap. But it left the first-creation call in place, which is what causes
this initial flash.

### The fix

There should be exactly one place that computes overlay position:
`set_overlay_frame()`. Having a second function that duplicates the calculation
is a maintenance hazard — the next time the formula changes, someone has to
remember to update both.

Remove `update_ca_layer_frame()` entirely. At first creation time, don't set a
frame at all — the CALayerHost defaults to zero-size at origin, which is
invisible. On the very next frame, `paint_pass()` calls `set_overlay_frame()`
and places it correctly. This is how terminal panes work — they don't
pre-compute position at creation time, they get rendered at the correct position
on the next paint pass.

The zero-size initial frame won't affect the browser. The CALayer frame is
purely a display property — it controls where the composited GPU layer appears
in the window. The browser's viewport size is controlled by the `Resize`
protobuf message over the Unix socket, which is independent. Chromium is already
rendering at its correct size before the `CaContext` message arrives.

## Experiments

### Experiment 1: Remove update_ca_layer_frame

#### Description

Delete `update_ca_layer_frame()` and its call site. The CALayerHost will be
created with no explicit frame (defaults to zero-rect), and
`set_overlay_frame()` from `paint_pass()` will place it correctly on the next
frame.

#### Changes

**`wezboard/wezboard-gui/src/termsurf/conn.rs`**

1. Remove the call to `update_ca_layer_frame(pane, root_layer)` at ~line 1286
   (inside the first-creation branch of `handle_ca_context()`).

2. Delete the entire `update_ca_layer_frame()` function (~lines 1330-1369).

No other changes needed. `set_overlay_frame()` and the `paint_pass()` call site
remain untouched.

#### Verification

```bash
scripts/build.sh wezboard
```

| # | Test                    | Steps                                          | Expected                              |
| - | ----------------------- | ---------------------------------------------- | ------------------------------------- |
| 1 | No flash in right split | Split pane, run `web google.com` in right pane | Webview appears on right, no flash    |
| 2 | No flash in left split  | Split pane, run `web google.com` in left pane  | Webview appears on left, no flash     |
| 3 | No flash without split  | Single pane, run `web google.com`              | Webview appears normally              |
| 4 | Resize after split      | Open webview in right split, resize window     | Webview repositions correctly         |
| 5 | Split after webview     | Open webview, then split pane                  | Webview resizes/repositions correctly |
