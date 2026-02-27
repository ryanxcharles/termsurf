# Issue 653: XPC Gateway Isolation

## Goal

The `web` TUI must connect to the correct TermSurf app — debug builds connect to
the debug app, installed release builds connect to the installed app. Currently
both builds share a single XPC gateway, so only one can work at a time.

## Problem

The XPC gateway (`com.termsurf.xpc-gateway`) is a singleton Mach service
registered with `launchd`. Both debug and release builds of TermSurf connect to
the same gateway and call `register_app` to store their endpoint. The gateway
stores exactly one endpoint (`appEndpoint` in
`xpc-gateway/Sources/main.swift:61`). Whichever app registers last wins — the
other app's `web` processes silently connect to the wrong app.

Additionally, the launchd plist (`gui/macos/com.termsurf.xpc-gateway.plist:15`)
points to a stale path:

```xml
<string>/Users/ryan/dev/termsurf/ghost/xpc-gateway/.build/debug/xpc-gateway</string>
```

The `ghost/` directory was renamed to `gui/` in Issue 613. The gateway binary is
now at `gui/xpc-gateway/.build/debug/xpc-gateway`.

## How the XPC gateway works

The architecture has three actors:

1. **XPC gateway daemon** — A tiny Swift process that owns the
   `com.termsurf.xpc-gateway` Mach service. Registered with `launchd` via a
   plist. It holds an app endpoint and brokers connections between the app and
   `web` TUI processes.

2. **TermSurf app** — On startup, connects to the gateway and calls
   `register_app` with an anonymous listener endpoint (`xpc.zig:153-158`). This
   is how the app makes itself discoverable.

3. **`web` TUI** — Connects to the gateway and calls `connect` to get the app's
   endpoint. Then connects directly to the app via the returned endpoint. All
   subsequent communication (set_overlay, navigate, mode_changed, etc.) goes
   over this direct connection.

The gateway is a rendezvous point. Once the `web` process has the app's
endpoint, it talks directly to the app — the gateway is no longer involved.

### Key files

| File                                       | Purpose                                         |
| ------------------------------------------ | ----------------------------------------------- |
| `gui/xpc-gateway/Sources/main.swift`       | Gateway daemon source                           |
| `gui/macos/com.termsurf.xpc-gateway.plist` | launchd plist                                   |
| `gui/src/apprt/xpc.zig:125-161`            | App-side: connect to gateway, register endpoint |
| `gui/src/apprt/xpc.zig:140`                | Mach service name: `com.termsurf.xpc-gateway`   |
| `gui/src/apprt/xpc.zig:776`                | Chromium server `--xpc-service` argument        |
| `tui/src/xpc.rs`                           | TUI-side: connect to gateway, get endpoint      |

## What needs to change

### 1. Separate Mach service names

Debug and release builds must use different Mach service names so they don't
interfere:

- **Release**: `com.termsurf.xpc-gateway` (unchanged)
- **Debug**: `com.termsurf.debug.xpc-gateway`

The service name is used in three places:

- `gui/src/apprt/xpc.zig:140` — app connects to gateway
- `gui/src/apprt/xpc.zig:776` — passed to Chromium server as `--xpc-service=`
- `gui/xpc-gateway/Sources/main.swift:9` — gateway listens on the service name

The Zig code can use `comptime` to select the service name based on
`builtin.mode`, the same pattern used for the debug data directory in Issue 650.

### 2. Fix the stale plist path

The plist at `gui/macos/com.termsurf.xpc-gateway.plist:15` needs the correct
path to the gateway binary. Since we now need two plists (one per service name),
we need:

- `gui/macos/com.termsurf.xpc-gateway.plist` — release gateway, points to the
  gateway binary inside the installed app bundle
  (`/Applications/TermSurf.app/Contents/MacOS/xpc-gateway`)
- `gui/macos/com.termsurf.debug.xpc-gateway.plist` — debug gateway, points to
  the dev build (`gui/xpc-gateway/.build/debug/xpc-gateway`)

### 3. Bundle the gateway binary

For the installed app, the gateway binary should be bundled inside
`TermSurf.app/Contents/MacOS/xpc-gateway` (or `Contents/Helpers/`). The release
plist points to this bundled copy. The install script copies the gateway binary
and loads the plist with `launchctl`.

### 4. TUI service name selection

The `web` TUI must know which gateway to connect to. It currently hardcodes the
service name. Options:

- **Environment variable**: The TermSurf app sets `TERMSURF_XPC_SERVICE` in the
  terminal environment. The `web` TUI reads it.
- **Compile-time**: Build the TUI with the service name baked in. But the TUI is
  a single binary used by both debug and release.
- **Convention**: The TUI checks `TERMSURF_XPC_SERVICE`, falling back to
  `com.termsurf.xpc-gateway` if unset. The debug app sets
  `TERMSURF_XPC_SERVICE=com.termsurf.debug.xpc-gateway` in its environment.

The environment variable approach is simplest — the app already controls the
terminal environment.
