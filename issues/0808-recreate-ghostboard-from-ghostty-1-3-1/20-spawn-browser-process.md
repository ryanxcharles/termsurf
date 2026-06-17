# Experiment 20: Spawn Browser Process For SetOverlay

## Description

Experiments 14 through 19 implemented the GUI-side state and browser-socket
registration path when a browser client already connects to Ghostboard. The next
parity step is for Ghostboard to launch the browser process itself when a new
`SetOverlay` needs a browser server.

Wezboard's `SetOverlay` path creates a server record, spawns the browser with
the GUI IPC socket and a browser listen socket, then waits for the spawned
browser to connect back with `ServerRegister`. This experiment will implement
the first launch-mechanics slice of that behavior in Ghostboard:

- resolve an absolute `SetOverlay.browser` value as the browser executable;
- widen Ghostboard's stored browser spec buffers so absolute executable paths
  fit in pane, server, and tab lookup state;
- create a deterministic per-server listen socket path;
- spawn the process with TermSurf-compatible arguments: `--ipc-socket`,
  `--user-data-dir`, `--listen-socket`, `--hidden`, `--no-sandbox`,
  `--enable-logging`, and `--log-file`;
- store the listen socket path and child process id in the server state;
- keep the existing `ServerRegister -> CreateTab -> TabReady` path unchanged.

This experiment intentionally does not implement installed-name browser
resolution for `roamium`, incognito-specific launch arguments, persistent
process supervision, child reap on every exit path, `BrowserReady`, browser
direct-client routing, CALayerHost overlay presentation, navigation forwarding,
or input forwarding. It also does not modify `webtui` or `roamium`.

The runtime harness will use an absolute temporary helper executable as the
browser path. That helper will record its argv and connect back to the GUI
socket with `ServerRegister`, so the experiment proves the spawn arguments and
the existing browser registration flow without depending on a full Chromium
startup.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - widen the fixed browser storage used by `PaneState`, `ServerState`, and
    `TabLookupState` from the current short browser-name limit to a path-capable
    limit, so absolute executable paths can be stored and compared safely;
  - extend `ServerState` with a bounded `listen_socket` buffer and a child
    process id field;
  - when `SetOverlay` creates a new server and the browser value is an absolute
    path, spawn that executable with the current GUI socket path and a generated
    listen socket path;
  - build `--user-data-dir` under
    `$XDG_DATA_HOME/termsurf/chromium-profiles/{profile}`, or
    `$HOME/.local/share/termsurf/chromium-profiles/{profile}` when
    `XDG_DATA_HOME` is unset;
  - build `--log-file` under `$XDG_STATE_HOME/termsurf/chromium-server.log`, or
    `$HOME/.local/state/termsurf/chromium-server.log` when `XDG_STATE_HOME` is
    unset;
  - keep named browser resolution (`browser = "roamium"`) and
    `profile = "incognito"` launch argument parity as explicit
    not-yet-implemented paths for later experiments;
  - do not send `BrowserReady` in this experiment.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, CLI install
behavior, `BrowserReady`, direct browser-client routing, overlay presentation,
navigation forwarding, or input forwarding.

## Verification

Pass criteria:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passes inside `ghostboard/`.
- The native GhosttyKit framework build passes:
  `zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`.
- The macOS app build passes:
  `macos/build.nu --scheme Ghostty --configuration Debug --action build`.
- Runtime harness launches `TermSurf.app`, connects to `TERMSURF_SOCKET`, and
  sends `SetOverlay(browser=/absolute/temp/helper, profile=default)`.
- The helper executable is spawned and records argv proving:
  - the absolute helper path is accepted even when longer than the old 64-byte
    browser-name limit;
  - `--ipc-socket` equals the GUI socket path inherited by the terminal child;
  - `--listen-socket` is nonempty and under `$TMPDIR/termsurf/`;
  - `--user-data-dir` ends in `termsurf/chromium-profiles/default`;
  - `--hidden`, `--no-sandbox`, `--enable-logging`, and `--log-file=...` are
    present.
- The helper connects back to the GUI socket and sends `ServerRegister`; the
  harness receives the resulting `CreateTab` frame for the pending pane.
- App logs include the browser spawn message, `ServerRegister: matched server`,
  and `sent CreateTab`.
- Existing synchronous request/reply behavior still works by sending a fresh
  `HelloRequest` and receiving `HelloReply`.
- The harness verifies shutdown cleanup still removes the socket file and leaves
  no stale `TermSurf.app/Contents/MacOS/termsurf` process.
- The harness verifies the temporary helper process exits and does not remain
  running.
- `git diff --check` is clean.

Fail criteria:

- `SetOverlay` creates a pending server but does not attempt to spawn the
  absolute browser path.
- Absolute browser paths longer than the old short browser-name limit are
  rejected or truncated.
- The spawned process is missing required TermSurf arguments.
- The spawned helper cannot connect back and complete
  `ServerRegister -> CreateTab`.
- Existing manually connected browser `ServerRegister -> CreateTab` behavior
  regresses.
- The implementation sends `BrowserReady`, launches overlay UI, forwards
  navigation or input, or changes direct browser-client routing in this
  experiment.
- Any `webtui`, `roamium`, protocol schema, app branding, config path, icon, or
  CLI install behavior changes are needed for this experiment.

## Design Review

Fresh-context adversarial design review initially returned **CHANGES REQUIRED**
with one required finding: the plan used an absolute temporary helper path even
though Ghostboard's current browser storage is capped at the short browser-name
limit.

Required finding accepted and fixed: the design now requires widening the
browser spec buffers used by pane, server, and tab lookup state before spawning
absolute browser paths, and verification now checks a path longer than the old
64-byte limit.

The reviewer also noted that Wezboard adds `--incognito` when the profile is
`incognito`. That optional finding was addressed by explicitly deferring
incognito-specific launch argument parity to a later experiment.

Fresh-context adversarial re-review returned **APPROVED**. The reviewer
confirmed the required finding was resolved, the incognito omission is now
explicitly deferred, and the fixes introduced no new required issues.

## Result

**Result:** Pass

Implemented browser process launch mechanics for new `SetOverlay` servers in
`ghostboard/src/apprt/termsurf.zig`.

The implementation now:

- widens stored browser specs so absolute executable paths fit in pane, server,
  and tab lookup state;
- records a generated per-server listen socket path on `ServerState`;
- stores the spawned child pid on `ServerState`;
- launches absolute browser paths with `--ipc-socket`, `--user-data-dir`,
  `--listen-socket`, `--hidden`, `--no-sandbox`, `--enable-logging`, and
  `--log-file`;
- keeps named browser resolution, incognito launch arguments, `BrowserReady`,
  direct browser-client routing, overlay presentation, navigation, and input
  forwarding out of scope.

Verification passed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp20-zig-native-xcframework-20260616-111912.log`.
- macOS app build passed:
  `logs/ghostboard-exp20-macos-build-debug-20260616-111932.log`.
- Runtime harness passed:
  `logs/ghostboard-exp20-runtime-harness-20260616-112056.log`.
- Runtime app log: `logs/ghostboard-exp20-runtime-app-20260616-112056.log`.
- `git diff --check` passed.

Observed successful runtime checks:

```text
PASS: helper path is longer than old 64-byte browser limit
PASS: child wrote TERMSURF_SOCKET
PASS: socket path is under TMPDIR/termsurf
PASS: socket exists while app is running
PASS: helper spawned and exited
PASS: helper argv[0] is absolute helper path
PASS: helper received GUI ipc socket
PASS: helper received generated listen socket
PASS: helper received profile user-data-dir
PASS: helper received --hidden
PASS: helper received --no-sandbox
PASS: helper received --enable-logging
PASS: helper received log-file
PASS: helper received CreateTab for pending pane
PASS: fresh TUI client received HelloReply
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale TermSurf process remains
PASS: no stale helper process remains
PASS: app log contains browser spawn
PASS: app log contains matched ServerRegister
PASS: app log contains sent CreateTab
PASS: no BrowserReady emitted
PASS: no CaContext emitted
PASS: no overlay presentation message emitted
runtime verification passed
```

The runtime harness used an absolute helper path with length 196, proving the
old short browser-name limit no longer blocks absolute executable paths.

## Conclusion

Ghostboard can now launch an absolute browser executable for a new overlay
server and complete the existing browser callback path:
`SetOverlay -> spawn process -> ServerRegister -> CreateTab`. This moves the
port from manually simulated browser connections toward real Roamium launch
without changing `webtui`, `roamium`, or the protocol schema.

## Result Review

Fresh-context adversarial result review returned **APPROVED** with no findings.

The reviewer confirmed:

- the result commit had not been made before review;
- the changed files are scoped to `ghostboard/src/apprt/termsurf.zig` and the
  Issue 808 experiment docs;
- `zig fmt --check src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed;
- the logs substantiate the key runtime claim: a long absolute helper path was
  spawned, expected TermSurf args were received, `ServerRegister -> CreateTab`
  completed, and no `BrowserReady`, `CaContext`, or overlay presentation was
  emitted.
