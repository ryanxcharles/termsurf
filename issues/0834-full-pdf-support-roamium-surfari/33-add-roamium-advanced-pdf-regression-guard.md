# Experiment 33: Add Roamium Advanced PDF Regression Guard

## Description

Experiments 30 through 32 classified the remaining Roamium advanced PDF rows:

- existing annotations render, while annotation editing is disabled by Chromium
  feature flags;
- accessibility/searchify exposes a DevTools accessibility tree, while Searchify
  is disabled/inactive by flags;
- context menus are now safety-classified, and the harness refuses to send a PDF
  right-click until a targeted native-menu watcher proves observe-and-dismiss
  readiness.

Those results are currently proven by direct advanced-harness commands and log
artifacts, but they are not yet part of
`scripts/test-issue-834-roamium-pdf-regression.py`. Issue 834 requires durable
Roamium regression guards before the Surfari phase begins, so the next step is
to add an explicit advanced tier to the Roamium PDF regression runner.

The goal is durability and honest classification. Do not broaden this into new
product behavior, Surfari/WebKit work, Chromium changes, or native OS UI
automation beyond the existing safety-gated context-menu preflight.

## Changes

- Update `scripts/test-issue-834-roamium-pdf-regression.py`.
- Add a new explicit tier, tentatively named `advanced`.
- The tier should orchestrate the existing advanced harness instead of
  duplicating feature logic:

  ```bash
  python3 scripts/test-issue-834-pdf-advanced.py --probe annotations
  python3 scripts/test-issue-834-pdf-advanced.py --probe accessibility-searchify
  python3 scripts/test-issue-834-pdf-advanced.py --probe context-menu
  ```

- Include the advanced forms smoke path if it is useful to prove the shared
  advanced harness still preserves protocol mouse evidence:

  ```bash
  python3 scripts/test-issue-834-pdf-advanced.py --probe forms
  ```

- Add classifier support for advanced summaries. The runner should not treat all
  non-`no-failure-observed` hops as failures when an experiment has explicitly
  classified an accepted current state. At minimum:
  - every advanced child check must run in a fresh per-child log directory, exit
    `0`, and write a current summary with `probe_status = "ok"` before it can
    pass or be accepted as a limitation;
  - annotations should pass only when `annotation_rendering.status = "pass"` and
    the top-level result is `no-failure-observed`;
  - accessibility/searchify should pass as an accepted limitation only when
    `accessibility_searchify.classification = "accessibility-searchify-disabled-by-flags"`
    and PDF load proof still passes;
  - context-menu safety should pass as an accepted limitation only when
    `context_menu.classification = "context-menu-native-watcher-missing"`,
    `right_click.sent = false`, protocol mouse messages are zero, and cleanup
    proves no native menu is left open;
  - advanced forms, if included, should pass as an accepted limitation only when
    it preserves the known `form-value-observable-missing` classification and
    `roamium_mouse_event_line = true`.
- Keep `smoke`, `focused`, `forms`, `native-print`, and `unsafe-manual` behavior
  unchanged.
- Extend the runner's tier choices and summary output only as needed for the new
  tier.
- Do not modify `scripts/test-issue-834-pdf-advanced.py` unless the regression
  runner exposes a concrete summary-field gap that must be fixed.
- Do not modify Chromium, Roamium product code, Ghostboard, Surfari/WebKit,
  protobuf, or native print behavior.

## Verification

Run syntax and hygiene checks:

```bash
rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-roamium-pdf-regression.py \
  scripts/test-issue-834-pdf-advanced.py
rm -rf scripts/__pycache__
node --check scripts/probe-pdf-advanced.mjs
git diff --check
git -C chromium/src diff --check
```

Run the new advanced tier:

```bash
rm -rf logs/issue-834-exp33-advanced
python3 scripts/test-issue-834-roamium-pdf-regression.py \
  --log-dir logs/issue-834-exp33-advanced \
  --tier advanced
```

Inspect the summary:

```bash
python3 - <<'PY'
import json
from pathlib import Path

summary = json.loads(
    Path(
        "logs/issue-834-exp33-advanced/"
        "roamium-pdf-regression-summary.json"
    ).read_text()
)
print(json.dumps({
    "overall_result": summary.get("overall_result"),
    "first_failing_hop": summary.get("first_failing_hop"),
    "checks": [
        {
            "name": check.get("name"),
            "result": check.get("result"),
            "first_failing_hop": check.get("first_failing_hop"),
            "accepted_limitation": check.get("accepted_limitation"),
        }
        for check in summary.get("checks", [])
    ],
}, indent=2, sort_keys=True))
PY
```

Run at least one existing cheap tier to prove tier behavior is not disrupted:

```bash
rm -rf logs/issue-834-exp33-smoke
python3 scripts/test-issue-834-roamium-pdf-regression.py \
  --log-dir logs/issue-834-exp33-smoke \
  --tier smoke
```

Pass criteria:

- the new `advanced` tier exits 0 and records `overall_result = "pass"`;
- the tier includes the advanced annotation, accessibility/searchify, and
  context-menu safety checks;
- every child check exits `0`, writes a fresh summary, and records
  `probe_status = "ok"` before the runner can classify it as pass or accepted
  limitation;
- each accepted limitation is explicit in the regression summary, not silently
  treated as a normal pass;
- context-menu safety does not send a PDF right-click unless watcher readiness
  is proven;
- no native menu is left open;
- the existing `smoke` tier still exits 0;
- no unrelated product code is changed;
- no Chromium source is changed;
- hygiene checks pass.

Partial criteria:

- the runner adds the advanced tier, but one advanced check records a concrete
  failing hop that requires a follow-up experiment while preserving safety.

Failure criteria:

- the runner reports `advanced` success without checking the compact
  `annotation_rendering`, `accessibility_searchify`, or `context_menu` objects;
- context-menu safety is treated as product support;
- a nonzero child command, missing summary, stale summary, or child
  `probe_status` other than `ok` is accepted as a pass or accepted limitation;
- a PDF right-click is sent before watcher readiness is proven;
- native OS UI is opened and not dismissed;
- existing tiers change behavior without explicit need;
- stale child summaries can make the advanced tier pass;
- product code, Chromium, Surfari/WebKit, or native print behavior is changed.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Required finding:

- The design under-specified the child-summary validity checks needed for a
  durable advanced tier. It required compact-object fields for accepted
  limitations but did not require each child command to return `0`, each child
  summary to have `probe_status = "ok"`, or the runner to reject accepted
  limitations from nonzero child return codes.

Fix:

- The design now requires every advanced child check to run in a fresh per-child
  log directory, exit `0`, and write a current summary with
  `probe_status = "ok"` before it can pass or be accepted as a limitation.
- Pass criteria now require child exit `0`, a fresh summary, and
  `probe_status = "ok"`.
- Failure criteria now reject accepting a nonzero child command, missing
  summary, stale summary, or `probe_status` other than `ok` as a pass or
  accepted limitation.

Final verdict after re-review: **Approved**.

The re-review found no findings.

## Result

**Result:** Pass

Added an explicit `advanced` tier to
`scripts/test-issue-834-roamium-pdf-regression.py`. The tier orchestrates the
existing advanced PDF harness instead of duplicating probe logic.

The new tier runs:

- `advanced-annotations`;
- `advanced-accessibility-searchify`;
- `advanced-context-menu-safety`;
- `advanced-forms-smoke`.

The runner now has dedicated advanced classifiers:

- annotation rendering passes only when the child exits `0`, records
  `probe_status = "ok"`, has `first_failing_hop = "no-failure-observed"`, and
  proves `annotation_rendering.status = "pass"` with a passing load proof;
- accessibility/searchify is an accepted limitation only when the child exits
  `0`, records `probe_status = "ok"`, reports
  `accessibility_searchify.classification = "accessibility-searchify-disabled-by-flags"`,
  and has a passing PDF load proof;
- context-menu safety is an accepted limitation only when the child exits `0`,
  records `probe_status = "ok"`, reports
  `context_menu.classification = "context-menu-native-watcher-missing"`, sends
  no PDF right-click, sends zero protocol mouse messages, and proves no native
  menu is left open;
- the shared advanced forms smoke is an accepted limitation only when the child
  exits `0`, records `probe_status = "ok"`, preserves the known
  `form-value-observable-missing` classification, and records
  `roamium_mouse_event_line = true`.

The runner also now fails nonzero child commands that would otherwise classify
as `pass` or `accepted-limitation`, preventing failed advanced children from
passing because of stale or partial summary content.

Verification run:

```bash
rm -rf logs/issue-834-exp33-advanced
python3 scripts/test-issue-834-roamium-pdf-regression.py \
  --log-dir logs/issue-834-exp33-advanced \
  --tier advanced

rm -rf logs/issue-834-exp33-smoke
python3 scripts/test-issue-834-roamium-pdf-regression.py \
  --log-dir logs/issue-834-exp33-smoke \
  --tier smoke

rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-roamium-pdf-regression.py \
  scripts/test-issue-834-pdf-advanced.py
rm -rf scripts/__pycache__
node --check scripts/probe-pdf-advanced.mjs
git diff --check
git -C chromium/src diff --check
```

Final advanced-tier evidence:

- `logs/issue-834-exp33-advanced/roamium-pdf-regression-summary.json` recorded
  `overall_result = "pass"`, `first_failing_hop = "no-failure-observed"`, and
  duration `29.918` seconds.
- `advanced-annotations` exited `0`, recorded `summary_status = "ok"`,
  `first_failing_hop = "no-failure-observed"`, and result `pass`.
- `advanced-accessibility-searchify` exited `0`, recorded
  `summary_status = "ok"`,
  `first_failing_hop = "accessibility-searchify-disabled-by-flags"`, and result
  `accepted-limitation`.
- `advanced-context-menu-safety` exited `0`, recorded `summary_status = "ok"`,
  `first_failing_hop = "context-menu-native-watcher-missing"`, and result
  `accepted-limitation`.
- `advanced-forms-smoke` exited `0`, recorded `summary_status = "ok"`,
  `first_failing_hop = "form-value-observable-missing"`, and result
  `accepted-limitation`.

Existing-tier sanity:

- `logs/issue-834-exp33-smoke/roamium-pdf-regression-summary.json` recorded
  `overall_result = "pass"`, `first_failing_hop = "no-failure-observed"`,
  duration `31.984` seconds, and the existing two smoke checks still passing.

## Conclusion

Roamium's advanced PDF classifications are now protected by the durable
regression runner. The new tier keeps routine smoke/focused runs unchanged while
giving advanced PDF work a direct guard that captures real passes separately
from accepted current limitations.

The Roamium phase now has durable guards for the latest advanced rows from
Experiments 30 through 32. The next Issue 834 experiment can begin the
Surfari/WebKit PDF audit phase unless a completion review identifies a remaining
Roamium guard gap.

## Completion Review

An external Codex review checked the completed implementation and result.

Initial verdict: **Changes required**.

Required finding:

- Some future product-support branches could still report `pass` without
  checking compact proof objects. In particular, accessibility/searchify could
  pass on top-level `no-failure-observed` without accessibility/searchify proof,
  context menus could pass on
  `context_menu.classification = "no-failure-observed"` without verifying routed
  click, native menu, and cleanup proof, and advanced forms could pass on
  top-level `no-failure-observed` without forms-specific proof.

Fix:

- Accessibility/searchify product pass now requires a passing load proof, an
  observable PDF iframe accessibility tree, and active Searchify text evidence.
- Context-menu product pass now requires passing load proof, a sent right-click,
  protocol mouse messages, Roamium mouse trace evidence, native menu
  observation, cleanup execution, and proof that the menu is gone.
- Advanced forms no longer has a generic top-level `no-failure-observed` pass
  branch; the current shared forms smoke is only accepted through the explicit
  known-limitation path.

Additional verification after the fix:

```bash
rm -rf logs/issue-834-exp33-advanced
python3 scripts/test-issue-834-roamium-pdf-regression.py \
  --log-dir logs/issue-834-exp33-advanced \
  --tier advanced

rm -rf logs/issue-834-exp33-smoke
python3 scripts/test-issue-834-roamium-pdf-regression.py \
  --log-dir logs/issue-834-exp33-smoke \
  --tier smoke

rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-roamium-pdf-regression.py \
  scripts/test-issue-834-pdf-advanced.py
rm -rf scripts/__pycache__
git diff --check
```

Final verdict after re-review: **Approved**.

The re-review found no findings and confirmed that accessibility/searchify,
context-menu, and forms no longer have generic top-level `no-failure-observed`
pass paths without compact proof.
