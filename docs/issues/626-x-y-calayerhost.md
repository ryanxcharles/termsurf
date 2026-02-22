# Issue 626: CALayerHost X/Y Positioning

## Goal

Fix the remaining ~10px Y and ~3px X offset in the CALayerHost browser overlay
so that web content aligns pixel-perfectly with the TUI viewport.

## Background

[Issue 625](625-calayerhost.md) replaced the `FrameSinkVideoCapturer` pipeline
with `CALayerHost`. Instead of capturing IOSurface frames at 120fps and
transferring Mach ports over XPC every frame, Chromium now sends a
`ca_context_id` (uint32) once per tab. The GUI creates a `CALayerHost` sublayer
on the IOSurfaceLayer, and Window Server composites the remote content directly
from GPU VRAM. Zero per-frame IPC, zero pixel copies, zero Metal shader
compositing.

### The positioning saga

The initial CALayerHost implementation (Issue 625 Experiment 2) had the web
content positioned catastrophically wrong — pushed ~400px to the bottom of the
screen with only the top ~10% visible. Six experiments were needed to diagnose
and fix the positioning:

- **Experiment 2:** Core CALayerHost pipeline works, but content is ~400px off.
- **Experiment 3:** Tried flipping Y (`flipped_y = screen_height - y - h`). Zero
  visible effect — the Y flip had no impact at all.
- **Experiment 4:** Added diagnostic logging. Confirmed
  `setProperty("frame",
  frame)` works (readback matches), the function is
  called with valid data, and the parent IOSurfaceLayer is 800×600 points with
  `contentsScale=2.0`. The hardcoded frame test proved the frame property
  controls positioning. Also discovered cell dimensions are in physical pixels
  while CALayer frames use logical points.
- **Experiment 5:** Researched the Chromium source. Found three root causes:
  1. **Missing `geometryFlipped`.** Chromium's CAContext root layer uses
     `geometryFlipped = YES` (Y=0 at top). Chromium's browser process hosts it
     in a `geometryFlipped` layer too. Our IOSurfaceLayer had no
     `geometryFlipped`, causing a full Y-axis inversion.
  2. **Shell window chrome.** The Chromium Profile Server's NSWindow had a title
     bar (~28px) and toolbar (24px) that created phantom offsets in the
     CAContext layer tree, even though the window was hidden.
  3. **Physical pixels vs logical points.** Cell dimensions and screen height
     were in physical pixels (2x on Retina), but CALayer frames use points.
- **Experiment 6:** Applied all three fixes. `geometryFlipped = YES` and
  `anchorPoint = CGPointZero` on the CALayerHost, borderless window with hidden
  toolbar on the Chromium side, pixel-to-point conversion by dividing by
  `contentsScale`. The ~400px offset is gone, but a small residual offset
  remains.

### Current state

The web content renders near the correct position but is offset by approximately
**10px too high** (Y) and **3px too far left** (X). This is close enough to see
that the CALayerHost pipeline works, but the misalignment prevents thorough
testing of:

- Scrolling responsiveness and latency
- Text selection tracking
- Pane resize behavior
- Multiple panes with different profiles
- CALayerHost cleanup on pane close
- Input latency comparison with native Chrome

The offset must be fixed before any of those can be verified.

### How positioning currently works

The TUI (`web` command) sends grid coordinates over XPC:

```
set_overlay: col=1 row=4 w=120 h=35
```

The GUI converts these to a CALayerHost frame in logical points:

```zig
scale = parent.contentsScale   // 2.0 on Retina
x = grid_col * cell_width / scale
y = grid_row * cell_height / scale
w = grid_width * cell_width / scale
h = grid_height * cell_height / scale
host.setProperty("frame", { origin: {x, y}, size: {w, h} })
```

The CALayerHost has `geometryFlipped = YES`, so Y=0 is at the top — matching the
terminal grid's coordinate system. No Y flip is applied.

### Possible causes of the residual offset

- **Grid padding.** The terminal grid may not start at pixel (0, 0) in the
  surface. There could be padding between the surface edge and the first cell.
  If `grid_col=1` maps to a pixel position that assumes no padding, the
  CALayerHost would be offset by the padding amount.
- **Chromium view insets.** Even with a borderless window and hidden toolbar,
  the Chromium content view may have a small inset within the window frame. The
  `RenderWidgetHostViewMac` frame might not start at exactly (0, 0) in the
  contentView.
- **Cell size rounding.** The cell dimensions are integers (physical pixels).
  Dividing by `contentsScale` may introduce fractional point values that don't
  align with the actual grid rendering.

## Experiments

### Experiment 1: Research Electron's CALayerHost handling

Electron embeds Chromium in a normal `BrowserWindow` using stock CALayerHost —
no custom display code. Their CALayerHost positioning works perfectly. This
experiment studies how Electron sets up its NSView/CALayer hierarchy and
positions the CALayerHost to understand what we're doing differently.

#### Research questions

**R1: Electron's NSView hierarchy.**

How does Electron's `BrowserWindow` set up its NSView tree? What view hosts the
web content? Is it a layer-hosting view or a layer-backed view? What is the view
hierarchy from NSWindow down to the CALayerHost?

Look in `vendor/electron/shell/browser/native_window_mac.mm` and related files.

**R2: Electron's CALayerHost setup.**

Does Electron create the CALayerHost itself, or does it inherit Chromium's
default `DisplayCALayerTree` behavior? Does it set `geometryFlipped`,
`anchorPoint`, or any other properties on the host layer? How does the host
layer relate to the content view?

Look in `vendor/electron/` for any CALayerHost references, and in Chromium's
`ui/accelerated_widget_mac/display_ca_layer_tree.mm` for the default behavior.

**R3: How does Chromium's default `DisplayCALayerTree` position the
CALayerHost?**

In stock Chromium (which Electron uses), `DisplayCALayerTree::GotCALayerFrame()`
creates the CALayerHost and adds it to a `maybe_flipped_layer_`. What is
`maybe_flipped_layer_`? How is it configured? What is its relationship to the
NSView's layer? This is the reference implementation that works — we need to
match it.

Look in `chromium/src/ui/accelerated_widget_mac/display_ca_layer_tree.mm` and
`display_ca_layer_tree.h`.

**R4: How does the content view's frame relate to the CAContext geometry?**

In Chromium's normal display path, the content view fills the window (minus any
chrome). The CALayerHost fills the content view. The CAContext from the GPU
process is sized to match. How does this size agreement happen? Is the
CALayerHost frame explicitly set, or does it auto-fill via autoresizing masks?

**R5: What would "no offset" look like for us?**

Given what we learn from R1–R4, what would we need to change so that the
CALayerHost content starts at exactly (0, 0) in the CAContext — with no title
bar, toolbar, or view inset shifting the content? Is it a Chromium-side fix
(make the content view fill the window at origin), a GUI-side fix (account for
the offset), or both?

#### Verification

Research is complete when we can draw a side-by-side comparison:

1. **Electron/Chrome:** Full layer tree from NSWindow → NSView → CALayer →
   CALayerHost, with every `geometryFlipped`, `anchorPoint`, `frame`, and
   autoresizing mask documented.
2. **TermSurf:** Our current layer tree from SurfaceView → IOSurfaceLayer →
   CALayerHost, with the same properties documented.
3. A concrete list of differences that could explain the ~10px Y / ~3px X
   offset.
