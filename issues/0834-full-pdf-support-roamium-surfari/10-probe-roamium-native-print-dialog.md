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

## Result

**Result:** Partial

Implemented a probe-only native print harness:

- `scripts/test-issue-834-pdf-native-print.py` owns the safety gate, harmless
  native-dialog preflight, print queue snapshots, Roamium launch, TermSurf
  protocol tab creation, and final `pdf-native-print-summary.json`.
- `scripts/probe-pdf-save-print-title-local.mjs` gained a narrow
  `--allow-native-print-click` path. Without that explicit flag, it preserves
  the previous behavior and refuses to click production print when no contained
  intercept file is configured.

No Chromium, Roamium, Ghostboard, protocol, or other product source files were
changed.

Final verification commands:

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

Additional checks:

```bash
node --check scripts/probe-pdf-save-print-title-local.mjs
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile scripts/test-issue-834-pdf-native-print.py
git diff --check
```

Final evidence:

- `logs/issue-834-exp10-contained-print/pdf-toolbar-summary.json`:
  `first_failing_hop = "no-failure-observed"`,
  `print.status = "print-contained-callback"`,
  `print.bridgeClassification = "print-reaches-contained-intercept"`,
  `print.printGuardState.has_engine = "1"`,
  `print.printGuardState.can_print = "1"`, and four fresh contained intercept
  lines were written. No native print dialog was opened by this contained run.
- `logs/issue-834-exp10-native-print-safety-gate/pdf-native-print-summary.json`:
  `first_failing_hop = "native-print-safety-gate-not-ready"`,
  `safety_gate_passed = false`, the harmless preflight used
  `osascript-display-dialog-plus-system-events`, and System Events returned
  `osascript is not allowed assistive access. (-25211)`. The print queues
  reported by `lpstat -o` and `lpstat -W completed -o` were empty.
- `logs/issue-834-exp10-native-print-dialog/pdf-native-print-summary.json`: even
  with `--allow-native-dialog-click`,
  `first_failing_hop = "native-print-safety-gate-not-ready"`,
  `safety_gate_passed = false`, `server_register_received = false`, and
  `devtools_port = null`. This proves the harness refused to launch/click
  production print because the harmless preflight did not pass.

The experiment did not prove native print dialog support because the cancel-only
watcher could not pass the required harmless-dialog preflight on this VM.
Specifically, macOS denied assistive access to `osascript`/System Events, so the
watcher could not objectively observe and dismiss even a harmless non-print
dialog. The approved design forbids production print clicks unless that
preflight passes, so the native print click was correctly blocked.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

Initial verdict: **Changes Required**.

Required finding:

- `scripts/__pycache__/test-issue-834-pdf-native-print.cpython-314.pyc` was
  present after `py_compile`, violating the verification requirement to remove
  `scripts/__pycache__/`.

Fix:

- Removed `scripts/__pycache__/`.

Re-review verdict: **Approved**. The reviewer confirmed no Required findings
remained and the working tree contained only the intended experiment files.

## Conclusion

Roamium's contained PDF print bridge remains healthy, but the real native print
dialog row is still unproven. The current blocker is automation safety, not yet
product behavior: the VM needs an approved watcher mechanism, most likely
granting assistive access to the `osascript`/System Events path or replacing the
watcher with another mechanism that can objectively observe and cancel a
harmless native dialog before any production print click.

The next experiment should either establish a working macOS native-dialog
watcher preflight or use a different objectively safe observation/cancel
mechanism. Only after that preflight passes should we click the production PDF
print control.
