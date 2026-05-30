# Experiment 1: Code Organization Audit

## Description

This experiment audits the PDF implementation for code organization,
readability, and ease of understanding. It is diagnostic only. It must not
change runtime behavior, rename symbols, move files, delete traces, or clean up
code directly.

The goal is to produce a precise cleanup plan for Experiment 2. The audit should
identify organization issues in the PDF code created across Issues 792, 793, and
794, then classify each issue by severity, confidence, affected files, and
recommended cleanup action.

This audit must focus on maintainability, not correctness or security. Security
gets its own audit after the organization cleanup is complete. Completeness gets
its own audit after the security cleanup is complete.

This experiment must receive Codex design review before it runs. After the
result is recorded, Codex must review the completed audit before Experiment 2 is
designed.

## Scope

Audit only PDF-related implementation and test code introduced or materially
changed in the recent PDF work.

Primary Chromium scope:

- `chromium/src/content/libtermsurf_chromium/ts_browser_client.*`
- `chromium/src/content/libtermsurf_chromium/ts_content_client.*`
- `chromium/src/content/libtermsurf_chromium/ts_content_renderer_client.*`
- `chromium/src/content/libtermsurf_chromium/ts_pdf_*`
- `chromium/src/content/libtermsurf_chromium/ts_plugin_*`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_*pdf*`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_*resource*`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_*extension*`
- PDF-specific edits in Chromium PDF/printing components, including
  `pdf_view_web_plugin.cc`, `pdf_view_web_plugin_client.cc`, and
  `print_render_frame_helper.cc`.

Primary Rust and automation scope:

- `roamium/src/dispatch.rs` PDF/input paths touched for PDF work;
- Wezboard TermSurf input/resize paths touched for PDF work;
- `scripts/test-issue-794-*.py`;
- `scripts/probe-pdf-*.mjs`;
- `scripts/capture-pdf-interactions.mjs`;
- issue records for Issues 792, 793, and 794 only when needed to understand why
  code exists.

Out of scope:

- native PDF printing as a feature; Issue 795 owns that;
- behavior changes;
- security conclusions;
- completeness conclusions;
- broad refactors outside the PDF implementation;
- formatting-only churn unrelated to specific audit findings.

## Audit Method

1. Inventory the PDF implementation.

   Produce a concise map of the current PDF code by responsibility:
   - component-extension setup;
   - resource and template serving;
   - stream and MIME handling;
   - PDF viewer private/resources private APIs;
   - browser-side binders and helpers;
   - renderer-side plugin and extension setup;
   - input, resize, selection, toolbar, title, save/download, local-file, and
     print-containment paths;
   - automation harnesses.

2. Find organization issues with current evidence.

   Search for and inspect:
   - duplicated helpers or env-var parsing;
   - repeated resource lookup or URL/origin helper logic;
   - misleading file names, function names, comments, or issue-numbered log
     labels;
   - experiment-only names or traces that now read like permanent API;
   - large mixed-responsibility files or functions;
   - code that makes ownership/lifetime/call order hard to understand;
   - automation scripts that duplicate launch, fixture, or DevTools probing
     logic.

3. Categorize each finding.

   Each finding must include:
   - **Severity:** `High`, `Medium`, or `Low`.
   - **Confidence:** `High`, `Medium`, or `Low`.
   - **Files:** exact file paths and line references where practical.
   - **Problem:** what makes the code harder to understand.
   - **Why now:** why this should or should not be cleaned before later audits.
   - **Recommended cleanup:** a behavior-preserving cleanup action for
     Experiment 2.
   - **Verification needed:** the build/test/log check that would prove the
     cleanup preserved behavior.

4. Separate findings from non-findings.

   Some rough-looking code may be intentionally shaped by Chromium embedder
   constraints. Do not force cleanup if the local shape is the clearest safe
   option. Record notable non-findings where useful, especially if they prevent
   Experiment 2 from chasing cosmetic churn.

5. Produce the Experiment 2 cleanup backlog.

   The audit conclusion must list:
   - cleanup items that should be in Experiment 2;
   - cleanup items that should be deferred;
   - cleanup items that should be rejected as not worth changing;
   - the minimum verification matrix for the cleanup.

## Commands and Evidence

Use `rg` first for searches. Suggested starting points:

```bash
rg -n "issue-79[234]|TERMSURF_PDF|pdf-print|pdf-input|PdfViewer|resourcesPrivate|pdfViewerPrivate|MimeHandler|Stream|CreateInternalPlugin|PrintRenderFrameHelper" \
  chromium/src/content/libtermsurf_chromium \
  chromium/src/pdf \
  chromium/src/components/pdf \
  chromium/src/components/printing \
  roamium/src \
  wezboard/wezboard-gui/src/termsurf \
  scripts
```

```bash
rg -n "TODO|FIXME|HACK|temporary|probe|trace|issue-" \
  chromium/src/content/libtermsurf_chromium \
  chromium/src/pdf \
  chromium/src/components/pdf \
  chromium/src/components/printing \
  roamium/src \
  wezboard/wezboard-gui/src/termsurf \
  scripts
```

Also inspect the patch history in:

```bash
find chromium/patches -maxdepth 2 -path '*issue-79[234]*/*.patch' -print
```

The final audit should cite actual files and line numbers from the current
worktree, not only patch names.

## Verification

This is a documentation-only audit experiment. Verification is:

- Codex design review completed and any real design findings fixed;
- the audit result is appended to this file under `## Result`;
- findings cite current files and line references where practical;
- findings are separated into actionable cleanup, deferred cleanup, rejected
  cleanup, and non-findings;
- the conclusion defines the exact intended scope of Experiment 2;
- Codex completion review completed and any real findings fixed;
- Prettier run on this file and the issue README.

No Chromium build or Rust build is required unless the audit process changes
code, which it must not do.

## Pass Criteria

This experiment passes if it produces a clear, evidence-backed organization
audit with a concrete, behavior-preserving cleanup backlog for Experiment 2, and
Codex agrees the audit is sufficient to proceed.

## Partial Criteria

This experiment is partial if the audit identifies the right general cleanup
areas but lacks enough line-level evidence, prioritization, or verification
guidance for a safe cleanup experiment.

## Failure Criteria

This experiment fails if:

- it changes runtime behavior;
- it combines audit and cleanup;
- it drifts into security or completeness decisions beyond noting that they
  belong to later tracks;
- it proposes broad non-PDF refactors;
- it omits Codex design or completion review;
- it produces a cleanup backlog too vague to implement safely.
