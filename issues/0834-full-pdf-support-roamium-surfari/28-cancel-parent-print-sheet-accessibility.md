# Experiment 28: Cancel Parent Print Sheet Through Accessibility

## Description

Experiment 27 proved that the native macOS print sheet is present and visible,
but the production watcher is looking in the wrong place:

- AppKit attaches a visible key `NSPanel` titled `Print` to Roamium's parent
  window.
- `NSPrintPanel` does not expose a dynamic `window` selector in this build.
- The current watcher searches the CoreGraphics window list for separate
  `"Print"` / `"Printer"` windows.
- CoreGraphics only reports the parent Roamium window, not the document-modal
  attached print sheet.
- The first failing hop is now
  `mac-print-parent-window-sheet-visible-watcher-missed`.

This experiment should make the native print watcher sheet-aware. It should use
Accessibility against the Roamium process/window hierarchy to find and press the
print sheet's Cancel button after the trace proves the sheet exists, instead of
depending only on CoreGraphics title matching.

The goal is to safely reach native print cancellation for Roamium PDFs without
submitting a print job.

## Changes

- Update `scripts/test-issue-834-pdf-native-print.py`.
- Reuse and extend the existing Swift Accessibility helper rather than adding a
  separate automation stack.
- Add a watcher path that can target a known process PID even when CoreGraphics
  does not expose a separate print-sheet window:
  - observe the native print trace for the parent-window sheet evidence from
    Experiment 27;
  - identify the Roamium process PID from the running browser process or trace
    lines;
  - walk that process's AX windows/sheets recursively;
  - find a `Cancel` button inside the print sheet or its attached window
    hierarchy;
  - invoke `AXPress` on that button.
- Preserve the existing CoreGraphics title watcher as a first attempt or
  diagnostic path, but do not require it for attached sheets.
- Record enough watcher output to distinguish:
  - AX permission failure;
  - Roamium process/window not found;
  - sheet found but Cancel button missing;
  - Cancel button pressed but callback not observed;
  - safe cancellation observed.
- Update classification so successful sheet-aware cancellation maps to
  `native-print-dialog-seen-cancelled`, and partial/failure cases get specific
  `first_failing_hop` values.
- Do not modify Chromium unless the harness proves a product-side callback bug
  after AX cancellation succeeds.

## Verification

Run the harness checks:

```bash
rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-native-print.py
rm -rf scripts/__pycache__
node --check scripts/probe-pdf-save-print-title-local.mjs
git diff --check
git -C chromium/src diff --check
```

Run the guarded native-print probe:

```bash
rm -rf logs/issue-834-exp28-sheet-ax-cancel
python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp28-sheet-ax-cancel \
  --probe native-dialog \
  --allow-native-dialog-click
```

Pass criteria:

- the native print safety preflight passes;
- the Roamium PDF print sheet is detected through the sheet-aware watcher;
- the watcher presses Cancel through Accessibility;
- Roamium receives a cancel path rather than `kSuccess`;
- no print job is submitted;
- Roamium remains alive until harness shutdown;
- the harness exits successfully with
  `first_failing_hop=native-print-dialog-seen-cancelled`.

Partial criteria:

- no print job is submitted and Roamium remains alive, but the sheet-aware
  watcher exposes a new specific failing hop that prevents safe cancellation.

Failure criteria:

- a print job is submitted;
- the native print safety gate is weakened;
- OK / printed / `kSuccess` is treated as safe;
- the watcher sends unbounded keyboard or mouse input to the wrong process;
- unrelated Chromium, Roamium, Ghostboard, or Surfari behavior is changed;
- the harness hides a permission failure as a product bug;
- the experiment claims native print is solved without proving queue state,
  cancellation, and Roamium liveness.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Verdict: **Approved**.

The reviewer found no findings. It confirmed that the design is linked from the
issue README as `Designed`, has the required Description, Changes, and
Verification sections, follows directly from Experiment 27's
`mac-print-parent-window-sheet-visible-watcher-missed` result, keeps scope to
the harness, preserves print safety, requires PID-targeted Accessibility
cancellation, distinguishes permission failures, and defines concrete
pass/partial/failure criteria for queue state, cancellation, and Roamium
liveness.

## Result

**Result:** Pass

The native print watcher now has a sheet-aware Accessibility path. When
CoreGraphics cannot observe a separate `"Print"` / `"Printer"` window, the
watcher targets the known Roamium process PID, walks that process's AX window
hierarchy, recursively finds the print sheet's `Cancel` button, and invokes
`AXPress`.

The guarded native-print probe now exits successfully:

- the native print safety preflight passed;
- CoreGraphics still did not observe a separate `"Print"` window;
- the watcher used `roamium-process-accessibility-press-cancel`;
- the watcher first observed live native trace evidence for a visible key
  `NSPanel` attached sheet titled `Print`;
- the target PID was Roamium's process PID;
- Accessibility was trusted;
- `AXPress` succeeded with `pressError=0`;
- the AX helper reported `windowTitle="bitcoin.pdf"` and `requireSheet=true`,
  meaning it only accepted a Cancel button found under a sheet/dialog-like AX
  subtree in the parent Roamium window hierarchy;
- Chromium recorded `mac-ask-user-parent-window-sheet-response-cancel code=0`;
- Chromium recorded `mac-ask-user-callback-canceled`;
- Roamium recorded `ts-scripted-print-callback-result-canceled`;
- the print queue was unchanged before/after;
- Roamium remained alive until harness shutdown;
- the harness summary reported
  `first_failing_hop=native-print-dialog-seen-cancelled`.

Verification run:

```bash
rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-native-print.py
rm -rf scripts/__pycache__
node --check scripts/probe-pdf-save-print-title-local.mjs
git diff --check
git -C chromium/src diff --check

rm -rf logs/issue-834-exp28-sheet-ax-cancel
python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp28-sheet-ax-cancel \
  --probe native-dialog \
  --allow-native-dialog-click
```

The probe summary is at
`logs/issue-834-exp28-sheet-ax-cancel/pdf-native-print-summary.json`, with:

```json
{
  "first_failing_hop": "native-print-dialog-seen-cancelled",
  "safety_gate_passed": true,
  "roamium_exited_before_shutdown": false,
  "print_dialog_watch": {
    "mechanism": "roamium-process-accessibility-press-cancel",
    "target_pid": 24785,
    "dialog_observed": false,
    "sheet_evidence": {
      "mechanism": "native-print-trace-parent-sheet",
      "observed": true
    },
    "sheet_cancel_sent": true,
    "cancel_sent": true
  }
}
```

## Completion Review

An adversarial Codex subagent reviewed the completed experiment and initially
required one fix: the first implementation fell back to pressing the first
`Cancel` button in any Roamium AX window after CoreGraphics missed the dialog,
without first proving that the button belonged to the native print sheet.

The harness was tightened so the fallback now:

- waits for live native trace evidence that the parent-window print sheet is
  attached, visible, sheet-like, and titled `Print`;
- passes `require-sheet` to the Swift AX helper;
- only accepts a `Cancel` button if it is under an AX sheet/dialog-like subtree;
- records the sheet evidence and `requireSheet=true` in the watcher summary.

Final verdict: **Approved**.

The reviewer confirmed that the prior Required finding is resolved, no new
Required finding was introduced, and the rerun summary proves the trace gate,
`require-sheet` command argument, `requireSheet=true`, successful AX press,
unchanged print queue, Roamium liveness, and
`first_failing_hop=native-print-dialog-seen-cancelled`.

## Conclusion

Roamium native PDF print cancellation is now automatable on this macOS VM. The
critical distinction is that document-modal print sheets may be visible in
AppKit and Accessibility without appearing as separately titled CoreGraphics
windows. The durable watcher must therefore keep the existing CoreGraphics path
for standalone dialogs, but fall back to PID-targeted Accessibility traversal
for attached sheets.

This completes the immediate Roamium native print automation blocker. The next
Issue 834 experiment should decide whether to add this successful native print
probe to the durable Roamium PDF regression guard, then continue the remaining
Roamium PDF matrix items before moving to Surfari.
