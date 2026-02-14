# Issue 501: Two Profiles in ts5

## Goal

Build a clean two-profile browser demo in ts5: a minimal Chromium embedder
(`termsurf-browser`) that renders webpages to IOSurfaces and sends them via XPC,
and a Swift compositor (`ts5/two-profiles/`) that receives and displays them
side by side at 60fps.

This replaces the ts4 proof-of-concept code (Issues 412–416) with
production-quality implementations that will serve as the foundation for
Ghostty's browser pane integration.

## Motivation

The ts4 experiments proved the architecture works. But the Chromium side was
built as a rapid PoC with problems that need fixing before we build on top of
it:

1. **218-file Content Shell copy.** The One Profile app is a wholesale copy of
   `content/shell/` with renamed identifiers. Most of those files are irrelevant
   (DevTools, web test infrastructure, Linux/Windows platform code). The actual
   delta we need is ~10 files.

2. **Bad naming.** "One Profile" describes an architectural constraint, not what
   the app does. It's a headless browser renderer — a profile server.

3. **Dock icon.** The `--hidden` flag hides the window with `orderOut:nil`, but
   the app is still a regular `NSApplication` that shows in the Dock. Each
   profile server adds a Dock icon, which is unacceptable.

4. **Fragile capturer attachment.** The video consumer is wired up via a
   hardcoded 2-second `PostDelayedTask` to wait for the `RenderWidgetHostView`
   to exist. This is a race condition waiting to happen.

Since the architecture is proven, we can afford to get this right.

## Background

### What's proven

| Issue | Finding                                                                                                                                           |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| 406   | Chromium Content API supports multiple `BrowserContext` instances with isolated storage. CEF ruled out (31fps ceiling).                           |
| 407   | In-process Chromium PoC: two profiles coexist, single pane at 60fps. Dual-pane hit 2fps due to visibility tracking.                               |
| 413   | Root cause: two `BrowserContext` in one process = 2fps. Two `WebContents` sharing one `BrowserContext` = 60fps. One profile per process required. |
| 414   | XPC + IOSurface Mach port transfer solves the dual-pane problem. `FrameSinkVideoCapturer` delivers IOSurfaces at 60fps even with hidden window.   |
| 415   | Same architecture in pure Swift. XPC C API, IOSurface, Metal, CADisplayLink all work natively.                                                    |

### Architecture

```
termsurf-browser A (headless)         termsurf-browser B (headless)
--session-id=profile-a                --session-id=profile-b
IOSurface 1600x1200 @ 60fps          IOSurface 1600x1200 @ 60fps
        |                                     |
        | XPC Mach port                       | XPC Mach port
        v                                     v
    +--------------------------------------+
    |     Swift Compositor (Metal)         |
    |  +----------------+----------------+ |
    |  |  Left viewport | Right viewport | |
    |  |  profile-a     | profile-b      | |
    |  |  60fps         | 60fps          | |
    |  +----------------+----------------+ |
    +--------------------------------------+
```

Two process types:

1. **termsurf-browser** — Headless Chromium embedder. One instance per browser
   profile. Renders a webpage via the Content API, captures frames with
   `FrameSinkVideoCapturer`, sends IOSurface Mach ports to the compositor via
   XPC. No Dock icon, no visible window.

2. **Compositor** (`ts5/two-profiles/`) — Swift app that registers as an XPC
   Mach service, receives IOSurface Mach ports, and composites two panes side by
   side in a Metal window at 60fps.

### XPC protocol

Messages from termsurf-browser to compositor:

- **`display_surface`** —
  `{ action: "display_surface", session_id: "<id>",
  iosurface_port: <mach_send>, width: <int64>, height: <int64> }`
- **`register`** — `{ action: "register", session_id: "<id>" }`

The compositor maps `session_id` to a pane: `"profile-b"` goes right, everything
else goes left.

## Part 1: termsurf-browser (Chromium)

### Naming

The Chromium embedder is called `termsurf-browser`. This describes its role: it
is the browser engine backend for TermSurf. Inside the Chromium source tree, the
directory follows Chromium's underscore convention: `content/termsurf_browser/`.

The build target is `termsurf_browser`. The output app bundle is
`TermSurf Browser.app`.

### Approach: link against content_shell_lib, don't copy it

The ts4 approach copied all 218 files from `content/shell/`. The better approach
is to link against `//content/shell:content_shell_lib` as a dependency and
subclass only what we need. This is the same pattern used in Issue 407's
`content/two_profiles/` directory, which worked with ~10 files.

Content Shell provides:

- `Shell` — Window and WebContents lifecycle
- `ShellBrowserContext` — Profile storage with `GetPath()`
- `ShellBrowserMainParts` — Browser lifecycle
- `ShellContentBrowserClient` — Browser client delegate
- `ShellMainDelegate` — Application entry point

We subclass the parts we need to override and leave everything else to
content_shell_lib.

### Files

```
chromium/src/content/termsurf_browser/
├── BUILD.gn
├── termsurf_browser_main.cc          — Entry point
├── termsurf_browser_main_mac.cc      — macOS entry (framework loading)
├── termsurf_browser_main_delegate.h  — Subclass ShellMainDelegate
├── termsurf_browser_main_delegate.cc
├── termsurf_browser_main_parts.h     — Subclass ShellBrowserMainParts
├── termsurf_browser_main_parts.cc
├── termsurf_browser_context.h        — Subclass ShellBrowserContext
├── termsurf_browser_context.cc
├── termsurf_video_consumer.h         — FrameSinkVideoCapturer + XPC sender
├── termsurf_video_consumer.cc
└── Info.plist                        — LSUIElement = true
```

~12 files instead of 218.

### Key classes

**`TermSurfBrowserMainDelegate`** — Extends `ShellMainDelegate`. Overrides
`CreateContentBrowserClient()` to return our custom browser client that uses
`TermSurfBrowserMainParts`.

**`TermSurfBrowserMainParts`** — Extends `ShellBrowserMainParts`. Overrides
browser context creation to use `TermSurfBrowserContext` with the
`--user-data-dir` path. Wires up `TermSurfVideoConsumer` with proper lifecycle
hooks (not a hardcoded 2-second delay).

**`TermSurfBrowserContext`** — Extends `ShellBrowserContext`. Overrides
`GetPath()` to return the `--user-data-dir` path, giving each instance isolated
cookies, localStorage, and cache.

**`TermSurfVideoConsumer`** — Implements `viz::mojom::FrameSinkVideoConsumer`.
This is the core of the frame delivery pipeline:

1. Creates a `viz::ClientFrameSinkVideoCapturer` from `HostFrameSinkManager`
2. Configures: `PIXEL_FORMAT_ARGB`, 16ms min capture period, auto-throttling
   disabled
3. Attaches to the WebContents' `FrameSinkId`
4. On `OnFrameCaptured`: extracts `IOSurfaceRef` from the GPU memory buffer,
   creates a Mach port with `IOSurfaceCreateMachPort()`, sends via XPC, signals
   `Done()` to release the buffer

Ported from `ShellVideoConsumer` in the One Profile app, but with proper
lifecycle management.

### Dock icon fix: LSUIElement

The Info.plist sets `LSUIElement = true`, making termsurf-browser a background
app with no Dock icon and no menu bar. This is the standard macOS mechanism for
helper processes.

The risk is that `LSUIElement` might affect Chromium's compositor visibility
chain — the same chain that causes 2fps when a `BrowserContext`'s views aren't
"visible." However, `LSUIElement` only removes the Dock icon; it does not affect
window existence or NSView visibility. The existing `orderOut:nil` approach
already proved that hiding the window doesn't affect `FrameSinkVideoCapturer`
(Issue 414). `LSUIElement` is a strictly less invasive change than hiding the
window, so it should not introduce new problems. This needs verification in
Experiment 1.

### CLI flags

Same flags as the One Profile app, carried forward:

| Flag                     | Purpose                                          |
| ------------------------ | ------------------------------------------------ |
| `--hidden`               | Hide the window after creation (`orderOut:nil`). |
| `--xpc-service=<name>`   | Connect to a named XPC Mach service.             |
| `--session-id=<id>`      | Identifier sent with every frame.                |
| `--user-data-dir=<path>` | Profile storage directory.                       |
| (positional URL)         | Webpage to load.                                 |

### Chromium branch

Create a new branch `146.0.7650.0-issue-501` in `chromium/src/`, forked from the
vanilla `146.0.7650.0` tag (not from Issue 414). This is a clean rewrite, not a
continuation of the One Profile app. The One Profile code at
`content/one_profile/` remains on its own branches as historical reference.

```bash
cd chromium/src
git checkout -b 146.0.7650.0-issue-501 146.0.7650.0
```

### BUILD.gn

```gn
import("//build/config/features.gni")

source_set("termsurf_browser_lib") {
  sources = [
    "termsurf_browser_main_delegate.cc",
    "termsurf_browser_main_delegate.h",
    "termsurf_browser_main_parts.cc",
    "termsurf_browser_main_parts.h",
    "termsurf_browser_context.cc",
    "termsurf_browser_context.h",
    "termsurf_video_consumer.cc",
    "termsurf_video_consumer.h",
  ]

  deps = [
    "//content/shell:content_shell_lib",
    "//components/viz/host:host",
    "//services/viz/privileged/mojom/compositing:compositing_interfaces",
    "//media",
  ]
}

# macOS app bundle
mac_app_bundle("termsurf_browser") {
  output_name = "TermSurf Browser"
  sources = [
    "termsurf_browser_main.cc",
    "termsurf_browser_main_mac.cc",
  ]
  deps = [
    ":termsurf_browser_lib",
    "//content/shell:content_shell_framework",
  ]
  info_plist = "Info.plist"
}
```

Register in root BUILD.gn:

```gn
group("gn_all") {
  deps = [
    ...
    "//content/termsurf_browser:termsurf_browser",
  ]
}
```

### Build command

```bash
cd chromium/src
export PATH="$(cd ../depot_tools && pwd):$PATH"
autoninja -C out/Default termsurf_browser
```

## Part 2: Swift compositor (ts5/two-profiles/)

### Project structure

```
ts5/two-profiles/
├── Package.swift                           — SwiftPM manifest
├── Sources/
│   └── TwoProfiles/
│       ├── main.swift                      — Entry point, XPC, Metal, rendering
│       └── Shaders.metal                   — Vertex + fragment shaders
├── com.termsurf.two-profiles.plist         — Launchd agent definition
└── Makefile                                — Build shaders + swift build
```

Ported from `ts4/two-profiles-swift/Sources/Receiver/main.swift` (328 lines).
The code is small enough that a single file with MARK sections is clearer than
multiple files.

### Changes from ts4

1. **XPC service name.** `com.termsurf.two-profiles` (no `-swift` or `-ts5`
   suffix — this is the canonical name going forward).
2. **Target name.** `TwoProfiles` instead of `Receiver`.
3. **Makefile.** Compiles Metal shaders with `xcrun metal` / `xcrun metallib`
   (SPM does not compile `.metal` files).
4. **Log path.** `~/dev/termsurf/logs/two-profiles.log`.

No functional changes to XPC handling, Metal pipeline, or rendering logic.

### Known Swift gotchas (resolved in Issue 415)

1. SPM does not compile `.metal` files — use `xcrun metal` / `xcrun metallib`.
2. `mach_task_self()` is a C macro — use `mach_task_self_` in Swift.
3. Pixel format is `.bgra8Unorm_srgb` (lowercase) in Swift.
4. `MTLTextureUsage.shaderRead` needs the explicit type prefix.
5. `IOSurfaceRef` is ARC-managed — no `CFRetain`/`CFRelease`.

## Build and run

```bash
# 1. Build termsurf-browser (Chromium)
cd chromium/src
export PATH="$(cd ../depot_tools && pwd):$PATH"
autoninja -C out/Default termsurf_browser

# 2. Build compositor (Swift)
cd ts5/two-profiles
make

# 3. Load compositor as launchd service
launchctl bootstrap gui/$(id -u) \
  ~/dev/termsurf/ts5/two-profiles/com.termsurf.two-profiles.plist

# 4. Start test page server
cd ts4/box-demo && bun run server.ts &

# 5. Start two profile servers
cd chromium/src

out/Default/TermSurf\ Browser.app/Contents/MacOS/TermSurf\ Browser \
  --hidden \
  --xpc-service=com.termsurf.two-profiles \
  --session-id=profile-a \
  --user-data-dir=$HOME/.config/termsurf/poc/profile-a \
  http://localhost:9407 2>&1 &

out/Default/TermSurf\ Browser.app/Contents/MacOS/TermSurf\ Browser \
  --hidden \
  --xpc-service=com.termsurf.two-profiles \
  --session-id=profile-b \
  --user-data-dir=$HOME/.config/termsurf/poc/profile-b \
  http://localhost:9407 2>&1 &
```

## Success criteria

- Two panes in one window, each showing the spinning blue square
- Different localStorage identity in each pane (profile isolation)
- Both at 60fps sustained for 60+ seconds
- Pixel-perfect Retina quality (1600x1200 IOSurface, sRGB)
- No Dock icon for termsurf-browser processes
- Pure Swift compositor, builds with `make`
- Chromium embedder is ~12 files linking against content_shell_lib (not a copy)

## Experiments

### Experiment 1: Build termsurf-browser

#### Hypothesis

A minimal Chromium target at `content/termsurf_browser/` that links against
`content_shell_lib` and subclasses only what's needed (~12 files) will produce a
working headless browser that captures frames and sends IOSurface Mach ports via
XPC at 60fps — without a Dock icon.

#### Steps

##### Step 1: Create Chromium branch

```bash
cd chromium/src
git checkout -b 146.0.7650.0-issue-501 146.0.7650.0
```

##### Step 2: Create directory and BUILD.gn

Create `content/termsurf_browser/` with the BUILD.gn from the design above.
Register the target in the root BUILD.gn.

##### Step 3: Write the entry points

- `termsurf_browser_main.cc` — Calls `content::ContentMain()` with our delegate
- `termsurf_browser_main_mac.cc` — macOS framework loading (same pattern as
  content_shell)

##### Step 4: Write the delegate classes

- `TermSurfBrowserMainDelegate` — Extends `ShellMainDelegate`, overrides
  `CreateContentBrowserClient()` to return our browser client
- `TermSurfBrowserMainParts` — Extends `ShellBrowserMainParts`, overrides
  browser context creation, wires up `TermSurfVideoConsumer`
- `TermSurfBrowserContext` — Extends `ShellBrowserContext`, overrides
  `GetPath()` for `--user-data-dir`

##### Step 5: Write the video consumer

Port `ShellVideoConsumer` from the One Profile app as `TermSurfVideoConsumer`.
Same `FrameSinkVideoCapturer` + XPC pipeline, but with proper lifecycle
management instead of a hardcoded delay.

##### Step 6: Create Info.plist with LSUIElement

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>LSUIElement</key>
    <true/>
    ...
</dict>
</plist>
```

##### Step 7: Build and test

```bash
autoninja -C out/Default termsurf_browser
```

Test with the existing ts4/two-profiles-swift receiver to verify frames arrive
at 60fps. Verify no Dock icon appears.

### Experiment 2: Build Swift compositor

#### Hypothesis

Porting the ts4/two-profiles-swift receiver into ts5/two-profiles with updated
names and paths will produce an identical working compositor. No new unknowns.

#### Steps

##### Step 1: Create project structure

- `Package.swift` — macOS 14+, target `TwoProfiles`
- `Sources/TwoProfiles/main.swift` — Port from ts4 with updated XPC service name
  (`com.termsurf.two-profiles`)
- `Sources/TwoProfiles/Shaders.metal` — Copy from ts4
- `Makefile` — Compile shaders + `swift build`
- `com.termsurf.two-profiles.plist` — Launchd agent

##### Step 2: Build and verify

```bash
cd ts5/two-profiles
make
```

##### Step 3: End-to-end test

Load the launchd plist, start the test page server, launch two termsurf-browser
instances (from Experiment 1), and verify two panes at 60fps with different
localStorage identities.
