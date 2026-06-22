# Experiment 31: Classify Roamium PDF Accessibility and Searchify

## Description

Experiment 11 classified Roamium PDF accessibility/searchify as
`accessibility-searchify-source-only`: the PDF plugin loaded, the Chromium
source contained accessibility and Searchify plumbing, and the viewer exposed a
hidden Searchify progress element, but the probe did not prove active
accessibility or Searchify behavior.

Experiments 12 through 30 have now handled forms, native print, and annotation
rendering. The remaining non-native advanced Roamium row is
accessibility/searchify. The Issue 834 matrix marks accessibility/searchify as
optional, but it still needs a clear Roamium status before the issue can move
cleanly into context menus and Surfari.

This experiment should classify the current Roamium accessibility/searchify
state with stronger evidence than Experiment 11. It should not attempt a broad
Chromium accessibility implementation. The goal is to prove one of:

- the current TermSurf/Roamium PDF path exposes active accessibility/searchify
  runtime behavior;
- Chromium's viewer/searchify runtime is disabled or inactive in this build;
- the automation cannot observe the relevant state yet, with a concrete missing
  layer.

## Changes

- Update `scripts/test-issue-834-pdf-advanced.py`.
- Update `scripts/probe-pdf-advanced.mjs`.
- Keep the experiment limited to Roamium PDF accessibility/searchify evidence.
  Do not modify Chromium, Roamium process code, Ghostboard, Surfari/WebKit,
  protocol code, annotation/form/native-print behavior, or context-menu probing.

The probe should collect and summarize stable runtime evidence from the PDF
viewer and any attached extension iframe:

1. PDF load proof:
   - plugin loaded;
   - viewer title/filename/original URL point to the expected PDF;
   - toolbar and plugin rectangles are non-zero.
2. Searchify state:
   - `searchifyProgress` presence, text, hidden/display state, and rect;
   - `viewerProps.hasSearchifyText_`;
   - `viewerProps.pdfSearchifySaveEnabled_`;
   - `loadTimeData` flags related to searchify, when present;
   - whether source-level Searchify hooks are present in the current Chromium
     checkout.
3. Accessibility state:
   - viewer/plugin properties that expose accessibility state, tree loading, or
     related flags;
   - whether DevTools accessibility APIs can return a meaningful accessibility
     tree for the PDF viewer target or child iframe;
   - source-level accessibility hooks in the current Chromium checkout.
4. Classification:
   - `no-failure-observed` only if active runtime accessibility/searchify state
     is proven;
   - `accessibility-searchify-disabled-by-flags` if runtime flags/properties
     show the feature is disabled;
   - `accessibility-searchify-inactive` if the feature is compiled/present but
     inactive for this PDF and viewer configuration;
   - `accessibility-tree-observable-missing` if DevTools accessibility
     inspection cannot observe the PDF/plugin accessibility tree;
   - `accessibility-searchify-source-only` only if source hooks exist but
     runtime evidence still cannot classify more specifically;
   - existing setup/load failures such as `pdf-load-failed` or
     `devtools-target-discovery-failed`.

The summary should include an `accessibility_searchify` object so future
experiments do not have to infer state from large raw DevTools dumps.

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

Run the accessibility/searchify probe:

```bash
rm -rf logs/issue-834-exp31-accessibility-searchify
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp31-accessibility-searchify \
  --probe accessibility-searchify
```

Inspect the summary:

```bash
python3 - <<'PY'
import json
from pathlib import Path

summary = json.loads(
    Path(
        "logs/issue-834-exp31-accessibility-searchify/"
        "pdf-advanced-summary.json"
    ).read_text()
)
print(json.dumps({
    "first_failing_hop": summary.get("first_failing_hop"),
    "probe_status": summary.get("probe_status"),
    "accessibility_searchify": summary.get("accessibility_searchify"),
}, indent=2, sort_keys=True))
PY
```

Run one shared-harness sanity check that does not use accessibility/searchify:

```bash
rm -rf logs/issue-834-exp31-annotations-smoke
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp31-annotations-smoke \
  --probe annotations

rm -rf logs/issue-834-exp31-forms-smoke
python3 scripts/test-issue-834-pdf-advanced.py \
  --log-dir logs/issue-834-exp31-forms-smoke \
  --probe forms
```

Pass criteria:

- the probe loads the expected PDF through Roamium over the TermSurf protocol;
- the summary contains a compact `accessibility_searchify` object with load
  proof, searchify state, accessibility state, source-audit evidence, and a
  named classification;
- the classification is more specific than Experiment 11's broad
  `accessibility-searchify-source-only`, unless the result proves source-only is
  still the strongest truthful classification;
- no product code is changed;
- no native OS UI is opened;
- the shared advanced annotation and forms harness sanity checks still pass;
- hygiene checks pass.

Partial criteria:

- the probe gathers better state but still cannot distinguish disabled,
  inactive, or unobservable accessibility/searchify behavior;
- or DevTools accessibility inspection fails with a concrete protocol or target
  error while the PDF load proof remains intact.

Failure criteria:

- the experiment claims accessibility/searchify works from source presence
  alone;
- the summary lacks a compact classification object;
- broad Chromium or Roamium product code is changed before the missing layer is
  identified;
- the shared annotation/forms harness path regresses;
- the README or experiment result overstates optional accessibility/searchify as
  complete without active runtime evidence or an explicit unsupported/inactive
  classification.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Initial verdict: **Changes required**.

Required finding:

- Verification only ran the annotation shared-harness smoke check, while the
  failure criteria said annotation/forms harness regression would fail.

Fix:

- Added an explicit forms smoke command to Verification.
- Updated pass criteria to require both annotation and forms advanced harness
  sanity checks.

Final verdict after Codex re-review: **Approved**.

The re-review found no findings.
