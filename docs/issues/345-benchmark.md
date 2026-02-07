# Issue 345: `web benchmark` — Automated Framerate Benchmark for ts3

## Background

### The performance gap

The cef-rs OSR example renders at ~60fps in-process. ts3's profile server
renders at 38fps. Issue 343 ran eight experiments trying to close the gap and
failed. Issue 344 built cef-test — a minimal multi-process CEF harness — to
isolate the cause. cef-test achieved ~50fps with two profiles running
simultaneously, proving the multi-process architecture is sound.

### The initial hypothesis (wrong)

Issue 344's conclusion hypothesized that the 12fps gap between cef-test (50fps)
and ts3 (38fps) was caused by WezTerm integration overhead — texture import,
redraw scheduling, or interaction with the terminal renderer.

### The revised hypothesis (input rate)

The real difference between cef-test's 50fps and ts3's 38fps is likely much
simpler: **input rate**.

cef-test's benchmark sends simulated scroll events at ~125Hz (8ms intervals)
directly inside the profile server via `BrowserHost::send_mouse_wheel_event()`.
This bypasses all input routing and guarantees a continuous stream of events
that force CEF to re-render every frame.

ts3's input path is very different:

```
User scrolls mouse
    │
    ▼
macOS generates scroll events (~125Hz hardware rate)
    │
    ▼
winit receives NSEvent in WezTerm's event loop
    │
    ▼
WezTerm routes through pane system
    │
    ▼
Serialized to XPC dictionary
    │
    ▼
Sent over XPC to profile server
    │
    ▼
Profile server calls host.send_mouse_wheel_event()
    │
    ▼
CEF renders
```

If any stage in this pipeline drops events, batches them, or introduces latency,
the effective input rate reaching CEF could be far lower than 125Hz. A lower
input rate means fewer scroll events per second, which means fewer frames where
the page content actually changes, which means fewer `on_accelerated_paint`
callbacks, which means lower measured fps.

In other words: **ts3 might not be rendering slowly — it might not be asking CEF
to render often enough**, because the input isn't arriving fast enough.

### Why we can't test this with manual scrolling

The 38fps measurement in ts3 was taken while manually scrolling with a mouse.
But manual scrolling introduces uncontrolled variables:

- **Inconsistent scroll speed** — humans don't scroll at a constant rate
- **Scroll direction changes** — pauses when reversing direction
- **Page boundary stalls** — hitting the top or bottom of the page stops
  rendering entirely
- **Different content** — the page being scrolled affects rendering cost

cef-test solved this by simulating scrolling directly in the profile server. ts3
needs the same capability to produce a valid comparison.

## Plan

### The `web benchmark` command

Add a new command to ts3:

```
web benchmark
```

This command:

1. Opens `https://www.google.com/search?q=asdf+asdf` in a webview pane
2. Waits for the page to load
3. Begins simulated scrolling at ~125Hz directly in the profile server
4. Runs for 70 seconds
5. Collects and prints framerate statistics
6. Closes the webview

The simulated scrolling happens entirely inside the profile server — no mouse
events traverse the GUI → XPC → profile path. This eliminates input routing as a
variable and isolates the rendering pipeline.

### Simulated scroll behavior

Identical to cef-test's Phase 8:

- Scroll events at 8ms intervals (~125Hz)
- `send_mouse_wheel_event()` called directly in the profile server's message
  loop
- Direction reverses every 25 events (~200ms) to keep the page in continuous
  motion without hitting top/bottom stalls
- Mouse position fixed at viewport center
- Delta of 120 per event (one standard scroll notch)

### Statistics output

After 70 seconds, close the webview, and then print to the terminal:

```
=== ts3 Benchmark (70s) ===

50.0 fps | 80.8% at 60fps | streak: 139 | p50: 16.7ms | p95: 33.6ms

3252 frames over 65.0s
```

This matches cef-test's benchmark output format for direct comparison.

### What does NOT happen

- **No keyboard input** is forwarded to the browser
- **No mouse input** is forwarded to the browser
- **No manual interaction** is required or accepted
- The benchmark is fully automated and deterministic

## Expected Outcomes

### If ts3 benchmark matches cef-test (~50fps)

The input hypothesis is confirmed. ts3's rendering pipeline is fine — the 38fps
measurement was caused by insufficient input rate during manual scrolling. The
fix is to improve the input forwarding path: ensure scroll events reach the
profile server at the full hardware rate (~125Hz) without batching or dropping.

### If ts3 benchmark is still ~38fps

The input hypothesis is wrong. The problem IS in ts3's integration — likely in
how the GUI imports IOSurface textures, schedules redraws, or coordinates with
WezTerm's terminal renderer. In this case, the 12fps gap is real overhead, and
the investigation moves to the rendering path.

### Either way, we learn something definitive

Just like cef-test eliminated the architecture question, this benchmark
eliminates the input question. One test, one answer.

## Implementation Notes

### How `web benchmark` reaches the profile server

The `web` command already sends a JSON message over a Unix socket to the GUI,
which triggers an XPC `spawn_profile` to the launcher. The benchmark variant
needs to tell the profile server to enable simulated scrolling. Options:

1. **CLI arg to profile server** — Add `--benchmark` flag to `termsurf-profile`.
   When set, the profile server simulates scrolling after page load, exactly
   like cef-test-profile does. The GUI passes this flag through the launcher's
   `spawn_profile` message.

2. **XPC command after connection** — GUI sends a `start_benchmark` XPC message
   to the profile server after the connection is established. Profile server
   starts simulated scrolling when it receives this message.

Option 1 is simpler and matches cef-test's approach.

### Collecting statistics

The profile server already has frame timing instrumentation from Issue 343
experiments. The benchmark mode adds:

- `FrameStats` tracking (same as cef-test-gui's implementation)
- After 70 seconds, print the summary to the profile server's log
- The `web benchmark` command reads the summary from the log and prints it to
  the terminal

Alternatively, the profile server can send the statistics back to the GUI via
XPC, and the GUI prints them. This is cleaner but requires a new XPC message
type.

### Relationship to cef-test's benchmark

| Aspect             | cef-test benchmark          | ts3 `web benchmark`         |
| ------------------ | --------------------------- | --------------------------- |
| GUI                | Bare winit + wgpu           | WezTerm (full terminal)     |
| Profile server     | cef-test-profile            | termsurf-profile            |
| Input simulation   | Direct in profile server    | Direct in profile server    |
| IOSurface transfer | XPC Mach port               | XPC Mach port               |
| Texture import     | Standalone wgpu pipeline    | WezTerm's wgpu integration  |
| Event loop         | Simple pump_app_events      | WezTerm's event loop        |
| Scroll rate        | ~125Hz (8ms)                | ~125Hz (8ms)                |
| Duration           | 70s                         | 70s                         |
| Input routing      | None (simulated in-process) | None (simulated in-process) |

The only differences are on the GUI side: WezTerm's renderer vs bare wgpu. If
the results match (~50fps), then WezTerm's integration is not the problem and
the input path was the bottleneck all along. If they don't match, WezTerm's
integration is the culprit and we can bisect it.
