# Issue 625: CALayerHost

## Goal

Replace the `FrameSinkVideoCapturer` with `CALayerHost` so that browser panes
display with the same latency as native Chrome — zero per-frame IPC, zero
application-side compositing, Window Server composites directly from GPU VRAM.

## Background

### The current pipeline

TermSurf runs Chromium out-of-process. A Chromium Profile Server renders web
content and streams frames to the GUI (a Ghostty fork) over XPC. The current
frame delivery path:

```
Chromium renders → compositor composites → FrameSinkVideoCapturer (timer) →
CopyOutputRequest → IOSurface → Mach port via XPC → GUI imports IOSurface →
Metal shader composites → CVDisplayLink vsync → screen
```

This adds 15–25ms of latency versus native Chrome
([Issue 619](619-input-latency.md)). The single biggest contributor is the
`FrameSinkVideoCapturer` — a recording API that runs on its own timer, adding
~5-7ms per frame from timer wait and GPU readback. On top of that, every frame
requires a Mach port transfer over XPC.

### What we learned in Issue 624

[Issue 624](624-chromium-ipc.md) mapped Chromium's full IPC architecture across
three experiments:

1. **Chrome's normal display path uses `CALayerHost`.** The GPU process creates
   a `CAContext`, sends a `ca_context_id` (uint32) once, and the browser process
   creates a `CALayerHost` pointing to that ID. Window Server composites the GPU
   process's CALayer tree directly from VRAM. Zero per-frame IPC, zero pixel
   copies.

2. **Our Chromium Profile Server already produces `CALayerParams` every frame.**
   The capturer is purely observational — the normal display path runs alongside
   it. We've been ignoring `CALayerParams` output at
   `RenderWidgetHostViewMac::AcceleratedWidgetCALayerParamsUpdated()`.

3. **Electron validates this approach.** Electron's normal `BrowserWindow` uses
   stock Chromium — CALayerHost, unmodified, zero custom display code.
   Electron's off-screen rendering mode uses the same `FrameSinkVideoCapturer`
   that TermSurf currently uses, with the same latency penalty. CALayerHost is
   the architecturally correct way to display Chromium content.

### Why CALayerHost

The `ca_context_id` is a uint32 that identifies a `CAContext` in the GPU
process. `CALayerHost` is a `CALayer` subclass that displays a remote
`CAContext` from another process — Window Server handles the compositing. This
is the same mechanism Chrome uses between its own GPU process and browser
process. Adding one more process boundary (Chromium server → TermSurf GUI) is
the same pattern.

**What it eliminates:**

- `FrameSinkVideoCapturer` and `ShellVideoConsumer` (~460 lines)
- Per-frame `CopyOutputRequest` GPU readback
- Per-frame IOSurface Mach port transfer over XPC
- Per-frame Metal texture import and shader compositing in the GUI
- The ~5-7ms capturer timer latency

**What it adds:**

- One XPC message per tab containing a uint32 `ca_context_id` (sent once, not
  per frame)
- A `CALayerHost` sublayer in the GUI, positioned at browser pane coordinates
- Dimming of inactive browser panes via a sibling `CALayer` with
  semi-transparent background

**Architectural change:** Browser pane content moves out of the Metal shader
pipeline. Terminal panes render via Metal (unchanged). Browser panes render via
`CALayerHost` — Window Server composites them. Both coexist as sibling
`CALayer`s in the same NSView layer tree. The GUI still controls positioning and
z-order, but does not touch browser pixels.

### What needs to change

Two sides:

**Chromium Profile Server** (in `chromium/src/`):

- Intercept `CALayerParams` at
  `RenderWidgetHostViewMac::AcceleratedWidgetCALayerParamsUpdated()`
- Extract `ca_context_id` from the params
- Send it over XPC to the GUI (once per tab, re-send on context change)
- Remove `ShellVideoConsumer` and all capturer setup

**TermSurf GUI** (in `gui/`):

- Receive `ca_context_id` over XPC
- Create `CALayerHost` with that `contextId`
- Add as sublayer of the window's content view, positioned at the browser pane's
  pixel coordinates
- Update position/size on pane resize and split changes
- Add dimming overlay `CALayer` for inactive browser panes
- Remove the IOSurface overlay pipeline from the Metal renderer (the pink
  texture proof-of-concept from [Issue 602](602-pink-texture.md), the IOSurface
  import from [Issue 603](603-box-demo.md))

### Open questions

Before implementing, we need to research the TermSurf codebase to understand:

1. **Where does the GUI currently receive IOSurface frames?** Trace the XPC
   message path from reception to Metal rendering. What code handles the Mach
   port, imports the IOSurface, and passes it to the renderer?

2. **How does the Metal renderer currently composite browser content?** What
   shaders, pipeline state, and draw calls are involved? What gets deleted?

3. **Where in the Zig/Swift layer hierarchy should `CALayerHost` live?** The GUI
   uses a `CAMetalLayer` for terminal rendering. `CALayerHost` needs to be a
   sibling or sublayer. What's the current NSView/CALayer structure?

4. **How does pane positioning work?** When a split pane resizes, how do pixel
   coordinates propagate? The `CALayerHost` needs to track these coordinates.

5. **Can `CALayerHost` be created from Zig?** Zig already calls Objective-C
   runtime APIs for Metal. `CALayerHost` is another `CALayer` subclass — same
   pattern. But we need to verify the specific API calls.

## Experiments

### Experiment 1: Map the current pipeline

A source code research experiment — no code changes, no builds. Read the
TermSurf GUI and Chromium Profile Server source to trace the complete frame
delivery pipeline from end to end. The goal is to know exactly what code to
change before writing a single line.

#### Q1: XPC frame reception in the GUI

Trace the path from XPC message arrival to Metal rendering. Where does the GUI
receive IOSurface Mach ports, and how do they reach the renderer?

**Where to look:**

- `gui/src/apprt/embedded.zig` — C API exports. Search for XPC-related functions
  and `overlay` / `iosurface` / `mach_port` references.
- `gui/src/Surface.zig` — the core surface. Search for overlay state, IOSurface
  fields, and any XPC callback handling.
- `gui/src/renderer/Metal.zig` — the Metal renderer. Search for IOSurface
  import, overlay texture, and how the browser frame gets composited.
- `gui/macos/` — Swift code. Search for XPC message handling, Mach port
  extraction, and calls into the Zig C API.

**Deliverable:** A sequence diagram from XPC message arrival to Metal draw call,
with every function, file, and line number labeled. Identify exactly what gets
deleted.

#### Q2: Metal overlay pipeline

Map the Metal shader pipeline that composites browser content. What render pass,
pipeline state, vertex buffer, and fragment shader are involved?

**Where to look:**

- `gui/src/renderer/Metal.zig` — search for `overlay`, `iosurface`, `browser`,
  or any render pass that handles non-terminal content.
- `gui/src/renderer/metal/` — shader files, pipeline definitions. What shaders
  draw the browser overlay?
- Search for `IOSurface` across all of `gui/src/` — every reference needs to be
  cataloged for removal.

**Deliverable:** A list of every Metal resource (pipeline state, shader
function, vertex buffer, texture, render pass) used for the browser overlay.
Each item should be annotated with: keep, delete, or modify.

#### Q3: NSView and CALayer hierarchy

Map the current layer tree. Where does `CAMetalLayer` live? What is the NSView
structure? Where should `CALayerHost` be inserted?

**Where to look:**

- `gui/macos/Sources/` — Swift app code. Search for `NSView`, `CALayer`,
  `CAMetalLayer`, `contentView`, `wantsLayer`, `makeBackingLayer`.
- `gui/src/apprt/embedded.zig` — how the Zig side references the view/layer.
- Look at how Ghostty's Metal renderer gets its `CAMetalLayer` — this is the
  layer that `CALayerHost` needs to sit on top of.

**Deliverable:** A layer tree diagram showing the current NSView/CALayer
hierarchy, with the proposed insertion point for `CALayerHost` marked.

#### Q4: Pane coordinate propagation

Understand how the GUI tracks browser pane pixel coordinates. The `CALayerHost`
frame (position + size) must update when panes resize.

**Where to look:**

- `gui/src/Surface.zig` — how does the surface know its pixel bounds within the
  window? Search for `screen`, `padding`, `grid`, `size`, `viewport`.
- `gui/src/apprt/embedded.zig` — how do resize events propagate?
- The current overlay pipeline — how does it know where to position the browser
  content? Whatever coordinate system it uses is what `CALayerHost` needs.

**Deliverable:** The exact fields and functions that provide browser pane pixel
coordinates (origin x, origin y, width, height) relative to the window.

#### Q5: Chromium server side — CALayerParams interception

Map the Chromium Profile Server's current frame sending code and identify where
to intercept `CALayerParams` instead.

**Where to look:**

- `chromium/src/content/shell/browser/shell_video_consumer.cc` and `.h` — the
  current capturer. Trace how it sends IOSurface Mach ports over XPC.
- `chromium/src/content/shell/browser/shell_browser_main_parts.cc` — where the
  capturer is created and connected to XPC. This is where `CALayerParams`
  interception would go instead.
- `chromium/src/content/browser/renderer_host/render_widget_host_view_mac.mm` —
  `AcceleratedWidgetCALayerParamsUpdated()` at line 156. How to hook into this
  callback for our Content Shell.

**Deliverable:** A before/after outline showing: (1) what currently sends frames
over XPC, (2) what replaces it (CALayerParams interception), and (3) what gets
deleted.

#### Verification

Research is complete when we can draw the full before/after picture:

1. **Before:** Complete trace from Chromium capturer → XPC → GUI → Metal draw
   call, with every file and function labeled.
2. **After:** Complete trace from Chromium `CALayerParams` callback → XPC
   (ca_context_id once) → GUI → `CALayerHost` sublayer, with proposed code
   changes for each step.
3. A definitive layer tree diagram showing where `CALayerHost` sits relative to
   `CAMetalLayer`.
4. The coordinate system for positioning `CALayerHost` at browser pane bounds.
