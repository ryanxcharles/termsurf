# Experiment 2: Prove WebKit hosting context export

## Description

This experiment proves the central compositor assumption before Surfari exists:
WebKit-rendered content must be exportable from the process that owns the
`WKWebView` and displayable in another host window/process through WebKit's
native layer-hosting machinery.

Chromium/Roamium does not share a `CALayerHost` object directly. The browser
process exports a renderable context identifier, and Ghostboard creates its own
hosting layer for that identifier. Surfari needs the analogous WebKit proof.

The local WebKit source already contains the primitives this experiment should
test:

- `Source/WebKit/Platform/cocoa/LayerHostingContext.h`
- `Source/WebKit/Platform/cocoa/LayerHostingContext.mm`
- `Source/WebKit/GPUProcess/webrtc/RemoteSampleBufferDisplayLayer.mm`
- `Source/WebKit/WebProcess/WebPage/mac/TiledCoreAnimationDrawingArea.mm`
- `Tools/MiniBrowser/mac`
- `Tools/WebKitTestRunner/mac`

The proof should start with the smallest two-process macOS harness that can
create a `WKWebView` in an owner process, navigate it to deterministic local
HTML, wrap the relevant WebKit/view/root layer in a `LayerHostingContext` or
equivalent WebKit hosting primitive, send the exported context/handle to a
separate host process, and display it there using the same class of Core
Animation hosting mechanism Ghostboard uses for Roamium.

The experiment should not implement Surfari, `libtermsurf_webkit`, Ghostboard
integration, protobuf changes, or broad WebKit patches. Its only job is to
determine whether the compositor boundary is viable and where the correct hook
lives.

## Changes

- Create a narrow two-process compositor proof harness for macOS. Prefer a
  tracked TermSurf-side harness over modifying WebKit first, unless the source
  audit proves a WebKit patch is necessary.
- If WebKit source changes are necessary, create a dedicated branch inside
  `webkit/src` before editing, record the branch name and upstream base commit
  in this experiment, and keep the patch limited to the compositor proof.
- The harness should create an owner process with a WebKit view using the
  MiniBrowser/WKWebView/WebKit2 pattern.
- The harness should create a separate host process with a window/layer that
  displays the exported WebKit surface through `LayerHostingContext`,
  `HostingContext`, `contextID`, `CALayerHost`, or WebKit's
  `createPlatformLayerForHostingContext` path.
- The owner-to-host handoff should use an explicit IPC or launch argument path
  that is close enough to the future Surfari-to-Ghostboard handoff to prove the
  process boundary. A same-process two-window proof may be useful evidence, but
  it is not sufficient for Pass.
- Add deterministic local test content for the proof, including visible text,
  CSS animation, a scrollable region, and a navigation target.
- Add a small README or notes file for any new harness directory explaining how
  to build and run the proof.
- Update this experiment's result with the exact hook path that worked or the
  exact reason the approach failed.
- Do not create Surfari, `libtermsurf_webkit`, Ghostboard integration, protobuf
  messages, release scripts, or install paths in this experiment.

## Verification

Run the harness from a clean TermSurf repo root after building WebKit debug
products:

```bash
git status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src status --short
webkit/src/Tools/Scripts/build-webkit --debug
```

Then build and run the compositor proof harness using the command documented by
the experiment implementation.

The proof must demonstrate all of the following:

- The owner process creates a WebKit view, loads deterministic local HTML, and
  logs a nonzero exported hosting/context identifier or equivalent hostable
  handle.
- The owner process sends the exported identifier/handle to a separate host
  process through an explicit IPC or launch argument path.
- The host process displays the WebKit-rendered content in a distinct host
  window/surface without creating its own `WKWebView`.
- The hosted content visibly updates for CSS animation or equivalent dynamic
  rendering without bitmap polling.
- Resizing the owner view or hosted surface causes the hosted WebKit content to
  resize without recreating the whole process.
- Scrolling and navigation still update the hosted surface.
- The implementation records whether the proof used:
  - `LayerHostingContext::create`;
  - `LayerHostingContext::setRootLayer`;
  - `LayerHostingContext::hostingContext`;
  - `LayerHostingContext::createPlatformLayerForHostingContext`;
  - raw `CALayerHost` / `contextId` SPI;
  - or another WebKit-native path.

**Pass** = WebKit content created by the owner process is rendered in a separate
host process through a WebKit/Core Animation hosting context, the host process
does not create its own `WKWebView`, dynamic updates are visible, resize works,
navigation works, and the result identifies the exact hook that should become
the future `libtermsurf_webkit` compositor API.

**Partial** = the harness builds and creates a WebKit view, but hosting fails or
only works in the same process/window, including a two-window same-process
prototype. The result must identify the exact failure point, the source path
investigated, and whether the next experiment should patch WebKit, use a
different WebKit hosting primitive, or fall back to a different compositor
strategy.

**Fail** = the harness cannot be built or cannot create a usable WebKit view, or
the failure is too ambiguous to identify a next technical step.

Before recording the result, capture:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
```

The TermSurf worktree must contain only the intended harness/docs/issue changes.
The WebKit checkout must either be clean or contain only the explicitly recorded
experiment patch on the dedicated experiment branch.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

**Verdict:** Changes required.

Finding:

- **Required:** The original pass criteria did not require proving the process
  boundary. A same-process two-window harness could have satisfied the wording
  without proving that WebKit content can cross from the owner process into a
  separate host process.

Fix:

- Tightened the design to require a two-process harness.
- Required the owner process to send the exported identifier or handle to a
  separate host process through explicit IPC or launch arguments.
- Required the host process to display the content without creating its own
  `WKWebView`.
- Marked same-process/two-window prototypes as **Partial**, not **Pass**.

The fixed design was re-reviewed by the same adversarial Codex subagent.

**Final verdict:** Approved.

The re-review confirmed that the two-process owner-to-host handoff is now
required for Pass, and that same-process prototypes are explicitly Partial.

## Result

**Result:** Pass

The compositor proof harness succeeded. A WebKit-owning process created a
`WKWebView`, loaded deterministic local HTML, exported the WebKit view's layer
through a private Core Animation remote context, and launched a separate host
process with the exported context ID. The host process created a `CALayerHost`
for that context ID and displayed the WebKit-rendered content without creating
its own `WKWebView`.

Implemented harness files:

- `surfari-proofs/hosting-context/README.md`
- `surfari-proofs/hosting-context/build.sh`
- `surfari-proofs/hosting-context/WebKitHostingProof.m`
- `surfari-proofs/hosting-context/test-content/index.html`
- `surfari-proofs/hosting-context/test-content/navigation.html`
- `surfari-proofs/hosting-context/.gitignore`

The harness uses a single Objective-C binary with two modes:

```text
WebKitHostingProof --owner
WebKitHostingProof --host <context-id>
```

The owner process performs the browser work. The host process only creates a
window with a `CALayerHost` pointed at the exported context ID.

Relevant implementation hook:

```objc
self.remoteContext = [CAContext remoteContextWithOptions:@{
    @"kCAContextCIFilterBehavior" : @"ignore",
}];
self.remoteContext.layer = self.webView.layer;
uint32_t contextId = self.remoteContext.contextId;
```

The host side uses:

```objc
CALayerHost *hostLayer = [CALayerHost layer];
hostLayer.contextId = self.contextId;
```

This proof used raw `CAContext` / `CALayerHost` SPI. It did not require a WebKit
source patch and did not use WebKit's internal `LayerHostingContext` wrapper
directly. The local WebKit source remains relevant because `LayerHostingContext`
wraps the same conceptual primitives and should still inform the future
`libtermsurf_webkit` API.

Verification commands:

```text
$ surfari-proofs/hosting-context/build.sh
built surfari-proofs/hosting-context/build/WebKitHostingProof

$ webkit/src/Tools/Scripts/build-webkit --debug
** BUILD SUCCEEDED ** [34.024 sec]
** BUILD SUCCEEDED ** [0.528 sec]
WebKit is now built (00m:40s).

$ git -C webkit/src rev-parse HEAD
1452a43959523449099b2616793fd2c5b6a6487e

$ git -C webkit/src rev-parse --abbrev-ref HEAD
main

$ git -C webkit/src status --short
<clean>
```

The harness run wrote logs to `logs/issue756-exp2-hosting-proof.log` and
screenshots to:

- `logs/issue756-exp2-screen-scroll.png`
- `logs/issue756-exp2-screen-navigation.png`
- `logs/issue756-exp2-screen.png` (copy of the final navigation screenshot)

Relevant log output:

```text
OWNER_LOADING pid=60363 url=/Users/astrohacker/dev/termsurf/surfari-proofs/hosting-context/test-content/index.html
OWNER_NAVIGATION_FINISHED pid=60363 url=file:///Users/astrohacker/dev/termsurf/surfari-proofs/hosting-context/test-content/index.html
OWNER_EXPORTED_CONTEXT pid=60363 context_id=2129917052 webview_layer=0x77d3d2700
OWNER_LAUNCHED_HOST host_pid=60373 context_id=2129917052
HOST_READY pid=60373 context_id=2129917052 host_has_no_wkwebview=1
OWNER_RESIZED_WEBVIEW pid=60363 size=620x388
OWNER_NAVIGATING_AFTER_EXPORT pid=60363 url=/Users/astrohacker/dev/termsurf/surfari-proofs/hosting-context/test-content/navigation.html
OWNER_NAVIGATION_FINISHED pid=60363 url=file:///Users/astrohacker/dev/termsurf/surfari-proofs/hosting-context/test-content/navigation.html
```

After completion review found that the original result did not separately record
scroll/dynamic-update evidence, the harness was strengthened with a
`WKScriptMessageHandler`. The initial page now reports the JavaScript-driven
scroll/update milestone from inside WebKit. The same JavaScript update changes
the document background to red and changes target text/color so the hosted
surface has visible dynamic-rendering evidence:

```text
OWNER_SCRIPT_MESSAGE pid=60994 name=proof body={
    event = scrolled;
    scrollY = 720;
    status = "Owner page updated by JavaScript animation tick.";
}
```

The rerun also captured the same owner/host handoff, resize, and post-export
navigation:

```text
OWNER_EXPORTED_CONTEXT pid=60994 context_id=2877406041 webview_layer=0x81cee2730
OWNER_LAUNCHED_HOST host_pid=61001 context_id=2877406041
HOST_READY pid=61001 context_id=2877406041 host_has_no_wkwebview=1
OWNER_RESIZED_WEBVIEW pid=60994 size=620x388
OWNER_NAVIGATING_AFTER_EXPORT pid=60994 url=/Users/astrohacker/dev/termsurf/surfari-proofs/hosting-context/test-content/navigation.html
OWNER_NAVIGATION_FINISHED pid=60994 url=file:///Users/astrohacker/dev/termsurf/surfari-proofs/hosting-context/test-content/navigation.html
```

Visual inspection of the screenshots confirmed:

- the owner and host are separate visible windows;
- the owner window is blank after export because its WebKit layer was moved into
  the remote context;
- `logs/issue756-exp2-screen-scroll.png` shows the host window displaying the
  initial WebKit page after the JavaScript-triggered dynamic update and scroll:
  the hosted surface has turned red and the scrollbar has moved down;
- `logs/issue756-exp2-screen-navigation.png` shows the host window displaying
  the WebKit-rendered navigation page;
- the displayed page says `Issue 756 navigation complete`;
- the host process log states `host_has_no_wkwebview=1`.

Final status checks:

```text
$ git status --short
?? surfari-proofs/

$ git status --short --ignored surfari-proofs/hosting-context
?? surfari-proofs/hosting-context/
!! surfari-proofs/hosting-context/build/
```

The untracked `surfari-proofs/` directory contains the intended harness source.
The compiled binary is under the ignored `build/` directory and is not intended
to be committed.

## Conclusion

The core compositor assumption is viable on macOS: a process that owns a
`WKWebView` can export that rendered surface through a Core Animation context
ID, and a separate host process can display it through `CALayerHost` without
creating its own `WKWebView`.

This is the critical proof needed before building `libtermsurf_webkit` and the
Surfari Rust process. The next experiment should establish the durable WebKit
branch/patch workflow and decide whether `libtermsurf_webkit` should:

- use raw `CAContext` / `CALayerHost` SPI directly for the first macOS
  implementation; or
- add a small WebKit-side wrapper around WebKit's existing `LayerHostingContext`
  path so Surfari can expose a cleaner engine-owned compositor API.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Verdict:** Changes required.

Finding:

- **Required:** The original result evidence did not separately prove the
  experiment's visible dynamic-rendering and scrolling criteria. It recorded the
  process handoff, resize, navigation, and final navigation screenshot, but not
  visible hosted animation/scroll/update evidence.

Fix:

- Added a `WKScriptMessageHandler` to the owner process.
- Updated the initial test page so JavaScript changes visible page state, turns
  the document red, scrolls to `720`, and reports the update through WebKit
  script messaging.
- Captured `logs/issue756-exp2-screen-scroll.png`, which visibly shows the
  hosted surface after the dynamic update and scroll.
- Updated the result section to record the script message, the scroll
  screenshot, and the visible dynamic-rendering evidence.

A focused re-review initially found that scroll evidence was present but the
visible dynamic-rendering evidence was still too indirect. The test page was
then changed so the JavaScript update turns the whole hosted surface red before
the scroll screenshot is captured.

The fixed result was re-reviewed by an adversarial Codex subagent.

**Final verdict:** Approved.

The re-review confirmed that the prior required finding is resolved: the log
records the WebKit script message with `event = scrolled` and `scrollY = 720`,
`logs/issue756-exp2-screen-scroll.png` visibly shows the hosted surface turned
red with the scrollbar moved down, and no new required findings were introduced.
