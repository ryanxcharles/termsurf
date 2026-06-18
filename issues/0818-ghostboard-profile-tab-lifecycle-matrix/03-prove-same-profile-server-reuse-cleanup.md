# Experiment 3: Prove Same-Profile Server Reuse and Cleanup

## Description

Experiment 1 left warm reconnect, server reuse, normal browser close/reopen, and
whole profile-server process cleanup only partially covered or uncovered.
Experiment 2 proved that different profiles isolate correctly by spawning
separate Roamium processes. The next lifecycle risk is the opposite case:
multiple browser panes for the same profile and same browser should share one
profile server while any pane is alive, then clean up the server process when
the last pane closes.

This experiment will add a focused runtime scenario for same-profile lifecycle
behavior. It should open browser A, open browser B with the same profile/browser
in another native tab, prove that Ghostboard reuses the existing server instead
of spawning a second Roamium process, prove both browser tabs route input
independently, close browser B, reopen browser C on the same profile/server,
prove browser A and C still work through that server without resurrecting B,
then close all remaining browsers and prove the final profile-server process
cleanup behavior with explicit pid evidence.

## Changes

Planned harness changes:

- `scripts/ghostboard-geometry-matrix.sh`
  - Add a `same-profile-server-lifecycle` scenario.
  - Reuse the existing native-tab automation shape from
    `open-browser-in-new-tab` and `close-browser-tab`.
  - Launch browser A with `web --browser "$ROAMIUM" --profile default "$URL_A"`.
  - Open a second native tab and launch browser B with
    `web --browser "$ROAMIUM" --profile default "$URL_B"`.
  - Capture the first `spawned browser path=${ROAMIUM}` pid for the
    `default/${ROAMIUM}` server.
  - Assert browser B logs
    `SetOverlay: reused pending server key=default/${ROAMIUM}` with an increased
    pane count and does not log a second `spawned browser path=${ROAMIUM}` for
    the default profile.
  - Assert browser A and browser B have distinct pane ids, browser tab ids, and
    CA context ids while sharing the same Roamium pid/server.
  - Prove mouse and keyboard routing for browser B, then switch back and prove
    mouse and keyboard routing for browser A.
  - Close browser B and assert `CloseTab` reaches Roamium for browser B, browser
    A remains alive, and no additional Roamium process is spawned.
  - Reopen browser C with the same profile and browser, assert it reuses the
    same `default/${ROAMIUM}` server/pid, gets a fresh pane id, browser tab id,
    and CA context id, and does not resurrect browser B's closed tab.
  - Prove mouse and keyboard routing for browser C, then switch back and prove
    browser A remains interactive through the same shared server.
  - Close browser C and browser A, then assert the final server-process cleanup
    behavior using pid evidence. The expected final behavior is that the profile
    server exits or is intentionally shut down after the last pane for that
    server closes.

Planned issue-document changes:

- Record the result in this experiment file.
- Update the Issue 818 README status for Experiment 3 after verification.

Planned app source changes:

- None in the initial proof. If the scenario proves that the last-pane cleanup
  behavior is missing, record the experiment as `Partial` or `Fail` with the
  exact evidence and make the fix a later design-reviewed experiment.

## Verification

Formatting actions:

1. `prettier --write --prose-wrap always --print-width 80 issues/0818-ghostboard-profile-tab-lifecycle-matrix/README.md issues/0818-ghostboard-profile-tab-lifecycle-matrix/03-prove-same-profile-server-reuse-cleanup.md`.

Static checks:

1. `git diff --check`.
2. `bash -n scripts/ghostboard-geometry-matrix.sh`.

Runtime checks:

1. `scripts/ghostboard-geometry-matrix.sh same-profile-server-lifecycle`.

Pass criteria:

- Browser A launches successfully and creates the `default/${ROAMIUM}` server.
- Browser B launches successfully with the same profile and same absolute
  Roamium path.
- Browser B reuses the existing `default/${ROAMIUM}` server instead of creating
  another server or spawning a second Roamium process.
- Browser A and browser B have distinct pane ids, browser tab ids, CA context
  ids, and native tab ids while sharing the same Roamium profile-server pid.
- Browser A receives keyboard/mouse input only when browser A is active.
- Browser B receives keyboard/mouse input only when browser B is active.
- Closing browser B sends `CloseTab` to Roamium for browser B and does not kill
  or respawn the shared server while browser A is still alive.
- Browser A remains interactive after browser B closes.
- Reopening browser C after browser B closes reuses the same
  `default/${ROAMIUM}` server and the same Roamium pid.
- Browser C gets a fresh pane id, browser tab id, CA context id, and native tab
  id rather than reusing browser B's closed identity.
- Browser C receives keyboard/mouse input only when browser C is active.
- Browser A remains interactive after browser C opens and closes.
- Closing browser C and then browser A, leaving no panes using the shared
  server, produces explicit final server cleanup evidence: the Roamium pid exits
  or Ghostboard sends an intentional shutdown and removes the server state.
- No stale closed browser tab receives input after its close.

Partial criteria:

- Same-profile server reuse and routing pass, but final server-process cleanup
  remains missing or cannot be proven with the current logs.
- Same-profile server reuse, routing, close, and reopen pass, but final
  server-process cleanup remains missing or cannot be proven with the current
  logs.
- The final server process remains alive after the last pane closes, but all
  tab-level cleanup and routing assertions pass.
- The scenario exposes a distinct server lifecycle bug that should be fixed in
  the next experiment.

Fail criteria:

- Browser B spawns a second same-profile Roamium process instead of reusing the
  existing server.
- Browser A or browser B cannot launch.
- Browser A and browser B reuse the same pane id, browser tab id, or CA context
  id.
- Keyboard or mouse input leaks between browser A and browser B.
- Closing browser B kills the shared server while browser A is still alive.
- Closed browser tabs continue receiving input.

## Design Review

This experiment is plan-only until a fresh-context adversarial design review
approves it. Record the reviewer verdict here, fix all real findings, and commit
the approved plan before implementation begins.

Fresh-context adversarial design review by Codex subagent `Herschel the 2nd`:

- **Initial verdict:** Changes required.
- **Required finding:** The original plan closed browser B, proved browser A
  remained alive, and then closed browser A, but did not prove normal browser
  close/reopen behavior. Fixed by adding a browser C reopen step after browser B
  closes. The plan now requires browser C to reuse the same server/pid, receive
  fresh pane/tab/context/native-tab identities, route input independently, and
  not resurrect browser B's closed tab.
- **Final verdict:** Approved. The reviewer confirmed the prior Required finding
  was resolved and no new Required finding was introduced.

## Completion Gate

After implementation and verification:

- add `## Result` and `## Conclusion` to this experiment file;
- update the Issue 818 README experiment status from `Designed` to `Pass`,
  `Partial`, or `Fail`;
- request a fresh-context completion review;
- fix all real completion-review findings and record the final verdict in this
  file; and
- commit the reviewed result separately before designing or implementing the
  next experiment.
