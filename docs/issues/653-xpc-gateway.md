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

## Experiments

### Experiment 1: Isolate debug and release gateways

**Goal:** Debug and release builds each get their own XPC gateway so `web`
connects to the correct app regardless of which builds are running.

#### Changes

**1. `gui/xpc-gateway/Sources/main.swift`** — Accept the service name as a
command-line argument instead of hardcoding it. The gateway binary is shared
between debug and release — the plist controls which service name it listens on:

```swift
let serviceName: String
if CommandLine.arguments.count > 1 {
    serviceName = CommandLine.arguments[1]
} else {
    serviceName = "com.termsurf.xpc-gateway"
}
```

This replaces the hardcoded `let serviceName = "com.termsurf.xpc-gateway"` at
line 9. The dispatch queue label also changes to use the dynamic service name.

**2. `gui/src/apprt/xpc.zig:140`** — Use `comptime` to select the Mach service
name based on build mode:

```zig
const xpc_service_name = if (comptime builtin.mode == .Debug)
    "com.termsurf.debug.xpc-gateway"
else
    "com.termsurf.xpc-gateway";

gateway = xpc_connection_create_mach_service(xpc_service_name, null, 0);
```

Same pattern as the debug data directory in Issue 650.

**3. `gui/src/apprt/xpc.zig:776`** — Update the `--xpc-service=` argument passed
to the Chromium server to use the same `comptime` service name:

```zig
"--xpc-service=" ++ (if (comptime builtin.mode == .Debug)
    "com.termsurf.debug.xpc-gateway"
else
    "com.termsurf.xpc-gateway"),
```

Since the `xpc_arg` is already formatted with `bufPrintZ`, this can use
`comptime` string concatenation directly.

**4. `gui/src/apprt/xpc.zig` (init function)** — Set `TERMSURF_XPC_SERVICE` in
the process environment so child terminal sessions inherit it. Debug builds set
it to `com.termsurf.debug.xpc-gateway`; release builds don't set it (the TUI
falls back to the default). Use `std.posix.setenv`:

```zig
if (comptime builtin.mode == .Debug) {
    _ = std.posix.setenv("TERMSURF_XPC_SERVICE", "com.termsurf.debug.xpc-gateway", true);
}
```

This goes in the `init` function, after connecting to the gateway. Every
terminal session spawned by the debug app will inherit this variable.

**5. `tui/src/xpc.rs:79`** — Read `TERMSURF_XPC_SERVICE` from the environment,
falling back to `com.termsurf.xpc-gateway`:

```rust
let service_name = std::env::var("TERMSURF_XPC_SERVICE")
    .unwrap_or_else(|_| "com.termsurf.xpc-gateway".to_string());
let gateway_name = CString::new(service_name).unwrap();
```

When launched from a debug app terminal, the TUI connects to the debug gateway.
When launched from a release app terminal (or standalone), it connects to the
release gateway.

**6. `gui/macos/com.termsurf.xpc-gateway.plist`** — Fix the stale path and make
this the release plist. Points to the bundled gateway binary and passes the
release service name as an argument:

```xml
<key>ProgramArguments</key>
<array>
    <string>/Applications/TermSurf.app/Contents/MacOS/xpc-gateway</string>
    <string>com.termsurf.xpc-gateway</string>
</array>
```

**7. `gui/macos/com.termsurf.debug.xpc-gateway.plist`** — New plist for the
debug gateway. Points to the dev build binary and passes the debug service name:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.termsurf.debug.xpc-gateway</string>
    <key>MachServices</key>
    <dict>
        <key>com.termsurf.debug.xpc-gateway</key>
        <true/>
    </dict>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/ryan/dev/termsurf/gui/xpc-gateway/.build/debug/xpc-gateway</string>
        <string>com.termsurf.debug.xpc-gateway</string>
    </array>
    <key>StandardOutPath</key>
    <string>/Users/ryan/dev/termsurf/logs/xpc-gateway-debug.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/ryan/dev/termsurf/logs/xpc-gateway-debug.log</string>
</dict>
</plist>
```

**8. `install.sh`** — Bundle the gateway binary and load the release plist:

```bash
# Bundle xpc-gateway.
echo "==> Bundling xpc-gateway..."
cp "$REPO_DIR/gui/xpc-gateway/.build/debug/xpc-gateway" "$APP/Contents/MacOS/xpc-gateway"

# Load release xpc-gateway plist.
echo "==> Loading xpc-gateway launchd service..."
launchctl bootout "gui/$(id -u)/com.termsurf.xpc-gateway" 2>/dev/null || true
cp "$REPO_DIR/gui/macos/com.termsurf.xpc-gateway.plist" \
  "$HOME/Library/LaunchAgents/com.termsurf.xpc-gateway.plist"
launchctl bootstrap "gui/$(id -u)" \
  "$HOME/Library/LaunchAgents/com.termsurf.xpc-gateway.plist"
```

#### Verification

1. **Build the gateway**: `cd gui/xpc-gateway && swift build`. Verify it accepts
   a service name argument: `.build/debug/xpc-gateway com.termsurf.test` should
   print `Listening on com.termsurf.test`.
2. **Load debug plist**: Copy `com.termsurf.debug.xpc-gateway.plist` to
   `~/Library/LaunchAgents/` and `launchctl bootstrap gui/$(id -u)`. Verify
   `launchctl print gui/$(id -u)/com.termsurf.debug.xpc-gateway` shows the
   service running.
3. **Debug build**: `cd gui && zig build`. Launch the debug app. Open a terminal
   pane and run `echo $TERMSURF_XPC_SERVICE` — should print
   `com.termsurf.debug.xpc-gateway`.
4. **Debug `web`**: In the debug app's terminal, run `web https://google.com`.
   It should connect to the debug gateway and render the page.
5. **Install release**: Run `install.sh`. Launch the installed release app. Run
   `echo $TERMSURF_XPC_SERVICE` — should be empty (unset). Run
   `web https://google.com` — should connect to the release gateway and render.
6. **Both simultaneously**: Launch both the debug app and the installed release
   app. Run `web` in each. Both should connect to their respective gateways and
   render independently.

**Result: Partial.** All code changes compile and the service name isolation is
correct — debug and release use separate Mach service names throughout the stack
(gateway daemon, app, Chromium server, TUI). However, the experiment still
requires manual `launchctl` commands to register the plists. The app should
launch the gateway automatically without any manual setup.

#### What worked

- Gateway accepts service name as CLI argument (change 1)
- `comptime` service name selection in xpc.zig (changes 2–3)
- `TERMSURF_XPC_SERVICE` environment variable for the TUI (changes 4–5)
- Separate plists with correct paths and service names (changes 6–7)
- All three components build cleanly (gateway, GUI, TUI)

#### What didn't work

The experiment requires manually loading plists via `launchctl bootstrap`. This
is a regression from the ts5 era where `SMAppService` handled registration
automatically — the app called `SMAppService.agent(plistName:).register()` on
startup, and launchd auto-started the gateway on demand. That approach was never
ported from ts5 to gui/.

The `launchctl` commands in `install.sh` (change 8) and the verification steps
requiring manual plist loading are wrong. The app must be self-contained — just
`open "TermSurf Debug.app"` should work with zero prior setup.

#### Next step

Experiment 2 should port the SMAppService auto-registration from ts5 (Issue 506
Experiment 2) to gui/. The gateway binary and plist must be bundled inside the
app during `zig build`, and the app must call `SMAppService.register()` on
startup before connecting to the gateway.
