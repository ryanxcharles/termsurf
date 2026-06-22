# Experiment 14: Compare Roamium PDF Form Input Paths

## Description

Experiment 13 proved that Roamium PDF form controls work individually and that
page-background focus resets make same-document sequences work. It did not prove
whether direct control-to-control failure is a TermSurf/Roamium input-routing
bug or Chromium PDF viewer behavior.

This experiment should answer that question by comparing two input paths against
the same generated AcroForm fixture and the same calibrated field coordinates:

- TermSurf protocol input, as used by the real product path;
- Chromium DevTools `Input.dispatchMouseEvent` / `Input.dispatchKeyEvent`, as a
  browser-native comparison path inside the same Roamium build.

If DevTools input succeeds for direct same-document form switching but TermSurf
protocol input fails, the next step is a product fix in Roamium input routing.
If both input paths fail the same direct sequences and both succeed with the
same focus resets, the correct result is to classify this as Chromium PDF form
focus semantics and keep the reset-path regression guard without changing
product source.

Do not broaden this experiment to annotations, context menus, native print,
Surfari/WebKit, or non-form PDF behavior.

## Changes

1. Extend the forms probe with a DevTools input mode.

   Update `scripts/probe-pdf-forms.mjs` and
   `scripts/test-issue-834-pdf-forms.py` so the harness can run each form
   sequence through either:

   - TermSurf protocol events (`--input-path termsurf`);
   - DevTools input events (`--input-path devtools`).

   DevTools mode should still launch repo-built Roamium through the same harness
   and load the same served PDF fixture. The only intended difference is the
   event injection path.

2. Reuse the calibrated geometry and fixture validation from Experiment 13.

   The comparison must use:

   - the same deterministic AcroForm fixture;
   - the same `qpdf --check` fixture validation;
   - the same live plugin/page geometry;
   - the same computed field coordinates;
   - the same screenshot-diff classifier.

   Do not hard-code screen coordinates from a prior run.

3. Run matched sequence cases for both input paths.

   At minimum, run these scenarios for both `termsurf` and `devtools`:

   - `text`;
   - `checkbox`;
   - `text-then-checkbox`;
   - `checkbox-then-text`;
   - `text-bg-checkbox`;
   - `checkbox-bg-text`.

   The summary must record results in a structure that makes the A/B comparison
   explicit, for example:

   - `termsurf_results`;
   - `devtools_results`;
   - `input_path_divergences`;
   - `first_failing_hop`.

4. Classify the first failing layer.

   Use named classifications:

   - `fixture-generation-gap`;
   - `pdf-load-failed`;
   - `devtools-target-discovery-failed`;
   - `form-geometry-observable-missing`;
   - `protocol-input-not-sent`;
   - `roamium-input-trace-missing`;
   - `devtools-input-not-sent`;
   - `devtools-input-state-missing`;
   - `termsurf-devtools-divergence`;
   - `chromium-pdf-focus-semantics`;
   - `form-sequence-workaround-required`;
   - `product-fix-required`;
   - `no-failure-observed`.

5. Make product changes only if the comparison proves they are required.

   If DevTools input proves direct sequence behavior works and TermSurf protocol
   input fails, identify the narrowest product integration point and fix it.
   Possible areas include Roamium mouse event translation, focus transfer, key
   target routing after PDF form focus changes, or PDF plugin focus state.

   If both input paths behave the same, do not change product source. Record
   whether the supported behavior should be direct switching, explicit reset
   behavior, or a documented Chromium PDF focus limitation with regression
   coverage for known working paths.

   If Chromium source under `chromium/src/` must be modified:

   - create a fresh Issue 834 Chromium branch before editing;
   - update `chromium/README.md` with the branch;
   - build the affected target;
   - regenerate the Issue 834 Chromium patch archive.

   If any Rust source changes, run `cargo fmt` and accept its output before
   running the relevant Rust build/test command.

## Verification

Verification for the completed result is:

```bash
node --check scripts/probe-pdf-forms.mjs

PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-forms.py

python3 scripts/test-issue-834-pdf-forms.py \
  --log-dir logs/issue-834-exp14-roamium-pdf-form-input-paths \
  --input-path compare

git diff --check
```

If product source changes, also run the relevant build/test commands and rerun
the Experiment 14 comparison against the rebuilt binary. Record all
product-change verification commands before completion review.

Required evidence:

- fixture validation is recorded;
- both input paths use the same fixture, geometry, and classifier;
- the summary records every matched scenario for both input paths;
- the summary records whether the two paths diverge;
- if there is divergence, the result identifies the likely TermSurf/Roamium
  failing layer;
- if there is no divergence, the result explains whether this is Chromium PDF
  form focus semantics or still too ambiguous to decide;
- no product source is changed unless comparison evidence requires it;
- markdown is formatted with Prettier;
- any Node helper passes `node --check`;
- any Python helper passes `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile`,
  and `scripts/__pycache__/` is removed afterward;
- `git diff --check` passes;
- design review is recorded, all required design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Pass Criteria

This experiment passes if it conclusively distinguishes TermSurf/Roamium input
routing from Chromium PDF focus behavior and either fixes a proven product
divergence or records a documented no-product-change classification with
regression evidence.

## Partial Criteria

This experiment is partial if it produces useful A/B evidence but DevTools input
cannot be made comparable enough to decide whether the direct sequence failure
is product-specific.

## Failure Criteria

This experiment fails if the comparison uses different fixtures or geometry for
the two paths, claims Chromium behavior from TermSurf-only evidence, changes
product source before proving divergence, or records path parity/divergence
without stable per-scenario evidence.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Verdict: **Approved**.

The reviewer found no required changes.
