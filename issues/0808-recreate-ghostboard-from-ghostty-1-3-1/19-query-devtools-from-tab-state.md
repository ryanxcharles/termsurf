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

## Result

**Result:** Pass

Implemented state-backed success replies for `QueryDevtoolsRequest` in
`ghostboard/src/apprt/termsurf.zig`.

The socket handler now keeps the existing validation path, then looks up the
requested `profile/browser/inspected_tab_id` in `tab_lookups` under
`state_mutex`. On success it copies `profile` and `browser` into local
null-terminated buffers, fills `QueryDevtoolsReply.tab_id`, `browser`, and
`profile`, and sends an empty `error`. Missing lookup keys keep the existing
`Inspected tab {id} not found in {browser}/{profile}` error.

Verification passed:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp19-zig-native-xcframework-20260616-110820.log`.
- macOS app build passed:
  `logs/ghostboard-exp19-macos-build-debug-20260616-110840.log`.
- Runtime harness passed:
  `logs/ghostboard-exp19-runtime-harness-20260616-110943.log`.
- Runtime app log: `logs/ghostboard-exp19-runtime-app-20260616-110943.log`.
- `git diff --check` passed.

Observed successful runtime checks:

```text
PASS: QueryDevtools missing browser validation
PASS: QueryDevtools missing profile validation
PASS: QueryDevtools missing tab id validation
PASS: QueryDevtools before TabReady returns not found
PASS: browser socket received pane-a CreateTab
PASS: QueryDevtools finds TabReady tab
PASS: QueryDevtools mismatched profile returns not found
PASS: QueryDevtools mismatched browser returns not found
PASS: fresh TUI client received HelloReply
PASS: app exited after SIGTERM
PASS: socket file removed after shutdown
PASS: no stale TermSurf process remains
PASS: app log contains TermSurf socket listening
PASS: app log contains QueryDevtoolsReply sends
PASS: app log contains TabReady lookup
PASS: no BrowserReady emitted
PASS: no CaContext emitted
PASS: no CreateDevtoolsTab emitted
PASS: no overlay presentation message emitted
PASS: no browser launch message emitted
runtime verification passed
```

The passing harness verified that
`QueryDevtoolsRequest(browser=roamium, profile=default, inspected_tab_id=42)`
returns `tab_id = 42`, `browser = "roamium"`, `profile = "default"`, and empty
`error` after `SetOverlay -> ServerRegister -> CreateTab -> TabReady`.

## Conclusion

Ghostboard now answers the normal-tab `QueryDevtoolsRequest` success path from
the `TabReady` lookup state. DevTools pane creation, duplicate DevTools
detection, `CreateDevtoolsTab`, `BrowserReady`, browser launch, CALayerHost
overlay presentation, and input forwarding remain for later experiments.

## Result Review

Fresh-context adversarial result review returned **APPROVED** with no required,
optional, or nit findings.

The reviewer confirmed:

- the implementation matches the approved scope;
- the diff is limited to `ghostboard/src/apprt/termsurf.zig`, this experiment
  file, and the issue README;
- `QueryDevtoolsRequest` behavior matches the currently implementable Wezboard
  success branch;
- shared `tab_lookups` state is read under `state_mutex`;
- success reply fields are copied into local buffers before send;
- verification logs prove the validation, not-found, success, mismatch, and
  negative-scope checks;
- the README status matches the result;
- the result commit had not been made before review.
