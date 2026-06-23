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

## Result

**Result:** Pass

The A/B comparison proved the direct PDF form sequence failure was specific to
the TermSurf/Roamium input path, not Chromium PDF form focus semantics.

Before the product fix, the same generated AcroForm fixture and calibrated field
geometry produced this comparison:

- DevTools input: `no-failure-observed`;
- TermSurf protocol input: `form-sequence-workaround-required`;
- direct `text-then-checkbox`: TermSurf `form-checkbox-state-missing`, DevTools
  `no-failure-observed`;
- direct `checkbox-then-text`: TermSurf `form-text-value-missing`, DevTools
  `no-failure-observed`.

The narrow product fix is in `roamium/src/dispatch.rs`: when Roamium receives a
TermSurf mouse `down` event, it now first forwards a synthetic
`ts_forward_mouse_move` at the same coordinates before forwarding the existing
mouse event. This matches the input shape used by Chromium DevTools
`Input.dispatchMouseEvent`, which sends `mouseMoved`, `mousePressed`, and
`mouseReleased`.

The harness was extended so this remains testable:

- `scripts/probe-pdf-forms.mjs` can now dispatch DevTools mouse, text, and
  Escape actions before collecting PDF form snapshots;
- `scripts/test-issue-834-pdf-forms.py` now supports `--input-path termsurf`,
  `--input-path devtools`, and `--input-path compare`;
- the compare mode records `termsurf_results`, `devtools_results`,
  `input_path_divergences`, and `first_failing_hop`;
- per-run sockets now use a short temporary path so nested issue log directories
  do not exceed the macOS `AF_UNIX` path limit.

After rebuilding Roamium, the final comparison log is
`logs/issue-834-exp14-roamium-pdf-form-input-paths-final/pdf-forms-summary.json`.
It records:

- top-level `first_failing_hop`: `no-failure-observed`;
- `input_path_divergences`: `{}`;
- TermSurf path `first_failing_hop`: `no-failure-observed`;
- DevTools path `first_failing_hop`: `no-failure-observed`;
- direct `text`, `checkbox`, `text-then-checkbox`, and `checkbox-then-text`:
  `no-failure-observed` for both paths;
- reset variants `text-bg-checkbox`, `text-escape-checkbox`,
  `text-double-checkbox`, `checkbox-bg-text`, `checkbox-escape-text`, and
  `checkbox-double-text`: `no-failure-observed` for both paths;
- both child commands returned `0`;
- TermSurf trace logs include 22 `pre-click-synthesis` entries.

Verification commands run:

```bash
node --check scripts/probe-pdf-forms.mjs

PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-forms.py

rm -rf scripts/__pycache__

cargo fmt

./scripts/build.sh roamium

python3 scripts/test-issue-834-pdf-forms.py \
  --log-dir logs/issue-834-exp14-roamium-pdf-form-input-paths-final \
  --input-path compare

git diff --check
```

No Chromium source was modified, so no Chromium branch or patch archive was
needed.

## Conclusion

Roamium PDF forms now pass the individual form scenarios, direct same-document
text/checkbox switching scenarios, and the focus-reset variants through the real
TermSurf protocol path. The bug was a product input-routing mismatch: Roamium
was forwarding click press/release without the preceding movement event that
Chromium PDF form focus expected in this path.

The next Issue 834 experiment should move on from Roamium PDF form input. Good
candidates are either converting the Roamium PDF coverage into a durable
regression guard or tackling the next incomplete Roamium PDF area, such as
native print behavior or the remaining advanced PDF surfaces from Experiment 11.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

Verdict: **Approved**.

The reviewer found no required issues. It raised one optional issue:

- compare mode returned success for `termsurf-devtools-divergence`, which made
  the command weaker as a future regression guard.

Accepted fix:

- `scripts/test-issue-834-pdf-forms.py` now returns success from compare mode
  only for `no-failure-observed` or the explicitly accepted
  `chromium-pdf-focus-semantics` classification. It returns failure for
  `termsurf-devtools-divergence` and other unresolved classifications.

The same reviewer re-reviewed the fix and approved it with no findings.
