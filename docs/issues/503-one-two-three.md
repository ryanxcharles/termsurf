# Issue 503: One, Two, Three

## Problem

The ts4 proof-of-concept demonstrated two **different** browser profiles
rendering side by side at 60fps (Issue 414, Issue 501). But TermSurf needs to
support a harder case: two tabs from the **same** profile in the same window.

There is exactly **one profile server process per profile** — this is a hard
constraint, not a design preference. Two Chromium processes cannot share the
same user data directory (SingletonLock). This was proven in ts2/ts3 with CEF
and remains true with the Content API. A profile server must therefore handle
**multiple WebContents** within a single process, each with its own
`FrameSinkVideoCapturer` delivering an independent IOSurface stream via XPC.

We've never tested this. The current Chromium Profile Server creates exactly one
`Shell` with one `WebContents` and one `ShellVideoConsumer` in
`InitializeMessageLoopContext()`. It needs to support an unlimited number.

Additionally, the working Swift compositor and box-demo test page live in
`ts4/`, but ts5 is the active development directory. These need to be ported to
`ts5/` so that ts5 has its own self-contained test infrastructure.

## Goals

1. **Port the box-demo and Swift compositor from ts4 to ts5.** The ts5 directory
   should have its own working copies that don't depend on ts4.

2. **Validate one profile, one tab.** A single Chromium Profile Server process
   with one WebContents rendering one page to one pane — the simplest case.

3. **Validate two profiles, one tab each.** Two Chromium Profile Server
   processes, each with a different `--user-data-dir`, each hosting one
   WebContents. Two panes in one window. This is the Issue 414/501 case,
   re-validated with the ts5 test infrastructure.

4. **Validate two profiles, three tabs.** Two Chromium Profile Server processes:
   one hosting **two** WebContents (same profile, two tabs), one hosting **one**
   WebContents (different profile). Three panes in one window. The compositor
   opens two XPC connections to the first profile server and one to the second.

## Architecture

### One profile server per profile

```
Profile Server A (profile-a data dir)
├── WebContents 1 → FrameSinkVideoCapturer → IOSurface → XPC connection 1
├── WebContents 2 → FrameSinkVideoCapturer → IOSurface → XPC connection 2
└── WebContents N → ...

Profile Server B (profile-b data dir)
├── WebContents 1 → FrameSinkVideoCapturer → IOSurface → XPC connection 1
└── WebContents N → ...
```

The compositor (Swift app) connects to profile servers via XPC. Each connection
represents one tab. A profile server can accept an unlimited number of
connections, creating a new WebContents and capturer for each.

### What changes in the Chromium Profile Server

Currently, the profile server creates one WebContents at startup:

```cpp
void ShellBrowserMainParts::InitializeMessageLoopContext() {
  Shell* shell = Shell::CreateNewWindow(...);
  video_consumer_ = std::make_unique<ShellVideoConsumer>();
  video_consumer_->ObserveContents(shell->web_contents());
}
```

For multi-tab support, the profile server needs to:

1. **Listen for incoming XPC connections** requesting new tabs.
2. **Create a new WebContents** (via `Shell::CreateNewWindow()`) for each
   connection.
3. **Create a new ShellVideoConsumer** per WebContents, each sending frames over
   its own XPC connection back to the compositor.

The existing single-tab startup path can remain as a default, but the server
must also accept "create tab" commands over XPC.

## What needs to be ported

### Box demo (`ts4/box-demo/` -> `ts5/box-demo/`)

The test page: a spinning blue square with a localStorage-based identity string
and FPS counter. Served by a Bun HTTP server on port 9407.

Files:

- `server.ts` — Bun HTTP server (12 lines)
- `public/index.html` — Test page (105 lines)

No changes needed — copy as-is.

### Swift compositor (`ts4/two-profiles-swift/` -> adapted for ts5)

The receiver app: an XPC Mach service that accepts IOSurface Mach ports from
Chromium Profile Server processes and composites them into a Metal window using
CADisplayLink.

Source files:

- `Package.swift` — SwiftPM manifest
- `Sources/Receiver/main.swift` — XPC listener, Metal pipeline, rendering (328
  lines)
- `Sources/Receiver/Shaders.metal` — Vertex + fragment shaders (33 lines)
- `com.termsurf.two-profiles-swift.plist` — Launchd agent definition

The two-pane compositor from ts4 must be adapted into three separate apps for
the three test cases.

## Three Swift apps

### `ts5/one-profile/` — One pane, one profile server

The simplest case. One Chromium Profile Server process with one WebContents
sends IOSurface frames to a Swift compositor that renders a single pane filling
the whole window.

- XPC service name: `com.termsurf.one-profile`
- Window: single pane (800x600)
- Profile servers: 1
- WebContents per server: 1
- Session IDs: `profile-a`

### `ts5/two-profiles/` — Two panes, two profile servers

Two Chromium Profile Server processes, each with a different `--user-data-dir`,
each hosting one WebContents. Two panes side by side. The two panes should show
different localStorage identities, proving profile isolation.

- XPC service name: `com.termsurf.two-profiles`
- Window: two panes side by side (1600x600)
- Profile servers: 2
- WebContents per server: 1
- Session IDs: `profile-a`, `profile-b`
- User data dirs: `~/.config/termsurf/poc/profile-a`,
  `~/.config/termsurf/poc/profile-b`

### `ts5/three-profiles/` — Three panes, two profile servers

The new test case. Two Chromium Profile Server processes: the first hosts
**two** WebContents (same profile, two tabs), the second hosts **one**
WebContents (different profile). Three panes in one window.

The two same-profile panes should show the **same** localStorage identity (they
share a `BrowserContext` with the same storage). The third pane should show a
**different** identity.

- XPC service name: `com.termsurf.three-profiles`
- Window: three panes side by side (2400x600)
- Profile servers: 2
- WebContents per server: 2 (profile-a), 1 (profile-b)
- Session IDs: `profile-a1`, `profile-a2`, `profile-b`
- User data dirs: `~/.config/termsurf/poc/profile-a`,
  `~/.config/termsurf/poc/profile-b`

## Success criteria

All three apps:

1. Build with `swift build` (+ `make` for Metal shaders).
2. Render at ~60fps sustained.
3. No Dock icon for Chromium Profile Server processes.
4. Correct profile isolation (same-profile panes share identity, different
   profiles have different identities).
5. For three-profiles: two panes from the same profile show the same
   localStorage identity, confirming they share the same `BrowserContext`.

## Experiments
