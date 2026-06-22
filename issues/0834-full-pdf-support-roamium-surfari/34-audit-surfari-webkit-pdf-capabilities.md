# Experiment 34: Audit Surfari and WebKit PDF Capabilities

## Description

Experiment 33 added durable Roamium regression coverage for the advanced PDF
rows. The issue can now move into the Surfari/WebKit phase, but Surfari PDF work
should start with a source and capability audit rather than immediate product
changes.

WebKit's macOS PDF path is structurally different from Chromium's extension PDF
viewer. Surfari also uses `WKWebView` through `libtermsurf_webkit`, not
Chromium's plugin/extension stack. Before building Surfari-specific probes, we
need to identify:

- which PDF workflows WebKit likely supports natively;
- which workflows require TermSurf/Surfari protocol integration work;
- which workflows need WebKit source hooks, private API, or custom fixtures;
- which Roamium assumptions do not transfer to WebKit.

This experiment is an audit only. Do not modify Surfari, WebKit, Ghostboard,
Roamium, protobuf, or test harness code.

## Changes

- Update only this experiment file with the audit result.
- Use the local WebKit checkout in `webkit/src/` and the local Surfari code in
  `surfari/`.
- Inspect at least:
  - `webkit/AGENTS.md` and `webkit/README.md`;
  - WebKit PDF layout tests under `webkit/src/LayoutTests/pdf/`;
  - WebKit API tests or expectations mentioning `UnifiedPDF`, `PDF`, or
    `WKWebView` PDF behavior;
  - WebKit source paths that implement or expose macOS PDF viewing, printing,
    selection, annotation, or plugin behavior;
  - `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`;
  - `surfari/src/dispatch.rs`;
  - existing Surfari/Issue 756 harnesses that prove `web --browser surfari`
    behavior.
- Produce a concise Surfari/WebKit PDF capability matrix covering the Issue 834
  feature rows:
  - likely native WebKit support;
  - Surfari integration status;
  - automation approach;
  - first probe needed;
  - risks or known limitations.
- Identify the next implementation experiment. The likely next step is a basic
  Surfari PDF load/render probe in the real TermSurf app, but the audit should
  confirm or correct that.
- Do not deepen `webkit/src` or create a WebKit issue branch unless the audit
  proves local history is required. If that happens, stop and design a follow-up
  experiment first.

## Verification

Run read-only audit commands and record the important output:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --is-shallow-repository

rg -n "UnifiedPDF|PDFPlugin|PDFDocument|pdf|PDF" \
  webkit/src/Source webkit/src/LayoutTests webkit/src/Tools \
  -g '!webkit/src/WebKitBuild/**'

rg -n "pdf|PDF|WKWebView|WKWebsiteDataStore|context|print|select|mouse|key" \
  surfari scripts issues/0756-surfari \
  -g '!target/**'
```

If the raw search output is too large, narrow it to the files that actually
drive the audit and record those file paths in the result.

Pass criteria:

- no product source code is changed;
- no WebKit source is changed;
- the audit identifies the relevant WebKit PDF mechanisms and local Surfari
  integration points;
- the audit maps every Issue 834 PDF feature row into a Surfari/WebKit status
  bucket, even if the bucket is `unknown-needs-probe`;
- the audit clearly distinguishes native WebKit capability from TermSurf/Surfari
  integration work;
- the audit names the next experiment and why it is the right next step;
- markdown is formatted with Prettier;
- design review and completion review are recorded.

Partial criteria:

- the audit finds the main WebKit PDF path but cannot classify several advanced
  rows without running a real Surfari probe.

Failure criteria:

- product or WebKit source is changed;
- the audit assumes Chromium PDF architecture applies to WebKit without source
  evidence;
- the audit omits major Issue 834 rows such as rendering, input, links, search,
  restrictions, print, forms, annotations, context menus, accessibility, or
  geometry;
- the next experiment is vague or not tied to audit evidence.

## Design Review

An external Codex review checked the design.

Verdict: **Approved**.

The review found no findings. It confirmed that this is the correct next step
after the Roamium advanced guard, that the scope is bounded to local
WebKit/Surfari source evidence and Issue 834 PDF rows, and that the verification
criteria require a capability matrix, native-vs-TermSurf distinction, and a
specific next experiment.
