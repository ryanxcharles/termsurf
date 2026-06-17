# Experiment 3: Direct Browser Paths

## Description

Experiment 2 corrected an important architectural assumption: current `webtui`
connects directly to Roamium after `BrowserReady`, so missing Ghostboard
dispatcher cases do not automatically prove browser chrome, dialog/auth,
console, crash, or color-scheme gaps.

This experiment audits those direct-browser paths and the remaining compositor
fallback paths. It should turn the Experiment 2 `Maybe` findings into a clearer
ranked list:

- which behavior has convincing direct-browser runtime evidence;
- which behavior still lacks Ghostboard-specific regression evidence;
- which behavior depends on a compositor fallback that Ghostboard appears not to
  implement;
- which findings should be deferred to the historical issue audit rather than
  treated as protocol gaps.

This is an audit/documentation experiment only. It must not change application
code or test harnesses.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/03-direct-browser-paths.md`
  - record this experiment design, design review, result, completion review, and
    conclusion;
  - record direct-browser and fallback path evidence for each Experiment 2
    `Maybe` finding.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 3 to the `## Experiments` index with status `Designed`, then
    update status after the result.

No application code, generated protobuf code, historical issue files, closed
issue files, scripts, or test harnesses should be edited.

## Verification

Design-gate pass criteria:

- The issue README links this experiment as `Designed`.
- A fresh-context adversarial design review approves the plan.
- The plan commit exists before implementation begins.

Implementation pass criteria:

- The result covers every `Maybe` finding from Experiment 2:
  - browser chrome/status over `BrowserConnection` and compositor fallback,
    including URL, loading state, title, hover target, and console capture;
  - JavaScript dialog and HTTP auth over `BrowserConnection` and compositor
    fallback;
  - renderer crash over `BrowserConnection` and compositor fallback;
  - color scheme initial state, direct runtime state, and compositor fallback;
  - cursor shape updates and `SetGuiActive`.
- For each item, the result records:
  - source protocol messages;
  - direct-browser path evidence, or evidence that no direct path exists;
  - compositor fallback evidence, or evidence that no fallback is required;
  - Ghostboard-specific evidence from Issue 809 or current code where it exists;
  - likelihood: `Highly likely`, `Maybe`, or `No`;
  - risk or impact;
  - recommended follow-up.
- The audit must distinguish:
  - direct Roamium socket evidence from Ghostboard compositor evidence;
  - static code-path evidence from end-to-end runtime/test evidence;
  - normal post-`BrowserReady` behavior from fallback/pre-ready behavior.
- `No` is allowed only when there is concrete implementation evidence and the
  path does not require untested Ghostboard behavior, or when existing Issue 809
  evidence already proves the relevant runtime path.
- `Highly likely` is allowed only when the required normal path appears absent
  or disconnected. A missing optional fallback path should not be labeled
  `Highly likely` unless the audit shows that normal usage depends on it.
- The result identifies the next audit slice. Expected next slice: begin the
  historical issue audit if the direct-browser audit reduces the protocol
  findings to bounded follow-ups; otherwise, design one focused audit for the
  highest-risk remaining protocol path.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/03-direct-browser-paths.md
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

- Any Experiment 2 `Maybe` finding is omitted.
- The audit treats direct-browser code existence as equivalent to
  Ghostboard-specific end-to-end proof.
- The audit labels fallback omissions as `Highly likely` without showing normal
  usage depends on the fallback.
- The audit edits application code, generated code, scripts, harnesses, or
  closed historical issues.
- The result makes broad parity claims without file references and evidence.

## Design Review

Fresh-context adversarial design review returned **APPROVED**.

Optional finding:

- Browser chrome/status likely includes console, but Experiment 2 explicitly
  named console capture. The reviewer suggested spelling out
  URL/loading/title/target/console in the pass criteria.

Fix:

- Updated the implementation pass criteria to explicitly include URL, loading
  state, title, hover target, and console capture.
