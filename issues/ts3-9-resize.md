# TS3-9: Dynamic Webview Resize (Continued)

Continuation of resize work from [ts3-8-resize.md](./ts3-8-resize.md). The basic
resize pipeline is now functional but exhibits inconsistent behavior.

## Goal

When the user resizes a terminal window or splits a pane, the webview should
dynamically resize to match the new pane dimensions. The resized content should:

1. Fill the pane completely (no gaps, no overflow)
2. Render at correct resolution (not stretched or squished)
3. Maintain crisp text (proper Retina scaling)
4. Respond reliably to all resize events

## Progress Made

### Working

1. **Single source of truth for textures** — The render loop now reads directly
   from `XpcManager::received_surfaces` instead of the disconnected
   `WebviewOverlayState::overlays`. New textures from resize are immediately
   available for rendering.

2. **Bidirectional XPC communication** — GUI can send `resize_browser` commands
   to the profile server. Profile server handles resize and sends new textures
   back.

3. **Resize command pathway** — The full command flow works:
   ```
   GUI detects resize → sends resize_browser via XPC →
   profile calls was_resized() + invalidate() → CEF re-renders →
   profile sends display_surface → GUI receives new texture
   ```

4. **Debounce logic** — 30ms settle delay prevents flooding the profile with
   resize commands during rapid window dragging.

5. **Scale factor conversion** — Logical dimensions (physical / scale) are sent
   to CEF, which expects DIP coordinates.

### Commits

- `de932e372` — Implement webview resize via bidirectional XPC
- `579039b2b` — Design experiment 4: single source of truth
- `e09a93628` — Design experiment 3: fix scale division
- `deaa638bd` — Mark experiment 2 as failed: wrong scale
- `3b5c99d27` — Design experiment 2: fix XPC handler order
- `f5f53aff8` — Mark experiment 1 as failed: XPC order bug

## What Has Failed

Despite the pipeline being functional, resize behavior is **inconsistent**:

1. **Resize doesn't always trigger** — Sometimes changing the window size causes
   no resize. No predictable pattern for when it works vs doesn't.

2. **Stretched appearance** — Sometimes the webview appears visibly stretched,
   indicating the texture dimensions don't match the viewport dimensions.

3. **Doesn't fill pane** — Sometimes the webview doesn't fill the pane
   dimensions correctly, leaving visible gaps or overflowing the bounds.

4. **Unpredictable** — The same resize action might work correctly one time and
   fail the next.

## Top Hypothesis: Scale Factor Inconsistency

**Initial spawn** uses the pane's DPI from the Mux:

```rust
// webview_socket.rs - spawn_browser handler
let scale = dims.dpi as f32 / 72.0;
```

**Resize** uses the window's DPI from TermWindow:

```rust
// draw.rs - render loop
let scale = self.dimensions.dpi as f32 / 72.0;
```

If `dims.dpi` (pane) differs from `self.dimensions.dpi` (window), the logical
dimensions sent for resize won't match what was used for initial spawn. This
causes the profile to create an IOSurface at the wrong size.

**Evidence:** The stretching and incorrect fill suggest dimension mismatches
rather than complete pipeline failure.

## Other Hypotheses

### 1. Texture/Viewport Transition Period

When resize occurs, there's a window where the old texture is rendered at the
new viewport size:

```
t0: Pane resizes → viewport changes immediately
t1: Resize command sent (after 30ms debounce)
t2: Profile receives, browser resizes
t3: CEF re-renders
t4: New texture sent via XPC
t5: GUI receives and renders new texture
```

During t0-t4, the old texture is stretched to fit the new viewport. If any step
fails or is delayed, stretching persists.

### 2. Debounce Logic Not Triggering

The debounce in `check_and_send_resize` requires:

1. The render loop to run
2. 30ms to elapse since the size changed
3. The size to be different from `last_sent_size`

Problems:

- If terminal content is static (no cursor blink, no animations), the render
  loop might not run frequently
- `last_sent_size` is updated optimistically before confirming the resize
  succeeded
- If resize fails, we don't retry

### 3. Render Loop Timing

Resize detection only happens inside `render_webview_overlays_webgpu`. This
function is only called when the window is being painted. If:

- Terminal content is static
- No cursor blinking
- No animations or updates

...the render loop might not run at the right time to detect size changes.

### 4. Viewport Position Calculation Drift

The viewport is calculated from:

```rust
let x = pos.left as f32 * cell_width + border.left;
let y = pos.top as f32 * cell_height + tab_bar_height + border.top;
let w = pos.pixel_width as f32;
let h = pos.pixel_height as f32;
```

If any of these change without triggering a recalculation:

- Tab bar visibility changes
- Cell size changes
- Border dimensions change

...the viewport won't align with the actual pane position.

### 5. Race Condition in Texture Updates

The XPC handler updates `received_surfaces` while the render loop might be
mid-render using the old texture. While this shouldn't cause stretching (just a
one-frame delay), there might be edge cases where dimensions are read
inconsistently.

### 6. CEF Not Actually Resizing

The profile server calls:

```rust
host.was_resized();
host.invalidate(PaintElementType::default());
```

But we haven't verified that CEF actually re-renders at the new size. Possible
issues:

- `view_rect()` might return stale dimensions
- The browser might not honor the resize immediately
- There might be a minimum time between resize calls

## Files Involved

| File                                               | Role                                                      |
| -------------------------------------------------- | --------------------------------------------------------- |
| `ts3/wezterm-gui/src/termwindow/render/draw.rs`    | Resize detection, viewport calculation, texture rendering |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs`    | XPC manager, debounce logic, send_resize                  |
| `ts3/wezterm-gui/src/termwindow/webview_socket.rs` | Initial spawn, pane dimension lookup                      |
| `ts3/termsurf-profile/src/main.rs`                 | Resize handler, CEF browser resize, texture sending       |

## Next Steps

1. **Add diagnostic logging** — Compare scale factors between initial spawn and
   resize to verify hypothesis #1

2. **Force render loop** — Ensure the render loop runs after resize events by
   calling `window.invalidate()`

3. **Verify CEF resize** — Add logging in profile server to confirm
   `view_rect()` returns updated dimensions after `was_resized()`

4. **Check debounce state** — Log when debounce prevents resize and when it
   allows resize through

5. **Unify scale factor source** — Use the same DPI source for both initial
   spawn and resize

---

## Experiments

### Experiment 1: Diagnostic Logging

**Status:** PENDING

**Goal:** Add comprehensive logging throughout the resize pipeline to pinpoint
the true cause of inconsistent resize behavior.

#### The Problem

Resize behavior is unpredictable:

- Sometimes resize triggers, sometimes it doesn't
- Sometimes the webview appears stretched
- Sometimes it doesn't fill the pane correctly
- The same action may work one time and fail the next

We have multiple hypotheses but no data to confirm which is correct. Before
attempting fixes, we need visibility into what's actually happening.

#### Logging Strategy

Add logging at 8 key points in the resize pipeline:

| # | Location | Purpose |
|---|----------|---------|
| 1 | Initial spawn | Capture baseline dimensions |
| 2 | Render loop layout | See pane dimensions from layout |
| 3 | Debounce logic | See when resize is sent vs blocked |
| 4 | Profile resize handler | Confirm receipt of resize command |
| 5 | CEF view_rect | What dimensions CEF thinks it has |
| 6 | Texture sending | Actual IOSurface dimensions |
| 7 | Texture receiving | What GUI receives via XPC |
| 8 | Texture rendering | Compare texture vs viewport |

#### Changes

**1. Initial Spawn (webview_socket.rs)**

After calculating dimensions in `spawn_browser` handler:

```rust
log::info!(
    "[SPAWN] pane={} cols={} rows={} cell={}x{} physical={}x{} dpi={} scale={:.2} logical={}x{}",
    pane_id,
    dims.cols,
    dims.viewport_rows,
    cell_width,
    cell_height,
    physical_width,
    physical_height,
    dims.dpi,
    scale,
    lw,
    lh
);
```

**2. Render Loop Layout (draw.rs)**

After finding pane position, before resize check:

```rust
log::info!(
    "[LAYOUT] pane={} pos.left={} pos.top={} pos.pixel={}x{} cell={}x{} window.dpi={}",
    pane_id,
    pos.left,
    pos.top,
    pos.pixel_width,
    pos.pixel_height,
    self.render_metrics.cell_size.width,
    self.render_metrics.cell_size.height,
    self.dimensions.dpi
);
```

**3. Debounce Logic (webview_xpc.rs)**

At the start of `check_and_send_resize`:

```rust
log::info!(
    "[DEBOUNCE] pane={} current={}x{} last_sent={:?} pending={:?}",
    pane_id,
    width,
    height,
    state.last_sent_size,
    state.pending_resize.map(|(w, h, t)| (w, h, t.elapsed().as_millis()))
);
```

When resize is actually sent:

```rust
log::info!("[DEBOUNCE] pane={} SENDING {}x{}", pane_id, w, h);
```

When resize is skipped (add new log):

```rust
// If size unchanged from last_sent
log::info!("[DEBOUNCE] pane={} SKIP size unchanged", pane_id);

// If still waiting for settle delay
log::info!(
    "[DEBOUNCE] pane={} WAIT {}ms remaining",
    pane_id,
    (SETTLE_DELAY - time.elapsed()).as_millis()
);
```

**4. Profile Resize Handler (termsurf-profile/src/main.rs)**

When resize command is received:

```rust
println!(
    "[RESIZE-RX] width={} height={} prev_state={}x{}",
    width,
    height,
    bs.width.load(Ordering::Relaxed),
    bs.height.load(Ordering::Relaxed)
);
```

After calling `was_resized()`:

```rust
println!("[RESIZE-RX] called was_resized() and invalidate()");
```

**5. CEF view_rect (termsurf-profile/src/main.rs)**

In the `view_rect` callback:

```rust
println!(
    "[VIEW_RECT] returning {}x{}",
    w,
    h
);
```

**6. Texture Sending (termsurf-profile/src/main.rs)**

In `on_accelerated_paint` when sending:

```rust
println!(
    "[TEXTURE-TX] handle={:p} iosurface={}x{} view_rect={}x{}",
    handle,
    info.width,
    info.height,
    self.state.width.load(Ordering::Relaxed),
    self.state.height.load(Ordering::Relaxed)
);
```

**7. Texture Receiving (webview_xpc.rs)**

In the XPC handler for `display_surface`:

```rust
log::info!(
    "[TEXTURE-RX] pane={} mach_port={} size={}x{}",
    pane_id,
    port,
    width,
    height
);
```

**8. Texture Rendering (draw.rs)**

Before importing the texture:

```rust
log::info!(
    "[RENDER] pane={} texture={}x{} viewport={}x{} match={}",
    pane_id,
    surface.width,
    surface.height,
    viewport_w as u32,
    viewport_h as u32,
    surface.width == viewport_w as u32 && surface.height == viewport_h as u32
);
```

#### Files to Modify

| File | Changes |
|------|---------|
| `ts3/wezterm-gui/src/termwindow/webview_socket.rs` | Add [SPAWN] log |
| `ts3/wezterm-gui/src/termwindow/render/draw.rs` | Add [LAYOUT] and [RENDER] logs |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs` | Add [DEBOUNCE] and [TEXTURE-RX] logs |
| `ts3/termsurf-profile/src/main.rs` | Add [RESIZE-RX], [VIEW_RECT], [TEXTURE-TX] logs |

#### Verification

```bash
cd ts3 && ./scripts/build-debug.sh --open

# Test 1: Initial spawn
web google.com
cat /tmp/termsurf-gui.log | grep -E "\[SPAWN\]|\[LAYOUT\]|\[RENDER\]"
cat /tmp/termsurf-profile-*.log | grep -E "\[VIEW_RECT\]|\[TEXTURE-TX\]"
# Expected: See baseline dimensions, verify texture matches viewport

# Test 2: Split pane (Cmd+Shift+D)
cat /tmp/termsurf-gui.log | grep -E "\[DEBOUNCE\]|\[TEXTURE-RX\]"
cat /tmp/termsurf-profile-*.log | grep -E "\[RESIZE-RX\]|\[VIEW_RECT\]"
# Expected: See resize command sent, received, and new texture

# Test 3: Drag window edge
# Watch logs in real-time:
tail -f /tmp/termsurf-gui.log | grep -E "\[DEBOUNCE\]|\[RENDER\]"
# Expected: See debounce behavior, texture/viewport comparison
```

#### What Each Log Reveals

| Issue | Log to Check | What to Look For |
|-------|--------------|------------------|
| Resize doesn't trigger | [DEBOUNCE] | SKIP or WAIT instead of SENDING |
| Stretched appearance | [RENDER] | texture size != viewport size |
| Doesn't fill pane | [LAYOUT] vs [SPAWN] | pixel dimensions differ |
| Profile not resizing | [RESIZE-RX] | Missing or wrong dimensions |
| CEF not updating | [VIEW_RECT] | Returns old dimensions after resize |
| Wrong texture sent | [TEXTURE-TX] | iosurface size != view_rect size |

#### Success Criteria

- [ ] All 8 logging points are implemented
- [ ] Logs are parseable with grep patterns
- [ ] Can trace a complete resize from detection to render
- [ ] Can identify WHERE in the pipeline failures occur
- [ ] Have data to inform Experiment 2 (the actual fix)
