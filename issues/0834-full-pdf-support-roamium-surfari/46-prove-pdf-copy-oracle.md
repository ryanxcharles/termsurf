# Experiment 46: Prove PDF Copy Oracles

## Description

Experiment 45 showed that the generated PDF fixture is extractable and that
standalone text, PDFKit, and WKWebView PDF controls can visibly select the
marker. However, automated `Cmd+C` did not update the pasteboard in those
standalone controls, so Surfari's PDF copy behavior still cannot be separated
from the test harness copy path.

This experiment should establish a trustworthy copy oracle before any Surfari
product-code change. The goal is to identify which copy invocation routes
actually copy selected text from standalone macOS controls, then apply only the
proven route or routes to the real Surfari-in-Ghostboard path.

## Changes

- Add a focused diagnostic harness, tentatively
  `scripts/test-issue-834-pdf-copy-oracles.sh`.
- Reuse the same deterministic marker and PDF fixture from Experiments 44 and
  45:
  - marker: `TS834PDFCOPYQXJZ`;
  - accepted substring: `TS834PDFCOPYQXJZ`;
  - real PDF text operator with explicit font encoding.
- Reuse or adapt the standalone probe app from Experiment 45, but run each
  copy-route/control pair in isolation so pasteboard state and focus do not leak
  between attempts.
- For each standalone control:
  - `NSTextView` with the marker as a known-good selectable text control;
  - PDFKit `PDFView` loading the generated PDF;
  - `WKWebView` loading the generated PDF.
- For each control, first create visible selection using the corrected
  Experiment 45 drag ratios and record screenshots proving the selection exists.
- Then test copy invocation routes separately:
  - CGEvent `Cmd+C` using `scripts/ghostty-app/inject.swift` as the baseline
    route from Experiment 45;
  - System Events `keystroke "c" using command down`;
  - Accessibility/menu route, such as invoking the frontmost app's Edit > Copy
    menu item when available;
  - an in-process AppKit action route inside the temporary probe app, such as
    `NSApp.sendAction(#selector(NSText.copy(_:)), to: nil, from: nil)`, exposed
    through a temporary trigger file or local control socket.
- Record for every attempt:
  - control name;
  - copy route;
  - process/app path;
  - window bounds and drag coordinates;
  - screenshots after selection and after copy;
  - pasteboard change counts before sentinel, after sentinel, after copy, and
    after restore;
  - clipboard length, hash, bounded sample, and whether it contains the marker;
  - whether the selection screenshot visibly shows the marker selected.
- Protect the clipboard across the multi-route run:
  - save the original clipboard exactly once at harness start;
  - restore it from a trap on every exit path;
  - use a distinct sentinel for every control/route attempt;
  - record final restoration status in the summary;
  - report `fail` if restoration fails and `partial` if required probe evidence
    is missing.
- Apply this outcome matrix:
  - **copy-oracle-found:** one specific external route copies the marker from
    all three standalone controls: `NSTextView`, PDFKit `PDFView`, and
    standalone `WKWebView` PDF;
  - **appkit-only-copy-oracle:** the in-process AppKit route copies the marker
    from all three standalone controls, but no single external route does, so
    selected text is copyable but external automation remains untrusted;
  - **pdfkit-copy-gap:** at least one external route copies from `NSTextView`,
    but no external route and no in-process AppKit route copies from PDFKit
    `PDFView`;
  - **webkit-pdf-copy-gap:** at least one route copies from `NSTextView` and
    PDFKit `PDFView`, but no external route and no in-process AppKit route
    copies from standalone `WKWebView` PDF;
  - **automation-copy-gap:** selection is visible but no external copy route
    copies from the known-good `NSTextView`;
  - **mixed-copy-oracle:** multiple routes prove some controls copy, but no
    single external route works across all standalone controls and the failures
    do not fit `pdfkit-copy-gap`, `webkit-pdf-copy-gap`, or
    `appkit-only-copy-oracle`;
  - **selection-gap:** the harness cannot reliably create visible selection.
- Map outcome classes to result status:
  - **Pass:** `copy-oracle-found`, `pdfkit-copy-gap`, `webkit-pdf-copy-gap`, or
    `automation-copy-gap`, when the required standalone evidence is complete and
    clipboard restoration succeeds;
  - **Partial:** `appkit-only-copy-oracle` or `mixed-copy-oracle`, because those
    outcomes are useful but do not produce a trusted Surfari-applicable external
    route;
  - **Fail:** `selection-gap`, clipboard restore failure, missing required probe
    evidence, or any product-code change before the copy oracle is established.
- If `copy-oracle-found` is reached, apply the best proven external route to the
  existing Surfari-in-Ghostboard PDF fixture:
  - run the current Browse-mode `Cmd+C` route;
  - run the same external route that succeeded across all standalone controls,
    where applicable;
  - keep the Surfari result separate from standalone controls.
- Do not modify Ghostboard, Surfari, WebKit, protocol, or product code in this
  experiment unless the diagnostic harness itself has an obvious bug.
- Update this experiment file with the result.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-pdf-copy-oracles.sh
git diff --check
git -C webkit/src status --short
```

Run the diagnostic harness:

```bash
rm -rf logs/issue-834-exp46-pdf-copy-oracles
scripts/test-issue-834-pdf-copy-oracles.sh
```

Pass criteria:

- clipboard safety is preserved across the whole multi-route run;
- each control/route attempt starts from a fresh sentinel and records clipboard
  evidence independently;
- standalone `NSTextView`, PDFKit `PDFView`, and standalone `WKWebView` PDF are
  tested separately;
- screenshots prove selection exists before interpreting copy failures;
- the result identifies one of the explicit Pass-class outcome classes above, or
  documents why the outcome is Partial/Fail according to the result-status
  mapping;
- if a trusted external copy route is found, Surfari is tested with that route
  and its result is recorded separately;
- no product code is changed;
- completion review is recorded.

Partial criteria:

- an in-process AppKit route proves selected text is copyable, but no external
  automation route is trustworthy yet;
- some standalone controls produce useful route evidence, but a macOS permission
  or windowing limitation prevents the full matrix;
- Surfari cannot be retested, but the standalone copy-route diagnosis is
  conclusive enough to guide the next experiment.

Failure criteria:

- the harness cannot create visible selection in the known-good text control;
- clipboard state is mutated without being restored;
- the result claims a Surfari-layer copy bug without a proven standalone copy
  oracle;
- product code is changed before the copy oracle is established.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Findings:

- `copy-oracle-found` was ambiguous about whether the same external route had to
  work across all standalone controls before being trusted for Surfari;
- the outcome matrix did not define mixed route/control outcomes;
- clipboard safety needed to be spelled out as explicitly as Experiment 45.

Resolution:

- required one specific external route to copy from `NSTextView`, PDFKit
  `PDFView`, and standalone `WKWebView` PDF before classifying
  `copy-oracle-found`;
- added route-specific gap classes and a `mixed-copy-oracle` class for useful
  but non-Surfari-applicable mixed results;
- added an explicit outcome-class-to-result-status mapping so Pass, Partial, and
  Fail cannot be inferred inconsistently;
- added explicit clipboard save-once, trap restore, per-attempt sentinel, final
  restoration status, and summary downgrade/failure requirements.

Follow-up verdict: **Approved**.

The reviewer found no remaining must-fix design issues and approved the
Experiment 46 plan commit.
