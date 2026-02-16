# Issue 512: Vsync

## Background

Issues 509–511 built a complete streaming pipeline: Chromium renders webpages,
streams IOSurface frames at 60fps over XPC, and the Metal renderer composites
them as overlays inside terminal panes. The pipeline supports multiple profiles,
multiple panes, server reuse, resize, and clean lifecycle management.

Both Chromium and the renderer report 60fps. But when comparing TermSurf
side-by-side with native Chromium, TermSurf looks noticeably choppier. The
average framerate is correct, but the visual cadence is uneven — micro-stutters
that the human eye detects even when the numbers look fine.

## Problem

The pipeline has two independent 60fps clocks that are not synchronized.

### Clock 1: Chromium's FrameSinkVideoCapturer

The capturer runs on its own timer, configured in `shell_video_consumer.cc`:

```cpp
capturer_->SetMinCapturePeriod(base::Milliseconds(16));
capturer_->SetAutoThrottlingEnabled(false);
```

This produces frames at ~60fps on Chromium's internal clock. Frames are pushed
to the compositor unconditionally — `OnFrameCaptured` calls `Done()` immediately
to return the buffer to Chromium's pool, with no backpressure from the renderer.

### Clock 2: Ghostty's Metal renderer

The renderer runs on a CVDisplayLink (macOS display vsync). When vsync is
enabled, only display-link-driven `drawFrame(true)` calls actually render. All
other `drawFrame(false)` calls are gated by `hasVsync()` and return immediately.

### The desynchronization

Even though both clocks average 60fps, their ticks drift relative to each other:

- Sometimes two captured frames arrive between two display refreshes. One frame
  is dropped (the old IOSurface is CFReleased, the new one CFRetained). Wasted
  work.
- Sometimes no captured frame arrives between two display refreshes. The
  renderer composites the same IOSurface again. Duplicate frame.

The result is micro-stutter — uneven frame intervals even at a correct average
framerate. XPC message delivery adds further jitter from scheduling latency.

## Additional issues found during analysis

### `needs_redraw` does not account for IOSurface changes

This may be the larger problem. In `generic.zig`, the display-link-driven
`drawFrame` checks:

```zig
const needs_redraw =
    size_changed or
    self.cells_rebuilt or
    self.hasAnimations() or
    sync;
```

When a new IOSurface arrives from Chromium:

- `size_changed` — false (the terminal didn't resize)
- `cells_rebuilt` — false (no terminal cell content changed)
- `hasAnimations()` — false (no custom shaders)
- `sync` — false (display link frames pass `sync=false`)

So `needs_redraw` is false, and the renderer calls `presentLastTarget()` — it
re-presents the previous frame instead of compositing the new IOSurface.

The `setOverlayIOSurface` call does invoke `queueRender()`, which notifies the
wakeup async. But when vsync is active, the wakeup path calls
`drawFrame(false)`, which is a no-op because `hasVsync()` returns true.

In practice, the overlay still renders because the `web` TUI is actively running
in the pane (ratatui draws the URL bar, status bar, cursor blink), which keeps
`cells_rebuilt` true on most frames. But if the TUI reaches a steady state, new
IOSurface frames would be silently missed until something else triggers a
redraw. This explains the uneven visual cadence — some frames render the new
overlay, some don't, depending on whether the terminal happened to have cell
changes at that moment.

### No backpressure from renderer to capturer

Chromium's `OnFrameCaptured` calls `Done()` immediately, returning the IOSurface
buffer to Chromium's pool. If Chromium reuses that buffer before the next
`drawFrame`, the IOSurface content may be mid-write when the Metal renderer
samples it — a potential tearing source.

### Single IOSurface pointer, no buffering

The overlay uses a single `overlay_iosurface` pointer. When Chromium sends
frames faster than the renderer consumes them, intermediate frames are silently
dropped (CFRelease old, CFRetain new in `setOverlayIOSurface`). There is no
double or triple buffering for the overlay — only the terminal's swap chain has
triple buffering.

## Ideas for fixing

### 1. Fix `needs_redraw` for overlay changes

The simplest fix with the highest impact. Add a flag like
`overlay_surface_changed` that is set in `setOverlayIOSurface` and checked in
the `needs_redraw` condition. This ensures every new IOSurface triggers a redraw
on the next display link tick.

This alone would fix the frame-skipping problem. It does not fix the two-clock
desynchronization, but it ensures every captured frame that arrives before a
vsync is composited on that vsync.

### 2. Increase capture rate to 120fps

Change `SetMinCapturePeriod` from 16ms to 8ms (120fps). With the capturer
producing frames at 2x the display rate, there is always a fresh frame available
at every vsync. The maximum age of the displayed frame drops from ~16ms to ~8ms,
smoothing out the visual cadence.

Cost: doubled capture work and XPC traffic (120 IOSurface Mach port transfers
per second per pane instead of 60). The frames themselves are zero-copy GPU
memory, so the overhead is in the capture scheduling and XPC messaging, not in
pixel copying.

This pairs well with fix #1. Without #1, increasing the capture rate just
produces more frames that get silently skipped.

### 3. Demand-driven capture (pull model)

Instead of the capturer pushing frames on its own timer, the renderer requests a
new frame after each vsync. Chromium's `FrameSinkVideoCapturer` supports
`RequestRefreshFrame()` — it forces the capturer to produce a frame on demand.
The consumer would set `SetMinCapturePeriod(0)` (unlimited rate) and call
`RequestRefreshFrame()` after each display refresh.

The challenge: the signaling path is renderer → Zig → Swift → XPC → Chromium
server → capturer. This round-trip adds latency that might make per-frame pull
impractical for 60fps. Each request needs to complete in under 16ms to stay
ahead of the next vsync.

This is the theoretically correct solution but the most complex to implement.

### 4. In-process Chromium (the endgame)

When Chromium is embedded directly via the Content API, the renderer can use the
same display link to drive both terminal and browser rendering. No XPC latency,
no independent clocks, no wasted frames. The streaming architecture was always a
stepping stone — this is what eliminates the vsync problem permanently.
