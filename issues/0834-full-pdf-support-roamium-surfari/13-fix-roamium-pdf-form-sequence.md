# Experiment 13: Fix Roamium PDF Form Sequences

## Description

Experiment 12 proved that individual Roamium PDF form controls work when tested
in fresh document instances:

- the text field accepted typed input with localized screenshot evidence;
- the checkbox toggled with localized screenshot evidence.

It also found a same-document sequence gap:

- `text-then-checkbox` failed with `form-checkbox-state-missing`;
- `checkbox-then-text` failed with `form-text-value-missing`.

This experiment should diagnose and fix, or precisely classify, same-document
multi-control PDF form interaction. The goal is to determine whether the gap is
caused by the hand-generated AcroForm fixture, Chromium PDF form focus behavior,
or TermSurf/Roamium focus and input routing.

Do not broaden this experiment to annotations, context menus, native print,
Surfari/WebKit, or non-form PDF behavior.

## Changes

1. Extend the forms harness with sequence diagnostics.

   Update `scripts/test-issue-834-pdf-forms.py` and
   `scripts/probe-pdf-forms.mjs` so the sequence scenarios record more than
   screenshot diffs. At minimum, record state before and after each interaction:

   - PDF viewer/plugin load state;
   - `document.activeElement`;
   - viewer properties related to form focus, especially `formFieldFocus_` and
     `documentHasFocus_` if exposed;
   - plugin rect, page rect, field screen rects, and click coordinates;
   - focused/active state changes visible through DevTools;
   - Roamium input trace lines for every mouse and keyboard event.

2. Validate the fixture before blaming product code.

   The deterministic AcroForm fixture must be checked for validity and
   interactivity assumptions. Use available local tools such as `qpdf --check`
   and source-level PDF inspection. If the fixture is invalid or underspecified,
   fix the fixture first and rerun Experiment 12-style scenarios.

   The result must explicitly answer whether the fixture is good enough to
   support a product conclusion.

3. Test focus-reset variants before editing product source.

   Add sequence variants that try small, user-realistic focus resets between
   controls, such as:

   - text, click page background, checkbox;
   - text, Escape, checkbox;
   - checkbox, click page background, text;
   - checkbox, Escape, text;
   - double-clicking the second control if single-click focus transfer is the
     only failing behavior.

   Record each variant separately with a named result. Do not choose a
   workaround as the product behavior unless it matches a reasonable user action
   and is documented as such.

4. Identify the first failing layer.

   Use named classifications:

   - `fixture-generation-gap`;
   - `pdf-load-failed`;
   - `devtools-target-discovery-failed`;
   - `form-geometry-observable-missing`;
   - `protocol-input-not-sent`;
   - `roamium-input-trace-missing`;
   - `form-focus-transfer-missing`;
   - `form-text-value-missing`;
   - `form-checkbox-state-missing`;
   - `form-sequence-workaround-required`;
   - `product-fix-required`;
   - `no-failure-observed`.

5. Make product changes only if diagnostics prove they are required.

   If the sequence failure is caused by TermSurf/Roamium input routing, make the
   smallest required product fix and rerun all Experiment 12 individual and
   sequence scenarios. Possible areas include mouse focus transfer, key target
   routing after PDF form focus changes, or PDF plugin focus state restoration.

   If Chromium source under `chromium/src/` must be modified:

   - create a fresh Issue 834 Chromium branch before editing;
   - update `chromium/README.md` with the branch;
   - build the affected target;
   - regenerate the Issue 834 Chromium patch archive.

   If the root cause is fixture quality or Chromium-native behavior that does
   not require a TermSurf product change, record that conclusion and do not edit
   product source.

## Verification

Verification for the completed result is:

```bash
node --check scripts/probe-pdf-forms.mjs

PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-forms.py

python3 scripts/test-issue-834-pdf-forms.py \
  --log-dir logs/issue-834-exp13-roamium-pdf-form-sequences

git diff --check
```

If the experiment changes product source, also run the relevant build/test
commands and rerun the Experiment 12 final forms probe against the rebuilt
binary. If any Rust source changes, run `cargo fmt` and accept its output before
running the relevant Rust build/test command. Record all product-change
verification commands before completion review.

Required evidence:

- fixture validity is checked and recorded;
- every same-document sequence and focus-reset variant records a named
  classification;
- the summary records interaction-by-interaction DevTools state, geometry,
  screenshots, and Roamium input traces;
- the result explains whether the first failing layer is fixture, Chromium PDF
  form behavior, or TermSurf/Roamium integration;
- no non-form PDF behavior is changed without evidence that the forms fix
  requires it;
- markdown is formatted with Prettier;
- any Node helper passes `node --check`;
- any Python helper passes `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile`,
  and `scripts/__pycache__/` is removed afterward;
- `git diff --check` passes;
- design review is recorded, all required design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Pass Criteria

This experiment passes if same-document text-field and checkbox interactions
work in both orders through the real TermSurf/Roamium PDF path, with stable
evidence for text value and checkbox state.

## Partial Criteria

This experiment is partial if it does not fully fix same-document form
sequences, but it proves the first failing layer and leaves a concrete next
implementation step.

## Failure Criteria

This experiment fails if it claims a product bug before validating the fixture,
relies on uncalibrated coordinates, changes product source before diagnostics
require it, or records sequence status without stable per-interaction evidence.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Initial verdict: **Changes Required**.

Required finding:

- The design allowed TermSurf/Roamium input-routing fixes but did not explicitly
  require `cargo fmt` if Rust source changed. The verification section now
  requires `cargo fmt` for Rust source changes, accepts formatter output, and
  requires product-change verification commands to be recorded before completion
  review.

Re-review verdict: **Approved**.

## Result

**Result:** Partial

The harness now validates the generated fixture and runs the individual
controls, the two direct same-document sequences, and six focus-reset variants
in one aggregate command:

```bash
python3 scripts/test-issue-834-pdf-forms.py \
  --log-dir logs/issue-834-exp13-roamium-pdf-form-sequences
```

The final summary is:

`logs/issue-834-exp13-roamium-pdf-form-sequences/pdf-forms-summary.json`

It records:

- `first_failing_hop = "form-sequence-workaround-required"`;
- `text_scenario.first_failing_hop = "no-failure-observed"`;
- `checkbox_scenario.first_failing_hop = "no-failure-observed"`;
- direct `text-then-checkbox = "form-checkbox-state-missing"`;
- direct `checkbox-then-text = "form-text-value-missing"`;
- successful focus-reset variants:
  - `text-bg-checkbox`;
  - `text-escape-checkbox`;
  - `checkbox-bg-text`.

Fixture validation is recorded in each child summary. `qpdf --check` passes for
the generated AcroForm fixture:

```text
PDF Version: 1.7
File is not encrypted
File is not linearized
No syntax or stream encoding errors found; the file may still contain
errors that qpdf cannot detect
```

The key scenario evidence is:

- individual text scenario: `5112` changed pixels inside the text-field region
  and `0` outside;
- individual checkbox scenario: `2655` changed pixels inside the checkbox region
  and `0` outside;
- direct `text-then-checkbox`: text still changes, but checkbox has `0` changed
  pixels inside the checkbox region;
- direct `checkbox-then-text`: checkbox still changes, but text has `0` changed
  pixels inside the text-field region;
- `text-bg-checkbox`: both text and checkbox regions change after an
  intermediate page-background click;
- `text-escape-checkbox`: both text and checkbox regions change after
  intermediate Escape;
- `checkbox-bg-text`: both checkbox and text regions change after an
  intermediate page-background click;
- `checkbox-escape-text` still fails text entry;
- double-clicking the second control does not fix either direction.

No product source changed. The first failing layer is classified as focus
transfer behavior rather than fixture generation or basic input routing: the
fixture is syntactically valid, individual controls work, TermSurf/Roamium input
traces are present, and realistic reset actions can restore the next control in
some directions.

Verification commands:

```bash
node --check scripts/probe-pdf-forms.mjs

PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-forms.py

python3 scripts/test-issue-834-pdf-forms.py \
  --log-dir logs/issue-834-exp13-roamium-pdf-form-sequences

git diff --check
```

## Completion Review

Initial verdict: **Changes Required**.

Required finding:

- `scripts/__pycache__/` was present after the reviewer's `py_compile` check.
  The generated cache directory was removed before commit.

Re-review verdict: **Approved**.

The reviewer verified that `scripts/__pycache__/` was gone and that no new
required finding was introduced by the cleanup.

## Conclusion

Experiment 13 narrows the Roamium PDF forms gap. Basic form input is not broken:
individual text and checkbox controls work, and the generated fixture passes
`qpdf --check`. The remaining issue is same-document focus transfer between PDF
form controls. Direct control-to-control switching fails in both directions, but
clicking the page background between controls succeeds in both directions, and
Escape succeeds for text-to-checkbox.

The next experiment should decide whether TermSurf should send or emulate a
focus-reset behavior when moving between PDF form controls, or whether the
correct product behavior is to document this as Chromium PDF viewer focus
semantics and keep a regression guard for the working reset paths.
