# Experiment 18: Count QueryTabs GUI Panes From State

## Description

Experiment 9 added the first `QueryTabsRequest` reply, but it intentionally
returned an empty inventory because Ghostboard did not yet track overlay pane
state. Experiments 14 through 17 now maintain bounded pane state from
`SetOverlay`, attach browser sockets, send `CreateTab`, record `TabReady`, and
answer `QueryLastRequest` from that state.

The next smallest parity step is to make `QueryTabsRequest` report the number of
GUI panes currently known to Ghostboard, matching Wezboard's current GUI-side
behavior:

- count tracked panes whose profile matches the request profile;
- if the request profile is empty, count all tracked panes;
- keep Chromium-side counts at zero;
- keep `tabs = []`;
- keep `error = ""`.

This experiment does not implement Chromium tab inventory, browser launch,
`BrowserReady`, overlay presentation, DevTools tab listing, navigation, or input
forwarding.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - update the `QueryTabsRequest` branch to pass the decoded request into
    `sendQueryTabsReply`;
  - under the state mutex, count `PaneState` entries with `in_use = true`;
  - apply the same profile filter as Wezboard: empty request profile matches all
    panes, nonempty request profile matches only panes with that profile;
  - populate `QueryTabsReply.gui_panes` with that count;
  - leave `chromium_tabs`, `chromium_browser`, `chromium_devtools`, `tabs`, and
    `error` at their initialized empty values.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, CLI install
behavior, browser process launch, `BrowserReady`, overlay presentation, or input
forwarding.

## Verification

Pass criteria:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passes inside `ghostboard/`.
- The native GhosttyKit framework build passes:
  `zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`.
- The macOS app build passes:
  `macos/build.nu --scheme Ghostty --configuration Debug --action build`.
- Runtime harness launches `TermSurf.app`, connects to `TERMSURF_SOCKET`, and
  proves:
  - before any `SetOverlay`, `QueryTabsRequest(profile=default)` returns
    `gui_panes = 0`;
  - after two default-profile `SetOverlay` messages and one other-profile
    `SetOverlay` message, `QueryTabsRequest(profile=default)` returns
    `gui_panes = 2`;
  - `QueryTabsRequest(profile="")` returns `gui_panes = 3`;
  - `QueryTabsRequest(profile=other)` returns `gui_panes = 1`;
  - updating an existing default-profile pane with another `SetOverlay` does not
    increment the count;
  - every `QueryTabsReply` keeps `chromium_tabs = 0`, `chromium_browser = 0`,
    `chromium_devtools = 0`, `tabs = []`, and `error = ""`;
  - no `BrowserReady`, browser process launch, or overlay presentation logs are
    emitted by this experiment.
- The runtime harness also sends a normal TUI `HelloRequest` on a fresh socket
  and receives `HelloReply`, proving existing request/reply behavior still
  works.
- The harness verifies shutdown cleanup still removes the socket file and leaves
  no stale `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- `QueryTabsRequest` still ignores existing `PaneState`.
- Profile-filtered counts are wrong.
- Duplicate `SetOverlay` updates inflate `gui_panes`.
- The implementation reports Chromium-side counts or tab entries before
  Ghostboard has implemented browser-side inventory.
- The implementation sends `BrowserReady`, launches a browser process, or
  creates overlay UI in this experiment.
- Browser/TUI classification or the synchronous request/reply paths from
  Experiments 8 through 17 regress.
- Any `webtui`, `roamium`, protocol schema, app branding, config path, icon, or
  CLI install behavior changes are needed for this experiment.

## Design Review

Fresh-context adversarial design review returned **APPROVED** with no findings.

The reviewer confirmed the README links Experiment 18 as `Designed`, the design
has the required sections, the scope is narrow, the `QueryTabsRequest` plan
matches Wezboard's GUI-pane counting behavior, and the verification criteria are
concrete enough to prove the intended behavior and guard regressions.

## Result

**Result:** Pass

Implemented state-backed GUI pane counting for `QueryTabsRequest` in
`ghostboard/src/apprt/termsurf.zig`.

The socket handler now passes the decoded `QueryTabsRequest` into
`sendQueryTabsReply`. The reply helper counts `PaneState` records under the
state mutex, applies Wezboard's profile filter semantics, and fills only
`QueryTabsReply.gui_panes`. Chromium-side counts, tab entries, and `error`
remain at their initialized empty values.

Verification passed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp18-zig-native-xcframework-20260616-110013.log`.
- macOS app build passed:
  `logs/ghostboard-exp18-macos-build-debug-20260616-110034.log`.
- Runtime harness passed:
  `logs/ghostboard-exp18-runtime-harness-20260616-110128.log`.
- Runtime app log: `logs/ghostboard-exp18-runtime-app-20260616-110128.log`.
- `git diff --check` passed.

Observed successful runtime checks:

```text
PASS: QueryTabs before SetOverlay returns zero default panes
PASS: QueryTabs default counts two default panes
PASS: QueryTabs empty profile counts all panes
PASS: QueryTabs other profile counts one pane
PASS: QueryTabs duplicate SetOverlay does not inflate count
PASS: fresh TUI client received HelloReply
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale TermSurf process remains
PASS: app log contains TermSurf socket listening
PASS: app log contains QueryTabsReply sends
PASS: app log contains duplicate pane update
PASS: no BrowserReady emitted
PASS: no CaContext emitted
PASS: no overlay presentation message emitted
PASS: no browser launch message emitted
runtime verification passed
```

The passing harness verified that every `QueryTabsReply` kept
`chromium_tabs = 0`, `chromium_browser = 0`, `chromium_devtools = 0`,
`tabs = []`, and `error = ""`.

## Conclusion

Ghostboard now reports the GUI pane count that `webtui` expects from
`QueryTabsRequest` after overlay state exists. This closes another synchronous
state query gap while leaving browser-side tab inventory, browser launch,
`BrowserReady`, CALayerHost overlay presentation, and input forwarding for later
experiments.

## Result Review

Fresh-context adversarial result review returned **APPROVED** with no required
findings.

The reviewer confirmed:

- the diff is limited to `ghostboard/src/apprt/termsurf.zig`, this experiment
  file, and the issue README;
- `QueryTabsRequest` now passes the decoded request into `sendQueryTabsReply`;
- `gui_panes` is counted under `state_mutex` with the same empty-profile
  all-panes semantics as Wezboard;
- Chromium-side fields, `tabs`, and `error` remain protobuf-c initialized
  defaults;
- the native framework build, macOS app build, and runtime harness logs prove
  the expected profile-count cases and negative checks;
- `zig fmt --check` and `git diff --check` passed;
- the result commit had not been made before review.
