# Experiment 9: Batch D Direct Browser and Protocol Audit

## Description

Classify Batch D from Experiment 4: issues `0680`-`0714`. This batch covers the
direct-browser and protocol stabilization era before Wezboard: dark mode,
process/quit commands, direct TUI-to-browser IPC, visited links, DevTools,
multi-profile tracking, Chromium crash recovery, tab lifecycle, file and smart
URL resolution, pane-vs-tab identity, click suppression, Unix socket migration,
protobuf-c generation, Roamium extraction, multi-engine research, Ghostboard and
webtui naming, engine labels, website rename cleanup, and issue-numbering
workflow.

This experiment should read every Batch D issue and map each durable lesson to
current Ghostboard risk using the schema defined in Experiment 4. The output is
a classification table, not fixes.

This is an audit/documentation experiment only. It must not change application
code, generated code, historical issue files, closed issue files, scripts, test
harnesses, screenshots, website assets, or build configuration.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/09-batch-d-direct-browser-protocol.md`
  - record this experiment design, design review, Batch D classification result,
    completion review, and conclusion;
  - classify every issue in Batch D using the Experiment 4 historical audit row
    schema.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 9 to the `## Experiments` index with status `Designed`, then
    update status after the result.

No application code, generated protobuf code, historical issue files, closed
issue files, scripts, test harnesses, screenshots, website assets, or build
configuration should be edited.

## Verification

Design-gate pass criteria:

- The issue README links this experiment as `Designed`.
- A fresh-context adversarial design review approves the plan.
- The plan commit exists before implementation begins.

Implementation pass criteria:

- The result audits every Batch D issue exactly once:
  - `0680-dark-mode`
  - `0681-quitall`
  - `0682-direct-xpc`
  - `0683-visited-links`
  - `0684-devtools`
  - `0685-multi-profile-tracking`
  - `0686-chromium-crash`
  - `0687-one-devtools`
  - `0688-devtools-split`
  - `0689-tab-lifecycle`
  - `0690-devtools-split`
  - `0691-devtools-direct-command`
  - `0692-file-subcommand`
  - `0693-smart-resolve`
  - `0694-tab-id-chromium`
  - `0695-suppress-activation-drag`
  - `0696-double-click-suppression`
  - `0697-update-docs`
  - `0698-unix-sockets`
  - `0699-protobuf-build`
  - `0700-tui-gui-sockets`
  - `0701-chromium-sockets`
  - `0702-socket-cleanup`
  - `0703-remove-click-suppression`
  - `0704-browser-bindings`
  - `0705-browser-bindings`
  - `0706-plusium-devtools`
  - `0707-roamium`
  - `0708-roamium-only`
  - `0709-wezboard`
  - `0710-gecko-webkit-ladybird`
  - `0711-rename-ghostboard-webtui`
  - `0712-engine-label`
  - `0713-rename-homepage-website`
  - `0714-seven-digit-issues`
- The result uses the Experiment 4 row schema for every classification: source
  issue, batch, subsystem, durable lesson, current Ghostboard relevance,
  evidence paths, likelihood, risk or impact, recommended follow-up, and
  historical classification note.
- The result classifies each row as `Highly likely`, `Maybe`, or `No`, and
  explains the classification from issue evidence plus current code/test/doc
  evidence.
- The result treats all Batch D issues as closed historical evidence and does
  not modify or reinterpret their closure state.
- The result distinguishes obsolete XPC-era mechanisms from current Unix
  socket/protobuf evidence.
- The result distinguishes browser-engine/Roamium findings from Ghostboard-owned
  GUI parity findings.
- The result carries forward relevant Issue 810 findings where Batch D overlaps
  current Ghostboard risk, especially direct-browser handoff, multi-profile
  routing, DevTools, tab lifecycle, cursor/input/click suppression, smart URL
  resolution, color scheme state, crash recovery, and generated protobuf
  coverage.
- The result explicitly handles duplicate historical themes, including duplicate
  DevTools split issues and duplicate browser-bindings issues, while still
  classifying every issue folder exactly once.
- The result groups or summarizes related repeated findings after the table, but
  the table itself must still contain one row per Batch D issue.
- The result identifies the next audit slice after Batch D.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/09-batch-d-direct-browser-protocol.md
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

- Any Batch D issue is omitted or classified more than once.
- The experiment edits historical issue files, application code, generated code,
  scripts, tests, screenshots, website assets, or build configuration.
- The result treats obsolete XPC-era implementation details as current
  Ghostboard requirements without mapping them to the current socket/protobuf
  architecture.
- The result treats Roamium or webtui behavior as a Ghostboard GUI bug without a
  direct current Ghostboard ownership path.
- The result treats older Ghostboard archive-era status or naming docs as proof
  of current restored-Ghostboard runtime defects.
- The result expands into other historical batches before Batch D is concluded.

## Design Review

Fresh-context adversarial design review returned **APPROVED**.

Reviewer checks confirmed:

- The issue README links Experiment 9 as `Designed`.
- The experiment has `Description`, `Changes`, and `Verification`.
- Batch D exactly matches Experiment 4: `0680`-`0714`, thirty-five issues.
- The design requires the Experiment 4 schema, obsolete XPC/current socket
  distinction, Roamium/webtui versus Ghostboard ownership distinction,
  duplicate-theme handling, pass/fail criteria, markdown formatting,
  `git diff --check`, completion review, and separate plan/result commit gates.
- Scope is audit-only and planned changes are limited to Issue 810 docs.
- `git diff --check` passed.
- The plan commit had not yet been made before review.

Findings: none.
