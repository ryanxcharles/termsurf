# Experiment 55: Test Embedded PDF Calibrated Gestures

## Description

Experiment 54 calibrated standalone `WKWebView` PDF selection geometry. The
standalone control copies `LEFT834 MID834 RIGHT834` for the oracle gesture band
around `start_x≈0.18`, `end_x≈0.86`, and `y≈0.21..0.29`. The old embedded
gesture from Experiments 51 through 53 (`start_x=0.58`, `end_x=0.99`, `y=0.43`)
is outside that band and should be treated as non-comparable.

This experiment should drive embedded Surfari with the calibrated standalone
gesture band. The goal is to determine whether embedded Surfari PDF
selection/copy works when the gesture is comparable to the standalone control,
or whether the embedded path still misses `RIGHT834` under calibrated gestures.

## Changes

- Add a harness, tentatively
  `scripts/test-issue-834-surfari-pdf-calibrated-gesture-copy.sh`, or extend an
  existing Issue 834 harness if that keeps the implementation simpler.
- Reuse the exact separated-token PDF fixture and oracle gates from Experiments
  50, 53, and 54.
- Require the Experiment 54 calibration summary to be present and valid, or run
  the calibration harness first:
  - `classification = embedded-gesture-outside-standalone-band`;
  - each calibrated embedded cell must have a matching Experiment 54 standalone
    cell with the same name and ratios;
  - each matching Experiment 54 standalone cell must have
    `contains_all_tokens = true`;
  - each matching Experiment 54 standalone cell must record copy route and trace
    evidence;
  - fixture identity matches the oracle;
  - all standalone copy routes are recorded.
- Run embedded Surfari cells with the calibrated standalone-band gestures:
  - `oracle-base`: `start_x=0.18`, `end_x=0.86`, `y=0.25`;
  - `oracle-y-low`: `start_x=0.18`, `end_x=0.86`, `y=0.21`;
  - `oracle-y-high`: `start_x=0.18`, `end_x=0.86`, `y=0.29`;
  - `oracle-x-wide`: `start_x=0.16`, `end_x=0.90`, `y=0.25`;
  - `oracle-x-tight`: `start_x=0.20`, `end_x=0.82`, `y=0.25`.
- Keep the old embedded-ratio cell (`start_x=0.58`, `end_x=0.99`, `y=0.43`) as a
  negative/comparison cell, but do not use it to prove product failure.
- For each embedded cell, record:
  - gesture ratios and computed web/global points;
  - the matched Experiment 54 standalone cell name, ratios, copied tokens, copy
    route, and trace path;
  - fixture identity;
  - primary post-selection copy sample;
  - fallback select-all copy sample;
  - direct-copy probe samples when enabled;
  - copied tokens;
  - copied tokens separated by route:
    - primary post-selection external Cmd+C tokens;
    - fallback select-all tokens;
    - direct-copy probe tokens;
  - copy route (`external Cmd+C` plus direct-copy probes when enabled);
  - Surfari geometry/copy trace paths;
  - whether all expected tokens were copied.
- Keep `TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE=1` and
  `TERMSURF_SURFARI_PDF_COPY_TRACE=1` enabled so responder/copy-target state is
  available if calibrated gestures still fail.
- Keep all changes diagnostic-only. Do not change Surfari product behavior in
  this experiment.
- Result language must treat this as a diagnostic matrix only. It may identify
  candidates and next targets, but it must not claim a root cause or product
  fix.
- Apply this outcome matrix:
  - **embedded-calibrated-single-cell-pass:** at least one matched calibrated
    embedded gesture copies all three tokens through primary post-selection
    external Cmd+C, with fixture/oracle/calibration gates open. This proves one
    working calibrated embedded gesture only; it is not a broad product fix or
    root-cause conclusion;
  - **embedded-calibrated-matrix-pass:** every matched calibrated embedded
    gesture copies all three tokens through primary post-selection external
    Cmd+C, with fixture/oracle/calibration gates open. This proves the
    calibrated matrix only; it is not a full PDF selection/copy product fix;
  - **embedded-calibrated-right-edge-gap:** calibrated embedded gestures
    reproduce the left/mid-only primary copy behavior while their matched
    standalone calibrated gestures copy all tokens;
  - **embedded-calibrated-coordinate-selection-gap:** calibrated embedded
    gestures select a substantially different token subset than their matched
    standalone calibrated gestures, such as left-only embedded copies where the
    matched standalone cell copies all tokens;
  - **embedded-calibrated-copy-routing-gap:** calibrated embedded gestures show
    fallback/direct-probe success but primary post-selection external Cmd+C
    fails. Fallback/select-all and direct-copy probes can support this
    classification, but they must not satisfy selection-pass criteria;
  - **responder-gap-candidate:** calibrated embedded gestures fail and traces
    show material key/main-window, first-responder, or `copy:` target
    differences from their matched successful standalone calibrated cells;
  - **harness-insufficient:** calibration gates are closed, fixture identity
    fails, traces are missing, clipboard restoration fails, or embedded cells do
    not run.
- Apply this classification precedence:
  1. `harness-insufficient` for closed gates, missing matched standalone cells,
     missing traces, fixture mismatch, or clipboard restoration failure.
  2. `embedded-calibrated-matrix-pass` if every matched calibrated embedded cell
     passes via primary post-selection external Cmd+C.
  3. `embedded-calibrated-single-cell-pass` if at least one matched calibrated
     embedded cell passes via primary post-selection external Cmd+C.
  4. `embedded-calibrated-copy-routing-gap` if fallback/direct probes copy all
     tokens but primary post-selection external Cmd+C does not.
  5. `responder-gap-candidate` if primary calibrated cells fail and matched
     standalone responder/copy-target baselines differ materially.
  6. `embedded-calibrated-coordinate-selection-gap` if primary calibrated cells
     select a substantially different token subset than their matched standalone
     baselines without stronger routing or responder evidence.
  7. `embedded-calibrated-right-edge-gap` if primary calibrated cells reproduce
     left/mid-only copy without stronger routing or responder evidence.
- Update this experiment file with the result.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-surfari-pdf-calibrated-gesture-copy.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
```

Run the calibrated embedded matrix:

```bash
rm -rf logs/issue-834-exp55-surfari-pdf-calibrated-gesture-copy
scripts/test-issue-834-surfari-pdf-calibrated-gesture-copy.sh
```

Pass criteria:

- Experiment 50 oracle gate is open;
- Experiment 54 calibration gate is open;
- every embedded calibrated cell is mechanically matched by name and ratios to a
  successful Experiment 54 standalone cell;
- fixture identity matches the exact separated-token fixture;
- every calibrated embedded cell records gesture, clipboard, copied-token,
  route, matched-standalone, and trace evidence;
- embedded Surfari selects one explicit non-`harness-insufficient` outcome;
- clipboard state is restored;
- no product selection behavior is changed;
- result language does not claim a root cause or product fix;
- completion review is recorded.

Partial criteria:

- the harness gathers useful embedded calibrated evidence but cannot classify
  confidently;
- some calibrated cells run and others fail for harness reasons;
- traces are present but not enough to distinguish geometry, responder, and copy
  routing.

Failure criteria:

- clipboard state is not restored;
- calibration or oracle gates are closed;
- fixture identity does not match;
- embedded automation cannot launch;
- the result claims a product root cause or product fix from this diagnostic
  matrix.

## Design Review

Codex reviewed the design and agreed Experiment 55 is the correct next step
after Experiment 54. The initial review required tighter evidence gates:

- each embedded calibrated cell must be mechanically matched to a successful
  Experiment 54 standalone cell by name and ratios;
- primary post-selection copy, fallback select-all, and direct-probe evidence
  must be recorded and classified separately;
- responder-gap candidates must compare against matched standalone responder and
  copy-target baselines;
- one-cell success must not be described as a product fix;
- result language must not claim a product root cause from this diagnostic
  matrix;
- classification precedence must be explicit.

The design was updated for each finding. A follow-up Codex review confirmed the
required findings were resolved and approved the design for the plan commit.

## Result

**Result:** Pass

Experiment 55 added
`scripts/test-issue-834-surfari-pdf-calibrated-gesture-copy.sh` and ran the real
embedded Surfari PDF selection path against the calibrated standalone gesture
band from Experiment 54.

The final harness run wrote:

`logs/issue-834-exp55-surfari-pdf-calibrated-gesture-copy/surfari-pdf-calibrated-gesture-copy-summary.json`

The summary reported:

- `overall_result = pass`
- `classification = responder-gap-candidate`
- `oracle_gate_open = true`
- `calibration_gate_open = true`
- `matched_calibrated_cells = true`
- `fixture_identity_match = true`
- `traces_complete = true`
- `primary_pass_count = 0`
- `responder_gap_names = [oracle-base, oracle-x-tight, oracle-x-wide, oracle-y-high, oracle-y-low]`
- `primary_left_only_names = [oracle-base, oracle-x-tight, oracle-x-wide, oracle-y-high, oracle-y-low]`

Every calibrated embedded cell was mechanically matched to a successful
Experiment 54 standalone cell with the same name and ratios. Those matched
standalone cells copied all three tokens, but embedded Surfari copied only
`LEFT834` for every calibrated cell:

- `oracle-base`: primary embedded copy `LEFT834 `
- `oracle-y-low`: primary embedded copy `LEFT834 `
- `oracle-y-high`: primary embedded copy `LEFT834 `
- `oracle-x-wide`: primary embedded copy `LEFT834 M`
- `oracle-x-tight`: primary embedded copy `LEFT834`

The old non-comparable `embedded-ratio` comparison cell still copied
`LEFT834 MID834`, matching prior experiments.

Fallback select-all and direct-copy probes did not recover all tokens for the
calibrated embedded cells; they also only reported left-side token subsets. This
keeps the result out of primary copy-route success territory.

The higher-precedence responder comparison also found material differences for
every calibrated cell:

- matched standalone cells had key/main windows; embedded `TSHostWindow`
  instances had `key_window=0` and `main_window=0`;
- matched standalone cells resolved both `target_nil` and `target_webview` to
  `WKWebView`;
- embedded cells reported `target_nil=nil` and `target_webview=nil`;
- first responder was `WKWebView` on both sides, but the responder chain differs
  because embedded goes through `TSHostWindow` while standalone goes directly
  through `NSWindow`.

Verification run:

```bash
bash -n scripts/test-issue-834-surfari-pdf-calibrated-gesture-copy.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
rm -rf logs/issue-834-exp55-surfari-pdf-calibrated-gesture-copy
scripts/test-issue-834-surfari-pdf-calibrated-gesture-copy.sh
```

`surfari/libtermsurf_webkit/build.sh` passed with the existing macOS SDK/WebKit
version warning. `cargo build -p surfari` passed.
`git -C webkit/src status --short` was clean.

## Conclusion

Embedded Surfari differs from matched standalone `WKWebView` in both copied
token subset and responder/copy-target state. The approved precedence makes the
result `responder-gap-candidate`: the embedded host window is not key/main, and
AppKit cannot resolve `copy:` targets for either nil or the `WKWebView`, while
standalone can.

The copied-token evidence remains important: with the calibrated standalone
gesture band, standalone copies all three tokens, but embedded Surfari selects
only the left side of the line. The next experiment should isolate the responder
gap first by making the embedded Surfari host window/app state comparable to
standalone during selection/copy, then rerunning the calibrated matrix. If that
does not change selection, the following experiment should diagnose coordinate
transform and PDF page-fit differences.

This result is still diagnostic. It identifies a responder/copy-target candidate
and secondary coordinate-selection evidence; it does not claim the final root
cause or a product fix.

## Completion Review

Codex reviewed the completed experiment and initially required the harness to
honor the approved responder-precedence gate. The harness was updated to parse
embedded and matched standalone responder/copy-target state, require that
evidence for trace completeness, and classify `responder-gap-candidate` before
coordinate-selection. The calibrated matrix was rerun and the result text was
updated so coordinate-selection evidence is secondary and no root cause or fix
is claimed.

A follow-up Codex review confirmed the required findings were resolved and
approved the Experiment 55 result for commit.
