# Experiment 10: Reply To QueryLastRequest

## Description

Experiment 9 added the second synchronous request/reply path by answering
`QueryTabsRequest` with an empty successful tab inventory. The next smallest
`webtui` request/reply path is `QueryLastRequest`, used by `web last` to ask the
GUI for the last active browser pane.

Ghostboard does not yet create browser panes, launch Roamium, track tab ids, or
record a last active browser pane. Therefore this experiment should implement
only the baseline no-state reply that Wezboard returns before any browser pane
exists: a `QueryLastReply` with default `pane_id`, `tab_id`, and `profile`, and
`error = "No browser pane yet"`.

This advances protocol parity without inventing browser state early. Later
experiments can replace this no-state reply with real pane tracking once
`SetOverlay`, browser launch, `ServerRegister`, and `TabReady` are implemented.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - recognize `QueryLastRequest` in the decoded `TermSurfMessage` switch;
  - log the request's `pane_id` and `profile`;
  - send a length-prefixed `QueryLastReply` whose `error` field is
    `No browser pane yet`;
  - add `QueryLastRequest` and `QueryLastReply` to the TermSurf message type
    name helper used by decoded-message logs.

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
  a length-prefixed current-schema `QueryLastRequest`, and decodes a
  length-prefixed `QueryLastReply`.
- The decoded reply has empty `pane_id`, `tab_id = 0`, empty `profile`, and
  `error = "No browser pane yet"`.
- The runtime harness also sends `HelloRequest` and `QueryTabsRequest` to prove
  Experiments 8 and 9 still work on the same socket implementation.
- The app log contains `TermSurf message decoded type=QueryLastRequest` and a
  reply-sent log for `QueryLastReply`.
- Shutdown cleanup still removes the socket file and leaves no stale
  `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- `QueryLastRequest` is ignored or returns no frame.
- The reply has the wrong oneof message type.
- The reply falsely reports a pane id, tab id, or profile before Ghostboard has
  implemented browser pane state.
- The reply omits the no-state error string.
- Any `webtui`, `roamium`, protocol schema, app branding, config path, icon, or
  CLI install behavior changes are needed for this experiment.

## Design Review

Fresh-context adversarial design review returned `CHANGES REQUIRED`.

Required finding accepted and fixed:

- The initial plan omitted the `msgTypeName` update needed by the planned log
  verification. Without explicit `QueryLastRequest` and `QueryLastReply` message
  names, the decoded-message log would report `Other`, while the verification
  expected `TermSurf message decoded type=QueryLastRequest`. The plan now
  includes the required message-name helper update.

Re-review returned `APPROVED`. The reviewer confirmed the prior finding is
resolved and that no new required finding was introduced by the fix.

## Result

**Result:** Pass

Implemented `QueryLastRequest` handling on Ghostboard's TermSurf socket.

`ghostboard/src/apprt/termsurf.zig` now:

- recognizes decoded `QueryLastRequest` messages;
- logs the request `pane_id` and `profile`;
- sends a length-prefixed `QueryLastReply`;
- sets `error = "No browser pane yet"` while leaving `pane_id`, `tab_id`, and
  `profile` at protobuf defaults because Ghostboard does not yet track browser
  pane state;
- includes `QueryLastRequest` and `QueryLastReply` in the decoded-message type
  name helper.

The encoded no-state reply observed by the harness was:

```text
18 00 00 00 d2 01 15 22 13 4e 6f 20 62 72 6f 77 73 65 72 20 70 61 6e 65 20 79 65 74
```

That is a 24-byte length-prefixed `TermSurfMessage` payload using oneof field 26
(`QueryLastReply`) with field 4 (`error`) set to `No browser pane yet`.

Verification performed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp10-zig-native-xcframework-20260616-093232.log`.
- macOS app build passed:
  `logs/ghostboard-exp10-macos-build-debug-20260616-093252.log`.
- Runtime verification passed:
  `logs/ghostboard-exp10-runtime-harness-20260616-093416.log`. The app's stderr
  log for that run is `logs/ghostboard-exp10-runtime-app-20260616-093416.log`.
- `git diff --check` passed.
- No Swift files were edited in this experiment, so SwiftLint was not required.
- Scope check found no diffs in `webtui`, `roamium`, `proto/termsurf.proto`,
  `ghostboard/build.zig`, `ghostboard/macos`,
  `ghostboard/src/build/GhosttyExe.zig`, app branding, config paths, icon
  assets, or CLI install behavior.

The first runtime harness used the wrong expected protobuf tag for the nested
`error` field and reported a local harness expectation failure even though the
app replied correctly. The final harness corrected the expected protobuf field 4
tag and exited hard on Python assertion failures.

Observed successful runtime output:

```text
PASS: socket path is under TMPDIR/termsurf
PASS: socket exists while app is running
PASS: HelloReply frame 03000000c20100
PASS: QueryTabsReply empty frame 03000000f20100
PASS: QueryLastReply no-state frame 18000000d2011522134e6f2062726f777365722070616e6520796574
runtime socket checks passed
PASS: app log contains TermSurf socket listening
PASS: app log contains TermSurf client connected
PASS: app log contains TermSurf message decoded type=HelloRequest
PASS: app log contains TermSurf HelloReply sent
PASS: app log contains TermSurf message decoded type=QueryTabsRequest
PASS: app log contains TermSurf QueryTabsReply sent
PASS: app log contains TermSurf message decoded type=QueryLastRequest
PASS: app log contains TermSurf QueryLastRequest pane_id=exp10 profile=default
PASS: app log contains TermSurf QueryLastReply sent
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale TermSurf process remains
runtime verification passed
```

The app log for the same run contained:

```text
info(termsurf): TermSurf socket listening on /var/folders/vx/wbmx10nd7tx8259xgg3v4vf80000gn/T/termsurf/termsurf-ghostboard-55673.sock
info(termsurf): TermSurf client connected fd=13
info(termsurf): TermSurf message decoded type=HelloRequest
info(termsurf): TermSurf HelloReply sent
info(termsurf): TermSurf client connected fd=13
info(termsurf): TermSurf message decoded type=QueryTabsRequest
info(termsurf): TermSurf QueryTabsReply sent
info(termsurf): TermSurf client connected fd=13
info(termsurf): TermSurf message decoded type=QueryLastRequest
info(termsurf): TermSurf QueryLastRequest pane_id=exp10 profile=default
info(termsurf): TermSurf QueryLastReply sent
```

## Conclusion

Ghostboard now handles the synchronous no-state `QueryLastRequest` path used by
`web last`, while keeping the previously implemented `HelloRequest` and
`QueryTabsRequest` paths working.

The next experiment should either implement the remaining no-state synchronous
query, `QueryDevtoolsRequest`, or introduce explicit TUI-versus-browser
connection classification before browser registration and launch behavior.

## Result Review

Fresh-context adversarial result review returned `APPROVED` with no findings.

The reviewer confirmed the diff is limited to
`ghostboard/src/apprt/termsurf.zig` and the Experiment 10 docs, the no-state
`QueryLastReply` matches Wezboard's behavior, protobuf-c initialization and
string lifetime are valid for packing, the runtime log proves `HelloRequest`,
`QueryTabsRequest`, and `QueryLastRequest` all received replies, the
`QueryLastReply` frame contains field 4 `error`, the README status is `Pass`,
the result commit had not yet been made, and `git diff --check` is clean.
