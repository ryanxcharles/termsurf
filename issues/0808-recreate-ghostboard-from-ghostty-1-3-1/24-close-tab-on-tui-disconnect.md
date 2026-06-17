# Experiment 24: Close Browser Tab on TUI Disconnect

## Description

Experiments 20 through 23 established the normal browser lifecycle path through
`BrowserReady`, resize forwarding, and focus forwarding. The next lifecycle gap
is teardown.

Fresh Ghostboard currently clears a pane's `tui_fd` when the TUI connection
exits, but it leaves the pane, tab lookup, server pane count, and browser tab
alive. That means a `webtui` process can exit while Roamium keeps an orphaned
tab. Wezboard's disconnect path removes panes owned by the disconnected TUI,
removes the `(server, tab_id) -> pane_id` lookup, decrements the server pane
count, and sends `CloseTab(tab_id)` to the browser server for panes with a
browser tab.

This experiment will implement the first Ghostboard version of that teardown
path: when a TUI connection exits, every pane owned by that TUI fd will be
removed from Ghostboard's pane/query state. Panes that already have a nonzero
`tab_id` and an attached matching browser server will additionally cause
`CloseTab` to be sent to that browser server. Pre-`TabReady` panes must be
removed too, but must not emit `CloseTab`. Pane state, tab lookup state, the
`last_browser_pane` pointer, and server pane counts will be updated while
holding `state_mutex`; socket writes must happen after `state_mutex` is
released.

This experiment will not terminate browser helper processes when their server
has no remaining panes, remove native overlay layers, send shutdown messages,
create or present overlays, forward input, create DevTools panes, split panes,
or change `webtui`, `roamium`, or the protocol schema.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - replace the TUI-disconnect-only `clearPaneTuiFd` behavior with a cleanup
    path for all panes owned by the disconnected TUI fd;
  - for each owned pane, remove the pane from query-visible pane state so
    `QueryLast` and `QueryTabs` cannot report disconnected panes;
  - for each owned pane, decrement the matched server's pane count without
    underflow, including panes that disconnect before `TabReady`;
  - for each owned pane with `tab_id != 0` and an attached matching browser
    server, snapshot browser fd, pane id, and tab id under `state_mutex`;
  - remove matching tab lookup state for that pane/tab;
  - clear `last_browser_pane` when it points at a pane removed by this cleanup,
    so `QueryLast` returns an error instead of a stale pane;
  - clear/remove the pane state after snapshotting all data needed for cleanup;
  - release `state_mutex` before sending any `CloseTab` protobuf to a browser
    fd;
  - log successful close sends with pane id and tab id;
  - keep browser process termination, server removal, `Shutdown`, overlay layer
    cleanup, and native rendering cleanup out of scope.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, CLI install
behavior, CALayerHost overlay presentation, input forwarding, DevTools pane
creation, split creation, or browser process shutdown in this experiment.

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
  exists, connects to it, and sends
  `SetOverlay(browser=/absolute/temp/helper, profile=default, pane_id=pane-a)`.
- The spawned helper connects back with `ServerRegister`, receives `CreateTab`,
  and sends `TabReady(pane-a, 42)`.
- The TUI socket receives `BrowserReady` with the real helper listen socket.
- The TUI socket closes.
- The helper receives `CloseTab(tab_id=42)` after the TUI socket closes.
- A pane that is disconnected before `TabReady` does not emit `CloseTab`, is
  absent from subsequent `QueryLastRequest(profile=default)` and
  `QueryTabsRequest(profile=default)` replies, and does not leave a stale server
  pane count behind.
- The pre-`TabReady` cleanup check proves the server pane count is not stale by
  creating a later overlay for the same profile/browser and verifying app logs
  show the reused server at `pane_count=1`, not a value inflated by the
  disconnected pre-ready pane.
- After the close, `QueryLastRequest(profile=default)` returns an error rather
  than the disconnected `pane-a`.
- After the close, `QueryTabsRequest(profile=default)` reports `gui_panes = 0`.
- App logs include `CloseTab: pane_id=pane-a tab_id=42`.
- The runtime harness verifies shutdown cleanup removes the socket file and
  leaves no stale `TermSurf.app/Contents/MacOS/termsurf` or helper process.
- `git diff --check` is clean.

Fail criteria:

- No `CloseTab` is sent for a disconnected TUI pane with a ready browser tab.
- `CloseTab` is sent before `TabReady`.
- `CloseTab` is sent to the TUI socket instead of the browser server socket.
- The close uses the wrong tab id.
- A pane disconnected before `TabReady` remains visible to `QueryLast` or
  `QueryTabs`.
- A pane disconnected before `TabReady` leaves the server pane count stale.
- Queries after disconnect still return the removed pane as an active browser
  pane.
- The cleanup path writes to sockets while holding `state_mutex`.
- The implementation terminates browser helper processes, sends `Shutdown`,
  removes native overlay layers, adds CALayerHost overlay presentation, adds
  input forwarding, creates DevTools panes, creates split panes, or changes
  `webtui`, `roamium`, or the protocol schema in this experiment.

## Design Review

An adversarial Codex subagent reviewed the initial design and required one fix:
the cleanup plan was too narrow because `SetOverlay` creates pane state and
increments server `pane_count` before `TabReady`, so pre-ready panes owned by a
disconnected TUI also need to be removed from pane/query state. The reviewer
also noted that runtime socket discovery needed to be explicit.

The design was updated to remove all panes owned by the disconnected TUI fd,
send `CloseTab` only for panes with nonzero `tab_id` and an attached browser fd,
clear `last_browser_pane`, decrement server pane counts for all removed panes,
verify pre-`TabReady` query cleanup and pane-count cleanup, and discover the
runtime socket via `$TMPDIR/termsurf/termsurf-ghostboard-{pid}.sock`.

The reviewer re-reviewed those fixes and approved the design with no remaining
required findings.

## Result

**Result:** Pass

Implemented TUI disconnect cleanup in `ghostboard/src/apprt/termsurf.zig`.

The implementation now:

- replaces the old `clearPaneTuiFd` behavior with `cleanupTuiPanes`;
- removes every pane owned by the disconnected TUI fd from pane/query state;
- decrements the matched server `pane_count` for every removed pane without
  underflow;
- clears `last_browser_pane` when it points at a removed pane;
- removes matching tab lookup state for ready tabs;
- snapshots `CloseTab` sends only for panes with nonzero `tab_id` and an
  attached browser fd;
- releases `state_mutex` before writing `CloseTab` to the browser fd;
- logs successful sends as `CloseTab: pane_id=... tab_id=...`.

Verification passed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed: `logs/ghostboard-exp24-zig-fmt-20260616.log`.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp24-zig-native-xcframework-20260616.log`.
- macOS app build passed:
  `logs/ghostboard-exp24-macos-build-debug-20260616.log`.
- Runtime harness passed: `logs/ghostboard-exp24-runtime-harness-20260616.log`.
- Runtime app log: `logs/ghostboard-exp24-runtime-app-20260616.log`.
- A process check found no stale matching
  `TermSurf.app/Contents/MacOS/termsurf`, `pre-helper.py`, or `ready-helper.py`
  processes, and the explicit command/result were appended to
  `logs/ghostboard-exp24-runtime-harness-20260616.log`.
- `git diff --check` passed.

Observed successful runtime checks:

```text
PASS: socket exists at deterministic PID path
PASS: pre-TabReady disconnected pane absent from QueryLast/QueryTabs
PASS: pre-ready helper did not receive CloseTab
PASS: TUI socket received BrowserReady with real helper listen socket
PASS: ready disconnected pane absent from QueryLast/QueryTabs
PASS: app log contains CloseTab, pre cleanup, and pane_count=1 reused server
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale matching TermSurf.app/Contents/MacOS/termsurf, pre-helper.py, or ready-helper.py processes
runtime verification passed
```

The pre-`TabReady` path was exercised by sending `SetOverlay` for `pane-pre`,
closing the TUI before the helper registered, then querying a fresh TUI client.
`QueryLastRequest(profile=default)` returned an error and
`QueryTabsRequest(profile=default)` reported zero GUI panes. A later overlay for
the same profile/browser reused the server at `pane_count=1`, proving the
pre-ready disconnect did not leave the server count inflated.

The ready-tab path was exercised by sending `SetOverlay` for `pane-a`, letting
the helper send `TabReady(pane-a, 42)`, receiving `BrowserReady` on the TUI
socket, then closing that TUI socket. The helper received `CloseTab(tab_id=42)`,
and fresh `QueryLast`/`QueryTabs` requests no longer reported the disconnected
pane.

## Conclusion

Ghostboard now performs the first useful browser-tab teardown on TUI disconnect:
disconnected panes are removed from GUI query state, ready browser tabs receive
`CloseTab`, and pre-ready panes are cleaned up without sending a premature
browser close. Browser process shutdown, native overlay cleanup, and broader
lifecycle cleanup remain separate future work.

## Completion Review

A fresh-context adversarial Codex subagent reviewed the completed Experiment 24
result and returned **APPROVED** with no required findings.

The reviewer had one optional finding: the no-stale-process check was summarized
in the experiment result, but the runtime harness log did not include the exact
command/result. The check was rerun and appended to
`logs/ghostboard-exp24-runtime-harness-20260616.log` with an explicit pass line
for `TermSurf.app/Contents/MacOS/termsurf`, `pre-helper.py`, and
`ready-helper.py`.
