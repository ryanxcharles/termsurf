# Experiment 9: Reply To QueryTabsRequest

## Description

Experiment 8 proved that Ghostboard can accept TermSurf socket clients, decode
the current protobuf schema, and answer `HelloRequest`. The next smallest step
toward running the existing `webtui` is to answer `QueryTabsRequest`, because
`webtui` already sends that message through the same synchronous request/reply
path and expects a `QueryTabsReply`.

This experiment will implement only the baseline GUI-side reply for the current
Ghostboard state. Ghostboard does not yet launch Roamium, register browser
servers, or maintain browser tab inventory, so the correct initial reply is an
empty successful inventory:

- `gui_panes = 0`
- `chromium_tabs = 0`
- `chromium_browser = 0`
- `chromium_devtools = 0`
- `tabs = []`
- `error = ""`

This intentionally mirrors the current Wezboard behavior before any browser tabs
exist, while keeping browser launch, overlay setup, pane tracking, and tab
registration for later experiments.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - recognize `QueryTabsRequest` in the decoded `TermSurfMessage` switch;
  - log the request's `pane_id` and `profile`;
  - send a length-prefixed `QueryTabsReply` with zero counts, no tab entries,
    and an empty error string.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, or CLI install
behavior.

## Verification

Pass criteria:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passes inside `ghostboard/`.
- The native GhosttyKit framework build passes.
- The macOS app build passes.
- Runtime harness launches `TermSurf.app`, connects to `TERMSURF_SOCKET`, sends
  a length-prefixed current-schema `QueryTabsRequest`, and decodes a
  length-prefixed `QueryTabsReply`.
- The decoded reply has `gui_panes = 0`, `chromium_tabs = 0`,
  `chromium_browser = 0`, `chromium_devtools = 0`, no `tabs`, and empty `error`.
- The runtime harness also sends `HelloRequest` before or after
  `QueryTabsRequest` to prove Experiment 8's behavior still works on the same
  socket implementation.
- The app log contains `TermSurf message decoded type=QueryTabsRequest` and a
  reply-sent log for `QueryTabsReply`.
- Shutdown cleanup still removes the socket file and leaves no stale
  `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- `QueryTabsRequest` is ignored or returns no frame.
- The reply has the wrong oneof message type.
- Any count is nonzero before Ghostboard has implemented browser/tab state.
- Any `webtui`, `roamium`, protocol schema, app branding, config path, icon, or
  CLI install behavior changes are needed for this experiment.

## Design Review

Fresh-context adversarial design review returned `APPROVED` with no required
findings.

The reviewer checked that the README links Experiment 9 as `Designed`, the
experiment has the required sections, the scope is limited to
`ghostboard/src/apprt/termsurf.zig`, the protocol schema supports the proposed
`QueryTabsReply` fields, Wezboard returns an empty Chromium inventory when no
browser tabs exist, Ghostboard currently has no browser/tab registry, and the
verification covers formatting, builds, runtime harness behavior, shutdown
cleanup, and `git diff --check`.

## Result

**Result:** Pass

Implemented `QueryTabsRequest` handling on Ghostboard's TermSurf socket.

`ghostboard/src/apprt/termsurf.zig` now:

- recognizes decoded `QueryTabsRequest` messages;
- logs the request `pane_id` and `profile`;
- sends a length-prefixed `QueryTabsReply`;
- leaves the reply as an empty successful inventory because Ghostboard does not
  yet launch Roamium, register browser servers, track overlays, or maintain tab
  state.

The encoded empty reply observed by the harness was:

```text
03 00 00 00 f2 01 00
```

That is a 3-byte `TermSurfMessage` payload using oneof field 30
(`QueryTabsReply`) with an empty nested message.

Verification performed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp9-zig-native-xcframework-20260616-092305.log`.
- macOS app build passed:
  `logs/ghostboard-exp9-macos-build-debug-20260616-092327.log`.
- Runtime verification passed:
  `logs/ghostboard-exp9-runtime-harness-20260616-092431.log`. The app's stderr
  log for that run is `logs/ghostboard-exp9-runtime-app-20260616-092431.log`.
- `git diff --check` passed.
- No Swift files were edited in this experiment, so SwiftLint was not required.
- Scope check found no diffs in `webtui`, `roamium`, `proto/termsurf.proto`,
  `ghostboard/build.zig`, `ghostboard/macos`,
  `ghostboard/src/build/GhosttyExe.zig`, app branding, config paths, icon
  assets, or CLI install behavior.

Observed successful runtime output:

```text
PASS: socket path is under TMPDIR/termsurf
PASS: socket exists while app is running
PASS: HelloReply frame 03000000c20100
PASS: QueryTabsReply empty frame 03000000f20100
runtime socket checks passed
PASS: app log contains TermSurf socket listening
PASS: app log contains TermSurf client connected
PASS: app log contains TermSurf message decoded type=HelloRequest
PASS: app log contains TermSurf HelloReply sent
PASS: app log contains TermSurf message decoded type=QueryTabsRequest
PASS: app log contains TermSurf QueryTabsRequest pane_id=exp9 profile=default
PASS: app log contains TermSurf QueryTabsReply sent
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale TermSurf process remains
runtime verification passed
```

The app log for the same run contained:

```text
info(termsurf): TermSurf socket listening on /var/folders/vx/wbmx10nd7tx8259xgg3v4vf80000gn/T/termsurf/termsurf-ghostboard-54530.sock
info(termsurf): TermSurf client connected fd=13
info(termsurf): TermSurf message decoded type=HelloRequest
info(termsurf): TermSurf HelloReply sent
info(termsurf): TermSurf client connected fd=13
info(termsurf): TermSurf message decoded type=QueryTabsRequest
info(termsurf): TermSurf QueryTabsRequest pane_id=exp9 profile=default
info(termsurf): TermSurf QueryTabsReply sent
```

## Conclusion

Ghostboard now handles a second synchronous TermSurf request/reply path. This
keeps `HelloRequest` working and adds the empty `QueryTabsReply` response that
`webtui` expects before any browser/tab state exists.

The next experiment should either add another synchronous query needed by
`webtui` startup, such as `QueryLastRequest`, or introduce the explicit
TUI-versus-browser connection classification needed before browser registration
and launch behavior can be implemented.

## Result Review

Fresh-context adversarial result review returned `APPROVED` with no required,
optional, or nit findings.

The reviewer checked the actual uncommitted working-tree diff and confirmed it
only touched `ghostboard/src/apprt/termsurf.zig`, this experiment file, and the
issue README. The reviewer also confirmed `git status --short` showed the result
commit had not yet been made, `git diff --check` was clean, `QueryTabsReply`
uses oneof field 30 with an initialized empty protobuf-c nested message, the
runtime harness log proves `HelloReply` still works and the empty
`QueryTabsReply` frame is returned, and the README status matches this
experiment's `Pass` result.
