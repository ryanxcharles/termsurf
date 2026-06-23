# Experiment 17: Prove Native Dialog Cancellation

## Description

Experiment 16 proved that CoreGraphics can observe harmless native dialogs on
this VM, but it did not prove a production-compatible cancel action. System
Events could not observe windows because assistive access was denied, and
CoreGraphics-based Escape/click attempts did not cancel the harmless dialog. The
only successful cleanup mechanism was terminating the harmless `osascript`
dialog process, which is not compatible with a real production print dialog.

This experiment should solve only the cancellation half of the native-dialog
safety gate. It must not launch Roamium, click the PDF print control, or open a
production print dialog. The goal is to find and prove a cancellation mechanism
that can dismiss a harmless native dialog using an action that could also be
applied to a real print dialog.

## Changes

1. Extend the preflight harness with cancellation-focused mechanisms.

   Update `scripts/test-issue-834-pdf-native-print.py` so
   `--probe watcher-preflight` can test more production-compatible cancellation
   strategies against the harmless dialog used in Experiment 16.

   Candidate mechanisms include:

   - better CoreGraphics click targeting based on observed button geometry;
   - focusing or activating the dialog owner before CGEvent Escape/click;
   - AppleScript targeted at the dialog process rather than global System Events
     enumeration;
   - an Accessibility API or Swift/AppKit helper that can press the Cancel
     button by role/title;
   - any other local macOS mechanism that can observe the harmless dialog,
     cancel it, and verify disappearance without killing the dialog process.

   The true answer may be a macOS permission grant rather than code. If a
   mechanism fails due to a permission error, record the exact permission or
   service involved.

2. Keep production print disabled.

   This experiment must not run the native PDF print probe and must not use
   `--allow-native-dialog-click`. The only UI under test is the harmless
   preflight dialog.

3. Require cancellation proof from the dialog itself.

   A production-compatible mechanism may pass only if all of these are true:

   - the harmless dialog is observed;
   - the mechanism sends a cancel action without terminating the dialog process;
   - the dialog process exits before timeout;
   - the dialog result proves cancellation rather than timeout, for example no
     `gave up:true` in the dialog stdout;
   - disappearance is verified after cancellation.

   Cleanup by terminating the harmless dialog process may remain as a fallback
   safety cleanup, but it must be recorded separately as non-production
   compatible and must not make the probe pass.

4. Produce clear result evidence.

   Continue writing
   `logs/issue-834-exp17-native-dialog-cancellation/native-dialog-preflight-summary.json`
   with:

   - `overall_result`;
   - `first_failing_hop`;
   - `selected_mechanism`;
   - `safe_for_production_print_probe`;
   - `production_print_click_attempted`;
   - per-mechanism observation, cancel, disappearance, dialog result, and
     permission diagnostics.

   Use these classifications:

   - `no-failure-observed`;
   - `dialog-observation-failed`;
   - `dialog-cancel-failed`;
   - `dialog-disappearance-not-proven`;
   - `permission-denied`;
   - `automation-gap`.

5. Wire the native print harness only if cancellation is proven.

   If a production-compatible mechanism passes, update
   `watch_and_cancel_print_dialog` to use that exact mechanism for a future
   production print experiment. Do not run that future experiment here.

   If no mechanism passes, record Partial and list the most likely next action,
   including any System Settings permission the user may need to grant.

## Verification

Verification for the completed result is:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-native-print.py

python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp17-native-dialog-cancellation \
  --probe watcher-preflight

git diff --check
```

Required evidence:

- no production print dialog is opened;
- no production PDF print control is clicked;
- the harmless dialog title is unique and recorded;
- at least one new cancellation mechanism beyond Experiment 16 is attempted, or
  the result records why no new mechanism can be attempted without external
  permission;
- mechanisms distinguish `cancel action sent` from `dialog actually cancelled`;
- timeout results such as `gave up:true` do not count as cancellation;
- process termination cleanup does not count as production-compatible
  cancellation;
- if a mechanism passes, `safe_for_production_print_probe = true` and
  `selected_mechanism` names a production-compatible cancellation mechanism;
- if no mechanism passes, `safe_for_production_print_probe = false` and the
  first failing hop identifies the remaining blocker;
- Python bytecode cache is removed after compilation;
- markdown is formatted with Prettier;
- `git diff --check` passes;
- design review is recorded, all real design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Pass Criteria

This experiment passes if a production-compatible mechanism observes the
harmless native dialog, sends a cancel action, proves the dialog exited because
of cancellation rather than timeout, verifies disappearance, and marks the next
native PDF print experiment safe to attempt a production print click behind that
mechanism.

## Partial Criteria

This experiment is partial if observation still works but no
production-compatible cancellation mechanism can be proven on this VM.

## Failure Criteria

This experiment fails if it clicks production print, treats process termination
as production-compatible cancellation, treats timeout as cancellation, leaves a
harmless dialog open without recording cleanup failure, or omits permission
diagnostics for failed mechanisms.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Verdict: **Approved**.

The reviewer found no issues.

## Result

**Result:** Pass

Extended `scripts/test-issue-834-pdf-native-print.py` with a
production-compatible Accessibility cancellation mechanism for the harmless
native-dialog preflight.

The new passing mechanism is `accessibility-press-cancel-button`:

- CoreGraphics observes the harmless dialog by title and captures its owner PID;
- a Swift Accessibility helper creates an AX application element for that PID;
- the helper finds the matching window and recursively locates the `Cancel`
  button;
- it invokes `AXPress` on that button;
- the dialog process exits with `User canceled. (-128)`;
- CoreGraphics verifies the dialog disappeared.

The harness also still records the Experiment 16 mechanisms:

- System Events remains blocked by `system-events-assistive-access-denied`;
- CGEvent Escape sends successfully but the dialog times out with
  `gave up:true`;
- CGEvent click attempts send successfully but the dialog still times out with
  `gave up:true`;
- process termination cleans up the harmless dialog, but is marked
  `production_print_compatible = false`.

Final evidence:

- summary:
  `logs/issue-834-exp17-native-dialog-cancellation/native-dialog-preflight-summary.json`;
- `overall_result = "pass"`;
- `first_failing_hop = "no-failure-observed"`;
- `selected_mechanism = "accessibility-press-cancel-button"`;
- `safe_for_production_print_probe = true`;
- `production_print_click_attempted = false`;
- selected mechanism recorded `observed = true`, `cancel_sent = true`, and
  `disappeared = true`;
- selected mechanism's dialog result recorded `cancelled = true` and stderr
  `User canceled. (-128)`;
- print queues before and after were empty.

The future production print watcher now uses CoreGraphics observation for
`Print` / `Printer` windows and the same Accessibility Cancel-button press
mechanism. This experiment did not run that production print path.

Verification commands run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-native-print.py

rm -rf scripts/__pycache__

python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp17-native-dialog-cancellation \
  --probe watcher-preflight

git diff --check
```

No production print dialog was opened, no production PDF print control was
clicked, and no print job was submitted.

## Conclusion

The native-dialog safety gate now has a production-compatible cancellation
mechanism on this VM: CoreGraphics window observation plus Accessibility
`AXPress` on the Cancel button. The remaining Roamium native-print work can now
return to the real PDF print probe, still behind the hard
`--allow-native-dialog-click` safety flag.

The next experiment should click the production Roamium PDF print control only
after the preflight passes, watch for a native Print/Printer window using the
CoreGraphics/Accessibility mechanism, cancel it, and prove from queue state that
no print job was submitted.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

Verdict: **Approved**.

The reviewer found no issues.
