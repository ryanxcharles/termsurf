# Experiment 14: Track Pending Overlay Servers

## Description

Experiment 13 made `ServerRegister` explicit, but because Ghostboard has no
pending server records yet every browser registration correctly reports
`no matching server`. The next step is to create the minimal in-memory state
that Wezboard has before a browser process registers.

In Wezboard, `SetOverlay` creates pane state and a server record keyed by
`profile + browser`. Later, `ServerRegister` matches a browser connection to a
server record with the same profile whose transport is not attached yet.

This experiment will add the same state transition in Ghostboard without
launching a browser process and without sending `CreateTab` yet:

1. A TUI sends `SetOverlay`.
2. Ghostboard records pane metadata from the overlay.
3. Ghostboard creates or reuses a pending server record for `profile + browser`.
4. A browser-classified socket sends `ServerRegister`.
5. Ghostboard matches that registration to the pending server by profile and
   marks the server as attached.

This is the smallest useful server-registry step after Experiment 13. It proves
that Ghostboard can remember TUI overlay intent and later associate a browser
connection with it, while keeping browser process launch, `CreateTab`,
`TabReady`, `BrowserReady`, CALayerHost presentation, and input forwarding out
of scope.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - add small process-local state for panes and pending browser servers;
  - protect that state with a mutex;
  - add an explicit `SetOverlay` branch in `handleClient`;
  - default an empty `SetOverlay.browser` to `roamium`, matching Wezboard;
  - store the overlay's `pane_id`, `profile`, `browser`, `url`, terminal-cell
    geometry, and browsing flag;
  - if `pane_id` already exists, update that pane's overlay metadata without
    creating a new pane and without incrementing server `pane_count`, matching
    Wezboard's resize/update path;
  - if `pane_id` is new, create a pending server record when `profile + browser`
    is new, or increment the existing server's `pane_count` only when the new
    pane attaches to an already-existing server;
  - update `ServerRegister` handling so a matching pending server is marked
    attached and logs the matched server key;
  - keep the Experiment 13 unmatched warning for registrations that still have
    no pending server.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, CLI install
behavior, browser launch, `CreateTab`, `TabReady`, `BrowserReady`, overlay
presentation, or input forwarding.

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
  - a TUI `SetOverlay` with `pane_id=pane-a`, `profile=default`, empty browser,
    and a URL logs `SetOverlay: pane_id=pane-a profile=default browser=roamium`;
  - that `SetOverlay` creates a pending server record for
    `profile=default browser=roamium`;
  - a second `SetOverlay` for the same `pane_id=pane-a` logs an update path,
    does not create a second pane, and does not increment server `pane_count`;
  - a `SetOverlay` for a new `pane_id=pane-b` with the same profile and browser
    reuses the pending server and increments `pane_count`;
  - a later browser-classified `ServerRegister(profile=default)` logs
    `ServerRegister: matched server key=default/roamium`;
  - that matched registration does not log
    `ServerRegister: no matching server for profile=default`;
  - a standalone `ServerRegister(profile=other)` still logs the unmatched
    warning;
  - no `CreateTab`, `BrowserReady`, `TabReady`, or overlay presentation logs are
    emitted by this experiment.
- The runtime harness also sends a normal TUI `HelloRequest` on a fresh socket
  and receives `HelloReply`, proving the new state path did not break existing
  request/reply behavior.
- The harness verifies shutdown cleanup still removes the socket file and leaves
  no stale `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- `SetOverlay` is still handled only by the generic ignored-message branch.
- Empty `SetOverlay.browser` does not default to `roamium`.
- `ServerRegister(profile=default)` cannot match a prior pending server for the
  same profile.
- A duplicate `SetOverlay` for the same `pane_id` creates a duplicate pane or
  increments `pane_count`.
- The implementation sends `CreateTab`, launches a browser process, or creates
  overlay UI in this experiment.
- Browser/TUI classification or the synchronous request/reply paths from
  Experiments 8 through 13 regress.
- Any `webtui`, `roamium`, protocol schema, app branding, config path, icon, or
  CLI install behavior changes are needed for this experiment.

## Design Review

A fresh-context adversarial design review returned **CHANGES REQUIRED**.

Required finding accepted and fixed: the original design did not distinguish
between a duplicate `SetOverlay` for an existing `pane_id` and a new pane that
reuses an existing server. That could overcount panes and diverge from
Wezboard's resize/update behavior. The design now requires existing `pane_id`
messages to update pane metadata without creating a new pane or incrementing
`pane_count`, and it adds runtime pass/fail checks for duplicate `SetOverlay`.

Optional finding accepted and fixed: the design now lists the exact native
GhosttyKit and macOS app build commands in the verification section.

Fresh-context adversarial re-review returned **APPROVED**. The reviewer
confirmed that the duplicate-pane update path, duplicate `SetOverlay` runtime
checks, duplicate-pane fail criterion, and exact build commands resolve the
prior findings without introducing new required issues.

## Result

**Result:** Pass

Implemented bounded process-local pane/server state in
`ghostboard/src/apprt/termsurf.zig`.

The socket handler now:

- tracks pane metadata from `SetOverlay`;
- defaults an empty `SetOverlay.browser` to `roamium`;
- creates a pending server record for a new `profile/browser` pair;
- updates an existing `pane_id` without creating a duplicate pane or
  incrementing server `pane_count`;
- increments `pane_count` when a new pane reuses an existing pending server;
- matches `ServerRegister(profile=...)` to the first unattached pending server
  for that profile;
- keeps unmatched `ServerRegister` warnings for profiles with no pending server;
- still does not launch a browser process, send `CreateTab`, send
  `BrowserReady`, handle `TabReady`, present overlays, or forward input.

Verification passed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp14-zig-native-xcframework-20260616-101940.log`.
- macOS app build passed:
  `logs/ghostboard-exp14-macos-build-debug-20260616-102001.log`.
- Runtime harness passed:
  `logs/ghostboard-exp14-runtime-harness-20260616-102101.log`.
- Runtime app log: `logs/ghostboard-exp14-runtime-app-20260616-102101.log`.
- `git diff --check` passed.

The first native build attempt failed because the `handleServerRegister` helper
was updated to store the accepted socket fd but its function signature had not
yet been updated to receive that fd. I fixed the signature and reran the build
successfully.

Observed successful runtime checks:

```text
PASS: child wrote TERMSURF_SOCKET
PASS: socket path is under TMPDIR/termsurf
PASS: socket exists while app is running
PASS: app log contains TermSurf socket listening
PASS: socket fd=11 classified exactly once as Tui
PASS: SetOverlay pane-a defaulted browser to roamium
PASS: SetOverlay created pending server
PASS: duplicate SetOverlay updated pane without pane_count increment
PASS: new pane reused pending server and incremented pane_count
PASS: socket fd=11 classified exactly once as Browser
PASS: ServerRegister matched pending server
PASS: matched ServerRegister did not log unmatched warning
PASS: socket fd=11 classified exactly once as Browser
PASS: unmatched ServerRegister still warns
PASS: no CreateTab emitted
PASS: no BrowserReady emitted
PASS: no TabReady emitted
PASS: no overlay presentation messages emitted
PASS: fresh TUI client received HelloReply
PASS: socket fd=11 classified exactly once as Tui
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale TermSurf process remains
runtime verification passed
```

The app log shows the core state transitions:

```text
info(termsurf): SetOverlay: pane_id=pane-a profile=default browser=roamium url=https://example.com
info(termsurf): SetOverlay: created pending server key=default/roamium pane_count=1
info(termsurf): SetOverlay: pane_id=pane-a profile=default browser=roamium url=https://example.org
info(termsurf): SetOverlay: updated pane_id=pane-a profile=default browser=roamium pane_count=1
info(termsurf): SetOverlay: pane_id=pane-b profile=default browser=roamium url=https://example.net
info(termsurf): SetOverlay: reused pending server key=default/roamium pane_count=2 has_fd=false
info(termsurf): ServerRegister: profile=default
info(termsurf): ServerRegister: matched server key=default/roamium
info(termsurf): ServerRegister: profile=other
warning(termsurf): ServerRegister: no matching server for profile=other
```

## Result Review

Fresh-context adversarial result review returned **APPROVED** with no findings.

The reviewer confirmed:

- the diff is limited to the expected source and issue documentation files;
- `SetOverlay` and `ServerRegister` behavior matches the approved experiment
  scope;
- no browser launch, `CreateTab`, `BrowserReady`, or overlay UI path was
  introduced;
- existing classification and `HelloRequest` reply behavior are preserved;
- the build and runtime logs support the recorded result;
- the result commit had not been made before review.

## Conclusion

Ghostboard now has the minimal state needed to connect TUI overlay intent with a
later browser-engine registration. `SetOverlay` creates or updates pane state
and pending server records, and `ServerRegister` can attach to that pending
server instead of always warning that no match exists.

The next experiment can build on this state to introduce browser process launch
or `CreateTab` delivery, while preserving the verified duplicate-pane update
behavior.
