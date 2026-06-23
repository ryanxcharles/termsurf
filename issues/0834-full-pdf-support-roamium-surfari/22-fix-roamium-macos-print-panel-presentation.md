# Experiment 22: Fix Roamium macOS Print Panel Presentation

## Description

Experiment 21 proved that Roamium reaches Chromium's macOS print implementation
and enters `[panel runModalWithPrintInfo:]`, but the call never produces an
observable `Print` / `Printer` window and never returns a modal response before
the guarded harness terminates Roamium.

The known good state before the failure is:

- `TsPdfPrintManager::ScriptedPrint()` calls `AskUserForSettings()`;
- `PrintingContextMac::AskUserForSettings()` runs on the main thread;
- the delegate has a native parent view;
- that view has a native parent window;
- the `CATransaction` completion block is installed and entered;
- `[panel runModalWithPrintInfo:]` is entered;
- no modal response is recorded;
- the watcher sees Roamium's existing `Content Shell` window but no separate
  print-panel candidate;
- the print queues remain empty.

This experiment should determine whether the stuck modal is caused by AppKit
presentation state in Roamium's content-shell embedding, then apply the
narrowest safe fix if one is proven. Candidate causes include inactive
application/window state at the moment of presentation, `runModal` being a poor
fit for this embedding, and the need to use a sheet/app-modal presentation path
similar to content shell's JavaScript dialog path.

## Changes

1. Create a fresh Chromium branch for this issue experiment.

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-834-exp21
   git checkout -b 148.0.7778.97-issue-834-exp22
   ```

   Update the branch table in `chromium/README.md`.

2. Add pre-presentation AppKit state trace points immediately before showing the
   print panel.

   Extend the Experiment 21 trace with:

   - `NSApp.activationPolicy`;
   - `NSApp.isActive`;
   - whether the parent window is key, main, visible, miniaturized, and ordered;
   - whether the parent window can become key/main;
   - whether `NSApp.keyWindow` and `NSApp.mainWindow` match the parent window;
   - the print panel class and whether it is visible before presentation.

   These are diagnostic events only; they must not click, submit, or dismiss a
   print dialog.

3. Try one presentation adjustment at a time, guarded by trace evidence.

   The preferred order is:

   1. If the app/window is inactive, make the parent window key/front and
      activate the app immediately before print-panel presentation.
   2. If activation does not fix the stuck modal, try presenting the print panel
      as a sheet on the parent window using
      `beginSheetWithPrintInfo: modalForWindow:delegate:didEndSelector:contextInfo:`
      or the modern block equivalent if available in this SDK.
   3. If sheet presentation is used, preserve the asynchronous callback contract
      and return `kCanceled` on watcher cancellation. Do not treat OK /
      `kSuccess` as a safe pass unless queue evidence proves no job was
      submitted, and do not count OK as cancellation.

   The experiment should stop after the first proven improvement. Do not pile
   multiple unproven AppKit workarounds into one result.

4. Keep the native-print safety gate intact.

   The harness must still:

   - require `--allow-native-dialog-click` for any production print click;
   - run the harmless preflight before the production click;
   - capture print queues before and after;
   - cancel any observed dialog;
   - hard-fail if a print job is submitted unexpectedly;
   - classify modal OK / callback `kSuccess` as a safety failure unless no-job
     evidence proves otherwise, and never count it as a safe cancellation.

5. Run the guarded native-print probe after each attempted presentation change.

   A passing result requires an observed native print panel, successful
   automated cancellation, unchanged print queues, and trace evidence showing
   the print path returned through the cancellation callback.

## Verification

Verification for the completed result is:

```bash
git status --short
git -C chromium/src status --short
git -C chromium/src rev-parse --abbrev-ref HEAD
git -C chromium/src rev-parse HEAD
git diff --check

cd chromium/src
export PATH="/Users/astrohacker/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium

cd /Users/astrohacker/dev/termsurf
rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-native-print.py
rm -rf scripts/__pycache__
node --check scripts/probe-pdf-save-print-title-local.mjs

rm -rf logs/issue-834-exp22-macos-print-panel-presentation
python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp22-macos-print-panel-presentation \
  --probe native-dialog \
  --allow-native-dialog-click

git diff --check
```

After committing Chromium branch changes inside `chromium/src`, regenerate the
cumulative Issue 834 patch archive from the local Chromium 148.0.7778.97 shallow
base:

```bash
cd /Users/astrohacker/dev/termsurf/chromium/src
rm -rf ../patches/issue-834
git format-patch 6b3fa66a923a9442c8ab0bc71b4b41ff24528d3b..HEAD \
  -o ../patches/issue-834
```

Required evidence:

- `chromium/README.md` records the new Chromium branch;
- Chromium source changes are committed inside `chromium/src`;
- `autoninja -C out/Default libtermsurf_chromium` passes;
- the Issue 834 patch archive is regenerated;
- the guarded native print probe records AppKit activation/window/panel state;
- no print job is submitted;
- if a native dialog appears, it is cancelled and queue state remains unchanged;
- if the dialog still does not appear, the result identifies the next precise
  failing sub-hop and whether activation/sheet presentation changed behavior;
- markdown is formatted with Prettier;
- Python bytecode cache is removed after compilation;
- `git diff --check` passes;
- design review is recorded, all real design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Pass Criteria

This experiment passes if Roamium native PDF print opens a native macOS print
panel, the safety watcher cancels it, the modal/callback path reports
cancellation rather than OK, and print queue evidence proves no job was
submitted.

## Partial Criteria

This experiment is partial if native print still does not pass but the result
proves a more precise AppKit presentation cause than Experiment 21, or proves
that one attempted presentation adjustment changes the failing sub-hop without
completing safe cancellation.

## Failure Criteria

This experiment fails if it submits a print job, weakens the native print safety
gate, treats OK / `kSuccess` as a safe cancellation, leaves Chromium
branch/patch records inconsistent, or makes broad AppKit changes without trace
evidence that they target the current modal presentation failure.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Verdict: **Approved**.

The reviewer found no Required findings.

## Result

**Result:** Partial

Chromium branch `148.0.7778.97-issue-834-exp22` was created from
`148.0.7778.97-issue-834-exp21`, and `chromium/README.md` was updated with the
new branch. Chromium commit `184e100840738ce30e7fe6e542b14d602493c05e` records
the fork change, and the cumulative patch archive was regenerated through
`chromium/patches/issue-834/0079-Probe-macOS-print-sheet-presentation.patch`.

The implementation added AppKit presentation tracing immediately before native
print-panel presentation:

- `NSApp.activationPolicy`;
- `NSApp.isActive`;
- whether the parent window is present, key, main, visible, miniaturized, able
  to become key/main, and equal to `NSApp.keyWindow` / `NSApp.mainWindow`;
- the print-panel class.

The first attempted presentation adjustment made Roamium temporarily switch from
`NSApplicationActivationPolicyProhibited` to
`NSApplicationActivationPolicyRegular`, ordered the parent window front, and
called `activateIgnoringOtherApps:` before presenting the print panel. This
built successfully, but the guarded native-print probe still reported
`mac-print-modal-response-missing`: Roamium became `activation_policy=regular`,
but `NSApp.active` stayed `false`, the parent window stayed non-key/non-main,
and `[panel runModalWithPrintInfo:]` still did not return a modal response or
create an observable print-panel candidate.

The second attempted presentation adjustment used
`beginSheetUsingPrintInfo:onWindow:completionHandler:` when running on macOS 14
or newer, with the existing modal path preserved as the fallback for older macOS
deployment targets. The probe result changed to
`mac-print-sheet-response-missing`.

Verification run:

```bash
cd chromium/src
/Users/astrohacker/dev/termsurf/chromium/depot_tools/autoninja \
  -C out/Default libtermsurf_chromium

cd /Users/astrohacker/dev/termsurf
rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-native-print.py
rm -rf scripts/__pycache__
node --check scripts/probe-pdf-save-print-title-local.mjs

rm -rf logs/issue-834-exp22-macos-print-panel-presentation scripts/__pycache__
python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp22-macos-print-panel-presentation \
  --probe native-dialog \
  --allow-native-dialog-click
```

Evidence from
`logs/issue-834-exp22-macos-print-panel-presentation/pdf-native-print-summary.json`:

- `first_failing_hop`: `mac-print-sheet-response-missing`;
- `safety_gate_passed`: `true`;
- `probe_status`: `ok`;
- `server_register_received`: `true`;
- `tab_ready_id`: `1`;
- `devtools_port`: `50273`;
- `roamium_exited_before_shutdown`: `false`;
- `print_dialog_watch.dialog_observed`: `false`;
- `print_dialog_watch.cancel_sent`: `false`;
- `print_queue_before.lpstat_o.stdout`: empty;
- `print_queue_after.lpstat_o.stdout`: empty;
- `print_queue_before.lpstat_W_completed_o.stdout`: empty;
- `print_queue_after.lpstat_W_completed_o.stdout`: empty.

The native trace reached:

```text
mac-ask-user-before-activation-app activation_policy=prohibited active=false
mac-ask-user-before-activation-window present=true key=false main=false visible=true miniaturized=false can_key=true can_main=true matches_key=false matches_main=false
mac-ask-user-set-activation-policy-regular-enter
mac-ask-user-set-activation-policy-regular-exit
mac-ask-user-after-activation-app activation_policy=regular active=false
mac-ask-user-after-activation-window present=true key=false main=false visible=true miniaturized=false can_key=true can_main=true matches_key=false matches_main=false
mac-ask-user-begin-sheet-enter
mac-ask-user-begin-sheet-exit
```

No `mac-ask-user-sheet-response-cancel`, `mac-ask-user-sheet-response-printed`,
or `mac-ask-user-callback-*` event was observed before the harness shut down
Roamium. No print job was submitted.

## Conclusion

Experiment 22 did not complete native PDF print cancellation, but it narrowed
the macOS failure from a blocking `runModalWithPrintInfo:` call to a more
specific AppKit presentation problem: Roamium can create an `NSPrintPanel`, can
enter AppKit sheet presentation, and returns from the sheet begin call, but the
process still never becomes an active app with a key/main parent window and the
sheet completion handler never fires.

The next experiment should focus on why Roamium's AppKit application/window
state remains inactive even after changing activation policy and ordering the
window front. Likely targets include how the Roamium content-shell process is
launched as an app, whether it has the right bundle/process activation
configuration for AppKit panels, and whether the print panel must be presented
through an existing Chromium/browser window modal mechanism rather than directly
from `PrintingContextMac`.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

Initial verdict: **Changes Required**.

Required finding:

- The Chromium source change had not yet been committed on
  `148.0.7778.97-issue-834-exp22`, and the cumulative Issue 834 patch archive
  still ended at `0078-Trace-macOS-PDF-print-modal.patch`.

Fix:

- Committed the Chromium source change as
  `184e100840738ce30e7fe6e542b14d602493c05e`.
- Regenerated `chromium/patches/issue-834`, adding
  `0079-Probe-macOS-print-sheet-presentation.patch`.

Optional finding accepted:

- The harness previously classified a queue delta as an unexpected submitted
  print job only when no dialog was observed. That was weaker than the safety
  contract.

Fix:

- Updated `scripts/test-issue-834-pdf-native-print.py` so any before/after print
  queue delta is classified as `native-print-job-submitted-unexpectedly` before
  dialog observations are considered.

Re-review verdict: **Approved**.

The reviewer confirmed that Chromium is clean on `148.0.7778.97-issue-834-exp22`
at `184e100840738ce30e7fe6e542b14d602493c05e`, the patch archive includes
`0079-Probe-macOS-print-sheet-presentation.patch`, Experiment 22 is marked
Partial in the issue README, the prior findings are recorded with fixes, and the
queue-delta safety guard now runs before dialog-observed logic.
