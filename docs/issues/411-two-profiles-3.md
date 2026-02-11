# Issue 411: Two Profiles at 60fps (No Electron Patches)

## Goal

Get the Two Profiles app rendering both panes at 60fps by fixing the compositor
initialization race condition directly, without any Electron patches.

## Background

### What Issue 410 proved

Issue 410 applied three Electron throttling patches to Chromium and ran two
experiments to fix the 2fps rendering in the Two Profiles app. Both experiments
failed. Logging in Experiment 2 revealed that the hiding/occlusion code paths
(`Hide()`, `WasOccluded()`, `WasHidden()`) are never called on either view. The
Electron patches intercept those code paths, making them irrelevant to our
problem.

### The root cause

The 2fps is caused by a visibility race condition in the compositor lifecycle:

1. `WebContents::Create()` creates the WebContents. No renderer process exists
   yet.
2. The `WebContentsViewCocoa` is added to the window (`addSubview`).
3. macOS fires `viewDidMoveToWindow`, which triggers
   `WebContentsImpl::UpdateWebContentsVisibility(VISIBLE)`.
4. `WasShown()` runs and tries to call `ShowWithVisibility(kVisible)` on the
   `RenderWidgetHostViewMac` — the method that transitions the
   `BrowserCompositorMac` from `HasNoCompositor` (dead) to `HasOwnCompositor`
   (alive).
5. But `GetRenderWidgetHostView()` returns null because the renderer doesn't
   exist yet. `ShowWithVisibility` is never called.
6. The visibility signal is consumed: `did_first_set_visible_ = true`,
   `visibility_ = VISIBLE`.
7. Later, the renderer is created. `BrowserCompositorMac` is constructed with
   `host()->IsHidden() = true` (the default), putting it in `HasNoCompositor`.
8. Nothing transitions it out. No `BeginFrame` signals are sent. The 2fps is a
   fallback timer in `DelegatedFrameHost`.

Content Shell avoids this because `Shell::CreateNewWindow` creates the renderer
and adds the view in the right order — by the time `viewDidMoveToWindow` fires,
the `RenderWidgetHostView` exists and receives `ShowWithVisibility`.

### Why Electron patches won't help

Electron's patches solve different problems (offscreen rendering, hidden
BrowserViews, background windows). Every relevant patch modifies `WasOccluded()`
or other hiding code paths that are never triggered in our case. The additional
patches identified in Issue 410 research (`disable_compositor_recycling`,
`revert_macwebcontentsocclusion`) also target `WasOccluded()`. More patches
would add maintenance burden on every Chromium upgrade without fixing the actual
problem.

## Starting point

Reset the Chromium fork to the base `146.0.7650.0` tag with only the Two
Profiles app code — no Electron patches. The experiment will modify only the Two
Profiles app code (and potentially minimal Chromium changes if needed) to fix
the race condition.

### Files

- `content/two_profiles/BUILD.gn` — build target
- `content/two_profiles/two_profiles_main_parts.h` — header
- `content/two_profiles/two_profiles_main_parts.mm` — implementation
- `BUILD.gn` (root) — `gn_all` group entry

## Approach

The compositor never starts because the visibility signal fires before the
renderer exists. There are several ways to fix this, ordered from simplest to
most invasive:

### Option A: Defer adding the view to the window

Don't call `[container addSubview:view_b]` until after the renderer is created.
Use a `WebContentsObserver::RenderFrameCreated` callback to add the view to the
window at the right time. This way, `viewDidMoveToWindow` fires when the
`RenderWidgetHostView` exists and can receive `ShowWithVisibility`.

This is the most natural fix — it aligns our code with the assumption Content
Shell's lifecycle already relies on.

### Option B: Force a visibility re-notification

After the renderer is created (detected via `RenderFrameCreated`), force the
`WebContentsImpl` to re-run the visibility chain by toggling visibility:

```cpp
web_contents->UpdateWebContentsVisibility(Visibility::HIDDEN);
web_contents->UpdateWebContentsVisibility(Visibility::VISIBLE);
```

This resets `did_first_set_visible_` and `visibility_`, then re-runs
`WasShown()` → `ShowWithVisibility()` with the `RenderWidgetHostView` present.

### Option C: Call SetRenderWidgetHostIsHidden(false) directly

After the renderer is created, reach into the `BrowserCompositorMac` and
explicitly transition it to `HasOwnCompositor`:

```cpp
auto* rwhv_mac = static_cast<RenderWidgetHostViewMac*>(rwhv);
rwhv_mac->browser_compositor_->SetRenderWidgetHostIsHidden(false);
```

This is the most direct fix but requires accessing private members and couples
our code to Chromium internals.

### Option D: Use Shell::CreateNewWindow for both profiles

Create two `Shell` windows (each with its own `WebContents` and correct
lifecycle), then reparent one Shell's web view into the other's window. This
uses the proven Content Shell lifecycle for both WebContents, avoiding the race
entirely.

## Experiment 1

### Branch setup

Create a new branch `146.0.7650.0-issue-411` in the `termsurf-chromium`
submodule, starting from the vanilla Chromium `146.0.7650.0` tag. No Electron
patches are applied.

Steps:

1. `cd ts4/termsurf-chromium/src`
2. `git checkout -b 146.0.7650.0-issue-411 146.0.7650.0`
3. Cherry-pick the Two Profiles app commit from `146.0.7650.0-termsurf`. This
   brings in `content/two_profiles/` (BUILD.gn, header, implementation) and the
   root `BUILD.gn` change that adds `//content/two_profiles` to `gn_all`. The
   commit includes the `ThrottleBypassObserver`, which we will simplify.
4. Apply the race condition fix (the experiment itself).
5. Build with `autoninja -C out/Default two_profiles`.

The `146.0.7650.0-termsurf` branch (with Electron patches) remains untouched.
If this experiment succeeds, the fix can be brought back to the main termsurf
branch. If it fails, we still have the patched branch to fall back on.

After the experiment, update the main repo's submodule pointer to the new branch
and commit.

### Hypothesis

The 2fps is caused by the `WebContentsViewCocoa` being added to the window
before the renderer exists, consuming the visibility signal. Deferring the view
attachment until after `RenderFrameCreated` fires (Option A) will allow the
visibility chain to reach the `RenderWidgetHostViewMac` and transition the
`BrowserCompositorMac` to `HasOwnCompositor`, producing frames at 60fps.

### Design

Modify the Two Profiles app to defer adding WebContents B's view to the window.
Instead of adding it in `InitializeMessageLoopContext()`, add it in a
`RenderFrameCreated` observer callback.

For Shell A (profile A), no change is needed — `Shell::CreateNewWindow` handles
the lifecycle correctly and Content Shell already runs at 60fps.

For WebContents B (profile B):

1. Create the WebContents and start navigation as before.
2. Do NOT add `view_b` to the container yet.
3. In a `RenderFrameCreated` observer callback (for profile B only), add
   `view_b` to the container and set its frame.
4. `viewDidMoveToWindow` fires. The renderer now exists.
   `ShowWithVisibility(kVisible)` reaches the `RenderWidgetHostViewMac`. The
   `BrowserCompositorMac` transitions to `HasOwnCompositor`. Frames flow.

Strip the `ThrottleBypassObserver` down to just the view-attachment logic.
Remove the bypass calls (`disable_hidden_`, `SetSchedulerThrottling`,
`WasShown`, `ShowWithVisibility`) since those target code paths that are never
triggered. The observer becomes a deferred-attachment hook, not a throttling
bypass.

### What this tests

- Whether deferring view attachment fixes the visibility race condition
- Whether the `BrowserCompositorMac` properly transitions to `HasOwnCompositor`
  when `viewDidMoveToWindow` fires with a live renderer
- Whether both panes render at 60fps without any Electron patches

### Expected result

Both panes at 60fps. Profile isolation still works (different localStorage
strings). No Electron patches needed.
