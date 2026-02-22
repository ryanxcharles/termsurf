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

## Experiments

### Experiment 1: Trace the full frame lifecycle across two renderers

A source code research experiment — no code changes, no builds. Before
instrumenting or modifying Chromium, we need to understand the exact sequence of
events in a single frame cycle when two renderer processes both have active rAF
loops. The goal is to map the complete chain from BeginFrame to ack and identify
where the ~500ms delay accumulates.

#### Question 1: What happens when the Display has pending surfaces?

The `DisplayScheduler` checks `HasPendingSurfaces` before drawing. Trace the
exact behavior when one renderer has submitted a CompositorFrame but the other
has not yet responded to its BeginFrame.

**Where to look:**

- `components/viz/service/display/display_scheduler.cc` —
  `OnBeginFrameDeadline()`, `ShouldDraw()`, and how the deadline timer is set.
  What happens when the deadline fires and a surface is still pending? Does the
  Display draw anyway (without the slow surface), or does it skip the entire
  draw?
- `components/viz/service/display/display_damage_tracker.cc:135-170` —
  `HasPendingSurfaces()` implementation. What counts as "pending"? Is it any
  surface that received a BeginFrame but hasn't submitted a CompositorFrame?
- `components/viz/service/display/display_scheduler.cc` — how the deadline
  duration is calculated. Is it a fixed offset from vsync, or adaptive? Does it
  grow when surfaces are slow?

**Key question:** Does the Display **skip the draw entirely** when surfaces are
pending at the deadline, or does it **draw with stale data** for the missing
surface? If it skips, that's a complete pipeline stall. If it draws with stale
data, the fast renderer should still get acked.

#### Question 2: How does the ack flow back to the renderer?

When the Display draws and the frame is presented, trace the exact path of the
ack back to the renderer's `pending_submit_frames_` counter.

**Where to look:**

- `components/viz/service/display/display.cc` — after `DrawAndSwap()`, how does
  it signal completion?
- `components/viz/service/frame_sinks/compositor_frame_sink_support.cc` —
  `DidReceiveCompositorFrameAck()`. When is this called relative to the draw? Is
  it immediate, or deferred to the next frame?
- `components/viz/service/surfaces/surface.cc:702` — the comment says "ack sent
  when next frame activates." Does this mean a renderer must wait for its OWN
  next frame to activate before receiving the ack for the current one? If so,
  that's a built-in one-frame delay that compounds with cross-surface waiting.
- `cc/scheduler/scheduler.cc` — how does receiving the ack unblock the renderer?
  Does it immediately allow a new BeginMainFrame, or does it wait for the next
  BeginFrame?

**Key question:** Is the ack latency one frame (sent at next draw), two frames
(sent at next surface activation), or variable? If it's two frames and the draw
itself is delayed by `HasPendingSurfaces`, the compounding could explain 2fps.

#### Question 3: What is the renderer doing between BeginFrame and CompositorFrame submission?

When the renderer receives a BeginFrame and `IsDrawThrottled()` is true (because
`pending_submit_frames_ >= 1`), what exactly happens? Does it silently drop the
BeginFrame? Does it send `DidNotProduceFrame`? Does it queue the work?

**Where to look:**

- `cc/scheduler/scheduler.cc` — `OnBeginFrameSourcePausedChanged()`,
  `BeginFrame()` — what does the scheduler do with a BeginFrame when it can't
  send BeginMainFrame?
- `cc/scheduler/scheduler_state_machine.cc` — trace through the full
  `NextAction()` decision tree when `IsDrawThrottled()` is true. What state does
  the scheduler settle into?
- `components/viz/service/frame_sinks/compositor_frame_sink_support.cc` — when
  the renderer responds with `DidNotProduceFrame`, does Viz still count it as
  "pending" for `HasPendingSurfaces`?

**Key question:** If a throttled renderer sends `DidNotProduceFrame`, does the
Display treat that surface as no longer pending (allowing it to draw the other
surface's frame)? Or does it still wait? If `DidNotProduceFrame` clears the
pending state, then the fast renderer should get its ack promptly and shouldn't
degrade. If it doesn't clear it, that's the amplification mechanism.

#### Verification

Research is complete when the full frame lifecycle is mapped:

```
BeginFrame → [renderer receives] → [throttle check] → [BeginMainFrame or drop]
  → [rAF + commit + activate + draw] → [submit CompositorFrame to Viz]
  → [Display checks HasPendingSurfaces] → [draw or skip]
  → [ack sent back] → [renderer unblocks]
```

Each step should have a file path, line number, and the decision logic. The
mapping should answer whether the magnitude problem (2fps vs expected ~30fps) is
explained by compounding delays in the ack chain, or whether we need to look
elsewhere.
