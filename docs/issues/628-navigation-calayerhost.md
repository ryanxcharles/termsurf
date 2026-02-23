# Issue 628: CALayerHost Navigation

## Goal

Fix browser navigation so that clicking a link does not cause the web overlay to
vanish. The overlay should persist across same-site and cross-site navigations,
updating seamlessly as Chromium produces new compositor surfaces.

## Background

### How we got here

[Issue 625](625-calayerhost.md) replaced the `FrameSinkVideoCapturer` pipeline
with `CALayerHost`. Instead of capturing IOSurface frames at 120fps and
transferring Mach ports over XPC every frame, Chromium now sends a
`ca_context_id` (uint32) once per tab. The GUI creates a `CALayerHost` sublayer,
and Window Server composites the remote content directly from GPU VRAM. Zero
per-frame IPC, zero pixel copies.

[Issue 626](626-x-y-calayerhost.md) fixed X/Y positioning of the CALayerHost
overlay. Six experiments revealed two coordinate system bugs: a missing
intermediate flipped layer and a bottom-origin Y coordinate on the
IOSurfaceLayer. The overlay is now pixel-perfect.

[Issue 627](627-resize-calayerhost.md) restored resize behavior. The Issue 625
commit had accidentally removed `sendResize()` and Chromium's `"resize"`
handler. A second experiment introduced a 3-layer architecture
(`flipped_layer → positioning_layer → CALayerHost`) to anchor the overlay to the
top edge during resize instead of the bottom.

Before the CALayerHost migration, navigation worked flawlessly. The old
IOSurface pipeline received a new Mach port on every frame — including after
navigation. The Metal shader re-read the IOSurface texture every frame in
`drawFrame()`, so navigation was invisible to the overlay pipeline. The new
surface just showed up on the next frame.

### What broke

With CALayerHost, there is no per-frame surface update. The `ca_context_id` is
set once on the `CALayerHost` layer, and Window Server composites from that
context. When the user clicks a link, Chromium navigates to a new page. This may
trigger a renderer process swap (site isolation) or a new compositor surface,
either of which produces a new `CAContext` with a new `ca_context_id`. The old
context becomes invalid, and the overlay vanishes.

### The ca_context_id lifecycle

The `ca_context_id` is produced by Chromium's GPU process and delivered to our
code via the `CALayerParams` callback chain:

```
GPU Process creates CAContext
    ↓
RenderWidgetHostViewMac receives CALayerParams
    ↓
TermSurf callback (registered via SetCALayerParamsCallbackOnView)
    ↓
XPC message: action="ca_context", ca_context_id=N, pane_id=...
    ↓
GUI xpc.zig handleCAContext() → Surface.setCAContextId()
    ↓
Metal.setCALayerHostContextId() — creates or updates CALayerHost
```

The callback is a `BindRepeating` with deduplication — it only sends when
`ca_context_id != 0` and `ca_context_id != last_id`:

```cpp
if (params.ca_context_id == 0 || params.ca_context_id == *last_id)
    return;
*last_id = params.ca_context_id;
// send XPC message...
```

### Two potential failure points

**1. The callback may not survive navigation.**

The callback is registered on a specific `RenderWidgetHostView`. On cross-site
navigation, Chromium may destroy the old `RenderWidgetHostView` and create a new
one (renderer process swap). The callback would be lost with the old view. The
new view would never send `ca_context_id` updates because no callback was
registered on it.

**2. Updating `contextId` on an existing `CALayerHost` may not work.**

Even if the callback survives and the GUI receives the new `ca_context_id`, our
code updates the property on the existing `CALayerHost`:

```zig
if (ca_layer_host_ptr.*) |existing| {
    const host = objc.Object.fromId(existing);
    host.setProperty("contextId", @as(u32, context_id));
}
```

But Chromium's own `DisplayCALayerTree::GotCALayerFrame()` does NOT do this. It
creates a **brand new** `CALayerHost` every time the `ca_context_id` changes:

```cpp
void DisplayCALayerTree::GotCALayerFrame(uint32_t ca_context_id) {
    if (remote_layer_.contextId == ca_context_id)
        return;

    CALayerHost* new_remote_layer = [[CALayerHost alloc] init];
    new_remote_layer.anchorPoint = CGPointZero;
    new_remote_layer.contextId = ca_context_id;
    new_remote_layer.autoresizingMask = kCALayerMaxXMargin | kCALayerMaxYMargin;

    [maybe_flipped_layer_ addSublayer:new_remote_layer];
    [remote_layer_ removeFromSuperlayer];
    remote_layer_ = new_remote_layer;
}
```

This suggests that Window Server may not re-bind to a new remote context when
`contextId` is changed on an existing `CALayerHost`. The safe approach is to
match Chromium's pattern: destroy the old `CALayerHost` and create a new one.

### Current layer tree (from Issue 627)

```
IOSurfaceLayer (Y=0 at bottom)
└─ flipped_layer (geometryFlipped=YES, auto-fills parent)
   └─ positioning_layer (explicit frame at overlay rect, top-origin Y)
      └─ CALayerHost (at origin, contextId set once)
```

The `flipped_layer` and `positioning_layer` should survive navigation — they are
independent of the `ca_context_id`. Only the `CALayerHost` needs to be replaced.

### Chromium branch

`146.0.7650.0-issue-628`, branched from `146.0.7650.0-issue-627`.

## Experiments

### Experiment 1: Re-register callback on view swap, replace CALayerHost on context change

Fix both failure points identified in the analysis.

#### Chromium side: re-register callback after navigation

The `SetCALayerParamsCallbackOnView` callback is registered once in
`CreateTab()` on the initial `RenderWidgetHostView`. If navigation causes a
renderer process swap, the old view is destroyed and the callback is lost.

`ShellTabObserver` already extends `WebContentsObserver` and has
`xpc_connection_` and `pane_id_`. `WebContentsObserver` provides
`RenderViewHostChanged(old_host,
new_host)`, which fires when the `WebContents`
swaps its `RenderViewHost` — exactly when the `RenderWidgetHostView` changes.

Move the CALayerParams callback registration into `ShellTabObserver` so it can
re-register after a view swap:

1. Add a `RegisterCALayerParamsCallback()` method to `ShellTabObserver` that
   gets the current `RenderWidgetHostView` from `web_contents()`, and calls
   `SetCALayerParamsCallbackOnView` with the same callback logic currently in
   `CreateTab()`. Track the `last_id` as a member variable (not `base::Owned`)
   so it persists across re-registrations.
2. Override `RenderViewHostChanged()` in `ShellTabObserver`. In the override,
   call `RegisterCALayerParamsCallback()` to re-register on the new view.
3. In `CreateTab()`, replace the inline callback registration with a call to
   `tab_observer->RegisterCALayerParamsCallback()`.

#### GUI side: replace CALayerHost instead of updating contextId

Match Chromium's `DisplayCALayerTree::GotCALayerFrame()` pattern. When the
`ca_context_id` changes on an existing overlay, destroy the old `CALayerHost`
and create a new one inside the existing `positioning_layer`.

In `Metal.setCALayerHostContextId`, change the existing-host branch: instead of
`host.setProperty("contextId", context_id)`, remove the old `CALayerHost` from
its superlayer and release it, then create a new `CALayerHost` with the new
`contextId`, set `anchorPoint = zero` and
`autoresizingMask = kCALayerMaxXMargin | kCALayerMaxYMargin`, add it as a
sublayer of the `positioning_layer`, and update `ca_layer_host_ptr`.

The `positioning_layer` pointer is needed in `setCALayerHostContextId` for this.
Pass it as a parameter (it's already stored in `generic.zig`).

#### Changes

**`chromium/.../shell_tab_observer.h`:**

- Add `void RegisterCALayerParamsCallback()` declaration.
- Add `void RenderViewHostChanged(RenderViewHost*, RenderViewHost*)` override.
- Add `uint32_t last_ca_context_id_ = 0` member.

**`chromium/.../shell_tab_observer.cc`:**

- Implement `RegisterCALayerParamsCallback()`: get
  `web_contents()->GetRenderWidgetHostView()`, guard on null, call
  `SetCALayerParamsCallbackOnView` with a lambda that sends the `"ca_context"`
  XPC message. Use `&last_ca_context_id_` for deduplication (member, not owned).
- Implement `RenderViewHostChanged()`: log the swap, call
  `RegisterCALayerParamsCallback()`.
- Add `#include "shell_ca_layer_bridge_mac.h"` for the bridge function.

**`chromium/.../shell_browser_main_parts.cc`:**

- In `CreateTab()`, remove the inline `SetCALayerParamsCallbackOnView` block.
  Replace with `tab_observer->RegisterCALayerParamsCallback()`.

**`gui/src/renderer/Metal.zig`:**

- Change `setCALayerHostContextId` to accept the `positioning_layer` pointer.
- In the existing-host branch: remove old CALayerHost from superlayer, release
  it, create a new one, add to `positioning_layer`, update `ca_layer_host_ptr`.

**`gui/src/renderer/generic.zig`:**

- Pass `self.ca_layer_positioning` to `setCALayerHostContextId`.

#### Verification

Run the app, open a browser overlay at `google.com`, search for something. The
search results page should render — the overlay should not vanish. Test clicking
links on the results page. Test navigating back with Cmd+[. Test cross-site
navigation (e.g., clicking a link from Google to Wikipedia).

#### Results

**Partial.** Navigation works — clicking a link no longer permanently kills the
overlay. The new page renders after navigation. But two issues remain:

1. **New page renders at stale size.** If the pane was resized before clicking a
   link, the new page renders at the original size (from tab creation), not the
   current size. Resizing the window again fixes it. The likely cause: when
   `RenderViewHostChanged` fires and the callback re-registers on the new view,
   the new `RenderWidgetHostView` has its default size, not the size that was
   set on the old view. The resize needs to be re-applied to the new view.

2. **Blank gap between pages.** When navigating, the old page vanishes
   completely (the old CALayerHost is destroyed when the new `ca_context_id`
   arrives), then there is a visible blank period before the new page loads. A
   normal browser keeps the old page visible until the new one is ready. The old
   `CALayerHost` should be kept alive until the new `ca_context_id` arrives,
   rather than being destroyed as soon as the `RenderViewHost` swaps.

#### Conclusion

The core fix works: `RenderViewHostChanged` re-registers the CALayerParams
callback on the new view, and the GUI replaces the `CALayerHost` when the new
`ca_context_id` arrives. Navigation no longer kills the overlay permanently.

The two remaining issues need separate fixes:

- **Stale size:** Re-apply the current pixel dimensions to the new
  `RenderWidgetHostView` in `RenderViewHostChanged`.
- **Blank gap:** Defer destruction of the old `CALayerHost` until the
  replacement `ca_context_id` arrives, rather than destroying it eagerly.

### Experiment 2: Re-apply size to new RenderWidgetHostView after navigation

#### Problem

After navigation triggers a `RenderViewHostChanged`, the new
`RenderWidgetHostView` has its default size, not the size that was applied via
`ResizeTab()`. If the pane was resized between tab creation and navigation, the
new page renders at the original creation size.

`ResizeTab()` calls `view->SetSize(logical)` on the current RWHV, but that view
is destroyed on cross-site navigation. The new RWHV knows nothing about the
previous resize.

#### Solution

Store the last pixel dimensions in `ShellTabObserver`. When `ResizeTab()` is
called, update the stored dimensions on the observer (via a new setter). When
`RenderViewHostChanged()` fires, re-apply the stored dimensions to the new view
— the same `view->SetSize(logical)` call that `ResizeTab()` uses.

This is a Chromium-only change. No GUI changes needed.

#### Changes

**`chromium/.../shell_tab_observer.h`:**

- Add `void SetLastPixelSize(int width, int height)`.
- Add `int last_pixel_width_ = 0` and `int last_pixel_height_ = 0` members.

**`chromium/.../shell_tab_observer.cc`:**

- Implement `SetLastPixelSize`: store width and height.
- In `RenderViewHostChanged`: after `RegisterCALayerParamsCallback()`, get the
  new `RenderWidgetHostView`, compute logical size from stored pixel dimensions
  and `view->GetDeviceScaleFactor()`, call `view->SetSize(logical)`.

**`chromium/.../shell_browser_main_parts.cc`:**

- In `CreateTab()`: after the initial `view->SetSize()`, call
  `tab_observer->SetLastPixelSize(pixel_width, pixel_height)`.
- In `ResizeTab()`: after `view->SetSize()`, call
  `tab->tab_observer->SetLastPixelSize(pixel_width, pixel_height)`.

#### Verification

Run the app, open a browser overlay, resize the window, then click a link. The
new page should render at the current pane size, not the original creation size.

#### Results

**Fail.** The new page still renders at the original creation size, not the
resized size. The fix had no effect.

#### Conclusion

`RenderViewHostChanged` does not fire for the navigations being tested.
Same-site navigations (e.g., clicking a link on Google) don't swap the
`RenderViewHost` — the same view persists. Cross-site navigations may also not
trigger it if site isolation isn't fully enabled in the Profile Server
configuration. The re-apply code never runs because the hook never fires.

The fix needs a different hook point — one that fires on every navigation, not
just view swaps. The CALayerParams callback itself is a candidate: when a new
`ca_context_id` arrives (meaning a new compositor surface was created), re-apply
the stored size to the current view.

### Experiment 3: Research how Electron handles resize across navigation

#### Problem

After navigation, the new page renders at the original creation size instead of
the current resized size. Experiment 2 tried re-applying the size in
`RenderViewHostChanged`, but that hook doesn't fire for same-site navigations.
We need a different approach.

Electron's `BrowserWindow` handles this correctly — you can resize a window and
navigate, and the new page renders at the current size. Electron uses the same
Chromium Content API that we do. Understanding how Electron maintains size
across navigations will reveal the correct approach.

#### Research questions

**R1: Does Electron call `view->SetSize()` at all?**

Electron's `BrowserWindow` has a fixed window frame. Does it ever explicitly
call `SetSize()` on the `RenderWidgetHostView`, or does the view automatically
inherit the window/NSView size? If the view auto-sizes to its parent NSView, the
resize problem doesn't exist for Electron — the view always matches the window.

Look in `vendor/electron/shell/browser/` for calls to `SetSize`, `Resize`, or
`SetBounds` on `RenderWidgetHostView`.

**R2: How does Chromium's normal display path handle view sizing?**

In stock Chromium (not content_shell), how does the `RenderWidgetHostView` get
its size? Is it set explicitly, or does it follow the NSView frame? When the
window resizes, does something call `SetSize()`, or does the view observe its
parent's bounds change?

Look in
`chromium/src/content/browser/renderer_host/render_widget_host_view_mac.mm` for
sizing logic — `setFrameSize`, `viewDidChangeBackingProperties`,
`boundsDidChange`, or similar NSView layout methods.

**R3: Why does content_shell need explicit `SetSize()` calls?**

Our Profile Server is based on content_shell. Content_shell creates a hidden
NSWindow with a `WebContentsViewCocoa`. The `RenderWidgetHostView` lives inside
that view hierarchy. When we call `view->SetSize()`, we're setting the view's
frame explicitly because the hidden window doesn't have normal window
management.

Is there an alternative? Could we resize the hidden NSWindow instead, letting
the normal NSView layout propagate the size to the `RenderWidgetHostView`? Would
that survive navigation automatically?

Look in
`chromium/src/content/chromium_profile_server/browser/shell_platform_delegate_mac.mm`
and `chromium/src/content/shell/browser/shell_platform_delegate_mac.mm` for how
the window and web contents view are set up.

**R4: Does the `RenderWidgetHostView` survive same-site navigation?**

Confirm or refute the Experiment 2 hypothesis. After a same-site navigation
(e.g., clicking a link on google.com that stays on google.com), is the
`RenderWidgetHostView` the same object? If so, the `SetSize()` from
`ResizeTab()` should still be in effect, and the problem is elsewhere — perhaps
the compositor creates a new surface at a default size before the view's size
takes effect.

Look at `RenderWidgetHostViewMac` lifecycle during navigation.

#### Verification

Research is complete when we can answer all four questions and propose a
concrete fix based on how Electron/Chromium maintains view size across
navigation.

#### Results

**Pass.** All four questions answered.

**R1: Does Electron call `view->SetSize()` at all?**

Yes. Electron explicitly calls `SetSize()` on its off-screen
`RenderWidgetHostView` whenever the window resizes. It does NOT rely on NSView
auto-layout. The call chain: `NativeWindow::NotifyWindowResize()` →
`OffScreenWebContentsView::OnWindowResize()` → `view->SetSize(GetSize())`. The
macOS NSView is a dummy placeholder — all sizing is explicit and programmatic.

Key files:

- `vendor/electron/shell/browser/osr/osr_web_contents_view.cc:62-66`
- `vendor/electron/shell/browser/osr/osr_render_widget_host_view.cc:289-296`

**R2: How does Chromium's normal display path handle view sizing?**

The RWHV gets its initial size from `SetParentWebContentsNSView()`, which copies
the parent's bounds and sets
`autoresizingMask = NSViewWidthSizable |
NSViewHeightSizable` on the Cocoa view.
After that, autoresizing keeps the view sized to its parent automatically. When
the NSView frame changes (from autoresizing or explicit `SetBounds`),
`setFrameSize:` fires → `sendViewBoundsInWindowToHost` →
`OnBoundsInWindowChanged()` → `UpdateScreenInfo()` →
`UpdateSurfaceFromNSView()`. This updates `dfh_size_dip_` in
`BrowserCompositorMac`, which is the size used when creating new compositor
surfaces.

Key files:

- `chromium/src/content/app_shim_remote_cocoa/render_widget_host_ns_view_bridge.mm:94-108`
- `chromium/src/content/app_shim_remote_cocoa/render_widget_host_view_cocoa.mm:1763-1766`
- `chromium/src/content/browser/renderer_host/render_widget_host_view_mac.mm:894-942`

**R3: Why does content_shell need explicit `SetSize()` calls?**

Our `SetContents` in `shell_platform_delegate_mac.mm` adds the web view to the
hidden NSWindow's contentView with
`autoresizingMask = NSViewWidthSizable |
NSViewHeightSizable`. But `ResizeTab()`
only calls `view->SetSize()` — it never resizes the hidden NSWindow itself. This
creates a mismatch: the NSView frame is set to the new size via `SetBounds()`,
but the NSWindow remains at its creation size. The autoresizing mask ties the
web view to the window's contentView bounds, so the NSView frame and the window
are inconsistent.

Resizing the NSWindow instead of (or in addition to) calling `view->SetSize()`
would let the normal autoresizing chain propagate size correctly: NSWindow
resize → contentView resize → web_view autoresize → `setFrameSize:` →
`sendViewBoundsInWindowToHost` → `OnBoundsInWindowChanged` → `UpdateScreenInfo`
→ `UpdateSurfaceFromNSView`.

Key files:

- `chromium/src/content/chromium_profile_server/browser/shell_platform_delegate_mac.mm:229-245`
- `chromium/src/content/chromium_profile_server/browser/shell_browser_main_parts.cc:417-452`

**R4: Does the `RenderWidgetHostView` survive same-site navigation?**

Yes. Same-site navigations reuse the current `RenderFrameHost` and its
`RenderWidgetHostView`. The view is the same object — it is not destroyed and
recreated. The `ca_context_id` changes because
`BrowserCompositorMac::DidNavigate()` generates a new `LocalSurfaceId`, which
creates a new compositor surface. The new surface is created with
`dfh_size_dip_` — whatever size `BrowserCompositorMac` thinks the view is.

This is the root cause: `DidNavigate()` calls `EmbedSurface()` with
`dfh_size_dip_`. If `dfh_size_dip_` reflects the NSWindow's contentView bounds
(from autoresizing) rather than the explicitly-set view size, the new surface
will be created at the wrong size.

Key files:

- `chromium/src/content/browser/renderer_host/browser_compositor_view_mac.mm:341-361`
- `chromium/src/content/browser/renderer_host/render_frame_host_manager.cc:1865-2021`

#### Conclusion

The stale-size-after-navigation bug has a clear root cause.

`ResizeTab()` calls `view->SetSize()` which sets the NSView frame and updates
`dfh_size_dip_` via the bounds-change callback chain. This works initially — the
page renders at the correct size. But the hidden NSWindow remains at its
creation size. The web view has `autoresizingMask` tying it to the window's
contentView, creating tension between the explicitly-set frame and the
autoresizing constraint.

During navigation, `BrowserCompositorMac::DidNavigate()` creates a new
compositor surface using `dfh_size_dip_`. If the autoresizing constraint has
pulled the NSView back to the window size, or if `dfh_size_dip_` was reset
during the navigation lifecycle, the new surface renders at the wrong (original)
size.

The fix is to resize the hidden NSWindow in `ResizeTab()` instead of calling
`view->SetSize()` directly. When the NSWindow resizes, the autoresizing chain
propagates naturally: contentView → web_view → RWHV Cocoa view → `setFrameSize:`
→ `sendViewBoundsInWindowToHost` → `OnBoundsInWindowChanged` →
`UpdateScreenInfo` → `UpdateSurfaceFromNSView`. This updates `dfh_size_dip_`
through the standard Chromium path, and the size survives navigation because the
NSWindow stays at the correct size.

### Experiment 4: Resize the NSWindow instead of calling view->SetSize()

#### Problem

`ResizeTab()` calls `view->SetSize(logical)` to resize the
`RenderWidgetHostView` directly. This sets the NSView frame but never resizes
the hidden NSWindow. The web view has
`autoresizingMask = NSViewWidthSizable |
NSViewHeightSizable`, tying it to the
window's contentView. When navigation triggers
`BrowserCompositorMac::DidNavigate()`, the new compositor surface is created
with `dfh_size_dip_`, which may reflect the window's original size rather than
the explicitly-set view size.

#### Solution

Resize the NSWindow instead of calling `view->SetSize()`. The
`Shell::ResizeWebContentForTests()` method already calls
`ShellPlatformDelegate::ResizeWebContent()`, which has access to the NSWindow in
the `.mm` file. Fix `ResizeWebContent` to set the window's content size (not
just `contentView.frame`), then use `ResizeWebContentForTests` from both
`CreateTab()` and `ResizeTab()`.

The autoresizing chain handles the rest: window resize → contentView resize →
web_view autoresize → RWHV Cocoa view `setFrameSize:` →
`sendViewBoundsInWindowToHost` → `OnBoundsInWindowChanged` → `UpdateScreenInfo`
→ `UpdateSurfaceFromNSView` → `dfh_size_dip_` updated.

#### Changes

**`chromium/.../shell_platform_delegate_mac.mm`:**

- In `ResizeWebContent()`, replace
  `shell_data.delegate.window.contentView.frame = frame` with
  `[shell_data.delegate.window setContentSize:NSMakeSize(width, height)]`. This
  resizes the window itself, letting autoresizing propagate to all subviews.

**`chromium/.../shell_browser_main_parts.cc`:**

- In `CreateTab()`, replace `view->SetSize(logical)` with
  `shell->ResizeWebContentForTests(logical)`.
- In `ResizeTab()`, replace `view->SetSize(logical)` with
  `tab->shell->ResizeWebContentForTests(logical)`.

#### Verification

Run the app, open a browser overlay, resize the window, then click a link. The
new page should render at the current pane size, not the original creation size.
Test same-site navigation (link within google.com) and cross-site navigation
(link from Google to Wikipedia). Both should preserve the resized dimensions.

#### Results

**Pass.** Navigation preserves the resized dimensions. The new page renders at
the current pane size after both same-site and cross-site navigation.

#### Conclusion

The root cause was confirmed: `view->SetSize()` set the RWHV's NSView frame but
left the hidden NSWindow at its creation size. The autoresizing mask created
tension between the two, and `BrowserCompositorMac::DidNavigate()` used the
wrong size when creating the new compositor surface.

Resizing the NSWindow via `[window setContentSize:]` lets the standard
autoresizing chain propagate the size through the entire view hierarchy. The
`dfh_size_dip_` in `BrowserCompositorMac` stays correct across navigations
because it derives from the actual NSView bounds, which follow the window.
