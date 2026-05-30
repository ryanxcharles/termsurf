# Experiment 3: Security Audit

## Description

This experiment audits the current PDF implementation for security issues. It is
diagnostic only. It must not change Chromium, Rust, JavaScript, Python, protocol
surface, fixtures, or runtime behavior.

The goal is to produce a concrete security cleanup plan for Experiment 4. The
audit should identify any way an untrusted PDF or web page could escape the
intended PDF viewer path, gain broader extension/API/resource/file access than
intended, confuse stream/frame ownership, trigger unsafe lifetime behavior, or
activate automation-only code in production.

This audit runs after the code organization cleanup, so it should review the
current helper structure on branch `148.0.7778.97-issue-796-exp2`, not the older
Issue 792-794 source layout. Native PDF printing remains out of scope except for
the existing print-containment and print-intercept guards that touch PDF viewer
safety.

This experiment must receive Codex design review before it runs. After the audit
result is recorded, Codex must review the completed audit before Experiment 4 is
designed.

## Scope

Audit only PDF-related security surfaces introduced or materially changed by
Issues 792, 793, 794, and the Issue 796 organization cleanup.

Primary Chromium scope:

- PDF-relevant call sites in
  `chromium/src/content/libtermsurf_chromium/ts_browser_client.*`,
  `chromium/src/content/libtermsurf_chromium/ts_content_renderer_client.*`, and
  `chromium/src/content/libtermsurf_chromium/ts_content_client.*`, because these
  thin dispatchers still decide when the extracted PDF helpers run;
- `chromium/src/content/libtermsurf_chromium/ts_pdf_browser_support.*`
- `chromium/src/content/libtermsurf_chromium/ts_pdf_renderer_support.*`
- `chromium/src/content/libtermsurf_chromium/ts_pdf_stream_delegate.*`
- `chromium/src/content/libtermsurf_chromium/ts_pdf_iframe_navigation_throttle.*`
- `chromium/src/content/libtermsurf_chromium/ts_plugin_response_interceptor_url_loader_throttle.*`
- `chromium/src/content/libtermsurf_chromium/ts_plugin_utils.*`
- `chromium/src/content/libtermsurf_chromium/ts_mime_handler_binders.*`
- `chromium/src/content/libtermsurf_chromium/ts_pdf_document_helper_client.*`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_pdf_*`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_resources_private_api.*`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_component_extension_resource_manager.*`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_extension_resource_loader.*`
- `chromium/src/content/libtermsurf_chromium/extensions/ts_extensions_*`
- PDF-specific TermSurf patches in `chromium/src/pdf/` and
  `chromium/src/components/printing/`.

Primary Rust, JavaScript, and automation scope:

- Roamium PDF/input/resize dispatch paths if they accept PDF-originated IDs,
  URLs, dimensions, or commands;
- Wezboard PDF input/resize routing only where it could expose browser-process
  trust or file access;
- `scripts/test-issue-794-*.py`;
- `scripts/termsurf_pdf_protocol_harness.py`;
- `scripts/probe-pdf-*.mjs`;
- `scripts/capture-pdf-interactions.mjs`.

Out of scope:

- unrelated browser security work;
- native PDF printing implementation from Issue 795;
- normal upstream Chromium/PDFium memory-safety audit outside TermSurf patches;
- broad extension-system hardening unrelated to the PDF viewer path;
- completeness/user-experience gaps that are not security relevant.

## Audit Method

### 1. Build a trust-boundary map

Map the security-relevant PDF flow from the original URL to the rendered viewer:

- original PDF URL classification and MIME detection;
- top-level and embedded PDF navigation throttles;
- stream claiming and lookup keys;
- frame tree node, render frame host, tab, and process identifiers;
- PDF component-extension registration and manifest permissions;
- extension-scheme resource serving;
- renderer-side PDF plugin creation and externalization;
- `resourcesPrivate` and `pdfViewerPrivate` API entrypoints;
- file, extensionless, HTTP, and HTTPS PDF paths;
- print-containment and print-intercept flags.

The map must distinguish trusted actors, untrusted actors, and data crossing
between them. It should explicitly state which code decides that a request is
"the PDF viewer" rather than an arbitrary extension page or web page.

### 2. Compare TermSurf's PDF security model to Chrome/Electron

Use local open-source copies where available. Compare TermSurf's implementation
against the relevant Chrome/Electron patterns for:

- PDF component-extension manifest permissions and web-accessible resources;
- extension process recognition and process-map grants;
- stream-manager ownership checks;
- `resourcesPrivate.getStrings(PDF)`;
- `pdfViewerPrivate.setPdfDocumentTitle`;
- internal PDF plugin origin checks;
- file access handling for `file://` and extensionless local PDFs;
- print containment or print interception.

The audit does not need to prove exact Chrome parity everywhere. It must record
where TermSurf intentionally differs, whether the difference is security
neutral, and what cleanup or test would make the boundary clearer.

### 3. Review URL, origin, and resource access

Inspect every path that accepts or constructs:

- `chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/...`;
- `chrome://resources/...`;
- original PDF URLs;
- stream URLs;
- local `file://` URLs;
- extensionless local file URLs;
- resource bundle paths or resource IDs.

Questions to answer:

- Can a non-PDF web page or a different extension invoke PDF-only APIs?
- Can a PDF origin get access to broader `chrome://resources` or extension
  resources than the PDF viewer needs?
- Can a crafted URL, redirect, path, query, fragment, or extensionless local
  path bypass MIME or origin checks?
- Are resource lookups bounded to known resources rather than path-derived disk
  access?
- Are `file://` grants restricted to the loaded PDF file and viewer needs?

### 4. Review stream, frame, tab, and process ownership

Inspect every path that stores, retrieves, forwards, or trusts:

- stream IDs;
- frame tree node IDs;
- render process and render frame IDs;
- tab IDs;
- `RenderFrameHost*`, `RenderFrame*`, `WebContents*`, and `BrowserContext*`;
- extension process-map grants;
- PDF title/update messages derived from extension frames.

Questions to answer:

- Can a stale frame or reused ID retrieve another PDF's stream?
- Can one tab or PDF viewer influence another tab's stream/title/state?
- Are null and lifecycle checks sufficient around frame and WebContents lookup?
- Are process-map grants scoped to the PDF extension process and revoked or
  naturally bounded by Chromium lifetime rules?
- Are renderer-originated calls validated in the browser before changing
  browser-visible state?

### 5. Review automation-only and diagnostic paths

Audit the env-var and command-line switch gates for:

- PDF input tracing;
- PDF resize tracing;
- PDF print bridge tracing;
- PDF print intercept;
- screenshot/devtools/automation harness paths.

Questions to answer:

- Are dangerous automation paths off by default?
- Does enabling a trace only log, or can it alter runtime behavior?
- Can production users or untrusted content trigger print interception without
  an explicit local env var/switch?
- Are trace file paths controlled only by local process environment, not web
  content?
- Do logs include sensitive URLs or local paths, and if so, is that acceptable
  for explicit debug traces?

### 6. Review C++ safety assumptions in TermSurf-owned PDF code

This is not a full upstream Chromium or PDFium memory-safety audit. Focus on
TermSurf-owned code and patches. Look for:

- unchecked null pointers;
- raw pointer lifetime assumptions;
- callbacks or posted tasks that can outlive frames/WebContents;
- unsafe casts;
- integer truncation or unbounded size conversion;
- unbounded string or byte logging;
- file path handling;
- fallthrough defaults that fail open;
- `CHECK`/`DCHECK` choices that could turn untrusted content into a browser or
  renderer crash.

### 7. Classify findings

Each finding must include:

- **Severity:** `Critical`, `High`, `Medium`, `Low`, or `Defense-in-depth`.
- **Confidence:** `High`, `Medium`, or `Low`.
- **Files:** exact file paths and line references where practical.
- **Threat:** what an untrusted PDF, web page, or local environment could do.
- **Current guard:** what guard exists today, if any.
- **Gap:** why the guard may be too broad, absent, unclear, or unverifiable.
- **Recommended cleanup:** the concrete fix or hardening step for Experiment 4.
- **Verification needed:** static check, automated test, negative test, or
  manual security reasoning needed to prove the cleanup.

### 8. Separate non-findings and intentional differences

Record notable non-findings where code looks risky but is acceptable because of
Chromium invariants, Electron parity, explicit local-only debug gating, or
closed-world resource IDs. These prevent Experiment 4 from chasing cosmetic or
non-security churn.

### 9. Produce the Experiment 4 cleanup backlog

The conclusion must split the audit output into:

- security fixes that must be implemented in Experiment 4;
- defense-in-depth improvements that should be included if small;
- findings that need a separate follow-up issue because they exceed this issue's
  scope;
- non-findings or rejected concerns;
- minimum verification matrix for Experiment 4.

If the audit finds no exploitable issues, Experiment 4 should still be designed
to add the smallest useful set of assertions, comments, or negative tests that
make the security boundary easier to maintain.

## Commands and Evidence

Use `rg` first for searches. Suggested starting points:

```bash
rg -n "chrome-extension://|chrome://resources|PDF_EXTENSION|pdfViewerPrivate|resourcesPrivate|GetStream|StreamInfo|frame_tree|FrameTree|process_map|Grant|Allow|Origin|file://|FilePath|TERMSURF_PDF|PRINT_INTERCEPT|CHECK|DCHECK|raw_ptr|Unretained|WeakPtr" \
  chromium/src/content/libtermsurf_chromium \
  chromium/src/pdf \
  chromium/src/components/pdf \
  chromium/src/components/printing \
  roamium/src \
  wezboard/wezboard-gui/src/termsurf \
  scripts
```

```bash
rg -n "CanExecute|ExtensionFunction|Run\\(|Respond|GetBrowserContext|GetWebContents|FromFrame|FromRenderFrameHost|RenderFrameHost|RenderProcessHost|SiteInstance|ChildProcessSecurityPolicy|URLLoader|NavigationThrottle|MimeHandler|CreateInternalPlugin|IsPdfInternalPluginAllowedOrigin" \
  chromium/src/content/libtermsurf_chromium \
  chromium/src/pdf \
  chromium/src/components/pdf \
  chromium/src/components/printing
```

```bash
rg -n "TERMSURF_PDF|--termsurf|print-intercept|trace-file|user-data-dir|file-pdf-url|file-extensionless-url" \
  scripts \
  roamium/src \
  wezboard/wezboard-gui/src/termsurf
```

Suggested local reference searches:

```bash
rg -n "pdfViewerPrivate|getStreamInfo|resourcesPrivate|PdfViewerStreamManager|ChromePdfStreamDelegate|IsPdfExtensionOrigin|IsPdfInternalPluginAllowedOrigin" \
  chromium/src/chrome \
  chromium/src/components/pdf \
  chromium/src/extensions
```

If using Electron as a reference, use the local open-source research workflow
and cite exact files/lines from the local checkout or note if a local checkout
is unavailable.

The final audit must cite current worktree files and line numbers. Patch names
alone are not sufficient.

## Verification

This is a documentation-only audit experiment. Verification is:

- Codex design review completed and real design findings fixed;
- no runtime code changed;
- the audit result is appended to this file under `## Result`;
- the trust-boundary map is present;
- findings cite current files and line references where practical;
- findings are separated into required fixes, defense-in-depth improvements,
  follow-up issues, rejected concerns, and non-findings;
- every required Experiment 4 cleanup item has a concrete verification plan;
- Codex completion review completed and real findings fixed;
- Prettier run on this file and the issue README.

No Chromium, Rust, or Roamium build is required unless the audit accidentally
changes code. It must not change code.

## Pass Criteria

This experiment passes if it produces an evidence-backed security audit that
identifies the actual security cleanup backlog for Experiment 4, or proves that
no exploitable security issues were found and defines the minimum
defense-in-depth cleanup needed to preserve that boundary.

## Partial Criteria

This experiment is partial if it identifies likely security issues but lacks
enough line-level evidence, threat modeling, or verification guidance to safely
design Experiment 4.

## Failure Criteria

This experiment fails if:

- it changes runtime behavior;
- it combines audit and cleanup;
- it audits broad upstream Chromium/PDFium code instead of TermSurf's PDF
  integration;
- it treats native PDF printing as in scope;
- it relies on old Issue 792-794 layouts instead of the current organized code;
- it claims safety without checking URL/origin/resource, stream/frame ownership,
  extension API, file access, automation gates, and C++ lifetime surfaces;
- it omits Codex design or completion review;
- it produces a cleanup backlog too vague to implement safely.

## Result

**Result:** Pass

This audit reviewed the current post-cleanup PDF implementation on Chromium
branch `148.0.7778.97-issue-796-exp2`. No runtime code was changed.

### Trust-boundary map

The PDF viewer path has these security boundaries:

1. **Untrusted PDF URL to browser interception.** Main-frame and subframe
   navigations enter TermSurf's browser client in
   `chromium/src/content/libtermsurf_chromium/ts_browser_client.cc:59` and
   `chromium/src/content/libtermsurf_chromium/ts_browser_client.cc:80`.
   `AddTsPdfNavigationThrottles()` installs Chromium's
   `pdf::PdfNavigationThrottle` with `TsPdfStreamDelegate` in
   `ts_pdf_browser_support.cc:125`. The response interceptor only proceeds for
   non-download `application/pdf` responses mapped to the fixed Chromium PDF
   extension id in `ts_plugin_response_interceptor_url_loader_throttle.cc:169`,
   `ts_plugin_response_interceptor_url_loader_throttle.cc:175`, and
   `ts_plugin_response_interceptor_url_loader_throttle.cc:181`.
2. **Browser stream ownership.** PDF bytes become a
   `extensions::StreamContainer` owned by `PdfViewerStreamManager` for one
   `WebContents` and one frame-tree node in
   `ts_plugin_response_interceptor_url_loader_throttle.cc:93` and
   `ts_plugin_response_interceptor_url_loader_throttle.cc:126`. The generated
   stream URL is unguessable and extension-scoped in
   `ts_plugin_response_interceptor_url_loader_throttle.cc:257`.
3. **PDF extension and resource loading.** TermSurf registers one component
   extension with the canonical Chromium PDF extension id in
   `extensions/ts_pdf_component_extension.cc:115` and strips the manifest
   permissions down to `pdfViewerPrivate` and `resourcesPrivate` in
   `extensions/ts_pdf_component_extension.cc:87`. Extension resources are served
   from `kPdfResources`, not from disk paths, through
   `extensions/ts_component_extension_resource_manager.cc:23` and
   `extensions/ts_extension_resource_loader.cc:65`.
4. **Renderer plugin creation.** The renderer grants the fixed PDF extension
   origin access to `chrome://resources` in `ts_pdf_renderer_support.cc:62`. It
   externalizes only the internal PDF MIME when
   `IsPdfInternalPluginAllowedOrigin()` accepts the frame origin in
   `ts_pdf_renderer_support.cc:95`, and creates the internal plugin only when
   Chromium says this is a PDF renderer in `ts_pdf_renderer_support.cc:138`.
5. **Extension API surface.** TermSurf registers only
   `pdfViewerPrivate.setPdfDocumentTitle` and `resourcesPrivate.getStrings` in
   `extensions/ts_extensions_browser_client.cc:64`. It does not implement the
   broader Chrome/Electron PDF private API set in this issue.
6. **Input, resize, and print traces.** Browser-to-renderer print intercept and
   trace switches are appended only from local environment variables in
   `ts_pdf_browser_support.cc:261`. Input traces in Roamium/Wezboard are gated
   by local `TERMSURF_PDF_INPUT_TRACE` environment variables in
   `roamium/src/dispatch.rs:28` and
   `wezboard/wezboard-gui/src/termsurf/input.rs:43`.

Trusted actors are the TermSurf browser process, Chromium's PDF extension
renderer for `mhjfbmdgcfjbbpaeojofohoefgiehjai`, and Chromium's internal PDF
plugin renderer path. Untrusted actors are the original PDF document, arbitrary
web pages embedding PDFs, and any future non-PDF extension page if TermSurf ever
loads more component extensions.

### Chrome and Electron comparison

TermSurf intentionally follows Electron's approach: it implements an
embedder-owned subset of the Chrome PDF extension stack instead of linking all
of Chrome's browser feature code.

Relevant reference points:

- Chromium's canonical internal PDF plugin guard allows only the PDF extension
  origin or explicit additional origins in
  `chromium/src/components/pdf/common/pdf_util.cc:43`. The renderer helper also
  requires a parent frame with an allowed origin and a PDF renderer process in
  `chromium/src/components/pdf/renderer/internal_plugin_renderer_helpers.cc:50`.
  TermSurf uses that same helper from `ts_pdf_renderer_support.cc:160`.
- Chrome externalizes the internal PDF plugin only for allowed origins in
  `chromium/src/chrome/renderer/chrome_content_renderer_client.cc:846`. Electron
  mirrors that check in
  `vendor/electron/shell/renderer/renderer_client_base.cc:406`; TermSurf mirrors
  it in `ts_pdf_renderer_support.cc:100`.
- Electron stores PDF stream containers in `PdfViewerStreamManager` for OOPIF
  PDF loads in
  `vendor/electron/shell/browser/extensions/api/streams_private/streams_private_api.cc:72`.
  TermSurf does the same in
  `ts_plugin_response_interceptor_url_loader_throttle.cc:126`.
- Electron registers the PDF component extension from Chromium's PDF manifest in
  `vendor/electron/shell/browser/extensions/electron_extension_system.cc:109`
  and serves resource-bundle entries through
  `vendor/electron/shell/browser/extensions/electron_component_extension_resource_manager.cc:24`.
  TermSurf uses a static manifest snapshot and `kPdfResources`, but strips the
  manifest permissions more aggressively in
  `extensions/ts_pdf_component_extension.cc:87`.
- Electron exposes a broader `pdfViewerPrivate` backend, including
  `getStreamInfo`, in
  `vendor/electron/shell/browser/extensions/api/pdf_viewer_private/pdf_viewer_private_api.cc:113`.
  TermSurf currently exposes only title propagation, which is a smaller
  privilege surface.

### Findings

#### 1. Extension-scheme handling is broader than the PDF extension

- **Severity:** Medium.
- **Confidence:** High.
- **Files:**
  `chromium/src/content/libtermsurf_chromium/ts_pdf_browser_support.cc:361`,
  `ts_pdf_browser_support.cc:387`, `ts_pdf_browser_support.cc:401`.
- **Threat:** If TermSurf later loads any non-PDF extension, or if an unexpected
  extension URL reaches these hooks, the browser client treats the whole
  `chrome-extension://` scheme as handled and may apply process-per-site or
  process-map behavior beyond the PDF viewer.
- **Current guard:** `MaybeUseTsPdfProcessPerSite()` requires the site URL to be
  an extension scheme and checks the enabled extension registry.
  `MaybeActivateTsPdfSiteInstance()` also requires an enabled extension before
  process-map insertion.
- **Gap:** These checks are not restricted to the fixed PDF component extension.
  `MaybeHandleTsPdfExtensionURL()` handles every extension-scheme URL without
  checking host at all in `ts_pdf_browser_support.cc:387`. The implementation is
  safe under today's single-extension world, but the code is broader than the
  PDF security model says it should be.
- **Recommended cleanup:** Restrict all three helpers to
  `extension_misc::kPdfExtensionId` and component-extension identity. Non-PDF
  extension URLs should fall through to `ShellContentBrowserClient`.
- **Verification needed:** Add a negative automated or unit-style check that a
  URL such as `chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/index.html`
  does not produce TermSurf PDF handled-url/process-map logs, while the real PDF
  extension URL still loads.

#### 2. PDF extension API functions rely on permission wiring more than sender identity

- **Severity:** Medium.
- **Confidence:** Medium.
- **Files:**
  `chromium/src/content/libtermsurf_chromium/extensions/ts_extensions_browser_client.cc:64`,
  `extensions/ts_resources_private_api.cc:29`,
  `extensions/ts_pdf_viewer_private_api.cc:32`.
- **Threat:** If another extension ever gets `pdfViewerPrivate` or
  `resourcesPrivate`, the browser functions do not independently verify that the
  sender is the fixed PDF component extension before returning PDF strings or
  changing the PDF tab title.
- **Current guard:** The current manifest strips permissions to only
  `pdfViewerPrivate` and `resourcesPrivate`, and TermSurf currently registers
  only the PDF component extension. The title function additionally checks that
  the `WebContents` MIME type is `application/pdf` in
  `extensions/ts_pdf_viewer_private_api.cc:41`.
- **Gap:** The security boundary is implicit in extension registration and
  permission features. For a small embedder-owned extension stack, the API
  implementation should make the sender check explicit.
- **Recommended cleanup:** Add a small helper that verifies the sender extension
  id is `extension_misc::kPdfExtensionId` and the sender URL is the PDF
  component extension. Use it in both PDF extension API functions. Keep the
  existing MIME guard for title updates.
- **Verification needed:** Add a negative test/probe that attempts to call the
  APIs from a non-PDF context and confirms rejection or absence, plus a positive
  PDF viewer probe confirming strings and title propagation still work.

#### 3. Chrome resources access is granted at process scope

- **Severity:** Defense-in-depth.
- **Confidence:** High.
- **Files:**
  `chromium/src/content/libtermsurf_chromium/ts_pdf_renderer_support.cc:62`,
  `chromium/src/content/libtermsurf_chromium/ts_pdf_browser_support.cc:448`.
- **Threat:** The PDF extension process receives access to `chrome://resources`.
  If a non-PDF extension ever shares that process, it could inherit the grant.
- **Current guard:** `ShouldUseProcessPerSite()` currently forces extension
  process-per-site behavior in `ts_pdf_browser_support.cc:361`, and the grant is
  only issued inside the `IsTsPdfComponentExtension()` branch in
  `ts_pdf_browser_support.cc:434`.
- **Gap:** This is acceptable today, but it depends on the process policy
  remaining PDF-only. Finding 1's cleanup should make that invariant explicit.
- **Recommended cleanup:** Fold this into Finding 1. Add a comment at the grant
  site explaining that the grant is intentionally process-scoped and therefore
  depends on the PDF-extension-only process-per-site guard.
- **Verification needed:** Same negative non-PDF extension URL/process-map test
  as Finding 1.

#### 4. The PDF response interceptor uses CHECK for data-pipe failures

- **Severity:** Defense-in-depth.
- **Confidence:** Medium.
- **Files:**
  `chromium/src/content/libtermsurf_chromium/ts_plugin_response_interceptor_url_loader_throttle.cc:229`.
- **Threat:** An intercepted PDF response causes TermSurf to create and fill a
  Mojo data pipe for a small wrapper payload. The payload is generated by
  Chromium, not by the untrusted PDF, so this is not an exploitable input-size
  issue. However, resource exhaustion or unexpected Mojo failure would crash the
  process rather than fail the PDF load.
- **Current guard:** The payload size is small and fixed by
  `MimeHandlerViewAttachHelper::CreateTemplateMimeHandlerPage()` in
  `ts_plugin_response_interceptor_url_loader_throttle.cc:204`.
- **Gap:** Browser-process `CHECK_EQ` on an I/O primitive is harsher than needed
  for untrusted document loading.
- **Recommended cleanup:** Replace the two `CHECK_EQ` calls with graceful
  failure: log a stable `[termsurf-pdf]` error, do not mark the response as
  intercepted, and let the load fail without crashing.
- **Verification needed:** Static review is probably enough for the code path.
  If practical, add a narrow unit-style seam or forced-failure test; otherwise
  build and run the standard PDF render/scroll/title/save regression matrix.

### Non-findings

- **Internal plugin origin checks are aligned with Chromium/Electron.** TermSurf
  delegates final plugin creation to Chromium's `pdf::CreateInternalPlugin()`,
  which requires an allowed parent origin and PDF renderer process. TermSurf's
  externalization check mirrors Chrome and Electron.
- **Stream lookup is tied to Chromium's `PdfViewerStreamManager`.**
  `TsPdfStreamDelegate` looks up the stream from the parent or grandparent frame
  and verifies the stream URL, extension id, and plugin attributes before
  handing out stream info in `ts_pdf_stream_delegate.cc:84`. That is the same
  ownership model Electron uses for OOPIF PDF streams.
- **Resource serving is closed-world.** PDF extension resource bodies come from
  `kPdfResources` and `ResourceBundle`, not path-derived disk reads.
  `GetBundleResourcePath()` rejects non-PDF extension hosts in
  `extensions/ts_extensions_browser_client.cc:301`.
- **The manifest is narrower than Chrome's snapshot.** Although the static
  fallback begins from Chrome's PDF manifest shape, TermSurf strips all
  permissions except `pdfViewerPrivate` and `resourcesPrivate` before creating
  the component extension.
- **Trace file paths are local-environment controlled.** The trace and intercept
  paths are read from local process environment or renderer command-line
  switches. Web content cannot set them directly. Logs can include URLs and
  local paths, but only when explicit debug tracing is enabled.
- **`PdfHost` binding for all frames matches Chrome's shape.** TermSurf
  registers `pdf::mojom::PdfHost` for render frames in
  `ts_pdf_browser_support.cc:248`, and Chrome does the same in
  `chrome_content_browser_client_receiver_bindings.cc:543`.
- **Local `file://` and extensionless PDFs do not add a TermSurf file-access
  grant.** The local and extensionless path is still the normal Chromium PDF
  MIME path: TermSurf registers the internal PDF plugin for `application/pdf`
  and the `pdf` extension in `ts_content_client.cc:32`, then the PDF response
  path checks the committed response MIME in
  `ts_plugin_response_interceptor_url_loader_throttle.cc:169` and
  `ts_plugin_response_interceptor_url_loader_throttle.cc:181`. The stream
  delegate carries the original URL through `StreamInfo` in
  `ts_pdf_stream_delegate.cc:115`, but it does not grant file-system read/write
  privileges. The PDF component manifest starts from Chrome's snapshot with a
  `fileSystem.write` entry, but TermSurf strips that permission before extension
  creation in `extensions/ts_pdf_component_extension.cc:87`. Prior Issue 794
  verification explicitly tested `file://` PDF and `file://` extensionless
  rendering in
  `issues/0794-pdf-viewer-interactions/13-probe-save-print-title-local.md:399`
  and `issues/0794-pdf-viewer-interactions/14-complete-pdf-strings.md:372`.
  Security conclusion: the local-file surface is acceptable for this audit
  because TermSurf does not introduce a separate local-file grant; it relies on
  Chromium's normal local navigation/MIME handling plus the same PDF stream
  container path as HTTP PDFs. Experiment 4 does not need a local-file-specific
  fix, but its regression matrix should continue to include `file://` and
  extensionless local PDFs.

### Experiment 4 backlog

Required cleanup:

1. Restrict extension-scheme handling, process-per-site, and process-map
   activation to the fixed PDF component extension only.
2. Add explicit sender-extension checks to TermSurf's
   `resourcesPrivate.getStrings(PDF)` and `pdfViewerPrivate.setPdfDocumentTitle`
   implementations.

Defense-in-depth cleanup:

1. Add a short comment at the `chrome://resources` grant explaining the
   process-scoped grant and its dependency on PDF-only process policy.
2. Replace data-pipe `CHECK_EQ` calls in the PDF response interceptor with
   graceful load failure if the change is small and low-risk.

Minimum verification matrix for Experiment 4:

- Build `libtermsurf_chromium` on a fresh Chromium branch.
- Run the existing PDF toolbar/save/title/local harness.
- Run the protocol scroll, resize, and mouse harnesses.
- Run the deterministic non-PDF HTML smoke test.
- Add or extend a negative probe showing that a non-PDF extension URL/context
  cannot receive PDF extension handling, process-map activation, or PDF private
  API success.
- Confirm default production print remains not-clicked and print intercept
  remains env/switch-gated.

No separate follow-up issue is required from this audit. Native print remains
tracked by Issue 795 and was not re-scoped here.

## Conclusion

The current PDF implementation has no obvious critical or high-severity security
flaw in the TermSurf-owned PDF path. The important cleanup is to make the
single-PDF-extension assumption explicit in code rather than implicit in today's
extension registry contents. Experiment 4 should harden those boundaries and add
negative tests before the issue moves to the completeness audit track.
