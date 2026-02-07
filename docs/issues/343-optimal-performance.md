# Issue 343: Optimal Performance — Perfect 60fps

## Goal

Achieve perfect, uncompromising 60fps frame delivery from the profile server to
the GUI. Not 38fps. Not 71% of frames at 60fps. Every frame, every time.

The current implementation delivers 38.2fps average with 71% of frames at 60fps
cadence and a max streak of 424 consecutive good frames. This is a dramatic
improvement over where we started (17fps), but it is visibly inferior to native
60fps. Scrolling still stutters. Animations still hitch. The remaining 29% of
dropped or delayed frames are perceptible and unacceptable.

We will not stop until the profile server delivers a sustained, unbroken 60fps
stream — matching or exceeding the cef-rs OSR example's performance.

## How We Got Here

### Issue 338: Discovery

[Issue 338](./338-lag.md) identified the problem: TermSurf's browser rendering
was noticeably laggy compared to native Chrome. Scrolling felt sluggish,
animations stuttered, and hover effects were jerky. Investigation revealed the
bottleneck was not XPC transport latency but CEF itself — the profile server was
producing frames at only ~20fps.

Key findings from 338:

- IOSurface import is fast (~0.37ms), not the bottleneck
- Mouse move events are unthrottled (100+ XPC messages/sec during movement)
- The GUI holds a read lock during the entire render loop
- Mach ports are recreated every frame even when the IOSurface handle hasn't
  changed

### Issue 340: The cef-rs Breakthrough

[Issue 340](./340-architecture.md) made the critical discovery: the cef-rs OSR
example achieves a sustained 60fps with the same CEF version. This disproved the
theory that CEF had a hard 30fps cap and proved the problem was in our process
environment, not CEF itself.

### Issue 341: 18 Experiments, One Dead End

[Issue 341](./341-performance.md) ran 18 systematic experiments to close the gap
between the cef-rs example (60fps) and the profile server (~20fps):

| #  | Experiment                               | Result  | 60fps% | Streak |
| -- | ---------------------------------------- | ------- | ------ | ------ |
| 1  | Document process architecture            | Diag    | —      | —      |
| 2  | Add winit event loop (no window)         | Failed  | 3%     | —      |
| 3  | Measure cef-rs example frame rate        | Diag    | ~90%   | —      |
| 4  | Enable `external_message_pump`           | Partial | 52%    | 5      |
| 5  | `NSApplicationActivationPolicyRegular`   | Failed  | 42%    | —      |
| 6  | Use `run_message_loop()`                 | Failed  | 24%    | 5      |
| 7  | Hidden 1x1 window                        | Success | 78%    | 57     |
| 8  | CVDisplayLink without window             | Failed  | 30%    | 4      |
| 9  | Restore hidden window baseline           | Success | 61%    | 35     |
| 10 | `NSApplicationActivationPolicyAccessory` | Failed  | —      | —      |
| 11 | Native NSWindow, `canBecomeKey: NO`      | Partial | 34%    | 4      |
| 12 | `orderFront` instead of `orderBack`      | Partial | 34%    | 4      |
| 13 | Layer-backed content view                | Failed  | 36%    | 3      |
| 14 | NSApplication event pumping              | Failed  | 33%    | 4      |
| 15 | Swizzle `canBecomeKey` on winit          | Failed  | —      | —      |
| 16 | GUI-side focus reclaim                   | Partial | 20%    | 16     |
| 17 | External begin frame at 60Hz             | Failed  | 10%    | 2      |
| 18 | Revert to baseline                       | Diag    | 40%    | 11     |

The only approach that worked was a hidden 1x1 window (Exp 7: 78% at 60fps), but
it steals focus from the GUI. Every attempt to fix focus stealing destroyed the
vsync signal. The hidden window approach was abandoned as an architectural dead
end — focus and vsync are fundamentally coupled through the macOS window server.

### Issue 342: The CFRunLoop Breakthrough

[Issue 342](./342-perf-no-win.md) took a different approach: instead of
providing CEF with an external vsync signal, understand why its internal frame
scheduling was failing. Five experiments:

| # | Experiment                | Result  | FPS  | 60fps% | Streak |
| - | ------------------------- | ------- | ---- | ------ | ------ |
| 1 | CEF debug logging         | Diag    | —    | —      | —      |
| 2 | NSApplication init        | Failed  | 28.5 | 40%    | 11     |
| 3 | `run_message_loop()`      | Failed  | 19.2 | —      | —      |
| 4 | CFRunLoop + external pump | Failed  | 0    | 0%     | 0      |
| 5 | `CFRunLoopRunInMode` swap | Success | 38.2 | 71%    | 424    |

**Root cause found:** CEF's `SyntheticBeginFrameSource` — the timer-based frame
scheduler for windowless mode — schedules work via CFRunLoop timer sources.
`thread::sleep()` suspends the thread without servicing the run loop, starving
these sources. Replacing `sleep(1ms)` with `CFRunLoopRunInMode(0.001)` services
pending timer callbacks, allowing CEF's internal scheduling to function.

**Current state after Issue 342:**

| Metric                    | Before (Issue 341) | After (Issue 342) |
| ------------------------- | ------------------ | ----------------- |
| Average FPS               | 28.5               | 38.2              |
| Frames at 60fps           | 40%                | 71%               |
| Max consecutive 60fps     | 11                 | 424               |
| Display link samples      | —                  | 3 (not working)   |
| SyntheticBeginFrame fires | 3                  | 19                |

## Current Code

The profile server's polling loop (`ts3/termsurf-profile/src/main.rs`):

```rust
while !QUIT_FLAG.load(Ordering::Relaxed) {
    cef::do_message_loop_work();
    #[cfg(target_os = "macos")]
    cfrunloop::run_for(0.001); // CFRunLoopRunInMode, 1ms
    #[cfg(not(target_os = "macos"))]
    std::thread::sleep(Duration::from_millis(1));
}
```

CEF settings: `windowless_rendering_enabled: 1`, `no_sandbox: 1`,
`log_severity: VERBOSE`. No `external_message_pump`. No window. No event loop.

## What We Know

1. **CEF can deliver 60fps** — the cef-rs OSR example proves it (Issue 341,
   Exp 3)
2. **The display link is broken** — `ExternalBeginFrameSourceMac.DisplayLink`
   fires only 3 times; it needs a window server connection we don't have
3. **SyntheticBeginFrameSource works but is inconsistent** — it fires at the
   correct 16ms interval but only produced 19 samples across the session (Issue
   342, Exp 5)
4. **71% of frames hit 60fps cadence** — something causes the other 29% to miss
5. **CFRunLoop servicing is necessary** — without it, CEF's timers starve
   completely
6. **The polling loop structure matters** — `do_message_loop_work()` +
   `CFRunLoopRunInMode(1ms)` is the best combination found so far

## The Remaining Gap: 71% → 100%

The 29% of non-60fps frames fall into several categories based on Issue 342's
Experiment 5 data:

- **10-19ms bucket:** 426 intervals (72%) — these are the good frames
- **30-36ms bucket:** ~50 intervals — frames arriving at exactly 2x the vsync
  period (30fps), suggesting a missed compositor beat
- **50-80ms bucket:** scattered — multi-beat misses, likely from longer stalls
- **>100ms:** rare — page load, layout, or JavaScript execution pauses

The 30fps frames (33ms intervals) are the primary target. These are not random
jitter — they are exactly one missed vsync beat, suggesting a systematic timing
issue where the compositor occasionally skips a cycle.

## Hypotheses: Why 29% of Frames Miss

### H1: Polling Loop Timing Mismatch

The current loop calls `do_message_loop_work()` then `CFRunLoopRunInMode(1ms)`.
The total loop iteration time is `do_message_loop_work()` latency + up to 1ms.
If `do_message_loop_work()` takes variable time, the loop cadence drifts
relative to CEF's internal 16.67ms compositor cycle.

When the loop call happens to align with CEF's timer — frame produced. When it
drifts out of alignment — timer fires but no `do_message_loop_work()` processes
it until the next iteration, missing the beat.

**Test:** Measure actual loop iteration times. If they cluster around 1.5-2ms
with occasional spikes, timing drift is the likely cause.

### H2: CFRunLoop 1ms Timeout Too Short

`CFRunLoopRunInMode` with a 1ms timeout returns after either handling one source
or timing out. If CEF has multiple timer sources that need servicing in a single
iteration (e.g., SyntheticBeginFrameSource + compositor task), only the first
one fires per call. The second waits for the next loop iteration.

**Test:** Try longer CFRunLoop timeouts (2ms, 5ms, 10ms) or call
`CFRunLoopRunInMode` multiple times per loop iteration until it returns "timed
out" (no more sources to handle).

### H3: `do_message_loop_work()` and CFRunLoop Fighting

`do_message_loop_work()` processes CEF's internal task queue.
`CFRunLoopRunInMode` processes CFRunLoop sources (which include CEF's timers).
These may interfere: `do_message_loop_work()` might partially process a task
that a CFRunLoop timer was about to trigger, or vice versa. The two systems are
not designed to be interleaved this way.

**Test:** Try calling only `CFRunLoopRunInMode` with a longer timeout (16ms) and
no explicit `do_message_loop_work()`. If CEF's timers internally call
`do_message_loop_work()` when they fire, the explicit call may be redundant — or
harmful.

### H4: `external_message_pump` Would Help (If Deadlock Is Fixed)

Issue 342 Experiment 4 failed because of a chicken-and-egg deadlock: CEF needs
`do_message_loop_work()` during initialization, but the cooperative timer that
calls it only fires after the run loop starts. However, the cef-rs OSR example
uses `external_message_pump: true` and achieves 60fps. The reference
implementation in `cef-rs/examples/tests_shared/` uses `NSApp().run()` (not
`CFRunLoopRun()`) to avoid this deadlock.

If we can correctly initialize CEF with `external_message_pump` — perhaps by
running a brief polling phase during init before switching to cooperative
scheduling — the `on_schedule_message_pump_work` callback would let CEF tell us
exactly when to call `do_message_loop_work()`, eliminating all timing guesswork.

**Test:** Two-phase approach: (1) poll during CEF init, (2) switch to
`on_schedule_message_pump_work`-driven scheduling after `on_context_initialized`
fires.

### H5: SyntheticBeginFrameSource Only Fires 19 Times

Issue 342 Exp 5 showed only 19 `Viz.ExternalBeginFrameSource.Interval` histogram
samples across a 15-second session with 593 frames. If SyntheticBeginFrameSource
is the frame clock, 19 fires across 593 frames means most frames are produced by
some other mechanism — possibly `Invalidate()` calls from content changes or
`do_message_loop_work()` directly triggering compositor runs.

The 19 fires could be the "seed" that kicks CEF into rendering, after which the
compositor runs on momentum for a while before stalling. The stalls would
correspond to the 30fps drops.

**Test:** Correlate SyntheticBeginFrameSource fire times with frame timing logs
to see if the 19 fires correspond to the starts of 60fps streaks.

### H6: Missing Vsync Signal Causes Frame Pacing Jitter

The display link (`ExternalBeginFrameSourceMac.DisplayLink`) fires only 3 times.
Without a real vsync signal, CEF relies on SyntheticBeginFrameSource — a
software timer. Software timers on macOS are subject to:

- Timer coalescing (macOS groups timers to save power)
- Thread scheduling jitter
- CFRunLoop iteration timing

A CVDisplayLink or CADisplayLink tied to the actual display refresh would
provide a hardware-accurate frame clock. Issue 341 Exp 8 tried CVDisplayLink
alone (it didn't help), but that was before the CFRunLoop fix. Combining a
CVDisplayLink with CFRunLoop servicing might provide both the timing signal and
the run loop integration.

**Test:** Create a CVDisplayLink and use its callback to trigger
`do_message_loop_work()` at precise vsync times, replacing the blind 1ms
polling.

### H7: GUI-Side Frame Pacing

The profile server may produce frames at 60fps, but the GUI may not consume them
at that rate. WezTerm's render loop has its own cadence. If the GUI renders at a
different rate or has variable frame times, it could drop profile server frames
or introduce jitter.

The GUI currently uses `PresentMode::Fifo` with
`desired_maximum_frame_latency:
2`, which means 2 frames in the GPU queue. If the
GUI's render loop takes variable time, frames from the profile server may sit in
the queue waiting.

**Test:** Add frame timing instrumentation to the GUI side to measure when
IOSurface frames arrive vs when they're actually rendered to screen.

### H8: Mach Port Creation Overhead

Every `on_accelerated_paint` callback creates a new Mach port from the IOSurface
handle, even when the handle hasn't changed. This is a kernel syscall on every
frame. CEF uses double/triple buffering so the handle changes on ~82% of frames,
but the 18% of redundant Mach port creations add kernel overhead in the hot
path.

More importantly, the XPC message to send the Mach port to the GUI adds latency.
Issue 338 measured XPC send at 10-30ms — a significant chunk of the 16.67ms
frame budget.

**Test:** Cache Mach ports by IOSurface handle. Only create and send a new Mach
port when the handle actually changes.

### H9: `return_after_source_handled` Flag

`CFRunLoopRunInMode` is called with `return_after_source_handled: true` (1).
This means it returns after processing a single source. If multiple CFRunLoop
sources need servicing in one frame cycle (e.g., a timer source + an input
source + a compositor callback), only the first is handled per call. The others
wait for the next loop iteration, introducing a 1ms+ delay.

**Test:** Try `return_after_source_handled: false` (0) so CFRunLoop processes
all pending sources before returning, or loop until CFRunLoop returns
`kCFRunLoopRunTimedOut` (no more sources to handle).

### H10: CEF Process Priority and QoS

macOS assigns Quality of Service (QoS) classes to threads and processes. A
process without a window may be assigned a lower QoS class, leading to:

- Lower scheduling priority
- Timer coalescing (reduced timer precision)
- Reduced CPU allocation

Even though `NSApplicationActivationPolicyRegular` didn't help frame rate in
Issue 341 Exp 5, thread-level QoS might matter. The main thread's QoS class
determines timer precision. Setting it to `QOS_CLASS_USER_INTERACTIVE` (highest)
would give maximum timer precision.

**Test:** Set the main thread's QoS class to `QOS_CLASS_USER_INTERACTIVE` before
entering the polling loop.

## Ideas Checklist

Ordered by likelihood of impact and implementation simplicity:

- [ ] **1. Drain CFRunLoop fully** (H2, H9) — Loop `CFRunLoopRunInMode` until it
      returns timed-out, processing all pending sources per iteration
- [ ] **2. Measure loop iteration timing** (H1) — Add microsecond-precision
      timing to the polling loop to understand actual cadence
- [ ] **3. Remove explicit `do_message_loop_work()`** (H3) — Test whether
      CFRunLoop alone drives CEF, without the explicit call
- [ ] **4. Increase CFRunLoop timeout** (H2) — Try 2ms, 5ms, 16ms timeouts to
      give run loop sources more time to fire
- [ ] **5. Set thread QoS to USER_INTERACTIVE** (H10) — Maximize timer precision
      and scheduling priority
- [ ] **6. Cache Mach ports by IOSurface handle** (H8) — Eliminate redundant
      kernel syscalls and XPC messages
- [ ] **7. Two-phase `external_message_pump`** (H4) — Poll during init, switch
      to cooperative scheduling after context initialized
- [ ] **8. CVDisplayLink + CFRunLoop** (H6) — Hardware vsync-driven frame timing
      combined with run loop servicing
- [ ] **9. Correlate SyntheticBeginFrameSource with frame timing** (H5) —
      Diagnostic to understand the 19-sample mystery
- [ ] **10. GUI-side frame timing instrumentation** (H7) — Measure arrival vs
      render time on the GUI side

## Constraints

- **No hidden windows.** Proven to be an architectural dead end (Issue 341).
- **No focus stealing.** The GUI must retain keyboard focus at all times.
- **macOS first.** macOS-specific APIs are acceptable; Linux/Windows are future
  work.
- **Profile server is a separate process.** One CEF instance per profile is a
  hard constraint.
- **No regressions.** Each experiment must match or exceed 38.2fps / 71% at
  60fps.

## Related Issues

- [Issue 338: Browser lag investigation](./338-lag.md) — Original performance
  discovery, input path and output path analysis
- [Issue 340: Architecture reconsideration](./340-architecture.md) — Discovery
  that cef-rs OSR example achieves 60fps
- [Issue 341: Performance investigation](./341-performance.md) — 18 experiments,
  hidden window approach discovered and abandoned
- [Issue 342: 60fps without a hidden window](./342-perf-no-win.md) — CFRunLoop
  breakthrough, current 38.2fps baseline established

## Experiments

### Experiment 1: Drain CFRunLoop Fully

**Status:** Not started

**Goal:** Service all pending CFRunLoop sources per loop iteration instead of
just one, eliminating timing gaps where CEF's internal timers fire but their
callbacks are deferred to the next iteration.

**Hypotheses tested:** H2 (timeout too short), H9 (`return_after_source_handled`
flag)

#### Problem

The current code calls `CFRunLoopRunInMode` once per loop iteration with
`return_after_source_handled: true` (1). This means:

1. If a timer source fires → it's handled → function returns immediately
2. If no source fires within 1ms → function returns on timeout
3. **Either way, at most one source is processed per call**

`CFRunLoopRunInMode` returns one of four values:

| Return value                 | Int | Meaning                             |
| ---------------------------- | --- | ----------------------------------- |
| `kCFRunLoopRunFinished`      | 1   | No sources or timers in this mode   |
| `kCFRunLoopRunStopped`       | 2   | Stopped via `CFRunLoopStop()`       |
| `kCFRunLoopRunTimedOut`      | 3   | Timeout expired, no source handled  |
| `kCFRunLoopRunHandledSource` | 4   | A source was handled (early return) |

When the return value is 4, there may be additional sources ready to fire. The
current code ignores this and proceeds to the next `do_message_loop_work()` +
`CFRunLoopRunInMode` cycle. If CEF has multiple run loop sources that need to
fire within a single 16.67ms compositor window (e.g., SyntheticBeginFrameSource
tick + compositor dispatch + IPC callback), the second and third sources are
delayed by one full loop iteration (~1-2ms). Over several cycles this drift
accumulates, eventually causing a missed compositor beat — which shows up as a
33ms frame (30fps) instead of 16ms (60fps).

#### Changes

Two modifications to `ts3/termsurf-profile/src/main.rs`:

**1. Add a `drain()` function to the `cfrunloop` module:**

Replace the single `run_for()` function with a `drain()` that loops until the
run loop has no more sources to handle:

```rust
/// Drain all pending CFRunLoop sources. Calls CFRunLoopRunInMode in a
/// loop until it returns kCFRunLoopRunTimedOut (3), meaning no more
/// sources are ready. Uses a minimal timeout (0.001s = 1ms) per call
/// to avoid blocking indefinitely.
pub fn drain() {
    const TIMED_OUT: i32 = 3;
    loop {
        let result = unsafe {
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.001, 1)
        };
        if result != 4 {
            // Not kCFRunLoopRunHandledSource — either timed out,
            // finished, or stopped. No more sources to drain.
            break;
        }
        // A source was handled. There may be more — loop again
        // with a fresh timeout.
    }
}
```

The key insight: when `CFRunLoopRunInMode` returns 4
(`kCFRunLoopRunHandledSource`), we immediately call it again with a fresh 1ms
timeout. This continues until it returns 3 (`kCFRunLoopRunTimedOut`), meaning
all pending sources have been serviced.

Safety: the loop cannot spin forever because each call either handles a source
(finite number pending) or times out after 1ms. In the worst case (no sources),
this behaves identically to the current single call.

**2. Replace `cfrunloop::run_for(0.001)` with `cfrunloop::drain()` in the
polling loop:**

```rust
while !QUIT_FLAG.load(std::sync::atomic::Ordering::Relaxed) {
    cef::do_message_loop_work();
    #[cfg(target_os = "macos")]
    cfrunloop::drain();
    #[cfg(not(target_os = "macos"))]
    std::thread::sleep(std::time::Duration::from_millis(1));
}
```

#### What Stays the Same

- `do_message_loop_work()` is still called every iteration
- CEF settings unchanged (no `external_message_pump`)
- No new dependencies
- Non-macOS fallback still uses `sleep(1ms)`
- All XPC, shutdown, and browser creation code unchanged

#### Expected Outcomes

| Result                       | Meaning                                                                                  |
| ---------------------------- | ---------------------------------------------------------------------------------------- |
| >80% at 60fps, higher streak | Multiple sources were being starved. H2/H9 confirmed.                                    |
| ~71% at 60fps (unchanged)    | Only one source fires per cycle anyway. H2/H9 ruled out. Investigate H1/H3 next.         |
| Performance regression       | Draining too aggressively delays `do_message_loop_work()`. Try capping drain iterations. |

#### Risk

Low. The drain loop adds at most a few microseconds per extra source handled. If
only one source ever fires (the common case today), the behavior is identical to
the current code — one call returns 4, the next returns 3, loop exits after two
calls instead of one.

#### Results

**Status: FAILED** — Performance regressed across every metric.

**Overall stats:** 495 frames over 16.1s = **30.6 fps average**

| Interval        | Count | Percentage |
| --------------- | ----- | ---------- |
| Burst (0-5ms)   | 18    | 3.6%       |
| 60fps (6-20ms)  | 303   | **61.3%**  |
| 30fps (21-40ms) | 108   | 21.9%      |
| Mid (41-70ms)   | 25    | 5.1%       |
| Low (>70ms)     | 40    | 8.1%       |

**Dominant intervals:** 17ms (79), 16ms (76), 18ms (70) — still vsync-aligned,
but weaker than before. Secondary peak at 33-35ms (87 combined).

**Max consecutive 60fps frames:** **62** (top streaks: 62, 21, 20, 19, 18)

CEF debug log histograms:

- `Viz.ExternalBeginFrameSourceMac.DisplayLink`: 3 samples (unchanged)
- `Viz.ExternalBeginFrameSource.Interval`: **11 samples**, mean 16ms (down from
  19 in Issue 342 Exp 5 — SyntheticBeginFrameSource firing less often)
- `Viz.FrameSinkVideoCapturer.CaptureDuration`: 494 samples, mean 7.6ms
- `Graphics.Smoothness.PercentDroppedFrames3.AllSequences`: **22.2%** (up from
  19% — more frames dropped)

#### Comparison

| Metric                    | Issue 342 Exp 5 (baseline) | **Exp 1 (drain)** |
| ------------------------- | -------------------------- | ----------------- |
| Average FPS               | 38.2                       | **30.6**          |
| Frames at 60fps           | 71%                        | **61.3%**         |
| Frames at 30fps           | ~15%                       | **21.9%**         |
| Max consecutive 60fps     | 424                        | **62**            |
| SyntheticBeginFrame fires | 19                         | **11**            |
| PercentDroppedFrames      | 19%                        | **22.2%**         |

#### Conclusion

Draining all CFRunLoop sources made everything worse. The
SyntheticBeginFrameSource dropped from 19 to 11 samples — we starved it more,
not less. The max streak collapsed from 424 to 62.

The cause: draining multiple sources delays the return to
`do_message_loop_work()`. CEF's compositor posts tasks to its internal queue
that require `do_message_loop_work()` to process. By spending extra time in the
CFRunLoop drain loop, we defer that processing, causing missed compositor beats.

This rules out **H2** (timeout too short) and **H9**
(`return_after_source_handled` flag). The single-source-per-call behavior in
Issue 342 Exp 5 was actually optimal — it provides the tightest interleaving
between CFRunLoop source handling and `do_message_loop_work()` task processing.

The result supports **H3** (the two systems fighting): `do_message_loop_work()`
and CFRunLoop need tight, alternating interleaving. More aggressive draining of
either side hurts the other. The next experiment should investigate this
relationship — either by removing `do_message_loop_work()` entirely (letting
CFRunLoop drive everything) or by tuning the balance between the two calls.

### Experiment 2: Set Thread QoS to USER_INTERACTIVE

**Status:** Not started

**Goal:** Eliminate macOS timer coalescing as a source of missed compositor beats
by setting the main thread's Quality of Service class to the highest level.

**Hypothesis tested:** H10 (process priority and QoS)

#### Problem

macOS assigns a QoS class to every thread. The QoS class determines:

- **Scheduling priority** — how quickly the thread gets CPU time after becoming
  runnable
- **Timer precision** — whether timers fire at their exact deadline or get
  coalesced with nearby timers to save power
- **Timer leeway** — the system-imposed tolerance on timer firing times

The five QoS classes, from lowest to highest:

| QoS Class              | Value | Timer behavior                     |
| ---------------------- | ----- | ---------------------------------- |
| `QOS_CLASS_BACKGROUND` | 0x09  | Aggressive coalescing, low priority |
| `QOS_CLASS_UTILITY`    | 0x11  | Moderate coalescing                |
| `QOS_CLASS_DEFAULT`    | 0x15  | Standard behavior                  |
| `QOS_CLASS_USER_INITIATED` | 0x19 | Reduced coalescing              |
| `QOS_CLASS_USER_INTERACTIVE` | 0x21 | **Minimal coalescing, highest priority** |

The profile server is a windowless background process. macOS likely assigns it
`QOS_CLASS_DEFAULT` or lower. At this level, the system is permitted to coalesce
CFRunLoop timer firings by several milliseconds — grouping them with other timers
to reduce CPU wake-ups and save power.

CEF's `SyntheticBeginFrameSource` is a CFRunLoop timer set to fire every 16.67ms.
If macOS coalesces this timer even slightly (e.g., delays it by 2-3ms), the
callback fires at 19ms instead of 16.67ms. The compositor misses its deadline
for the current vsync beat and the frame slips to the next one — producing a
33ms interval (30fps) instead of 16ms (60fps).

This would explain the bimodal pattern: most frames hit 16-17ms (timer fires on
time), but ~20% land at 33-35ms (timer coalesced past the deadline). The pattern
is not random jitter — it's the exact missed-beat signature of timer coalescing.

#### Changes

One addition to `ts3/termsurf-profile/src/main.rs`:

**Add a `pthread_set_qos_class_self_np` call before the polling loop:**

```rust
// Issue 343, Experiment 2: Set highest QoS for maximum timer precision.
#[cfg(target_os = "macos")]
unsafe {
    extern "C" {
        fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
    }
    let QOS_CLASS_USER_INTERACTIVE: u32 = 0x21;
    let ret = pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0);
    println!("Profile: Set QoS to USER_INTERACTIVE: {}", if ret == 0 { "ok" } else { "failed" });
}
```

This goes just before the `while !QUIT_FLAG` loop, after Ctrl+C handler setup.
The call sets the current thread (main thread) to the highest QoS class,
telling the kernel this thread is doing user-interactive work that requires
maximum responsiveness.

#### What Stays the Same

- Polling loop structure unchanged (`do_message_loop_work()` + `run_for(0.001)`)
- CEF settings unchanged
- No new dependencies (uses raw FFI to a single POSIX function)
- All other code unchanged

#### Expected Outcomes

| Result                         | Meaning                                           |
| ------------------------------ | ------------------------------------------------- |
| >80% at 60fps, fewer 33ms drops | Timer coalescing was the cause. H10 confirmed.   |
| ~71% at 60fps (unchanged)      | Timer precision is already adequate. H10 ruled out. |
| Performance regression         | Extremely unlikely — higher QoS cannot reduce timer precision. |

#### Risk

Effectively zero. `QOS_CLASS_USER_INTERACTIVE` is what every GUI app's main
thread runs at. It increases CPU priority and timer precision — it cannot make
timers less accurate. The only cost is slightly higher power consumption, which
is irrelevant for a process that's already polling at 1ms intervals.

#### Results

**Status: FAILED** — Performance regressed despite QoS being set successfully.

QoS was confirmed set: `Profile: Set QoS to USER_INTERACTIVE: ok`

**Overall stats:** 298 frames over 10.2s = **28.9 fps average**

| Interval        | Count | Percentage |
| --------------- | ----- | ---------- |
| Burst (0-5ms)   | 18    | 6.1%       |
| 60fps (6-20ms)  | 192   | **64.6%**  |
| 30fps (21-40ms) | 23    | 7.7%       |
| Mid (41-70ms)   | 42    | **14.1%**  |
| Low (>70ms)     | 22    | 7.4%       |

**Dominant intervals:** 17ms (126), 16ms (62) — vsync-aligned peak intact.
New cluster at 50ms (25 occurrences) and 66-67ms (10 combined) that did not
exist in the baseline.

**Max consecutive 60fps frames:** **61** (top streaks: 61, 35, 18, 7, 7)

CEF debug log histograms:

- `Viz.ExternalBeginFrameSourceMac.DisplayLink`: 3 samples (unchanged)
- `Viz.ExternalBeginFrameSource.Interval`: 17 samples, mean 16ms (down from 19)
- `Viz.FrameSinkVideoCapturer.CaptureDuration`: 297 samples, mean 8.4ms
- `Graphics.Smoothness.PercentDroppedFrames3.AllSequences`: **22.5%** (up from
  19%)

#### Comparison

| Metric                    | Issue 342 Exp 5 (baseline) | **Exp 2 (QoS)** |
| ------------------------- | -------------------------- | ---------------- |
| Average FPS               | 38.2                       | **28.9**         |
| Frames at 60fps           | 71%                        | **64.6%**        |
| Frames at 30fps           | ~15%                       | **7.7%**         |
| Mid (41-70ms)             | —                          | **14.1%**        |
| Max consecutive 60fps     | 424                        | **61**           |
| SyntheticBeginFrame fires | 19                         | **17**           |
| PercentDroppedFrames      | 19%                        | **22.5%**        |

#### Conclusion

QoS made things worse. While the 30fps bucket improved (7.7% vs ~15%), a new
41-70ms cluster appeared (14.1%), centered on 50ms (3 vsync beats) and 66ms
(4 vsync beats). These multi-beat misses didn't exist in the baseline and
dragged the average FPS down from 38.2 to 28.9.

The higher scheduling priority may have changed how macOS interleaves the main
thread with CEF's internal threads (GPU process communication, IPC handlers).
With `QOS_CLASS_USER_INTERACTIVE`, the main thread gets more aggressive
scheduling, potentially starving CEF's background threads that feed work into
the compositor pipeline.

**H10 is ruled out.** Timer coalescing was not the cause of the 30fps drops.
The problem is not scheduling precision — it's something in the interaction
between `do_message_loop_work()`, CFRunLoop, and CEF's internal task pipeline.

### Experiment 3: Measure Loop Iteration Timing

**Status:** Not started

**Goal:** Instrument the polling loop with microsecond-precision timing to
understand the actual behavior of `do_message_loop_work()` and
`CFRunLoopRunInMode` — how long each call takes, how they vary, and whether
spikes correlate with dropped frames.

**Hypothesis tested:** H1 (polling loop timing mismatch), plus general
diagnostics to guide future experiments.

#### Motivation

Two interventions have failed (Exp 1: drain, Exp 2: QoS). Both changed timing
behavior without understanding it, and both made things worse. Before trying
another intervention, we need to see what the loop is actually doing.

Key questions:

1. **`do_message_loop_work()` duration** — Is it consistently fast (<0.1ms)? Or
   does it spike when processing compositor tasks (5-10ms)? Spikes would
   directly explain missed beats.
2. **`CFRunLoopRunInMode` duration** — Does it always block for the full 1ms
   timeout? Or does it sometimes return instantly (source handled in
   microseconds)? This reveals how often CFRunLoop sources actually fire.
3. **Total loop iteration time** — How consistent is the cadence? If most
   iterations take 1.2ms but some take 8ms, timing drift is the cause.
4. **Correlation with frame drops** — Do `do_message_loop_work()` spikes precede
   33ms frame intervals? If yes, the spike is the cause. If no, the problem is
   elsewhere (CEF internal, GPU process, IPC).

#### Changes

One modification to `ts3/termsurf-profile/src/main.rs`:

**Replace the polling loop with an instrumented version:**

```rust
// Issue 343, Experiment 3: Instrumented polling loop.
println!("Profile: Running message loop (instrumented)...");
let mut loop_count: u64 = 0;
let mut max_mlw_us: u128 = 0;  // max do_message_loop_work duration
let mut max_cfl_us: u128 = 0;  // max CFRunLoopRunInMode duration
let mut max_total_us: u128 = 0; // max total iteration
let mut mlw_spike_count: u64 = 0; // iterations where mlw > 1ms
let mut cfl_instant_count: u64 = 0; // iterations where cfl < 0.1ms

while !QUIT_FLAG.load(std::sync::atomic::Ordering::Relaxed) {
    let t0 = std::time::Instant::now();

    cef::do_message_loop_work();
    let t1 = std::time::Instant::now();

    #[cfg(target_os = "macos")]
    cfrunloop::run_for(0.001);
    #[cfg(not(target_os = "macos"))]
    std::thread::sleep(std::time::Duration::from_millis(1));
    let t2 = std::time::Instant::now();

    let mlw_us = (t1 - t0).as_micros();
    let cfl_us = (t2 - t1).as_micros();
    let total_us = (t2 - t0).as_micros();

    if mlw_us > max_mlw_us { max_mlw_us = mlw_us; }
    if cfl_us > max_cfl_us { max_cfl_us = cfl_us; }
    if total_us > max_total_us { max_total_us = total_us; }
    if mlw_us > 1000 { mlw_spike_count += 1; }
    if cfl_us < 100 { cfl_instant_count += 1; }

    loop_count += 1;

    // Log every 1000 iterations (~1 second)
    if loop_count % 1000 == 0 {
        println!(
            "[LOOP-TIMING] iter={} max_mlw={}us max_cfl={}us max_total={}us mlw_spikes={} cfl_instant={}",
            loop_count, max_mlw_us, max_cfl_us, max_total_us, mlw_spike_count, cfl_instant_count
        );
    }
}

// Final summary
println!(
    "[LOOP-TIMING] FINAL iter={} max_mlw={}us max_cfl={}us max_total={}us mlw_spikes={} cfl_instant={}",
    loop_count, max_mlw_us, max_cfl_us, max_total_us, mlw_spike_count, cfl_instant_count
);
```

The instrumentation adds three `Instant::now()` calls per iteration (each ~20ns
on Apple Silicon) — negligible overhead relative to the 1ms+ loop cadence.

#### What This Measures

| Metric | What it tells us |
| --- | --- |
| `max_mlw` | Worst-case `do_message_loop_work()` duration. If >5ms, it's eating into the frame budget. |
| `max_cfl` | Worst-case `CFRunLoopRunInMode` duration. Should be ~1ms (timeout). If much longer, macOS is blocking us. |
| `max_total` | Worst-case loop iteration. If >16ms, we're guaranteed to miss a beat. |
| `mlw_spikes` | How often `do_message_loop_work()` takes >1ms. Frequent spikes = H1 confirmed. |
| `cfl_instant` | How often CFRunLoop returns instantly (<0.1ms). High count = sources rarely fire (most iterations are just timeout waits). |

#### What Stays the Same

- Loop behavior is identical — same `do_message_loop_work()` + `run_for(0.001)`
- CEF settings unchanged
- No new dependencies
- Frame production and XPC communication unaffected

#### Expected Outcomes

| Pattern | Meaning | Next step |
| --- | --- | --- |
| `mlw_spikes` is high, correlates with drops | `do_message_loop_work()` occasionally blocks. H1 confirmed. | Try `external_message_pump` (idea 7) for cooperative scheduling |
| `cfl_instant` is very high (>90%) | CFRunLoop sources rarely fire — most iterations are idle waits | The 1ms timeout is mostly wasted. Try shorter timeout or busy-poll |
| `max_total` stays under 2ms | Loop cadence is rock-solid. The problem is not in the loop. | Investigate CEF internals (idea 9) or GUI side (idea 10) |
| `max_mlw` or `max_total` occasionally >10ms | Rare but large spikes cause the 33ms drops | Try yielding differently after spikes, or cap iteration time |

#### Risk

None. This is purely diagnostic — additive logging with negligible overhead.
The loop behavior is bit-for-bit identical to the baseline.

#### Results

**Status: DIAGNOSTIC COMPLETE** — Reveals `do_message_loop_work()` is the
dominant cost, not CFRunLoop.

**Loop timing data:**

```
[LOOP-TIMING] FINAL iter=865 max_mlw=33293us max_cfl=747us max_total=33326us mlw_spikes=865 cfl_instant=831
```

| Metric                    | Value          | Meaning                                   |
| ------------------------- | -------------- | ----------------------------------------- |
| Total iterations          | 865            | ~65 iter/sec (not ~1000 as 1ms sleep implies) |
| `max_mlw`                 | **33,293us**   | `do_message_loop_work()` blocks up to 33ms |
| `max_cfl`                 | 747us          | `CFRunLoopRunInMode` well-behaved, always <1ms |
| `max_total`               | 33,326us       | Worst iteration = 33ms, entirely from mlw |
| `mlw_spikes` (>1ms)       | **865 of 865** | mlw takes >1ms on **every single call**   |
| `cfl_instant` (<0.1ms)    | **831 of 865** | CFRunLoop returns instantly **96% of the time** |

**Frame performance:**

453 frames over 13.4s = **33.7 fps average**

| Interval        | Count | Percentage |
| --------------- | ----- | ---------- |
| Burst (0-5ms)   | 19    | 4.2%       |
| 60fps (6-20ms)  | 374   | **82.7%**  |
| 30fps (21-40ms) | 23    | 5.1%       |
| Mid (41-70ms)   | 8     | 1.8%       |
| Low (>70ms)     | 28    | 6.2%       |

**Dominant intervals:** 17ms (236), 16ms (128) — strong vsync-aligned peak.

**Max consecutive 60fps frames:** **89** (top streaks: 89, 45, 34, 26, 18)

CEF debug log histograms:

- `Viz.ExternalBeginFrameSource.Interval`: 13 samples, mean 16ms
- `Viz.FrameSinkVideoCapturer.CaptureDuration`: 453 samples, mean 8.0ms
- `Graphics.Smoothness.PercentDroppedFrames3.AllSequences`: 21.0%

#### Key Findings

1. **`do_message_loop_work()` takes >1ms on every call.** This is not a fast
   function with occasional spikes — it consistently consumes 1-33ms per call.
   It is the dominant cost in the loop, accounting for >95% of iteration time.

2. **CFRunLoop sources almost never fire.** 96% of iterations, CFRunLoop returns
   in <0.1ms with nothing to do. The run loop is not what's driving frame
   production — `do_message_loop_work()` is. CFRunLoop's role is limited to the
   4% of iterations where it actually services a source.

3. **Only 865 iterations in 13.4 seconds.** At ~65 iterations/sec, the loop runs
   far slower than the theoretical 1000/sec that a 1ms sleep would produce.
   `do_message_loop_work()` consumes the time budget, leaving almost nothing for
   CFRunLoop.

4. **The worst-case 33ms spike matches the 30fps drops exactly.** When
   `do_message_loop_work()` blocks for 33ms, it consumes two entire vsync
   periods in a single call, producing the characteristic 33ms frame interval.

5. **82.7% at 60fps — higher than baseline (71%).** The `Instant::now()` calls
   add ~60ns of overhead per iteration, which may subtly alter the interleaving
   rhythm. This is noise, not a real improvement, but it suggests the system is
   sensitive to tiny timing changes.

#### Conclusion

**H1 is confirmed** — the polling loop timing mismatch is real, but it's not
drift or jitter. It's that `do_message_loop_work()` itself is the bottleneck.
It blocks for variable durations (1-33ms), consuming the entire frame budget
and leaving CFRunLoop with almost no time to service its sources.

This reframes the problem: the Issue 342 CFRunLoop fix helped not because
CFRunLoop sources needed to fire frequently, but because the 4% of iterations
where a source fires are critical — they're the SyntheticBeginFrameSource timer
ticks that trigger compositor cycles. Without CFRunLoop servicing, those 4% of
critical moments never happen at all.

**Implications for next experiments:**

- **Idea 3 (remove `do_message_loop_work()`) is now the most interesting.** If
  `do_message_loop_work()` is consuming 95% of the time and CFRunLoop sources
  drive the actual frame scheduling, what happens if we let CFRunLoop run
  longer and call `do_message_loop_work()` less frequently — or not at all?
- **Idea 7 (two-phase `external_message_pump`)** is also motivated: cooperative
  scheduling via `on_schedule_message_pump_work` would let CEF tell us exactly
  when it needs `do_message_loop_work()`, instead of calling it blindly every
  iteration.
- **Idea 4 (increase CFRunLoop timeout)** deserves revisiting. With mlw taking
  1-33ms anyway, increasing the CFRunLoop timeout from 1ms to 16ms wouldn't
  change the loop cadence much — but it would give CFRunLoop sources much more
  opportunity to fire during the 96% of idle iterations.
