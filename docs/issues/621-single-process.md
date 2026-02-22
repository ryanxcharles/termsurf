# Issue 621: Single Process Multi-Profile Performance

## Goal

Make two browser profiles (BrowserContext instances) render at 60fps in a single
Chromium process. Issue 620 proved the architecture works — a thin C++ shim
drives the Content API through a C function boundary — but complex pages
(google.com) degrade to 2fps when a second BrowserContext exists. This issue
investigates and fixes that degradation.

## Background

Issue 620 built a minimal Chromium embedder (3 files, ~190 lines) that loads web
pages through the Content API via a C function boundary. 15 experiments
systematically narrowed a 2fps rendering throttle:

### What works

| Configuration        | FPS   | Issue 620 Experiments |
| -------------------- | ----- | --------------------- |
| 1 profile, 1 window  | 60fps | 1, 10                 |
| 1 profile, 2 windows | 60fps | 11                    |

### What doesn't work

| Configuration                | FPS   | Issue 620 Experiments |
| ---------------------------- | ----- | --------------------- |
| 2 profiles, any window count | 2fps  | 2–9, 12               |
| 2 profiles, DDG + google.com | mixed | 14–15                 |

Experiments 14–15 revealed that the 2fps is page-dependent: lite.duckduckgo.com
renders at 60fps while google.com renders at 2fps, regardless of which profile
is created first. This was consistent across 6 runs in both URL orderings.

### What was eliminated in Issue 620

- **C API wrappers** — identical 2fps with and without them (Exp 8 vs 9)
- **Callback/lifecycle machinery** — identical 2fps (Exp 7 vs 8)
- **Direct WebContents vs Shell::CreateNewWindow** — identical (Exp 5–7)
- **Custom launcher vs stock shell_main_mac.cc** — identical (Exp 8)
- **Creation order** — 5 runs, always google slow, DDG fast (Exp 15)
- **ExternalBeginFrameSourceMac thrashing** — found in Exp 12, fixed in Exp 13,
  but 2fps persisted
- **BeginFrameTracker throttle/stop** — never fired (0 events in Exp 14)
- **ShouldSendBeginFrame gate** — 9 rejections in 90s, all during init (Exp 14)
- **DisplayScheduler ShouldDraw conditions** — output_surface_lost, visible,
  root_frame_missing all healthy; only `needs_draw=0` fails (Exp 14)

### Prior art: Issue 413

Issue 413 (ts4 era) found the same boundary: two WebContents from different
BrowserContexts = 2fps, two WebContents from the same BrowserContext = 60fps.
The conclusion at the time was that multi-profile-in-one-process doesn't work
and each profile needs its own process. This issue challenges that conclusion by
investigating whether the degradation can be fixed.

## Chromium internals research

The following architecture was mapped during Issue 620's investigation.

### Vsync and frame delivery chain

```
CADisplayLinkMac / CVDisplayLinkMac (macOS vsync driver)
  → ExternalBeginFrameSourceMac (per-compositor begin frame source)
    → ExternalBeginFrameSource::AddObserver/RemoveObserver
      → DisplayScheduler::OnBeginFrame (schedules deadline)
        → DisplayScheduler::AttemptDrawAndSwap
          → ShouldDraw() gate (needs_draw_, visible, output_surface_lost,
                               root_frame_missing)
          → DrawAndSwap() → actual pixel output
      → CompositorFrameSinkSupport::OnBeginFrame (forwards to renderer)
        → ShouldSendBeginFrame() gate (multiple throttle checks)
        → client_->OnBeginFrame() (Mojo IPC to renderer process)
```

### Throttle mechanisms investigated

| Mechanism                         | Location                         | Threshold            | Status in Issue 620      |
| --------------------------------- | -------------------------------- | -------------------- | ------------------------ |
| StopObservingBeginFrames          | display_scheduler.cc:603         | ShouldDraw()=false   | Fixed (Exp 13), not root |
| BeginFrameTracker::ShouldThrottle | begin_frame_tracker.cc           | outstanding >= 10    | Never triggered          |
| BeginFrameTracker::ShouldStop     | begin_frame_tracker.cc           | outstanding >= 100   | Never triggered          |
| kUndrawnFrameLimit                | compositor_frame_sink_support.cc | undrawn > 3          | Never triggered          |
| SetIsGpuBusy                      | display_scheduler.cc:644         | pending_swaps >= max | Not investigated         |
| client_needs_begin_frame_         | compositor_frame_sink_support.cc | client stopped       | 9 events (init only)     |

### The observer–renderer deadlock (found and partially fixed)

When `DisplayScheduler::AttemptDrawAndSwap()` calls `StopObservingBeginFrames()`
after `ShouldDraw()` returns false, the renderer loses BeginFrames and can't
produce new CompositorFrames. Without new frames, no damage arrives, and the
compositor stays stopped. This creates a deadlock that manifests as ~3fps
(register → draw 1–2 frames → unregister → wait ~300ms → repeat).

Experiment 13 fixed this by commenting out `StopObservingBeginFrames()`. The
vsync pipeline stayed alive, but rendering was still 2fps. The deadlock was a
symptom, not the root cause.

### The real bottleneck: renderer not submitting frames

After fixing the observation deadlock, the DisplayScheduler receives BeginFrames
at 60fps but `needs_draw_` stays false — no damage arrives from the renderer.
The renderer receives BeginFrames (via
`CompositorFrameSinkSupport::OnBeginFrame`) but only produces CompositorFrames
at ~3fps for complex pages. Lightweight pages are unaffected.

### Key source files

| File                               | Role                                                         |
| ---------------------------------- | ------------------------------------------------------------ |
| cv_display_link_mac.mm             | macOS CVDisplayLink vsync driver                             |
| ca_display_link_mac.mm             | macOS CADisplayLink vsync driver (newer macOS)               |
| external_begin_frame_source_mac.cc | Per-compositor vsync source, registers/unregisters callbacks |
| begin_frame_source.cc              | Observer management, OnNeedsBeginFrames dispatch             |
| display_scheduler.cc               | Draw timing, ShouldDraw gate, AttemptDrawAndSwap             |
| display_damage_tracker.cc          | Tracks root_frame_missing, surface damage                    |
| compositor_frame_sink_support.cc   | Forwards BeginFrames to renderer, throttle gates             |
| begin_frame_tracker.cc             | Tracks outstanding BeginFrames for throttle/stop             |
| root_compositor_frame_sink_impl.cc | Creates ExternalBeginFrameSourceMac per profile              |
| layer_tree_host_impl.cc            | Renderer-side compositor, WillBeginImplFrame, damage check   |
| scheduler_state_machine.h          | Renderer scheduler state, throttle_frame_rate_               |
| shared_image_manager.h             | GPU process SharedImageManager, serializes GPU access        |
| render_process_host_impl.cc        | Process allocation, process-per-site logic                   |
| page_scheduler_impl.cc             | Blink page scheduler, visibility/background throttling       |
| frame_rate_throttling.cc           | Browser-side frame sink throttling                           |

### Uninvestigated mechanisms

These are candidate throttle points that Issue 620 did not instrument:

**1. GPU channel serialization.** The GPU process has a single
`SharedImageManager` (gpu/command_buffer/service/shared_image/) that serializes
access to GPU resources across all contexts. When two renderers actively submit
GPU commands, they're serialized through lock-protected critical sections. This
could halve effective frame rate.

**2. Renderer-side early damage check.** `LayerTreeHostImpl::WillBeginImplFrame`
(cc/trees/layer_tree_host_impl.cc:3873) performs an early damage check that can
skip the entire frame:

```cpp
bool recent_frame_had_no_damage =
    consecutive_frame_with_damage_count_ < settings_.damaged_frame_limit;
if (settings_.enable_early_damage_check && recent_frame_had_no_damage &&
    CanDraw()) {
    // ... check for damage, skip frame if none found
}
```

Complex pages generate frequent damage, so
`consecutive_frame_with_damage_count_` stays high and the early check always
runs. Lightweight pages generate less damage, causing the counter to stay low
and the check to be skipped (allowing frames through without checking).

**3. Scheduler state machine throttling.**
`cc/scheduler/scheduler_state_machine.h` has `throttle_frame_rate_` and
`main_frame_throttled_interval_` that can explicitly throttle BeginMainFrame
delivery. If one renderer's scheduler enters throttled mode due to contention,
it would produce fewer frames.

**4. `SetIsGpuBusy` backpressure.** When `pending_swaps_ >= MaxPendingSwaps()`
(display_scheduler.cc:644), the DisplayScheduler calls
`begin_frame_source_->SetIsGpuBusy(true)`, which can throttle BeginFrame
delivery at the source level. With two profiles sharing the GPU, swap buffers
may back up past the threshold.

**5. Renderer process priority.** `RenderProcessHostPriorityClient`
(content/public/browser/render_process_host_priority_client.h) tracks
`is_hidden`, `frame_depth`, `intersects_viewport`. The OS scheduler distributes
CPU time according to process importance. If a renderer process is
deprioritized, its frame production drops.

**6. Browser-side frame rate throttling.**
`content/browser/performance_manager/frame_rate_throttling.cc` provides
`StartThrottlingAllFrameSinks()` and `UpdateThrottlingFrameSinks()`. If a
performance manager policy throttles frame sinks globally (e.g., for power
savings), all profiles would be affected.

**7. Blink main thread starvation.** The Blink main thread's `TaskQueueSelector`
(base/task/sequence_manager/task_queue_selector.h) uses priority-based
scheduling. In multi-context scenarios, if both renderers' main threads queue
BeginFrame-related tasks (style recalc, layout, paint), they compete for
execution time. The priority mechanism can cause one context's main thread work
to back up, delaying CompositorFrame submission.

## Chromium branch

`146.0.7650.0-issue-621` — branched from the vanilla `146.0.7650.0` tag. Fresh
start with no Issue 620 instrumentation. Code patterns from Issue 620's branch
(`146.0.7650.0-issue-620`) can be referenced but are not carried forward.

## Approach

Same as Issue 620: the Zig Content Shell is a thin C++ shim
(`content/zig_content_shell/`) that subclasses Content Shell's classes and
exports `ContentMain()` as `extern "C"`. The shim is built with GN/autoninja
inside `chromium/src/` and produces a macOS `.app` bundle.

The shim code (BUILD.gn, content_api_shim.h, content_api_shim.mm, plist files,
ts_main.mm) is copied from Issue 620's final state. The only change between
experiments will be `content_api_shim.mm` (URLs, profiles) and Chromium source
modifications (instrumentation, fixes).

## Ideas for experiments

### 1. Reproduce the baseline: single profile, google.com

Copy the Zig Content Shell files from Issue 620's branch to the new branch.
Configure for a single profile loading google.com. Verify 60fps rendering. This
establishes the baseline and confirms the build works on the fresh branch.

### 2. Reproduce the degradation: two profiles, google.com + DDG

Add a second BrowserContext (Profile B) loading lite.duckduckgo.com. Verify that
google.com degrades to 2fps while DDG stays at 60fps. This reproduces the Issue
620 finding and gives us a stable reproduction case. The two pages are easily
distinguishable (both have search inputs for typing tests).

### 3. Instrument renderer-side frame production

The viz compositor pipeline is clean (Issue 620 Exp 14 proved this). The
bottleneck is the renderer not submitting CompositorFrames. Instrument the
renderer side:

- **`cc/trees/layer_tree_host_impl.cc:WillBeginImplFrame`** — log whether the
  early damage check skips the frame, and what
  `consecutive_frame_with_damage_count_` is
- **`cc/scheduler/scheduler.cc`** — log when `SetShouldThrottleFrameRate()` is
  called and whether `throttle_frame_rate_` is true
- **`cc/trees/layer_tree_host_impl.cc:PrepareToDraw`** — log when the renderer
  actually prepares a draw vs skips

This would reveal whether the renderer skips frames at the cc layer.

### 4. Instrument GPU channel contention

If the renderer produces frames but they're slow to reach the GPU:

- **`gpu/command_buffer/service/shared_image/shared_image_manager.cc`** — log
  lock acquisition/contention
- **`display_scheduler.cc:DidSwapBuffers`** — log `pending_swaps_` count and
  whether `SetIsGpuBusy(true)` fires

This would reveal whether GPU serialization is the bottleneck.

### 5. Test with two google.com profiles

Load google.com on both profiles. If both degrade to 2fps, the problem is
complexity-dependent (any complex page triggers it). If only one degrades, the
problem is asymmetric contention (first or second profile specifically).

### 6. Test with two lite.duckduckgo.com profiles

Load DDG on both profiles. If both render at 60fps, the problem is
page-complexity-specific. If one degrades, even lightweight pages are affected
under certain conditions.

### 7. Disable renderer-side early damage check

Set `settings_.enable_early_damage_check = false` in the renderer's
LayerTreeSettings. If the early damage check
(`layer_tree_host_impl.cc:3941–3952`) is causing frame skips for the complex
page when contention exists, disabling it would let all frames through.

### 8. Separate GPU channels per BrowserContext

Investigate whether two BrowserContexts can use separate GPU channels or command
buffer streams. If GPU serialization is the bottleneck, this would eliminate the
contention. Look at `GpuProcessHost::EstablishGpuChannel()` in
`content/browser/gpu/gpu_process_host.cc`.

### 9. Adjust MaxPendingSwaps threshold

`DisplayScheduler::MaxPendingSwaps()` (display_scheduler.cc:357) returns
different values based on display refresh rate. If
`pending_swaps_ >=
MaxPendingSwaps()`, the GPU is marked busy and BeginFrame
delivery throttles. Increasing the threshold might prevent premature GPU busy
signals when two profiles share the same GPU.

### 10. Disable frame rate throttling in the scheduler state machine

The renderer's `SchedulerStateMachine` has `throttle_frame_rate_` and
`main_frame_throttled_interval_`. If these are being set due to multi-context
contention, disabling them would force the renderer to produce frames at full
speed.

### 11. Process-per-BrowserContext investigation

Check whether two BrowserContexts get separate renderer processes or share one.
`RenderProcessHostImpl::GetProcessHostForSiteInstance()` in
`content/browser/renderer_host/render_process_host_impl.cc` decides process
allocation. If they share a renderer, all frame production competes on one
process's threads. If they're separate, the contention is in the browser or GPU
process.

### 12. Profile with Chrome tracing

Run with
`--trace-startup=cc,viz,gpu --trace-startup-duration=10
--trace-startup-file=/tmp/trace.json`.
Load the trace in `chrome://tracing` or Perfetto UI. The trace will show exactly
where time is spent per frame for each compositor, revealing whether the
bottleneck is in cc scheduling, GPU command submission, or Blink main thread
work.
