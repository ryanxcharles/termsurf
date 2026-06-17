# Experiment 21: Send BrowserReady After TabReady

## Description

Experiment 20 gave Ghostboard a real browser listen socket path when it spawns
an absolute browser executable for `SetOverlay`. Experiments 15 and 16 already
send `CreateTab` to a browser connection and record `TabReady`. The next parity
step is to notify the originating TUI that its browser tab is ready.

Wezboard sends `BrowserReady` after `TabReady` for a normal browser pane. The
message contains:

- `pane_id` from `TabReady`;
- `tab_id` from `TabReady`;
- `browser_socket` from the matched server's listen socket;
- `browser` from the pane's browser spec.

This experiment will implement that same state-backed notification in
Ghostboard. It must use the real listen socket stored in Experiment 20; it must
not fabricate a socket path, use the GUI socket as the browser socket, or send
`BrowserReady` before `TabReady`.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - extend `PaneState` with the originating TUI fd from the `SetOverlay`
    connection;
  - pass the current client fd into `handleSetOverlay`;
  - preserve that TUI fd when updating an existing pane;
  - after `TabReady` records `tab_id` and lookup state, find the pane's server;
  - if the server has a nonempty listen socket and the pane has a live TUI fd,
    snapshot the pane id, tab id, browser, listen socket, and TUI fd under
    `state_mutex`;
  - release `state_mutex` before writing the length-prefixed `BrowserReady`
    protobuf to the TUI fd;
  - log the successful `BrowserReady` send with pane id, tab id, socket, and
    browser;
  - leave browser direct-client routing, overlay presentation, navigation, and
    input forwarding out of scope.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, CLI install
behavior, direct browser-client routing, CALayerHost overlay presentation,
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
  sends `SetOverlay(browser=/absolute/temp/helper, profile=default)` from a TUI
  socket.
- The spawned helper connects back with `ServerRegister`, receives `CreateTab`,
  and sends `TabReady(pane-a, 42)`.
- The TUI socket receives `BrowserReady` after `TabReady`.
- The decoded `BrowserReady` has:
  - `pane_id = "pane-a"`;
  - `tab_id = 42`;
  - `browser_socket` equal to the `--listen-socket` argument passed to the
    helper;
  - `browser` equal to the absolute helper browser spec used by `SetOverlay`.
- App logs include
  `BrowserReady: pane_id=pane-a tab_id=42 socket=... browser=...`.
- The runtime harness also sends a normal TUI `HelloRequest` on a fresh socket
  and receives `HelloReply`, proving existing request/reply behavior still
  works.
- The harness verifies shutdown cleanup still removes the socket file and leaves
  no stale `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- `BrowserReady` is sent before `TabReady`.
- `BrowserReady.browser_socket` is empty, fabricated, or equal to the GUI
  socket.
- `BrowserReady` is sent to the browser socket instead of the originating TUI
  socket.
- `BrowserReady` contains the wrong pane id, tab id, browser socket, or browser
  value.
- Existing `SetOverlay -> spawn -> ServerRegister -> CreateTab -> TabReady`
  behavior regresses.
- The implementation adds direct browser-client routing, CALayerHost overlay
  presentation, navigation forwarding, input forwarding, or changes `webtui`,
  `roamium`, or the protocol schema in this experiment.

## Design Review

Fresh-context adversarial design review returned **APPROVED** with no required
findings.

Optional finding accepted and fixed: the design now requires snapshotting the
`BrowserReady` fields under `state_mutex`, then releasing the lock before
writing to the TUI fd.

Nit accepted and fixed: the expected app log check now includes the browser
value because the design requires logging it.

## Result

**Result:** Pass

Implemented state-backed `BrowserReady` delivery in
`ghostboard/src/apprt/termsurf.zig`.

The implementation now:

- records the originating TUI fd on pane state when handling `SetOverlay`;
- preserves the TUI fd when updating an existing pane;
- clears matching pane TUI fds when a TUI client disconnects, preventing
  `BrowserReady` from being sent to a stale or reused fd;
- after `TabReady`, snapshots pane id, tab id, browser spec, server listen
  socket, and TUI fd under `state_mutex`;
- releases `state_mutex` before writing `BrowserReady` to the TUI fd;
- sends `BrowserReady` only when the pane has a TUI fd and the matched server
  has a nonempty real listen socket.

Verification passed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp21-zig-native-xcframework-20260616-113218-after-review.log`.
- macOS app build passed:
  `logs/ghostboard-exp21-macos-build-debug-20260616-113240-after-review.log`.
- Runtime harness passed:
  `logs/ghostboard-exp21-runtime-harness-20260616-113407-after-review.log`.
- Initial runtime app log:
  `logs/ghostboard-exp21-runtime-app-20260616-112936.log`.
- Post-review runtime app log:
  `logs/ghostboard-exp21-runtime-app-20260616-113407-after-review.log`.
- `git diff --check` passed.

Observed successful runtime checks:

```text
PASS: helper path is longer than old 64-byte browser limit
PASS: child wrote TERMSURF_SOCKET
PASS: socket path is under TMPDIR/termsurf
PASS: socket exists while app is running
PASS: TUI socket received BrowserReady with real listen socket
PASS: BrowserReady browser_socket is not GUI socket
PASS: stale helper received CreateTab
PASS: stale helper sent TabReady after TUI disconnect
PASS: fresh TUI client received HelloReply
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale normal helper process remains
PASS: no stale stale helper process remains
PASS: no stale TermSurf process remains
PASS: app log contains normal BrowserReady
PASS: no BrowserReady emitted for disconnected TUI
PASS: no CaContext emitted
PASS: no overlay presentation message emitted
runtime verification passed
```

The runtime harness decoded `BrowserReady` on the original TUI socket after
`TabReady` and verified:

- `pane_id = "pane-a"`;
- `tab_id = 42`;
- `browser_socket` equals the helper's `--listen-socket` argument;
- `browser_socket` is not the GUI socket;
- `browser` equals the absolute helper browser spec used by `SetOverlay`.

The post-review harness also verified a stale-fd case: a second TUI socket sent
`SetOverlay`, disconnected before the helper sent `TabReady`, and the app log
did not contain `BrowserReady: pane_id=pane-stale`.

## Conclusion

Ghostboard now completes the first end-to-end browser readiness notification:
`SetOverlay -> spawn browser -> ServerRegister -> CreateTab -> TabReady -> BrowserReady`.
This gives `webtui` the real browser listen socket it needs to establish its
direct browser connection, while direct browser-client routing, CALayerHost
overlay presentation, navigation forwarding, and input forwarding remain for
later experiments.

## Result Review

Fresh-context adversarial result review initially returned **CHANGES REQUIRED**
with one required finding: `BrowserReady` could be sent to a stale or reused TUI
fd because pane state did not clear `tui_fd` when the originating TUI client
exited.

Required finding accepted and fixed: `handleClient` now clears matching pane
`tui_fd` values when a TUI connection exits. The post-review runtime harness
added a stale-fd case where a TUI sends `SetOverlay`, disconnects before
`TabReady`, and the app does not emit `BrowserReady` for that pane.

Fresh-context adversarial re-review returned **APPROVED**. The reviewer
confirmed the stale/reused fd finding is resolved because TUI client teardown
now clears matching pane `tui_fd` values before fd close, `snapshotBrowserReady`
refuses panes without a live TUI fd, and the post-review runtime harness proves
no `BrowserReady` is emitted for the disconnected `pane-stale` case. No new
required findings were introduced by the fix.
