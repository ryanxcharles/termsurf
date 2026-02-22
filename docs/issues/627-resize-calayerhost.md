# Issue 627: CALayerHost Resize

## Goal

Make the browser overlay resize correctly when the user resizes the window or
pane. The overlay should track the TUI viewport at all times, matching the
behavior that worked flawlessly before the CALayerHost migration.

## Background

### How we got here

[Issue 625](625-calayerhost.md) replaced the `FrameSinkVideoCapturer` pipeline
with `CALayerHost`. Instead of capturing IOSurface frames at 120fps and
transferring Mach ports over XPC every frame, Chromium now sends a
`ca_context_id` (uint32) once per tab. The GUI creates a `CALayerHost` sublayer
on the IOSurfaceLayer, and Window Server composites the remote content directly
from GPU VRAM. Zero per-frame IPC, zero pixel copies.

[Issue 626](626-x-y-calayerhost.md) fixed the X/Y positioning of the CALayerHost
overlay. Six experiments revealed two coordinate system bugs: a missing
intermediate flipped layer and a bottom-origin Y coordinate system on the
IOSurfaceLayer. The overlay is now pixel-perfect at initial placement.

### What broke

Before CALayerHost, resize worked flawlessly. The full chain was in place:

1. User resizes window/pane
2. TUI detects the new terminal dimensions
3. TUI sends `set_overlay` XPC with updated grid coordinates and dimensions
4. GUI receives `set_overlay`, stores new grid coordinates, computes pixel
   dimensions, and forwards the resize to Chromium
5. Chromium renders at the new size
6. The Metal shader composited the IOSurface at the new grid coordinates every
   frame in `drawFrame()`

Step 6 is what changed. The old IOSurface pipeline re-read the overlay grid
coordinates every frame and rendered the Metal shader at the current position
and size. Resize was automatic — update the coordinates and the next frame
renders correctly.

With CALayerHost, there is no per-frame rendering. The `flipped_layer` has a
static frame set once by `updateCALayerHostFrame()`. When the pane resizes, the
XPC messages still flow (steps 1–5 still work), but the `flipped_layer` frame is
not updated to reflect the new dimensions.

### The resize chain today

```
TUI sends set_overlay (new grid coords)
    │
    ▼
xpc.zig handleSetOverlay() — stores grid coords, computes pixel dims
    │
    ▼
Surface.setOverlay() — updates renderer overlay_grid_* fields
    │
    ▼
renderer.updateCALayerHostFrame() — sets flipped_layer.frame
    │
    ▼
Metal.updateCALayerHostFrame() — converts grid→points, Y-flip, sets frame
```

The chain looks correct on paper — `setOverlay()` calls
`updateCALayerHostFrame()` which should update the `flipped_layer` frame. But
something in this path fails on resize. Either the `flipped_layer` pointer is
null at that point, the parent bounds are stale, or the new coordinates don't
produce the correct frame.

### Current layer tree (from Issue 626)

```
IOSurfaceLayer (geometryFlipped=false, Y=0 at bottom)
└─ flipped_layer (geometryFlipped=YES, anchorPoint=zero, auto-fill mask,
│                  explicit frame at overlay rect)
   └─ CALayerHost (anchorPoint=zero, pinned top-left via mask, at origin)
```

The `flipped_layer` frame is set via:

```zig
y_from_top = grid_row * cell_height / scale + padding_top / scale
y = parent_bounds.height - y_from_top - h
```

On resize, `parent_bounds.height` changes (the IOSurfaceLayer resizes with the
window). The grid coordinates, cell dimensions, and padding may also change. All
of these must be current when `updateCALayerHostFrame()` runs.
