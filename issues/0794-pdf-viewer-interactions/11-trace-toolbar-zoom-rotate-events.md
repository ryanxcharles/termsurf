# Experiment 11: Trace Toolbar Zoom and Rotate Events

## Description

Experiment 10 proved that the PDF toolbar is reachable but only partially
functional:

- fit mode works;
- page selector navigation works;
- save/download and print are present but intentionally unclicked;
- zoom in, zoom out, and rotate receive pointer activation/ripple, but no PDF
  viewer state changes.

Chromium's PDF source says the intended event path is:

```text
cr-icon-button click
  -> viewer-toolbar onZoomInClick_ / onZoomOutClick_ / onRotateClick_
  -> custom event on <viewer-toolbar>
  -> <pdf-viewer> handler in pdf_viewer.html / pdf_viewer_base.ts
  -> viewport_.zoomIn()/zoomOut() or currentController.rotateCounterclockwise()
  -> PDF plugin/PDFium geometry or rotation update
```

Experiment 11 should not guess at a fix. It should instrument that path in the
live PDF extension document and identify the first missing hop for zoom-in,
zoom-out, and rotate. Fit mode should be traced as a positive control because it
works in Experiment 10.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Add a live PDF toolbar event-chain probe.

   Prefer extending `scripts/probe-pdf-toolbar.mjs` with a focused trace mode or
   adding a companion script such as `scripts/probe-pdf-toolbar-events.mjs`.
   Reuse `scripts/test-issue-794-pdf-toolbar.py` or add a wrapper that launches
   the local Bitcoin PDF fixture in debug Roamium exactly like Experiment 10.

   The probe must attach to the PDF extension child target, not just the
   top-level PDF wrapper. It should record the child target/session id and URL.

2. Inject temporary JavaScript instrumentation into the PDF extension frame.

   Before clicking any toolbar controls, evaluate a script in the PDF extension
   child target that:
   - locates `<pdf-viewer id="viewer">`;
   - locates `<viewer-toolbar id="toolbar">` inside the viewer's open shadow
     root;
   - locates the zoom-in, zoom-out, rotate, and fit controls inside the
     toolbar's open shadow root;
   - adds capture and bubble listeners for:
     - native `click`;
     - `zoom-in`;
     - `zoom-out`;
     - `rotate-left`;
     - `fit-to-changed`;
   - wraps, if present and writable:
     - `toolbar.onZoomInClick_`;
     - `toolbar.onZoomOutClick_`;
     - `toolbar.onRotateClick_`;
     - `toolbar.onFitToButtonClick_`;
     - `viewer.onZoomIn`;
     - `viewer.onZoomOut`;
     - `viewer.onRotateLeft_`;
     - `viewer.onFitToChanged`;
     - `viewer.viewport_.zoomIn`;
     - `viewer.viewport_.zoomOut`;
     - `viewer.viewport_.setFittingType`;
     - `viewer.currentController.rotateCounterclockwise`;
   - pushes every observed hop into `window.__termsurfPdfToolbarTrace`.

   The injected instrumentation is diagnostic only. It must not replace the
   methods with behavior-changing stubs; wrappers must call the original method
   with the original `this` and arguments. If a property cannot be wrapped,
   record `wrap_failed` with the reason instead of failing silently.

   Method wrapping must check both own properties and prototype-chain
   properties. For every requested method, record:
   - whether it was found;
   - whether it was found as an own property or on which prototype depth;
   - descriptor shape (`value`, `get`, `set`, writable/configurable flags);
   - whether it was a function;
   - whether wrapping was installed.

   Only wrap function descriptors that can be safely replaced and restored. If
   the descriptor is absent, non-function, non-writable, or non-configurable,
   record `wrap_failed` with one of `not-found`, `not-function`, `not-writable`,
   or `not-configurable`.

   Wrapped methods must record:
   - method name;
   - control/action id active at the time;
   - arguments count and safe primitive argument summary;
   - whether the original returned normally;
   - whether the original threw, including exception text.

   A thrown original method is a distinct result from "method fired but state
   did not change."

3. Activate each control with the same method used in Experiment 10.

   Use CDP mouse activation in the PDF extension child target for:
   - zoom in;
   - zoom out;
   - rotate;
   - fit mode positive control.

   Record the target/session, control identity, coordinate space, x/y, and
   screenshot before/after. The probe must not use direct method calls as the
   activation path; direct calls would bypass the broken click/event path.

   Before the first click, run an instrumentation self-check and write it to the
   log. The self-check must confirm:
   - PDF extension child target selected;
   - `<pdf-viewer id="viewer">` found;
   - `<viewer-toolbar id="toolbar">` found;
   - each target control found or explicitly missing;
   - event listeners installed;
   - wrapper installation result for every requested method.

   Then run fit mode as the positive control after instrumentation is installed.
   Fit must still change state while instrumented; otherwise the instrumentation
   may have perturbed the viewer and the run is Partial/Fail rather than useful
   evidence about zoom/rotate.

4. Collect trace and state after each activation.

   For each control, collect:
   - `window.__termsurfPdfToolbarTrace`;
   - zoom text/value and page geometry;
   - page area screenshot;
   - relevant Roamium/Chromium/PDF traces already available from earlier
     experiments if `TERMSURF_PDF_INPUT_TRACE` is enabled.

   Reset or tag the trace per control. Prefer assigning an
   `activeActionId`/`activeControl` before each click and filtering results to
   that action id. At minimum, snapshot the trace length before each click and
   only classify newly appended entries. A working fit trace must not
   contaminate the later zoom/rotate classification.

   Custom event listeners must be installed at multiple relevant levels:
   - target control;
   - toolbar shadow root;
   - `<viewer-toolbar>`;
   - viewer shadow root;
   - `<pdf-viewer>`;
   - `document`.

   Record capture/bubble phase where observable. This prevents Shadow DOM
   composed/bubbling behavior from making a fired event look missing.

   Classify each feature with a first failing hop:
   - `control-not-found`;
   - `click-not-observed`;
   - `toolbar-handler-not-called`;
   - `custom-event-not-dispatched`;
   - `viewer-handler-not-called`;
   - `viewport-method-not-called`;
   - `controller-method-not-called`;
   - `method-threw`;
   - `state-did-not-change-after-method`;
   - `no-failure-observed`.

5. Preserve Experiment 10 regressions if product code changes are made.

   Experiment 11 is expected to be diagnostic-only. If it only changes the
   harness, run syntax checks and the toolbar trace. If the trace identifies and
   fixes a product bug in the same experiment, then run the full Experiment 10
   regression set:
   - PDF wheel scroll;
   - PDF keyboard select/copy;
   - PDF drag selection;
   - HTML click smoke.

   Do not make product changes before recording the first diagnostic run. If the
   diagnostic result makes the fix obvious, record the diagnostic first, then
   either keep the fix in a clearly separated second half of this experiment or
   design Experiment 12 for the fix.

6. Formatting and review.

   If Markdown changes, run Prettier. If Rust changes, run `cargo fmt` and
   accept its output. If Chromium code changes, follow the Chromium branch rule.

## Verification

1. Run the event-chain trace against the local Bitcoin PDF fixture.

   Required artifacts:
   - command log;
   - Roamium stdout/stderr;
   - HTTP fixture log;
   - injected instrumentation setup result;
   - per-control trace JSON;
   - per-control before/after state and screenshot paths;
   - summary table with first failing hop for zoom in, zoom out, rotate, and fit
     positive control.

2. Required result table:

   | Feature  | Click observed | Toolbar handler | Custom event | Viewer handler | Viewport/controller method | State changed | First failing hop |
   | -------- | -------------- | --------------- | ------------ | -------------- | -------------------------- | ------------- | ----------------- |
   | Zoom in  |                |                 |              |                |                            |               |                   |
   | Zoom out |                |                 |              |                |                            |               |                   |
   | Rotate   |                |                 |              |                |                            |               |                   |
   | Fit mode |                |                 |              |                |                            |               |                   |

3. Required interpretation:
   - If fit mode traces cleanly but zoom/rotate do not, compare their divergence
     point in code.
   - If zoom/rotate handlers fire but viewport/controller methods do not, the
     fix target is the viewer handler/event binding layer.
   - If viewport/controller methods fire but state does not change, the fix
     target is the PDF viewer model/plugin propagation layer.
   - If the click is not observed, the fix target is the control activation or
     hit-test layer.

4. Codex must review the completed output.

   Do not proceed to Experiment 12 until real issues from Codex's completion
   review are addressed.

## Pass Criteria

Experiment 11 passes if:

- it identifies the first missing hop for zoom-in, zoom-out, and rotate;
- fit mode acts as a positive control and traces through its working path;
- the result names the exact next fix target.

## Partial Criteria

Experiment 11 is partial if:

- the instrumentation can observe clicks and custom events, but method wrapping
  is blocked by compiled/private implementation details;
- the trace identifies a likely failing layer but not a single exact hop;
- the harness changes needed to observe the event chain are larger than expected
  and should be split before product changes.

## Failure Criteria

Experiment 11 fails if:

- it changes product behavior before recording a diagnostic trace;
- it calls viewer methods directly and treats that as proof of toolbar click
  behavior;
- it relies on screenshot diffs without event-chain evidence;
- it omits the fit-mode positive control;
- it opens native save/download/print UI;
- it uses installed/stable Roamium instead of repo-built debug Roamium;
- it omits Codex design or completion review.

## Result

**Result:** Pass

The diagnostic harness was implemented in `scripts/probe-pdf-toolbar-events.mjs`
and run through `scripts/test-issue-794-pdf-toolbar.py --probe events` against
the local Bitcoin PDF fixture.

Final run:

```bash
LOG_DIR="logs/issue-794-exp11-toolbar-events-20260530-104229" \
  scripts/test-issue-794-pdf-toolbar.py \
  --probe events \
  --log-dir "$LOG_DIR" \
  --serve-bitcoin-pdf \
  --pdf-port 9809
```

Artifacts:

- `logs/issue-794-exp11-toolbar-events-20260530-104229/pdf-toolbar-summary.json`
- `logs/issue-794-exp11-toolbar-events-20260530-104229/toolbar-events/toolbar-events-summary.json`
- per-control traces under
  `logs/issue-794-exp11-toolbar-events-20260530-104229/toolbar-events/`

The harness run status is `partial` because zoom in/out still fail as product
features. The experiment result is `Pass` because Experiment 11 was diagnostic:
it identified the first failing hop for zoom, proved fit mode as a positive
control, and clarified that rotate is not on the remaining failure path.

The setup self-check passed:

- the probe selected the PDF extension child target;
- `<pdf-viewer id="viewer">` was found;
- `<viewer-toolbar id="toolbar">` was found;
- zoom-in, zoom-out, rotate, and fit controls were found;
- all requested wrappers were installed, including prototype-chain methods.

Result table:

| Feature  | Click observed | Toolbar handler | Custom event | Viewer handler | Viewport/controller method | State changed | First failing hop     |
| -------- | -------------- | --------------- | ------------ | -------------- | -------------------------- | ------------- | --------------------- |
| Zoom in  | yes            | yes             | yes          | yes            | yes                        | no            | `method-threw`        |
| Zoom out | yes            | yes             | yes          | yes            | yes                        | no            | `method-threw`        |
| Rotate   | yes            | yes             | yes          | no             | yes                        | yes           | `no-failure-observed` |
| Fit mode | yes            | no              | yes          | no             | yes                        | yes           | `no-failure-observed` |

Important details:

- Fit mode reached `viewer.viewport_.setFittingType`, returned normally, and
  changed `viewport_.fittingType_` from `none` to `fit-to-page`.
- Rotate reached `viewer.currentController.rotateCounterclockwise`, returned
  normally, and changed `viewer.clockwiseRotations_` from `0` to `3`.
- Zoom in reached the full path through the toolbar handler, `zoom-in` custom
  event, viewer handler, and `viewer.viewport_.zoomIn`, but `Viewport.zoomIn()`
  threw Chromium's `Error: Assertion failed` from `Viewport.mightZoom_`. Any
  state changes before or around the throw were incidental setup changes, not a
  successful zoom state change.
- Zoom out reached the full path through the toolbar handler, `zoom-out` custom
  event, viewer handler, and `viewer.viewport_.zoomOut`, but
  `Viewport.zoomOut()` threw the same `Viewport.mightZoom_` assertion.

No product behavior was changed in this experiment. The only code changes were
diagnostic harness changes.

## Conclusion

The toolbar click/event path is not the remaining problem. Zoom and rotate
controls are reachable inside the PDF extension child target, and the toolbar
custom events are dispatched.

Rotate now appears functional by internal PDF viewer state:
`rotateCounterclockwise()` runs and updates `clockwiseRotations_`. Experiment
10's rotate result was a harness limitation because it only looked for shallow
state changes.

The exact remaining toolbar failure is zoom. Both zoom-in and zoom-out fail
inside Chromium's PDF viewer `Viewport.mightZoom_` assertion after
`allowedToChangeZoom_` has been set true and after the viewer has switched out
of fit mode. The next experiment should inspect Chromium's PDF viewer
`Viewport.mightZoom_` preconditions and identify which embedder-provided
viewport/page/zoom state is still invalid in TermSurf's Electron-style PDF
embedding.
