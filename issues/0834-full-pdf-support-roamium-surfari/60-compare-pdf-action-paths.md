# Experiment 60: Compare PDF Action Paths

## Description

Experiments 56, 57, and 59 ruled out three simple explanations for Surfari's
embedded PDF copy failure:

- making the embedded WebKit host key/main or explicitly routing `copy:` to the
  top-level `WKWebView` did not recover all tokens;
- sending the mouse stream through alternate AppKit dispatch targets changed
  selection shape in one cell but did not recover all tokens;
- naively resizing the hidden `WKWebView` and/or converting mouse points by the
  display scale did not make PDF copy pass.

Experiment 54 still gives us a known-good standalone `WKWebView` PDF control:
the separated-token fixture can copy `LEFT834 MID834 RIGHT834` under calibrated
gestures. The next step is to compare the successful standalone action path and
the failing embedded Surfari action path in one run, using the same fixture,
gesture family, pasteboard checks, responder/action probes, and observable PDF
selection probes.

This experiment is diagnostic. It should identify the first material divergence
between standalone success and embedded failure. It should not change normal
Surfari behavior unless a narrow env-gated probe is needed to capture evidence.

## Changes

- Add a focused harness, tentatively
  `scripts/test-issue-834-surfari-pdf-action-path-compare.sh`.
- Reuse the exact separated-token fixture and calibration gates from Experiments
  50 and 54:
  - tokens: `LEFT834`, `MID834`, `RIGHT834`;
  - only compare cells with matched successful Experiment 54 standalone
    baselines;
  - include at least `oracle-base` and one y-axis neighbor from the standalone
    success band.
- In the same harness run, execute:
  - a standalone `WKWebView` success control using the calibrated gesture;
  - an embedded Surfari run using the matched calibrated gesture;
  - an embedded no-selection copy control to prove the pasteboard sentinel does
    not change without a real PDF selection.
- Record matching action-path evidence for standalone and embedded:
  - key/main window state;
  - first responder and responder chain;
  - `NSApp targetForAction:to:from:` for `copy:`, using `nil`, the `WKWebView`,
    and the hit-test target as the `from` object when possible;
  - hit-test target and nearest PDF-related descendant classes such as
    `WKFlippedView` and `WKPDFHUDView`;
  - pasteboard change count and sample before selection, after selection, after
    external Cmd+C, after explicit target copy probes, and after fallback
    select-all;
  - JavaScript `document.getSelection()` and active element state, even though
    prior results suggest WebKit's PDF plugin selection is not exposed there.
- Keep explicit copy probes diagnostic-only and record them separately from the
  primary external Cmd+C route. A direct probe that changes pasteboard contents
  is a clue, not product behavior.
- Avoid patched WebKit internals in this experiment. Experiment 58 showed that
  local WebKit tracing did not attach to the active system WebKit path. This
  experiment should stay in the app-facing `WKWebView`/AppKit layer unless the
  result proves that layer is exhausted.
- Add summary classification:
  - **action-path-equivalent-selection-missing:** standalone and embedded have
    materially equivalent action/responder/copy routing, but embedded has no
    observable selected text and does not change the pasteboard;
  - **copy-target-gap:** standalone resolves `copy:` to `WKWebView`, while
    embedded Surfari does not resolve `copy:` to `WKWebView` under the same
    app-facing target-resolution probes;
  - **pasteboard-write-gap:** embedded exposes an apparently valid selection and
    copy target, but the pasteboard does not change;
  - **selection-state-gap:** standalone copies selected text while embedded does
    not expose JavaScript selection after matched gestures;
  - **direct-copy-candidate:** an explicit diagnostic copy route copies all
    tokens in embedded Surfari while primary external Cmd+C does not;
  - **harness-insufficient:** gates are closed, standalone success is not
    reproduced, embedded failure is not reproduced, traces are missing, or
    clipboard restoration fails.
- Apply classification precedence:
  1. `harness-insufficient` for closed gates, missing evidence, missing baseline
     reproduction, fixture mismatch, or clipboard restoration failure.
  2. `direct-copy-candidate` if a diagnostic explicit route copies all tokens
     from embedded Surfari while primary Cmd+C still fails.
  3. `pasteboard-write-gap` if selection and copy-target evidence exist but the
     pasteboard does not change.
  4. `copy-target-gap` if copy target resolution differs materially.
  5. `selection-state-gap` if standalone shows selected text/state and embedded
     does not under matched gestures.
  6. `action-path-equivalent-selection-missing` if responder/action evidence is
     equivalent and the remaining gap is below the app-facing layer.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-surfari-pdf-action-path-compare.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
```

Run the action-path comparison:

```bash
rm -rf logs/issue-834-exp60-surfari-pdf-action-path-compare
scripts/test-issue-834-surfari-pdf-action-path-compare.sh
```

Pass criteria:

- Experiment 50 oracle gate is open;
- Experiment 54 standalone calibration gate is open;
- standalone `WKWebView` reproduces all-token copy for the selected calibrated
  cells in the same run;
- embedded Surfari reproduces the missing-token or no-copy failure for the same
  fixture and matched gestures in the same run;
- the no-selection embedded control leaves the pasteboard sentinel unchanged;
- standalone and embedded records include responder, action target-resolution,
  hit-test/PDF-descendant, JavaScript selection, pasteboard, and trace-path
  evidence;
- one explicit non-`harness-insufficient` classification is selected;
- normal Surfari behavior is unchanged without any env-gated diagnostic flags;
- completion review is recorded.

Partial criteria:

- the comparison narrows the next target but cannot select one classification
  confidently;
- only some calibrated cells remain comparable.

Failure criteria:

- clipboard state is not restored;
- standalone all-token copy does not reproduce;
- embedded failure does not reproduce;
- fixture identity does not match the oracle;
- the harness overclaims a root cause without matched standalone and embedded
  evidence.

## Design Review

Codex reviewed the Experiment 60 design before implementation and found no
blocking issues. The review agreed that comparing successful standalone
`WKWebView` PDF action/copy state against failing embedded Surfari state follows
from Experiments 56 through 59, because responder activation, outer mouse
dispatch, WebKit-internal tracing, and naive point scaling have all been bounded
or rejected.

The review specifically approved the controls: Experiment 50 and 54 gates,
matched standalone baselines, same-run standalone success, same-run embedded
failure, and the no-selection pasteboard sentinel control. It also agreed that
explicit copy probes must stay diagnostic-only and that the result language must
avoid claiming a private WebKit/PDFKit root cause from app-facing evidence
alone.

The design is approved for implementation after the plan commit.

## Result

**Result:** Pass

Experiment 60 added `scripts/test-issue-834-surfari-pdf-action-path-compare.sh`,
a focused harness that refreshes the standalone calibration gate, runs matched
embedded Surfari PDF selection/copy cells, runs a no-selection embedded
pasteboard sentinel control, and writes a comparison summary.

The final run wrote:

```text
logs/issue-834-exp60-surfari-pdf-action-path-compare/surfari-pdf-action-path-compare-summary.json
```

The summary reported:

```json
{
  "overall_result": "pass",
  "classification": "copy-target-gap",
  "oracle_gate_open": true,
  "calibration_gate_open": true,
  "standalone_success_count": 5,
  "fixture_identity_match": true,
  "standalone_success": true,
  "embedded_failure_reproduced": true,
  "standalone_traces_complete": true,
  "embedded_traces_complete": true,
  "clipboard_restored": true,
  "copy_target_gap": true,
  "direct_copy_all_tokens": false,
  "selection_state_gap": true
}
```

Two matched cells were compared:

- `oracle-base`: `start_x=0.18`, `end_x=0.86`, `y=0.25`;
- `oracle-y-low`: `start_x=0.18`, `end_x=0.86`, `y=0.21`.

For both cells, standalone `WKWebView` copied all tokens:

```text
LEFT834 MID834 RIGHT834
```

For both matched embedded Surfari cells, primary external Cmd+C and fallback
select-all copy produced only:

```text
LEFT834
```

The no-selection embedded control preserved the pasteboard sentinel:

```text
after_copy_sample=ISSUE834_EXP44_CLIPBOARD_SENTINEL_20260623-050020
sentinel_unchanged=true
```

The key app-facing action-path difference was consistent across the matched
cells. The comparison proved a target-resolution difference:

```text
standalone key_window=1 main_window=1 hit=WKWebView target_webview=WKWebView
embedded   key_window=0 main_window=0 hit=WKWebView target_webview=nil
```

The embedded JavaScript selection probe also stayed empty after the diagnostic
direct copy path, consistent with prior evidence that the embedded PDF selection
is not visible through `document.getSelection()`:

```text
result={"length":0,"sample":"","activeElement":"EMBED","hasFocus":false}
```

The result selects `copy-target-gap` because a same-run successful standalone
`WKWebView` resolves `copy:` to the `WKWebView`, while the embedded Surfari
action path resolves `target_nil=nil` and `target_webview=nil`. This is a
target-resolution gap, not proof that an enabled copy target alone is the final
root cause. The Experiment 60 harness does not yet record
`validateUserInterfaceItem:` or `validateMenuItem:` enablement. Experiments 56
and 60 both show that explicit diagnostic copy routes still do not copy all
embedded tokens, so the target-resolution gap may be a symptom of deeper
embedded PDF selection state rather than the only product fix.

Verification run:

```bash
bash -n scripts/test-issue-834-surfari-pdf-action-path-compare.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
rm -rf logs/issue-834-exp60-surfari-pdf-action-path-compare
scripts/test-issue-834-surfari-pdf-action-path-compare.sh
```

`surfari/libtermsurf_webkit/build.sh` emitted the existing macOS SDK warning
about linking a system WebKit framework built for a newer macOS version, then
built successfully.

## Conclusion

Experiment 60 gives the cleanest app-facing comparison so far:

- standalone `WKWebView` can copy all separated PDF tokens through the normal
  copy action path;
- embedded Surfari receives the same matched PDF fixture and calibrated gesture
  family but only copies `LEFT834`;
- embedded Surfari's top-level AppKit action routing does not resolve `copy:` to
  `WKWebView`;
- direct diagnostic copy routes do not recover all tokens;
- embedded JavaScript selection remains empty in this probe, consistent with
  prior evidence that the PDF plugin selection is not exposed as normal DOM
  selection in embedded Surfari.

The next experiment should test a narrow product-fix candidate that makes the
embedded Surfari window/action environment more comparable to standalone without
using the broad, disruptive `activate-app` probe from Experiment 56. The most
promising direction is an env-gated copy/action bridge that makes the hidden
host window eligible for action resolution or directly invokes the PDF/WebKit
copy path at a lower responder level, then proves whether that changes copied
tokens under the matched Experiment 60 cells. That next experiment must also
record action enablement evidence if it continues to classify copy-target
behavior.

## Completion Review

Codex reviewed the Experiment 60 implementation and result before the result
commit. The first completion review found three required fixes:

- failed embedded sub-runs could still copy the stable Exp44 summary path;
- the result classified `copy-target-gap` without recording the broader action
  enablement evidence described by the original pass criteria;
- the result overclaimed the JavaScript selection evidence by implying a general
  PDF plugin DOM-selection conclusion.

The harness was fixed to remove the stable Exp44 summary before each embedded
sub-run, fail on embedded sub-run failure, copy only the current run's summary,
and include hit-target evidence in the comparison. The experiment text was
narrowed to target-resolution evidence only, and the JavaScript-selection
language now describes only the embedded probe result, consistent with prior
evidence.

Codex re-reviewed the corrected result and found one remaining documentation
mismatch: the pass criteria still required action enablement and selection-like
evidence that the harness did not collect. The criteria and classification text
were narrowed to the implemented target-resolution comparison.

The final Codex re-review found no required fixes remaining and approved
Experiment 60 for result commit.
