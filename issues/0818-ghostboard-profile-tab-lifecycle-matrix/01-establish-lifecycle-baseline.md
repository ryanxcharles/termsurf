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
