# Experiment 15: Flush Pending CreateTab

## Description

Experiment 14 created pending pane/server state and matched
`ServerRegister(profile=...)` to an unattached pending server. The next Wezboard
behavior is to flush pending browser tabs to that registered browser connection.

In Wezboard, `handle_server_register` finds a matching server, stores the
browser connection sender, then sends `CreateTab` for each pending non-DevTools
pane whose `profile/browser` matches and whose `tab_id` is still zero.

Ghostboard does not yet launch Roamium. This experiment will still move the real
wire protocol forward by treating the runtime harness as the browser-engine
socket:

1. A TUI socket sends `SetOverlay`.
2. Ghostboard records a pending pane/server.
3. A browser-classified socket sends `ServerRegister`.
4. Ghostboard matches that socket and writes a length-prefixed `CreateTab`
   protobuf frame back to the browser socket.

This experiment must not launch a browser process, handle `TabReady`, send
`BrowserReady`, create overlay UI, or forward input. It only proves that
Ghostboard can deliver the correct browser-directed tab creation message once a
browser connection is attached.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - add `CreateTab` to the message type name helper;
  - add `tab_id: i64 = 0` to `PaneState` so Ghostboard can distinguish pending
    panes from panes that will later be assigned a browser tab id by `TabReady`;
  - add a helper that sends a length-prefixed `CreateTab` for one pending pane;
  - after `ServerRegister` matches a pending server, iterate matching panes and
    send `CreateTab` for each pane with `tab_id == 0`;
  - derive `CreateTab.pixel_width` and `CreateTab.pixel_height` from
    `SetOverlay.width` and `SetOverlay.height` using Wezboard's fallback cell
    size of `10x20` pixels because Ghostboard does not yet expose live terminal
    cell metrics to this socket module;
  - keep the pane pending after sending, because `TabReady` is not implemented
    yet and will be responsible for assigning `tab_id`.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, CLI install
behavior, browser process launch, `TabReady`, `BrowserReady`, overlay
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
  - two TUI `SetOverlay` messages create two pending panes for
    `default/roamium`;
  - one additional TUI `SetOverlay` creates a nonmatching pending pane for a
    different profile or browser;
  - a later browser-classified `ServerRegister(profile=default)` receives a
    length-prefixed `CreateTab` frame for each matching pending pane on that
    browser socket;
  - exactly two `CreateTab` frames are received for the two `default/roamium`
    panes, and the nonmatching pane is not flushed to that socket;
  - each decoded `CreateTab` has the original `url`, `pane_id`, fallback
    `pixel_width = width * 10`, fallback `pixel_height = height * 20`, and
    `dark = false`;
  - the app log contains one `sent CreateTab: pane_id=... url=...` entry for
    each flushed matching pane;
  - no `BrowserReady`, `TabReady`, browser process launch, or overlay
    presentation logs are emitted by this experiment.
- The runtime harness also sends a normal TUI `HelloRequest` on a fresh socket
  and receives `HelloReply`, proving existing request/reply behavior still
  works.
- The harness verifies shutdown cleanup still removes the socket file and leaves
  no stale `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- `ServerRegister` matches but no `CreateTab` frame is sent.
- Fewer or more `CreateTab` frames are flushed than the number of matching
  pending panes.
- A nonmatching pending pane is flushed to the registered browser socket.
- Any `CreateTab` has the wrong oneof type or wrong `pane_id`, `url`,
  dimensions, or dark flag.
- `CreateTab` is sent before `ServerRegister` attaches the browser socket.
- The implementation launches a browser process, sends `BrowserReady`, handles
  `TabReady`, or creates overlay UI in this experiment.
- Browser/TUI classification or the synchronous request/reply paths from
  Experiments 8 through 14 regress.
- Any `webtui`, `roamium`, protocol schema, app branding, config path, icon, or
  CLI install behavior changes are needed for this experiment.

## Design Review

A fresh-context adversarial design review returned **CHANGES REQUIRED**.

Required finding accepted and fixed: the original plan referenced Wezboard's
`tab_id == 0` pending-pane filter but did not plan a `tab_id` field in
Ghostboard's `PaneState`. The plan now explicitly adds `tab_id: i64 = 0`, with
`TabReady` left out of scope except as the future owner of assigning nonzero tab
ids.

Required finding accepted and fixed: the original runtime proof only required
one pending pane and one `CreateTab`, which would not prove Wezboard's
flush-all-matching-pending-panes behavior. The plan now requires two matching
pending panes, one nonmatching pending pane, exactly two decoded `CreateTab`
frames for the matching panes, and proof that the nonmatching pane is not
flushed to that browser socket.

Fresh-context adversarial re-review returned **APPROVED**. The reviewer
confirmed both required findings were resolved and that the fixes introduced no
new required issues.

## Result

**Result:** Pass

Implemented pending `CreateTab` flushing in `ghostboard/src/apprt/termsurf.zig`.

The socket handler now:

- tracks `tab_id: i64 = 0` on pane state so pending panes can be identified;
- sends `CreateTab` frames after `ServerRegister` matches an unattached pending
  server;
- flushes every pending pane with matching `profile/browser` and `tab_id == 0`;
- excludes pending panes for other profiles or browsers;
- encodes `CreateTab.url`, `pane_id`, fallback `pixel_width`, fallback
  `pixel_height`, and `dark = false`;
- keeps browser launch, `TabReady`, `BrowserReady`, overlay UI, and input
  forwarding out of scope.

Verification passed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp15-zig-native-xcframework-20260616-102953.log`.
- macOS app build passed:
  `logs/ghostboard-exp15-macos-build-debug-20260616-103014.log`.
- Runtime harness passed:
  `logs/ghostboard-exp15-runtime-harness-20260616-103130.log`.
- Runtime app log: `logs/ghostboard-exp15-runtime-app-20260616-103130.log`.
- `git diff --check` passed.

Observed successful runtime checks:

```text
PASS: default pending server created
PASS: second matching pane reused default server
PASS: nonmatching pending server created
PASS: browser socket received exactly two CreateTab frames
PASS: CreateTab frames were for matching panes only
PASS: pane-a CreateTab url matches
PASS: pane-a CreateTab fallback pixel size matches
PASS: pane-a CreateTab dark is false
PASS: pane-b CreateTab url matches
PASS: pane-b CreateTab fallback pixel size matches
PASS: pane-b CreateTab dark is false
PASS: nonmatching pane was not flushed
PASS: app log contains pane-a CreateTab send
PASS: app log contains pane-b CreateTab send
PASS: app log does not contain nonmatching CreateTab send
PASS: no BrowserReady emitted
PASS: no TabReady emitted
PASS: no overlay presentation message emitted
PASS: fresh TUI client received HelloReply
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale TermSurf process remains
runtime verification passed
```

The runtime harness decoded the browser socket frames and verified:

- `pane-a` received `url=https://a.example`, `pixel_width=800`,
  `pixel_height=480`, and `dark=false`;
- `pane-b` received `url=https://b.example`, `pixel_width=1000`,
  `pixel_height=600`, and `dark=false`;
- `pane-other` was not flushed to the `default/roamium` browser socket.

The app log also records the flush:

```text
info(termsurf): sent CreateTab: pane_id=pane-a url=https://a.example
info(termsurf): sent CreateTab: pane_id=pane-b url=https://b.example
```

## Result Review

Fresh-context adversarial result review returned **APPROVED** with no required,
optional, or nit findings.

The reviewer confirmed:

- scope stayed within `ghostboard/src/apprt/termsurf.zig` plus issue docs;
- no browser launch, `BrowserReady`, `TabReady`, or overlay UI was added;
- `ServerRegister` flushes `CreateTab` only for matching pending panes with
  `tab_id == 0`;
- pending `tab_id` state is present and future `TabReady` remains out of scope;
- existing `HelloRequest` replies and first-message connection classification
  are preserved;
- the native build, macOS app build, runtime harness, decoded frame checks, and
  absence of `BrowserReady`/`TabReady`/overlay presentation logs support the
  result;
- the result commit had not been made before review.

## Conclusion

Ghostboard can now deliver browser-directed `CreateTab` messages to an attached
browser socket for every pending pane that matches the registered
`profile/browser` server. This proves the GUI can move from TUI overlay intent
to browser tab creation over the current TermSurf protobuf wire format without
modifying `webtui`, `roamium`, or the schema.

The next experiment can implement `TabReady` handling so the browser can assign
tab ids back to the pending pane state.
