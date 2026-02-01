# 321: Scroll Support

Scroll webview content using mouse wheel and trackpad gestures.

## Status

Not started.

## Product Requirements

Users need to scroll web content in webview panes:

1. **Vertical scroll** — Scroll up and down through page content using mouse
   wheel or two-finger trackpad gesture.

2. **Horizontal scroll** — Scroll left and right for wide content using
   horizontal wheel tilt or two-finger horizontal swipe.

3. **Smooth scrolling** — Trackpad gestures should feel smooth and responsive,
   matching native browser behavior.

4. **Momentum scrolling** — On macOS, trackpad flick gestures should continue
   scrolling with momentum (inertia), then decelerate naturally.

5. **Scroll anywhere** — Scrolling should work regardless of where the cursor is
   positioned within the webview pane.

## Background

### What Works (from Issues 319, 320)

Previous issues established mouse input infrastructure for ts3 webviews:

| Feature                    | Status  | Issue |
| -------------------------- | ------- | ----- |
| Mouse move                 | Working | 319   |
| Left click                 | Working | 319   |
| Hover effects              | Working | 319   |
| Coordinate transform       | Working | 319   |
| Double-click (word select) | Working | 320   |
| Triple-click (line select) | Working | 320   |

### Current Scroll Handling

WezTerm already receives scroll events for terminal panes:

```rust
// From mouseevent.rs - terminal scroll handling
WMEK::VertWheel(amount) => {
    if amount > 0 {
        TMB::WheelUp(amount as usize)
    } else {
        TMB::WheelDown((-amount) as usize)
    }
}
WMEK::HorzWheel(amount) => {
    if amount > 0 {
        TMB::WheelLeft(amount as usize)
    } else {
        TMB::WheelRight((-amount) as usize)
    }
}
```

The `amount` is a signed integer:

- Positive = up/left
- Negative = down/right

### CEF Scroll API

CEF provides `send_mouse_wheel_event` for scroll input:

```rust
// From cef-rs OSR example
let (delta_x, delta_y) = match delta {
    // Line-based scroll (traditional mouse wheel): multiply by 120
    MouseScrollDelta::LineDelta(x, y) => ((x * 120.0) as i32, (y * 120.0) as i32),
    // Pixel-based scroll (trackpad gestures): multiply by 2
    MouseScrollDelta::PixelDelta(pos) => ((pos.x * 2.0) as i32, (pos.y * 2.0) as i32),
};
host.send_mouse_wheel_event(Some(&mouse_event), delta_x, delta_y);
```

Key points:

- CEF expects delta values, not absolute positions
- Line-based scrolling (discrete wheel clicks) uses 120 units per line
- Pixel-based scrolling (smooth trackpad) uses smaller increments
- Both horizontal and vertical deltas can be sent in one call

### Scroll Input Types

| Input Device            | Event Type  | Scroll Style                    |
| ----------------------- | ----------- | ------------------------------- |
| Traditional mouse wheel | Line delta  | Discrete steps (120 units/line) |
| Apple Magic Mouse       | Pixel delta | Smooth gesture-based            |
| Trackpad two-finger     | Pixel delta | Smooth gesture-based            |
| Trackpad with momentum  | Pixel delta | Continues after lift            |

WezTerm's `WMEK::VertWheel` and `WMEK::HorzWheel` events should already
encapsulate both types—the window system translates device input into these
events.

### Architecture Reference

```
Scroll Event Flow:

User scrolls (wheel or gesture)
    │
    ▼
Window System (winit/macOS)
    │
    ▼
WMEK::VertWheel(amount) or WMEK::HorzWheel(amount)
    │
    ▼
mouse_event_impl() in mouseevent.rs
    │
    ▼
handle_webview_mouse_event()
    │
    ├─ [NEEDED] Handle VertWheel/HorzWheel events
    ├─ [NEEDED] Convert amount to CEF delta format
    │
    └─ xpc_manager.send_mouse_wheel(...)  [NEEDED]
            │
            ▼
        XPC to Profile Server
            │
            ▼
        [NEEDED] Handle "mouse_wheel" action
            │
            ▼
        CEF host.send_mouse_wheel_event(delta_x, delta_y)
            │
            ▼
        Page scrolls
```

## Implementation Approach

### 1. Add XPC Method (webview_xpc.rs)

```rust
pub fn send_mouse_wheel(
    &self,
    pane_id: PaneId,
    x: i32,
    y: i32,
    delta_x: i32,
    delta_y: i32,
    modifiers: u32,
) -> bool
```

### 2. Handle Scroll Events (mouseevent.rs)

Add cases for `WMEK::VertWheel` and `WMEK::HorzWheel` in
`handle_webview_mouse_event()`.

### 3. CEF Handler (termsurf-profile/main.rs)

Add handler for "mouse_wheel" action that calls `host.send_mouse_wheel_event()`.

### Delta Conversion

WezTerm provides scroll amounts that need conversion to CEF deltas:

```rust
// Vertical scroll
WMEK::VertWheel(amount) => {
    // amount is typically small integers for line scrolling
    // or larger values for pixel-based trackpad scrolling
    let delta_y = amount * 120; // Scale for CEF
    xpc_manager.send_mouse_wheel(pane_id, cef_x, cef_y, 0, delta_y, 0);
}
```

The exact scaling factor may need tuning based on testing with actual hardware.

## Success Criteria

- [ ] Vertical scroll works with mouse wheel
- [ ] Vertical scroll works with trackpad two-finger gesture
- [ ] Horizontal scroll works
- [ ] Scroll feels smooth (not jerky or too fast/slow)
- [ ] Momentum scrolling works on macOS trackpad
- [ ] Scroll direction matches system preferences (natural vs traditional)

## Next Steps (Other Mouse Input)

After scroll, these features remain for full mouse support:

| Feature         | Priority | Notes                                |
| --------------- | -------- | ------------------------------------ |
| Drag selection  | Medium   | Click-and-drag to select text ranges |
| Modifier keys   | Medium   | Shift-click, Cmd-click, Ctrl-click   |
| Right-click     | Medium   | Context menus                        |
| Middle-click    | Low      | Paste or open in new tab             |
| Cursor feedback | Low      | Change cursor shape over links, text |

## Experiments

_No experiments yet._

## References

- `docs/issues/319-mouse.md` — Basic mouse input (completed)
- `docs/issues/320-double-click.md` — Double/triple click (completed)
- `cef-rs/examples/osr/src/main.rs` — CEF scroll handling reference (lines
  462-466)
- `ts3/wezterm-gui/src/termwindow/mouseevent.rs` — Mouse event handling
- `ts3/wezterm-gui/src/termwindow/webview_xpc.rs` — XPC mouse methods
- `ts3/termsurf-profile/src/main.rs` — CEF event handlers
