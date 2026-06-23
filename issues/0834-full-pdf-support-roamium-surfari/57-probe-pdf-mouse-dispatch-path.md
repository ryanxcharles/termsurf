# Experiment 57: Probe PDF Mouse Dispatch Path

## Description

Experiments 55 and 56 ruled out the simplest responder/copy-target explanation
for embedded Surfari PDF selection/copy. Standalone `WKWebView` copies all three
separated tokens under calibrated gestures, but embedded Surfari still selects
only the left-side token subset. Making the embedded host key/main, making the
`WKWebView` first responder, and explicitly routing `copy:` to the WebView did
not recover the missing tokens.

The remaining likely gap is how embedded Surfari synthesizes and dispatches
mouse events into WebKit/PDFKit. The current path constructs an `NSEvent`,
hit-tests from `contents->web_view`, and directly invokes mouse handlers on the
hit target. A normal standalone AppKit window receives CG mouse input through
the window event dispatch path. WebKit's PDF plugin/PDFKit selection logic may
depend on that dispatch path, a descendant view target, or event metadata that
the direct target path does not reproduce.

This experiment should add env-gated PDF mouse-dispatch probes and run the same
calibrated embedded cells from Experiment 55. It must stay diagnostic until a
specific dispatch mode is proven.

## Changes

- Add env-gated mouse dispatch probes in
  `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`:
  - `TERMSURF_SURFARI_PDF_MOUSE_DISPATCH_PROBE=1`;
  - `TERMSURF_SURFARI_PDF_MOUSE_DISPATCH_MODE=current|window-send-event|webview-direct|flipped-view-direct|pdf-hud-direct`;
  - optional trace lines in the existing PDF copy or geometry trace files.
- Keep normal behavior unchanged unless
  `TERMSURF_SURFARI_PDF_MOUSE_DISPATCH_PROBE=1` is present.
- Preserve the current dispatch behavior as the no-flag control and as the
  flagged `current` control.
- Each dispatch mode must apply to the whole gesture stream, not only mouse-down
  and mouse-up. The probe must route and trace mouse down, mouse dragged/moved,
  and mouse up consistently for a given mode, including whether
  `window-send-event` is used for drag events.
- Probe at least these dispatch modes:
  - **normal-control:** no dispatch probe flag;
  - **flagged-current:** probe flag set with `current`, expected to behave like
    the current code path;
  - **window-send-event:** send the synthesized event through
    `contents->window sendEvent:` instead of directly invoking the target view;
  - **webview-direct:** directly invoke the mouse handler on
    `contents->web_view` regardless of hit-test result;
  - **flipped-view-direct:** locate the first `WKFlippedView` descendant and
    directly invoke the mouse handler on it when present;
  - **pdf-hud-direct:** locate the first `WKPDFHUDView` descendant and directly
    invoke the mouse handler on it when present. This is expected to be a
    negative control unless WebKit routes PDF gestures through the HUD view.
- Record, for every event:
  - original web coordinates and converted window coordinates;
  - selected dispatch mode;
  - target class and target frame/bounds;
  - hit-test class;
  - whether the target exists;
  - event type, button, click count, modifiers, and event number.
- For `window-send-event`, record this as synthetic window dispatch evidence,
  not standalone AppKit parity. The trace must include pre-dispatch target and
  hit-test state, key/main/visible/ordered window state, current-event and mouse
  swizzle state, and whether AppKit appears to retarget, drop, or deliver the
  event.
- For descendant-target modes, record unavailable target cases per mode/cell
  instead of treating a missing descendant as a generic harness failure when the
  rest of the matrix remains interpretable.
- Add a harness, tentatively
  `scripts/test-issue-834-surfari-pdf-mouse-dispatch-path.sh`, that:
  - requires the Experiment 50 oracle summary;
  - requires the Experiment 54 standalone calibration summary;
  - requires or references the Experiment 55 calibrated embedded baseline;
  - runs the five calibrated cells from Experiment 55 for every dispatch mode;
  - unsets or rejects stale Experiment 52/56 probe variables for every run,
    including `TERMSURF_SURFARI_PDF_SELECTION_EDGE_*` and
    `TERMSURF_SURFARI_PDF_RESPONDER_*`, because this experiment is not testing
    edge correction or responder activation;
  - carries matched Experiment 54 standalone cell name/ratios/copy route/trace
    for each cell;
  - records copied tokens separately for primary external Cmd+C, fallback
    select-all, and direct-copy probes;
  - records dispatch trace lines and matched standalone baselines;
  - keeps `TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE=1`,
    `TERMSURF_SURFARI_PDF_COPY_TRACE=1`, and
    `TERMSURF_SURFARI_PDF_COPY_DIRECT=1` enabled.
- Apply this outcome matrix:
  - **mouse-dispatch-fix-candidate:** one or more non-control dispatch modes
    copy all three tokens through primary external Cmd+C for at least one
    calibrated cell, with oracle/calibration/fixture gates open;
  - **mouse-dispatch-matrix-candidate:** one non-control dispatch mode copies
    all three tokens through primary external Cmd+C for all five calibrated
    cells;
  - **dispatch-changes-selection-only:** a non-control dispatch mode changes the
    copied token subset, but still does not copy all three tokens;
  - **dispatch-path-unchanged:** all comparable modes reproduce the current
    left-side-token behavior;
  - **dispatch-target-unavailable:** a descendant-target mode cannot locate its
    target view, but the rest of the harness remains interpretable;
  - **harness-insufficient:** oracle/calibration gates are closed, fixture
    identity fails, baseline controls do not reproduce Experiment 55, required
    traces are missing, or clipboard restoration fails.
- Apply this classification precedence:
  1. `harness-insufficient` for closed gates, missing required traces, missing
     baseline reproduction, fixture mismatch, or clipboard restoration failure.
  2. `mouse-dispatch-matrix-candidate` for a non-control mode that copies all
     tokens in all calibrated cells through primary external Cmd+C.
  3. `mouse-dispatch-fix-candidate` for a non-control mode that copies all
     tokens in at least one calibrated cell through primary external Cmd+C.
  4. `dispatch-changes-selection-only` if a non-control mode changes the token
     subset without full-token success.
  5. `dispatch-target-unavailable` if all failed modes are unavailable target
     probes and comparable modes otherwise reproduce baseline.
  6. `dispatch-path-unchanged` if comparable non-control modes run but selection
     remains unchanged.
- Keep result language diagnostic. A fix candidate is not product behavior until
  a follow-up experiment converts it into normal dispatch and proves no
  regressions.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-surfari-pdf-mouse-dispatch-path.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
```

Run the dispatch probe:

```bash
rm -rf logs/issue-834-exp57-surfari-pdf-mouse-dispatch-path
scripts/test-issue-834-surfari-pdf-mouse-dispatch-path.sh
```

Pass criteria:

- Experiment 50 oracle gate is open;
- Experiment 54 calibration gate is open;
- normal-control and flagged-current reproduce the Experiment 55 embedded
  baseline;
- every calibrated cell/mode is mechanically matched by name and ratios to a
  successful Experiment 54 standalone cell;
- fixture identity matches the separated-token oracle;
- every comparable mode records dispatch target, hit-test, copied-token, route,
  and trace evidence;
- one explicit non-`harness-insufficient` outcome is selected;
- normal behavior is unchanged without the env-gated probe flag;
- result language does not claim a product fix unless a later follow-up makes
  the chosen dispatch normal behavior and regression-tests it;
- completion review is recorded.

Partial criteria:

- baseline controls reproduce and some dispatch modes produce useful evidence,
  but one or more descendant target modes are unavailable;
- one dispatch mode disrupts the automation while other modes remain
  interpretable;
- traces are present but cannot distinguish event dispatch from lower-level
  PDFKit coordinate handling.

Failure criteria:

- clipboard state is not restored;
- oracle or calibration gates are closed;
- baseline controls do not reproduce the embedded left-side-token behavior;
- probe flags alter normal behavior when disabled;
- the result overclaims a final root cause or product fix.

## Design Review

Codex reviewed the Experiment 57 design and agreed it is the logical next
diagnostic step after Experiment 56. The review required stricter controls
before the plan commit:

- dispatch modes must cover the full gesture stream, including dragged/moved
  events, not only mouse-down and mouse-up;
- the harness must unset or reject stale Experiment 52 selection-edge and
  Experiment 56 responder probe variables;
- `window-send-event` must be described and traced as synthetic window dispatch
  evidence, not as proof of standalone AppKit parity;
- descendant target availability should be recorded per mode/cell;
- the design-review result must be recorded before committing the plan.

The README already included the required `Designed` status for Experiment 57,
wrapped onto the following line by Prettier. The design was updated for the
substantive findings above.

## Result

**Result:** Pass

Experiment 57 added env-gated Surfari PDF mouse-dispatch probes and the
`scripts/test-issue-834-surfari-pdf-mouse-dispatch-path.sh` harness. The final
run wrote:

```text
logs/issue-834-exp57-surfari-pdf-mouse-dispatch-path/surfari-pdf-mouse-dispatch-path-summary.json
```

The summary reported:

```json
{
  "overall_result": "pass",
  "classification": "dispatch-changes-selection-only",
  "oracle_gate_open": true,
  "calibration_gate_open": true,
  "mode_names_complete": true,
  "matched_calibrated_cells": true,
  "fixture_identity_match": true,
  "traces_complete": true,
  "dispatch_traces_complete": true,
  "normal_baseline_reproduced": true,
  "flagged_baseline_reproduced": true,
  "changed_selection_count": 1,
  "clipboard_restore_status": "restored"
}
```

The first harness run exposed a real implementation bug in the probe itself:
`NSEventTypeLeftMouseDragged` was not handled by the refactored dispatch helper,
so flagged-current delivered down/up but dropped the drag. That made the
flagged-current control non-comparable. The helper was fixed to dispatch
`mouseDragged:` for `NSEventTypeLeftMouseDragged`, and the matrix was rerun from
a clean log directory.

The rerun proved that the probe flag does not change the selection-relevant
down/drag/up dispatch path or copied-token outcome for the flagged-current
control. Non-drag mouse-move remains on the original no-flag
`_simulateMouseMove:` path so the control does not change hover/move behavior
outside the drag stream. Normal-control and flagged-current both reproduced the
Experiment 55 embedded baseline. Every calibrated cell in both controls copied
only `LEFT834` or the same left-side subset, never all three tokens.

The non-control modes mostly reproduced the same left-side-token behavior:

- `webview-direct`;
- `flipped-view-direct`;
- `pdf-hud-direct`.

`window-send-event` matched the baseline for four calibrated cells, but changed
`oracle-x-wide` from the baseline left-side subset to an even smaller partial
copy (`LEFT`). It still did not copy all three tokens, so it is not a fix
candidate.

Every non-control mode recorded dispatch traces for the selection-relevant
`mouse-down`, `mouse-drag`, and `mouse-up` events. Pre-drag non-drag
`mouse-move` is intentionally excluded from the classification gate because the
PDF text selection operation is driven by the down/drag/up stream, and the
control keeps non-drag movement on the original `_simulateMouseMove:` path. No
mode copied all three tokens through primary external Cmd+C, and no descendant
target mode reported an unavailable target.

Verification passed:

```bash
bash -n scripts/test-issue-834-surfari-pdf-mouse-dispatch-path.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
rm -rf logs/issue-834-exp57-surfari-pdf-mouse-dispatch-path
scripts/test-issue-834-surfari-pdf-mouse-dispatch-path.sh
```

`surfari/libtermsurf_webkit/build.sh` emitted the existing macOS SDK warning
about linking a WebKit framework built for a newer macOS version, then built
`libtermsurf_webkit.dylib` and `smoke-test` successfully.

## Conclusion

The embedded Surfari PDF selection/copy gap is not fixed by routing the
synthesized mouse stream through `NSWindow sendEvent:`, directly to the
`WKWebView`, directly to `WKFlippedView`, or directly to `WKPDFHUDView`.
`window-send-event` can change the copied selection in at least one cell, but in
this run it made the selection smaller rather than recovering the missing
tokens. Those dispatch paths are therefore ruled out as simple fix candidates
for the separated-token PDF selection failure.

The next experiment should move below generic AppKit dispatch and inspect the
WebKit/PDFKit PDF view internals or selection state more directly. The current
evidence points toward PDF plugin/PDFKit coordinate interpretation, page/text
layout mapping, or selection-state extraction inside WebKit's PDF path rather
than top-level responder activation or mouse target selection.

## Completion Review

Codex reviewed the Experiment 57 implementation and result before the result
commit. The first completion review found a real control issue: flagged-current
preserved copied-token output but routed non-drag mouse-move through a different
path than no-flag current dispatch. The implementation was fixed so non-drag
mouse-move remains on the original `_simulateMouseMove:` path, and the result
language was tightened to describe the classification as covering the
selection-relevant down/drag/up stream.

Codex then re-reviewed the corrected diff, final summary, and result text. The
re-review found no remaining must-fix implementation or result-language issues
and approved the result commit after this completion-review section was
recorded.
