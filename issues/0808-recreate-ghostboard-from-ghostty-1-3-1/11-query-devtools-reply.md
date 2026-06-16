# Experiment 11: Reply To QueryDevtoolsRequest

## Description

Experiments 8 through 10 established Ghostboard's TermSurf socket and the
no-state synchronous request/reply paths for `HelloRequest`, `QueryTabsRequest`,
and `QueryLastRequest`. The remaining synchronous query that `webtui` can send
before normal TUI startup is `QueryDevtoolsRequest`, used when opening a
DevTools TUI for an existing browser tab.

Ghostboard does not yet launch Roamium, create browser panes, track browser
profiles, track tab ids, or maintain DevTools panes. Therefore this experiment
will implement only the baseline validation/error behavior that is correct
before browser state exists.

The reply should follow Wezboard's validation order:

- if `browser` is empty, return `error = "DevTools target browser is required"`;
- else if `profile` is empty, return
  `error = "DevTools target profile is required"`;
- else if `inspected_tab_id == 0`, return
  `error = "DevTools target tab id is required"`;
- otherwise return
  `error = "Inspected tab {id} not found in {browser}/{profile}"`.

All success fields should remain at protobuf defaults because there is no
browser/tab state to resolve yet.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - recognize `QueryDevtoolsRequest` in the decoded `TermSurfMessage` switch;
  - log the request's `pane_id`, `inspected_tab_id`, `profile`, and `browser`;
  - send a length-prefixed `QueryDevtoolsReply` with the validation/no-state
    error described above;
  - add `QueryDevtoolsRequest` and `QueryDevtoolsReply` to the TermSurf message
    type name helper used by decoded-message logs.

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
  length-prefixed current-schema `QueryDevtoolsRequest` messages, and decodes
  length-prefixed `QueryDevtoolsReply` messages.
- The harness verifies all four no-state/validation replies:
  - empty `browser` returns `DevTools target browser is required`;
  - nonempty `browser` with empty `profile` returns
    `DevTools target profile is required`;
  - nonempty `browser` and `profile` with `inspected_tab_id = 0` returns
    `DevTools target tab id is required`;
  - nonempty `browser`, `profile`, and nonzero `inspected_tab_id` returns
    `Inspected tab {id} not found in {browser}/{profile}`.
- The runtime harness also sends `HelloRequest`, `QueryTabsRequest`, and
  `QueryLastRequest` to prove Experiments 8 through 10 still work on the same
  socket implementation.
- The app log contains `TermSurf message decoded type=QueryDevtoolsRequest` and
  a reply-sent log for `QueryDevtoolsReply`.
- Shutdown cleanup still removes the socket file and leaves no stale
  `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- `QueryDevtoolsRequest` is ignored or returns no frame.
- The reply has the wrong oneof message type.
- The validation order differs from Wezboard.
- The nonzero tab-id no-state case falsely reports success before Ghostboard has
  implemented browser pane state.
- Any `webtui`, `roamium`, protocol schema, app branding, config path, icon, or
  CLI install behavior changes are needed for this experiment.

## Design Review

Fresh-context adversarial design review returned `APPROVED` with no required
findings.

Optional notes:

- The reviewer noted that the build verification names the native GhosttyKit and
  macOS app builds without spelling out exact commands. I will use the same
  commands as Experiments 8 through 10 and record the exact commands in the
  result.
- The reviewer noted that the runtime harness is described behaviorally rather
  than as a named script. I will record the exact harness behavior and logs in
  the result, as in the previous socket experiments.

The reviewer confirmed the README links Experiment 11 as `Designed`, the
experiment has the required sections, the scope is limited to
`ghostboard/src/apprt/termsurf.zig`, the validation order matches Wezboard's
handler, and the no-state fallback is faithful because without tab/server state
Wezboard reaches the `Inspected tab ... not found` branch.

## Result

**Result:** Pass

Implemented `QueryDevtoolsRequest` handling on Ghostboard's TermSurf socket.

`ghostboard/src/apprt/termsurf.zig` now:

- recognizes decoded `QueryDevtoolsRequest` messages;
- logs the request `pane_id`, `inspected_tab_id`, `profile`, and `browser`;
- sends a length-prefixed `QueryDevtoolsReply`;
- validates requests in the same order as Wezboard:
  - missing `browser` -> `DevTools target browser is required`;
  - missing `profile` -> `DevTools target profile is required`;
  - missing `inspected_tab_id` -> `DevTools target tab id is required`;
  - otherwise -> `Inspected tab {id} not found in {browser}/{profile}`;
- keeps success fields at protobuf defaults because Ghostboard does not yet
  track browser tab state;
- includes `QueryDevtoolsRequest` and `QueryDevtoolsReply` in the
  decoded-message type name helper.

Verification performed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed after result-review fixes:
  `logs/ghostboard-exp11-zig-native-xcframework-20260616-094912-after-review2.log`.
- macOS app build passed after result-review fixes:
  `logs/ghostboard-exp11-macos-build-debug-20260616-094934-after-review2.log`.
- Runtime verification passed after result-review fixes:
  `logs/ghostboard-exp11-runtime-harness-20260616-095013-after-review2.log`. The
  app's stderr log for that run is
  `logs/ghostboard-exp11-runtime-app-20260616-095013-after-review2.log`.
- `git diff --check` passed.
- No Swift files were edited in this experiment, so SwiftLint was not required.
- Scope check found no diffs in `webtui`, `roamium`, `proto/termsurf.proto`,
  `ghostboard/build.zig`, `ghostboard/macos`,
  `ghostboard/src/build/GhosttyExe.zig`, app branding, config paths, icon
  assets, or CLI install behavior.

The first runtime harness used the wrong expected error field for
`QueryLastReply` while rechecking Experiment 10 behavior; the app returned the
same correct frame as Experiment 10. A later after-review harness exposed that
the standard nonzero-tab not-found path could panic when the formatted error
used an undersized NUL-terminated buffer. The final harness uses the corrected
parser, proves the normal cases, and includes a long browser/profile not-found
case that would have failed with the fixed-size implementation.

Observed successful runtime output:

```text
PASS: socket path is under TMPDIR/termsurf
PASS: socket exists while app is running
PASS: QueryDevtoolsReply missing browser
PASS: QueryDevtoolsReply missing profile
PASS: QueryDevtoolsReply missing tab id
PASS: QueryDevtoolsReply not found
PASS: QueryDevtoolsReply long not found
runtime socket checks passed
PASS: app log contains TermSurf message decoded type=QueryDevtoolsRequest
PASS: app log contains TermSurf QueryDevtoolsReply sent
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale TermSurf process remains
runtime verification passed
```

The app log for the same run contained the expected decoded request and reply
logs for `HelloRequest`, `QueryTabsRequest`, `QueryLastRequest`, and
`QueryDevtoolsRequest`.

## Result Review

Fresh-context adversarial result review returned `CHANGES REQUIRED`.

Required finding accepted and fixed:

- Valid nonzero `QueryDevtoolsRequest` inputs could fail to receive any reply
  when `browser` or `profile` made the formatted not-found error exceed the
  fixed 256-byte buffer. The reviewer correctly noted that protobuf strings are
  unconstrained up to the existing frame limit, and Wezboard formats the same
  error without a small fixed cap. I replaced the fixed buffer with an
  allocator-backed buffer sized from `std.fmt.count`, passed a `count + 1`
  buffer to `std.fmt.bufPrintZ`, and free it after `sendProtobuf` packs the
  reply.

Verification after the fix:

- `logs/ghostboard-exp11-zig-native-xcframework-20260616-094912-after-review2.log`
  — Zig formatting and native GhosttyKit build passed.
- `logs/ghostboard-exp11-macos-build-debug-20260616-094934-after-review2.log` —
  macOS app build passed.
- `logs/ghostboard-exp11-runtime-harness-20260616-095013-after-review2.log` —
  runtime harness passed all four original DevTools validation branches plus a
  long browser/profile not-found case that exceeds the original 256-byte buffer.

Re-review returned `APPROVED`. The reviewer confirmed the fixed code computes
the formatted not-found error length, allocates `error_len + 1`, formats with
`bufPrintZ`, keeps the NUL-terminated string alive until after synchronous
protobuf packing, and that the long browser/profile runtime case passed.

## Conclusion

Ghostboard now handles all four synchronous request/reply paths that `webtui`
can use before browser launch or overlay creation: `HelloRequest`,
`QueryTabsRequest`, `QueryLastRequest`, and `QueryDevtoolsRequest`.

The next experiment should introduce explicit TUI-versus-browser connection
classification and/or the first browser-side message path (`ServerRegister`), so
Ghostboard can start moving from no-state replies toward real Roamium launch and
tab lifecycle behavior.
