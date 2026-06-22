# Experiment 26: Retry Parent-Window Print Sheet After Activation

## Description

Experiments 22 and 23 tested macOS print-panel presentation while Roamium's
AppKit state was bad: the app was not active, and the shell window was not key
or main. Experiment 25 fixed that part of the chain:

- `SetGuiActive(active=true)` promotes Roamium from
  `NSApplicationActivationPolicyProhibited` to regular;
- by the time native print starts, `NSApp.active == true`;
- the parent shell window is key and main;
- print queues remain unchanged;
- native print still stalls at `mac-print-app-modal-response-missing`.

The remaining current code path presents `NSPrintPanel` with
`beginSheetWithPrintInfo:modalForWindow:nil`, which was chosen as an app-modal
analogue before the parent window was known to be valid. Now that the parent
window is valid, this experiment should retry a real parent-window sheet and
compare it against the app-modal nil-window sheet under the corrected AppKit
state.

This is intentionally not a broad print rewrite. The experiment should change
only the presentation mode and trace enough state to prove whether the
parent-window sheet can produce an observable dialog and cancellation callback.

## Changes

1. Create a fresh Chromium branch for this issue experiment.

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-834-exp25
   git checkout -b 148.0.7778.97-issue-834-exp26
   ```

   Update the branch table in `chromium/README.md`.

2. In `printing/printing_context_mac.mm`, switch the native-print trace path
   from the nil-window app-modal sheet to a parent-window sheet when
   `parent_window` is present.

   The intended narrow change is:

   - preserve the existing `TermSurfPrintPanelDelegate` callback path;
   - pass the real `parent_window` to `beginSheetWithPrintInfo:modalForWindow:`;
   - emit distinct trace events, such as
     `mac-ask-user-begin-parent-window-sheet-enter`,
     `mac-ask-user-parent-window-sheet-response-cancel`, and
     `mac-ask-user-parent-window-sheet-response-printed`;
   - if `parent_window` is missing, keep a trace-only fallback rather than
     silently reverting to an ambiguous presentation path.

3. Update the native-print harness classifier only if needed.

   If new trace names are added, teach
   `scripts/test-issue-834-pdf-native-print.py` to classify the equivalent
   parent-window response-missing / cancel / printed states without weakening
   the safety rules.

4. Preserve all native-print safety constraints.

   Do not change the preflight, watcher, queue checks, or the rule that `OK`,
   `printed`, and `kSuccess` are unsafe unless explicitly proven not to submit a
   job. A passing result still requires observed cancellation and unchanged
   queues.

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

rm -rf logs/issue-834-exp26-parent-window-print-sheet
python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp26-parent-window-print-sheet \
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
- the Issue 834 patch archive is regenerated and includes the Experiment 26
  Chromium commit;
- `autoninja -C out/Default libtermsurf_chromium` passes;
- the harness still records `gui_active_sent=true`;
- logs prove that the app is regular/active and parent window is key/main before
  sheet presentation;
- logs prove whether the parent-window sheet enter/exit and response callbacks
  run;
- no print job is submitted;
- if a native dialog appears, it is cancelled and queue state remains unchanged;
- markdown is formatted with Prettier;
- Python bytecode cache is removed after compilation;
- `git diff --check` passes;
- design review is recorded, all real design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Pass Criteria

This experiment passes if parent-window sheet presentation opens a native macOS
print panel, the safety watcher cancels it, the callback path reports
cancellation rather than OK / printed / success, and print queue evidence proves
no job was submitted.

## Partial Criteria

This experiment is partial if native print still does not pass but the result
proves one of these narrower facts:

- parent-window sheet presentation is entered under valid AppKit activation
  state but still produces no response;
- parent-window sheet presentation returns cancellation but the watcher cannot
  observe the dialog;
- parent-window sheet presentation behaves differently from the nil-window
  app-modal sheet but still does not meet the pass criteria;
- parent-window sheet presentation cannot be tested because the parent window is
  unexpectedly missing despite Experiment 25.

## Failure Criteria

This experiment fails if it submits a print job, weakens the native print safety
gate, treats OK / printed / `kSuccess` as safe cancellation, changes unrelated
GUI/frontend code, leaves Chromium branch/patch records inconsistent, or makes
additional AppKit lifecycle changes beyond the parent-window sheet presentation
probe.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Verdict: **Approved**.

The reviewer found no Required, Optional, or Nit findings. It confirmed that the
README links Experiment 26 as `Designed`, the design has Description, Changes,
Verification, and Pass / Partial / Failure criteria, the experiment follows
directly from Experiment 25, the current Chromium print path still presents the
traced sheet with `modalForWindow:nil`, AppKit activation promotion exists in
the Experiment 25 code, and no implementation had started before design review.

## Result

**Result:** Partial

The print presentation path now retries a real parent-window sheet after the
Experiment 25 AppKit activation fix. The guarded probe still does not pass, but
it proves the behavior of the corrected presentation mode:

- `gui_active_sent=true`;
- `setActivationPolicy` still succeeds before shell activation;
- before native print, `NSApp.activationPolicy=regular` and `active=true`;
- the parent window is present, key, main, visible, and matches both
  `NSApp.keyWindow` and `NSApp.mainWindow`;
- `mac-ask-user-begin-parent-window-sheet-enter` is recorded;
- `mac-ask-user-begin-parent-window-sheet-exit` is recorded;
- no `mac-ask-user-parent-window-sheet-response-*` callback is recorded;
- the watcher does not observe a native print dialog;
- print queue state is unchanged;
- no print job is submitted;
- Roamium remains alive until harness shutdown.

The first failing hop is now `mac-print-parent-window-sheet-response-missing`,
which is more precise than the Experiment 25
`mac-print-app-modal-response-missing` result. Parent-window sheet presentation
behaves like the nil-window app-modal sheet: it enters and exits the begin call,
but neither the dialog nor the delegate callback appears before the guarded
harness times out.

Verification run:

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

rm -rf logs/issue-834-exp26-parent-window-print-sheet
python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp26-parent-window-print-sheet \
  --probe native-dialog \
  --allow-native-dialog-click
```

The final probe returned nonzero because the experiment did not reach safe
native-dialog cancellation. Its summary is at
`logs/issue-834-exp26-parent-window-print-sheet/pdf-native-print-summary.json`.

The Chromium branch `148.0.7778.97-issue-834-exp26` was committed at
`c35de398223fb2a70c9cf47fe41d489323a2c54b`, and `chromium/patches/issue-834/`
was regenerated through `0083-Retry-parent-print-sheet.patch`.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment and initially
required two fixes:

- The missing-parent fallback still reached `beginSheetWithPrintInfo:` with a
  nil parent window. The implementation now returns before delegate creation and
  before sheet presentation when the parent window is missing.
- The missing-parent early return skipped restoration of the temporary AppKit
  activation policy. The implementation now restores
  `previous_activation_policy` with the same restore trace events before
  reporting cancellation.

The same reviewer rechecked the final patch after those fixes.

Verdict: **Approved**.

The reviewer confirmed that the prior Required findings are resolved, Chromium
is on `148.0.7778.97-issue-834-exp26` at
`c35de398223fb2a70c9cf47fe41d489323a2c54b`, patch `0083` starts from that hash,
the experiment document records the hash, and both `git diff --check` and
`git -C chromium/src diff --check` pass. The reviewer did not rerun `autoninja`
or compile checks because the review was read-only.

## Conclusion

Experiment 26 proves that parent-window sheet presentation is not sufficient
even after AppKit activation policy, app active state, and key/main parent
window state are correct. The next experiment should investigate why
`NSPrintPanel` itself is not producing an observable panel or delegate response
after a successful `beginSheetWithPrintInfo:` call. Likely next probes include
instrumenting panel/window visibility and ordered-window state after the begin
call, comparing `NSPrintPanel` against a simple `NSPanel`/`NSAlert` sheet in the
same parent window, or scheduling presentation onto the next AppKit run-loop
turn instead of only the CATransaction completion block.
