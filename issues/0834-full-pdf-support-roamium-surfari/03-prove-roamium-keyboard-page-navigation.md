# Experiment 3: Prove Roamium Keyboard and Page Navigation

## Description

Experiment 2 refreshed the current Roamium baseline for rendering, mouse, scroll
wheel, resize, selection/copy, toolbar fit/zoom/rotate, save/download, title,
local parity, contained print, security, and one non-PDF smoke. It still left
two closely related Roamium workflow rows unproven in the current tree:

- keyboard page/scroll navigation;
- toolbar page selector/page navigation.

Issue 794 historically proved pieces of this behavior, but the current
Experiment 2 probes do not directly assert it. This experiment adds current
automation for those two rows before any product-code changes. The experiment
passes only if Roamium can navigate within the PDF by TermSurf protocol keyboard
input and by the PDF toolbar's page selector.

## Changes

1. Add a narrow Roamium PDF navigation probe.

   Create `scripts/test-issue-834-pdf-navigation.py` as a harness that reuses
   the existing TermSurf socket/protobuf patterns from the Issue 794 harnesses:

   - launch `chromium/src/out/Default/roamium` with trace/log output under a
     caller-provided `--log-dir`;
   - serve `test-html/public/bitcoin.pdf` through a deterministic local HTTP
     server when `--serve-bitcoin-pdf` is passed;
   - create a Roamium tab through the TermSurf protocol;
   - resize the tab to a stable viewport;
   - discover the DevTools port;
   - attach to the PDF extension child target;
   - capture before/after screenshots and viewer state;
   - write one summary JSON file at `<log-dir>/pdf-navigation-summary.json`.

   The harness should support at least two probes:

   - `--probe keyboard-page-scroll`: focus the PDF plugin through TermSurf
     protocol input, send user-level TermSurf key events such as PageDown,
     Space, ArrowDown, or PageUp, and pass only when viewer state or screenshot
     evidence changes in a way consistent with page/scroll navigation.
   - `--probe toolbar-page-selector`: use DevTools against the PDF extension
     child target to set `#pageSelector` from page `1` to a later page, dispatch
     the same events a user edit would trigger, and pass only when page-selector
     state, viewer/page state, or screenshot evidence changes to the requested
     page.

2. Prefer extending existing helpers over duplicating large blocks.

   If the implementation can keep the new harness small by sharing helper code
   with existing scripts, do that. Do not refactor unrelated harnesses unless it
   is required to avoid unsafe copy/paste or to expose a small reusable helper.

3. Run the new probes.

   Use fresh log directories:

   ```bash
   python3 scripts/test-issue-834-pdf-navigation.py \
     --log-dir logs/issue-834-exp3-keyboard-page-scroll \
     --serve-bitcoin-pdf \
     --probe keyboard-page-scroll
   python3 scripts/test-issue-834-pdf-navigation.py \
     --log-dir logs/issue-834-exp3-toolbar-page-selector \
     --serve-bitcoin-pdf \
     --probe toolbar-page-selector
   ```

4. If a probe fails, stop and record the first failing layer.

   Do not continue into links, find/search, restrictions, password PDFs,
   malformed PDFs, or Surfari work in this experiment. If keyboard navigation or
   page selector navigation fails, record whether the failure is in protocol
   delivery, focus, Chromium routing, PDF viewer state, DevTools automation, or
   evidence collection.

## Verification

Verification for the completed result is:

- the new harness has concrete pass/fail logic, not only exit-code success;
- the keyboard probe sends TermSurf protocol key events, not only DevTools
  `Input.dispatchKeyEvent`;
- the keyboard probe records protocol key count, focus evidence, Roamium key
  trace evidence, Chromium/PDF routing evidence when available, before/after
  screenshot hashes, and the page/scroll state delta used for pass/fail;
- the toolbar page-selector probe records the selector's before/after value, the
  requested target page, screenshot hashes, and viewer/page state evidence;
- both probes write summary JSON files under `logs/issue-834-exp3-*`;
- the experiment result cites command, exit status, summary file, summary
  status, first failing hop, and matrix rows proven or not proven;
- no product code is changed unless a probe exposes a real TermSurf integration
  bug and that fix is explicitly documented in this experiment;
- no Chromium source is changed unless a fresh Chromium branch and patch archive
  are created according to `chromium/AGENTS.md`;
- design review is recorded and the plan commit exists before implementation
  begins;
- markdown is formatted with Prettier;
- `git diff --check` passes;
- completion review is recorded before the result commit.

## Design Review

Fresh-context adversarial review by Codex subagent `Franklin`: **Changes
required**, then **Approved** after fixes.

Required finding:

- The initial design said the harness would write one summary at a fixed
  `logs/issue-834-exp3-navigation/pdf-navigation-summary.json` path, while the
  probe commands used two distinct `--log-dir` values.

Optional finding:

- The verification checklist could explicitly mention the plan commit gate.

Fixes:

- Changed the summary location to `<log-dir>/pdf-navigation-summary.json`.
- Added a verification item requiring design review to be recorded and the plan
  commit to exist before implementation begins.

Re-review verdict: **Approved**. The reviewer confirmed that the required
finding was resolved and that no new required findings were introduced.

## Pass Criteria

This experiment passes if both new probes pass and provide current evidence that
Roamium supports PDF keyboard page/scroll navigation through TermSurf protocol
input and PDF toolbar page-selector navigation.

## Partial Criteria

This experiment is partial if one navigation path is proven and the other fails
or cannot be automated with a concrete first failing layer.

## Failure Criteria

This experiment fails if neither navigation path can be proven, if the harness
claims success without state/screenshot evidence, or if it bypasses the TermSurf
keyboard path for the keyboard navigation row.
