# Issue 629: Understand Navigation Blank in CALayerHost

## Goal

Understand **why** the browser overlay disappears for ~10 seconds when the user
clicks a link. This is a research issue — the goal is diagnosis, not a fix.

## Background

### The CALayerHost migration

[Issue 625](625-calayerhost.md) replaced the `FrameSinkVideoCapturer` pipeline
with `CALayerHost`. Instead of capturing IOSurface frames at 120fps and
transferring Mach ports over XPC every frame, Chromium now sends a
`ca_context_id` (uint32) once per tab. The GUI creates a `CALayerHost` sublayer,
and Window Server composites the remote content directly from GPU VRAM.

This migration broke several things that worked under the old pipeline:

- [Issue 626](626-x-y-calayerhost.md) — X/Y positioning was offset. Fixed.
- [Issue 627](627-resize-calayerhost.md) — Resize stopped working. Fixed.
- [Issue 628](628-navigation-calayerhost.md) — Navigation causes a ~10s blank.
  **Unresolved after 8 experiments.**

Under the old IOSurface pipeline, navigation was invisible — every frame
delivered a new Mach port, and the Metal shader re-read the texture every frame
in `drawFrame()`. The new surface just showed up. With CALayerHost, there is no
per-frame update. The `ca_context_id` is set once, and Window Server composites
from that context. When navigation produces a new `CAContext`, the old one
becomes invalid and the overlay goes blank.

There may be additional regressions from the CALayerHost migration that haven't
been tested yet.

### What Issue 628 tried and failed

Issue 628 ran 8 experiments targeting the Chromium-side pipeline. All failed to
fix the ~10-second blank:

| Exp | Approach                                               | Result             |
| --- | ------------------------------------------------------ | ------------------ |
| 1   | Re-register callback on view swap, replace CALayerHost | No effect on blank |
| 2   | Re-apply size in `RenderViewHostChanged`               | Fail               |
| 3   | Research: Electron/Chromium sizing                     | Research only      |
| 4   | Resize NSWindow instead of `view->SetSize()`           | No effect on blank |
| 5   | Research: navigation transitions, dedup gate           | Research only      |
| 6   | Set fallback surface before navigation                 | Fail               |
| 7   | Diagnostic logging                                     | Research only      |
| 8   | Reduce dedup gate to 100ms                             | Fail               |

Key finding from Experiment 7's diagnostic logging: Chromium sends the new
`ca_context_id` within 100ms of the click. The page loads in ~70ms. The GUI
receives the ID and replaces the `CALayerHost` immediately. Yet the new
`CALayerHost` shows nothing for ~10 seconds.

All code changes from Issue 628 should be reverted — none had any effect.

### What we know

1. **Chromium is fast.** The new `ca_context_id` arrives in ~100ms. The page
   loads in ~70ms.
2. **The GUI is fast.** The `CALayerHost` is replaced immediately upon receiving
   the new ID.
3. **The blank is ~10 seconds.** Suspiciously consistent.
4. **The problem is NOT:** callback lifecycle, compositor surface fallback,
   dedup gate timing, NSWindow sizing, or `SetSize()` vs `setContentSize:`.
5. **The problem likely IS:** something in the Window Server's handling of
   cross-process CAContext/CALayerHost connections, or something about how the
   hidden NSWindow interacts with CAContext compositing.

### What we don't know

- Does the blank happen with a **visible** Chromium window? If not, the hidden
  window is the cause.
- Is the new `CAContext` actually producing content when the `CALayerHost`
  connects to it? Or is it empty?
- Does macOS have an internal timeout for establishing cross-process CAContext
  connections (`CARemoteLayerServer` / `CARemoteLayerClient`)?
- Does `CALayerHost` need an explicit trigger (e.g., `setNeedsDisplay`,
  `CATransaction`) to start displaying a newly-connected remote context?
- Is the old `CAContext` torn down before the new one is ready, creating a gap
  where no context has content?

### Chromium branch

Start from `146.0.7650.0-issue-627` (discarding Issue 628's branch). Create
`146.0.7650.0-issue-629` if any Chromium changes are needed for diagnostics.

### GUI revert

Revert `a73f3e1` (`gui/src/renderer/Metal.zig` — CALayerHost replacement logic
from Issue 628 Experiment 1) before starting experiments.

## Experiments

### Experiment 1: Compare Electron and Chromium CALayerHost usage to TermSurf

#### Problem

We don't understand why the overlay goes blank for ~10 seconds during
navigation. Issue 628 spent 8 experiments modifying the Chromium pipeline with
no effect. Before trying more fixes, we need to understand how our CALayerHost
usage differs from the working implementations.

#### Research questions

**R1: Does Electron use CALayerHost at all?**

How does Electron's off-screen rendering display compositor output on macOS?
Does it create a `CALayerHost` with a `ca_context_id`, or does it use a
completely different mechanism?

**R2: How does normal Chromium use CALayerHost?**

In a standard Chrome window, how does `DisplayCALayerTree` manage the
`CALayerHost`? What is the layer tree structure? What happens during navigation
when the `ca_context_id` changes?

**R3: How does TermSurf use CALayerHost?**

Trace the full pipeline: Chromium Profile Server sends `ca_context_id` over XPC
→ GUI creates `CALayerHost`. What layer tree structure do we use? How does it
differ from Chromium's `DisplayCALayerTree`?

**R4: What are the architectural differences?**

Compare the three approaches side by side. Identify anything TermSurf does
differently that could explain the 10-second blank.

#### Results

**R1: Electron does NOT use CALayerHost.**

Electron's off-screen rendering on macOS intercepts `CALayerParams` at the
`HostDisplayClient` level and extracts the IOSurface directly — it never creates
a `CALayerHost`:

```cpp
// vendor/electron/shell/browser/osr/osr_host_display_client_mac.mm
void OffScreenHostDisplayClient::OnDisplayReceivedCALayerParams(
    const gfx::CALayerParams& ca_layer_params) {
  if (!ca_layer_params.is_empty) {
    IOSurfaceRef io_surface = IOSurfaceLookupFromMachPort(
        ca_layer_params.io_surface_mach_port.get());
    void* pixels = IOSurfaceGetBaseAddress(io_surface);
    SkBitmap bitmap;
    bitmap.installPixels(..., pixels, stride);
    callback_.Run(ca_layer_params.damage, bitmap, {});
  }
}
```

Electron reads pixels from the IOSurface Mach port on every frame. It has two
rendering paths:

1. **Hardware accelerated:** `FrameSinkVideoCapturer` (the same pipeline
   TermSurf used before Issue 625).
2. **Software:** `CALayerParams` → extract IOSurface → read pixels → SkBitmap.

Neither path involves `CALayerHost`. Electron sidesteps the entire
CAContext/CALayerHost mechanism. This means **Electron cannot tell us anything
about CALayerHost navigation behavior** — they don't use it.

**R2: Normal Chromium uses CALayerHost inside a visible window.**

In stock Chrome, `DisplayCALayerTree` (in the browser process) creates a
`CALayerHost` inside the window's NSView layer tree:

```
RenderWidgetHostViewCocoa (NSView, wantsLayer=YES)
└─ background_layer_ (CALayer, view's backing layer)
   └─ maybe_flipped_layer_ (CALayer, geometryFlipped=YES)
      └─ remote_layer_ (CALayerHost, contextId = ca_context_id)
```

Key details from `display_ca_layer_tree.mm`:

- `GotCALayerFrame()` creates a **new** `CALayerHost` when `ca_context_id`
  changes (never updates `contextId` on an existing host).
- Uses `ScopedCAActionDisabler` to suppress CALayer animations during the swap.
- Adds the new host **before** removing the old one — atomic visual swap.
- The NSView is in a **visible** window on screen.

When `SetCALayerParams()` is called on the NSView, it calls
`DisplayCALayerTree::UpdateCALayerTree()` which calls `GotCALayerFrame()`. This
happens inside `AcceleratedWidgetCALayerParamsUpdated()`.

**R3: TermSurf uses CALayerHost cross-process with a hidden intermediary.**

TermSurf's pipeline:

1. Chromium Profile Server runs with a **hidden** NSWindow
   (`[window orderOut:nil]`).
2. Inside that hidden window, the standard Chromium pipeline runs:
   `RenderWidgetHostViewCocoa` → `DisplayCALayerTree` → `CALayerHost`. This
   `CALayerHost` lives inside the hidden window and points at the GPU process's
   `CAContext`.
3. We hook `SetCALayerParamsCallback` on the `RenderWidgetHostViewMac` to
   intercept the `ca_context_id`.
4. We send the `ca_context_id` over XPC to the TermSurf GUI (a completely
   separate process).
5. The GUI creates its **own** `CALayerHost` in its Metal renderer's layer tree:

```
IOSurfaceLayer (Metal renderer)
└─ flipped_layer (geometryFlipped=YES)
   └─ positioning_layer (explicit frame at overlay grid rect)
      └─ CALayerHost (contextId = ca_context_id from XPC)
```

So there are **two CALayerHosts** pointing at the same `CAContext`:

1. One inside the Chromium Profile Server's hidden window (created by
   `DisplayCALayerTree`, standard Chromium behavior).
2. One inside the TermSurf GUI (created by `Metal.zig`).

**R4: Architectural differences.**

| Aspect               | Normal Chrome            | Electron OSR            | TermSurf                          |
| -------------------- | ------------------------ | ----------------------- | --------------------------------- |
| Mechanism            | CALayerHost              | IOSurface pixel read    | CALayerHost                       |
| CALayerHost location | Browser window (visible) | N/A                     | GUI process (visible)             |
| Intermediate window  | None                     | Placeholder NSView      | Hidden NSWindow                   |
| CALayerHost count    | 1 per CAContext          | 0                       | **2 per CAContext**               |
| Window visibility    | Visible                  | N/A                     | Hidden                            |
| Process topology     | GPU → Browser            | GPU → Browser (extract) | GPU → Server (hidden) → XPC → GUI |

Three critical differences in TermSurf:

1. **Two CALayerHosts per CAContext.** The hidden window's `DisplayCALayerTree`
   creates one, and our GUI creates another. Both point at the same `CAContext`.
   macOS may not support multiple `CALayerHost` instances for the same
   `CAContext`, or the hidden window's host may interfere with the GUI's host.

2. **Hidden intermediary window.** The Chromium Profile Server's NSWindow is
   hidden via `[window orderOut:nil]`. The `DisplayCALayerTree` inside that
   window still runs and manages its own `CALayerHost`. The Window Server may
   deprioritize or defer compositing for off-screen windows, affecting the
   `CAContext` that both hosts share.

3. **No `ScopedCAActionDisabler`.** Chromium wraps CALayerHost creation in
   `ScopedCAActionDisabler` to suppress Core Animation's implicit animations
   (fade-in, position interpolation). Our GUI code does not do this. A 0.25s
   fade-in animation wouldn't explain a 10-second blank, but other implicit
   animation behaviors might.

#### Conclusion

Electron is irrelevant — they don't use CALayerHost at all. The comparison that
matters is TermSurf vs. normal Chrome.

The most suspicious difference is **two CALayerHosts pointing at the same
CAContext**. In normal Chrome, there is exactly one `CALayerHost` per
`CAContext`. In TermSurf, the hidden window's `DisplayCALayerTree` creates one,
and the GUI creates a second. This is an untested configuration — macOS may not
properly handle it.

The next experiment should test whether eliminating the hidden window's
`CALayerHost` (by disabling `DisplayCALayerTree` or by not calling
`SetCALayerParams` on the hidden NSView) resolves the blank. Alternatively, test
whether making the hidden window visible fixes the problem — this would confirm
that window visibility affects `CAContext` compositing.

#### Verification

Research is complete when all four questions are answered and we have a concrete
hypothesis for the next experiment.

### Experiment 2: Deep dive into the CAContext lifecycle during navigation

#### Problem

Experiment 1 identified three critical differences between TermSurf and stock
Chrome, but it didn't explain _why_ the blank lasts ~10 seconds. We need to
trace the exact lifecycle of CAContext creation and destruction during
navigation, understand whether the hidden window's `DisplayCALayerTree`
interferes with our GUI's `CALayerHost`, and determine whether
`DisableDisplay()` is the key mechanism Chrome uses to avoid the problem.

#### Research questions

**R1: What is the exact CAContext lifecycle during navigation?**

When a user clicks a link, the renderer process may swap (cross-site navigation)
or the compositor may recreate its output surface (same-site). In either case, a
new `CALayerTreeCoordinator` is created in the GPU process, which allocates a
new `CAContext` via `+[CAContext contextWithCGSConnection:options:]`. This gives
a new `ca_context_id`.

Trace the full sequence:

1. Navigation commits.
2. `RenderViewHostChanged` fires (if cross-site) — or the compositor swaps the
   output surface (same-site).
3. Old `CALayerTreeCoordinator` is destroyed → old `CAContext` is released.
4. New `CALayerTreeCoordinator` is created → new `CAContext` → new
   `ca_context_id`.
5. `AcceleratedWidgetCALayerParamsUpdated()` fires with new params.
6. `ns_view_->SetCALayerParams()` → hidden window's
   `DisplayCALayerTree::GotCALayerFrame()` creates a `CALayerHost` with the new
   `ca_context_id`.
7. `ca_layer_params_callback_` sends the `ca_context_id` over XPC.
8. GUI receives it and creates its own `CALayerHost`.

Questions: Does step 6 happen before step 8? (Yes — they're in the same
function, and `SetCALayerParams` is called first.) Does the hidden window's host
"claim" the CAContext before the GUI can connect? What is the timing gap between
step 6 and step 8?

**R2: Is `DisableDisplay()` ever called in the Chromium Profile Server?**

`DisableDisplay()` on `RenderWidgetHostNSViewBridge` destroys the
`DisplayCALayerTree` and sets `display_disabled_ = true`. After that,
`SetCALayerParams()` returns early — no `CALayerHost` is created in the hidden
window.

The only call site is `RenderWidgetHostViewMac::SetParentUiLayer()`, which fires
only when a `parent_ui_layer` (Views/Aura compositor layer) is provided. The
comment at line 362 explicitly notes: "not all code has been updated to use
`ui::Views` (e.g, `content_shell`)".

Since the Chromium Profile Server is based on `content_shell`, and
`content_shell` does not use `ui::Views` / `ui::Layer` compositing,
`SetParentUiLayer()` is never called with a non-null layer. Therefore
`DisableDisplay()` is **never called** in our pipeline.

This means the hidden window's `DisplayCALayerTree` is **always active**. Every
frame, it receives `SetCALayerParams()` and maintains its own `CALayerHost`
inside the hidden window's NSView.

**R3: How does Chrome's Views UI eliminate the dual-host problem?**

In stock Chrome (not `content_shell`), the browser uses `ui::Views` for
rendering. When `SetParentUiLayer()` is called with a valid layer:

1. `DisableDisplay()` is called on the `RenderWidgetHostNSViewBridge`.
2. The `DisplayCALayerTree` is destroyed (`display_ca_layer_tree_.reset()`).
3. `display_disabled_ = true` prevents future `SetCALayerParams()` calls from
   creating any `CALayerHost` in the NSView.
4. Instead, the `ui::Compositor` (parent layer compositor) handles display. The
   `CALayerHost` lives in the Views compositor's layer tree, not in the NSView's
   `DisplayCALayerTree`.

This means in stock Chrome with Views, there is exactly **one CALayerHost per
CAContext** — the one in the Views compositor. The NSView's `DisplayCALayerTree`
is disabled. This is the normal path for Chrome on macOS.

In our Chromium Profile Server (content_shell-based), `DisableDisplay()` is
never called, so the NSView's `DisplayCALayerTree` stays alive and creates a
**second CALayerHost** for the same CAContext. This is the dual-host problem
identified in Experiment 1.

**R4: What is the exact code flow in
`AcceleratedWidgetCALayerParamsUpdated()`?**

```cpp
void RenderWidgetHostViewMac::AcceleratedWidgetCALayerParamsUpdated() {
  SetBackgroundLayerColor(last_frame_root_background_color_);
  const gfx::CALayerParams* ca_layer_params =
      browser_compositor_->GetLastCALayerParams();
  if (ca_layer_params) {
    ns_view_->SetCALayerParams(*ca_layer_params);    // (A) Hidden window host
    if (ca_layer_params_callback_)
      ca_layer_params_callback_.Run(*ca_layer_params); // (B) Our XPC callback
  }
}
```

Step (A) calls `RenderWidgetHostNSViewBridge::SetCALayerParams()` →
`DisplayCALayerTree::UpdateCALayerTree()` → `GotCALayerFrame()`. This creates a
`CALayerHost` in the hidden window with the new `ca_context_id`. Because
`display_disabled_` is false (see R2), this always runs.

Step (B) sends the `ca_context_id` over XPC to the GUI. The GUI then creates its
own `CALayerHost`.

The hidden window's `CALayerHost` is always created **before** the GUI's, in the
same function call. Both point at the same `CAContext`.

**R5: Where is the CAContext created and what happens to the old one?**

`CAContext` is created in `CALayerTreeCoordinator::CALayerTreeCoordinator()`
(`ui/accelerated_widget_mac/ca_layer_tree_coordinator.mm:29-68`):

```cpp
ca_context_ = [CAContext contextWithCGSConnection:connection_id options:@{}];
ca_context_.layer = root_ca_layer_;
```

Each `ImageTransportSurfaceOverlayMacEGL` (GPU output surface) creates one
`CALayerTreeCoordinator`. During navigation, the old output surface is destroyed
(old `CAContext` released) and a new one is created (new `CAContext` with a new
`ca_context_id`).

The `ca_context_id` is read from `[ca_context_ contextId]` in
`CommitPresentedFrameToCA()` and sent to the browser process via the swap
completion callback.

#### Results

**R1: The CAContext lifecycle during navigation is well-defined.**

The GPU process creates one `CAContext` per output surface
(`CALayerTreeCoordinator`). Navigation destroys the old surface and creates a
new one, producing a new `ca_context_id`. The old `CAContext` is released when
its `CALayerTreeCoordinator` destructor runs.

The exact sequence is:

1. Navigation commits → `RenderViewHostChanged` fires.
2. New `RenderWidgetHostView` is created with a new `BrowserCompositorMac`.
3. GPU process creates a new `ImageTransportSurfaceOverlayMacEGL` → new
   `CALayerTreeCoordinator` → new `CAContext` → new `ca_context_id`.
4. First compositor frame arrives → `AcceleratedWidgetCALayerParamsUpdated()`
   fires.
5. `ns_view_->SetCALayerParams()` → hidden window's `DisplayCALayerTree` creates
   a `CALayerHost` with the new ID (step A).
6. `ca_layer_params_callback_` sends the ID over XPC (step B).
7. GUI receives it and creates its own `CALayerHost` (step C).

Steps A and B happen synchronously in the same function. Step C happens
asynchronously after XPC delivery (microseconds to low milliseconds). The hidden
window's host always beats the GUI's host.

**R2: `DisableDisplay()` is never called in the Chromium Profile Server.**

Confirmed. The Chromium Profile Server is content_shell-based. `content_shell`
does not use `ui::Views` or `ui::Layer` compositing. `SetParentUiLayer()` is
never called with a non-null layer. Therefore `DisableDisplay()` never fires,
and the hidden window's `DisplayCALayerTree` is always active.

This is the root of the dual-host problem: `DisplayCALayerTree` was designed to
be disabled when the Views compositor takes over. In our pipeline, nothing takes
over, so it stays active and creates a competing `CALayerHost` for every
`ca_context_id`.

**R3: Chrome's Views UI eliminates the dual-host problem via
`DisableDisplay()`.**

In stock Chrome, `SetParentUiLayer()` is called early in the view lifecycle.
This calls `DisableDisplay()`, which:

1. Destroys the `DisplayCALayerTree` (`display_ca_layer_tree_.reset()`).
2. Sets `display_disabled_ = true`.
3. Future `SetCALayerParams()` calls return early — no `CALayerHost` created.

The Views compositor handles display through its own layer tree. There is
exactly one `CALayerHost` per `CAContext` in the Views compositor, and zero in
the NSView's `DisplayCALayerTree` (because it's been destroyed).

Our Chromium Profile Server never takes this path. The hidden window's
`DisplayCALayerTree` runs for the entire lifetime of the tab, creating a
`CALayerHost` for every `ca_context_id` it receives. Our GUI creates a second
one. Two `CALayerHost` instances compete for the same `CAContext`.

**R4: The call ordering in `AcceleratedWidgetCALayerParamsUpdated()` is
confirmed.**

```
AcceleratedWidgetCALayerParamsUpdated()
  ├── ns_view_->SetCALayerParams()       // Hidden window: DisplayCALayerTree
  │   └── GotCALayerFrame()              // Creates CALayerHost in hidden window
  └── ca_layer_params_callback_()        // Our hook: sends over XPC
      └── (async) GUI creates CALayerHost // Second host, same CAContext
```

The hidden window's `CALayerHost` is created **synchronously before** our XPC
callback fires. The GUI's `CALayerHost` is created asynchronously after XPC
delivery. Both point at the same `CAContext`.

**R5: Each navigation produces a new CAContext.**

The GPU process creates `CAContext` via `+[CAContext contextWithCGSConnection:]`
in `CALayerTreeCoordinator`'s constructor. Each output surface gets one. During
navigation, old surface → destroyed (old `CAContext` released), new surface →
created (new `CAContext`, new `ca_context_id`).

The `ca_context_id` is sent to the browser in `gfx::CALayerParams` via the swap
completion callback, then forwarded to both the hidden window's
`DisplayCALayerTree` (via `SetCALayerParams`) and our XPC callback.

#### Conclusion

The dual-`CALayerHost` problem is now fully understood. In stock Chrome,
`DisableDisplay()` destroys the NSView's `DisplayCALayerTree` when the Views
compositor takes over, ensuring exactly one `CALayerHost` per `CAContext`. In
our Chromium Profile Server (content_shell-based), `DisableDisplay()` is never
called, so the hidden window's `DisplayCALayerTree` stays active and creates a
competing `CALayerHost` for every `ca_context_id`.

The strongest hypothesis is: **the hidden window's `CALayerHost` interferes with
the GUI's `CALayerHost`** because both point at the same `CAContext`. macOS
Window Server may only composite a `CAContext` to one `CALayerHost` at a time,
or may deprioritize compositing for the GUI's host because the hidden window's
host was created first.

There are two testable hypotheses for the next experiment:

1. **Call `DisableDisplay()` on the NSView bridge** — This would destroy the
   hidden window's `DisplayCALayerTree`, eliminating the competing
   `CALayerHost`. The `SetCALayerParams()` call in
   `AcceleratedWidgetCALayerParamsUpdated()` would become a no-op, and only our
   XPC callback would run. This is the most targeted fix.

2. **Make the hidden window visible** — If the Window Server deprioritizes
   compositing for off-screen windows, making the window visible might fix the
   blank without eliminating the dual host. This would distinguish between the
   "dual host" and "hidden window" hypotheses.

#### Verification

Research is complete when all five questions are answered and we have testable
hypotheses for the next experiment.
