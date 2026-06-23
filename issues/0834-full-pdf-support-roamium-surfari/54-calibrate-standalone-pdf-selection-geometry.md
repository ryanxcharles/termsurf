# Experiment 54: Calibrate Standalone PDF Selection Geometry

## Description

Experiment 53 produced complete embedded and standalone geometry traces but
could not compare them safely. The exact separated-token fixture is selectable
in standalone `WKWebView` according to the Experiment 50 oracle, but the
standalone control in Experiment 53 did not copy any tokens when driven with the
same normalized all-token/over-wide gesture ratios used by embedded Surfari.
That means the comparison harness still lacks a trustworthy standalone selection
geometry baseline.

This experiment should calibrate standalone `WKWebView` PDF selection geometry
before making any product fix. The goal is to find and record one or more
standalone gestures that reliably copy `LEFT834 MID834 RIGHT834` from the exact
fixture, with complete geometry/responder/copy-target traces, and then compare
those successful standalone gestures against the embedded gesture that copies
only `LEFT834 MID834`.

## Changes

- Add a new harness, tentatively
  `scripts/test-issue-834-pdf-standalone-geometry-calibration.sh`, or extend the
  Experiment 53 harness if that keeps the code simpler.
- Reuse the exact separated-token PDF fixture and identity checks from
  Experiments 50 and 53:
  - tokens: `LEFT834`, `MID834`, `RIGHT834`;
  - operators:
    `BT /F1 24 Tf 72 620 Td (LEFT834) Tj ET | BT /F1 24 Tf 220 620 Td (MID834) Tj ET | BT /F1 24 Tf 360 620 Td (RIGHT834) Tj ET`;
  - token boxes and page geometry identical to the oracle.
- Build a standalone `WKWebView` control that installs a normal Edit > Copy menu
  and records the same geometry trace fields as Experiment 53:
  - view tree;
  - hit-test target;
  - converted web/window/target coordinates;
  - key/main window state;
  - first responder and responder chain;
  - `copy:` targets;
  - clipboard sample and pasteboard change count.
- Sweep or explicitly test standalone PDF selection gestures until the harness
  finds at least one successful all-token copy. The matrix must include:
  - the Experiment 53 embedded-ratio gesture (`start_x=0.58`, `end_x=0.99`,
    `y=0.43`);
  - the Experiment 50 known-good `WKWebView` oracle gesture family
    (`start_x≈0.18`, `end_x≈0.86`, `y≈0.25`);
  - a small y-axis and x-axis sweep around any successful gesture so the result
    records the tolerance, not just one lucky point.
- For each standalone cell, record:
  - route used for copy (`cg-event`, menu, or in-process);
  - start/end ratios;
  - start/end global points;
  - start/end `WKWebView` points;
  - copied tokens;
  - geometry trace path;
  - whether all expected tokens were copied.
- Run the embedded Surfari separated-token cell from Experiment 53 in the same
  harness run with geometry tracing enabled.
- Compare only after the standalone side has at least one successful all-token
  gesture:
  - whether embedded's failing drag point lies outside the standalone successful
    selection band;
  - whether embedded and standalone differ in responder/copy-target state;
  - whether the same top-level `WKWebView` hit-test target is observed;
  - whether any evidence points to coordinate scaling, responder state, or PDF
    selection internals as the next product fix target.
- Keep all changes diagnostic-only. Do not change Surfari product selection
  behavior in this experiment.
- Apply this outcome matrix:
  - **standalone-calibration-only:** at least one standalone gesture copies all
    three tokens, traces are complete, and the embedded failure is reproduced in
    the same run, but the harness cannot yet make a direct comparability
    decision;
  - **embedded-gesture-outside-standalone-band:** standalone calibration passes
    and shows the embedded gesture is not comparable because it is outside the
    successful standalone band;
  - **responder-gap-candidate:** standalone calibration passes and comparable
    geometry exists, but embedded differs materially in key/main window,
    first-responder, or `copy:` target state;
  - **hit-test-target-gap:** standalone calibration passes and comparable
    geometry exists, but embedded and standalone hit different view classes;
  - **geometry-equivalent:** standalone calibration passes and comparable
    geometry/responder/hit-test state looks equivalent, pushing the next
    experiment toward deeper WebKit/PDFKit selection internals;
  - **harness-insufficient:** no standalone all-token gesture is found, traces
    are incomplete, fixture/oracle gates are closed, or embedded failure is not
    reproduced.
- Update this experiment file with the result.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-pdf-standalone-geometry-calibration.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
```

Run the calibration:

```bash
rm -rf logs/issue-834-exp54-pdf-standalone-geometry-calibration
scripts/test-issue-834-pdf-standalone-geometry-calibration.sh
```

Pass criteria:

- the Experiment 50 oracle gate is open;
- fixture identity matches the exact separated-token fixture;
- standalone `WKWebView` copies `LEFT834 MID834 RIGHT834` for at least one
  calibrated gesture in the same run;
- standalone traces include view tree, hit-test target, coordinate conversion,
  responder state, and copy-target evidence;
- embedded Surfari reproduces the missing-`RIGHT834` behavior in the same run;
- embedded traces include the same evidence categories;
- one explicit comparable-geometry outcome class is selected:
  `embedded-gesture-outside-standalone-band`, `responder-gap-candidate`,
  `hit-test-target-gap`, or `geometry-equivalent`;
- no product selection behavior is changed;
- completion review is recorded.

Partial criteria:

- embedded reproduces the failure and traces are useful, but no standalone
  all-token gesture is found;
- standalone calibration succeeds and selects `standalone-calibration-only`, but
  the harness cannot prove whether any successful standalone gesture is
  comparable to embedded;
- private WebKit/PDF views hide enough detail that the next step must remain
  cautious.

Failure criteria:

- clipboard state is not restored;
- standalone or embedded automation cannot launch;
- fixture identity does not match the oracle;
- the harness changes Surfari product behavior;
- the result claims a root cause without a calibrated standalone baseline and
  direct evidence for the selected outcome.

## Design Review

Codex reviewed the design and agreed Experiment 54 is the correct next
experiment after Experiment 53. The initial review found one required issue:
`standalone-calibrated` could have passed without proving comparability. The
design was tightened so calibration-only is a Partial outcome, while Pass
requires a comparable-geometry outcome such as
`embedded-gesture-outside-standalone-band`, `responder-gap-candidate`,
`hit-test-target-gap`, or `geometry-equivalent`.

A follow-up Codex review confirmed the required finding was resolved and
approved the design for the plan commit.

## Result

**Result:** Pass

Experiment 54 added
`scripts/test-issue-834-pdf-standalone-geometry-calibration.sh` and ran a fresh
standalone `WKWebView` calibration matrix against the exact separated-token PDF
fixture.

The final harness run wrote:

`logs/issue-834-exp54-pdf-standalone-geometry-calibration/pdf-standalone-geometry-calibration-summary.json`

The summary reported:

- `overall_result = pass`
- `classification = embedded-gesture-outside-standalone-band`
- `oracle_gate_open = true`
- `fixture_identity_match = true`
- `standalone_success_count = 5`
- `standalone_success_names = [oracle-base, oracle-x-tight, oracle-x-wide, oracle-y-high, oracle-y-low]`
- `standalone_success_y_ratios = [0.21, 0.25, 0.29]`
- `standalone_embedded_ratio_success = false`
- `embedded_reproduced_missing_right = true`
- `embedded_outside_success_y_band = true`
- `clipboard_restore_status = restored`

The calibrated standalone `WKWebView` control copied all three tokens for the
known-good oracle gesture family:

- `oracle-base`: `start_x=0.18`, `end_x=0.86`, `y=0.25`
- `oracle-y-low`: `start_x=0.18`, `end_x=0.86`, `y=0.21`
- `oracle-y-high`: `start_x=0.18`, `end_x=0.86`, `y=0.29`
- `oracle-x-wide`: `start_x=0.16`, `end_x=0.90`, `y=0.25`
- `oracle-x-tight`: `start_x=0.20`, `end_x=0.82`, `y=0.25`

The standalone `embedded-ratio` cell, using Experiment 53's embedded gesture
(`start_x=0.58`, `end_x=0.99`, `y=0.43`), did not copy all tokens. Its clipboard
sample was `HT834`, not `LEFT834 MID834 RIGHT834`.

Embedded Surfari reproduced the real failure in the same run: it copied only
`LEFT834 MID834`, not `RIGHT834`. Therefore the prior Experiment 53 comparison
should be treated as non-comparable because the embedded-ratio gesture was not a
successful standalone selection gesture. The next product-facing experiment
should use the calibrated standalone band as the control when deciding whether
Surfari's embedded failure is due to coordinate selection geometry,
responder/copy-target state, or deeper WebKit/PDFKit selection behavior.

Verification run:

```bash
bash -n scripts/test-issue-834-pdf-standalone-geometry-calibration.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
rm -rf logs/issue-834-exp54-pdf-standalone-geometry-calibration
scripts/test-issue-834-pdf-standalone-geometry-calibration.sh
```

`surfari/libtermsurf_webkit/build.sh` passed with the existing macOS SDK/WebKit
version warning. `cargo build -p surfari` passed.
`git -C webkit/src status --short` was clean.

## Conclusion

The standalone PDF selection control is now calibrated. The embedded Surfari
gesture that was used in Experiments 51 through 53 is outside the standalone
success band and should no longer be treated as comparable to the standalone
oracle. The next experiment should drive embedded Surfari with calibrated
standalone-band gestures, especially around `start_x≈0.18`, `end_x≈0.86`, and
`y≈0.21..0.29`, then determine whether embedded Surfari still misses `RIGHT834`.
If calibrated embedded gestures still fail, the responder/copy-target gap from
Experiment 53 becomes a stronger fix candidate. If calibrated embedded gestures
pass, the immediate issue is the TUI/harness gesture geometry rather than PDF
selection/copy behavior itself.

## Completion Review

Codex reviewed the completed experiment and initially required one harness fix:
each standalone cell needed to record its copy route. The harness now records
`copy_route = "cg-event-command-c"` for every standalone cell, the calibration
was rerun, and the summary confirmed that all standalone cells used that route.

A follow-up Codex review confirmed the copy-route finding was resolved and
approved the Experiment 54 result for commit. The review also accepted the
`Pass` / `embedded-gesture-outside-standalone-band` classification and found no
remaining blocking issues.
