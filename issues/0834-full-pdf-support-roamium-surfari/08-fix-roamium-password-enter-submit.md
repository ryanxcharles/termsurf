# Experiment 8: Fix Roamium PDF Password Enter Submission

## Description

Experiment 7 proved that Roamium can load password-protected PDFs when the
password is typed through TermSurf protocol key events and the visible Chromium
PDF password dialog submit button is clicked. It also found a keyboard parity
gap: Enter key down/up events reach the Chromium PDF extension target, but the
password dialog remains open and does not submit.

This experiment should determine why Enter submission fails, fix the narrowest
real integration bug, and rerun the password PDF probes until the
password-protected row no longer needs the click-submit caveat.

Because the failing path is inside Roamium/Chromium PDF input routing, this
experiment may modify the Chromium fork if investigation proves the current
synthetic keyboard event is incomplete. If Chromium source changes are needed,
create a fresh Chromium branch for Experiment 8 before editing Chromium source,
update the Chromium branch table, build `libtermsurf_chromium`, and archive the
patches according to `chromium/AGENTS.md`.

## Changes

1. Reproduce the Experiment 7 Enter-only failure from a clean log directory.

   Run:

   ```bash
   python3 scripts/test-issue-834-pdf-password.py \
     --log-dir logs/issue-834-exp8-password-enter-before \
     --probe password-protected \
     --credential-flow correct-only \
     --submit-mode enter
   ```

   The expected starting failure is
   `first_failing_hop = "correct-password-not-accepted"` with Enter key down/up
   events recorded as `windows_key_code = 13`.

2. Inspect the synthetic key event path.

   Compare the TermSurf-generated Enter event with Chromium's expected
   keyboard-event fields for activation keys. Inspect at least:

   - `chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.cc`;
   - the local `ts_forward_key_event` wrapper;
   - Chromium references for `NativeWebKeyboardEvent`, `DomCode::ENTER`,
     `DomKey::ENTER`, `VKEY_RETURN`, `text`, `unmodified_text`, and platform
     native event fields;
   - existing TermSurf PDF keyboard probes, especially find/search and page
     navigation, to avoid regressing working keyboard behavior.

   Determine whether the failure is caused by:

   - missing or incorrect `dom_key`;
   - missing or incorrect `dom_code`;
   - sending a `kChar` event for Enter when the PDF dialog expects only raw key
     events;
   - missing macOS/native keyboard fields;
   - focus targeting the `cr-input` host instead of the native inner input;
   - Chromium PDF viewer dialog behavior that requires button activation even
     for real Enter;
   - another specific layer.

3. Fix only the proven layer.

   Prefer a narrow fix that improves TermSurf key synthesis for non-text keys
   without changing the protocol or broad PDF behavior. Do not add DevTools DOM
   submission, JavaScript button clicks, or test-only bypasses. The password
   must still be typed and submitted through the TermSurf protocol input path.

   If Chromium is changed:

   - create a branch named `148.0.7778.97-issue-834-exp8` from the current
     relevant Issue 834 Chromium branch;
   - update `chromium/README.md`;
   - build with:

     ```bash
     cd chromium/src
     export PATH="/Users/astrohacker/dev/termsurf/chromium/depot_tools:$PATH"
     autoninja -C out/Default libtermsurf_chromium
     ```

   - regenerate the cumulative Issue 834 patch archive under
     `chromium/patches/issue-834/`, preserving the established Issue 834
     Chromium archive location.

4. Rerun focused password probes.

   Use fresh final log directories:

   ```bash
   python3 scripts/test-issue-834-pdf-password.py \
     --log-dir logs/issue-834-exp8-password-enter-after \
     --probe password-protected \
     --credential-flow correct-only \
     --submit-mode enter
   python3 scripts/test-issue-834-pdf-password.py \
     --log-dir logs/issue-834-exp8-password-wrong-enter-after \
     --probe password-protected \
     --credential-flow wrong-only \
     --submit-mode enter
   python3 scripts/test-issue-834-pdf-password.py \
     --log-dir logs/issue-834-exp8-password-control-after \
     --probe unrestricted-control
   ```

5. Run keyboard regression probes.

   At minimum rerun the Roamium PDF keyboard probes already introduced in Issue
   834:

   ```bash
   python3 scripts/test-issue-834-pdf-navigation.py \
     --log-dir logs/issue-834-exp8-keyboard-page-scroll-regression \
     --serve-bitcoin-pdf \
     --probe keyboard-page-scroll
   python3 scripts/test-issue-834-pdf-navigation.py \
     --log-dir logs/issue-834-exp8-toolbar-page-selector-regression \
     --serve-bitcoin-pdf \
     --probe toolbar-page-selector
   python3 scripts/test-issue-834-pdf-find.py \
     --log-dir logs/issue-834-exp8-find-positive-regression \
     --probe positive-search
   ```

   Each regression summary should record
   `first_failing_hop = "no-failure-observed"` or the result must explain the
   concrete regression before proceeding.

   Rerun the password click-submit probes from Experiment 7 if the Enter fix
   touches shared key routing.

   Add a smaller regression command only if the existing probes are too broad,
   but do not replace them with a weaker check unless the result explains why.

## Verification

Verification for the completed result is:

- the pre-fix Enter-only failure is reproduced and classified from
  `logs/issue-834-exp8-password-enter-before`;
- the result identifies the exact failing layer before applying a fix;
- no DevTools DOM mutation or JavaScript submission is used as a product or test
  substitute;
- after the fix, correct-password Enter submission exits 0 with
  `first_failing_hop = "no-failure-observed"`, `submit_mode = "enter"`,
  `correct_password_loaded = true`, and Enter key-code evidence;
- wrong-password Enter submission exits 0 with `wrong_password_rejected = true`
  and, where stable, `wrong_password_invalid_observed = true`;
- unrestricted PDF control still exits 0;
- raw fixed test passwords do not appear in summaries or logs;
- required keyboard regression probes still pass;
- if Chromium source changes, `chromium/README.md` and the cumulative
  `chromium/patches/issue-834/` archive are updated and
  `autoninja -C out/Default libtermsurf_chromium` passes;
- if Chromium source does not change, the result explains where the fix landed;
- `node --check scripts/probe-pdf-password.mjs` passes if the Node probe is
  edited;
- `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile scripts/test-issue-834-pdf-password.py`
  passes if the Python harness is edited, and `scripts/__pycache__/` is removed
  afterward;
- markdown is formatted with Prettier;
- `git diff --check` passes;
- design review is recorded, all required design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Design Review

Fresh-context adversarial review by Codex subagent `Ohm`: **Changes required**.

Required findings:

- The design omitted the required design-review and plan-commit gate before
  implementation. Fixed by adding explicit verification that design review is
  recorded, required findings are fixed, approval is obtained, and the plan
  commit exists before implementation begins.
- The design originally named `chromium/patches/issue-834-exp8/`, which
  conflicted with the established cumulative Issue 834 Chromium patch archive.
  Fixed by requiring updates to `chromium/patches/issue-834/`.

Optional finding:

- Keyboard regression verification named the prior experiments but did not give
  exact commands or expected summary status. Fixed by adding concrete navigation
  and find/search regression commands and requiring
  `first_failing_hop = "no-failure-observed"` or an explained regression.

Fresh-context adversarial re-review by Codex subagent `Lagrange`: **Approved**.

Findings: none.

The reviewer confirmed that the design-review/plan-commit gate, cumulative
`chromium/patches/issue-834/` archive requirement, and concrete keyboard
regression commands are fixed, with no new required findings.

## Pass Criteria

This experiment passes if Roamium password-protected PDFs can be submitted with
Enter through the TermSurf protocol, wrong-password Enter submission is rejected
correctly, and existing Roamium PDF keyboard workflows do not regress.

## Partial Criteria

This experiment is partial if it reproduces and classifies the Enter submission
failure but cannot safely fix it in this experiment, or if the fix works for the
correct-password path but exposes a separate keyboard regression.

## Failure Criteria

This experiment fails if the pre-fix failure cannot be reproduced, if the fix
submits the password through DevTools or a test-only bypass instead of TermSurf
protocol input, if raw test passwords leak into logs, or if a Chromium change is
made without the required branch, build, and patch archive workflow.

## Result

**Result:** Pass

The Enter-submit gap was reproduced, traced to Chromium's PDF password dialog
resource, fixed in the Chromium fork, built, and verified with focused password
probes plus existing PDF keyboard regressions.

### Reproduction

Command:

```bash
python3 scripts/test-issue-834-pdf-password.py \
  --log-dir logs/issue-834-exp8-password-enter-before \
  --probe password-protected \
  --credential-flow correct-only \
  --submit-mode enter
```

Exit status: 1.

Summary: `logs/issue-834-exp8-password-enter-before/pdf-password-summary.json`

- `first_failing_hop = "correct-password-not-accepted"`
- `submit_mode = "enter"`
- `correct_password_loaded = false`
- Enter key down/up events were recorded with `windows_key_code = 13`
- the password dialog remained present with `valueLength = 11` and
  `invalid = false`
- `raw_password_leaks = []`

The failure was not key routing: Roamium received the key events, called
`ts_forward_key_event`, and Chromium routed Enter to the focused PDF extension
widget. The failure was the PDF password dialog itself: the dialog had a click
handler for the submit button but no keydown handler on the password input.

### Changes

Chromium branch: `148.0.7778.97-issue-834-exp8`

Chromium commit: `0195b42d78 Let PDF passwords hear Enter`

Changed Chromium files:

- `chromium/src/chrome/browser/resources/pdf/elements/viewer_password_dialog.html.ts`
- `chromium/src/chrome/browser/resources/pdf/elements/viewer_password_dialog.ts`

The fix adds a keydown handler to the password `cr-input`. If the key is Enter,
it prevents the default event and calls the same `onSubmitClick_()` path used by
the visible Submit button. No DevTools DOM mutation, JavaScript test bypass, or
TermSurf protocol change was added.

Main-repo tracking changes:

- `chromium/README.md` now lists latest documented branch
  `148.0.7778.97-issue-834-exp8` and adds the branch table row.
- `chromium/patches/issue-834/` was regenerated as the cumulative Issue 834
  patch archive from the local Chromium base root to the Experiment 8 branch
  tip. The new final patch is
  `chromium/patches/issue-834/0076-Let-PDF-passwords-hear-Enter.patch`.

### Verification

Build/resource commands:

```bash
cd chromium/src
export PATH="/Users/astrohacker/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default chrome/browser/resources/pdf:build_ts
autoninja -C out/Default chrome/browser/resources/pdf:build_bundle
autoninja -C out/Default chrome/browser/resources/pdf:resources_grit
autoninja -C out/Default libtermsurf_chromium
```

All build/resource commands exited 0. `resources_grit` regenerated
`out/Default/gen/chrome/pdf_resources.pak`, which is the pak consumed by
Roamium's PDF viewer resource path.

Focused password probes:

```bash
python3 scripts/test-issue-834-pdf-password.py \
  --log-dir logs/issue-834-exp8-password-enter-after \
  --probe password-protected \
  --credential-flow correct-only \
  --submit-mode enter
python3 scripts/test-issue-834-pdf-password.py \
  --log-dir logs/issue-834-exp8-password-wrong-enter-after \
  --probe password-protected \
  --credential-flow wrong-only \
  --submit-mode enter
python3 scripts/test-issue-834-pdf-password.py \
  --log-dir logs/issue-834-exp8-password-control-after \
  --probe unrestricted-control
```

All focused probes exited 0.

Evidence:

- `logs/issue-834-exp8-password-enter-after/pdf-password-summary.json` recorded
  `first_failing_hop = "no-failure-observed"`, `submit_mode = "enter"`,
  `prompt_before_password = true`, `correct_password_loaded = true`,
  `protocol_key_messages_sent = 24`, Enter key down/up events with
  `windows_key_code = 13`, `raw_password_leaks = []`, and a loaded PDF extension
  plugin state with `loadState = "success"`, `loadProgress = 100`,
  `docLength = 9`, and a non-empty plugin rectangle.
- `logs/issue-834-exp8-password-wrong-enter-after/pdf-password-summary.json`
  recorded `first_failing_hop = "no-failure-observed"`, `submit_mode = "enter"`,
  `wrong_password_rejected = true`, `wrong_password_invalid_observed = true`,
  Enter key down/up events with `windows_key_code = 13`, and
  `raw_password_leaks = []`.
- `logs/issue-834-exp8-password-control-after/pdf-password-summary.json`
  recorded `first_failing_hop = "no-failure-observed"`,
  `unrestricted_control_loaded = true`, `prompt_before_password = false`, and
  `raw_password_leaks = []`.
- `rg -n "issue834pdf|issue834wrong|owner-issue834-exp7" logs/issue-834-exp8-password-enter-before logs/issue-834-exp8-password-enter-after logs/issue-834-exp8-password-wrong-enter-after logs/issue-834-exp8-password-control-after -S`
  produced no matches.

Keyboard regressions:

```bash
python3 scripts/test-issue-834-pdf-navigation.py \
  --log-dir logs/issue-834-exp8-keyboard-page-scroll-regression \
  --serve-bitcoin-pdf \
  --probe keyboard-page-scroll
python3 scripts/test-issue-834-pdf-navigation.py \
  --log-dir logs/issue-834-exp8-toolbar-page-selector-regression \
  --serve-bitcoin-pdf \
  --probe toolbar-page-selector
python3 scripts/test-issue-834-pdf-find.py \
  --log-dir logs/issue-834-exp8-find-positive-regression \
  --probe positive-search
```

All regression probes exited 0. Their summaries recorded
`first_failing_hop = "no-failure-observed"`.

Hygiene:

```bash
git -C chromium/src status --short --branch
git diff --check
```

`chromium/src` was clean after the Chromium commit, and `git diff --check`
passed before result review.

## Completion Review

Fresh-context adversarial review by Codex subagent `Bernoulli`: **Approved**.

Required findings: none.

The reviewer verified that Chromium is on branch `148.0.7778.97-issue-834-exp8`
at `0195b42d78`, the Chromium checkout is clean, the main repo result commit had
not been made before review, the Chromium commit contains only the narrow PDF
password dialog change, the patch archive contains
`0076-Let-PDF-passwords-hear-Enter.patch` matching Chromium `HEAD`, the README
marks Experiment 8 as Pass, the evidence summaries prove pre-fix failure,
post-fix Enter success, wrong-password Enter rejection, unrestricted control
success, and keyboard/find regressions, and no DevTools DOM mutation,
harness-side submit bypass, or test-only product bypass was found.

Optional finding: the local Chromium checkout does not resolve `148.0.7778.97`
as a tag/ref, so the reviewer could not independently run the documented
`git format-patch 148.0.7778.97..HEAD` command. The current archive was
generated from the local base root commit instead and contains the expected
76-patch stack. This should be cleaned up later by restoring or documenting the
local base ref, but it does not block this result.

## Conclusion

Roamium password-protected PDFs now support full keyboard submission parity for
the tested Chromium PDF password dialog:

- password prompt appears before credential entry;
- the password is typed through TermSurf protocol key events;
- Enter key down/up events are delivered through the TermSurf protocol and
  submit the dialog;
- correct-password Enter submission loads the PDF;
- wrong-password Enter submission keeps the dialog open and exposes the invalid
  password state;
- unrestricted PDFs still load normally;
- existing PDF keyboard navigation, toolbar page selector, and find/search
  probes still pass.

The password-protected PDF row can now be treated as proven for Roamium's core
workflow. Surfari remains untested for this row.
