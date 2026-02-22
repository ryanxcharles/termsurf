# Issue 622: JavaScript Is Slow

## Goal

Identify and fix the Chromium mechanism that throttles JavaScript-driven
rendering to 2fps when two BrowserContexts coexist in a single process. The fix
must allow two profiles, each running `requestAnimationFrame` loops, to both
render at 60fps.

## Background

Two prior issues (620, 621) systematically narrowed a 2fps rendering degradation
across 20 experiments. The result: **JavaScript execution on the Blink main
thread is the sole trigger.** Everything else — the compositor, the GPU
pipeline, the viz frame delivery system — is clean.

### What's fast

Two BrowserContexts with **CSS-only animations** both render at 60fps. CSS
`@keyframes` animations run in the compositor thread. They generate continuous
compositor damage every vsync — new CompositorFrames, new draw calls, new GPU
commands — yet two profiles handle this without any degradation.

Two BrowserContexts both loading **lite.duckduckgo.com** (a static HTML form
with virtually no JavaScript) also render at 60fps.

This proves the compositor thread, GPU command serialization, paint layer
complexity, and compositor damage frequency are all fine.

| Profile A           | Profile B           | A fps | B fps | Experiment |
| ------------------- | ------------------- | ----- | ----- | ---------- |
| CSS animation       | CSS animation       | 60    | 60    | 621.4      |
| lite.duckduckgo.com | lite.duckduckgo.com | 60    | 60    | 621.3      |

### What's slow

Two BrowserContexts both running **any JavaScript animation** degrade to 2fps.
This includes google.com (heavyweight: analytics, autocomplete, service workers,
ad scripts) and the ts4 box demo (lightweight: a 30-line `requestAnimationFrame`
loop drawing one rectangle on a 300x300 canvas). The degradation is identical
regardless of JavaScript complexity — even the most trivial rAF loop triggers
it.

| Profile A         | Profile B         | A fps | B fps | Experiment |
| ----------------- | ----------------- | ----- | ----- | ---------- |
| google.com        | google.com        | 2     | 2     | 621.2      |
| JS box demo (rAF) | JS box demo (rAF) | 2     | 2     | 621.5      |

### What's mixed

When one profile runs JavaScript and the other doesn't, only the JavaScript
profile degrades. The non-JavaScript profile is unaffected.

google.com (continuous JS) paired with lite.duckduckgo.com (no JS): google drops
to 2fps, DDG stays at 60fps. Reversing the profile order reverses which window
is slow — it's always the one running JavaScript, regardless of which
BrowserContext it belongs to.

| Profile A           | Profile B           | A fps | B fps | Experiment |
| ------------------- | ------------------- | ----- | ----- | ---------- |
| google.com          | lite.duckduckgo.com | 2     | 60    | 620.14     |
| lite.duckduckgo.com | google.com          | 60    | 2     | 620.15     |

### What the viz pipeline research eliminated

Issue 620 Experiments 12–15 instrumented the entire viz/compositor pipeline.
BeginFrames arrive at 60fps to both profiles. The renderer receives them but
only produces CompositorFrames at ~3fps for JavaScript-heavy pages. Every
throttle mechanism in the viz pipeline was checked and either never triggered or
confirmed as a symptom rather than a root cause:

- StopObservingBeginFrames — symptom, fixed in 620 Exp 13
- ShouldDraw() gate — healthy except `needs_draw_`
- CVDisplayLink thrashing — observed but not causal
- BeginFrameTracker throttle — never triggered
- kUndrawnFrameLimit — never triggered
- root_frame_missing() — reinforces the stall but doesn't cause it

### The unexplored layer

The bottleneck is between the compositor thread (which receives BeginFrames at
60fps) and the Blink main thread (which executes `requestAnimationFrame`
callbacks). This interface — **BeginMainFrame dispatch** — is where the
compositor tells the main thread "start your frame work now." When two
BrowserContexts both have active rAF loops, something in this layer serializes
or throttles the callbacks.

Key areas to investigate:

- **Renderer process allocation**
  (`content/browser/renderer_host/render_process_host_impl.cc`) — do two
  BrowserContexts get separate renderer processes, or share one? If they share a
  process, there's literally one Blink main thread running both rAF loops.
- **Blink's main thread scheduler**
  (`third_party/blink/renderer/platform/scheduler/`) — how it prioritizes and
  dispatches tasks across multiple renderer contexts
- **BeginMainFrame** — the compositor-to-main-thread signal that triggers rAF
  callbacks, style recalc, layout, and paint
- **ProxyMain / ThreadProxy** (`cc/trees/`) — the cc-layer interface between the
  compositor thread and the main thread

## Approach

Research the Chromium source code first, guided by the precise signal from
Issues 620–621. Previous searches were blind — now we know the bottleneck is
JavaScript on the Blink main thread, not the compositor or GPU. Start by
answering the critical architectural question: do two BrowserContexts share a
renderer process? The answer determines the entire investigation direction.

If a likely culprit is identified, design experiments to confirm and fix it.
