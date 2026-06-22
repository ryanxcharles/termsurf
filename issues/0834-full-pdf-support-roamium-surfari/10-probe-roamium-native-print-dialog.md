# Experiment 10: Probe Roamium Native Print Dialog Safely

## Description

Experiment 2 proved the current Roamium PDF print path reaches the contained
print intercept:

- the PDF toolbar print control can be exposed in contained mode;
- JavaScript print activation reaches the Chromium PDF plugin bridge;
- `PdfViewWebPlugin::Print()` sees `has_engine = 1` and `can_print = 1`;
- the contained intercept writes `pdf-print.log`;
- no native dialog or real print job is submitted in contained mode.

That is not the same as proving native PDF print support. The Issue 834 matrix
still has real native print UI behavior as a required Roamium row. This
experiment should safely answer whether current Roamium can open the native
print dialog from the real PDF viewer path, and if not, classify the first
missing layer.

The experiment must never submit a real print job. It may only click production
print when a cancel-only OS watcher is active and able to prove that it can
detect and dismiss the dialog. If safe cancellation cannot be proven, do not
click production print; record a Partial result with the blocker.

## Changes

1. Reconfirm the existing contained print control.

   Reuse the current save/print/title/local probe in contained mode:

   ```bash
   python3 scripts/test-issue-794-pdf-toolbar.py \
     --probe save-print-title-local \
     --log-dir logs/issue-834-exp10-contained-print \
     --serve-bitcoin-pdf \
     --enable-pdf-print-intercept
   ```

   Required evidence:

   - `print.status = "print-contained-callback"`;
   - `print.bridgeClassification = "print-reaches-contained-intercept"`;
   - `print.printGuardState.has_engine = "1"`;
   - `print.printGuardState.can_print = "1"`;
   - fresh `pdf-print.log` lines exist;
   - no native print dialog is opened;
   - no real print job is submitted.

2. Add a safe native-dialog probe harness.

   Add `scripts/test-issue-834-pdf-native-print.py` and extend
   `scripts/probe-pdf-save-print-title-local.mjs` only as narrowly as needed.

   The harness should:

   - launch repo-built `chromium/src/out/Default/roamium`;
   - serve `test-html/public/bitcoin.pdf`;
   - create a tab through the TermSurf protocol;
   - attach DevTools to the PDF viewer path;
   - verify the print control is present and production printing is enabled;
   - start an OS watcher before clicking print;
   - prove the watcher with a harmless non-print native-dialog preflight before
     any production print click;
   - click the print control only when `--allow-native-dialog-click` is passed;
   - cancel the native print dialog by pressing Escape or the Cancel button;
   - record whether a native print dialog/window was observed;
   - record whether cancellation was sent and whether the dialog disappeared;
   - record print queue state before and after using `lpstat` or the closest
     macOS-safe equivalent available on this VM;
   - write `<log-dir>/pdf-native-print-summary.json`.

   The OS watcher may use AppleScript/System Events, screenshots, accessibility
   APIs, or another local macOS mechanism. It must record the mechanism used and
   the exact observation/cancel evidence.

3. Keep production clicking behind a hard safety gate.

   The native-dialog probe must refuse to click production print unless all of
   these are true:

   - `--allow-native-dialog-click` is present;
   - the OS watcher reports it is ready before the click;
   - a harmless non-print native-dialog preflight has opened a dialog/window,
     observed it, cancelled/dismissed it, and verified disappearance using the
     same watcher mechanism that will watch the print dialog;
   - initial print queue state was captured;
   - the probe has a bounded timeout;
   - the only planned post-click action is cancel/dismiss, never submit.

   If any safety precondition is missing, the probe exits non-zero with
   `first_failing_hop = "native-print-safety-gate-not-ready"` and does not click
   print.

4. Classify the first missing layer.

   The result must classify one of:

   - `contained-print-control-failed`;
   - `native-print-safety-gate-not-ready`;
   - `native-print-control-missing`;
   - `native-print-disabled-by-load-time-flags`;
   - `native-print-click-not-sent`;
   - `native-print-click-sent-no-dialog`;
   - `native-print-dialog-seen-cancelled`;
   - `native-print-dialog-seen-cancel-failed`;
   - `native-print-job-submitted-unexpectedly`;
   - `native-print-observation-gap`.

5. Do not fix product code in this experiment.

   Experiment 10 is probe/classification-only. If the native-dialog probe proves
   a real TermSurf, Roamium, Chromium, Ghostboard, or protocol integration gap,
   record a Partial result with the first missing layer and design a follow-up
   implementation experiment. Do not make product source changes here.

## Verification

Verification for the completed result is:

```bash
python3 scripts/test-issue-794-pdf-toolbar.py \
  --probe save-print-title-local \
  --log-dir logs/issue-834-exp10-contained-print \
  --serve-bitcoin-pdf \
  --enable-pdf-print-intercept

python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp10-native-print-safety-gate \
  --probe safety-gate

python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp10-native-print-dialog \
  --probe native-dialog \
  --allow-native-dialog-click
```

The native-dialog command is allowed only after the safety-gate command proves
the watcher is ready and the implementation records why it is safe to click. The
proof must include a harmless non-print native-dialog preflight that opens a
dialog/window, observes it, cancels/dismisses it, and verifies disappearance
using the same watcher mechanism intended for the print dialog.

Required checks:

- contained print still reaches the contained intercept;
- the safety-gate probe refuses to click when the watcher is not ready or the
  harmless native-dialog preflight has not passed;
- the safety-gate probe records the preflight mechanism, observation,
  cancellation, and disappearance evidence;
- the native-dialog probe records OS watcher readiness, click decision, dialog
  observation, cancellation evidence, queue state before/after, and first
  failing hop;
- no real print job is submitted;
- if no native dialog appears, the result includes bridge/control evidence
  proving the click was sent before classifying
  `native-print-click-sent-no-dialog`;
- no Chromium, Roamium, Ghostboard, protocol, or other product source files are
  changed;
- any new Node helper passes `node --check`;
- any new Python helper passes
  `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile`, and `scripts/__pycache__/`
  is removed afterward;
- markdown is formatted with Prettier;
- `git diff --check` passes;
- design review is recorded, all required design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Pass Criteria

This experiment passes if current Roamium opens the native PDF print dialog from
the real PDF viewer path, the harness cancels it, and queue evidence proves no
real print job was submitted.

## Partial Criteria

This experiment is partial if contained print still works but native dialog
testing cannot safely click, cannot observe/cancel the dialog, or classifies a
real product gap that needs a follow-up implementation experiment.

## Failure Criteria

This experiment fails if it submits a real print job, clicks production print
without a ready cancel-only watcher, regresses contained print, or claims native
print support without dialog and no-job evidence.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Initial verdict: **Changes Required**.

Required findings:

- The experiment scope was too broad because it allowed product-code fixes in
  what should be a probe/classification experiment. The design now forbids
  product source changes and requires any product gap to be recorded as Partial
  for a follow-up implementation experiment.
- The safety gate was self-attested because it only required the watcher to
  report readiness. The design now requires a harmless non-print native-dialog
  preflight that opens a dialog/window, observes it, cancels/dismisses it, and
  verifies disappearance using the same watcher mechanism before any production
  PDF print click.

Re-review verdict: **Approved**.
