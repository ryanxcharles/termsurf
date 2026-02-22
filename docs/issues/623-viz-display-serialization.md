# Issue 623: Viz Display Serialization

## Goal

Confirm or rule out Viz Display serialization as the root cause of the 2fps
degradation when two BrowserContexts both run `requestAnimationFrame` loops. If
confirmed, find a fix. If ruled out, identify what's actually causing it.

## Background

### The problem

Two BrowserContexts in one Chromium process with JavaScript animations degrade
to 2fps. This requires BOTH conditions — neither alone triggers it:

| Configuration                | rAF | FPS     | Experiment |
| ---------------------------- | --- | ------- | ---------- |
| 2 BrowserContexts, 2 windows | yes | 2 + 2   | 621.5      |
| 1 BrowserContext, 2 windows  | yes | 60 + 60 | 622.3      |
| 2 BrowserContexts, CSS only  | no  | 60 + 60 | 621.4      |
| 1 BrowserContext, 1 window   | yes | 60      | 621.1      |

### What 24 experiments have eliminated

Across Issues 620 (15 experiments), 621 (5 experiments), and 622 (4
experiments):

- **Renderer process contention** — each BrowserContext gets its own renderer
  process with its own Blink main thread, compositor thread, and scheduler. No
  cross-process serialization in the renderer layer.
- **BrowserContext-specific frame logic** — "BrowserContext" appears in zero
  files across all of `components/viz/` and `cc/`. The frame pipeline has no
  concept of browser profiles.
- **Compositor/GPU pipeline** — CSS animations generate continuous compositor
  damage through the full viz pipeline at 60fps across two profiles.
- **BeginFrame delivery** — BeginFrames arrive at 60fps to both renderers. The
  renderer receives them but only produces CompositorFrames at ~3fps.
- **Scheduler state machine** — `--enable-main-frame-before-activation` is
  enabled on all renderers, so pipelining should be active.
- **All viz-side throttles** — `kUndrawnFrameLimit` never triggered,
  `BeginFrameTracker` throttle/stop never triggered, `ShouldSendBeginFrame`
  rejections only during init.
- **ExternalBeginFrameSourceMac thrashing** — found and fixed in 620 Exp 13
  (commented out `StopObservingBeginFrames`). Vsync stayed alive but 2fps
  persisted. Symptom, not cause.

### The hypothesis: Viz Display serialization

The `DisplayScheduler` waits for all pending surfaces before drawing
(`HasPendingSurfaces` in `display_damage_tracker.cc:135-170`). Each renderer
blocks on `kMaxPendingSubmitFrames = 1` (`scheduler_state_machine.cc:32`) until
Viz acknowledges the previous frame. The ack arrives only after the Display
draws.

The proposed chain reaction:

1. Both renderers receive BeginFrame simultaneously
2. Both start the BeginMainFrame → rAF → commit → activate → draw cycle
3. Renderer A finishes first, submits CompositorFrame, blocks on ack
4. Display waits for Renderer B (still in its main-thread round-trip)
5. Renderer B finishes, submits, Display draws both, sends acks
6. Both renderers unblock, start next frame — but a vsync was wasted waiting

CSS animations bypass this because the compositor responds to BeginFrame
immediately — no main-thread round-trip, no pending tree, no slow commit cycle.
The Display never waits.

Same-BrowserContext bypasses this because both WebContents share one renderer
process. Their compositor batches both into a single submission.

### The magnitude problem

The hypothesis qualitatively explains all three experimental cases. But there is
a gap: simple cross-process waiting should produce ~30fps (miss every other
frame), not 2fps (miss 29 out of 30). The 2fps implies a ~500ms frame cycle, far
beyond what "wait for the slower renderer" alone can produce.

This suggests either:

1. **A cascading feedback loop** — the initial delay triggers secondary
   throttling mechanisms that amplify the degradation. Missed deadlines cause
   the renderer to be treated as unresponsive, which causes more missed
   deadlines, which triggers more throttling.
2. **The hypothesis is wrong** — Viz Display serialization is not the cause, and
   the real mechanism is something else entirely that happens to correlate with
   separate renderer processes + main-thread involvement.

### Key observations that must be explained

Any correct theory must account for all of these:

1. **Both windows degrade equally** — not just the slower one
2. **The renderer stops producing frames** — `needs_draw=0` at the Display,
   meaning no CompositorFrames arrive (620 Exp 14)
3. **BeginFrames arrive at 60fps** — the Viz→renderer path is healthy
4. **The magnitude is ~2fps** — not 30fps, not 15fps
5. **CSS animations are immune** — despite going through the same Viz ack
   pipeline
6. **Same-BrowserContext JS is immune** — despite using the same GPU/Viz process
7. **Trivial JS triggers it** — a 30-line rAF loop is enough (621 Exp 5)
8. **Pipelining flag is enabled** — `--enable-main-frame-before-activation`
   should allow overlapped work

### Key code locations

| Component                      | File                                                                            | What it does                                                  |
| ------------------------------ | ------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Pending surface wait           | `components/viz/service/display/display_damage_tracker.cc:135-170`              | `HasPendingSurfaces` — Display waits for all surfaces         |
| Display draw decision          | `components/viz/service/display/display_scheduler.cc`                           | `ShouldDraw()`, `OnBeginFrameDeadline()`                      |
| Frame submission backpressure  | `cc/scheduler/scheduler_state_machine.cc:32`                                    | `kMaxPendingSubmitFrames = 1`                                 |
| Draw throttle gate             | `cc/scheduler/scheduler_state_machine.cc:1602-1607`                             | `IsDrawThrottled()` blocks draws, commits, and BeginMainFrame |
| BeginMainFrame gate            | `cc/scheduler/scheduler_state_machine.cc:670-679`                               | `ShouldSendBeginMainFrame()` blocked when draw-throttled      |
| Undrawn frame throttle         | `components/viz/service/frame_sinks/compositor_frame_sink_support.h:81`         | `kUndrawnFrameLimit = 3`                                      |
| Unresponsive client throttle   | `components/viz/service/frame_sinks/begin_frame_tracker.h:23-24`                | `kLimitThrottle = 10`, `kLimitStop = 100`                     |
| CompositorFrame ack            | `components/viz/service/frame_sinks/compositor_frame_sink_support.cc`           | `DidReceiveCompositorFrameAck` flow                           |
| Surface activation             | `components/viz/service/surfaces/surface.cc:702`                                | Ack sent when next frame activates                            |
| Frame sink BeginFrame delivery | `components/viz/service/frame_sinks/compositor_frame_sink_support.cc:1138-1233` | `OnBeginFrame()` with throttle checks                         |
| BeginFrame source fan-out      | `components/viz/common/frame_sinks/begin_frame_source.cc:558-597`               | Flat loop over all observers                                  |

## Approach

Two tracks: empirical experiments to test the hypothesis directly, and source
code research to trace the exact frame lifecycle and find the amplification
mechanism (or the real cause). Start with research to understand the
`HasPendingSurfaces` → ack → unblock chain in detail before modifying code.
