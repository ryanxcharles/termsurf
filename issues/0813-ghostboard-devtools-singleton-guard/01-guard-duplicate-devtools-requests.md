# Experiment 1: Guard Duplicate DevTools Requests

## Description

Ghostboard already participates in the DevTools request/reply flow:

- webtui sends `QueryDevtoolsRequest` before opening a DevTools split;
- Ghostboard verifies that the inspected tab exists and sends
  `QueryDevtoolsReply`;
- webtui opens a split only after the query succeeds;
- the DevTools pane later sends `SetDevtoolsOverlay`, and Ghostboard records the
  pane's `inspected_tab_id`.

The missing invariant is the historical Issue 687 guard: one inspected browser
tab may have at most one DevTools frontend. The correct place to reject a
duplicate is Ghostboard's `QueryDevtoolsRequest` path, before webtui sends
`OpenSplit`. That gives immediate user-facing feedback and avoids creating a
second DevTools pane or asking Roamium/Chromium to create a second DevTools
frontend for the same inspected page.

This experiment will implement the query-time guard and add an end-to-end
regression scenario proving the full lifecycle:

- first DevTools open succeeds;
- second DevTools open for the same inspected tab is rejected before
  `OpenSplit`;
- closing the first DevTools clears the guard state;
- reopening DevTools for that inspected tab succeeds;
- a different browser tab is allowed to open its own DevTools while the first
  tab's DevTools remains open.

## Changes

Planned source changes:

- `ghostboard/src/apprt/termsurf.zig`
  - Extend `sendQueryDevtoolsReply` / `fillQueryDevtoolsSuccess` so a duplicate
    DevTools request returns a non-empty `QueryDevtoolsReply.error`.
  - Add a small DevTools guard state keyed by `profile`, `browser`, and
    `inspected_tab_id`. A successful `QueryDevtoolsRequest` reserves the target
    immediately, before webtui sends `OpenSplit`, so a rapid second query cannot
    slip through the gap before the first DevTools pane sends
    `SetDevtoolsOverlay`.
  - Treat both reserved targets and live DevTools panes as blocking. A helper
    should check for:
    - any live pane with the same `profile`, `browser`, and `inspected_tab_id`;
    - any in-flight reservation with the same key.
  - Keep the guard scoped by profile and browser so unrelated browser processes
    or profiles are not blocked by matching tab ids. Runtime will prove the
    distinct-tab branch; source review of the keyed helper will prove the
    profile/browser scoping unless a reliable same-tab-id multi-profile harness
    path is available.
  - Clear or reconcile guard state when:
    - `SetDevtoolsOverlay` arrives for the reserved target, converting the
      reservation into live pane state;
    - the DevTools pane closes or its TUI disconnects, releasing the live pane
      state through existing pane cleanup;
    - a reserved launch never reaches `SetDevtoolsOverlay`, using a bounded
      timeout so a failed split/open cannot permanently block the inspected tab.
  - Return a clear error such as
    `Tab {N} already has DevTools open in {browser}/{profile}`.
  - Keep normal target validation unchanged: nonexistent inspected tabs should
    still return the existing
    `Inspected tab {N} not found in {browser}/{profile}` style error.

- `scripts/ghostboard-geometry-matrix.sh`
  - Add a focused scenario such as `devtools-singleton-guard`.
  - Reuse the existing `devtools-split-geometry` flow for the first DevTools
    open.
  - Add a focused in-flight guard probe using the compositor socket directly or
    another deterministic harness path: send a first `QueryDevtoolsRequest` for
    a browser tab, do not send `SetDevtoolsOverlay`, immediately send a second
    `QueryDevtoolsRequest` for the same key, and verify the second reply is the
    duplicate-DevTools error. Then wait past the reservation timeout and verify
    a new query can succeed again.
  - Attempt `:devtools right` again while the first DevTools pane is still open
    and verify:
    - Ghostboard logs a second `QueryDevtoolsRequest` for the same inspected
      tab;
    - Ghostboard sends `QueryDevtoolsReply`;
    - webtui records or displays the duplicate error;
    - no `OpenSplit`, `SetDevtoolsOverlay`, or `CreateDevtoolsTab` occurs after
      that duplicate-query boundary.
  - Close the DevTools pane, repeat `:devtools right`, and verify a new
    `SetDevtoolsOverlay` / `CreateDevtoolsTab` succeeds for the same inspected
    browser tab.
  - Open a second native terminal tab with a browser, then open DevTools there
    while the first browser tab's DevTools exists, proving an unrelated
    inspected tab is not blocked.
  - Keep the existing `devtools-split-geometry` scenario behavior unchanged
    except for shared helper extraction if needed.

Planned issue-doc changes:

- Record the result, runtime log paths, broad build/test checks, reviewer
  verdict, and final conclusion in this experiment file.
- Update the Issue 813 README experiment status.

## Verification

Static/build checks:

1. `zig fmt ghostboard/src/apprt/termsurf.zig`.
2. `bash -n scripts/ghostboard-geometry-matrix.sh`.
3. `shellcheck scripts/ghostboard-geometry-matrix.sh` if available.
4. `cd ghostboard && zig build -Demit-macos-app=false`.
5. `cd ghostboard && macos/build.nu --scheme Ghostty --configuration Debug --action build`.
6. `cargo check -p web`.
7. `cargo check -p roamium`.
8. `git diff --check`.

Runtime checks:

1. Run `scripts/ghostboard-geometry-matrix.sh devtools-singleton-guard`.
2. Verify the first DevTools request for browser A succeeds and produces exactly
   one `OpenSplit`, one `SetDevtoolsOverlay`, and one `CreateDevtoolsTab` for
   browser A's inspected tab.
3. Verify the in-flight reservation guard independently: after a successful
   query but before any `SetDevtoolsOverlay`, a second query for the same
   `profile` + `browser` + inspected tab returns the duplicate-DevTools error
   and no split-related messages are possible from that second query.
4. Verify reservation timeout behavior: after an intentionally unregistered
   reservation expires, a fresh query for the same key succeeds again.
5. Verify the duplicate request for browser A returns a duplicate-DevTools error
   and produces no new split, no new DevTools overlay, and no new
   `CreateDevtoolsTab` after the duplicate boundary.
6. Close browser A's DevTools pane and verify Ghostboard sends `CloseTab` for
   the DevTools browser tab and removes the pane state.
7. Reopen DevTools for browser A and verify `SetDevtoolsOverlay` and
   `CreateDevtoolsTab` occur again for browser A's inspected tab.
8. Open browser B in a distinct native tab, open DevTools for browser B while
   browser A's DevTools is open, and verify the browser B request succeeds.
9. Verify all successful DevTools panes still receive CA context, resize, mouse,
   focus, and keyboard evidence at the same level as the existing
   `devtools-split-geometry` scenario.

Pass criteria:

- Duplicate DevTools for the same `profile` + `browser` + inspected tab is
  rejected before `OpenSplit`, including the in-flight interval after successful
  query and before `SetDevtoolsOverlay`.
- Failed or abandoned in-flight launches do not permanently reserve the target.
- Closing the existing DevTools pane clears the guard and allows reopening.
- A different inspected tab is not blocked.
- Source review confirms the guard key includes `profile` and `browser` so
  unrelated profiles/browsers are not blocked by matching tab ids.
- Existing DevTools split geometry/input coverage remains intact.
- The app and relevant Rust/Zig build checks pass, or any broad-test failures
  are explicitly classified with current evidence.

Partial criteria:

- The duplicate guard works, but the close/reopen or unrelated-tab runtime
  coverage exposes a separate pre-existing lifecycle limitation.
- The implementation works manually, but automation cannot reliably exercise one
  branch because of VM input/window-management instability.

Fail criteria:

- Duplicate DevTools still reaches `OpenSplit`, `SetDevtoolsOverlay`, or
  `CreateDevtoolsTab`.
- A rapid duplicate query succeeds during the in-flight interval before
  `SetDevtoolsOverlay`.
- An abandoned reservation permanently blocks DevTools for that inspected tab.
- Closing DevTools leaves stale guard state that prevents reopening.
- The guard blocks an unrelated inspected tab.
- The app no longer builds.

## Design Review

Fresh-context adversarial review by Codex subagent `Feynman`:

- **Verdict:** Changes required.
- **Required finding:** A live-pane-only scan would miss the in-flight interval
  after a successful `QueryDevtoolsRequest` and before `SetDevtoolsOverlay`,
  allowing a rapid duplicate to reach `OpenSplit`.
- **Required finding:** The runtime plan did not prove that in-flight duplicate
  case.
- **Optional finding:** Runtime unrelated-target proof covered a distinct tab
  but not a different profile/browser with a colliding tab id.
- **Resolution:** Revised the design to add immediate query-time reservations
  keyed by `profile` + `browser` + inspected tab, timeout cleanup for abandoned
  reservations, explicit in-flight runtime checks, and source-review proof for
  profile/browser scoping.
- **Re-review verdict:** Approved.
- **Re-review findings:** None.
