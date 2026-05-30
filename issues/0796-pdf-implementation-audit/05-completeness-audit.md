# Experiment 5: Completeness Audit

## Description

This experiment audits whether TermSurf's non-print PDF viewer is complete
enough after Issues 792-794 and the Issue 796 security cleanup. It is diagnostic
only. It must not change Chromium, Rust, JavaScript, Python, protocol, fixtures,
or runtime behavior.

The audit starts from Chromium branch `148.0.7778.97-issue-796-exp4`. The goal
is to produce a concrete Experiment 6 cleanup plan that either implements the
remaining non-print gaps or explicitly documents them as follow-up issues.

Native PDF printing is out of scope. Issue 795 owns native PDF printing. This
audit may mention print only to confirm it remains intentionally deferred and
does not block the non-print PDF viewer scope.

This experiment must receive Codex design review before it runs. After the audit
result is recorded, Codex must review the completed audit before Experiment 6 is
designed.

## Scope

Audit only the PDF implementation and test coverage created or materially
changed by Issues 792, 793, 794, and 796.

Primary Chromium scope:

- `chromium/src/content/libtermsurf_chromium/` PDF viewer, extension,
  MimeHandler, stream, resource, title, toolbar, print-containment, and
  input/resize helper code;
- TermSurf Chromium patches under `chromium/patches/issue-792/`,
  `chromium/patches/issue-793/`, `chromium/patches/issue-794-*`, and
  `chromium/patches/issue-796-exp*`;
- any PDF-specific patches under `chromium/src/pdf/` and
  `chromium/src/components/printing/`.

Primary Rust, JavaScript, and automation scope:

- Roamium PDF/input/resize dispatch paths touched for PDF behavior;
- Wezboard PDF input/resize routing touched for PDF behavior;
- `scripts/test-issue-794-*.py`;
- `scripts/termsurf_pdf_protocol_harness.py`;
- `scripts/probe-pdf-*.mjs`;
- `scripts/capture-pdf-interactions.mjs`;
- `scripts/test-issue-796-pdf-security.py`;
- logs from the latest passing Issue 794 and Issue 796 PDF runs where needed.

Out of scope:

- native PDF printing implementation;
- unrelated browser features;
- broad upstream Chromium/PDFium feature parity outside the TermSurf embedder
  integration;
- large accessibility work unless TermSurf specifically broke or omitted a
  required PDF viewer integration point;
- changing code during this audit.

## Audit Method

### 1. Build a feature inventory

Create a table of expected non-print PDF viewer behavior. At minimum include:

- full-page PDF rendering;
- embedded PDF rendering;
- HTTP and HTTPS PDFs;
- `file://` PDFs;
- extensionless local PDFs;
- extensionless HTTP PDFs;
- titled and untitled PDFs;
- restricted PDFs;
- PDF permission-restricted documents;
- copy restrictions;
- save/download restrictions;
- disabled toolbar states for document restrictions;
- scroll wheel;
- keyboard scroll/navigation;
- mouse click and focus;
- text selection;
- copy;
- resize and reflow;
- toolbar visibility;
- toolbar page navigation;
- zoom in/out;
- fit-to-page / fit-to-width;
- rotate;
- save/download;
- title propagation;
- internal PDF links and external links;
- find/search within PDF;
- forms;
- annotations;
- password-protected or error-page PDFs;
- context menu behavior;
- accessibility/searchify behavior if applicable;
- normal HTML regression behavior.

For each item, classify the current status as one of:

- `Implemented and automated`;
- `Implemented but manual only`;
- `Implemented but weakly automated`;
- `Missing`;
- `Intentionally out of scope`;
- `Unknown`.

Each row must cite evidence: issue result, test script, log file, code path, or
local source comparison. If evidence is weak or old, mark it weak instead of
treating it as proof.

### 2. Compare against Chrome and Electron where useful

Use local source references to compare TermSurf's PDF behavior against Chrome
and Electron for missing or uncertain features. Focus on the embedder
integration points TermSurf owns, not PDFium internals.

Reference areas include:

- PDF viewer extension UI/resources;
- stream handoff and MimeHandler behavior;
- toolbar actions;
- save/download;
- title propagation;
- find/search;
- link handling;
- forms and annotations;
- accessibility/searchify setup;
- context menu hooks;
- error-page or password handling.

The audit does not need exact Chrome parity for every feature. It must identify
whether a missing feature is:

- required for a normal usable PDF viewer;
- nice-to-have but acceptable as follow-up;
- already provided by upstream PDF viewer code once TermSurf wiring is correct;
- blocked by a TermSurf embedder gap.

### 3. Audit automation coverage

Map every non-print feature to existing automation.

Questions:

- Which features are covered by `scripts/test-issue-794-pdf-toolbar.py`?
- Which features are covered by protocol scroll/resize/mouse harnesses?
- Which features are only covered by old one-off probes?
- Which features are manually tested but automatable?
- Which existing harnesses produce partial or ambiguous summaries?
- Which assertions are too broad, too weak, or too dependent on screenshots?

The audit must specifically address the known `save-print-title-local` harness
nuance from Experiment 4: it exited successfully and verified save/title/print,
but its local-parity DevTools wheel subcheck reported `partial` while the
dedicated protocol scroll harness passed. Decide whether Experiment 6 should fix
that harness, document it, or replace the local-parity subcheck with a better
automated assertion.

### 4. Audit user-facing completeness gaps

For each missing or weak feature, determine the user impact:

- Does the user lose core PDF viewing ability?
- Does the user lose common document workflow behavior?
- Is the behavior browser-standard but rarely needed?
- Is it blocked by native UI, OS permissions, or cross-process architecture?
- Can it be solved in the TermSurf embedder layer without large new
  infrastructure?

Native print must be classified as `Intentionally out of scope` and linked to
Issue 795, not pulled into Experiment 6.

### 5. Produce the Experiment 6 cleanup backlog

The conclusion must split the audit output into:

- required fixes for Experiment 6;
- automation cleanup required for Experiment 6;
- acceptable follow-up issues outside this issue;
- intentional non-print scope exclusions;
- non-findings where current behavior is sufficient.

If the audit finds no missing required non-print behavior, Experiment 6 should
still be designed to add the smallest useful documentation or automation cleanup
that makes the completeness boundary easier to maintain.

## Commands and Evidence

Use `rg` first for source searches. Suggested starting points:

```bash
rg -n "pdf|PDF|viewer-toolbar|viewer-page-selector|zoom|fit|rotate|find|search|annotation|form|password|download|save|copy|selection|link|context|accessibility|searchify|print" \
  chromium/src/content/libtermsurf_chromium \
  chromium/src/pdf \
  chromium/src/chrome/browser/resources/pdf \
  chromium/src/components/pdf \
  roamium/src \
  wezboard/wezboard-gui/src/termsurf \
  scripts
```

```bash
rg -n "status|first_failing_hop|localParity|titlePropagation|saveDownload|print|toolbar|scroll|resize|selection|copy|link|find|form|annotation" \
  scripts/test-issue-794-*.py \
  scripts/probe-pdf-*.mjs \
  scripts/capture-pdf-interactions.mjs \
  scripts/termsurf_pdf_protocol_harness.py
```

Suggested local reference searches:

```bash
rg -n "pdfViewerPrivate|PDFViewer|viewer-toolbar|viewer-page-selector|find|search|annotation|form|password|download|save|copy|selection|contextMenu|accessibility|searchify" \
  vendor/electron \
  chromium/src/chrome/browser/resources/pdf \
  chromium/src/chrome/browser/pdf \
  chromium/src/components/pdf \
  chromium/src/pdf
```

If a local Electron checkout is unavailable, note that in the result and rely on
the local Chromium source plus existing issue records.

Useful current logs:

- `logs/issue-796-exp4-security-rerun/`
- `logs/issue-796-exp4-save-title-local-rerun/`
- `logs/issue-796-exp4-save-title-local-print-intercept/`
- `logs/issue-796-exp4-protocol-scroll/`
- `logs/issue-796-exp4-protocol-resize/`
- `logs/issue-796-exp4-protocol-mouse-click/`
- `logs/issue-796-exp4-protocol-mouse-select-copy/`
- `logs/issue-796-exp4-non-pdf-html/`

Do not treat these logs as proof for features they do not assert. Use them only
where the script summaries and captured artifacts directly cover the feature.

## Verification

This is a documentation-only audit experiment. Verification is:

- Codex design review completed and real design findings fixed;
- no runtime code changed;
- the audit result is appended to this file under `## Result`;
- the feature inventory table is present;
- every feature status cites evidence or explicitly says evidence is missing;
- automation gaps are separated from product/feature gaps;
- native print is left to Issue 795 and not re-scoped;
- the Experiment 6 cleanup backlog is concrete enough to implement;
- Codex completion review completed and real findings fixed;
- Prettier run on this file and the issue README.

No Chromium, Rust, Roamium, or Wezboard build is required unless the audit
accidentally changes code. It must not change code.

## Pass Criteria

This experiment passes if it produces an evidence-backed completeness audit that
identifies the actual non-print PDF cleanup backlog for Experiment 6, or proves
that no required non-print feature work remains and defines the minimum
documentation/automation cleanup needed to preserve that conclusion.

## Partial Criteria

This experiment is partial if it identifies likely completeness gaps but lacks
enough evidence, user-impact classification, or verification guidance to safely
design Experiment 6.

## Failure Criteria

This experiment fails if:

- it changes runtime behavior;
- it combines audit and cleanup;
- it treats native PDF printing as in scope;
- it claims completeness without checking rendering, input, toolbar, save,
  title, local-file, embedded, and normal-HTML behavior;
- it treats old issue notes as proof without checking whether current code and
  current harnesses still support the claim;
- it omits Codex design or completion review;
- it produces a cleanup backlog too vague to implement safely.
