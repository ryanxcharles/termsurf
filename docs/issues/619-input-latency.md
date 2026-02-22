# Issue 619: Input latency

## Goal

Reduce the visible lag between user input (mouse movement, text selection,
scrolling) and the browser's visual response. The goal is to close the
perceptible gap between TermSurf and native Chrome.

## Background

Issue 512 solved vsync micro-stutter (uneven frame cadence) with 120fps
oversampling. The frame cadence is now smooth. But there is a separate problem:
**input-to-display latency**. When selecting text, the selection visibly trails
the cursor. When scrolling, the page visibly lags behind the scroll gesture.
Bounce effects at the top and bottom of the page feel sluggish. The whole
experience feels less refined than native Chrome.

### The round-trip in native Chrome

```
Mouse event → compositor thread → render → display (same vsync)
Total: 0–16ms (one frame)
```

Input received before the vsync deadline appears on that vsync. The compositor
thread can respond to scroll and selection immediately — often within the same
frame — because everything is in-process with a single clock.

### The round-trip in TermSurf

```
Mouse event → Zig Surface → XPC to Chromium → Chromium processes input →
renderer paints → compositor composites → capturer captures (timer) →
IOSurface → XPC to GUI → next CVDisplayLink vsync → Metal composites
```

| Stage                       | Latency    | Notes                                   |
| --------------------------- | ---------- | --------------------------------------- |
| Input → XPC to Chromium     | ~1–3ms     | Async dispatch queue scheduling         |
| Chromium processes input    | ~2–5ms     | Layout, paint, composite                |
| Wait for next capture cycle | **0–8ms**  | Capturer on 120fps timer, not on-demand |
| Captured frame → XPC to GUI | ~1–3ms     | Another async dispatch queue hop        |
| Wait for next vsync         | **0–16ms** | CVDisplayLink tick                      |

Worst case: ~35ms. Average: ~15–25ms. That's 1–2 frames of extra latency versus
native Chrome.

### Three sources of lag

**1. FrameSinkVideoCapturer is a recording API, not a display API.**

The capturer runs on its own 120fps timer and issues `CopyOutputRequest`s
periodically. It does not know that input just arrived and a fresh frame is
urgently needed. After Chromium renders the new frame in response to input, you
wait up to 8ms for the next capture cycle to notice it. In Chrome, input
directly triggers compositor work within the same BeginFrame — no capture delay.

**2. XPC is asynchronous.**

Messages are enqueued on dispatch queues and delivered when the OS scheduler
gets around to it. This cost is paid twice — once for input going to Chromium,
once for the frame coming back. There is no way to make XPC synchronous without
blocking the caller, which would be worse.

**3. The double-vsync penalty.**

In Chrome, input received before the vsync deadline appears on that vsync. In
TermSurf, input has to travel to Chromium, get rendered, get captured, travel
back, and then wait for the _next_ vsync. You effectively always lose at least
one frame compared to Chrome. This is inherent to any out-of-process streaming
architecture.

## How Chrome stays fast across process boundaries

Chrome uses separate processes for rendering and GPU compositing — the same kind
of cross-process architecture that TermSurf has. But Chrome feels responsive
because its performance-critical path does not use message-passing IPC. It uses
shared memory.

### Chrome's process model

| Process           | Role                                                               |
| ----------------- | ------------------------------------------------------------------ |
| **Browser**       | UI chrome, input dispatch, coordination                            |
| **Renderer** (1+) | Blink (DOM, layout, paint) + compositor thread (scroll, animation) |
| **GPU/Viz** (1)   | All GPU calls, display compositing, rasterization                  |

Renderers never touch the GPU directly. Every graphics call crosses a process
boundary to the GPU/Viz process. Yet Chrome still achieves ~1–2 frame latency.

### Shared memory, not message passing

The critical difference from TermSurf's architecture:

**GPU Command Buffer** — Renderers write GL-equivalent commands into a shared
memory ring buffer. Hundreds of commands batch up before a single lightweight
IPC notification tells the GPU process to consume them. No per-call kernel
transition. No serialization overhead.

**CompositorFrames are metadata, not pixels** — When a renderer submits a frame
to Viz, it sends a small struct describing quads that reference textures already
in GPU memory (IOSurface on macOS). The heavy pixel data never crosses the
process boundary — it was rasterized directly into GPU memory via the command
buffer.

**Sync tokens** — Instead of blocking to wait for raster to complete, the
compositor submits frames with non-blocking sync tokens. The GPU resolves them
before drawing. The pipeline never stalls.

**Compositor-thread input handling** — Scroll and selection don't need the main
thread. The compositor thread receives input, applies scroll offsets to the
existing layer tree, and submits a new frame — all without touching JavaScript
or layout. This is why scrolling stays smooth even when JS is blocked.

### TermSurf vs Chrome: the architectural gap

| Aspect                | Chrome                                       | TermSurf                                     |
| --------------------- | -------------------------------------------- | -------------------------------------------- |
| Graphics commands     | Shared memory ring buffer (zero copy)        | N/A (capturer does the rendering)            |
| Frame submission      | Small metadata struct (quads + texture refs) | Full IOSurface Mach port transfer via XPC    |
| Input → compositor    | Mojo to compositor thread (same process)     | XPC to Chromium process (kernel hop)         |
| Frame synchronization | BeginFrame from single vsync clock           | Two independent clocks (120fps oversampling) |
| Scroll/selection      | Compositor thread handles directly           | Full Chromium render + capture round-trip    |

The fundamental gap: TermSurf uses a recording API (`FrameSinkVideoCapturer`) on
top of message-passing IPC (XPC), whereas Chrome uses shared memory command
buffers with zero-copy GPU textures and compositor-driven input. Every input
event in TermSurf requires a full round-trip: XPC out, Chromium render, capture,
XPC back. In Chrome, the compositor thread handles scroll and selection within
the same process, often within the same frame.

### What is fixable (short-term)

The capturer timer is the most actionable target. Chromium's
`FrameSinkVideoCapturer` supports `RequestRefreshFrame()` — it forces an
immediate capture on demand. If we triggered this after receiving input events,
we would eliminate the 0–8ms capture wait. Combined with XPC delivery jitter
reduction (high-priority dispatch queues), we could shave ~5–10ms off the
average round-trip.

### What is not fixable (without in-process embedding)

The XPC async latency and double-vsync penalty are inherent to the
out-of-process streaming architecture. They cannot be eliminated without
in-process Chromium embedding — the long-term endgame described in
`docs/vsync.md`. In-process embedding would give TermSurf access to Chrome's own
compositor thread for input handling, shared memory for frame submission, and a
single BeginFrame clock for synchronization.

## Investigation plan

1. **Measure** — Instrument the pipeline to measure actual input-to-display
   latency. Timestamp mouse events when sent, timestamp when the corresponding
   frame arrives. Identify which stage dominates.
2. **Request-driven capture** — After sending input events to Chromium, send a
   `RequestRefreshFrame()` call to make the capturer produce a frame immediately
   instead of waiting for its timer.
3. **Dispatch queue priority** — Ensure XPC connections on both sides use
   high-priority dispatch queues to minimize scheduling latency.
4. **Evaluate** — Compare TermSurf vs Chrome after optimizations. Determine how
   much of the remaining gap is inherent to out-of-process streaming.
