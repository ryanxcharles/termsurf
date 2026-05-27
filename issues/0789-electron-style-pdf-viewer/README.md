+++
status = "open"
opened = "2026-05-27"
+++

# Issue 789: Electron-Style PDF Viewer Infrastructure

## Goal

Make PDFs render inline in Roamium by adding the Electron-style Chromium
embedder infrastructure that the PDF viewer requires.

This issue continues from Issue 776. Issue 776 proved that PDF rendering is not
fixed by a single PDFium plugin toggle, wrapper page, MIME mapping, or direct
link to Chrome's full browser implementation. TermSurf needs its own small
embedder layer that mirrors the pieces Electron provides for Chromium's PDF
viewer path.

## Background

Opening a PDF with `web file.pdf` currently does not render a working inline PDF
viewer. Issue 776 investigated the failure and established several facts:

- Roamium is based on Chromium's `content_shell`-style embedding, so it does not
  automatically inherit Chrome's full PDF viewer feature stack.
- The internal PDF plugin can be registered and created, but that is not enough.
- A wrapper-only approach can load static viewer resources, but it does not
  enter Chromium's real PDF stream / PDF renderer pipeline.
- Chromium can manage the PDF renderer process itself once the embedder enters
  the proper PDF viewer path. TermSurf should not manually spawn or manage a
  separate PDF process.
- Directly linking Chrome's stock `PluginResponseInterceptorURLLoaderThrottle`
  path is too broad for Roamium. Issue 776 Experiment 8 showed that adding
  `//chrome/browser/plugins:impl` pulled in large Chrome product subsystems and
  failed at link time.

The important architectural lesson from Issue 776 is that Electron is the right
model. Electron does not make itself Chrome. It provides Electron-owned glue for
the PDF viewer pieces that Chrome normally owns, then patches Chromium's PDF
stream path to call Electron's implementations.

TermSurf should do the same for Roamium.

## Electron Model

Electron's PDF implementation has several important pieces:

- `ElectronContentClient` registers the internal PDF plugin.
- `RendererClientBase::RenderFrameCreated()` binds
  `MimeHandlerViewContainerManager` in renderer frames.
- `RendererClientBase::IsPluginHandledExternally()` routes `application/pdf`
  through `MimeHandlerViewContainerManager::CreateFrameContainer()`.
- `ElectronBrowserClient::CreateURLLoaderThrottles()` installs
  `PluginResponseInterceptorURLLoaderThrottle`.
- Electron carries a Chromium patch that redirects Chrome's plugin response
  interceptor from Chrome's `streams_private` API to Electron's
  `streams_private` API.
- Electron's `streams_private` implementation receives intercepted PDF streams
  and feeds `PdfViewerStreamManager`.
- Electron serves PDF viewer extension resources with an Electron-owned
  component extension resource manager.
- Electron provides enough `pdf_viewer_private` and `PdfHost` /
  `PDFDocumentHelper` glue for the PDF viewer shell and plugin to run.

The key pattern is ownership: Electron copies or adapts the embedder-facing glue
instead of importing Chrome's whole browser layer.

## TermSurf Direction

TermSurf should add a Roamium-owned PDF viewer embedder layer under
`content/libtermsurf_chromium/` and nearby TermSurf-specific Chromium files.

The target architecture is:

1. Keep the internal PDF plugin registration from Issue 776.
2. Keep the static PDF viewer resource serving from Issue 776 Experiment 7.
3. Replace the failed direct Chrome dependency from Issue 776 Experiment 8 with
   a TermSurf-owned PDF response throttle or a narrow Chromium patch that calls
   TermSurf-owned code.
4. Add a TermSurf `streams_private` equivalent that stores intercepted PDF
   streams in `PdfViewerStreamManager`.
5. Add the renderer-side MimeHandlerView container wiring needed to convert
   `application/pdf` into a PDF viewer frame.
6. Add browser-side PDF URL loader request interception so the viewer's content
   frame can claim the original PDF stream.
7. Add enough `pdf_viewer_private` and `PdfHost` / `PDFDocumentHelper` support
   for the viewer shell to display the PDF.
8. Let Chromium's existing PDF navigation / SiteInstance / renderer launch path
   create the correct PDF renderer role.

## Constraints

- Do not link Chrome's full browser feature stack into Roamium just to get PDFs.
- Do not add `//chrome/browser/plugins:impl` back as the primary solution unless
  a later experiment proves a narrowly bounded form can link without dragging in
  unrelated Chrome product infrastructure.
- Do not enable general user extension support as a side effect of PDF support.
- Do not weaken PDF origin checks or mark ordinary renderers as PDF renderers.
- Do not fake PDF rendering with static HTML, screenshots, external apps, or
  macOS Preview handoff.
- Do not change the TermSurf protocol unless a later experiment proves the PDF
  viewer path needs protocol-level information that cannot be represented inside
  Chromium/Roamium.
- Every Chromium experiment in this issue must use its own branch following the
  project convention.

## Starting Point

The immediate next step is to design Experiment 1 for this issue.

Experiment 1 should be scoped around the first narrow Electron-style layer that
can replace the dead end from Issue 776 Experiment 8:

- study Electron's `streams_private` redirect and implementation in detail;
- design a TermSurf-owned PDF response throttle / `streams_private` handoff that
  avoids `//chrome/browser/plugins:impl`;
- identify the exact Chromium files that must be patched and the exact
  TermSurf-owned files that should receive the copied/adapted behavior;
- define a build verification that proves the new path avoids the dependency
  explosion from Issue 776 Experiment 8 before trying to make PDFs visibly
  render.

This issue should proceed one experiment at a time. Each experiment should land
one coherent layer or prove why that layer must be shaped differently.
