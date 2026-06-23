# Experiment 12: Prove Roamium PDF Forms

## Description

Experiment 11 classified Roamium PDF forms as `form-value-observable-missing`,
but its evidence also showed that the form click was not calibrated to the
actual PDF plugin coordinates. The summary for
`logs/issue-834-exp11-advanced-forms-final-v2/pdf-advanced-summary.json`
reported a plugin rect beginning at `x = 301`, while the probe clicked at
`x = 220`. That proves TermSurf mouse and keyboard input reached Roamium, but it
does not prove the input reached the AcroForm field.

This experiment should prove Roamium PDF form support correctly before making
product changes. It will create a calibrated form probe that:

- generates a deterministic AcroForm PDF with known page-space field rectangles;
- measures the live PDF plugin/page geometry through DevTools;
- converts field rectangles to screen-space TermSurf mouse coordinates;
- clicks the text field and checkbox;
- sends keyboard input to the text field;
- records a stable observable proving focus/value/checkbox state, or identifies
  the exact missing layer.

The goal is to close the Roamium `forms` row if forms already work with correct
coordinates. If calibrated input reaches the field but the form still cannot be
focused or edited, the experiment may make the smallest required product fix. It
must not touch annotations, context menus, native print, Surfari/WebKit, or
non-form PDF behavior.

## Changes

1. Add a dedicated calibrated forms probe.

   Add `scripts/test-issue-834-pdf-forms.py` and `scripts/probe-pdf-forms.mjs`.

   The harness should reuse proven pieces from
   `scripts/test-issue-834-pdf-advanced.py` instead of duplicating protocol
   logic unnecessarily, but it should be specific enough that its result is a
   durable forms regression guard candidate.

2. Generate a deterministic AcroForm fixture with explicit field metadata.

   The fixture should include at least:

   - one visible text field with a known field name and page-space rectangle;
   - one visible checkbox with a known field name and page-space rectangle;
   - text labels near the fields so screenshot diffs are interpretable.

   The summary must record the generated file path, byte size, field names,
   page-space rectangles, and generation method. If the hand-built fixture is
   invalid or Chromium ignores its form widgets, fix the fixture before treating
   Roamium as broken.

3. Calibrate PDF field coordinates before sending input.

   The DevTools probe should record:

   - PDF viewer/plugin presence and load success;
   - plugin rect in viewport coordinates;
   - page/container rects if exposed by the viewer;
   - current zoom/viewport state if observable;
   - the computed TermSurf click coordinates for the text field and checkbox.

   Do not hard-code stale click coordinates from Experiment 11. The harness must
   compute coordinates from the live plugin/page geometry and the generated
   field metadata.

4. Probe text-field editing.

   The text-field probe should:

   - click the computed center of the text field;
   - send a short deterministic string, such as `TermSurf834`;
   - capture pre-input and post-input screenshots;
   - record any stable focus/value observable available through DevTools, plugin
     state, PDF viewer state, accessibility state, or saved PDF data.

   Acceptable proof includes any one of:

   - a stable runtime value observable reports the typed string;
   - a saved/downloaded PDF or PDF plugin state contains the typed value;
   - a screenshot diff is localized to the text-field rectangle and independent
     evidence proves the input path targeted that field.

   Screenshot-only evidence is not enough unless it is tied to calibrated field
   geometry and no broader page repaint can explain the diff.

5. Probe checkbox toggling.

   The checkbox probe should:

   - click the computed center of the checkbox;
   - capture pre-click and post-click screenshots;
   - record any stable checked-state observable available through DevTools,
     plugin state, PDF viewer state, accessibility state, or saved PDF data.

   Acceptable proof mirrors the text-field proof: a stable state/value
   observable is preferred, with localized screenshot evidence allowed only if
   it is geometrically tied to the checkbox and no broader repaint can explain
   it.

6. Classify the first failing layer if forms do not pass.

   Use named classifications:

   - `fixture-generation-gap`;
   - `pdf-load-failed`;
   - `devtools-target-discovery-failed`;
   - `form-geometry-observable-missing`;
   - `form-coordinate-calibration-failed`;
   - `protocol-input-not-sent`;
   - `roamium-input-trace-missing`;
   - `form-focus-observable-missing`;
   - `form-text-value-missing`;
   - `form-checkbox-state-missing`;
   - `form-screenshot-evidence-ambiguous`;
   - `product-fix-required`;
   - `no-failure-observed`.

7. Make product changes only after calibrated evidence proves they are needed.

   If the calibrated probe proves that input reaches the field coordinates but
   the text field or checkbox still cannot be edited, identify the narrowest
   product integration point before editing source. Possible areas include
   Roamium input event translation, PDF plugin focus routing, or Chromium PDF
   viewer/plugin integration.

   If Chromium source under `chromium/src/` must be modified:

   - create a fresh Issue 834 Chromium branch before editing;
   - update `chromium/README.md` with the branch;
   - build the affected target;
   - regenerate the Issue 834 Chromium patch archive.

   If only Rust harness code is changed, run the relevant Rust formatting or
   build checks. Do not modify Surfari/WebKit in this experiment.

## Verification

Verification for the completed result is:

```bash
node --check scripts/probe-pdf-forms.mjs

PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-forms.py

python3 scripts/test-issue-834-pdf-forms.py \
  --log-dir logs/issue-834-exp12-roamium-pdf-forms

git diff --check
```

If the experiment changes product source, also run the smallest build/test
commands that prove the changed component still builds and the forms probe
passes against the rebuilt binary. Record those commands in the result before
the completion review.

Required evidence:

- the fixture metadata records field names and page-space rectangles;
- the probe records live plugin/page geometry and computed click coordinates;
- TermSurf protocol mouse and keyboard messages are sent;
- Roamium input traces record the mouse and keyboard input;
- text-field and checkbox outcomes are proven by stable observables or narrowly
  localized screenshot evidence tied to calibrated geometry;
- the summary records one of the named classifications;
- no non-form PDF behavior is changed without explicit evidence that the forms
  fix requires it;
- markdown is formatted with Prettier;
- any Node helper passes `node --check`;
- any Python helper passes `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile`,
  and `scripts/__pycache__/` is removed afterward;
- `git diff --check` passes;
- design review is recorded, all required design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Pass Criteria

This experiment passes if the calibrated probe proves both text-field editing
and checkbox toggling through the real TermSurf/Roamium PDF path, records stable
evidence for both outcomes, and leaves Roamium form support covered by a
repeatable harness.

## Partial Criteria

This experiment is partial if the calibrated probe proves only one of the two
form controls, or if it identifies a concrete product fix that is larger than
this experiment should safely make.

## Failure Criteria

This experiment fails if the valid control PDF cannot load, the AcroForm fixture
is not actually recognized as a form by Chromium, the probe relies on
uncalibrated coordinates, product changes are made before calibrated evidence
requires them, or the result claims form support from ambiguous screenshot
changes or source presence alone.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Initial verdict: **Changes Required**.

Required finding:

- The design made `scripts/probe-pdf-forms.mjs` optional in the Changes section
  but mandatory in Verification. The design now makes both
  `scripts/test-issue-834-pdf-forms.py` and `scripts/probe-pdf-forms.mjs`
  mandatory.

Re-review verdict: **Approved**.

## Result

**Result:** Partial

Implemented a calibrated Roamium PDF forms harness:

- `scripts/test-issue-834-pdf-forms.py` generates a deterministic AcroForm
  fixture, launches repo-built Roamium, drives the TermSurf protocol, computes
  field click coordinates from live PDF plugin geometry, compares screenshot
  regions, and writes `pdf-forms-summary.json`.
- `scripts/probe-pdf-forms.mjs` attaches through DevTools, captures PDF
  viewer/plugin state, records plugin/page-related geometry, and writes
  checkpoint screenshots.

Verification commands:

```bash
node --check scripts/probe-pdf-forms.mjs

PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-forms.py

python3 scripts/test-issue-834-pdf-forms.py \
  --log-dir logs/issue-834-exp12-roamium-pdf-forms-final

git diff --check
```

The final combined summary is:

`logs/issue-834-exp12-roamium-pdf-forms-final/pdf-forms-summary.json`

It records `first_failing_hop = "form-checkbox-state-missing"` for the aggregate
because the two individual-control scenarios pass, but both same-document
sequence scenarios expose focus/input order gaps.

The individual-control proof is:

- text field scenario:
  - click coordinate: `x = 432.9117647058823`, `y = 122.79820261437908`;
  - PDF field rect: `[160, 650, 380, 675]`;
  - screen rect: `x = 379.16993464052285`, `y = 116.69117647058823`,
    `width = 107.48366013071895`, `height = 12.2140522875817`;
  - TermSurf/Roamium trace recorded mouse and keyboard input;
  - localized text-field diff recorded `5112` changed pixels inside the field
    region and `0` changed pixels outside it.
- checkbox scenario:
  - click coordinate: `x = 391.38398692810455`, `y = 148.44771241830065`;
  - PDF field rect: `[160, 585, 210, 635]`;
  - screen rect: `x = 379.16993464052285`, `y = 136.23366013071896`,
    `width = 24.4281045751634`, `height = 24.4281045751634`;
  - TermSurf/Roamium trace recorded mouse input;
  - localized checkbox diff recorded `2655` changed pixels inside the checkbox
    region and `0` changed pixels outside it.

The same-document sequence evidence is:

- text-then-checkbox scenario:
  - `first_failing_hop = "form-checkbox-state-missing"`;
  - text edit still passed with `5112` changed pixels inside the text-field
    region and `0` changed pixels outside it;
  - the subsequent checkbox click recorded `0` changed pixels inside the
    checkbox region, with only `28` changed pixels outside it.
- checkbox-then-text scenario:
  - `first_failing_hop = "form-text-value-missing"`;
  - checkbox toggle still passed with `2655` changed pixels inside the checkbox
    region and `0` changed pixels outside it;
  - the subsequent text input recorded `0` changed pixels inside the text-field
    region, with `248` changed pixels outside it.

The probe also corrected a bad assumption from Experiment 11. Experiment 11
clicked at `x = 220` even though the live plugin rect began at `x = 301`; this
experiment computes coordinates from the live plugin rect and the generated PDF
field rectangles before sending input.

No product source changed.

The result is Partial, not Pass, because the final summary now logs a remaining
same-document sequence gap. Text editing works when it is the first calibrated
form interaction, and checkbox toggling works when it is the first calibrated
form interaction. But in the same document instance, text-then-checkbox fails at
checkbox state, while checkbox-then-text fails at text value.

That sequence behavior may be a fixture limitation, a Chromium PDF form focus
quirk, or a real TermSurf/Roamium integration gap. It is not proven enough to
call Roamium PDF forms complete.

## Completion Review

Initial verdict: **Changes Required**.

Required finding:

- The first completion-review pass found that the Partial rationale depended on
  same-document sequence failures that were not present in the final summary.
  The harness now runs and records `text-then-checkbox` and `checkbox-then-text`
  scenarios under the final summary, and the result section cites those logged
  failures directly.

Re-review verdict: **Approved**.

The reviewer verified that the final summary records both same-document
failures, that the result cites those logged failures directly, that
`scripts/__pycache__/` is gone, and that `git diff --check` passes.

## Conclusion

Experiment 12 proves calibrated individual PDF form controls through the real
TermSurf/Roamium path. It replaces the uncalibrated Experiment 11 forms evidence
with a repeatable harness and concrete geometry/screenshot proof for a text
field and a checkbox.

The next forms experiment should focus narrowly on same-document multi-control
interaction. It should first determine whether the sequence gap is caused by the
hand-generated fixture, Chromium PDF form behavior, or TermSurf/Roamium focus
and input routing.
