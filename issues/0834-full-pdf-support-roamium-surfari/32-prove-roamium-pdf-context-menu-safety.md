# Experiment 32: Prove Roamium PDF Context-Menu Safety

## Description

Experiment 11 classified Roamium PDF context menus as
`context-menu-native-watcher-missing`. The harness correctly refused to
right-click inside PDF plugin coordinates because native context menus can open
outside the browser DOM and must be observed and dismissed safely.

The other Roamium advanced rows are now handled:

- forms have a durable comparison guard;
- existing annotations render and annotation editing is classified by Chromium
  flags;
- accessibility/searchify has a compact runtime classification.

The remaining Roamium advanced row is context menus. This experiment should
prove whether a safe native-menu watcher exists in this macOS VM and, only if
that watcher is proven ready, perform a real TermSurf protocol right-click
inside the Roamium PDF plugin.

The goal is context-menu safety and classification, not broad product changes.
Do not modify Chromium or Roamium product code before proving the failing layer.

## Changes

- Update `scripts/test-issue-834-pdf-advanced.py`.
- Update `scripts/probe-pdf-advanced.mjs` only if extra DevTools state is needed
  after the context-menu event.
- Keep the experiment limited to Roamium PDF context-menu evidence. Do not
  modify Chromium, Roamium process code, Ghostboard, Surfari/WebKit, protocol
  code, native print, forms, annotations, or accessibility/searchify behavior.

Implementation should add a safe context-menu watcher with two phases:

1. Watcher preflight.

   Before sending any right-click to the PDF plugin, prove the watcher can
   observe and dismiss a harmless native menu or menu-like surface. Acceptable
   mechanisms include:

   - a Swift Accessibility helper that targets the Roamium process id and looks
     for `AXMenu`, `AXMenuItem`, or equivalent menu roles;
   - a System Events / AppleScript probe only if it is targeted and proves
     assistive access is granted;
   - a screenshot/AX hybrid watcher, if it proves both detection and dismissal.

   The preflight must record:

   - watcher mechanism;
   - target pid;
   - accessibility trust or permission state;
   - whether a harmless menu was detected;
   - whether Escape or another targeted dismissal closed it;
   - a timeout/failure reason if readiness is not proven.

2. PDF context-menu probe.

   Only if preflight succeeds:

   - load `valid.pdf` through the normal TermSurf/Roamium advanced harness;
   - prove the PDF plugin load and plugin rectangle;
   - send a protocol right-click at coordinates inside the visible PDF plugin;
   - record the TermSurf protocol mouse message;
   - record Roamium PDF input trace evidence for the right-click;
   - use the watcher to detect a native menu;
   - dismiss the menu safely and prove it is gone;
   - capture DevTools state and screenshot before and after the event.

   Once any native menu may have opened, the implementation must use
   `try`/`finally` or an equivalent guaranteed cleanup path that attempts
   targeted dismissal even if detection, trace collection, or classification
   fails. The summary must record whether this cleanup path ran and whether the
   menu was gone afterward.

The summary should include a compact `context_menu` object with:

- `classification`;
- `watcher_preflight`;
- `pdf_load_proof`;
- `right_click`;
- `native_menu`;
- `cleanup`;
- source-audit evidence for Chromium PDF context-menu hooks.

Classifications:

- `no-failure-observed` when preflight passes, the right-click is routed, a
  native menu is observed, and cleanup proves it is dismissed;
- `context-menu-native-watcher-missing` when no watcher can be proven ready and
  no PDF right-click is sent;
- `context-menu-permission-denied` when macOS denies the watcher permission;
- `context-menu-right-click-not-routed` when the protocol event is sent but
  Roamium input trace does not see it;
- `context-menu-native-menu-not-observed` when input is routed but no native
  menu appears before timeout;
- `context-menu-cleanup-failed` when a native menu opens but cannot be dismissed
  safely;
- existing setup/load failures such as `pdf-load-failed` or
  `devtools-target-discovery-failed`.

If watcher preflight fails, the experiment may still pass as a safety
classification only if the harness proves it did not send a PDF right-click and
records a concrete watcher failure reason. It must not claim context-menu
product support in that case.

## Verification

Run syntax and hygiene checks:

```bash
node --check scripts/probe-pdf-advanced.mjs
rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-advanced.py
rm -rf scripts/__pycache__
git diff --check
git -C chromium/src diff --check
```

Run the context-menu probe:

```bash
rm -rf logs/issue-834-exp32-context-menu
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp32-context-menu \
  --probe context-menu
```

Inspect the summary:

```bash
python3 - <<'PY'
import json
from pathlib import Path

summary = json.loads(
    Path("logs/issue-834-exp32-context-menu/pdf-advanced-summary.json").read_text()
)
print(json.dumps({
    "first_failing_hop": summary.get("first_failing_hop"),
    "probe_status": summary.get("probe_status"),
    "context_menu": summary.get("context_menu"),
}, indent=2, sort_keys=True))
PY
```

Run shared-harness smoke checks:

```bash
rm -rf logs/issue-834-exp32-annotations-smoke
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp32-annotations-smoke \
  --probe annotations

rm -rf logs/issue-834-exp32-accessibility-smoke
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp32-accessibility-smoke \
  --probe accessibility-searchify

rm -rf logs/issue-834-exp32-forms-smoke
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp32-forms-smoke \
  --probe forms
```

Pass criteria:

- if watcher preflight succeeds, the probe sends exactly one protocol
  right-click inside the PDF plugin, Roamium input trace sees it, the watcher
  observes a native menu, and cleanup proves the menu is dismissed;
- if watcher preflight fails, the probe sends no PDF right-click and records a
  concrete safety classification such as `context-menu-native-watcher-missing`
  or `context-menu-permission-denied`;
- the summary contains the compact `context_menu` object;
- no native menu is left open;
- no unrelated product code is changed;
- no Chromium source is changed;
- shared advanced annotation, accessibility/searchify, and forms smoke checks
  still pass, including the forms probe's expected protocol mouse trace
  evidence;
- hygiene checks pass.

Partial criteria:

- preflight succeeds and the right-click is routed, but the menu is not
  observable or cleanup cannot be proven, with a concrete failing hop;
- or watcher readiness is blocked by a macOS permission state that is recorded
  precisely enough for a follow-up environment fix.

Failure criteria:

- a PDF right-click is sent before watcher readiness is proven;
- a native menu is opened and not dismissed;
- the implementation lacks a guaranteed cleanup path after a native menu may
  have opened;
- the summary claims context-menu support from source presence alone;
- native menu detection is broad/global rather than targeted to the Roamium
  process or a proven safe surface;
- product code is changed before the failing layer is identified;
- shared annotation/accessibility/forms harness paths regress.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Required findings:

- Verification did not include the forms/protocol mouse smoke path, even though
  this experiment may add a real protocol right-click path.
- The design required cleanup proof but did not explicitly require
  `try`/`finally` or an equivalent guaranteed cleanup path after a native menu
  may have opened.

Fixes:

- Added the forms smoke command and pass/failure criteria covering the expected
  forms protocol mouse trace evidence.
- Added an explicit guaranteed cleanup requirement once any native menu may have
  opened, including summary evidence that cleanup ran and the menu was gone
  afterward.

Final verdict after re-review: **Approved**.

The re-review found no findings.

## Result

**Result:** Pass

Implemented a compact Roamium PDF context-menu safety classifier in the advanced
PDF harness.

Changes:

- `scripts/test-issue-834-pdf-advanced.py` now runs a Swift Accessibility
  targeted-menu preflight for the Roamium process when `--probe context-menu` is
  selected.
- The context-menu path records a compact `context_menu` object with:
  - `classification`;
  - `watcher_preflight`;
  - `pdf_load_proof`;
  - `right_click`;
  - `native_menu`;
  - `cleanup`;
  - Chromium PDF context-menu source-audit evidence.
- The harness only permits the product right-click path when watcher readiness
  is proven. In the current run readiness was not proven, so no PDF right-click
  was sent.

Verification run:

```bash
rm -rf logs/issue-834-exp32-context-menu
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp32-context-menu \
  --probe context-menu

rm -rf logs/issue-834-exp32-annotations-smoke
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp32-annotations-smoke \
  --probe annotations

rm -rf logs/issue-834-exp32-accessibility-smoke
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp32-accessibility-smoke \
  --probe accessibility-searchify

rm -rf logs/issue-834-exp32-forms-smoke
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp32-forms-smoke \
  --probe forms

rm -rf scripts/__pycache__
node --check scripts/probe-pdf-advanced.mjs
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-advanced.py
rm -rf scripts/__pycache__
git diff --check
git -C chromium/src diff --check
```

Final context-menu evidence:

- `logs/issue-834-exp32-context-menu/pdf-advanced-summary.json` recorded
  `probe_status = "ok"` and
  `first_failing_hop = "context-menu-native-watcher-missing"`.
- `context_menu.pdf_load_proof.status = "pass"` with all checks true:
  `valid.pdf` loaded, file name and original URL matched, and plugin/toolbar
  rectangles were non-zero.
- `context_menu.watcher_preflight` recorded:
  - mechanism: `swift-accessibility-targeted-menu-scan`;
  - target pid: the Roamium process id from the run;
  - `trusted = true`;
  - `observed_menu = false`;
  - `ready = false`;
  - reason: `targeted-native-menu-not-observed`.
- Since the watcher was not ready:
  - `context_menu.right_click.sent = false`;
  - `protocol_mouse_messages_sent = 0`;
  - `roamium_mouse_event_line = false`;
  - `context_menu.native_menu.observed = false`;
  - `context_menu.cleanup.menu_gone = true` with reason `no-native-menu-opened`.
- Source-audit paths for Chromium PDF context-menu hooks existed:
  `pdf_document_helper.h`, `pdf_document_helper.cc`, and `gesture_detector.ts`.

Shared-harness sanity:

- `logs/issue-834-exp32-annotations-smoke/pdf-advanced-summary.json` recorded
  `first_failing_hop = "no-failure-observed"`,
  `annotation_rendering.status = "pass"`, and `probe_status = "ok"`.
- `logs/issue-834-exp32-accessibility-smoke/pdf-advanced-summary.json` recorded
  `first_failing_hop = "accessibility-searchify-disabled-by-flags"`,
  `accessibility_searchify.classification = "accessibility-searchify-disabled-by-flags"`,
  and `probe_status = "ok"`.
- `logs/issue-834-exp32-forms-smoke/pdf-advanced-summary.json` recorded the
  prior expected classification, `form-value-observable-missing`, with
  `probe_status = "ok"` and `roamium_mouse_event_line = true`.

## Conclusion

Roamium PDF context-menu automation is now safely classified. The current VM has
Accessibility trust for the targeted watcher, but the watcher did not observe a
targeted native menu surface before the PDF right-click. The harness therefore
sent no right-click into the PDF plugin and left no native menu open.

This is a safety pass, not a claim that PDF context-menu product behavior is
proven. It closes the unsafe gap from Experiment 11 by making the safety gate
explicit and machine-checkable. To prove product context-menu behavior later, a
future experiment must first make a targeted watcher observe and dismiss the
actual native menu, then enable the right-click path.

With forms, annotations, accessibility/searchify, native print, and context-menu
safety now classified for Roamium, the next Issue 834 experiment should either
add the remaining Roamium advanced guards to the regression runner or begin the
Surfari/WebKit PDF audit phase.

## Completion Review

An external Codex review checked the completed implementation and result.

Verdict: **Approved**.

The review found no findings. It specifically confirmed that the two prior
completion-review findings were fixed:

- watcher readiness now requires `trusted`, `observed_menu`, and
  `dismissal_proven`, with `dismissal_proven` explicitly false in the current
  preflight, so broad/static menu observation cannot unlock the right-click
  path;
- the gated success path now exists, sends the right-click only after watcher
  readiness and PDF load proof, and wraps native-menu probing in a cleanup path.

The review also confirmed that recording Experiment 32 as **Pass** is acceptable
as a safety classification because the docs do not claim product context-menu
support and the logs prove no PDF right-click was sent when the watcher was not
ready.
