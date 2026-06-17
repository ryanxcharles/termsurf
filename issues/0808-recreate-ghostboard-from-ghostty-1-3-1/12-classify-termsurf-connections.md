# Experiment 12: Classify TermSurf Connections

## Description

Experiments 8 through 11 implemented the synchronous TUI request/reply paths
that can run before browser launch or overlay creation. The next protocol step
is to distinguish TUI clients from browser-engine clients on the GUI socket.

Wezboard classifies each connection by its first decoded message:

- first message is `ServerRegister` -> browser-engine connection;
- any other first message -> TUI connection.

Ghostboard currently handles every accepted socket identically. Before
implementing server registry, browser process launch, `ServerRegister`, and
`TabReady` lifecycle behavior, this experiment will add the same first-message
classification and logging in Ghostboard's socket handler.

This is intentionally a behavioral foundation only. It should not yet create a
server registry, launch Roamium, send `CreateTab`, send `BrowserReady`, track
panes, or change the current no-state query replies.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - add a small connection-type enum for `unknown`, `tui`, and `browser`;
  - track the connection type inside each `handleClient` loop;
  - classify the connection when the first valid `TermSurfMessage` is decoded;
  - classify `ServerRegister` as browser, and all other first messages as TUI;
  - log the selected connection type with the accepted socket file descriptor so
    verification can prove one socket is classified exactly once;
  - keep existing request/reply behavior unchanged.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, or CLI install
behavior.

## Verification

Pass criteria:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passes inside `ghostboard/`.
- The native GhosttyKit framework build passes.
- The macOS app build passes.
- Runtime harness launches `TermSurf.app`, connects to `TERMSURF_SOCKET`, and
  proves:
  - a client whose first message is `HelloRequest` receives `HelloReply` and
    logs `TermSurf connection type=Tui`;
  - a client whose first message is `QueryTabsRequest` receives `QueryTabsReply`
    and logs `TermSurf connection type=Tui`;
  - a client whose first message is `ServerRegister` logs
    `TermSurf connection type=Browser` and
    `TermSurf message ignored type=ServerRegister`;
  - on one socket, a client that sends `HelloRequest` and then `ServerRegister`
    receives the `HelloReply`, logs exactly one classification for that socket
    as TUI, and does not later log a browser classification for that same
    socket;
  - on one socket, a client that sends `ServerRegister` and then `HelloRequest`
    logs exactly one classification for that socket as browser, does not later
    log a TUI classification for that same socket, and keeps the existing
    message dispatch behavior unchanged;
  - a fresh TUI client after the browser-classified client still receives a
    valid `HelloReply`.
- The harness verifies shutdown cleanup still removes the socket file and leaves
  no stale `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- Browser/TUI classification changes the reply behavior from Experiments 8
  through 11.
- `ServerRegister` is classified as TUI.
- The classification is recalculated after later messages on the same connection
  instead of being set by the first decoded message.
- This experiment starts implementing server registry, browser launch,
  `CreateTab`, `BrowserReady`, pane tracking, or overlay behavior.
- Any `webtui`, `roamium`, protocol schema, app branding, config path, icon, or
  CLI install behavior changes are needed for this experiment.

## Design Review

A fresh-context adversarial design review returned **CHANGES REQUIRED**.

Required finding accepted and fixed: the original verification only tested
separate client sockets whose first messages were `HelloRequest`,
`QueryTabsRequest`, and `ServerRegister`. That would not prove classification
was sticky after the first decoded message on a single connection. The plan now
requires same-socket TUI-then-browser and browser-then-TUI message sequences and
requires connection-type logs to include the socket file descriptor, so the
harness can prove each accepted socket is classified exactly once.

Fresh-context adversarial re-review approved the updated design. The reviewer
confirmed the prior required finding was resolved by the same-socket mixed
message pass criteria and found no remaining issues.

## Result

**Result:** Pass

Implemented first-message TermSurf connection classification in
`ghostboard/src/apprt/termsurf.zig`.

The socket handler now:

- tracks a per-client `ConnType` initialized to `unknown`;
- classifies the connection after the first decoded `TermSurfMessage`;
- classifies `ServerRegister` as `Browser`;
- classifies every other first message as `Tui`;
- logs the selected type as `TermSurf connection type={type} fd={fd}`;
- leaves the existing message dispatch behavior unchanged.

Verification passed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp12-zig-native-xcframework-20260616-095924.log`.
- macOS app build passed:
  `logs/ghostboard-exp12-macos-build-debug-20260616-095938.log`.
- Runtime harness passed:
  `logs/ghostboard-exp12-runtime-harness-20260616-100207.log`.
- Runtime app log: `logs/ghostboard-exp12-runtime-app-20260616-100207.log`.
- `git diff --check` passed.

The runtime harness launched the debug `TermSurf.app` binary with a temporary
`GHOSTTY_CONFIG_PATH` and `GHOSTTY_LOG=stderr`, read the inherited
`TERMSURF_SOCKET` from the terminal child, and connected directly to the GUI
socket with length-prefixed current-schema protobuf messages.

Observed successful runtime checks:

```text
PASS: child wrote TERMSURF_SOCKET
PASS: socket path is under TMPDIR/termsurf
PASS: socket exists while app is running
PASS: app log contains TermSurf socket listening
PASS: Tui client received expected reply
PASS: socket fd=11 classified exactly once as Tui
PASS: app log contains TermSurf message decoded type=HelloRequest
PASS: Tui client received expected reply
PASS: socket fd=11 classified exactly once as Tui
PASS: app log contains TermSurf message decoded type=QueryTabsRequest
PASS: socket fd=11 classified exactly once as Browser
PASS: ServerRegister ignored log present
PASS: same-socket TUI-first client received HelloReply
PASS: socket fd=11 classified exactly once as Tui
PASS: same socket fd=11 did not reclassify to Browser
PASS: same-socket later ServerRegister used existing dispatch
PASS: same-socket browser-first client kept existing HelloReply dispatch
PASS: socket fd=11 classified exactly once as Browser
PASS: same socket fd=11 did not reclassify to Tui
PASS: same-socket first ServerRegister ignored log present
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale TermSurf process remains
runtime verification passed
```

The app log shows the sticky classification behavior on the mixed-message
same-socket cases:

```text
info(termsurf): TermSurf client connected fd=11
info(termsurf): TermSurf message decoded type=HelloRequest
info(termsurf): TermSurf connection type=Tui fd=11
info(termsurf): TermSurf HelloReply sent
info(termsurf): TermSurf message decoded type=ServerRegister
info(termsurf): TermSurf message ignored type=ServerRegister
info(termsurf): TermSurf client connected fd=11
info(termsurf): TermSurf message decoded type=ServerRegister
info(termsurf): TermSurf connection type=Browser fd=11
info(termsurf): TermSurf message ignored type=ServerRegister
info(termsurf): TermSurf message decoded type=HelloRequest
info(termsurf): TermSurf HelloReply sent
```

## Result Review

Fresh-context adversarial result review returned **APPROVED** with no required
findings.

The reviewer confirmed:

- the implementation scope stayed limited to classification and logging in
  `ghostboard/src/apprt/termsurf.zig`;
- classification happens only on the first decoded message;
- `ServerRegister` maps to browser and all other first messages map to TUI;
- the existing request/reply dispatch remains unchanged;
- the README marks Experiment 12 as `Pass`;
- the native build, macOS app build, runtime harness, and `git diff --check`
  evidence support the recorded result.

## Conclusion

Ghostboard now has the same first-message connection classification foundation
as Wezboard: browser-engine connections are identified by an initial
`ServerRegister`, while TUI connections are identified by any other initial
message. Classification is per socket and does not change after later messages.

The next experiment can build on this by adding browser server registration
state without changing the already-working synchronous TUI request/reply paths.
