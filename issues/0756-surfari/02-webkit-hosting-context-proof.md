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
