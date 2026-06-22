# Experiment 24: Bridge GUI Active State Into Roamium App Activation

## Description

Experiments 21-23 proved that Roamium reaches macOS native PDF print
presentation, but every `NSPrintPanel` presentation variant stalls before a
visible/cancellable dialog or callback response:

- `runModalWithPrintInfo:`: `mac-print-modal-response-missing`;
- parent-window sheet: `mac-print-sheet-response-missing`;
- app-modal delegate sheet: `mac-print-app-modal-response-missing`.

Across the successful probes, the key AppKit state remains unchanged:

- `NSApp.activationPolicy` starts as `prohibited`;
- setting it to `regular` succeeds;
- `NSApp.active` stays `false`;
- the content-shell parent window remains non-key and non-main;
- no print job is submitted.

The next likely issue is not the print-panel API. It is the GUI-active and
window-activation bridge. Ghostboard sends `SetGuiActive` when the macOS app
becomes active, and Roamium receives that message through `ts_set_gui_active`.
However, the native-print harness currently sends only `Resize` and
`FocusChanged`, not `SetGuiActive`, and `TsBrowserMainParts::SetGuiActive`
currently updates Chromium page focus rather than the macOS shell window/app
activation path.

This experiment should first make the native-print harness representative by
sending `SetGuiActive(active=true)`, then determine whether Roamium must map GUI
active state into `Shell::ActivateContents()` / `ShellPlatformDelegate` so that
`NSApp.active`, `NSApp.keyWindow`, and `NSApp.mainWindow` can become correct
before native print.

## Changes

1. Create a fresh Chromium branch for this issue experiment.

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-834-exp23
   git checkout -b 148.0.7778.97-issue-834-exp24
   ```

   Update the branch table in `chromium/README.md`.

2. Make the native-print harness send GUI-active state.

   In `scripts/test-issue-834-pdf-native-print.py`:

   - add a `SetGuiActive` payload helper for protocol field `33`;
   - after `TabReady`, send `Resize`, `FocusChanged(focused=true)`, and
     `SetGuiActive(active=true, reason="native_print_probe")`;
   - record `gui_active_sent` in harness state and summary;
   - keep the existing safety gate and print queue checks unchanged.

3. Trace whether GUI-active reaches Roamium and Chromium.

   Existing Roamium trace already logs `set-gui-active` when
   `TERMSURF_PDF_INPUT_TRACE=1`. Add Chromium-side trace in
   `TsBrowserMainParts::SetGuiActive()` when the native print trace or input
   trace is enabled:

   - active value and reason;
   - shell/window pointers available through the tab state;
   - `NSApp.activationPolicy`, `NSApp.active`, `NSApp.keyWindow`, and
     `NSApp.mainWindow` on macOS;
   - whether `Shell::ActivateContents()` was called.

4. If the first probe proves `SetGuiActive(active=true)` reaches Chromium but
   does not activate the macOS app/window, map active GUI state into shell
   activation.

   The narrow candidate change is:

   - in `TsBrowserMainParts::SetGuiActive()`, when `active == true` on macOS,
     call `tab->shell->ActivateContents(web_contents)` before or alongside
     `RenderWidgetHostImpl::SetPageFocus(active)`;
   - when `active == false`, keep the existing page-focus deactivation behavior
     and do not attempt to deactivate macOS globally.

   Do not modify Ghostboard, Roamium IPC dispatch, or print-panel presentation
   unless the trace proves the GUI-active bridge cannot be tested without it.

5. Run the guarded native-print probe after each attempted change.

   Stop after the first proven improvement. A pass still requires a visible
   native print panel, automated cancellation, unchanged print queues, and a
   cancellation callback. A partial result is useful if the probe proves that
   GUI-active reaches Chromium and either does or does not fix the AppKit
   activation state.

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

rm -rf logs/issue-834-exp24-gui-active-app-activation
python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp24-gui-active-app-activation \
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
- Chromium source changes are committed inside `chromium/src` if any Chromium
  code changes are made;
- the Issue 834 patch archive is regenerated if Chromium changes are committed;
- `autoninja -C out/Default libtermsurf_chromium` passes if Chromium changes are
  made;
- `scripts/test-issue-834-pdf-native-print.py` records `gui_active_sent`;
- logs prove whether Roamium dispatch received `SetGuiActive`;
- logs prove whether Chromium `TsBrowserMainParts::SetGuiActive()` received it;
- logs record `NSApp`/window state before print after GUI-active handling;
- no print job is submitted;
- if a native dialog appears, it is cancelled and queue state remains unchanged;
- markdown is formatted with Prettier;
- Python bytecode cache is removed after compilation;
- `git diff --check` passes;
- design review is recorded, all real design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Pass Criteria

This experiment passes if GUI-active activation makes Roamium native PDF print
open a native macOS print panel, the safety watcher cancels it, the callback
path reports cancellation rather than OK / printed / success, and print queue
evidence proves no job was submitted.

## Partial Criteria

This experiment is partial if native print still does not pass but the result
proves one of these narrower facts:

- the prior probes were missing a required `SetGuiActive` message;
- `SetGuiActive` reaches Roamium but not Chromium;
- `SetGuiActive` reaches Chromium but does not affect macOS app/window
  activation;
- mapping GUI-active to `Shell::ActivateContents()` changes the AppKit state or
  failing sub-hop without completing safe native print cancellation.

## Failure Criteria

This experiment fails if it submits a print job, weakens the native print safety
gate, treats OK / printed / `kSuccess` as safe cancellation, changes unrelated
GUI/frontend code, leaves Chromium branch/patch records inconsistent, or makes
broad AppKit/process changes without trace evidence from the GUI-active path.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Verdict: **Approved**.

The reviewer found no Required findings. It confirmed that Experiment 24 is
linked as `Designed`, has Description / Changes / Verification plus Pass /
Partial / Failure criteria, follows directly from Experiments 21-23, preserves
the native-print safety contract, correctly identifies protocol field `33` as
`SetGuiActive`, and requires the plan commit before implementation begins.
