# Experiment 47: Trace Surfari PDF Copy Target

## Description

Experiment 46 proved that standalone macOS copy oracles are valid: the same
external CGEvent `Cmd+C` route copies selected text from standalone
`NSTextView`, PDFKit `PDFView`, and standalone `WKWebView` PDF. The same
Surfari-in-Ghostboard path still leaves the clipboard unchanged.

The next step is to inspect Surfari's embedded `WKWebView` state at the moment
Ghostboard forwards PDF selection and `Cmd+C`. This experiment should determine
whether Surfari has a real selection, whether the `WKWebView` is the active copy
target, and whether an in-process copy action succeeds when the external key
path does not.

## Changes

- Add env-gated diagnostic tracing to Surfari/WebKit integration code, enabled
  only for this experiment, tentatively via `TERMSURF_SURFARI_PDF_COPY_TRACE=1`.
- Trace Surfari state around the existing PDF selection/copy harness:
  - when mouse drag events reach Surfari;
  - when focus changes reach Surfari;
  - immediately before handling forwarded `Cmd+C`;
  - immediately after handling forwarded `Cmd+C`;
  - after any env-gated in-process copy probe.
- Record at least:
  - tab ID and pane ID;
  - current URL;
  - whether the target view is the expected `WKWebView`;
  - `WKWebView` frame/bounds;
  - key window and first responder class/name where available;
  - `NSApp.target(forAction:to:from:)` for `copy:` before external `Cmd+C` and
    before any in-process copy probe, including target class/identity where
    available;
  - first-responder chain details where available, not only the leaf first
    responder;
  - whether Surfari believes the tab is focused;
  - result of a JavaScript selection probe such as
    `window.getSelection()?.toString()` where available;
  - whether `NSApp.sendAction(copy:, to: nil, from: nil)` returns true when run
    under the env-gated probe;
  - clipboard sample/hash before and after the in-process probe.
- Treat PDF selection evidence carefully:
  - empty or unavailable DOM selection is not enough to prove there is no PDF
    selection, because native PDF selection inside `WKWebView` may not surface
    as DOM selection;
  - a successful in-process copy of the marker is valid evidence that selection
    existed even if DOM selection is empty;
  - a copy-target trace identifying the expected `WKWebView` or PDF responder
    plus failed copy is copy-target evidence, not selection evidence by itself;
  - if public APIs cannot prove selection or non-selection, classify the result
    as `trace-insufficient` rather than claiming `no-selection-in-surfari` or
    `selection-present-copy-fails`;
  - require screenshot, overlay geometry, and drag-coordinate evidence from the
    harness to show the drag targeted the visible PDF text region.
- Update or add a harness, tentatively
  `scripts/test-issue-834-surfari-pdf-copy-target.sh`, that:
  - runs the existing Surfari PDF selection/copy fixture with tracing enabled;
  - preserves clipboard state with the same save-once/trap-restore rules as
    Experiments 45 and 46;
  - first runs a passive trace baseline with the normal external `Cmd+C` path
    and no focus/copy-target intervention;
  - if the passive baseline still fails, triggers the env-gated in-process copy
    probe and records whether that copies the marker;
  - if a focus or copy-target intervention is needed for diagnosis, runs it only
    as a separate env-gated diagnostic probe after the passive baseline, and
    records it as a candidate for a later product-fix experiment;
  - keeps normal external-copy and in-process-copy results separate.
- Apply this outcome matrix:
  - **external-copy-baseline-pass:** the passive normal external `Cmd+C` path
    copies the marker with tracing enabled and no intervention;
  - **external-copy-fixed-by-focus-probe:** the passive baseline fails, but a
    separate env-gated focus/copy-target diagnostic intervention makes external
    copy work, producing a candidate for a later product-fix experiment;
  - **inprocess-copy-succeeds:** external `Cmd+C` fails, but an in-process
    `copy:` action copies the marker, pointing at focus/responder/key routing
    inside Surfari or Ghostboard;
  - **no-selection-in-surfari:** neither external nor in-process copy can copy
    the marker, and trace evidence indicates Surfari's `WKWebView` has no
    selection after drag;
  - **selection-present-copy-fails:** trace evidence indicates selection exists,
    but both external and in-process copy fail;
  - **trace-insufficient:** the trace cannot prove selection/copy target state
    strongly enough to choose among the cases above.
- Map result status:
  - **Pass:** any of `external-copy-baseline-pass`,
    `external-copy-fixed-by-focus-probe`, `inprocess-copy-succeeds`,
    `no-selection-in-surfari`, or `selection-present-copy-fails`, with complete
    trace and clipboard evidence;
  - **Partial:** `trace-insufficient` with useful logs;
  - **Fail:** clipboard restore failure, missing trace evidence, no Surfari
    probe, or an unreviewed behavior change outside the env-gated diagnostic.
- If this experiment identifies a product fix, do not silently roll it into the
  trace result. Record the fix candidate and design the next experiment around
  applying it deliberately.
- Update this experiment file with the result.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-surfari-pdf-copy-target.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
```

Run the diagnostic harness:

```bash
rm -rf logs/issue-834-exp47-surfari-pdf-copy-target
scripts/test-issue-834-surfari-pdf-copy-target.sh
```

Pass criteria:

- tracing is gated behind an explicit experiment environment variable;
- normal external `Cmd+C` and in-process copy probe results are recorded
  separately;
- the passive baseline runs before any focus/copy-target intervention;
- trace lines prove enough Surfari state to classify the result using the
  outcome matrix;
- selection and non-selection claims follow the PDF selection evidence rules
  above;
- copy-target claims include `NSApp.target(forAction:to:from:)` evidence, not
  only `NSApp.sendAction` return values;
- clipboard state is saved once, restored from every exit path, and final
  restoration status is recorded;
- Surfari build/format checks are run for touched code;
- no non-diagnostic product behavior change is made;
- completion review is recorded.

Partial criteria:

- Surfari trace captures useful focus/responder/copy evidence but cannot prove
  selection state;
- the in-process copy probe cannot be triggered reliably, but normal forwarded
  key and focus traces are useful;
- the harness reproduces the Surfari failure but one trace source is unavailable
  due public WebKit API limits.

Failure criteria:

- the harness cannot reproduce the Surfari PDF copy failure;
- trace mode changes user-visible behavior without recording that as a
  deliberate finding;
- clipboard state is not restored;
- the result claims a product fix without proving it through the trace.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Findings:

- PDF selection state was underspecified because native PDF selection inside
  `WKWebView` may not appear through DOM selection APIs;
- copy-target state was underspecified because `NSApp.sendAction(copy:)`
  returning true does not identify the responder that accepted the action;
- verification did not build the Surfari/WebKit code the experiment plans to
  touch;
- the `external-copy-fixed-by-focus` class risked mixing passive diagnosis with
  a behavior-changing focus correction.

Resolution:

- added explicit PDF selection evidence rules: DOM selection alone cannot prove
  no selection, successful in-process copy can prove selection, and unavailable
  public APIs must produce `trace-insufficient` rather than overclaiming;
- required `NSApp.target(forAction:to:from:)` copy-target tracing and
  first-responder chain details where available;
- added Surfari format/build verification with
  `cargo fmt -p surfari -- --check`, `surfari/libtermsurf_webkit/build.sh`, and
  `cargo build -p surfari`;
- required a passive baseline before any focus/copy-target intervention and
  renamed the focus outcome to `external-copy-fixed-by-focus-probe` to make it a
  candidate for a later product-fix experiment.

Follow-up verdict: **Approved**.

The reviewer found no remaining must-fix design issues and approved the
Experiment 47 plan commit.

## Result

**Result:** Partial

The Surfari copy-target trace was added behind
`TERMSURF_SURFARI_PDF_COPY_TRACE=1`, with optional in-process copy probing
behind `TERMSURF_SURFARI_PDF_COPY_INPROCESS=1`. The harness was added as
`scripts/test-issue-834-surfari-pdf-copy-target.sh`.

Verification:

```bash
bash -n scripts/test-issue-834-surfari-pdf-copy-target.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
rm -rf logs/issue-834-exp47-surfari-pdf-copy-target
scripts/test-issue-834-surfari-pdf-copy-target.sh
```

The diagnostic run was `20260622-230806`. Its summary is:

```text
logs/issue-834-exp47-surfari-pdf-copy-target/surfari-pdf-copy-target-summary.json
```

The run classified the result as:

```json
{
  "classification": "trace-insufficient",
  "overall_result": "partial"
}
```

Key evidence:

- The passive baseline reproduced the existing Surfari PDF copy failure:
  `surfari-pdf-selection-copy-partial`.
- The trace shows Surfari receives focus, drag, and `Cmd+C`, and that
  `WKWebView` is the first responder.
- The host `TSHostWindow` is not key or main, and `NSApp.keyWindow` is `nil`.
- `NSApp.target(forAction: copy:, to: nil, from: nil)` and
  `NSApp.target(forAction: copy:, to: nil, from: webView)` both returned `nil`.
- DOM selection remained empty, while the active element became `EMBED`; this is
  not enough to prove there is no native PDF selection.
- In-process probe routes showed mixed behavior:
  - responder-chain `sendAction(copy:)` returned `ok=0`;
  - direct `sendAction(copy:)` to `WKWebView` returned `ok=1` but did not copy
    the marker on the first copy attempt;
  - direct `WKWebView.copy(nil)` was invoked because `WKWebView` responds to
    `copy:`, but it did not copy the marker on the first copy attempt;
  - on the later fallback copy attempt, the clipboard changed to
    `TS834PDFCOPYQXJ`, missing the final `Z`, so the full accepted marker still
    was not proven.
- Clipboard restoration succeeded.

Important artifacts:

- `logs/issue-834-exp47-surfari-pdf-copy-target/baseline-copy-trace-20260622-230806.log`
- `logs/issue-834-exp47-surfari-pdf-copy-target/inprocess-copy-trace-20260622-230806.log`
- `logs/issue-834-exp47-surfari-pdf-copy-target/baseline-exp44-summary-20260622-230806.json`
- `logs/issue-834-exp47-surfari-pdf-copy-target/inprocess-exp44-summary-20260622-230806.json`

No non-diagnostic product behavior change was made. The new code is gated by
experiment environment variables.

## Conclusion

Experiment 47 moved the failure boundary inward but did not yet prove a final
root cause. The strongest findings are:

- Surfari's hidden host window is not key/main, `NSApp.keyWindow` is `nil`, and
  AppKit cannot find a normal `copy:` target even though the `WKWebView` is the
  first responder.
- DOM selection is not useful for native PDF selection here: it stays empty with
  `EMBED` active.
- Direct WKWebView copy calls can report success, but they still do not reliably
  copy the full marker in the embedded Surfari path.

The next experiment should test a focused candidate fix around Surfari's AppKit
responder/key-window state or around the exact selection coordinates used by the
Surfari PDF copy harness. That fix should be separated from tracing and verified
against the same copy-oracle evidence from Experiment 46.

## Completion Review

An external Codex review checked the completed experiment, harness, trace code,
logs, and result text.

Initial verdict: **Changes required**.

Findings:

- the trace logged `performWebViewCopy ok=1`, which overstated what happened:
  the probe only knew that `WKWebView` responded to `copy:` and that the call
  was invoked, not that it returned a success value or copied text;
- the completion review had not yet been recorded in this experiment file.

Resolution:

- changed the trace wording to `responds=1 invoked=1` / `responds=0 invoked=0`
  for the direct `WKWebView.copy(nil)` probe;
- updated the result text to say the direct `copy:` call was invoked but did not
  copy the marker;
- reran the diagnostic after that fix, producing run `20260622-230806`;
- added this completion-review section before the result commit.

The reviewer found no other must-fix issues. The review confirmed that `Partial`
/ `trace-insufficient` is supported: the baseline reproduces the failure,
in-process probing does not prove the full accepted marker, DOM selection is
treated cautiously, copy-target traces are useful but not decisive, clipboard
restoration is recorded as restored, and the diagnostic code is env-gated.

Follow-up verdict: **Approved**.

The reviewer found no remaining must-fix issues and approved the Experiment 47
result commit.
