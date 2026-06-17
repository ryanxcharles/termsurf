# Experiment 7: Batch F Wezboard UX Regression Audit

## Description

Classify Batch F from Experiment 4: issues `0743`-`0788`. This batch covers
Wezboard-era UX and regression work: overlay positioning, split and tab
behavior, browser input, clipboard, target blank, persistent cookies, link hover
state, DevTools, native popups, split borders, PDF bootstrap, and Chromium
upgrade preparation.

This experiment should read every Batch F issue and map each durable lesson to
current Ghostboard risk using the schema defined in Experiment 4. The output is
a classification table, not fixes.

This is an audit/documentation experiment only. It must not change application
code, generated code, historical issue files, closed issue files, scripts, test
harnesses, screenshots, or website assets.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/07-batch-f-wezboard-ux-regressions.md`
  - record this experiment design, design review, Batch F classification result,
    completion review, and conclusion;
  - classify every issue in Batch F using the Experiment 4 historical audit row
    schema.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 7 to the `## Experiments` index with status `Designed`, then
    update status after the result.

No application code, generated protobuf code, historical issue files, closed
issue files, scripts, test harnesses, screenshots, or website assets should be
edited.

## Verification

Design-gate pass criteria:

- The issue README links this experiment as `Designed`.
- A fresh-context adversarial design review approves the plan.
- The plan commit exists before implementation begins.

Implementation pass criteria:

- The result audits every Batch F issue exactly once:
  - `0743-cmd-r-reload`
  - `0744-website-icon`
  - `0745-self-hosted-git`
  - `0746-overlay-positioning`
  - `0747-multiscreen-overlay`
  - `0748-clipboard`
  - `0749-initial-overlay-flash`
  - `0750-target-blank`
  - `0751-blog-top-level`
  - `0752-scroll-inactive-pane`
  - `0753-blog-failures`
  - `0754-screenshots`
  - `0755-scroll-neovim`
  - `0756-surfari`
  - `0757-overlay-fade`
  - `0758-tui-message-routing`
  - `0759-link-hover-url`
  - `0760-cli-short-flags`
  - `0761-browser-label`
  - `0762-persistent-cookies`
  - `0763-scroll-initial`
  - `0764-viewport-profile-label`
  - `0765-terminfo-crash`
  - `0766-new-logo`
  - `0767-overlay-titlebar-offset`
  - `0768-cloudflare-website`
  - `0769-tab-id-collision`
  - `0770-browser-not-loading`
  - `0771-tab-id-collision`
  - `0772-command-shortcuts`
  - `0773-loading-screen`
  - `0774-zoom-webview-overlay`
  - `0775-devtools-multi-profile`
  - `0776-pdf-not-loading`
  - `0777-split-border-overlap`
  - `0778-back-nav-title-stale`
  - `0779-date-picker-popup-position`
  - `0780-link-drag-freeze`
  - `0781-chromium-upgrade`
  - `0782-native-popup-followups`
  - `0783-native-popup-remainders`
  - `0784-datalist-popup`
  - `0785-split-border-bottom-row`
  - `0786-grid-native-split-borders`
  - `0787-split-border-outer-margin`
  - `0788-native-popup-split-pane-y`
- The result uses the Experiment 4 row schema for every classification: source
  issue, batch, subsystem, durable lesson, current Ghostboard relevance,
  evidence paths, likelihood, risk or impact, recommended follow-up, and
  historical classification note.
- The result classifies each row as `Highly likely`, `Maybe`, or `No`, and
  explains the classification from issue evidence plus current code/test/doc
  evidence.
- The result treats open Issue `0756` as open historical evidence without trying
  to close or modify it.
- The result distinguishes Wezboard-specific UI fixes from current Ghostboard
  evidence. A Wezboard bug fix is not proof that Ghostboard works, and a
  Wezboard-only problem is not automatically a Ghostboard bug.
- The result groups or summarizes related repeated findings after the table, but
  the table itself must still contain one row per Batch F issue.
- The result carries forward relevant Issue 810 findings where Batch F overlaps
  current Ghostboard risk, especially viewport geometry coverage, cursor/link
  hover behavior, DevTools, native popup behavior, browser state updates, and
  GUI-responsibility messages.
- The result identifies the next audit slice after Batch F.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/07-batch-f-wezboard-ux-regressions.md
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

- Any Batch F issue is omitted or classified more than once.
- The experiment edits historical issue files, application code, scripts, tests,
  screenshots, or website assets.
- The result treats Wezboard historical fixes as current Ghostboard proof
  without current Ghostboard evidence.
- The result labels website/blog/Git infrastructure issues as Ghostboard bugs
  without a direct current product path.
- The result expands into other historical batches before Batch F is concluded.

## Design Review

Fresh-context adversarial design review returned **APPROVED**.

Reviewer checks confirmed:

- The README links Experiment 7 as `Designed`.
- Required sections are present.
- Scope is audit-only.
- Batch F matches `0743`-`0788` exactly once.
- Verification carries the Experiment 4 schema and Issue 810 findings forward.
- Issue `0756` is treated as open evidence.
- `git diff --check` passed.
- The plan commit had not yet been made before review.

Findings: none.
