# 311: Resize Accuracy Issues

## Summary

After implementing debounce-based resize in 309-resize.md and fixing the tab
leak in 310-tabs.md, two resize-related issues remain:

1. **Borders appear after resizing**: When resizing the window, black borders
   often appear at the top/bottom or left/right of the webview. This indicates
   the texture size doesn't match the viewport size.

2. **Webview doesn't fill exact pixel bounds**: The webview matches the nearest
   grid (rows/columns) rather than the exact pixel dimensions of the pane. Extra
   space appears as blank areas because the pane's pixel dimensions rarely align
   perfectly with cell boundaries.

Both issues exist in ts3 but not in ts2. The hypothesis is that ts3's sizing
logic diverged from ts2's during the XPC architecture migration.

## Prior Work

- **309-resize.md**: Implemented debounce pattern with `pending_size`,
  `pending_since`, `last_sent_size` and invalidate callbacks
- **310-tabs.md**: Fixed browser overlay leaking across tabs by filtering by
  `tab_id`

## Research: ts2 vs ts3 Comparison

### ts2: How It Works (Correctly)

**Spawn-time sizing** (`ts2/wezterm-gui/src/termwindow/mod.rs`):

```rust
// Uses render_metrics.cell_size (DPI-aware, always current)
let physical_width = dims.cols * render_metrics.cell_size.width;
let physical_height = dims.viewport_rows * render_metrics.cell_size.height;
let logical_width = physical_width / scale;
let logical_height = physical_height / scale;
```

**Render-time sizing** (`ts2/wezterm-gui/src/termwindow/render/pane.rs`):

```rust
// Called EVERY frame in paint_browser_overlay()
let (x, y, w, h) = calculate_pane_pixel_bounds();  // Exact pixels
let logical_w = w / scale;
let logical_h = h / scale;

// Debounce logic
if target_size != pending_size {
    set_pending_size(target_size);
    mark_resize_time();
}
if time_since_last_resize() >= 30ms {
    browser.resize(logical_w, logical_h);
    // Synchronous CEF update:
    host.was_resized();
    host.invalidate(PaintElementType::default());
    cef::do_message_loop_work();  // Pump message loop
}
```

**Key patterns:**

1. Uses `render_metrics.cell_size` (updated every render, DPI-aware)
2. Calls `calculate_pane_pixel_bounds()` for exact pixel dimensions
3. After resize, pumps CEF message loop synchronously
4. Texture updates immediately (same process)

### ts3: How It Works (Buggy)

**Spawn-time sizing** (`ts3/wezterm-gui/src/termwindow/webview_socket.rs`):

```rust
// Uses GLOBAL STATIC cell size (may be stale!)
let (cell_width, cell_height) = get_cell_size();  // From CELL_WIDTH/CELL_HEIGHT
let physical_width = dims.cols as f32 * cell_width as f32;
let physical_height = dims.viewport_rows as f32 * cell_height as f32;
let logical_w = (physical_width / scale) as u32;
let logical_h = (physical_height / scale) as u32;

xpc_manager.request_profile_spawn(pane_id, url, profile, logical_w, logical_h, scale);
```

**Render-time sizing** (`ts3/wezterm-gui/src/termwindow/render/draw.rs`):

```rust
// Uses pos.pixel_width/height (correct, exact pixels)
let viewport_w = pos.pixel_width as f32;
let viewport_h = pos.pixel_height as f32;
let logical_w = (viewport_w / scale) as u32;
let logical_h = (viewport_h / scale) as u32;

// Debounce logic (similar to ts2)
if state.last_sent_size == Some(target_size) {
    // Fast path - already sent
} else if target_size changed {
    pending_size = target_size;
    pending_since = now;
} else if elapsed >= 30ms {
    xpc_manager.send_resize(logical_w, logical_h);  // Async!
    last_sent_size = target_size;
}
```

**Problems identified:**

| Issue            | ts2                                 | ts3                           |
| ---------------- | ----------------------------------- | ----------------------------- |
| Cell size source | `render_metrics.cell_size` (fresh)  | Global static (stale)         |
| Resize delivery  | Synchronous (same process)          | Async (XPC to profile server) |
| Post-resize sync | `was_resized()` + message loop pump | None (fire and forget)        |
| Texture update   | Immediate                           | Waits for IOSurface via XPC   |

## Root Cause Analysis

### Issue 1: Borders After Resizing

**Timeline of the bug:**

```
Frame 1: Pane is 800px wide, texture is 800px
Frame 2-5: User drags to resize to 700px
  - Viewport immediately becomes 700px (from pos.pixel_width)
  - Debounce timer starts
  - Texture still 800px (waiting for XPC)
  - Render: 800px texture in 700px viewport → clipped, but no border yet
Frame 6-8: Debounce waiting (< 30ms)
  - Viewport: 700px
  - Texture: 800px
  - Still rendering old texture
Frame 9: 30ms elapsed, debounce fires
  - Sends XPC resize command for 700px
  - Texture: STILL 800px (profile server hasn't responded)
  - Viewport: 700px
Frame 10-15: Waiting for profile server...
  - Texture: 800px (stale)
  - Viewport: 700px
  - Border visible if texture doesn't cover viewport
Frame 16: Profile server sends new 700px IOSurface
  - Finally matches!
```

**Root cause:** Async XPC communication means texture lags behind viewport.
During the lag, size mismatch creates visible borders.

### Issue 2: Blank Space at Edges

**The mismatch:**

```
Spawn time (webview_socket.rs):
  cell_width = 8 (from global static, maybe stale)
  cols = 100
  physical = 100 * 8 = 800px

Render time (draw.rs):
  pos.pixel_width = 803px (actual pane dimensions from layout)

Result: Texture is 800px, viewport is 803px → 3px blank on right
```

**Root cause:** Spawn uses grid-based calculation with potentially stale cell
size. Render uses exact pixel dimensions. These diverge.

## Hypotheses

### Hypothesis 1: Use Exact Pixels at Spawn Time

Instead of calculating `cols × cell_width`, use `pos.pixel_width` directly at
spawn time. This requires access to the positioned pane layout from the socket
handler.

**Challenge:** The socket handler runs in a background thread and may not have
access to the current layout. Need to either:

- Pass pixel dimensions from the `web` command caller
- Query the layout from the socket handler
- Accept grid-based sizing at spawn but immediately resize to exact pixels

### Hypothesis 2: Add Post-Debounce Synchronization

After sending resize via XPC, continue invalidating until the received texture
size matches the expected size.

```rust
// Track what we've received, not just what we've sent
if received_surface.width == expected_width
   && received_surface.height == expected_height {
    // Sizes match, stop invalidating
} else {
    // Keep invalidating until texture catches up
    window.invalidate();
}
```

### Hypothesis 3: Send Exact Pixel Dimensions in Resize

Currently, resize sends logical dimensions calculated from viewport:

```rust
let logical_w = (viewport_w / scale) as u32;
```

The `as u32` truncation may lose precision. If viewport is 803px and scale is
2.0, logical becomes 401 (truncated from 401.5). CEF renders at 401 logical =
802 physical. Result: 1px border.

**Fix:** Use consistent rounding or send physical pixels and let profile server
handle scaling.

### Hypothesis 4: Match ts2's Exact Sizing Flow

Port ts2's sizing logic more faithfully:

1. At spawn: Calculate size the same way ts2 does in `handle_web_open()`
2. At resize: Use `render_metrics.cell_size` instead of global static
3. After resize: Add acknowledgment from profile server before clearing
   `pending_size`

## Proposed Experiments

### Experiment 1: Diagnostic Logging

Add detailed logging to understand the exact mismatch:

- Log spawn-time dimensions (what we request)
- Log received texture dimensions (what profile server sends)
- Log viewport dimensions (what we render to)
- Log any mismatches

### Experiment 2: Use Exact Pixels at Spawn

Modify `webview_socket.rs` to use exact pixel dimensions instead of grid-based:

- Get `pos.pixel_width/height` from the positioned pane
- Send physical pixels to profile server
- Profile server converts to logical using scale

### Experiment 3: Continue Invalidating Until Match

Modify debounce logic to track received size:

- Add `last_received_size` to state
- Update it when new IOSurface arrives via XPC
- Only stop invalidating when received matches expected

### Experiment 4: Synchronous Resize Acknowledgment

Add XPC response from profile server after resize completes:

- Profile server sends "resize_complete" with actual dimensions
- GUI waits for acknowledgment before rendering at new size
- Eliminates the async lag problem

## Files to Modify

| File                                               | Changes                                |
| -------------------------------------------------- | -------------------------------------- |
| `ts3/wezterm-gui/src/termwindow/webview_socket.rs` | Fix spawn-time sizing                  |
| `ts3/wezterm-gui/src/termwindow/render/draw.rs`    | Add synchronization logic              |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs`    | Track received vs sent sizes           |
| `ts3/termsurf-profile/src/main.rs`                 | Send resize acknowledgment (if needed) |

## Success Criteria

- [ ] No black borders appear during or after window resize
- [ ] Webview fills exact pixel bounds of pane (no blank space at edges)
- [ ] Resize behavior matches ts2's accuracy
- [ ] No regression in debounce functionality (still batches rapid resizes)
