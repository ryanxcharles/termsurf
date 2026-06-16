# Experiment 23: Forward ModeChanged Focus to Browser

## Description

Experiments 20 through 22 established the normal browser process path:
`SetOverlay -> spawn browser -> ServerRegister -> CreateTab -> TabReady -> BrowserReady`,
plus resize forwarding for repeated overlay updates.

The next small parity step is the Wezboard-backed browsing-mode focus path.
`webtui` sends `ModeChanged(pane_id, browsing)` to the GUI when the user enters
or leaves browser mode. Wezboard updates pane browsing state, then forwards
`FocusChanged(tab_id, focused=browsing)` to the matched browser server when the
pane has a browser tab. Fresh Ghostboard currently ignores `ModeChanged`, so
Roamium is not told when the browser tab should gain or lose focus.

This experiment will implement only that state-backed
`ModeChanged -> FocusChanged` path. It intentionally leaves `Navigate` and
`SetColorScheme` forwarding out of scope. Current Wezboard only logs `Navigate`
in the GUI path, and `webtui` can also send `SetColorScheme` directly to the
browser after `BrowserReady`, so those require separate analysis to avoid
duplicating browser messages or claiming false parity.

This experiment will not implement direct browser-client routing,
browser-originated state forwarding, native overlay UI, CALayerHost
presentation, keyboard/mouse input forwarding, DevTools pane creation, split
creation, or changes to `webtui`, `roamium`, or the protocol schema.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - add an explicit decode branch for `ModeChanged`;
  - resolve the request by `pane_id` under `state_mutex`;
  - update the pane's `browsing` state from the message;
  - require the pane to have a nonzero `tab_id` before forwarding to a browser;
  - require the pane's matched server to have an attached browser fd;
  - snapshot the target browser fd, pane id, tab id, and focus value under
    `state_mutex`;
  - release `state_mutex` before writing the `FocusChanged` protobuf to the
    browser fd;
  - log successful forwards with pane id, tab id, and focus value;
  - keep `SetOverlay`, browser spawning, `CreateTab`, `TabReady`,
    `BrowserReady`, `Resize`, `Navigate`, `SetColorScheme`, native overlay
    presentation, navigation state updates, DevTools behavior, split behavior,
    and input forwarding otherwise unchanged.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, CLI install
behavior, DevTools pane creation, split creation, CALayerHost overlay
presentation, browser-originated state forwarding, navigation forwarding,
`SetColorScheme` forwarding, or input forwarding.

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
- Runtime harness launches `TermSurf.app`, connects to `TERMSURF_SOCKET`, and
  sends
  `SetOverlay(browser=/absolute/temp/helper, profile=default, pane_id=pane-a)`.
- The spawned helper connects back with `ServerRegister`, receives `CreateTab`,
  and sends `TabReady(pane-a, 42)`.
- Before `TabReady`, the harness sends
  `ModeChanged(pane_id=pane-a, browsing=true)` and verifies the helper receives
  no `FocusChanged`.
- Before any pane exists, the harness sends
  `ModeChanged(pane_id=unknown, browsing=true)` and verifies the app does not
  crash and no browser frame is delivered.
- After `TabReady`, the harness sends:
  - `ModeChanged(pane_id=pane-a, browsing=false)`;
  - `ModeChanged(pane_id=pane-a, browsing=true)`.
- The helper receives, in response:
  - `FocusChanged(tab_id=42, focused=false)`;
  - `FocusChanged(tab_id=42, focused=true)`.
- The TUI socket still receives `BrowserReady` with the real helper listen
  socket, proving Experiment 21 behavior still works.
- Existing repeated `SetOverlay` resize forwarding still works at least once in
  the harness, proving Experiment 22 behavior is not regressed.
- App logs include successful `FocusChanged` forward lines for both focus
  values.
- The runtime harness verifies shutdown cleanup removes the socket file and
  leaves no stale `TermSurf.app/Contents/MacOS/termsurf` or helper process.
- `git diff --check` is clean.

Fail criteria:

- `FocusChanged` is sent before `TabReady`.
- `FocusChanged` is sent for an unknown pane id.
- A forwarded `FocusChanged` is sent to the TUI socket instead of the browser
  server socket.
- A forwarded `FocusChanged` has the wrong tab id or focus value.
- Unknown pane ids or panes without attached browser servers cause crashes.
- Existing
  `SetOverlay -> spawn -> ServerRegister -> CreateTab -> TabReady -> BrowserReady`
  behavior regresses.
- Existing repeated `SetOverlay -> Resize` behavior regresses.
- The implementation forwards `Navigate` or `SetColorScheme`, adds direct
  browser-client routing, browser-originated state forwarding, CALayerHost
  overlay presentation, input forwarding, DevTools pane creation, split
  creation, or changes `webtui`, `roamium`, or the protocol schema in this
  experiment.

## Design Review

Fresh-context adversarial design review initially returned **CHANGES REQUIRED**
with two required findings and one optional finding.

Required findings accepted and fixed:

- The original design incorrectly claimed Wezboard forwards GUI-routed
  `Navigate`; it currently only logs that path. `Navigate` forwarding is now out
  of scope for this experiment.
- The original design proposed forwarding `SetColorScheme`, but `webtui` can
  also send that message directly to the browser after `BrowserReady`, and
  Wezboard does not forward it in the GUI path. `SetColorScheme` forwarding is
  now out of scope for this experiment.

Optional finding accepted and fixed: the verification now explicitly exercises
an unknown `pane_id` and a known pane before `TabReady`, verifying no
`FocusChanged` is delivered and the app does not crash.

Fresh-context adversarial re-review returned **APPROVED**. The reviewer
confirmed the `Navigate` and `SetColorScheme` findings are resolved by narrowing
the experiment to `ModeChanged -> FocusChanged`, and confirmed the verification
now includes the unknown-pane and pre-`TabReady` no-forward checks. No new
required findings were introduced by the fix.
