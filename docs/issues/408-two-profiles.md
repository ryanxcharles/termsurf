# Issue 408: Two Profiles at 60fps

## Goal

Render two Chromium `BrowserContext` instances side by side in a single macOS
window at 60fps or higher. Issue 407 proved that multiple profiles coexist in
one process and that content_shell renders a single WebContents at 60fps. But
placing two WebContents in one window dropped both to 2-3fps because manual
NSView manipulation broke Chromium's internal visibility tracking. This issue
solves the framerate problem.

## Background

Issue 407 established:

- **Multi-profile works.** Two `ShellBrowserContext` instances with different
  storage paths run in the same process with full isolation (separate cookies,
  localStorage, cache).
- **Single WebContents renders at 60fps.** content_shell's windowed rendering
  path has no framerate ceiling.
- **Two WebContents in one window renders at 2-3fps.** The
  `RenderWidgetHostViewCocoa` has NSView-level visibility tracking that
  overrides explicit `WasShown()` calls. Manually reparenting and resizing views
  causes it to misreport visibility, triggering Chromium's background tab
  throttle.

The throttling chain:

```
RenderWidgetHostViewCocoa (NSView visibility)
  -> RenderWidgetHostImpl::WasShown() / WasHidden()
  -> Blink PageSchedulerImpl::SetPageVisible()
  -> CC SchedulerStateMachine::visible_
  -> ShouldSubscribeToBeginFrames()
  -> vsync subscription ON (60fps) or OFF (~1fps)
```

## Success Criteria

- Two panes in one window, each showing a different localStorage string.
- Both panes render the spinning blue square at 60fps or higher.
- Strings persist across app restarts.
- No custom IPC protocol between the embedder and Chromium.

## Approaches to Investigate

### Approach 1: Chromium `views` framework

content_shell uses raw NSWindows, bypassing Chromium's `views` layer. Chrome
itself uses `views::WebView` to embed WebContents into `views::Widget` windows.
The `views` framework handles visibility, layout, occlusion, and resize
notifications through proper Chromium channels.

**Idea:** Create a `views::Widget` with two `views::WebView` children, each
backed by a different `BrowserContext`. The `views` framework manages visibility
through `NativeWidgetMacNSWindowHost` and `windowDidChangeOcclusionState:`,
which should keep both views at full framerate.

**Risk:** The `views` framework is large and tightly coupled to Chrome's UI.
Using it from a minimal embedder may pull in unwanted dependencies. It may also
assume Chrome-specific infrastructure (like `BrowserView`, `TabStripModel`) that
doesn't exist in our app.

### Approach 2: Off-screen compositing via CopyFromSurface

Use `RenderWidgetHostView::CopyFromSurface()` to capture each WebContents'
rendered output as a bitmap or GPU texture, then composite both into a single
Metal render pass in the host window.

**Idea:** Each WebContents renders into its own off-screen surface. The host app
reads these surfaces on a display-link timer and composites them into the
window. This is conceptually similar to CEF's off-screen rendering but uses
Chromium's Content API directly.

**Risk:** `CopyFromSurface()` may involve GPU readback (GPU -> CPU -> GPU),
which is slow. Need to verify whether there's a zero-copy path that yields an
IOSurface or Metal texture directly. If this is just a glorified screenshot API,
it will have the same throughput ceiling as CEF's OSR.

### Approach 3: Patch RenderWidgetHostViewCocoa visibility

Override or patch `RenderWidgetHostViewCocoa`'s NSView-level visibility
detection so that explicit `WasShown()` calls are respected regardless of the
view's position in the NSView hierarchy.

**Idea:** Find the specific code in `RenderWidgetHostViewCocoa` (or its backing
`RenderWidgetHostViewMac`) that checks NSView/NSWindow visibility and either
disable it or make it configurable. This is the most surgical fix — if the only
problem is the visibility misdetection, patching it should restore 60fps.

**Risk:** The visibility detection may exist for good reason (power savings,
correctness). Disabling it could cause subtle rendering bugs, excessive GPU
usage, or break other Chromium features that depend on accurate visibility
state.

### Approach 4: Two Shell windows, shared parent

Create two full `Shell` instances through the proper `Shell::CreateNewWindow()`
pipeline (each with its own NSWindow, proper platform delegate setup, correct
visibility tracking). Then reparent their content views into a shared parent
NSWindow.

**Idea:** Each Shell gets its own invisible NSWindow for Chromium's internal
bookkeeping, but the actual `RenderWidgetHostViewCocoa` NSViews are moved to a
visible parent window for display. Chromium thinks each view is in its own
window (preserving the one-WebContents-per-window assumption), while the user
sees a single window.

**Risk:** Reparenting NSViews between windows may trigger the same visibility
issues as Issue 407. NSViews removed from a key window may be treated as hidden.
Need to verify whether Chromium tracks the NSView's window or the Shell's
original window for visibility purposes.

### Approach 5: DelegatedFrameHost / IOSurface extraction

Intercept the compositor output at the `DelegatedFrameHost` or `ui::Compositor`
level to get each WebContents' rendered frames as IOSurfaces before they reach
the NSView, then composite both IOSurfaces into a single Metal render pass.

**Idea:** This is the "proper" version of off-screen compositing — instead of
screenshotting via `CopyFromSurface()`, tap into the compositor's frame
submission pipeline to get zero-copy access to the rendered surfaces. This is
how Chrome's tab preview thumbnails and picture-in-picture work internally.

**Risk:** The compositor internals (`viz`, `DelegatedFrameHost`,
`FrameSinkManager`) are complex and poorly documented for external use. This
approach requires deep understanding of Chromium's GPU compositing pipeline.

## Approach Selection

Start with **Approach 3** (patch visibility detection). It is the most surgical
fix with the smallest blast radius. If the only reason for the 2-3fps throttle
is that `RenderWidgetHostViewCocoa` misreports visibility, then fixing that one
check should restore 60fps without restructuring the app.

If Approach 3 fails (the problem is deeper than visibility), fall back to
**Approach 1** (views framework) or **Approach 5** (IOSurface extraction)
depending on which proves more tractable.

## Relationship to Other Issues

| Issue   | Relationship                                                                      |
| ------- | --------------------------------------------------------------------------------- |
| 325-350 | Proved CEF's off-screen rendering caps at ~31fps on macOS                         |
| 403     | Proved IOSurface compositing at 60fps with colored rectangles                     |
| 406     | Proved multiple profiles work in one Chromium process                             |
| 407     | Proved multi-profile in practice; identified visibility throttling as the blocker |
| 408     | This issue -- solves the framerate problem for multi-profile rendering            |
