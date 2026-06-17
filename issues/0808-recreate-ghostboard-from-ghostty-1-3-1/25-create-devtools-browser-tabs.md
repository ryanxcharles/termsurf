# Experiment 25: Create DevTools Browser Tabs

## Description

Experiments 19 and 24 left Ghostboard with enough state to identify an existing
browser tab and clean it up on TUI disconnect. The next DevTools-specific gap is
`SetDevtoolsOverlay`.

`webtui` opens DevTools by first asking the GUI whether an inspected tab can be
targeted, then asking the GUI to open a split running `web devtools://<tab_id>`.
The new TUI process sends `SetDevtoolsOverlay` instead of `SetOverlay`. Fresh
Ghostboard currently ignores `SetDevtoolsOverlay`, so even if a DevTools TUI
process exists, the browser server never receives `CreateDevtoolsTab`.

This experiment will implement the first Ghostboard DevTools tab-creation path:
when a TUI sends `SetDevtoolsOverlay` for an existing attached browser server,
Ghostboard records a pane for that DevTools TUI and sends
`CreateDevtoolsTab(pane_id, inspected_tab_id, pixel_width, pixel_height, dark)`
to the browser server. When that browser server later sends `TabReady` for the
DevTools pane, existing `TabReady` and `BrowserReady` handling should make the
DevTools TUI ready in the same way as a normal browser TUI, with one important
DevTools distinction: DevTools panes must not become the `last_browser_pane`.
Wezboard keeps `QueryLast` pointed at ordinary browser panes by updating last
only when `inspected_tab_id == 0`.

This experiment will not implement `OpenSplit`, native overlay presentation,
CALayerHost attachment, input forwarding, DevTools duplicate detection,
standalone DevTools server spawning for a missing browser server, browser
process shutdown, or changes to `webtui`, `roamium`, or the protocol schema.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - add `SetDevtoolsOverlay` to message dispatch and message-name logging;
  - add DevTools pane state for `inspected_tab_id`;
  - add a `CreateDevtoolsTab` sender that mirrors the existing `CreateTab`
    framing style;
  - for a new `SetDevtoolsOverlay`, create/update pane state with profile,
    browser, geometry, browsing state, TUI fd, and inspected tab id;
  - resolve the existing server by profile/browser and send `CreateDevtoolsTab`
    only when that server has an attached browser fd;
  - increment the matched server pane count for the new DevTools pane;
  - update `handleTabReady` to record `last_browser_pane` only for normal panes
    whose `inspected_tab_id == 0`, so DevTools `TabReady` does not change
    `QueryLast`;
  - for repeated `SetDevtoolsOverlay` on an existing ready DevTools pane, reuse
    the existing resize path so the browser receives `Resize(tab_id=...)`;
  - release `state_mutex` before writing `CreateDevtoolsTab` or `Resize` to a
    browser fd;
  - log successful DevTools tab creation with pane id and inspected tab id;
  - keep `OpenSplit`, native overlay presentation, input forwarding, and browser
    process shutdown out of scope.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, CLI install
behavior, CALayerHost overlay presentation, input forwarding, or browser process
shutdown in this experiment.

## Verification

Pass criteria:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passes inside `ghostboard/`, with the command, cwd, and exit status recorded
  in a log.
- The native GhosttyKit framework build passes:
  `zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`,
  with the command, cwd, and exit status recorded in a log.
- The macOS app build passes:
  `macos/build.nu --scheme Ghostty --configuration Debug --action build`.
- Runtime harness launches `TermSurf.app`, discovers the GUI socket via the
  deterministic PID-derived path
  `$TMPDIR/termsurf/termsurf-ghostboard-{pid}.sock`, verifies that socket file
  exists, and creates a normal browser pane through the existing
  `SetOverlay -> ServerRegister -> CreateTab -> TabReady(pane-normal, 42) -> BrowserReady`
  path.
- The harness sends
  `QueryDevtoolsRequest(profile=default, browser=<helper>, inspected_tab_id=42)`
  and receives a successful reply for the normal tab.
- A second TUI socket sends
  `SetDevtoolsOverlay(browser=<same helper>, profile=default, inspected_tab_id=42, pane_id=pane-dev)`.
- The already-attached helper receives
  `CreateDevtoolsTab(pane_id=pane-dev, inspected_tab_id=42)`.
- After the helper sends `TabReady(pane-dev, 99)`, the DevTools TUI socket
  receives `BrowserReady(pane_id=pane-dev, tab_id=99)` with the real helper
  listen socket.
- After DevTools `TabReady`, `QueryLastRequest(profile=default)` still returns
  the original normal browser pane `pane-normal` with `tab_id=42`, not the
  DevTools pane `pane-dev`.
- Repeating `SetDevtoolsOverlay` for `pane-dev` after `TabReady` sends
  `Resize(tab_id=99)` rather than a second `CreateDevtoolsTab`.
- `QueryTabsRequest(profile=default)` reports `gui_panes = 2` while both the
  normal and DevTools panes are live.
- Closing the DevTools TUI socket sends `CloseTab(tab_id=99)` to the helper and
  leaves the normal pane live.
- App logs include `CreateDevtoolsTab: pane_id=pane-dev inspected_tab_id=42`.
- The runtime harness verifies shutdown cleanup removes the socket file and
  leaves no stale `TermSurf.app/Contents/MacOS/termsurf` or helper process.
- `git diff --check` is clean.

Fail criteria:

- `SetDevtoolsOverlay` is ignored.
- `CreateDevtoolsTab` is sent before a matching browser server is attached.
- `CreateDevtoolsTab` is sent to the TUI socket instead of the browser server
  socket.
- The `CreateDevtoolsTab` uses the wrong pane id or inspected tab id.
- The DevTools TUI does not receive `BrowserReady` after `TabReady`.
- DevTools `TabReady` makes `QueryLastRequest(profile=default)` return the
  DevTools pane instead of the original normal browser pane.
- Repeated `SetDevtoolsOverlay` creates duplicate DevTools tabs instead of
  resizing the ready one.
- Closing the DevTools TUI closes the normal browser tab or removes the normal
  pane from `QueryTabs`.
- The implementation adds `OpenSplit`, native overlay presentation, CALayerHost
  attachment, input forwarding, standalone missing-server DevTools spawning,
  browser process shutdown, or changes `webtui`, `roamium`, or the protocol
  schema in this experiment.

## Design Review

A fresh-context adversarial Codex subagent reviewed the initial design and
required one fix: DevTools `TabReady` must not make the DevTools pane become
`last_browser_pane`. The reviewer pointed out that current Ghostboard updates
`last_browser_pane` unconditionally on every `TabReady`, while Wezboard updates
last only for panes whose `inspected_tab_id == 0`. The reviewer also suggested
making the normal tab id explicit in the verification.

The design was updated so `handleTabReady` must preserve `QueryLast` for normal
browser panes when DevTools panes become ready, and the runtime verification now
checks that `QueryLastRequest(profile=default)` still returns `pane-normal` with
`tab_id=42` after `TabReady(pane-dev, 99)`. The normal setup path now explicitly
uses `TabReady(pane-normal, 42)`.

The reviewer re-reviewed those fixes and approved the design with no remaining
required findings.

## Result

**Result:** Pass

Implemented DevTools browser tab creation in
`ghostboard/src/apprt/termsurf.zig`.

The implementation now:

- decodes and dispatches `SetDevtoolsOverlay`;
- tracks `inspected_tab_id` in `PaneState`;
- creates a DevTools pane for a TUI that targets an existing attached browser
  server;
- sends `CreateDevtoolsTab` to the browser server with pane id, inspected tab
  id, and pixel geometry;
- sends DevTools `BrowserReady` through the existing `TabReady` path;
- keeps `last_browser_pane` pointed at normal browser panes by updating it only
  when `inspected_tab_id == 0`;
- sends `Resize(tab_id=...)` for repeated `SetDevtoolsOverlay` messages after
  the DevTools pane is ready;
- lets Experiment 24's disconnect cleanup send `CloseTab` for DevTools tabs;
- logs successful DevTools tab creation as
  `CreateDevtoolsTab: pane_id=... inspected_tab_id=...`.

Verification passed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed: `logs/ghostboard-exp25-zig-fmt-20260616.log`.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp25-zig-native-xcframework-20260616.log`.
- macOS app build passed:
  `logs/ghostboard-exp25-macos-build-debug-20260616.log`.
- Runtime harness passed: `logs/ghostboard-exp25-runtime-harness-20260616.log`.
- Runtime app log: `logs/ghostboard-exp25-runtime-app-20260616.log`.
- `git diff --check` passed.

Observed successful runtime checks:

```text
PASS: socket exists at deterministic PID path
PASS: normal pane reached BrowserReady tab_id=42
PASS: QueryDevtoolsRequest succeeded for normal tab 42
PASS: DevTools TUI received BrowserReady tab_id=99
PASS: QueryLast still returns pane-normal tab_id=42 after DevTools TabReady
PASS: repeated SetDevtoolsOverlay emitted Resize to helper
PASS: QueryTabs reports gui_panes=2 while normal and DevTools panes are live
PASS: closing DevTools TUI sent CloseTab 99 and left normal pane live
PASS: app log contains CreateDevtoolsTab and DevTools CloseTab
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale matching TermSurf.app/Contents/MacOS/termsurf or devtools-helper.py processes
runtime verification passed
```

The runtime harness created a normal browser pane, had the helper send
`TabReady(pane-normal, 42)`, verified `QueryDevtoolsRequest` succeeded for that
tab, then sent `SetDevtoolsOverlay` for `pane-dev`. The attached helper received
`CreateDevtoolsTab(pane-dev, inspected_tab_id=42)`, sent
`TabReady(pane-dev, 99)`, and the DevTools TUI received `BrowserReady`.

After DevTools `TabReady`, `QueryLastRequest(profile=default)` still returned
`pane-normal` with `tab_id=42`, proving DevTools panes do not take over the
normal "last browser pane" slot. A repeated `SetDevtoolsOverlay` sent
`Resize(tab_id=99)`, and closing the DevTools TUI sent `CloseTab(tab_id=99)`
without removing the normal pane.

## Conclusion

Ghostboard can now create DevTools browser tabs for an existing attached browser
server and keep normal-pane `QueryLast` semantics intact. DevTools split
creation, duplicate DevTools detection, native overlay presentation, and
standalone missing-server DevTools startup remain separate future work.

## Completion Review

A fresh-context adversarial Codex subagent reviewed the completed Experiment 25
result and returned **APPROVED** with no findings.

The reviewer confirmed that the diff is limited to
`ghostboard/src/apprt/termsurf.zig` and the issue docs, that no `webtui`,
`roamium`, or protocol changes were made, and that the runtime logs prove normal
`TabReady` 42, `QueryDevtools` success, `CreateDevtoolsTab`, DevTools
`BrowserReady` 99, `QueryLast` still returning the normal pane, `Resize` 99,
`QueryTabs` 2, DevTools `CloseTab` 99, and no stale matching processes.
