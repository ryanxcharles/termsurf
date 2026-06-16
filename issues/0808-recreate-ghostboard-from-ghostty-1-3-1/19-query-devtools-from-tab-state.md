# Experiment 19: Reply To QueryDevtools From Tab State

## Description

Experiment 11 added the validation and no-state `QueryDevtoolsRequest` reply.
Experiments 14 through 18 now maintain enough normal-tab state to answer the
positive lookup case: `SetOverlay` records pane profile/browser metadata, and
`TabReady` records a `profile/browser/tab_id -> pane_id` lookup.

The next smallest DevTools parity step is to make `QueryDevtoolsRequest` resolve
an existing inspected normal tab from that lookup, matching the successful
branch of Wezboard's behavior:

- keep the existing validation errors for missing browser, missing profile, and
  missing inspected tab id;
- for a nonzero inspected tab id, look up the request's
  `profile/browser/inspected_tab_id`;
- if the lookup exists, return `tab_id`, `browser`, `profile`, and empty
  `error`;
- if the lookup does not exist, keep returning
  `Inspected tab {id} not found in {browser}/{profile}`.

This experiment intentionally does not implement `SetDevtoolsOverlay`,
`CreateDevtoolsTab`, DevTools-pane state, duplicate DevTools detection,
`BrowserReady`, browser launch, overlay presentation, navigation, or input
forwarding. The Wezboard branches for "already has DevTools open" and "Cannot
open DevTools for a DevTools pane" require DevTools pane state that Ghostboard
does not track yet, so they remain out of scope.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - update `sendQueryDevtoolsReply` to consult the `tab_lookups` state after
    existing validation succeeds;
  - read `tab_lookups` only while holding `state_mutex`, matching the mutation
    discipline used by `TabReady`;
  - copy any reply string fields needed from shared state into local
    null-terminated buffers before sending the protobuf frame;
  - when the requested `profile/browser/inspected_tab_id` exists, populate
    `QueryDevtoolsReply.tab_id`, `browser`, and `profile`, with empty `error`;
  - keep the existing validation errors and not-found error string for all
    failure cases;
  - avoid adding DevTools pane state or browser/overlay behavior.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, app config paths, icon assets, Xcode project files, CLI install
behavior, browser process launch, `BrowserReady`, overlay presentation,
`SetDevtoolsOverlay`, `CreateDevtoolsTab`, or input forwarding.

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
  - missing `browser` still returns
    `error = "DevTools target browser is required"`;
  - missing `profile` still returns
    `error = "DevTools target profile is required"`;
  - `inspected_tab_id = 0` still returns
    `error = "DevTools target tab id is required"`;
  - before `TabReady`, a valid-looking request still returns
    `error = "Inspected tab 42 not found in roamium/default"`;
  - after `SetOverlay -> ServerRegister -> CreateTab -> TabReady(pane-a, 42)`,
    `QueryDevtoolsRequest(browser=roamium, profile=default, inspected_tab_id=42)`
    returns `tab_id = 42`, `browser = "roamium"`, `profile = "default"`, and
    empty `error`;
  - a mismatched profile or browser still returns the not-found error for that
    request key;
  - no `BrowserReady`, browser process launch, `CreateDevtoolsTab`, or overlay
    presentation logs are emitted by this experiment.
- The runtime harness also sends a normal TUI `HelloRequest` on a fresh socket
  and receives `HelloReply`, proving existing request/reply behavior still
  works.
- The harness verifies shutdown cleanup still removes the socket file and leaves
  no stale `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- `QueryDevtoolsRequest` still ignores existing `tab_lookups` state.
- The successful reply has the wrong `tab_id`, `browser`, `profile`, or nonempty
  `error`.
- Existing validation error strings regress.
- Missing profile/browser combinations incorrectly resolve a tab.
- The implementation adds DevTools pane state, sends `CreateDevtoolsTab`, sends
  `BrowserReady`, launches a browser process, or creates overlay UI in this
  experiment.
- Browser/TUI classification or the synchronous request/reply paths from
  Experiments 8 through 18 regress.
- Any `webtui`, `roamium`, protocol schema, app branding, config path, icon, or
  CLI install behavior changes are needed for this experiment.

## Design Review

Fresh-context adversarial design review initially returned **CHANGES REQUIRED**
with one required finding: the design did not explicitly require reading the
shared `tab_lookups` state under `state_mutex` or copying reply fields before
sending.

Required finding accepted and fixed: the design now requires
`sendQueryDevtoolsReply` to read `tab_lookups` only while holding `state_mutex`
and to copy reply string fields into local null-terminated buffers before
sending the protobuf frame.

Fresh-context adversarial re-review returned **APPROVED**. The reviewer
confirmed the required finding was resolved and that the fix introduced no new
required issues.
