# Experiment 7: Start the TermSurf GUI Socket

## Description

Experiment 6 established the minimal app identity baseline. The next protocol
step should be deliberately small: start a TermSurf GUI Unix socket when the
macOS app launches, publish its path through `TERMSURF_SOCKET`, accept a client
connection, and clean the socket up when the app exits.

This experiment should not parse protobuf messages, launch Roamium, render
overlays, forward input, or modify `webtui`/`roamium`. It only proves that the
new Ghostboard-derived TermSurf app can expose the first protocol surface that
TUIs need to discover the GUI.

Reference implementations:

- Wezboard starts its socket in `wezboard/wezboard-gui/src/main.rs` and
  `wezboard/wezboard-gui/src/termsurf/listener.rs`.
- Ghostboard Legacy starts its socket in `ghostboard-legacy/src/apprt/xpc.zig`,
  including `TERMSURF_SOCKET` propagation.

## Changes

- `ghostboard/src/apprt/termsurf.zig` or a similarly scoped new Zig module —
  implement a minimal TermSurf listener lifecycle:
  - choose a PID-scoped Unix socket path under `$TMPDIR/termsurf/`;
  - use the app identity in the socket name, for example
    `termsurf-ghostboard-{pid}.sock` or `termsurf-{pid}.sock`;
  - create the parent directory;
  - remove any stale socket at that exact path before binding;
  - bind and listen on an `AF_UNIX` stream socket;
  - set `TERMSURF_SOCKET` to the bound path only after listen succeeds;
  - accept client connections and close them immediately or keep them tracked
    only long enough to log the accepted connection;
  - expose a cleanup path that closes the listener, closes any accepted fds,
    unsets or leaves `TERMSURF_SOCKET` harmlessly process-local, and unlinks the
    socket file.
- `ghostboard/src/main_c.zig` or another existing C API boundary — export small
  C-callable functions such as `termsurf_ipc_start()` and `termsurf_ipc_stop()`
  for the macOS Swift app. Keep existing Ghostty C ABI names unchanged.
- `ghostboard/include/ghostty.h` or the generated/checked-in header surface used
  by Swift — declare only the new exported functions needed by Swift.
- `ghostboard/macos/Sources/App/macOS/ghostty-bridging-header.h` — include the
  declaration if the new exported functions are not already visible through the
  imported GhosttyKit header.
- `ghostboard/macos/Sources/App/macOS/AppDelegate.swift` — call the start hook
  during launch after Ghostty initialization is ready but before any terminal
  surface is created, and call the stop hook during application termination. Log
  failures without preventing normal terminal startup.
- Issue docs — record the result and update the experiment index.

This experiment intentionally does not:

- add generated protobuf code;
- dispatch any TermSurf message;
- spawn or supervise browser engine processes;
- add browser overlay views;
- change keyboard or mouse input behavior;
- change `webtui` or `roamium`;
- install or emit a standalone CLI;
- rename internal Ghostty modules beyond the minimal names needed for this new
  TermSurf-specific module/API.

## Verification

1. Run Zig formatting on edited Zig files.
2. If Swift files are edited, run SwiftLint:

   ```bash
   cd ghostboard
   swiftlint lint --strict --fix
   swiftlint lint --strict
   ```

3. Format edited markdown.
4. Build the native GhosttyKit framework:

   ```bash
   cd ghostboard
   zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false
   ```

5. Build the macOS app:

   ```bash
   cd ghostboard
   macos/build.nu --scheme Ghostty --configuration Debug --action build
   ```

6. Launch `TermSurf.app` with a temporary logging environment or deterministic
   debug output that records the bound socket path.
7. Verify the socket path:
   - is under `$TMPDIR/termsurf/`;
   - contains the running app PID or is otherwise unique per process;
   - exists as a Unix domain socket while the app is running;
   - is exported to the app process environment as `TERMSURF_SOCKET`.
8. Verify environment propagation into an actual terminal session created by
   `TermSurf.app` after listener startup:
   - use the initial automatically opened window, or launch the app with a
     deterministic command that prints `TERMSURF_SOCKET` to a temporary file;
   - confirm the terminal child sees the same bound socket path that the app
     logged;
   - perform this check before relying on any external process connection test.
9. Connect to the socket without using `webtui` yet, preferably from the
   terminal session's inherited `TERMSURF_SOCKET` value:

   ```bash
   python3 -c 'import os, socket; s=socket.socket(socket.AF_UNIX); s.connect(os.environ["TERMSURF_SOCKET"]); s.close()'
   ```

   If automation cannot run the Python client inside the terminal session in
   this experiment, record the terminal child's inherited `TERMSURF_SOCKET`
   value, compare it to the logged bound path, and use that exact value for an
   external client connection test.

10. Confirm the app logs an accepted TermSurf client connection and remains
    running after that client disconnects.
11. Terminate the app and verify the socket file is removed.
12. Confirm the diff did not touch protocol dispatch, browser launch, overlay,
    input forwarding, `webtui`, or `roamium` code.

Pass criteria:

- The app still builds and launches as `TermSurf.app`.
- The GUI starts a PID-scoped TermSurf Unix socket under `$TMPDIR/termsurf/`.
- `TERMSURF_SOCKET` is set inside the app process only after the listener is
  bound.
- The initial terminal session created by `TermSurf.app` inherits
  `TERMSURF_SOCKET` with the same bound socket path.
- A separate local client can connect to the socket and disconnect without
  crashing or hanging the app.
- The accepted connection is logged.
- The socket file is removed on app shutdown.
- No protobuf dispatch, browser launch, overlay rendering, input forwarding,
  `webtui`, or `roamium` changes are made.

Fail criteria:

- The socket is not created or cannot accept a local client.
- `TERMSURF_SOCKET` points at a missing or unbound path.
- The app crashes, hangs, or blocks terminal startup when the listener starts.
- The socket path is not process-unique.
- The socket file is left behind after normal app shutdown.
- The experiment expands into message handling, browser process management, or
  UI overlay behavior.

## Notes

If this experiment passes, the next experiment can add protobuf framing and a
single read-only handshake/diagnostic message. That should happen before any
browser process or overlay lifecycle work so socket correctness remains easy to
debug.

## Design Review

Fresh-context adversarial review initially returned `CHANGES REQUIRED`.

Required finding accepted and fixed:

- The verification did not prove `TERMSURF_SOCKET` propagation into terminal
  sessions. The design now requires a terminal child created by `TermSurf.app`
  to see the same bound socket path that the app logged, and it makes that
  inheritance a pass criterion.

Optional finding accepted and fixed:

- Listener start timing was underspecified for environment inheritance. The
  design now requires the listener to start before any terminal surface is
  created and names the initial automatically opened window in verification.

Re-review returned `APPROVED` with no remaining required findings.

## Result

**Result:** Pass

Implemented a minimal TermSurf GUI socket for the macOS app:

- `ghostboard/src/apprt/termsurf.zig` starts a PID-scoped Unix socket under
  `$TMPDIR/termsurf/`, sets `TERMSURF_SOCKET` only after `listen()` succeeds,
  accepts local clients, and removes the socket during shutdown.
- `ghostboard/src/main_c.zig` exports `termsurf_ipc_start()` and
  `termsurf_ipc_stop()` through the existing C ABI boundary.
- `ghostboard/include/ghostty.h` declares the two new C-callable hooks.
- `ghostboard/macos/Sources/App/macOS/AppDelegate.swift` starts the listener in
  `applicationWillFinishLaunching`, before the first terminal window can be
  created, and stops it on application termination. It also registers a
  deterministic SIGTERM shutdown path for automation.

Two runtime failures shaped the final placement:

- Starting the listener in `applicationDidFinishLaunching` was too late. The
  first terminal command could start from `applicationDidBecomeActive` before
  the socket existed, so the child inherited an empty `TERMSURF_SOCKET`.
- A main-queue SIGTERM `DispatchSource` did not fire reliably in this VM launch
  path. Moving SIGTERM handling to a global queue and exiting after
  `termsurf_ipc_stop()` made automated cleanup deterministic.

Verification performed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig` passed.
- `swiftlint lint --strict --fix` and `swiftlint lint --strict` passed with zero
  violations.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp7-zig-native-xcframework-20260616-final-command.log`.
- macOS app build passed:
  `logs/ghostboard-exp7-macos-build-debug-20260616-after-global-sigterm.log`.
- Runtime verification passed:
  `logs/ghostboard-exp7-runtime-harness-20260616-084855.log`. The app's stderr
  log for that run is `logs/ghostboard-exp7-runtime-app-20260616-084855.log`.

The successful runtime check launched:

```bash
GHOSTTY_CONFIG_PATH="$config" GHOSTTY_LOG=stderr \
  ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf
```

with a temporary `initial-command` shell script inside the first terminal
session. That child wrote its inherited `TERMSURF_SOCKET`, connected to that
same socket, then slept while the test terminated the app with SIGTERM.

Observed successful runtime output:

```text
runtime verification passed
app pid: 47390
child env: /var/folders/vx/wbmx10nd7tx8259xgg3v4vf80000gn/T/termsurf/termsurf-ghostboard-47390.sock
child connect: /var/folders/vx/wbmx10nd7tx8259xgg3v4vf80000gn/T/termsurf/termsurf-ghostboard-47390.sock
PASS: child env/connect paths match
PASS: socket path is under TMPDIR/termsurf and has expected name
PASS: socket path includes app pid
PASS: socket exists while app is running
PASS: app log contains listener line
PASS: app log contains accepted client line
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
info(termsurf): TermSurf socket listening on /var/folders/vx/wbmx10nd7tx8259xgg3v4vf80000gn/T/termsurf/termsurf-ghostboard-47390.sock
info(termsurf): TermSurf client connected fd=13
```

After SIGTERM, the app process was gone and the socket file no longer existed.
Follow-up checks found no leftover `TermSurf.app/Contents/MacOS/termsurf`
processes and no stale `termsurf-ghostboard-*.sock` files.

Scope check:

- No protobuf dispatch was added.
- No browser engine launch or supervision was added.
- No overlay rendering was added.
- No keyboard or mouse forwarding was changed.
- `webtui` and `roamium` were not modified.
- No standalone CLI install or emit behavior was changed.

## Result Review

Fresh-context adversarial result review initially returned `CHANGES REQUIRED`.

Required finding accepted and fixed:

- The experiment cited the app stderr log as runtime proof, but that log only
  contained the listener and accepted-client lines. It did not preserve the
  harness assertions proving child environment inheritance, matching child
  connection path, process exit, and socket cleanup. I reran runtime
  verification with the harness output captured in
  `logs/ghostboard-exp7-runtime-harness-20260616-084855.log` and updated this
  result to cite that evidence.

Optional finding accepted and fixed:

- The original native framework build log was empty because the command
  succeeded silently. I reran it with the command and exit status captured in
  `logs/ghostboard-exp7-zig-native-xcframework-20260616-final-command.log`.

Re-review returned `APPROVED`. The reviewer confirmed the required evidence gap
was resolved by `logs/ghostboard-exp7-runtime-harness-20260616-084855.log`, the
native framework build evidence was resolved by
`logs/ghostboard-exp7-zig-native-xcframework-20260616-final-command.log`, no new
required findings were introduced, `git diff --check` was clean, and the result
commit had not yet been made.

## Conclusion

Ghostboard now exposes the first required TermSurf GUI discovery surface:
terminal sessions launched by `TermSurf.app` inherit a live `TERMSURF_SOCKET`
that accepts local client connections and is cleaned up on controlled shutdown.

The next experiment can build on this by adding protobuf framing and a minimal
diagnostic/handshake read path, still without launching Roamium or rendering
browser overlays.
