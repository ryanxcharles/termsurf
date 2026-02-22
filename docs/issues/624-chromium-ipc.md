# Issue 624: Chromium IPC

## Goal

Understand how Chromium's processes communicate internally — what processes
exist, what IPC mechanisms they use, and specifically how input reaches the
renderer and how rendered frames reach the display. This knowledge will inform
how to replace TermSurf's current XPC message-passing with something faster.

## Background

### The latency problem

TermSurf runs Chromium out-of-process. The GUI (Ghostty fork) communicates with
a Chromium Profile Server over XPC. This works, but every interaction has
visible lag:

```
Mouse event → Zig Surface → XPC to Chromium → Chromium processes input →
renderer paints → compositor composites → capturer captures (timer) →
IOSurface → XPC to GUI → next CVDisplayLink vsync → Metal composites
```

[Issue 619](619-input-latency.md) measured this at 15–25ms average, 1–2 frames
of extra latency versus native Chrome. Three sources: the FrameSinkVideoCapturer
running on its own timer (0–8ms), async XPC dispatch (1–3ms each direction), and
a double-vsync penalty.

### What we tried and abandoned

[Issues 620](620-zig-content-shell.md)–[623](623-viz-display-serialization.md)
spent 25 experiments across four issues trying to run multiple browser profiles
in a single Chromium process. If multiple `BrowserContext`s could coexist at
60fps, there would be no IPC at all — the GUI would host Chromium in-process.

The attempt failed. Two BrowserContexts with JavaScript animations degrade to
2fps. [Issue 621](621-single-process.md) isolated the trigger to JavaScript on
the Blink main thread (CSS animations are immune).
[Issue 622](622-javascript-is-slow.md) proved both conditions are required —
multiple BrowserContexts AND JavaScript.
[Issue 623](623-viz-display-serialization.md) debunked the leading theory (Viz
Display serialization). After 25 experiments, the root cause remains unknown.

### The new direction

Rather than continue debugging the single-process 2fps mystery, we're pursuing
the multi-process architecture that TermSurf already uses — but making it
faster. The key insight from Issue 619's research: **Chrome itself is
multi-process, yet achieves 1-frame latency.** Chrome's browser process,
renderer processes, and GPU/Viz process are all separate — the same kind of
cross-process architecture TermSurf has. Chrome stays fast because its
performance-critical paths use shared memory, not message passing.

Issue 619 identified that Chromium uses shared memory ring buffers for GPU
commands and shared GPU textures (IOSurface) for frame data. Mojo on macOS uses
Mach ports — the same kernel mechanism as XPC. The transport is not the
bottleneck. What matters is what travels over it.

Before we can adopt these patterns, we need to deeply understand how they
actually work in Chromium's codebase.

### What we already know (from Issue 619)

Issue 619's research established:

- **GPU Command Buffer** — renderers write GL-equivalent commands into a shared
  memory ring buffer (`gpu/command_buffer/client/cmd_buffer_helper.h`). Hundreds
  of commands batch before a single IPC notification.
- **CompositorFrames are metadata, not pixels** — a `CompositorFrame` contains
  texture references and draw quads. Zero pixel data crosses the boundary.
- **Mojo uses Mach ports on macOS** — `MOJO_USE_APPLE_CHANNEL` buildflag,
  `channel_mac.cc` implements transport via `mach_msg`.
- **Compositor-thread input handling** — `cc/input/InputHandler` handles scroll
  on the compositor thread without touching the main thread.
- **CALayerParams** — Chrome's normal display path uses `ca_context_id` for
  zero-copy GPU compositing, or `io_surface_mach_port` as a fallback.

But this was a high-level survey. We need to trace the actual code paths.

## Research questions

### 1. What processes exist when viewing a web page?

We know the broad categories (browser, renderer, GPU/Viz) but need the precise
picture:

- Exactly how many processes does Content Shell spawn for one tab? For two tabs?
- Which process is the "browser process" — is it the one that calls
  `ContentMain()`, or does Chromium spawn a separate one?
- Where does the GPU/Viz process get created? Is it always a separate process,
  or can it run in-process?
- Are there other processes (utility, network, audio) relevant to rendering?

### 2. How do they communicate?

The IPC landscape in Chromium is layered and confusing. We need to understand
the stack:

- **Mojo** — Chromium's primary IPC framework. What exactly is it? Message
  pipes, data pipes, shared buffers — how do these map to OS primitives?
- **Legacy IPC** — does any of it remain, or is everything Mojo now?
- **Shared memory** — how does Chromium create and share memory regions across
  processes? What API (`base::SharedMemory`, `base::WritableSharedMemoryRegion`,
  platform-specific)?
- **Mach ports** — how are they used beyond Mojo channels? IOSurface transfer,
  task ports, etc.

### 3. What IPC protocols exist?

- What Mojo interfaces carry rendering-critical messages?
- What is the `viz.mojom.CompositorFrameSink` interface?
- What is the `viz.mojom.DisplayClient` / `viz.mojom.DisplayPrivate` interface?
- What carries input events from browser to renderer?

### 4. Where is shared memory used?

The GPU Command Buffer uses shared memory. What else does?

- **Bitmaps / raster buffers** — are software-rasterized tiles shared via shared
  memory?
- **Input events** — are they sent as Mojo messages or through shared memory?
- **Frame metadata** — is the CompositorFrame itself in shared memory, or
  serialized over a Mojo message pipe?
- **Sync tokens / fences** — are these in shared memory or IPC messages?

### 5. How does user input reach the renderer?

Trace the complete path for a mouse click:

- Where does the browser process receive the OS event?
- How does it decide which renderer gets it?
- What Mojo interface carries the event?
- Does the event go directly to the renderer, or through the GPU/Viz process?
- How does the compositor thread receive it for scroll/selection?
- What is the latency of this path?

### 6. How does the rendered frame reach the display?

Trace the complete path for a rendered pixel:

- Renderer rasterizes into... what? GPU textures? Shared memory bitmaps?
- The CompositorFrame is submitted to... where? The GPU process? The browser
  process?
- How does the GPU/Viz process aggregate frames from multiple renderers?
- How does the final composited result reach the screen on macOS?
- What is `CALayerParams`? Where is it produced and consumed?
- What is a `ca_context_id`? How does `CALayerHost` work?

## Approach

Source code research only — no code changes, no builds. Read the Chromium source
in `chromium/src/` to trace the actual code paths. The goal is a detailed map of
the IPC architecture that we can use to design TermSurf's replacement for XPC
message-passing.
