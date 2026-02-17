# Issue 514: Mouse Input Forwarding

## Goal

Click on links, scroll up and down on a page, and see the cursor change when
hovering over links. Full mouse control in browse mode.

## Background

Issues 509–512 built a complete Chromium streaming pipeline: webpages render at
120fps in terminal panes via IOSurface overlay. Issue 513 added bidirectional
mode synchronization — the window knows when a pane is in browse mode and can
intercept input. But there is no input forwarding yet. The browser renders but
cannot be interacted with.

Mouse input is the highest-impact next step. Clicking links, scrolling content,
and seeing cursor changes are the minimum interactions needed for a usable
browser pane.

## Architecture

### Event flow

```
NSEvent (mouse click/move/scroll in browse mode)
    ↓
CompositorXPC local event monitor (main thread)
    ├─ Pane in browse mode → forward via XPC
    └─ Not browsing → pass through to terminal
    ↓
XPC message to Chromium Profile Server
    ↓
shell_browser_main_parts.cc (XPC handler on control_connection_)
    ↓
RenderWidgetHost::ForwardMouseEvent() / ForwardWheelEvent()
    ↓
Chromium renderer process
    ↓
Blink handles the event (click, hover, scroll)
```

### Coordinate transformation

The overlay occupies a grid region of the terminal pane. Mouse coordinates from
NSEvent are in window coordinates (pixels, Retina-scaled). These must be
transformed to coordinates relative to the overlay's origin:

```
overlay_x = event.locationInWindow.x - overlay_pixel_origin_x
overlay_y = overlay_pixel_height - (event.locationInWindow.y - overlay_pixel_origin_y)
```

The Y-axis is flipped: macOS has origin at bottom-left, Chromium has origin at
top-left. The overlay's pixel origin and size are derived from its grid
coordinates and cell size (already computed in `handleSetOverlay`).

Hit testing: if the transformed coordinates fall outside (0, 0) to
(overlay_width, overlay_height), the event is outside the overlay and should
pass through to the terminal.

### XPC message format

Mouse events are sent as XPC dictionaries from CompositorXPC to the Chromium
Profile Server on the existing control connection:

```
{
    action: "mouse_event",
    pane_id: "<uuid>",
    type: "move" | "down" | "up" | "entered" | "exited",
    x: <float>,           // pixels, relative to overlay origin
    y: <float>,           // pixels, relative to overlay origin
    button: "left" | "right" | "middle" | "none",
    click_count: <int>,   // 1 = single, 2 = double, 3 = triple
    modifiers: <uint64>,  // bitmask: shift=1, ctrl=2, alt=4, meta=8
}

{
    action: "scroll_event",
    pane_id: "<uuid>",
    x: <float>,           // cursor position, overlay-relative
    y: <float>,
    delta_x: <float>,     // scroll amount (pixels, continuous)
    delta_y: <float>,
    modifiers: <uint64>,
}
```

### Cursor feedback

Chromium changes the cursor based on what's under the mouse (pointer for links,
text cursor for input fields, etc.). The cursor change needs to propagate back
from the Chromium Profile Server to the window.

Chromium's `RenderWidgetHostViewMac` normally handles this via `UpdateCursor()`.
Since we don't have a real view, we need to observe cursor changes and send them
back via XPC so CompositorXPC can set the cursor on the NSWindow.

### Chromium-side injection

The Chromium Profile Server receives mouse events via XPC and injects them into
the rendering pipeline:

```cpp
#include "third_party/blink/public/common/input/web_mouse_event.h"

// In the XPC handler:
blink::WebMouseEvent mouse_event(
    blink::WebInputEvent::Type::kMouseDown,  // or kMouseMove, kMouseUp
    gfx::PointF(x, y),                       // position in widget
    gfx::PointF(screen_x, screen_y),         // position on screen
    blink::WebPointerProperties::Button::kLeft,
    click_count,
    modifiers,
    base::TimeTicks::Now());

shell->web_contents()
    ->GetRenderWidgetHostView()
    ->GetRenderWidgetHost()
    ->ForwardMouseEvent(mouse_event);
```

For scroll events:

```cpp
#include "third_party/blink/public/common/input/web_mouse_wheel_event.h"

blink::WebMouseWheelEvent wheel_event(
    blink::WebInputEvent::Type::kMouseWheel,
    modifiers,
    base::TimeTicks::Now());
wheel_event.SetPositionInWidget(x, y);
wheel_event.delta_x = delta_x;
wheel_event.delta_y = delta_y;
wheel_event.phase = blink::WebMouseWheelEvent::kPhaseBegan;

shell->web_contents()
    ->GetRenderWidgetHostView()
    ->GetRenderWidgetHost()
    ->ForwardWheelEvent(wheel_event);
```

### XPC performance

XPC Mach port transfers are sub-millisecond (proven across Issues 403–414). The
current pipeline already delivers 120 IOSurface frames per second over XPC
(Issue 512). Mouse events are tiny dictionaries (~100 bytes) compared to Mach
port transfers — delivery latency is negligible.

Issue 345 measured mouse move events traversing GUI → XPC → CEF profile server.
The p50 frame interval was identical with and without mouse input (18.7ms vs
17.4ms). The p95 spike (34ms → 79ms) was caused by CEF message loop contention
in the old ts3 architecture, not XPC latency. The Chromium Content API used in
ts5 does not have this contention — `ForwardMouseEvent` is a simple IPC call to
the renderer process, not a synchronous block on the browser message loop.

Mouse moves at display refresh rate (60–120Hz) generate 60–120 XPC messages per
second — far less traffic than the 120fps IOSurface stream already running.

## Ideas for Experiments

The capabilities needed to satisfy the goal — roughly in order of impact:

- **Clicks** — The simplest mouse event. A single point, no continuous tracking.
  Forward left/right mouse down/up from CompositorXPC to the Chromium Profile
  Server via XPC, inject as `blink::WebMouseEvent`, call `ForwardMouseEvent()`.
- **Movement and hover** — Mouse tracking enables CSS `:hover`, tooltips, and
  cursor feedback. Forward `mouseMoved` and track enter/exit for the overlay
  region.
- **Scrolling** — Forward scroll wheel events (`scrollingDeltaX`/`deltaY`) as
  `blink::WebMouseWheelEvent`. Handle both discrete (mouse wheel) and continuous
  (trackpad) scrolling with momentum phases.
- **Cursor feedback** — Propagate cursor type changes (pointer, I-beam, arrow)
  from the Chromium renderer back to the window via XPC so `NSCursor` updates.
