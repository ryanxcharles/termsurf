# Experiment 13: Handle Unmatched ServerRegister

## Description

Experiment 12 gave each TermSurf socket a sticky connection type. The next
browser-side protocol step is to stop treating `ServerRegister` as a generic
ignored message and handle it as an explicit registration path.

In Wezboard, `ServerRegister` does not create a browser server record by itself.
It attaches a browser connection to a server record that was already created
when the GUI launched the browser process for a pending overlay. If no pending
server matches the registering profile, Wezboard logs
`ServerRegister: no matching server for profile=...` and returns no server key.

Ghostboard does not yet implement `SetOverlay`, browser launch, pending server
records, `CreateTab`, `TabReady`, or overlay presentation. Therefore this
experiment will implement only the faithful no-pending-server behavior:

- recognize `ServerRegister` in the decoded message switch;
- log the registering profile;
- log that no matching server exists yet;
- leave the connection open and keep existing dispatch behavior unchanged.

This creates a clear protocol hook for the later experiment that introduces
pending server records and browser launch.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - add an explicit `ServerRegister` branch in `handleClient`;
  - add a helper that logs `ServerRegister: profile={profile}`;
  - for now, always log
    `ServerRegister: no matching server for profile={profile}` because
    Ghostboard has no pending server registry yet;
  - keep `server_key`, server maps, browser process launch, tab creation, and
    overlay state out of scope.

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
  - a first-message `ServerRegister` still classifies the socket as `Browser`;
  - `ServerRegister` logs `ServerRegister: profile=default`;
  - `ServerRegister` logs
    `ServerRegister: no matching server for profile=default`;
  - the old generic `TermSurf message ignored type=ServerRegister` log is no
    longer emitted for that handled message;
  - the browser-classified socket remains open long enough to receive a later
    `HelloRequest`, preserving the existing dispatch behavior from
    Experiment 12.
- The runtime harness also sends a normal TUI `HelloRequest` on a fresh socket
  and receives `HelloReply`, proving the registration path did not break TUI
  request/reply behavior.
- The harness verifies shutdown cleanup still removes the socket file and leaves
  no stale `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- `ServerRegister` is still handled only by the generic ignored-message branch.
- The implementation creates fake server state before Ghostboard has launched a
  browser process or created a pending server record.
- The implementation sends `CreateTab`, `BrowserReady`, or any other browser
  lifecycle message in this experiment.
- Browser/TUI classification or the synchronous request/reply paths from
  Experiments 8 through 12 regress.
- Any `webtui`, `roamium`, protocol schema, app branding, config path, icon, or
  CLI install behavior changes are needed for this experiment.

## Design Review

Fresh-context adversarial design review returned **APPROVED** with no required,
optional, or nit findings.

The reviewer confirmed the README links Experiment 13 as `Designed`, the
experiment has the required sections, the scope is narrow, and the planned
unmatched `ServerRegister` behavior matches Wezboard's no-pending-server path:
log the profile, find no matching pending server, return no server key, keep the
socket alive, and avoid fake server, browser, or tab state.

## Result

**Result:** Pass

Implemented explicit unmatched `ServerRegister` handling in
`ghostboard/src/apprt/termsurf.zig`.

The socket handler now:

- routes `ServerRegister` to a dedicated branch instead of the generic
  ignored-message branch;
- extracts the protobuf-c profile C string into a Zig slice;
- logs `ServerRegister: profile={profile}`;
- logs `ServerRegister: no matching server for profile={profile}`;
- keeps the socket open and leaves all existing request/reply dispatch behavior
  unchanged.

Verification passed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp13-zig-native-xcframework-20260616-100912.log`.
- macOS app build passed:
  `logs/ghostboard-exp13-macos-build-debug-20260616-100946.log`.
- Runtime harness passed:
  `logs/ghostboard-exp13-runtime-harness-20260616-101031.log`.
- Runtime app log: `logs/ghostboard-exp13-runtime-app-20260616-101031.log`.
- `git diff --check` passed.

The first native build attempt failed because the helper mixed a protobuf-c
`[*c]u8` profile pointer with a Zig string literal fallback. I fixed that by
normalizing the optional protobuf-c profile pointer with `std.mem.span` and
returning an empty Zig slice when the request or profile pointer is missing.

Observed successful runtime checks:

```text
PASS: child wrote TERMSURF_SOCKET
PASS: socket path is under TMPDIR/termsurf
PASS: socket exists while app is running
PASS: app log contains TermSurf socket listening
PASS: browser-classified socket stayed open and received later HelloReply
PASS: client connection was logged
PASS: socket fd=11 classified exactly once as Browser
PASS: ServerRegister profile log present
PASS: unmatched ServerRegister warning present
PASS: ServerRegister did not use generic ignored branch
PASS: fresh TUI client received HelloReply
PASS: client connection was logged
PASS: socket fd=11 classified exactly once as Tui
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale TermSurf process remains
runtime verification passed
```

## Result Review

Fresh-context adversarial result review returned **APPROVED** with no findings.

The reviewer confirmed:

- `ServerRegister` is handled explicitly;
- the helper logs the profile and unmatched-server warning without creating fake
  server state or launching browsers;
- the behavior matches Wezboard's no-matching-server path;
- the claimed native build, macOS app build, runtime harness, and
  `git diff --check` evidence support the result;
- the README marks Experiment 13 as `Pass`;
- the result commit had not been made before review.

## Conclusion

Ghostboard now has an explicit browser registration hook. A registering browser
connection is classified as `Browser`, logs its profile, and follows Wezboard's
no-pending-server behavior by warning that no matching server exists yet.

The next experiment can introduce actual pending server records from the GUI/TUI
side, then attach a registering browser connection to those records instead of
warning unconditionally.
