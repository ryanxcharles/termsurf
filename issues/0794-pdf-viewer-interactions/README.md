+++
status = "closed"
opened = "2026-05-29"
closed = "2026-05-30"
+++

# Issue 794: Complete PDF Viewer Interactions

## Goal

Make TermSurf's Chromium PDF support behave like a usable in-pane PDF viewer,
not just a first-page renderer. A PDF must scroll, respond to resize, accept
clicks, allow text selection and copy, and expose the normal PDF viewer controls
that Chromium/Electron users expect, except native PDF printing, which is
explicitly deferred to a separate follow-up issue.

## Background

Issues 792 and 793 proved the hard loading path:

- Issue 792 made a full-page PDF render inside Roamium by bringing in the
  Electron-style PDF component-extension, stream, and MimeHandler plumbing.
- Issue 793 fixed the tiny PDF iframe by preserving the PDF viewer CSS as a web
  accessible resource.

Manual testing after those fixes shows the current state is still not a complete
PDF viewer:

- The first page renders.
- The first page initially fits the visible webview height.
- Wheel scrolling does not move through the PDF.
- Resizing the webview does not resize/reflow the rendered pages.
- Click and drag text selection does not work.

That strongly suggests the remaining work is not "load one more PDF resource."
The viewer is present, but the interaction, viewport, and browser-side feature
surfaces are incomplete.

## Electron and Chromium Feature Inventory

Electron does not get complete PDF support by only calling
`pdf::CreateInternalPlugin()`. Its implementation mirrors the pieces of Chrome's
PDF viewer that an embedder must provide:

- `ElectronBrowserClient::CreateThrottlesForNavigation()` installs
  `pdf::PdfNavigationThrottle`.
- `ElectronBrowserClient::WillCreateURLLoaderRequestInterceptors()` installs
  `pdf::PdfURLLoaderRequestInterceptor`.
- `ElectronBrowserClient::CreateURLLoaderThrottles()` installs
  `PluginResponseInterceptorURLLoaderThrottle`.
- `ElectronBrowserClient::RegisterAssociatedInterfaceBindersForRenderFrameHost()`
  binds `pdf::mojom::PdfHost` through `pdf::PDFDocumentHelper`.
- Electron registers the PDF extension as a component extension and serves PDF
  resources plus template/i18n replacements through
  `ElectronComponentExtensionResourceManager`.
- Electron implements `pdfViewerPrivate` extension functions including
  `getStreamInfo`, `setPdfPluginAttributes`, `setPdfDocumentTitle`,
  `isAllowedLocalFileAccess`, and save-to-drive stubs or flows.
- Electron's PDF document helper propagates `SetPluginCanSave()` into
  `PdfViewerStreamManager` and emits a print-ready signal when content
  restrictions indicate printing is available.
- Electron's renderer client handles the internal PDF plugin and external plugin
  handling paths that create the PDF extension frame/container.

Chromium's PDF tests show the feature surface that a complete viewer needs:

- Full-page PDF load with the expected frame tree.
- Embedded PDF load with the expected frame tree.
- Focus into the full-page PDF.
- Mouse click and drag selection inside the PDF plugin.
- Selected text propagation/copy.
- Wheel scrolling and keyboard navigation.
- Resize and viewport updates.
- Toolbar controls for page navigation, zoom, fit modes, rotation, save, and
  non-print actions.
- Correct document title for titled and untitled PDFs.
- Local `file://` PDFs and extensionless local PDFs.
- Link navigation and PDF permissions.
- Save/download and content restriction behavior.
- Form focus, context menus, accessibility/searchify, and XFA where enabled.

## Current TermSurf Gaps

The current TermSurf PDF code has enough infrastructure to render, but several
Electron/Chromium behavior paths are still stubbed, unverified, or absent.

### Input, Focus, and Hit Testing

The current manual result proves rendering, not interaction. We do not yet know
whether wheel, click, drag, or keyboard events are reaching the PDF extension
frame and internal plugin with the focus and hit-test state Chromium expects.

Chromium's own PDF tests explicitly click into the plugin frame and wait for hit
test data before asserting focus or text selection. TermSurf currently needs a
dedicated experiment that proves where input stops:

- Wezboard synthetic mouse/wheel forwarding.
- Roamium protocol dispatch.
- Chromium `WebContents` input routing.
- PDF extension frame focus.
- Internal PDF plugin event handling.

Do not assume this is a Wezboard bug or a Chromium bug until that ladder is
measured.

### Scroll and Resize

The user-visible symptom is that the first page fits the initial webview height,
but the PDF does not scroll and does not resize when the pane changes size.

The most likely missing surfaces are:

- Viewer viewport update plumbing, including the
  `pdfViewerPrivate.onShouldUpdateViewport` path used by the PDF viewer.
- Browser-side frame/stream manager state that tells the viewer and plugin when
  dimensions change.
- Input/focus routing for wheel events if the viewer never receives scroll
  input.

These must be separated in the first experiment. A resize failure and a wheel
failure can share a symptom but have different causes.

### Text Selection and Copy

Text selection needs more than the page bitmap:

- The PDF plugin must receive mouse down/move/up events.
- The PDF plugin frame must be focusable.
- Selected text must propagate through Chromium's selection APIs.
- Copy must reach the selected PDF text, not the surrounding extension document
  or terminal pane.

Chromium's PDF tests cover selected text replies and direct `GetSelectedText()`
checks. TermSurf needs equivalent verification.

### Save and Toolbar State

TermSurf's `TsPDFDocumentHelperClient` currently logs callbacks such as
`UpdateContentRestrictions()`, `OnSaveURL()`, and `SetPluginCanSave()`. Electron
turns these into behavior:

- `SetPluginCanSave()` updates `PdfViewerStreamManager`.
- Save/download actions route through browser-side helpers.

These may not block scrolling, but they are part of a complete PDF viewer and
must be tracked.

Native PDF printing is deliberately out of scope for this issue. Experiment 20
proved the print button can be clicked without making the PDF viewport gray, but
it still does not open native print UI. That remaining print-specific work is
deferred to Issue 795.

### Document Metadata and Local File Behavior

Electron implements `pdfViewerPrivate.setPdfDocumentTitle()` so the PDF viewer
can update the tab title. Chrome's tests also cover titled PDFs, untitled PDFs,
local `file://` PDFs, and extensionless local PDF files.

TermSurf should not treat HTTP-only PDF rendering as complete support.

## Automation Strategy

The remaining PDF work should be tested automatically wherever possible. Manual
visual inspection is useful as a final sanity check, but the experiments should
produce objective pass/fail evidence for the interaction surface.

Use two levels of automation:

- **Chromium-level automation:** Drive Roamium through the Chrome DevTools
  Protocol. Use `Input.dispatchMouseEvent` for clicks, drags, and wheel events;
  use `Input.dispatchKeyEvent` for keyboard navigation; use screenshots and
  JavaScript evaluation to inspect viewer state, scroll position, page number,
  selected text, toolbar state, and resize behavior. This is the most precise
  way to prove whether the Chromium/PDF internals work.
- **Real app-path automation:** Drive the actual Wezboard window with macOS
  input events, then inspect screenshots and logs. This tests the complete
  TermSurf path: Wezboard input forwarding, protocol messages, Roamium dispatch,
  Chromium input routing, PDF extension focus, and the internal PDF plugin.

The first diagnostic harness should cover:

- **Render:** The PDF screenshot contains visible PDF text/pages.
- **Scroll:** Wheel input changes PDF viewer scroll state or visibly changes the
  screenshot.
- **Resize/reflow:** Resizing the webview changes PDF viewer viewport and page
  dimensions.
- **Click/focus:** Clicking inside the PDF focuses the PDF/plugin frame or
  produces the expected focus logs/state.
- **Drag selection:** Dragging across known text produces selected text through
  Chromium selection APIs or through the clipboard after copy.
- **Keyboard navigation:** PageDown, Arrow keys, and Space change scroll
  position or page number.
- **Toolbar controls:** Zoom, page navigation, fit mode, and rotate controls
  change viewer state.
- **Save/download:** Save/download actions fire the expected browser-side path
  or produce an output file in a controlled test directory.
- **Title:** The tab/title state reflects the PDF title or the expected URL
  fallback for untitled PDFs.
- **Local file:** `file://` PDFs behave the same as the local HTTP fixture.
- **Normal web regression:** A non-PDF page still scrolls, clicks, resizes, and
  accepts text selection normally.

Some checks will be inherently easier to automate than others. Native print
dialogs are deferred to a follow-up issue. OS save panels and subjective visual
quality can remain screenshot or log triage points. The core question, "does PDF
support work as an interactive viewer," should be automated with state checks,
screenshots, and targeted logs.

## Proposed Direction

This issue should not be solved with one giant patch. The work should proceed as
experiments, each reviewed by Codex before implementation and again after
completion:

1. First, build an interaction diagnostic harness for the current PDF viewer. It
   must distinguish rendering, wheel scroll, resize/reflow, click focus,
   drag-select text, selected-text copy, toolbar save state, and title updates.
2. Then fix the first measured broken layer. If input never reaches Chromium,
   fix the TermSurf input path. If input reaches Chromium but not the PDF
   plugin, fix the PDF frame/focus/hit-test layer. If resize reaches the webview
   but not the PDF viewer, fix the viewport/update path.
3. Continue one layer at a time until the PDF viewer is usable, not merely
   visible.

Electron remains the guide for embedder-owned browser infrastructure. Chromium's
PDF tests remain the feature checklist.

## Experiments

- [Experiment 1: Build PDF interaction harness](01-build-pdf-interaction-harness.md)
  — **Pass**
- [Experiment 2: Trace real wheel input](02-trace-real-wheel-input.md) —
  **Partial** (agent-side macOS wheel injection did not reach Wezboard)
- [Experiment 3: Direct protocol scroll injection](03-direct-protocol-scroll-injection.md)
  — **Pass** (protocol scroll reaches Roamium and Chromium FFI; PDF does not
  scroll)
- [Experiment 4: Route wheel events through Chromium's input router](04-route-wheel-through-input-router.md)
  — **Pass** (PDF wheel scrolling works through Chromium's input router)
- [Experiment 5: Route mouse events through Chromium's input router](05-route-mouse-through-input-router.md)
  — **Pass** (mouse routing works through Chromium's input router; PDF drag
  selection still needs deeper focus/selection work)
- [Experiment 6: Route keyboard selection to the focused PDF widget](06-route-keyboard-selection.md)
  — **Pass** (PDF keyboard select-all/copy works; mouse drag selection remains)
- [Experiment 7: Trace PDF drag selection inside PDFium](07-trace-pdf-drag-selection.md)
  — **Pass** (PDF drag selection works when the protocol drag starts on PDF
  text)
- [Experiment 8: Trace protocol resize and PDF reflow](08-trace-protocol-resize-reflow.md)
  — **Pass** (direct protocol resize reaches Roamium, Chromium, PDF plugin
  geometry, and PDFium plugin size)
- [Experiment 9: Trace real Wezboard pane resize](09-trace-real-wezboard-pane-resize.md)
  — **Pass** (real Wezboard split-pane resize reaches Roamium, Chromium, PDF
  plugin geometry, and PDFium plugin size)
- [Experiment 10: Prove safe PDF toolbar controls](10-toolbar-controls.md) —
  **Partial** (fit and page selector work; zoom in/out and rotate receive
  activation but do not change viewer state)
- [Experiment 11: Trace toolbar zoom and rotate events](11-trace-toolbar-zoom-rotate-events.md)
  — **Pass** (zoom reaches `Viewport.zoomIn/zoomOut` and throws
  `Viewport.mightZoom_`; fit and rotate work)
- [Experiment 12: Wire PDF resourcesPrivate strings for zoom](12-wire-pdf-resources-private-strings.md)
  — **Pass** (`resourcesPrivate.getStrings(PDF)` now returns PDF strings and 17
  preset zoom factors; toolbar zoom, fit, and rotate work)
- [Experiment 13: Probe save, print, title, and local-file parity](13-probe-save-print-title-local.md)
  — **Partial** (save/download creates a contained PDF file; local and
  extensionless PDFs render; PDF UI strings/title propagation/print containment
  remain incomplete)
- [Experiment 14: Complete PDF viewer strings and load-time data](14-complete-pdf-strings.md)
  — **Pass** (PDF viewer strings and template replacements are complete enough
  for the current viewer path; title propagation and print remain separate
  follow-ups)
- [Experiment 15: Wire PDF title propagation](15-wire-pdf-title-propagation.md)
  — **Pass** (full-page PDF titles now propagate through the top-level TermSurf
  title path while embedded PDFs preserve their host page title)
- [Experiment 16: Prove contained PDF print activation](16-contained-print-activation.md)
  — **Partial** (print can be safely exposed only under the intercept guard, but
  viewer-to-plugin `print` messages do not reach the PDF plugin print handler)
- [Experiment 17: Trace the PDF print message bridge](17-trace-pdf-print-message-bridge.md)
  — **Partial** (print reaches `PdfViewWebPlugin::HandlePrintMessage()`, but the
  contained renderer print intercept does not fire)
- [Experiment 18: Probe the PDF print intercept guard](18-probe-pdf-print-intercept-guard.md)
  — **Pass** (contained print intercept reaches `PdfViewWebPlugin::Print()` and
  writes `pdf-print.log`; default print remains disabled)
- [Experiment 19: Enable the production PDF print control](19-enable-production-print-control.md)
  — **Partial** (default print control is visible and contained print still
  works; manual production print smoke turns the PDF viewport gray instead of
  opening native print UI)
- [Experiment 20: Install the renderer print helper](20-install-renderer-print-helper.md)
  — **Pass** (print still does not open native UI, but the gray-viewport failure
  is fixed; native PDF printing is deferred to Issue 795)

## Conclusion

Issue 794 is complete under the revised scope. PDF viewing is now a usable
in-pane viewer experience: PDFs render at normal size, scroll, respond to real
and protocol resize, accept mouse and keyboard input, support text selection and
copy, expose working toolbar controls for page navigation, fit, zoom, rotate,
and save/download, propagate titles, and preserve local-file parity.

Native PDF printing is intentionally not solved here. Experiment 19 made the
print button visible but clicking it made the PDF viewport gray. Experiment 20
installed Chromium's renderer print helper and removed that gray-viewport
failure, but native print UI still does not appear. That print-specific
remaining work is deferred to Issue 795 so this issue can close around the
interactive viewer features that now work.

## Constraints

- Do not reopen or modify closed Issues 792 and 793.
- Preserve the existing PDF loading and iframe sizing fixes.
- Do not add more PDF protocol surface unless an experiment proves the current
  browser input/resize messages are insufficient.
- Do not make Wezboard-specific input changes until Chromium-side PDF routing
  has been measured.
- If an experiment modifies Chromium, create a fresh Chromium branch for this
  issue and add it to `chromium/README.md`.
- Every experiment design and every completed experiment result must be reviewed
  by Codex. Fix real issues from the review before proceeding.
