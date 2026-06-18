# Experiment 1: Establish Lifecycle Baseline

## Description

Issue 818 needs a focused profile, tab, pane, DevTools, reconnect, close/reopen,
and process-cleanup matrix before making source changes. Existing Ghostboard
geometry and state scenarios already cover several lifecycle-adjacent rows, but
the coverage is spread across prior issues and has not been mapped against this
issue's requirements.

This experiment will run a compact baseline, record the evidence, classify every
requested lifecycle behavior as `Covered`, `Partially covered`, or `Uncovered`,
and recommend the smallest next experiment based on the weakest row. It is a
documentation and verification experiment first; it should not make app source
changes.

## Changes

Planned issue-document changes:

- Add a lifecycle baseline matrix to this experiment file.
- Update the Issue 818 README experiment index after the result is known.

Planned harness changes:

- None expected.
- If existing logs are insufficient to classify a row, first record the missing
  signal in the result. Only add assertion-only harness changes to
  `scripts/ghostboard-geometry-matrix.sh` if they do not change app behavior,
  and record exactly why the existing evidence was insufficient before making
  them.

Planned source changes:

- None.

## Verification

Formatting actions:

1. `prettier --write --prose-wrap always --print-width 80 issues/0818-ghostboard-profile-tab-lifecycle-matrix/README.md issues/0818-ghostboard-profile-tab-lifecycle-matrix/01-establish-lifecycle-baseline.md`.

Static checks:

1. `git diff --check`.
2. If the harness changes, run `bash -n scripts/ghostboard-geometry-matrix.sh`.

Runtime checks:

Run this compact baseline:

1. `scripts/ghostboard-geometry-matrix.sh open-browser-in-new-tab`.
2. `scripts/ghostboard-geometry-matrix.sh close-browser-tab`.
3. `scripts/ghostboard-geometry-matrix.sh split-right-close-browser-pane`.
4. `scripts/ghostboard-geometry-matrix.sh open-browser-in-new-window`.
5. `scripts/ghostboard-geometry-matrix.sh multiple-windows-with-browsers`.
6. `scripts/ghostboard-geometry-matrix.sh devtools-singleton-guard`.

Reference prior evidence without rerunning it when it is enough for baseline
classification:

- Issue 816 Experiment 5:
  `issues/0816-ghostboard-browser-state-interruptions/05-prove-renderer-crash-recovery.md`
  for same-tab renderer crash recovery.
- Issue 814 Experiment 1:
  `issues/0814-ghostboard-launch-discovery-workflow/01-resolve-named-roamium-debug-launch.md`
  for browser launch discovery and failed-spawn cleanup.
- Issue 809 Experiment 10:
  `issues/0809-ghostboard-viewport-geometry/10-close-browser-pane.md` for
  browser-pane cleanup boundaries.
- Issue 809 Experiments 13 through 16:
  `issues/0809-ghostboard-viewport-geometry/13-open-browser-in-new-tab.md`,
  `issues/0809-ghostboard-viewport-geometry/14-close-browser-tab.md`,
  `issues/0809-ghostboard-viewport-geometry/15-open-browser-in-new-window.md`,
  and
  `issues/0809-ghostboard-viewport-geometry/16-multiple-windows-with-browsers.md`
  for native tab/window routing baselines.

Pass criteria:

- The experiment records pass/fail evidence for each runtime check that was run.
- The matrix maps every Issue 818 requested behavior:
  - multi-profile isolation;
  - multi-pane routing;
  - multi-tab routing;
  - warm reconnect;
  - server reuse;
  - close/reopen behavior;
  - stale process cleanup;
  - DevTools target lookup;
  - profile display or user-visible profile identity.
- Each row has an evidence pointer and a concrete next action.
- Any failing or uncovered row is not hidden; it becomes the recommended next
  experiment.
- No app source changes are made.

Partial criteria:

- Runtime automation proves the main tab/pane/window/DevTools rows, but one or
  more lifecycle rows remain uncovered or only indirectly proven.
- One baseline scenario fails for a reason that can be isolated and should
  become the next experiment.

Fail criteria:

- The baseline cannot launch Ghostboard/Roamium.
- The experiment cannot classify every requested Issue 818 behavior.
- The experiment makes app source changes before proving which lifecycle row
  needs a fix.

## Design Review

This experiment is plan-only until a fresh-context adversarial design review
approves it. Record the reviewer verdict here, fix all real findings, and commit
the approved plan before implementation begins.

Fresh-context adversarial design review by Codex subagent `Planck`:

- **Verdict:** Approved.
- **Optional finding 1:** Prior-evidence references were broad. Fixed by naming
  the exact Issue 809, 814, and 816 experiment files that may be used as
  baseline evidence.
- **Optional finding 2:** Conditional harness changes blurred the baseline-only
  scope. Fixed by requiring the result to record the missing signal first and by
  limiting any harness edit to assertion-only changes that do not alter app
  behavior.
- **Required findings:** None.

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

## Result

**Result:** Partial

All six planned runtime baseline scenarios passed. The result is `Partial`
because the matrix still exposes lifecycle rows that are not directly proven:
multi-profile isolation, warm reconnect/server reuse, and user-visible profile
identity.

Runtime checks:

| Scenario                         | Result | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| -------------------------------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `open-browser-in-new-tab`        | Pass   | Harness log `logs/ghostboard-geometry-open-browser-in-new-tab-harness-20260618-011221.log`; app log `logs/ghostboard-geometry-open-browser-in-new-tab-app-20260618-011221.log`; Roamium trace `logs/ghostboard-geometry-open-browser-in-new-tab-roamium-20260618-011221.log`; screenshots `logs/ghostboard-geometry-open-browser-in-new-tab-screenshot-20260618-011221.png`, `logs/ghostboard-geometry-open-browser-in-new-tab-browser-b-screenshot-20260618-011221.png`.                                            |
| `close-browser-tab`              | Pass   | Harness log `logs/ghostboard-geometry-close-browser-tab-harness-20260618-011321.log`; app log `logs/ghostboard-geometry-close-browser-tab-app-20260618-011321.log`; Roamium trace `logs/ghostboard-geometry-close-browser-tab-roamium-20260618-011321.log`; screenshot `logs/ghostboard-geometry-close-browser-tab-after-close-screenshot-20260618-011321.png`.                                                                                                                                                      |
| `split-right-close-browser-pane` | Pass   | Harness log `logs/ghostboard-geometry-split-right-close-browser-pane-harness-20260618-011422.log`; app log `logs/ghostboard-geometry-split-right-close-browser-pane-app-20260618-011422.log`; Roamium trace `logs/ghostboard-geometry-split-right-close-browser-pane-roamium-20260618-011422.log`; screenshot `logs/ghostboard-geometry-split-right-close-browser-pane-close-screenshot-20260618-011422.png`.                                                                                                        |
| `open-browser-in-new-window`     | Pass   | Harness log `logs/ghostboard-geometry-open-browser-in-new-window-harness-20260618-011552.log`; app log `logs/ghostboard-geometry-open-browser-in-new-window-app-20260618-011552.log`; Roamium trace `logs/ghostboard-geometry-open-browser-in-new-window-roamium-20260618-011552.log`; screenshots `logs/ghostboard-geometry-open-browser-in-new-window-window-b-screenshot-20260618-011552.png`, `logs/ghostboard-geometry-open-browser-in-new-window-window-a-restored-screenshot-20260618-011552.png`.            |
| `multiple-windows-with-browsers` | Pass   | Harness log `logs/ghostboard-geometry-multiple-windows-with-browsers-harness-20260618-011623.log`; app log `logs/ghostboard-geometry-multiple-windows-with-browsers-app-20260618-011623.log`; Roamium trace `logs/ghostboard-geometry-multiple-windows-with-browsers-roamium-20260618-011623.log`; screenshots `logs/ghostboard-geometry-multiple-windows-with-browsers-window-b-screenshot-20260618-011623.png`, `logs/ghostboard-geometry-multiple-windows-with-browsers-window-c-screenshot-20260618-011623.png`. |
| `devtools-singleton-guard`       | Pass   | Harness log `logs/ghostboard-geometry-devtools-singleton-guard-harness-20260618-011701.log`; app log `logs/ghostboard-geometry-devtools-singleton-guard-app-20260618-011701.log`; Roamium trace `logs/ghostboard-geometry-devtools-singleton-guard-roamium-20260618-011701.log`; screenshot `logs/ghostboard-geometry-devtools-singleton-guard-devtools-split-screenshot-20260618-011701.png`.                                                                                                                       |

Static checks:

- `git diff --check` — pass.
- No harness or app source changes were made for this experiment, so no
  `bash -n scripts/ghostboard-geometry-matrix.sh` rerun was required beyond the
  already-executed runtime scenarios.

Lifecycle baseline matrix:

| Behavior                                      | Status            | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                             | Next action                                                                                                                              |
| --------------------------------------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Multi-profile isolation                       | Uncovered         | The baseline scenarios all use the default profile. DevTools guard logic is scoped by `profile` and `browser`, and Issue 813 reviewed that source shape, but no current runtime row launches two profiles and proves storage/routing isolation.                                                                                                                                                                                      | Next experiment should create a two-profile runtime scenario and prove independent browser state plus routing.                           |
| Multi-pane routing                            | Partially covered | `split-right-close-browser-pane` proves one browser pane remains addressable through a split, then close cleanup removes that browser pane while the sibling terminal pane remains usable. It also proves former browser-pane and sibling-area clicks do not route to the closed browser context. It does not prove routing isolation between two simultaneous browser panes in one split layout.                                    | Add a two-browser split-pane row that opens browser A and browser B in separate panes and proves mouse/keyboard routing isolation.       |
| Multi-tab routing                             | Covered           | `open-browser-in-new-tab` proves browser A and browser B have distinct pane ids, browser tab ids, and CA context ids; keyboard markers route only to the selected browser and do not leak between native tabs. `close-browser-tab` proves the closed browser tab is not selectable and does not receive input.                                                                                                                       | Keep `open-browser-in-new-tab` and `close-browser-tab` as tab-routing rows.                                                              |
| Multi-window routing                          | Covered           | `open-browser-in-new-window` proves browser A and B use distinct native window ids, pane ids, browser tab ids, and contexts. `multiple-windows-with-browsers` extends this to A/B/C windows and verifies keyboard markers do not leak between browsers.                                                                                                                                                                              | Keep the two-window row as a normal smoke and the three-window row as a slower diagnostic row.                                           |
| Warm reconnect                                | Uncovered         | Issue 814 proves initial browser launch and failed-spawn cleanup, but this baseline does not disconnect and reconnect a TUI to an existing browser/profile server.                                                                                                                                                                                                                                                                   | Add a reconnect/server-reuse experiment after multi-profile behavior is bounded.                                                         |
| Server reuse                                  | Partially covered | Issue 814 Experiment 1 proves debug launch discovery, `BrowserReady`, failed-spawn cleanup before pending server creation, and preserving the server/browser key. The current baseline does not prove that a second TUI/browser request reuses an already-running server instead of spawning a duplicate process.                                                                                                                    | Add instrumentation or a runtime row that captures server child pid/count before and after a second same-profile request.                |
| Close/reopen behavior                         | Partially covered | `close-browser-tab` proves browser B tab close selects browser A, clears browser B overlay state, sends `CloseTab`, destroys/removes browser B in Roamium, and prevents input from reaching closed browser B. `devtools-singleton-guard` proves DevTools close releases the guard and reopening DevTools succeeds for the same inspected tab. The baseline does not prove closing and reopening a normal browser tab/profile/server. | Add normal browser close/reopen to the reconnect/server-reuse lifecycle row.                                                             |
| Stale process/tab cleanup                     | Partially covered | `close-browser-tab` and `split-right-close-browser-pane` prove stale browser tabs are destroyed and removed in Roamium. `devtools-singleton-guard` proves DevTools tabs are destroyed/removed on pane close. Issue 814 proves failed-spawn cleanup creates no pending server and spawns no browser process. This does not yet prove whole browser profile-server process cleanup when all profile panes/tabs close.                  | Add server-process cleanup to the reconnect/server-reuse experiment or create a separate process-lifecycle row if needed.                |
| DevTools target lookup                        | Covered           | `devtools-singleton-guard` proves QueryDevtoolsRequest/Reply success, in-flight reservation, duplicate rejection, abandoned-reservation expiry, live duplicate rejection, DevTools pane close cleanup, reopen success, and allowing browser B DevTools while browser A DevTools remains open.                                                                                                                                        | Keep as the DevTools lifecycle regression row.                                                                                           |
| Profile display/user-visible profile identity | Uncovered         | No baseline run checks a profile label, config-visible profile name, or any user-visible profile identity.                                                                                                                                                                                                                                                                                                                           | After multi-profile runtime support is proven, add a narrow UI/config row for whichever profile identity is intended to be user-visible. |

Reference evidence used:

- `issues/0814-ghostboard-launch-discovery-workflow/01-resolve-named-roamium-debug-launch.md`
  records debug browser launch discovery, failed-spawn cleanup, and no pending
  server/no browser process on invalid configuration.
- `issues/0809-ghostboard-viewport-geometry/10-close-browser-pane.md` records
  the prior browser-pane cleanup design and result that this baseline reran.
- `issues/0816-ghostboard-browser-state-interruptions/05-prove-renderer-crash-recovery.md`
  proves same-tab renderer crash recovery and Roamium liveness after renderer
  crash, but it does not replace the missing whole profile-server process
  lifecycle row.

## Conclusion

The native-tab, window, browser-tab cleanup, browser-pane cleanup, and DevTools
target lookup portions of Issue 818 have strong runtime coverage and all passed
on this VM. The remaining weak area is profile/server lifecycle and richer
multi-pane lifecycle behavior: there is no direct runtime row for two browser
panes in one split, two profiles, normal browser close/reopen, same-profile
server reuse or reconnect, whole server-process cleanup, or user-visible profile
identity.

The next experiment should focus narrowly on multi-profile isolation. If that
passes, the following experiment should prove same-profile reconnect/server
reuse and final server-process cleanup with explicit pid/count evidence.

## Completion Review

Fresh-context adversarial completion review by Codex subagent `Nash the 2nd`:

- **Initial verdict:** Changes required.
- **Finding 1:** The matrix marked multi-pane routing as `Covered`, but the
  cited runtime evidence only proved one browser pane plus a sibling terminal
  pane, not routing isolation between two simultaneous browser panes. Fixed by
  marking the row `Partially covered` and adding a two-browser split-pane next
  action.
- **Finding 2:** The matrix marked close/reopen behavior as `Covered`, but the
  cited runtime evidence proved normal browser tab close cleanup and DevTools
  close/reopen, not closing and reopening a normal browser tab/profile/server.
  Fixed by marking the row `Partially covered` and adding normal browser
  close/reopen to the reconnect/server-reuse next action.
- **Final verdict:** Approved. The reviewer confirmed both rows are now
  `Partially covered`, the evidence no longer overclaims, the missing normal
  browser close/reopen and two-browser split-pane rows are explicit future work,
  and no new Required findings were introduced.
