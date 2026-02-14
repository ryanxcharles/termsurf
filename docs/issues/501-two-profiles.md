# Issue 501: Two Profiles in ts5

## Goal

Port the Swift Two Profiles receiver from ts4 (Issue 415) into
`ts5/two-profiles` as a standalone SwiftPM app. Two hidden Chromium profile
servers send IOSurface frames via XPC to a Swift receiver that composites them
side by side in one Metal window at 60fps.

This is the same proven architecture from Issues 414 and 415, relocated to the
ts5 directory where active development happens.

## Motivation

ts5 will embed Chromium browser panes alongside Ghostty terminal panes. Before
touching Ghostty's rendering pipeline, we need a working Two Profiles receiver
in the ts5 tree that can serve as:

1. **A reference implementation.** The receiver demonstrates the exact IOSurface
   import, Metal compositing, and XPC patterns that will eventually live inside
   Ghostty's Swift shell.

2. **A test harness.** As we modify the Chromium fork (branch changes, Content
   API experiments), the receiver provides a quick way to verify that profile
   servers still deliver frames correctly.

3. **A starting point for integration.** The ~330 lines of Swift in the receiver
   map directly to what Ghostty's `MetalView` (or equivalent) will need to
   composite browser panes.

## Background

### What's proven

| Issue | Finding                                                                                                                                                 |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 406   | Chromium Content API supports multiple `BrowserContext` instances in one process with isolated storage. CEF ruled out (31fps ceiling).                  |
| 407   | In-process Chromium PoC: two profiles coexist, single pane renders at 60fps. Dual-pane hit 2fps due to visibility tracking.                             |
| 414   | XPC + IOSurface Mach port transfer solves the dual-pane problem: two hidden Chromium profile servers, one Objective-C++ Metal receiver, 60fps per pane. |
| 415   | Same architecture in pure Swift. Proves XPC C API, IOSurface, Metal, and CADisplayLink all work natively in Swift.                                      |

### Architecture

```
Profile Server A (hidden Chromium)     Profile Server B (hidden Chromium)
--session-id=profile-a                 --session-id=profile-b
IOSurface 1600x1200 @ 60fps           IOSurface 1600x1200 @ 60fps
        |                                      |
        | XPC Mach port                        | XPC Mach port
        v                                      v
    +--------------------------------------+
    |     Swift Receiver (Metal)           |
    |  +----------------+----------------+ |
    |  |  Left viewport | Right viewport | |
    |  |  profile-a     | profile-b      | |
    |  |  60fps         | 60fps          | |
    |  +----------------+----------------+ |
    +--------------------------------------+
```

Two process types (no launcher):

1. **Receiver** (this app) — Creates a macOS window with two Metal viewports.
   Registers as XPC Mach service. Receives IOSurface Mach ports from profile
   servers, reconstructs IOSurfaces, creates Metal textures, composites at 60fps
   via CADisplayLink.

2. **Profile servers** — Hidden Chromium instances (One Profile app from the
   Chromium fork). Each renders a webpage off-screen to an IOSurface, sends the
   Mach port to the receiver via XPC. One process per browser profile.

### XPC protocol

Messages from profile server to receiver:

- **`display_surface`** —
  `{ action: "display_surface", session_id: "<id>",
  iosurface_port: <mach_send> }`
- **`register`** — `{ action: "register", session_id: "<id>" }`

The receiver maps `session_id` to a pane: `"profile-b"` goes right, everything
else goes left.

## Chromium sender

### No new Chromium modifications needed

The One Profile app already exists at `chromium/src/content/one_profile/` with
full XPC frame delivery support. It was built across Issues 412–414 and is
ready to use as-is.

### Branch

Use branch `146.0.7650.0-issue-414` (or any descendant — `issue-415`,
`issue-416`). The current checked-out branch is `146.0.7650.0-issue-416`.
All branches from Issue 414 onward contain the XPC frame delivery code.

### What the One Profile app is

A Content Shell clone at `chromium/src/content/one_profile/` — an independent
copy of Chromium's reference Content API embedder (`content/shell/`) with
renamed identifiers and web test support removed. It creates a single
`BrowserContext` and renders a webpage via the Content API.

Key classes (all in `content/one_profile/`):

- **`OneProfileMainDelegate`** — Application entry point. Replaces
  `ShellMainDelegate`.
- **`OneProfileBrowserMainParts`** — Browser lifecycle. Creates the
  `ShellBrowserContext`, wires up the `ShellVideoConsumer`.
- **`OneProfileContentBrowserClient`** — Browser client delegate.
- **`ShellBrowserContext`** — Profile storage. `GetPath()` returns the
  `--user-data-dir` path, isolating cookies/localStorage/cache per profile.
- **`ShellVideoConsumer`** — Frame capture and XPC delivery (the critical
  piece). Implements `viz::mojom::FrameSinkVideoConsumer`.

### How frame capture works

`ShellVideoConsumer` (added in Issue 414) captures rendered frames and sends
them over XPC:

1. Creates a `viz::ClientFrameSinkVideoCapturer` from Chromium's
   `HostFrameSinkManager`.
2. Configures capture: `PIXEL_FORMAT_ARGB`, 16ms minimum capture period
   (60fps), auto-throttling disabled.
3. Gets the `FrameSinkId` from the WebContents' `RenderWidgetHostView`.
4. Starts capture with `kPreferMappableSharedImage` — on macOS this delivers
   GPU memory buffers backed by IOSurfaces.
5. On each `OnFrameCaptured` callback:
   - Extracts the `IOSurfaceRef` from the GPU memory buffer handle
   - Creates a Mach port: `IOSurfaceCreateMachPort(io_surface)`
   - Builds an XPC message: `{ action: "display_surface",
     iosurface_port: <mach_send>, width: <int64>, height: <int64>,
     session_id: "<id>" }`
   - Sends via `xpc_connection_send_message()` (async, no reply)
   - Deallocates the Mach port and signals `Done()` to release the buffer

### CLI flags

| Flag | Purpose |
| ---- | ------- |
| `--hidden` | Hide the window after creation (`orderOut:nil`). The capturer keeps delivering at 60fps. |
| `--xpc-service=<name>` | Connect to a named XPC Mach service as a client. |
| `--session-id=<id>` | Identifier sent with every frame message. The receiver maps this to a pane. |
| `--user-data-dir=<path>` | Profile storage directory. Each instance gets a different path for isolation. |
| (positional URL) | The webpage to load (e.g., `http://localhost:9407`). |

### Build command

```bash
cd chromium/src
export PATH="$(cd ../depot_tools && pwd):$PATH"
autoninja -C out/Default one_profile
```

Build time: ~15–20 seconds incremental (after initial full build of ~1.5 hours).

The output is `chromium/src/out/Default/One Profile.app`.

## Project structure

```
ts5/two-profiles/
├── Package.swift                           — SwiftPM manifest
├── Sources/
│   └── TwoProfiles/
│       ├── main.swift                      — Entry point, XPC listener, Metal
│       │                                     setup, rendering (single file)
│       └── Shaders.metal                   — Vertex + fragment shaders
├── com.termsurf.two-profiles-ts5.plist     — Launchd agent definition
└── Makefile                                — Build shaders + swift build
```

### Why a single file

The ts4/two-profiles-swift receiver is 328 lines in a single `main.swift`. At
this size, splitting into AppDelegate/XPCListener/Renderer adds indirection
without clarity. The code splits naturally into MARK sections: XPC state,
message handler, XPC listener, Metal setup, render loop, main.

### Why a new XPC service name

`com.termsurf.two-profiles-ts5` avoids conflicting with the existing
`com.termsurf.two-profiles-swift` (ts4). Both can coexist in launchd for
side-by-side comparison.

## Differences from ts4/two-profiles-swift

The port is nearly identical. The differences are:

1. **Location.** `ts5/two-profiles/` instead of `ts4/two-profiles-swift/`.
2. **XPC service name.** `com.termsurf.two-profiles-ts5` instead of
   `com.termsurf.two-profiles-swift`.
3. **Target name.** `TwoProfiles` instead of `Receiver`.
4. **Makefile.** Adds a `make` target that compiles Metal shaders and runs
   `swift build` in one step (SPM does not compile `.metal` files).
5. **Log paths.** `~/dev/termsurf/logs/two-profiles-ts5.log`.

No functional changes to the receiver logic, Metal pipeline, or XPC handling.

## Launchd service

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.termsurf.two-profiles-ts5</string>
    <key>MachServices</key>
    <dict>
        <key>com.termsurf.two-profiles-ts5</key>
        <true/>
    </dict>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/ryan/dev/termsurf/ts5/two-profiles/.build/debug/TwoProfiles</string>
    </array>
    <key>StandardOutPath</key>
    <string>/Users/ryan/dev/termsurf/logs/two-profiles-ts5.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/ryan/dev/termsurf/logs/two-profiles-ts5.log</string>
</dict>
</plist>
```

## Build and run

```bash
# Build receiver
cd ts5/two-profiles
make

# Load launchd plist
launchctl bootstrap gui/$(id -u) \
  ~/dev/termsurf/ts5/two-profiles/com.termsurf.two-profiles-ts5.plist

# Start test page server
cd ts4/box-demo && bun run server.ts &

# Start profile servers (One Profile app from Chromium fork)
cd chromium/src

out/Default/One\ Profile.app/Contents/MacOS/One\ Profile \
  --hidden \
  --xpc-service=com.termsurf.two-profiles-ts5 \
  --session-id=profile-a \
  --user-data-dir=$HOME/.config/termsurf/poc/profile-a \
  http://localhost:9407 2>&1 &

out/Default/One\ Profile.app/Contents/MacOS/One\ Profile \
  --hidden \
  --xpc-service=com.termsurf.two-profiles-ts5 \
  --session-id=profile-b \
  --user-data-dir=$HOME/.config/termsurf/poc/profile-b \
  http://localhost:9407 2>&1 &
```

## Success criteria

- Two panes in one window, each showing the spinning blue square
- Different localStorage identity in each pane (profile isolation)
- Both at 60fps sustained for 60+ seconds
- Pixel-perfect Retina quality (1600x1200 IOSurface, sRGB, 1:1 mapping)
- Pure Swift receiver, builds with `make` (SwiftPM + xcrun metal)
- Uses CADisplayLink (not deprecated CVDisplayLink)

## Known issues from ts4

These were resolved in Issue 415 and the solutions carry forward:

1. **SPM does not compile `.metal` files.** Compile manually with `xcrun metal`
   / `xcrun metallib`. The Makefile handles this.
2. **`mach_task_self()` is a C macro.** Use `mach_task_self_` in Swift.
3. **Pixel format naming.** `.bgra8Unorm_srgb` (lowercase) in Swift.
4. **`MTLTextureUsage.shaderRead`** needs the explicit type prefix.
5. **CFRetain/CFRelease unnecessary.** Swift manages `IOSurfaceRef` via ARC.

## Experiments

### Experiment 1: Port the receiver

#### Hypothesis

Copying the proven ts4/two-profiles-swift code into ts5/two-profiles with
updated paths and service names will produce an identical working receiver. No
new unknowns — every API and pattern was validated in Issue 415.

#### Steps

##### Step 1: Create project structure

Create the SwiftPM project at `ts5/two-profiles/`:

- `Package.swift` — macOS 14+, target `TwoProfiles`, link Cocoa/Metal/
  QuartzCore/IOSurface frameworks
- `Sources/TwoProfiles/main.swift` — Port from
  `ts4/two-profiles-swift/Sources/Receiver/main.swift` with updated XPC service
  name (`com.termsurf.two-profiles-ts5`) and window title
- `Sources/TwoProfiles/Shaders.metal` — Copy from
  `ts4/two-profiles-swift/Sources/Receiver/Shaders.metal`

##### Step 2: Create Makefile

```makefile
.PHONY: build clean

build: shaders
	swift build

shaders: .build/debug/shaders.metallib

.build/debug/shaders.metallib: Sources/TwoProfiles/Shaders.metal
	mkdir -p .build/debug
	xcrun metal -c $< -o .build/debug/Shaders.air
	xcrun metallib .build/debug/Shaders.air -o $@

clean:
	swift package clean
	rm -f .build/debug/Shaders.air .build/debug/shaders.metallib
```

##### Step 3: Create launchd plist

Create `com.termsurf.two-profiles-ts5.plist` with service name
`com.termsurf.two-profiles-ts5`, binary path pointing to
`.build/debug/TwoProfiles`, logs to `~/dev/termsurf/logs/two-profiles-ts5.log`.

##### Step 4: Build and verify

```bash
cd ts5/two-profiles
make
```

Verify that `.build/debug/TwoProfiles` and `.build/debug/shaders.metallib` are
produced.

##### Step 5: Run with profile servers

```bash
# Load launchd plist
launchctl bootstrap gui/$(id -u) \
  ~/dev/termsurf/ts5/two-profiles/com.termsurf.two-profiles-ts5.plist

# Start test server
cd ts4/box-demo && bun run server.ts &

# Start two profile servers
cd chromium/src
out/Default/One\ Profile.app/Contents/MacOS/One\ Profile \
  --hidden \
  --xpc-service=com.termsurf.two-profiles-ts5 \
  --session-id=profile-a \
  --user-data-dir=$HOME/.config/termsurf/poc/profile-a \
  http://localhost:9407 2>&1 &

out/Default/One\ Profile.app/Contents/MacOS/One\ Profile \
  --hidden \
  --xpc-service=com.termsurf.two-profiles-ts5 \
  --session-id=profile-b \
  --user-data-dir=$HOME/.config/termsurf/poc/profile-b \
  http://localhost:9407 2>&1 &
```

Verify:

- Window appears with two panes
- Both panes show spinning blue square at 60fps
- Different localStorage identity in each pane
- Logs show `L: 60 (60.0 fps) R: 60 (60.0 fps)`
