# Experiment 10: Batch C Product Hardening Audit

## Description

Classify Batch C from Experiment 4: issues `0600`-`0679`. This batch covers the
late TermSurf/Ghostboard product-hardening era before the direct-browser and
socket migration work: initial Ghostboard/XPC setup, early browser overlay
demos, multi-pane and multi-profile behavior, mouse and keyboard input, URL and
search input, app/icon/rename/docs cleanup, XDG behavior, web features, alpha
readiness, URL sync, input latency, Chromium embedding, CALayerHost rendering,
navigation, persistent compositing, active pane tracking, editable URL bar,
profile server work, TUI mode/keybinding behavior, context menus, escape-key
behavior, resize/focus fixes, scripts, website, hello messages, URL
normalization, and licensing.

This experiment should read every Batch C issue and map each durable lesson to
current Ghostboard risk using the schema defined in Experiment 4. The output is
a classification table, not fixes.

This is an audit/documentation experiment only. It must not change application
code, generated code, historical issue files, closed issue files, scripts, test
harnesses, screenshots, website assets, or build configuration.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/10-batch-c-product-hardening.md`
  - record this experiment design, design review, Batch C classification result,
    completion review, and conclusion;
  - classify every issue in Batch C using the Experiment 4 historical audit row
    schema.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 10 to the `## Experiments` index with status `Designed`, then
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

- The result audits every Batch C issue exactly once:
  - `0600-termsurf-ghost`
  - `0601-zig-xpc`
  - `0602-pink-texture`
  - `0603-box-demo`
  - `0604-two-panes`
  - `0605-two-profiles`
  - `0606-mouse-input`
  - `0607-keyboard-input`
  - `0608-search-input`
  - `0609-keyboard-input-2`
  - `0610-app-icon`
  - `0611-rename`
  - `0612-icon`
  - `0613-rename-directories`
  - `0614-docs-review`
  - `0615-xdg`
  - `0616-web-features`
  - `0617-alpha`
  - `0618-url-sync`
  - `0619-input-latency`
  - `0620-zig-content-shell`
  - `0621-single-process`
  - `0622-javascript-is-slow`
  - `0623-viz-display-serialization`
  - `0624-chromium-ipc`
  - `0625-calayerhost`
  - `0626-x-y-calayerhost`
  - `0627-resize-calayerhost`
  - `0628-navigation-calayerhost`
  - `0629-understand-nav-calayerhost`
  - `0630-nav-calayerhost-6`
  - `0631-continue-nav-calayerhost`
  - `0632-nav-flicker-calayerhost`
  - `0633-persistent-compositor`
  - `0634-calayerhost-audit`
  - `0635-multi-pane-calayerhost`
  - `0636-calayerhost-audit`
  - `0637-editable-url-bar`
  - `0638-page-title`
  - `0639-open-in-same-tab`
  - `0640-project-cleanup`
  - `0641-chromium-patches`
  - `0642-zig-profile-server`
  - `0643-zig-profile-server-2`
  - `0644-simplified-cpp`
  - `0645-audit-xdg`
  - `0646-normal-insert`
  - `0647-tui-restructure`
  - `0648-devtools-research`
  - `0649-control-mode`
  - `0650-installation`
  - `0651-bundle-identifier`
  - `0652-termsurf-cli`
  - `0653-xpc-gateway`
  - `0654-cmd-h`
  - `0655-substack-blank`
  - `0656-rename-script`
  - `0657-url-edit-color`
  - `0658-edtui-improvements`
  - `0659-command-mode`
  - `0660-lazyvim-tokyonight-colors`
  - `0661-title-spacing`
  - `0662-context-menu`
  - `0663-js-context-menu`
  - `0664-clap`
  - `0665-esc`
  - `0666-devils-esc`
  - `0667-active-pane`
  - `0668-fix-resize`
  - `0669-active-pane`
  - `0670-click-to-focus`
  - `0671-app-icon`
  - `0672-border-padding`
  - `0673-consolidate-scripts`
  - `0674-homepage`
  - `0675-hello-message`
  - `0676-url-normalization`
  - `0677-website-deps`
  - `0678-website-lint-format`
  - `0679-license`
- The result uses the Experiment 4 row schema for every classification: source
  issue, batch, subsystem, durable lesson, current Ghostboard relevance,
  evidence paths, likelihood, risk or impact, recommended follow-up, and
  historical classification note.
- The result classifies each row as `Highly likely`, `Maybe`, or `No`, and
  explains the classification from issue evidence plus current code/test/doc
  evidence.
- The result treats all Batch C issues as closed historical evidence and does
  not modify or reinterpret their closure state.
- The result distinguishes obsolete XPC and prototype-era implementation
  mechanisms from current socket/protobuf, CALayerHost, and restored Ghostboard
  evidence.
- The result distinguishes Ghostboard GUI-owned parity findings from Roamium,
  Chromium, webtui, website, packaging, and docs-only findings.
- The result carries forward relevant Issue 810 findings where Batch C overlaps
  current Ghostboard risk, especially keyboard/mouse input, URL synchronization,
  active-pane/focus state, multi-pane and multi-profile routing, overlay
  geometry, resize behavior, context menus, mode/keybinding behavior, app
  activation, installation, and named/default browser startup.
- The result explicitly handles duplicate or recurring themes, including
  keyboard input, active pane tracking, CALayerHost audits, app icons, rename
  cleanup, and mode/keybinding work, while still classifying every issue folder
  exactly once.
- The result groups or summarizes related repeated findings after the table, but
  the table itself must still contain one row per Batch C issue.
- The result identifies the next audit slice after Batch C.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/10-batch-c-product-hardening.md
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

- Any Batch C issue is omitted or classified more than once.
- The experiment edits historical issue files, application code, generated code,
  scripts, tests, screenshots, website assets, or build configuration.
- The result treats obsolete XPC-era implementation details as current
  Ghostboard requirements without mapping them to the current socket/protobuf
  architecture.
- The result treats Roamium, Chromium, webtui, website, packaging, or docs-only
  behavior as a Ghostboard GUI bug without a direct current Ghostboard ownership
  path.
- The result treats older Ghostboard prototype status, rename history, or
  archived docs as proof of current restored Ghostboard behavior.

## Design Review

Mencius reviewed the design and approved it with no required findings.

The review verified that the plan is audit-only, linked from the Issue 810
README as `Designed`, covers the exact eighty-issue Batch C inventory with no
omissions or duplicates, requires the Experiment 4 row schema, has concrete
pass/fail criteria, preserves closed historical issue immutability, and requires
distinguishing obsolete XPC/prototype work and non-Ghostboard-owned areas from
current Ghostboard GUI parity risk.
