# Experiment 27: Inspect Parent Print Sheet Visibility

## Description

Experiment 26 proved that Roamium now calls
`beginSheetWithPrintInfo:modalForWindow:delegate:didEndSelector:contextInfo:`
with the right high-level preconditions:

- `NSApp.activationPolicy=regular`;
- `NSApp.active=true`;
- the parent window is present, key, main, visible, and matches
  `NSApp.keyWindow` / `NSApp.mainWindow`;
- the `beginSheetWithPrintInfo` call enters and exits;
- no native dialog is observed;
- no delegate response arrives;
- no print job is submitted.

The next unknown is whether AppKit creates and attaches any visible print panel
window after the begin call. This experiment is a trace-only probe that keeps
the guarded no-print safety posture and records what happens to the
`NSPrintPanel` and parent window immediately after presentation and on later
main-run-loop turns.

The goal is diagnostic, not a fix. The result should identify the next failing
hop more precisely than `mac-print-parent-window-sheet-response-missing`, for
example:

- `NSPrintPanel.window` never exists;
- the panel window exists but is never visible;
- the parent window never reports an attached sheet;
- the sheet appears only after a later run-loop turn that the current harness
  fails to wait through;
- a sheet/window appears but the watcher cannot observe or cancel it.

## Changes

- Create a new Chromium branch from `148.0.7778.97-issue-834-exp26`, named
  `148.0.7778.97-issue-834-exp27`.
- Update `chromium/README.md` with the new branch row after implementation.
- In `chromium/src/printing/printing_context_mac.mm`, add trace-only AppKit
  inspection around the parent-window sheet path:
  - trace the panel window state immediately before and after
    `beginSheetWithPrintInfo`;
  - trace the parent window's `attachedSheet` state after the begin call;
  - schedule one or more `dispatch_async(dispatch_get_main_queue(), ...)`
    inspections on later main-run-loop turns and trace the same state again;
  - use weak or otherwise non-retaining references for delayed inspections so
    the trace probe does not keep the autoreleased `NSPrintPanel` or related
    `NSWindow` objects alive longer than the current implementation would;
  - trace a distinct outcome when a delayed weak reference is already gone, so
    the result can distinguish "object was deallocated before delayed
    inspection" from "object still exists but is hidden or unattached";
  - trace enough identifiers to correlate whether `panel.window`, the parent
    `attachedSheet`, and `NSApp.orderedWindows` refer to the same window.
- Keep the existing delegate callback and activation-policy restoration logic
  unchanged except for any trace-only additions required to report state.
- Update `scripts/test-issue-834-pdf-native-print.py` only if needed to classify
  the new trace outcomes into more specific `first_failing_hop` values.
- Regenerate `chromium/patches/issue-834/` so the archive includes the
  Experiment 27 Chromium commit.
- Record the Chromium commit hash and probe result in this experiment file.

## Verification

Run the hygiene checks:

```bash
git status --short
git -C chromium/src status --short
git -C chromium/src rev-parse --abbrev-ref HEAD
git -C chromium/src rev-parse HEAD
git diff --check
git -C chromium/src diff --check

rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-native-print.py
rm -rf scripts/__pycache__
node --check scripts/probe-pdf-save-print-title-local.mjs
```

Build the Chromium library:

```bash
cd chromium/src
export PATH="/Users/astrohacker/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium
```

Run the guarded native-print probe:

```bash
cd /Users/astrohacker/dev/termsurf
rm -rf logs/issue-834-exp27-print-sheet-visibility
python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp27-print-sheet-visibility \
  --probe native-dialog \
  --allow-native-dialog-click
```

Pass criteria:

- no print job is submitted;
- Roamium remains alive until harness shutdown;
- the probe captures the new sheet/window visibility trace lines;
- the result identifies whether the panel window and/or parent attached sheet
  exists, becomes visible, or remains missing across later main-run-loop turns;
- delayed inspections distinguish deallocated weak references from hidden or
  unattached live objects;
- if a native print dialog is observed, the watcher cancels it and the result is
  classified as safe cancellation.

Partial criteria:

- the probe remains safely non-printing and captures enough new trace evidence
  to identify the next failing hop, but native print still does not reach safe
  observed cancellation.

Failure criteria:

- a print job is submitted;
- the native print safety gate is weakened;
- OK / printed / `kSuccess` is treated as safe;
- unrelated Chromium, Roamium, Ghostboard, or Surfari behavior is changed;
- the trace addition changes print behavior instead of only observing it;
- delayed trace blocks retain `NSPrintPanel` or related `NSWindow` objects in a
  way that could change their lifetime;
- the Chromium branch, README, patch archive, or experiment result is left
  inconsistent.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Initial verdict: **Changes Required**.

The reviewer found that the delayed `dispatch_async` trace plan was not
guaranteed to be trace-only. Because Objective-C blocks retain captured objects
by default, capturing the autoreleased `NSPrintPanel` or related windows for
later inspection could keep them alive longer than the current implementation
does and change the behavior under diagnosis.

The design now requires weak or otherwise non-retaining delayed inspection, a
distinct trace outcome when a delayed weak reference is already gone, and a
failure criterion that rejects delayed trace blocks which retain the panel or
related windows in a behavior-changing way.

Final verdict: **Approved**.

The reviewer confirmed that the prior Required finding is resolved, no new
Required finding was introduced by the fix, and the README still links
Experiment 27 as `Designed`.

## Result

**Result:** Partial

The trace-only AppKit visibility probe succeeded and narrowed the failure. The
native print sheet is not missing. After
`beginSheetWithPrintInfo:modalForWindow:delegate:didEndSelector:contextInfo:`
returns, AppKit has attached a visible key `NSPanel` titled `Print` to the
Roamium parent window:

- before begin, `attachedSheet present=false`;
- after begin, `attachedSheet present=true`;
- after begin, the attached sheet is `class=NSPanel`, `visible=true`,
  `key=true`, `sheet=true`, and `title=Print`;
- `NSPrintPanel` does not respond to a dynamic `window` selector in this build,
  so there is no separate `panel.window` object to correlate;
- delayed main-run-loop probes still see the same visible attached sheet;
- the weak delayed references remain live, so the panel was not immediately
  deallocated;
- no `mac-ask-user-parent-window-sheet-response-*` callback is recorded;
- the CoreGraphics title-based watcher does not observe a `"Print"` or
  `"Printer"` window and therefore does not send cancel;
- the print queue is unchanged;
- Roamium remains alive until harness shutdown.

The harness now classifies this as
`mac-print-parent-window-sheet-visible-watcher-missed`, which is more precise
than Experiment 26's `mac-print-parent-window-sheet-response-missing`.

Verification run:

```bash
cd chromium/src
export PATH="/Users/astrohacker/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium

cd /Users/astrohacker/dev/termsurf
rm -rf logs/issue-834-exp27-print-sheet-visibility
python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp27-print-sheet-visibility \
  --probe native-dialog \
  --allow-native-dialog-click

rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-native-print.py
rm -rf scripts/__pycache__
node --check scripts/probe-pdf-save-print-title-local.mjs
git -C chromium/src diff --check
git diff --check
```

The guarded probe returned nonzero because native print still did not reach safe
observed cancellation. Its summary is at
`logs/issue-834-exp27-print-sheet-visibility/pdf-native-print-summary.json`,
with:

```json
{
  "first_failing_hop": "mac-print-parent-window-sheet-visible-watcher-missed",
  "safety_gate_passed": true,
  "roamium_exited_before_shutdown": false
}
```

The Chromium branch `148.0.7778.97-issue-834-exp27` was committed at
`5d290a336479d86f85c2097c280911cc9d85e267`, and `chromium/patches/issue-834/`
was regenerated through `0084-Trace-parent-print-sheet-visibility.patch`.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment and initially
required one fix: the implementation traced the parent window's `attachedSheet`,
but did not explicitly trace or correlate a `panel.window` lookup as required by
the approved design.

The Chromium patch was amended so `TraceMacPrintSheetVisibility` now:

- records whether `NSPrintPanel` responds to a dynamic `window` selector;
- traces `panel-window` state when that selector is available;
- records `matches_panel_window` on correlated parent, attached-sheet, and
  ordered-window traces.

The guarded probe was rerun after the fix. It showed
`panel-window-selector present=false` at each inspection point while still
showing the visible key attached `NSPanel` titled `Print`.

Final verdict: **Approved**.

The reviewer confirmed that the prior Required finding is resolved, no new
Required finding was introduced, the patch archive is from amended Chromium hash
`5d290a336479d86f85c2097c280911cc9d85e267`, and the experiment document records
the updated evidence and hash.

## Conclusion

Experiment 27 proves the macOS print sheet is present and visible inside
Roamium/AppKit, but our current automation is looking in the wrong place. A
document-modal print sheet does not appear as a separately named CoreGraphics
window titled `Print`; CoreGraphics sees the parent Roamium window instead. The
next experiment should make the native dialog watcher sheet-aware, likely by
using Accessibility against the Roamium process/window hierarchy to find and
press the sheet's Cancel button rather than relying only on CGWindow title
matching.
