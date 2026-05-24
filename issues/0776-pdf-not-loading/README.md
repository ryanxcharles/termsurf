+++
status = "open"
opened = "2026-04-11"
+++

# Issue 776: PDF files show blank white screen instead of rendering

## Goal

Opening a local PDF file via `web file.pdf` should render the PDF content, not a
blank white screen.

## Background

When a user runs `web file.pdf` (or any local PDF path), the browser pane opens
but displays a blank white screen instead of rendering the PDF. The navigation
appears to work (no error is shown), but no PDF content is visible.

Chromium normally has a built-in PDF viewer (PDFium) that renders PDFs inline.
The blank screen suggests either:

1. The PDF viewer plugin is not enabled or not included in the Chromium
   build/configuration used by Roamium.
2. The `file://` URL scheme is not being handled correctly for PDF MIME types.
3. The PDF viewer requires UI chrome (toolbar, scroll handling) that isn't
   available in the embedded context.
4. Content Security Policy or sandboxing restrictions are blocking the PDF
   plugin from loading.

## Analysis

Possible areas to investigate:

- **Chromium PDF plugin registration** — Check whether `libtermsurf_chromium`
  enables the PDF viewer plugin (`chrome_pdf`) during browser initialization.
  Headless or minimal embeddings often skip plugin registration.
- **MIME type handling** — Verify that `file://` URLs with `.pdf` extension are
  being served with `application/pdf` MIME type and routed to the PDF viewer.
- **Plugin process sandboxing** — The PDF viewer runs in a utility process;
  sandbox restrictions in the TermSurf build may prevent it from launching.
- **Alternative approach** — If enabling the built-in viewer is complex,
  consider whether PDF.js (Mozilla's JavaScript PDF renderer) could work as a
  fallback.

## Experiments

### Experiment 1: Wire Chromium's PDF Viewer Into Roamium

#### Description

The likely cause is not `webtui` path handling. TermSurf's Chromium embedder is
built on Content Shell, and Content Shell does not wire Chromium's PDF viewer
pipeline. Electron's embedder shows the required pieces explicitly:

- PDF build deps and resources;
- PDF plugin registration through `ContentClient::AddPlugins()`;
- PDF component-extension/resource loading;
- PDF stream routing for the PDF viewer;
- renderer-side creation of the internal PDF plugin with
  `pdf::CreateInternalPlugin()`.

TermSurf currently overrides only `CreateContentBrowserClient()` in
`TsMainDelegate`. It does not provide a TermSurf `ContentClient`, does not
provide a TermSurf renderer client, and `libtermsurf_chromium/BUILD.gn` has no
PDF deps. Therefore the PDF navigation can succeed while no viewer exists to
render the document.

This experiment should wire the smallest Chromium PDF viewer pipeline required
for `web file.pdf` to render local PDFs in Roamium. Do not add webtui-side PDF
special cases unless the Chromium viewer path proves impossible.

#### Changes

1. Create a new Chromium branch for this issue:
   - start from the most relevant current branch, currently
     `148.0.7778.97-issue-778`;
   - create `148.0.7778.97-issue-776`;
   - add it to the Branches table in `chromium/README.md`.
2. Update `chromium/src/content/libtermsurf_chromium/BUILD.gn` to include the
   minimal PDF viewer deps. Start with the Electron/Chrome set that is directly
   needed by the embedder:
   - `//pdf`;
   - `//pdf:features`;
   - `//pdf:content_restriction`;
   - `//components/pdf/common:constants`;
   - `//components/pdf/common:util`;
   - `//components/pdf/renderer`;
   - `//components/pdf/browser`;
   - `//components/pdf/browser:interceptors`;
   - `//chrome/browser/resources/pdf:resources` if the component-extension
     viewer path is required.
3. Add a TermSurf content client, for example `ts_content_client.{cc,h}`,
   derived from `ShellContentClient`.
   - Override `AddPlugins()`.
   - Register the internal PDF plugin using the Chrome/Electron pattern:
     `pdf::kInternalPluginMimeType`, extension `"pdf"`, description
     `"Portable Document Format"`, and
     `content::WebPluginInfo::PLUGIN_TYPE_BROWSER_INTERNAL_PLUGIN`.
   - Keep all existing `ShellContentClient` behavior by inheriting from it.
4. Update `TsMainDelegate` to override `CreateContentClient()` and return the
   new TermSurf content client.
5. Add a TermSurf renderer client, for example `ts_renderer_client.{cc,h}`,
   derived from `ShellContentRendererClient`.
   - Override `OverrideCreatePlugin()`.
   - First preserve the existing Shell behavior, including surface-embed plugin
     handling.
   - If `params.mime_type.Utf8() == pdf::kInternalPluginMimeType`, call
     `pdf::CreateInternalPlugin(std::move(params), render_frame, {})`.
   - Return `true` only when a plugin was actually created. If PDF creation
     returns `nullptr`, let the normal Shell fallback continue or fail visibly;
     do not silently synthesize a blank page.
6. Update `TsMainDelegate` to override `CreateContentRendererClient()` and
   return the new TermSurf renderer client.
7. Wire the PDF component-extension path if the internal plugin alone is not
   sufficient:
   - register/load the PDF component extension manifest using
     `pdf_extension_util::GetManifest()`;
   - register PDF resources (`kPdfResources`) and template replacements;
   - ensure PDF streams are routed through `pdf::PdfViewerStreamManager` when
     `chrome_pdf::features::kPdfOopif` / the Chromium 148 equivalent is enabled.
     This step should follow Electron's implementation in:
   - `shell/browser/extensions/electron_extension_system.cc`;
   - `shell/browser/extensions/electron_component_extension_resource_manager.cc`;
   - `shell/browser/extensions/api/streams_private/streams_private_api.cc`.
8. Do not change:
   - `webtui`;
   - Roamium's Rust IPC;
   - `termsurf.proto`;
   - Wezboard overlay positioning or input forwarding.
9. Build `libtermsurf_chromium` with `autoninja`, rebuild Roamium against the
   new library, and regenerate the Issue 776 Chromium patch archive.

#### Non-Negotiable Invariants

- Local PDF rendering is fixed in Chromium/Roamium, not by special-casing PDF
  paths in `webtui`.
- Normal HTML navigation, local file navigation, link clicks, scrolling, and
  keyboard input remain unchanged.
- The fix must not introduce a second browser surface or a separate native
  window for PDFs; PDFs should render in the existing browser overlay.
- The PDF viewer must not require changes to the TermSurf protobuf protocol.
- The implementation must preserve Content Shell behavior unrelated to PDFs.
- The Chromium changes must live on the `148.0.7778.97-issue-776` branch and be
  archived under `chromium/patches/issue-776/`.

#### Verification

1. Build Chromium:
   ```bash
   cd chromium/src
   export PATH="$(cd ../depot_tools && pwd):$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```
2. Build Roamium and webtui in debug mode:
   ```bash
   cd /Users/ryan/dev/termsurf
   ./scripts/build.sh roamium
   ./scripts/build.sh webtui
   ```
3. Run debug Wezboard and debug Roamium together using the existing local test
   workflow. The Roamium process must load the newly built
   `libtermsurf_chromium`; testing an older installed Roamium invalidates the
   result.
4. Open a local PDF:
   ```bash
   web /absolute/path/to/file.pdf
   ```
   Expected: the PDF viewer renders visible document pages, not a blank white
   page.
5. Test a relative local PDF path from a shell working directory:
   ```bash
   web ./file.pdf
   ```
   Expected: it resolves to `file://...` and renders.
6. Test a remote PDF URL. Expected: the PDF viewer renders the PDF inline in the
   same overlay.
7. Scroll the PDF with mouse wheel/trackpad and keyboard. Expected: pages move
   and the overlay remains aligned with the pane.
8. Click normal links and open ordinary HTML pages after viewing a PDF.
   Expected: non-PDF navigation still works in the same tab.
9. Open DevTools on the PDF tab if supported. Expected: DevTools does not crash
   Roamium. It is acceptable if PDF internals are limited.
10. Regression smoke tests:
    - normal HTML page loads;
    - local HTML file loads;
    - text selection still works;
    - recent native popup fixes still behave as before.

#### Pass Criteria

Local and remote PDFs render visibly inside the existing TermSurf browser
overlay. Scrolling works, normal navigation still works, and no protocol,
webtui, or Wezboard changes are required.

#### Partial Criteria

The Chromium PDF viewer pipeline starts but one supporting piece is still
missing, such as component-extension resources, OOPIF stream routing, or PDF
renderer process setup. Record the exact failing layer and design Experiment 2
around that layer.

#### Failure Criteria

- PDFs still show a blank white page.
- The fix only handles PDFs by opening an external app or native window.
- The fix adds PDF-specific behavior to `webtui` while the Chromium viewer
  remains unwired.
- Normal HTML or local file navigation regresses.
- Roamium crashes when opening a PDF.
- The implementation cannot be archived as a clean Issue 776 Chromium branch.
