# Issue 350: Event-Driven CEF Message Pump

## Background

The profile server (`termsurf-profile`) needs to call CEF's
`do_message_loop_work()` to drive rendering. The current implementation uses a
busy-wait loop that burns one full CPU core at 100%:

```rust
while !QUIT_FLAG.load(Ordering::Relaxed) {
    cef::do_message_loop_work();
    cfrunloop::run_for(0.000);  // returns immediately = infinite loop
}
```

This achieves ~50fps but causes thermal throttling within minutes, making
performance benchmarks unreliable and wasting energy during normal use.

CEF provides `external_message_pump` mode with the
`on_schedule_message_pump_work(delay_ms)` callback — CEF tells you exactly when
to call `do_message_loop_work()`. Between callbacks, the process can sleep. This
is the intended API for driving CEF from a custom event loop.

### Prior art

| Implementation               | Architecture             | Main loop                     | Performance      |
| ---------------------------- | ------------------------ | ----------------------------- | ---------------- |
| cef-rs reference             | In-process, windowed     | `NSApp().run()`               | 60fps            |
| ts2                          | In-process with WezTerm  | WezTerm's AppKit loop         | 60fps            |
| ts3 busy-wait                | Out-of-process, headless | `while { work(); sleep(0); }` | ~50fps, 100% CPU |
| ts3 Experiment 3 (Issue 349) | Out-of-process, headless | `CFRunLoopRun()` + timers     | ~29fps, low CPU  |

The gap between the busy-wait (~50fps) and the event-driven attempt (~29fps) is
the problem this issue aims to close.

### What Issue 349 Experiment 3 did

- Added `external_message_pump: 1` to CEF settings
- Implemented `on_schedule_message_pump_work` in `ProfileBPH`
- Created `cef_pump` module with reentrancy guard and 33ms fallback timer
- Replaced the while-loop with `CFRunLoopRun()` (blocking, event-driven)
- Moved benchmark scroll simulation to an 8ms repeating CFRunLoop timer

The pump ran without crashing or deadlocking — a first for ts3. But fps dropped
from ~50 to ~29, with p50 shifting from 17ms to 25ms.

### Open questions from Issue 349

1. **Is the callback actually firing?** We assumed it stops being reliable, but
   never logged it. The problem might be response latency, not callback silence.

2. **Timer creation overhead.** Every `on_schedule_message_pump_work` call
   creates a new `CFRunLoopTimer` — even for delay=0. Timer allocation + run
   loop registration is heavyweight compared to signaling a `CFRunLoopSource`.

3. **Benchmark timer contention.** The 8ms scroll timer runs on the main thread.
   While its callback executes, no other timer (including the CEF pump timer)
   can fire. This steals up to 12.5% of available time.

4. **Multiple pump calls per frame.** CEF may need several
   `do_message_loop_work()` calls to push one frame through its pipeline. The
   busy-wait loop provides thousands of calls/sec. The event-driven pump
   provides ~30/sec (fallback timer). If frames need 5-10 calls each, 30/sec is
   nowhere near enough.

5. **CFRunLoopRun() vs NSApp().run().** The reference uses AppKit's full event
   loop. A headless process can create NSApplication with activation policy
   `.prohibited`. CEF's Chrome runtime may depend on AppKit event dispatching.

## Ideas

### Idea 1: Diagnostic logging

Add logging to `on_schedule_message_pump_work` (timestamp, delay_ms, thread) and
`pump_timer_callback` (timestamp, was it a fallback?) to see whether the callback
fires reliably or if we're running on the 33ms fallback. This is the most
important first step — it tells us which of the open questions matters.

### Idea 2: Reduce fallback timer to 1ms

Change `MAX_TIMER_DELAY_MS` from 33 to 1. If fps jumps to ~50, the callback
wasn't firing and we were running on the fallback — a 1ms fallback gives 1000
pumps/sec with negligible CPU. Cheapest possible test.

### Idea 3: CFRunLoopSource for immediate work

Replace `CFRunLoopTimer` with a persistent `CFRunLoopSource` for delay=0
requests. Signaling a source is just setting a flag — no timer allocation. It
triggers on the very next run loop iteration.

### Idea 4: NSApp().run() in the headless process

A headless process can create NSApplication with activation policy `.prohibited`
(no dock icon, no menu). If CEF needs AppKit event dispatching internally, this
would close the gap between bare `CFRunLoopRun()` and the reference
implementation's `NSApp().run()`.

## Experiments

### Experiment 1: Diagnostic logging of the pump cycle

**Goal:** Determine whether the performance gap is caused by the callback not
firing, or by our timer scheduling adding too much latency. Without this data,
every fix is a guess.

**Method:** Add counters and per-second summary logging to the `cef_pump` module.
No architectural changes — just instrumentation.

**What to measure (per-second counters):**

1. `schedule_work` calls with delay=0 (CEF wants immediate work)
2. `schedule_work` calls with delay>0 (CEF wants deferred work)
3. Fallback timer fires (the 33ms timer scheduled when no callback arrives)
4. Callback-scheduled timer fires (timers created from `schedule_work`)
5. Reentrancy detections (`is_active` was true when timer fired)
6. `do_message_loop_work()` calls actually executed

Print a one-line summary every second:

```
[PUMP] callbacks=247(imm=180 def=67) fires=62(cb=31 fb=31) reentrant=0 work=62
```

This tells us:

- **callbacks >> fires**: timer scheduling is the bottleneck — callbacks fire
  but timers don't keep up (new timers supersede old ones before they fire)
- **fires ≈ 30, mostly fallback**: callback isn't driving the pump — we're
  running at the fallback rate
- **callbacks ≈ 30**: CEF itself only requests work ~30 times/sec — the problem
  is upstream of our scheduling
- **callbacks >> 60, fires ≈ 60**: pump is working correctly, problem is
  elsewhere (GUI-side presentation, etc.)

**What to change in `cef_pump`:**

Add atomic counters (no mutex needed for counters):

```rust
static SCHED_IMMEDIATE: AtomicU64 = AtomicU64::new(0);
static SCHED_DEFERRED: AtomicU64 = AtomicU64::new(0);
static FIRE_CALLBACK: AtomicU64 = AtomicU64::new(0);
static FIRE_FALLBACK: AtomicU64 = AtomicU64::new(0);
static REENTRANT: AtomicU64 = AtomicU64::new(0);
static WORK_DONE: AtomicU64 = AtomicU64::new(0);
```

In `schedule_work`: increment `SCHED_IMMEDIATE` or `SCHED_DEFERRED` based on
whether the original `delay_ms` was <= 0 or > 0. Track whether the timer being
scheduled is a fallback (called from `pump_timer_callback` with
`MAX_TIMER_DELAY_MS`) by adding a `is_fallback: bool` parameter or a separate
`schedule_fallback` function.

In `pump_timer_callback`: increment `FIRE_CALLBACK` or `FIRE_FALLBACK` based on
whether this timer was scheduled by a callback or the fallback path. Increment
`WORK_DONE` after each `do_message_loop_work()`. Increment `REENTRANT` when
`is_active` is true.

Print the summary every second using a timestamp check in `pump_timer_callback`
(since it's the only code that runs regularly on the main thread).

**Status:** Not started
