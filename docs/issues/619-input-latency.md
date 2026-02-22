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

### What is fixable

The capturer timer is the most actionable target. Chromium's
`FrameSinkVideoCapturer` supports `RequestRefreshFrame()` — it forces an
immediate capture on demand. If we triggered this after receiving input events,
we would eliminate the 0–8ms capture wait. Combined with XPC delivery jitter
reduction (high-priority dispatch queues), we could shave ~5–10ms off the
average round-trip.

The XPC async latency and double-vsync penalty are inherent to the
out-of-process streaming architecture. They cannot be eliminated without
in-process Chromium embedding (the long-term endgame described in
`docs/vsync.md`).

## Architecture

The investigation should proceed in stages:

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
