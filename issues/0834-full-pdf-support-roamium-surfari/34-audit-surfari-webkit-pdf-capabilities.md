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

## Result

**Result:** Pass

Audited the local WebKit and Surfari source trees without changing product code
or WebKit source.

Workspace state:

- main repo status before the audit was clean;
- `webkit/src` status was clean;
- WebKit branch: `webkit-1452a439-issue-756-exp12`;
- WebKit HEAD: `cdfb8cbf86f7c5e52cef0b2f14e8ab30ceeea91c`;
- `webkit/src` is still shallow: `true`.

Audit commands:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --is-shallow-repository

rg -l "UnifiedPDF|PDFPlugin|PDFDocument|PDFKit|pdf" \
  webkit/src/Source webkit/src/LayoutTests webkit/src/Tools \
  webkit/src/TestExpectations \
  -g '!webkit/src/WebKitBuild/**'

rg -n "UnifiedPDF|PDFPlugin|PDFDocument|PDFKit|WKPDF|PDFHUD|PDFAnnotation|PDFSelection|context menu|print" \
  webkit/src/Source/WebKit webkit/src/Source/WebCore \
  webkit/src/Tools/TestWebKitAPI webkit/src/TestExpectations \
  -g '!webkit/src/WebKitBuild/**'

rg -n "ts_forward_mouse|ts_forward_scroll|ts_forward_key|set_view_size|CAContext|remoteContext|contextId|loadURL|load_url|target_url|title|loading|focus|print|evaluateJavaScript|hitTest|context" \
  surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm \
  surfari/src/dispatch.rs \
  surfari/src/ffi.rs
```

## Source Findings

WebKit has a first-party macOS PDF plugin implementation. The relevant local
paths are:

- `webkit/src/Source/WebKit/WebProcess/Plugins/PDF/`;
- `webkit/src/Source/WebKit/WebProcess/Plugins/PDF/UnifiedPDF/`;
- `webkit/src/Source/WebKit/Shared/mac/PDFContextMenu.h`;
- `webkit/src/Source/WebKit/Shared/PDFDisplayMode.h`;
- `webkit/src/Source/WebKit/UIProcess/PDF/`;
- `webkit/src/Source/WebKit/UIProcess/API/Cocoa/WKPDFConfiguration.h`;
- `webkit/src/Source/WebKit/Platform/spi/Cocoa/PDFKitSPI.h`;
- `webkit/src/Source/WebKit/Shared/Cocoa/PDFKitSoftLink.mm`;
- `webkit/src/Source/WebCore/html/PDFDocument.cpp`;
- `webkit/src/Source/WebCore/platform/graphics/cg/PDFDocumentImage.cpp`;
- `webkit/src/Source/WebCore/platform/graphics/mac/PDFDocumentImageMac.mm`.

This is not analogous to Chromium's extension-based PDF viewer. WebKit's macOS
path is built around PDFKit-backed plugin code plus WebKit's page/plugin input,
scrolling, accessibility, and UI-process context-menu plumbing.

Important implementation evidence:

- `PDFPluginBase.h` declares plugin operations for mouse events, selection,
  find, text-match discovery, password unlocking, link lookup, scrolling,
  annotation test hooks, and print.
- `PDFPluginBase.mm` contains scroll state, print dispatch, find pasteboard
  state, selection handling, annotation styling, and PDFKit string search.
- `PDFPlugin.mm` routes mouse move/down/up/drag into `PDFLayerController`,
  forwards key events and `selectAll`, constructs `PDFContextMenu` items, sends
  `ShowPDFContextMenu` to the UI process, handles password unlocking, manages
  annotation elements, exposes selection strings, and maps annotation bounds.
- `UnifiedPDFPlugin.*` and the `UnifiedPDF/` directory provide the newer
  UnifiedPDF implementation and presentation controllers.
- `PDFContextMenu.h` defines a structured PDF context-menu payload with menu
  items; this means context menus are likely observable through WebKit IPC or
  UI-process hooks, not through Chromium's PDF extension DOM.
- `WKPDFHUDView.*`, `WKPDFPageNumberIndicator.*`, and `WKPDFConfiguration.*`
  show a WebKit-owned PDF HUD/UI layer that is separate from Chromium's toolbar.

WebKit tests already cover many PDF behaviors relevant to Issue 834:

- `webkit/src/LayoutTests/pdf/` covers embed/iframe PDFs, scrolling tree state,
  dynamic PDF install/remove, printing events, loading PDFs twice, fullscreen,
  root page zoom, continuous/discrete display modes, and text annotations.
- `webkit/src/LayoutTests/pdf/annotations/` covers checkbox, radio, and dropdown
  annotation interaction.
- `webkit/src/LayoutTests/accessibility/mac/basic-embed-pdf-accessibility.html`
  proves PDFs loaded in embed tags are exposed in the accessibility tree.
- `webkit/src/Tools/TestWebKitAPI/Tests/WebKit/WKWebView/UnifiedPDFTests.mm`
  covers display modes, copy/select, print paths, annotation interaction,
  password unlock, scrolling state, and embedded/iframe cases.
- `webkit/src/Tools/TestWebKitAPI/Tests/WebKit/WKWebView/LegacyPDFPluginTests.mm`
  still covers legacy PDF plugin behavior such as PDF print in top-level and
  iframe contexts.
- `webkit/src/TestExpectations/apitests` has current skips for some UnifiedPDF
  rows: print size on macOS Sequoia and select-all text in embed/object-hosted
  PDFs. Those are upstream caveats to watch when defining TermSurf pass/fail
  criteria.

Surfari already has the generic TermSurf engine integration needed to host a
PDF-capable `WKWebView`:

- `surfari/libtermsurf_webkit/include/libtermsurf_webkit.h` exposes C ABI
  functions for browser contexts, web contents, URL loading, mouse, mouse move,
  scroll, key, focus, view size, JavaScript dialogs, HTTP auth, CA context,
  title, target URL, loading, console, and crash callbacks.
- `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm` creates
  `WKWebsiteDataStore` profile directories, `WKWebView` instances, `CAContext`
  remote contexts, DevTools web views, and forwards TermSurf input as `NSEvent`
  objects to the hit-tested WebKit view.
- `surfari/src/dispatch.rs` maps TermSurf protocol messages to the C ABI:
  `CreateTab`, `CreateDevtoolsTab`, `Resize`, `Navigate`, `MouseEvent`,
  `MouseMove`, `ScrollEvent`, `KeyEvent`, `FocusChanged`, and lifecycle events.
- Issue 756 has real-app Surfari guards proving launch, CAContext presentation,
  navigation, keyboard, click, drag, wheel, resize, pane/split/tab/window/focus
  geometry, profile isolation, crash handling, and Roamium comparison parity.

## Capability Matrix

| Issue 834 row                       | WebKit native evidence                                                     | Surfari integration status                                                            | First probe / risk                                                                                              |
| ----------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| full-page PDF rendering             | PDF plugin + UnifiedPDF source and tests                                   | `WKWebView` can load URLs and export CAContext; not yet proven with a PDF in TermSurf | Basic Surfari PDF load/render probe with screenshot/pixel proof.                                                |
| embedded PDF rendering              | Layout tests for embed/iframe PDFs                                         | Generic WKWebView rendering should cover it, but TermSurf has no PDF-specific probe   | After full-page proof, serve an HTML fixture with embed/iframe PDF.                                             |
| HTTP/HTTPS PDFs                     | WebKit URL loading and PDF tests                                           | Surfari navigation and loading callbacks already work for HTTP fixtures               | Basic probe should serve HTTP PDF first; HTTPS can follow if certificate/setup matters.                         |
| `file://` PDFs                      | WKWebView supports file URLs; WebKit has file tests                        | Surfari `urlFromCString` maps path-like strings to file URLs                          | Needs explicit local-file PDF probe because profile/sandbox behavior may differ.                                |
| extensionless PDFs                  | WebKit MIME handling likely controls this                                  | Surfari has no content-type PDF fixture yet                                           | Serve PDF bytes from extensionless path with `application/pdf`.                                                 |
| scroll wheel                        | PDF plugin scrolling code and scrolling-tree tests                         | Surfari wheel input is proven on HTML pages                                           | Reuse Issue 756 wheel automation against a PDF and observe scroll state or pixels.                              |
| keyboard navigation                 | PDF plugin key handling and display-mode tests                             | Surfari key input is proven on HTML pages                                             | Send page/arrow keys to PDF and observe scroll/page state.                                                      |
| mouse click and focus               | PDF plugin mouse event routing                                             | Surfari click/focus are proven on HTML pages                                          | Click inside PDF and verify focus/input trace or PDF state.                                                     |
| text selection and copy             | PDF selection APIs and UnifiedPDF copy tests                               | Surfari drag selection is proven on HTML pages                                        | Needs PDF drag/select/copy probe; upstream skips for select-all in embed/object are caveats.                    |
| internal PDF links                  | `PDFPluginBase` link lookup hooks                                          | Surfari target URL callback exists and is proven on HTML hover                        | Need PDF link fixture and navigation/target URL proof.                                                          |
| external PDF links                  | Same link hooks                                                            | Surfari navigation and target URL callbacks exist                                     | Need fixture with external URL and safe local target.                                                           |
| find/search                         | `findString`, text-match, and PDFKit search paths                          | Surfari has no TermSurf find/search command coverage for PDF                          | Need decide whether TUI/browser API can invoke WebKit find for PDF; likely needs Surfari-specific harness/API.  |
| toolbar/page navigation             | WebKit HUD/page indicator exists, not Chromium toolbar                     | Surfari does not expose a Chromium-style PDF toolbar                                  | Treat as engine-specific: probe WebKit HUD/page state, not Chromium toolbar selectors.                          |
| zoom and fit modes                  | WebKit display-mode/HUD infrastructure exists                              | Surfari has no PDF HUD automation                                                     | Probe if WebKit HUD is visible/controllable or use WebKit internals only for source classification.             |
| rotate                              | PDFKit likely supports page operations; source not yet mapped              | Surfari has no rotate API exposed                                                     | Unknown-needs-probe; may be unsupported in WebKit HUD for TermSurf.                                             |
| save/download                       | WebKit download/normal navigation paths exist                              | Surfari download behavior has not been audited for PDFs                               | Needs separate download/save audit; may not map to Chromium toolbar download.                                   |
| title propagation                   | Surfari title callback exists                                              | Proven for HTML; PDF title/file behavior unproven                                     | Basic PDF probe should assert WebTUI title/URL state.                                                           |
| copy/save restrictions              | PDFKit permissions may expose this differently                             | No Surfari restriction fixture yet                                                    | Need security/restriction fixture after core rendering/input works.                                             |
| disabled toolbar states             | WebKit HUD differs from Chromium toolbar                                   | No direct analogous UI known                                                          | Engine-specific matrix row; classify through WebKit HUD or mark unsupported-by-design with evidence.            |
| password-protected PDFs             | `attemptToUnlockPDF`, password field/form source, UnifiedPDF password test | Surfari key/mouse input proven, but PDF password UI unproven                          | Password fixture probe should be feasible after rendering.                                                      |
| malformed/error PDFs                | PDF plugin has load/destruction tests                                      | Surfari loading error callbacks exist                                                 | Need malformed PDF fixture and expected WebKit error/load state.                                                |
| native print                        | PDF plugin print dispatch plus LegacyPDF/UnifiedPDF print tests            | Surfari exposes no PDF print-specific TermSurf path yet                               | Needs a guarded native-print safety design later; do not open OS print UI in the basic render probe.            |
| forms                               | WebKit annotation tests cover checkbox/radio/dropdown                      | Surfari mouse/key input proven on HTML pages                                          | Likely feasible with WebKit annotation fixtures; need PDF form probe.                                           |
| annotations                         | Text annotation and annotation container/source tests                      | Surfari has no PDF annotation probe                                                   | Existing annotation rendering and widget tests suggest good native support; needs pixel/state proof.            |
| context menus                       | `PDFContextMenu` IPC structure and `ShowPDFContextMenu` path               | Surfari currently has no callback/API for PDF context menus                           | Likely needs `libtermsurf_webkit`/WebKit hook before safe automation; mirror Roamium context-menu safety first. |
| accessibility/searchify             | PDF accessibility object and mac accessibility layout tests                | Surfari has no PDF AX probe; Searchify is Chromium-specific                           | WebKit AX should be probed; Searchify should be marked Chromium-specific/not applicable for Surfari.            |
| split, tab, window, resize behavior | Engine-independent WKWebView CAContext resize                              | Issue 756 real-app guards prove Surfari geometry for HTML                             | Reuse Issue 756 geometry harness with a PDF URL after basic PDF render works.                                   |
| non-PDF regression smoke            | Existing Surfari HTML guards                                               | Issue 756 guards exist                                                                | Add Surfari PDF tests without weakening existing Issue 756 guards.                                              |

## Analysis

The audit supports a phased Surfari PDF approach:

1. Prove basic PDF rendering in the real TermSurf app first.
2. Reuse the existing Issue 756 real-app harness style and
   `web --browser surfari` launch path, because that already proves the app can
   host Surfari correctly.
3. Do not port Chromium PDF DevTools probes directly. WebKit's PDF viewer is not
   a Chromium extension DOM; evidence should come from WebKit/Safari-visible
   state, pixels, TermSurf protocol traces, Surfari traces, WebKit accessibility
   state, or new targeted `libtermsurf_webkit` hooks where needed.
4. Treat Chromium-only rows explicitly:
   - Searchify is Chromium-specific and should not be required for Surfari.
   - Chromium toolbar selectors do not map directly to WebKit's HUD/page
     indicator.
   - Context menus likely need WebKit/Safari-specific IPC or UI-process
     observation, not the Roamium watcher unchanged.
5. Delay WebKit source changes until a Surfari probe proves the missing layer.

## Conclusion

The next experiment should be a basic Surfari PDF load/render probe in the real
TermSurf app. It should:

- launch repo-built Ghostboard/WebTUI/Surfari using the Issue 756 harness
  patterns;
- serve a deterministic PDF over HTTP;
- run `web --browser surfari <pdf-url>`;
- prove Surfari receives the create-tab/navigation path;
- prove WebKit exports a nonzero CAContext for the PDF page;
- capture a screenshot or pixel sample proving PDF content is visible inside the
  terminal overlay;
- assert title/URL/loading state if available;
- close Surfari cleanly;
- leave all product and WebKit source unchanged unless the probe exposes a
  concrete integration bug.

This should happen before PDF-specific input, links, forms, print, or advanced
WebKit hooks are attempted.

## Completion Review

An external Codex review checked the completed audit.

Initial verdict: **Changes required**.

Required finding:

- The capability matrix omitted a dedicated print/native-print row even though
  the design required the audit to cover every major Issue 834 row and
  explicitly named print.

Fix:

- Added a native print row that separates WebKit native print evidence from
  Surfari/TermSurf integration status and names the first probe/risk: a guarded
  native-print safety design later, not during the basic render probe.

Final verdict after re-review: **Approved**.

The re-review found no findings.
