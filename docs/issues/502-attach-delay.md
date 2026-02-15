# Issue 502: Eliminate Hardcoded 2-Second Capturer Attach Delay

## Problem

The Chromium Profile Server uses a hardcoded 2-second `PostDelayedTask` to
attach the `FrameSinkVideoCapturer` to the `WebContents`:

```cpp
// shell_browser_main_parts.cc:152-181
void ShellBrowserMainParts::InitializeMessageLoopContext() {
  Shell* shell = Shell::CreateNewWindow(browser_context_.get(), GetStartupURL(),
                                        nullptr, gfx::Size());

  video_consumer_ = std::make_unique<ShellVideoConsumer>();
  // ... XPC setup ...

  base::SingleThreadTaskRunner::GetCurrentDefault()->PostDelayedTask(
      FROM_HERE,
      base::BindOnce(
          [](ShellVideoConsumer* consumer, WebContents* web_contents) {
            consumer->Attach(web_contents);
          },
          video_consumer_.get(), shell->web_contents()),
      base::Seconds(2));
}
```

The delay exists because `Shell::CreateNewWindow()` starts a navigation, but the
`RenderWidgetHostView` (RWHV) — which owns the compositor surface — doesn't
exist yet. The capturer needs the RWHV's `FrameSinkId` to know which surface to
capture. The 2-second delay is a crude workaround: just wait long enough that
the RWHV is guaranteed to exist.

### Why this is bad

1. **Wastes ~2 seconds of frames.** The page is already rendering during those 2
   seconds, but nothing is captured or sent via XPC. The receiver sees nothing
   for 2 seconds after launch.
2. **Fragile.** On a slow machine or under heavy load, 2 seconds might not be
   enough. On a fast machine, it wastes 1.99 seconds.
3. **Unnecessarily slow.** The RWHV is typically ready within milliseconds, not
   seconds.

## Electron's Approach

Electron solves this problem in `vendor/electron/shell/browser/osr/`. It
provides a custom `RenderWidgetHostView` subclass
(`OffScreenRenderWidgetHostView`) and a custom `WebContentsView` subclass
(`OffScreenWebContentsView`). This gives Electron total control over the RWHV
lifecycle:

1. **`OffScreenWebContentsView::CreateViewForWidget()`** — When Chromium asks
   for a view, Electron returns its own `OffScreenRenderWidgetHostView`. This is
   the factory hook.

2. **RWHV constructor creates the capturer immediately**
   (`osr_render_widget_host_view.cc:225-231`):
   ```cpp
   video_consumer_ = std::make_unique<OffScreenVideoConsumer>(
       this, base::BindRepeating(&OffScreenRenderWidgetHostView::OnPaint,
                                 weak_ptr_factory_.GetWeakPtr()));
   video_consumer_->SetActive(is_painting());
   ```

3. **`OffScreenVideoConsumer` calls `view_->CreateVideoCapturer()` in its
   constructor** (`osr_video_consumer.cc:42`), which sets up the capture
   pipeline via `HostFrameSinkManager`. This is safe because it just establishes
   plumbing — no frames flow yet.

4. **`ShowWithVisibility()` is the readiness signal**
   (`osr_render_widget_host_view.cc:319-339`). Called by `WebContentsImpl` when
   the tab becomes visible, it attaches the `DelegatedFrameHost` to the
   compositor and tells the renderer to start producing frames. Everything is
   already wired up, so frames flow immediately.

No delay. No timer. No race condition. The capturer is born alongside the view.

### Why we don't need the full Electron approach

Electron's custom RWHV is ~1500 lines because Electron replaces the entire
compositor pipeline — it does its own off-screen rendering. We don't do that. We
use Chromium's default RWHV and default rendering pipeline, then capture from
the compositor externally via `FrameSinkVideoCapturer`.

We just need to know **when** the default RWHV is ready so we can call
`Attach()`. We don't need to own the RWHV — we just need to observe its
creation.

## Proposed Solution

Use a `WebContentsObserver` to detect when the RWHV is ready, then attach
immediately. Chromium's `content::WebContentsObserver` provides the
`RenderViewReady()` callback, which fires when the `blink::WebView` is ready. By
this point, the RWHV and its `FrameSinkId` exist.

The change is small:

1. **Make `ShellVideoConsumer` inherit from `WebContentsObserver`** (or create a
   small helper class that does).

2. **Override `RenderViewReady()`** to call `Attach()`.

3. **Delete the `PostDelayedTask` block** from
   `ShellBrowserMainParts::InitializeMessageLoopContext()`.

4. **Start observing the `WebContents`** right after `Shell::CreateNewWindow()`
   returns, instead of scheduling a delayed task.

This replaces the 2-second timer with an event-driven hook. The capturer
attaches as soon as the RWHV exists — no wasted frames, no arbitrary timeout, no
race condition.

### Estimated scope

~20-30 lines changed across 2-3 files (`shell_video_consumer.h`,
`shell_video_consumer.cc`, `shell_browser_main_parts.cc`).

## Experiments
