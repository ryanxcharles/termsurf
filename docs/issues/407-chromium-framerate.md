# Issue 406: Chromium High-Framerate Proof of Concept

## Goal

Prove that Chromium can render web content off-screen and deliver frames to a
Swift Metal window at 120fps or higher via IOSurface. This must be validated
before forking Ghostty, because if Chromium cannot hit the target framerate, the
entire ts4 architecture falls apart.

## Why This Matters

Issue 405 chose to fork Ghostty with an out-of-process Chromium browser. The
terminal side is proven — Ghostty renders at 60fps natively. The browser side is
the unknown. Can Chromium render off-screen fast enough?

Electron achieves 200+ fps rendering web content into a window. But Electron
owns the window and uses Chromium's native compositor. We are asking Chromium to
render off-screen to an IOSurface that gets sent to a foreign process. This adds
overhead:

1. Off-screen rendering (no direct-to-display path)
2. IOSurface creation and Mach port transfer
3. Cross-process XPC messaging
4. Metal texture import and compositing in the Swift window

Each of these is fast individually (Issue 403 measured compositing at 0.04ms),
but we have not measured them with real Chromium rendering producing frames
continuously at high rates.

## The Demo

A WebGL spinning cube rendered by Chromium off-screen, composited into a Swift
Metal window. The cube is deliberately simple — the point is to measure the
frame delivery pipeline, not Chromium's rendering performance.

### index.html

A single HTML file with an inline WebGL (or WebGPU) program:

- A colored cube rotating at 1 revolution per second (1 Hz)
- `requestAnimationFrame` loop driving rendering
- A frame counter overlay showing the current FPS
- Canvas size matches the window dimensions
- No external dependencies — fully offline, fully self-contained

The 1 Hz rotation rate is chosen so that visual smoothness is obvious to the
eye. At 60fps the cube moves 6 degrees per frame. At 120fps it moves 3 degrees.
At 240fps it moves 1.5 degrees. The difference is visible.

### Chromium wrapper (C++)

A C++ process that embeds Chromium and renders off-screen:

- Loads `index.html` from a local file path
- Chromium renders to an IOSurface via off-screen rendering
- Each new frame: create Mach port from IOSurface, send via XPC
- Must not throttle to 60fps — let `requestAnimationFrame` run as fast as
  Chromium will allow

### Swift window

A minimal Swift window (similar to the ts4 Phase 2 window from Issue 403):

- NSWindow + CAMetalLayer
- CVDisplayLink or Metal display link for refresh
- Receives IOSurface Mach ports via XPC
- Imports IOSurface, creates Metal texture, renders full-screen quad
- Displays an FPS counter (frames received per second from Chromium)
- Logs frame timing: time between consecutive frame arrivals

## What We Are Measuring

| Metric                      | Target     | Description                                    |
| --------------------------- | ---------- | ---------------------------------------------- |
| Frame delivery rate         | ≥120 fps   | Frames received by Swift window per second     |
| Frame-to-frame interval     | ≤8.3 ms    | Time between consecutive frame arrivals        |
| Render-to-display latency   | ≤16 ms     | Time from Chromium render to screen present    |
| Mach port transfer overhead | ≤0.1 ms    | Time for IOSurface Mach port XPC send/receive  |
| Metal import overhead       | ≤0.5 ms    | Time for IOSurfaceLookupFromMachPort + texture |
| CPU usage (Chromium)        | Reasonable | Should not peg a core at 100%                  |
| CPU usage (Swift window)    | ≤10%       | Compositor should be lightweight               |

The primary question is whether Chromium's off-screen rendering path can sustain
120+ fps frame delivery. Everything downstream (XPC, IOSurface, Metal import,
compositing) is already proven fast from Issue 403.

### Stretch goals

- **240 fps** — can Chromium deliver frames at 4x the typical display rate?
- **Variable refresh** — does the pipeline work with ProMotion displays
  (adaptive 24–120 Hz)?
- **Frame pacing** — are frames evenly spaced or bursty?

## Chromium Embedding Options

There are several ways to embed Chromium in a C++ process. Each has different
trade-offs for off-screen rendering performance.

### Option 1: CEF (Chromium Embedded Framework)

CEF is what ts3 uses via `cef-rs`. It provides a stable C API for embedding
Chromium with off-screen rendering support.

**Off-screen rendering path:**

1. Create `CefBrowserHost` with `SetWindowlessFrameRate(rate)` — can request up
   to 240 fps (the CEF maximum)
2. Implement `CefRenderHandler::OnPaint()` — called with a
   `shared_texture_enabled` IOSurface handle when a new frame is ready
3. Create Mach port from IOSurface, send via XPC

**Advantages:**

- Stable API, well documented
- `shared_texture_enabled` gives direct IOSurface access (zero-copy)
- `SetWindowlessFrameRate()` controls the target frame rate
- ts3 already has working CEF integration via `cef-rs`

**Disadvantages:**

- CEF is a large binary (~300 MB framework)
- `OnPaint()` callback may be throttled by CEF's compositor
- `SetWindowlessFrameRate()` maximum is 240 fps (configurable)

### Option 2: Chromium Content API

The Content API is Chromium's internal embedding API. It is lower-level than CEF
and gives more control over the rendering pipeline.

**Advantages:**

- Direct access to Chromium's compositor
- Can potentially bypass frame rate limits
- Smaller binary than CEF (no CEF wrapper overhead)

**Disadvantages:**

- Unstable API — changes with every Chromium release
- No off-screen rendering convenience like CEF's `OnPaint()`
- Must manage the Chromium browser process lifecycle manually
- Build complexity is extreme (full Chromium checkout + build)

### Option 3: WebView2 / WKWebView (rejected)

- WebView2 is Windows-only
- WKWebView was rejected in ts1 due to API limitations
- Neither provides off-screen rendering to IOSurface with Mach port export

### Recommendation: Start with CEF

CEF is the pragmatic choice. It has a working off-screen rendering path with
IOSurface support, a configurable frame rate up to 240 fps, and ts3 already has
integration code. If CEF's frame rate is insufficient, the Content API is a
fallback — but that is a much larger undertaking.

## Architecture

```
┌─────────────────────────────────┐
│  Swift Window Process           │
│  ├── NSWindow + CAMetalLayer    │
│  ├── CVDisplayLink (120 Hz+)    │
│  ├── XPC listener               │
│  ├── IOSurface import           │
│  ├── Metal texture composite    │
│  └── FPS counter overlay        │
│            │ XPC                │
│            ▼                    │
│  C++ Chromium Process (CEF)     │
│  ├── CefApp + CefBrowserHost   │
│  ├── Off-screen rendering       │
│  │   (shared_texture_enabled)   │
│  ├── OnPaint → IOSurface        │
│  ├── IOSurfaceCreateMachPort    │
│  └── XPC send to Swift window   │
│            │                    │
│            ▼                    │
│  index.html (local file)        │
│  ├── WebGL spinning cube        │
│  ├── requestAnimationFrame loop │
│  └── FPS counter overlay        │
└─────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Create the WebGL spinning cube

Write `index.html` with a WebGL spinning cube. Test it in a regular browser
first to confirm it renders correctly and reports its own FPS.

- [x] Create `index.html` with WebGL context
- [ ] Draw a colored cube with per-face colors
- [ ] Rotate at 1 revolution per second (1 Hz)
- [ ] `requestAnimationFrame` loop
- [ ] FPS counter overlay (rendered in the page)
- [ ] Verify in Safari/Chrome: smooth rotation, correct FPS

### Phase 2: CEF off-screen renderer (C++)

Build a C++ process that loads `index.html` via CEF and renders off-screen.

- [ ] Set up CEF with off-screen rendering enabled
- [ ] Enable `shared_texture_enabled` for IOSurface output
- [ ] Set `SetWindowlessFrameRate(240)` for maximum frame rate
- [ ] Implement `CefRenderHandler::OnPaint()` to capture IOSurface
- [ ] Create Mach port from IOSurface in `OnPaint()`
- [ ] Set up XPC listener (named Mach service via launchd)
- [ ] Send frame messages via XPC:
      `{ action: "frame", iosurface_port, width,
      height }`
- [ ] Log frame timing: timestamp of each `OnPaint()` call

### Phase 3: Swift window receiver

Build a minimal Swift window that receives and displays frames from the CEF
process.

- [ ] NSWindow + CAMetalLayer (reuse from Issue 403 Phase 2)
- [ ] XPC client connecting to CEF process
- [ ] Receive frame messages, extract Mach port
- [ ] `IOSurfaceLookupFromMachPort` → `MTLTexture`
- [ ] Render full-screen quad with the Chromium texture
- [ ] FPS counter: count frames received per second
- [ ] Log frame-to-frame intervals
- [ ] Deallocate Mach ports after import

### Phase 4: Measure and analyze

Run the system and collect performance data.

- [ ] Measure frame delivery rate (frames per second at Swift window)
- [ ] Measure frame-to-frame interval distribution (mean, p50, p95, p99)
- [ ] Measure render-to-display latency
- [ ] Measure CPU usage of both processes
- [ ] Test at different `SetWindowlessFrameRate` values (60, 120, 240)
- [ ] Test with different window sizes (small, large, fullscreen)
- [ ] Test on different hardware (if available)
- [ ] Compare WebGL FPS reported in `index.html` vs frames received by Swift
- [ ] Document findings in this issue

## Success Criteria

1. **Minimum viable:** 60 fps frame delivery from Chromium to Swift window. The
   spinning cube is visually smooth. This matches typical display refresh.

2. **Target:** 120 fps frame delivery. This matches ProMotion displays and
   proves the pipeline has headroom beyond 60 fps.

3. **Stretch:** 240 fps frame delivery. This matches Electron-class performance
   and proves the off-screen rendering path adds negligible overhead.

If we cannot achieve 60 fps, the off-screen Chromium approach is not viable and
we need to reconsider the architecture (e.g., in-process Chromium compositor, or
a different browser engine).

## Relationship to Other Issues

| Issue | Relationship                                                 |
| ----- | ------------------------------------------------------------ |
| 403   | Proved IOSurface/XPC compositing at 60fps with colored rects |
| 404   | Selected Ghostty as the terminal emulator                    |
| 405   | Chose Ghostty fork + out-of-process Chromium architecture    |
| 406   | This issue — proves Chromium can deliver frames fast enough  |

This is a prerequisite for the Ghostty fork work. If this fails, we revisit the
architecture. If this succeeds, we proceed with confidence.

## Notes

### Why not scroll google.com?

A WebGL spinning cube is better than scrolling a real webpage because:

1. **Repeatable.** The cube spins at a fixed rate. Scrolling depends on network,
   page complexity, and user input timing.
2. **Offline.** No network dependency. Works in airplane mode.
3. **Measurable.** The rotation rate is a built-in clock. If the cube completes
   one revolution per second, we know the animation loop is running at the
   correct rate. Frame drops are visible as jerky motion.
4. **Simple.** The WebGL workload is trivial — the bottleneck will be the frame
   delivery pipeline, not the rendering workload. This isolates what we are
   trying to measure.
5. **Scalable.** We can increase complexity later (add more objects, use WebGPU
   compute shaders) to stress-test the pipeline.

### Why measure above 60 fps?

Most displays refresh at 60 Hz, so 60 fps seems sufficient. But:

1. **ProMotion.** Apple's ProMotion displays (MacBook Pro, iPad Pro) support up
   to 120 Hz. Users with these displays expect 120 fps.
2. **Headroom.** If the pipeline maxes out at 62 fps, any additional overhead
   (real webpage rendering, input handling, resize) will drop it below 60. We
   want headroom.
3. **Electron baseline.** Electron delivers 200+ fps for simple content. If we
   are significantly slower, users will notice.
4. **Future displays.** 240 Hz displays exist on gaming monitors and are coming
   to more devices.

### CEF `SetWindowlessFrameRate`

CEF's off-screen rendering has a configurable frame rate via
`CefBrowserHost::SetWindowlessFrameRate(int frame_rate)`. The default is 30 fps.
The maximum is implementation-defined but typically 240 fps. Setting this to 240
tells CEF's compositor to deliver frames as fast as possible, up to that limit.

Note: The actual frame rate depends on the content. If the WebGL animation calls
`requestAnimationFrame`, CEF will try to match the requested frame rate. If the
content is static, CEF only sends frames when something changes.
