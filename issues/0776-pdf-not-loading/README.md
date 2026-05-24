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

### Experiment 1: Probe Chromium PDF Viewer Plumbing

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

This experiment is a scoped plumbing probe, not a full PDF-viewer port. It
should add the smallest TermSurf `ContentClient` / renderer-client hooks needed
to register PDF plugin metadata, observe whether Chromium calls the relevant PDF
hooks, and identify the next missing layer. The expected outcome is **Partial**:
PDFs may still not render after this experiment, but the result should precisely
identify whether the next layer is plugin registration, renderer plugin
creation, `IsPluginHandledExternally()`, MimeHandlerView/extension
infrastructure, or stream routing.

This experiment commits to the expensive product direction: in-pane PDF viewing
inside the TermSurf browser overlay. A simpler fallback would be to hand PDFs to
macOS Preview or another external viewer, but that breaks the in-terminal
experience. If the Chromium pipeline proves far larger than expected in later
experiments, that product trade-off can be revisited explicitly.

#### Changes

1. Precondition checks:
   - Verify the current Chromium GN args do not disable PDF/plugin/extension
     support:
     ```bash
     cd chromium/src
     gn args out/Default --list | rg 'enable_pdf|enable_extensions|enable_plugins'
     ```
     Expected: `enable_pdf`, `enable_extensions`, and `enable_plugins` are
     enabled under defaults or are not overridden to `false`.
   - Check whether `libtermsurf_chromium`'s `testonly = true` blocks the
     proposed deps. Run `gn check out/Default //content/libtermsurf_chromium`.
     If a needed PDF dep is rejected because it is not test-only-compatible,
     stop and record that as the result; do not silently remove
     `testonly = true` without a separate design.
2. Create a new Chromium branch for this issue:
   - start from the most relevant current branch, currently
     `148.0.7778.97-issue-778`;
   - create `148.0.7778.97-issue-776`;
   - add it to the Branches table in `chromium/README.md`.
3. Update `chromium/src/content/libtermsurf_chromium/BUILD.gn` to include only
   the minimal PDF deps needed for this plumbing probe:
   - `//pdf`;
   - `//pdf:features`;
   - `//components/pdf/common:constants`;
   - `//components/pdf/common:util`;
   - `//components/pdf/renderer`. Do not add
     `//chrome/browser/resources/pdf:resources`, `//components/pdf/browser`,
     extension, MimeHandlerView, or stream-manager deps in this experiment
     unless a compile error proves one of them is needed by the plumbing probe
     itself.
4. Add a TermSurf content client, for example `ts_content_client.{cc,h}`,
   derived from `ShellContentClient`.
   - Override `AddPlugins()`.
   - Register the internal PDF plugin using the Chrome/Electron pattern:
     `pdf::kInternalPluginMimeType`, extension `"pdf"`, description
     `"Portable Document Format"`, and
     `content::WebPluginInfo::PLUGIN_TYPE_BROWSER_INTERNAL_PLUGIN`.
   - Keep all existing `ShellContentClient` behavior by inheriting from it.
   - Add a temporary `LOG(INFO)` line confirming PDF plugin registration.
5. Update `TsMainDelegate` to override `CreateContentClient()` and return the
   new TermSurf content client.
6. Add a TermSurf renderer client, for example `ts_renderer_client.{cc,h}`,
   derived from `ShellContentRendererClient`.
   - Override `OverrideCreatePlugin()`.
   - First preserve the existing Shell behavior, including surface-embed plugin
     handling.
   - If `params.mime_type.Utf8() == pdf::kInternalPluginMimeType`, call
     `pdf::CreateInternalPlugin(std::move(params), render_frame, {})`.
   - Return `true` only when a plugin was actually created. If PDF creation
     returns `nullptr`, let the normal Shell fallback continue or fail visibly;
     do not silently synthesize a blank page.
   - Add temporary `LOG(INFO)` lines that report each PDF MIME type seen by
     `OverrideCreatePlugin()` and whether `pdf::CreateInternalPlugin()` returned
     a plugin or `nullptr`.
7. Also override `IsPluginHandledExternally()` in the TermSurf renderer client
   if it is available on the Chromium 148 `ContentRendererClient` interface.
   - Log when `mime_type == pdf::kPDFMimeType` or
     `pdf::kInternalPluginMimeType`.
   - Do not try to implement MimeHandlerView in this experiment.
   - If the method is not available or cannot be implemented without pulling in
     extensions infrastructure, record that explicitly in the result.
8. Update `TsMainDelegate` to override `CreateContentRendererClient()` and
   return the new TermSurf renderer client.
9. Do not wire the PDF component-extension, MimeHandlerView, GuestView, or
   streams-private path in this experiment. Instead, document them as the likely
   next layer if plugin registration succeeds but PDFs still do not render. The
   future implementation should be based on Electron's:
   - `shell/browser/extensions/electron_extension_system.cc`;
   - `shell/browser/extensions/electron_component_extension_resource_manager.cc`;
   - `shell/browser/extensions/api/streams_private/streams_private_api.cc`;
   - Chromium patch equivalent for redirecting
     `plugin_response_interceptor_url_loader_throttle.cc` to the embedder's
     streams-private implementation.
10. Do not change:
    - `webtui`;
    - Roamium's Rust IPC;
    - `termsurf.proto`;
    - Wezboard overlay positioning or input forwarding.

11. Build `libtermsurf_chromium` with `autoninja`, rebuild Roamium against the
    new library, and run the diagnostic verification. Only regenerate and commit
    the Issue 776 Chromium patch archive if this experiment produces a coherent
    branch state worth preserving. On Partial or Fail, record the branch state
    and the next required layer before deciding whether to archive.

#### Non-Negotiable Invariants

- Local PDF rendering is fixed in Chromium/Roamium, not by special-casing PDF
  paths in `webtui`.
- Normal HTML navigation, local file navigation, link clicks, scrolling, and
  keyboard input remain unchanged.
- The fix must not introduce a second browser surface or a separate native
  window for PDFs; PDFs should render in the existing browser overlay.
- The PDF viewer must not require changes to the TermSurf protobuf protocol.
- The implementation must preserve Content Shell behavior unrelated to PDFs.
- Experiment 1 must not quietly grow into an extensions/MimeHandlerView port. If
  that layer is required, stop with a Partial result and design Experiment 2
  around it.
- The Chromium changes must live on the `148.0.7778.97-issue-776` branch and be
  archived under `chromium/patches/issue-776/`.

#### Verification

1. Record the precondition results:
   - `enable_pdf`, `enable_extensions`, and `enable_plugins` state;
   - whether `testonly = true` blocks the minimal PDF deps;
   - whether `gn check out/Default //content/libtermsurf_chromium` passes after
     the minimal deps are added.
2. Build Chromium:
   ```bash
   cd chromium/src
   export PATH="$(cd ../depot_tools && pwd):$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```
3. Build Roamium and webtui in debug mode:
   ```bash
   cd /Users/ryan/dev/termsurf
   ./scripts/build.sh roamium
   ./scripts/build.sh webtui
   ```
4. Run debug Wezboard and debug Roamium together using the existing local test
   workflow. The Roamium process must load the newly built
   `libtermsurf_chromium`; testing an older installed Roamium invalidates the
   result.
5. Open a local PDF:
   ```bash
   web /absolute/path/to/file.pdf
   ```
   Expected for this experiment: the page may still be blank, but the log should
   answer whether PDF plugin registration happened, whether
   `OverrideCreatePlugin()` saw `pdf::kInternalPluginMimeType`, whether
   `pdf::CreateInternalPlugin()` returned a plugin or `nullptr`, and whether
   `IsPluginHandledExternally()` saw the PDF MIME type.
6. Test a relative local PDF path from a shell working directory:
   ```bash
   web ./file.pdf
   ```
   Expected: it resolves to `file://...` and reaches the same PDF diagnostic
   path.
7. Test a remote PDF URL. Expected: it reaches the same PDF diagnostic path.
8. Click normal links and open ordinary HTML pages after trying a PDF. Expected:
   non-PDF navigation still works in the same tab.
9. Regression smoke tests:
   - normal HTML page loads;
   - local HTML file loads;
   - text selection still works;
   - recent native popup fixes still behave as before.
10. Record a failure-layer table:

    | Layer                                                 | Result                 |
    | ----------------------------------------------------- | ---------------------- |
    | PDF build flags enabled                               | yes/no                 |
    | Minimal PDF deps compile                              | yes/no                 |
    | PDF plugin registered in `AddPlugins()`               | yes/no                 |
    | `OverrideCreatePlugin()` sees internal PDF MIME type  | yes/no                 |
    | `CreateInternalPlugin()` returns a plugin             | yes/no/null-not-called |
    | `IsPluginHandledExternally()` sees top-level PDF      | yes/no/not-available   |
    | Evidence that MimeHandlerView/extension layer is next | yes/no                 |

#### Pass Criteria

PDFs render visibly inside the existing TermSurf browser overlay using only the
minimal content-client / renderer-client wiring. This is unlikely, but if it
happens, verify scrolling, normal navigation, and local/remote PDFs before
closing the issue.

#### Partial Criteria

The expected outcome is Partial. The experiment succeeds as a diagnostic if it
builds and identifies the first missing layer in dependency order:

1. PDF build flags/deps;
2. plugin registration;
3. renderer plugin creation;
4. top-level PDF external plugin handling;
5. MimeHandlerView/extension infrastructure;
6. PDF stream routing.

If multiple layers are missing, the next experiment should target the earliest
missing layer in this list.

#### Failure Criteria

- The experiment makes PDFs blank or crashes without producing enough logs to
  identify the first missing layer.
- The fix only handles PDFs by opening an external app or native window.
- The fix adds PDF-specific behavior to `webtui` while the Chromium viewer
  remains unwired.
- Normal HTML or local file navigation regresses.
- Roamium crashes when opening a PDF.
- The experiment quietly starts porting the extension/MimeHandlerView stack
  instead of stopping at the scoped plumbing probe.
