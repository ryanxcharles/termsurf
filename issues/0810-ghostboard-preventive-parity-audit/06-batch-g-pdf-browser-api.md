# Experiment 6: Batch G PDF and Browser API Audit

## Description

Classify Batch G from Experiment 4: issues `0789`-`0799`. This batch covers PDF
viewer infrastructure, PDF viewer interactions, native print, PDF workflow
coverage, advanced PDF features, Chromium/app-shell embedding research, and
browser API automation triage.

This experiment should read every Batch G issue and map each durable lesson to
current Ghostboard risk using the schema defined in Experiment 4. The output is
a classification table, not fixes.

This is an audit/documentation experiment only. It must not change application
code, generated code, historical issue files, closed issue files, scripts, test
harnesses, or PDF assets.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/06-batch-g-pdf-browser-api.md`
  - record this experiment design, design review, Batch G classification result,
    completion review, and conclusion;
  - classify every issue in Batch G using the Experiment 4 historical audit row
    schema.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 6 to the `## Experiments` index with status `Designed`, then
    update status after the result.

No application code, generated protobuf code, historical issue files, closed
issue files, scripts, test harnesses, or PDF assets should be edited.

## Verification

Design-gate pass criteria:

- The issue README links this experiment as `Designed`.
- A fresh-context adversarial design review approves the plan.
- The plan commit exists before implementation begins.

Implementation pass criteria:

- The result audits every Batch G issue exactly once:
  - `0789-electron-style-pdf-viewer`
  - `0790-pdf-viewer-mojo-bindings`
  - `0791-app-shell-foundation`
  - `0792-pdf-support`
  - `0793-pdf-iframe-size`
  - `0794-pdf-viewer-interactions`
  - `0795-pdf-native-print`
  - `0796-pdf-implementation-audit`
  - `0797-pdf-core-workflow-coverage`
  - `0798-pdf-advanced-features`
  - `0799-browser-api-automation-triage`
- The result uses the Experiment 4 row schema for every classification: source
  issue, batch, subsystem, durable lesson, current Ghostboard relevance,
  evidence paths, likelihood, risk or impact, recommended follow-up, and
  historical classification note.
- The result classifies each row as `Highly likely`, `Maybe`, or `No`, and
  explains the classification from issue evidence plus current code/test/doc
  evidence.
- The result treats open issues `0795`, `0797`, and `0798` as open historical
  evidence without trying to close or modify them.
- The result distinguishes PDF/Roamium/browser-engine risk from Ghostboard GUI
  risk. A PDF feature can be important without being a Ghostboard bug if it is
  owned by Roamium, webtui, Chromium, or PDF extension code.
- The result distinguishes current Ghostboard ordinary browsing evidence from
  unproven PDF-specific and browser-API workflows.
- The result carries forward relevant Issue 810 findings where they affect Batch
  G, especially GUI-responsibility messages that direct webtui/Roamium paths
  cannot cover.
- The result identifies the next audit slice after Batch G.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/06-batch-g-pdf-browser-api.md
  ```

- Whitespace check passes:

  ```bash
  git diff --check
  ```

- A fresh-context completion review approves the completed result before the
  result commit.
- All real completion-review findings are fixed and recorded in this experiment
  file.
- The result commit is made after completion-review approval and before any next
  experiment is designed.

Fail criteria:

- Any Batch G issue is omitted or classified more than once.
- The experiment edits historical issue files, application code, scripts, tests,
  or PDF assets.
- The result treats open PDF issues as closed or solved.
- The result labels an engine-owned PDF gap as a Ghostboard GUI gap without
  evidence of Ghostboard involvement.
- The result expands into other historical batches before Batch G is concluded.

## Design Review

Fresh-context adversarial design review returned **APPROVED**.

Reviewer checks confirmed:

- The README links Experiment 6 as `Designed`.
- The design has `Description`, `Changes`, and `Verification`.
- Scope is audit-only and excludes code, generated code, historical issue files,
  scripts, test harnesses, and PDF assets.
- Batch G is exactly `0789`-`0799`, each listed once.
- Verification requires the Experiment 4 schema, open issue handling for `0795`,
  `0797`, and `0798`, PDF/Roamium/browser-engine versus Ghostboard GUI
  separation, and carried-forward GUI-responsibility findings.
- `git diff --check` passed.
- The plan commit had not yet been made before review.

Findings: none.
