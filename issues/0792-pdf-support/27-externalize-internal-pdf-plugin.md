# Experiment 27: Externalize Internal PDF Plugin

## Description

Experiment 26 proved that `TsContentRendererClient::OverrideCreatePlugin()` now
sees the PDF viewer's internal plugin embed, but it sees it in the wrong
renderer process:

```text
[issue-792-exp15] is-plugin-handled-externally mime_type=application/x-google-chrome-pdf ... plugin_lookup=missing handled=0
[issue-792-exp26] internal-plugin-create-check document_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html mime_type=application/x-google-chrome-pdf ... has_pdf_renderer=0
[issue-792-exp26] internal-plugin-create-skipped reason=missing-pdf-renderer
```

That is not the path Chrome and Electron take. In both embedders, the internal
PDF plugin MIME type is treated as externally handled when it appears inside an
origin allowed to embed the internal PDF plugin. Electron's renderer client does
this before falling back to `MimeHandlerViewContainerManager`:

```cpp
if (plugin_info->actual_mime_type == pdf::kInternalPluginMimeType) {
  if (IsPdfInternalPluginAllowedOrigin(
          render_frame->GetWebFrame()->GetSecurityOrigin(), {})) {
    return true;
  }
}
```

TermSurf currently requires an `internalid` attribute before it returns
`handled=1`. The generated internal plugin wrapper from
`components/pdf/browser/plugin_response_writer.cc` does not provide that
attribute, so TermSurf incorrectly returns `handled=0`; Blink then tries
`OverrideCreatePlugin()` directly in the PDF extension renderer, where
`pdf::IsPdfRenderer()` is false.

Experiment 27 ports the Electron/Chrome behavior for the internal PDF MIME: when
`IsPluginHandledExternally()` is called for `application/x-google-chrome-pdf`
from an allowed PDF viewer origin, return `true` even without `internalid`. The
expected consequence is that Chromium continues down the normal PDF
content-frame path, where `pdf::PdfNavigationThrottle` maps the stream URL to
the original PDF URL with `params.is_pdf = true`. That should cause Chromium's
existing `RenderProcessHostImpl::IsPdf()` path to append `--pdf-renderer`
naturally, instead of TermSurf manually forcing the switch onto a renderer.

This is deliberately narrower than Electron's full plugin-info path. Electron
asks the browser process for the resolved `actual_mime_type`, then applies the
internal-PDF check. Experiment 27 only handles the current PDF viewer wrapper
case where the embed MIME is already `application/x-google-chrome-pdf`.
Arbitrary user HTML such as `<embed type="application/pdf">` can be revisited
later if needed.

The verification uses the existing Experiment 16
`PdfViewerStreamManager::DidStartNavigation()` log. That log already records
`is_pdf=` from `navigation_handle->IsPdf()`, so no extra stream-manager
instrumentation is required in this experiment.

This experiment must receive Claude design review before it runs. After the
result is recorded, Claude must review the completed output before any cleanup,
closure, or next experiment.

## Changes

1. Create a new Chromium branch from Experiment 26.

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-792-exp26
   git checkout -b 148.0.7778.97-issue-792-exp27
   ```

   Add the branch to `chromium/README.md`.

2. Update `content/libtermsurf_chromium/ts_content_renderer_client.cc`.

   Add the include needed for `IsPdfInternalPluginAllowedOrigin()`:

   ```cpp
   #include "components/pdf/common/pdf_util.h"
   ```

   In `TsContentRendererClient::IsPluginHandledExternally()`, before the current
   `internalid`-based handling, add a special case for
   `mime_type == pdf::kInternalPluginMimeType`.

   Required behavior:
   - compute whether the current frame origin is allowed with
     `IsPdfInternalPluginAllowedOrigin(render_frame->GetWebFrame()->GetSecurityOrigin(), {})`;
   - log `[issue-792-exp27] internal-plugin-external-check` with:
     - document URL;
     - original URL;
     - MIME type;
     - `allowed_origin=0/1`;
   - if allowed, log `[issue-792-exp27] internal-plugin-externalized handled=1`
     and return `true`;
   - if not allowed, log
     `[issue-792-exp27] internal-plugin-externalized handled=0 reason=disallowed-origin`
     and continue to the existing `internalid` logic.

3. Keep the Experiment 26 `OverrideCreatePlugin()` route in place.

   It remains the final internal plugin creation route once Chromium navigates
   the PDF content frame into a PDF renderer process. Do not remove its
   `pdf::IsPdfRenderer()` guard.

4. Do not:
   - manually append `--pdf-renderer` in `AppendExtraCommandLineSwitches`;
   - mark all PDF extension renderers as PDF renderers;
   - port Electron's full browser-side `GetPluginInfo` round trip;
   - port Electron's generic MimeHandlerView fallback for non-PDF MIME types;
   - change `PdfViewerStreamManager`;
   - change the PDF wrapper response body;
   - change Roamium Rust, Wezboard, webtui, or the TermSurf protocol.

   If this experiment does not cause a PDF content-frame navigation with
   `is_pdf=1`, record a Partial and design the next experiment around the
   precise missing navigation/process-selection layer. Do not hide that failure
   by forcing the command-line switch globally.

5. Build Chromium:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

6. Regenerate the Issue 792 Chromium patch archive only after the Chromium
   branch commit:

   ```bash
   cd chromium/src
   rm -rf ../../chromium/patches/issue-792/
   git format-patch 148.0.7778.97..HEAD -o ../../chromium/patches/issue-792/
   ```

## Verification

1. Run the fake-GUI stream-info preflight:

   ```bash
   LOG_DIR="logs/issue-792-exp27-fakegui-$(date +%Y%m%d-%H%M%S)"
   scripts/test-issue-792-fake-gui.py \
     http://127.0.0.1:9787/bitcoin.pdf \
     --serve-bitcoin-pdf \
     --log-dir "$LOG_DIR" \
     --seconds 18
   ```

   Required:

   ```text
   real-mime-handler-get-stream-info has_stream=1
   [issue-792-exp27] internal-plugin-external-check ... allowed_origin=1
   [issue-792-exp27] internal-plugin-externalized handled=1
   ```

2. Run the real-GUI DevTools HTML sanity check:

   ```bash
   TERMSURF_PDF_SETTLE_SECONDS=8 \
   LOG_DIR="logs/issue-792-exp27-html-devtools-$(date +%Y%m%d-%H%M%S)" \
   scripts/test-issue-792-devtools-screenshot.sh https://example.com
   ```

   The DevTools screenshot must show rendered `example.com`.

3. Run the real-GUI PDF DevTools capture:

   ```bash
   TERMSURF_PDF_SETTLE_SECONDS=18 \
   LOG_DIR="logs/issue-792-exp27-pdf-devtools-$(date +%Y%m%d-%H%M%S)" \
   scripts/test-issue-792-devtools-screenshot.sh http://localhost:9616/bitcoin.pdf
   ```

4. Inspect the PDF DevTools PNG with `view_image`.

   Classify it as:
   - **Rendered PDF:** recognizable Bitcoin PDF content is visible.
   - **Plugin fallback:** "Couldn't load plugin" still appears.
   - **PDF renderer crash:** Chromium reaches the PDF renderer path but crashes.
   - **Wrong target:** DevTools captured the wrong page.
   - **Automation failure:** no reliable DevTools PNG was produced.

5. Inspect PDF logs.

   Required for Pass:

   ```text
   real-mime-handler-get-stream-info has_stream=1
   [issue-792-exp27] internal-plugin-externalized handled=1
   [issue-792-exp16] pvs-start ... is_pdf=1
   [issue-792-exp26] internal-plugin-create-check ... has_pdf_renderer=1
   [issue-792-exp26] internal-plugin-create-result created=1
   ```

   The log should not contain the Experiment 26 skip as the decisive terminal
   state:

   ```text
   [issue-792-exp26] internal-plugin-create-skipped reason=missing-pdf-renderer
   ```

6. Record the result in this file.

   Include:
   - Chromium branch name and commit;
   - build command and result;
   - fake-GUI log directory and stream-info result;
   - HTML DevTools screenshot path and classification;
   - PDF DevTools screenshot path and classification;
   - whether `IsPluginHandledExternally()` externalized the internal PDF MIME;
   - whether `PdfNavigationThrottle` produced a PDF content-frame navigation
     with `is_pdf=1`;
   - whether `pdf::IsPdfRenderer()` returned true;
   - whether `pdf::CreateInternalPlugin()` returned a plugin;
   - Pass/Partial/Fail status;
   - next action.

## Pass Criteria

Experiment 27 passes only if:

- Chromium builds;
- fake-GUI stream-info preflight passes;
- HTML DevTools sanity capture passes;
- internal PDF MIME externalization logs `handled=1`;
- PDF logs show the downstream PDF content-frame / PDF renderer path reached;
- `pdf::CreateInternalPlugin()` returns a non-null plugin;
- the PDF DevTools screenshot shows recognizable Bitcoin PDF content;
- logs do not contradict the run.

## Partial Criteria

Experiment 27 is partial if:

- stream-info remains healthy;
- the internal PDF MIME externalization route runs;
- but the PDF content-frame navigation does not appear, `pdf::IsPdfRenderer()`
  remains false, `pdf::CreateInternalPlugin()` returns null, or the screenshot
  still shows "Couldn't load plugin."

In that case, the next experiment should target the first missing downstream
layer shown by the logs.

## Failure Criteria

Experiment 27 fails if:

- Chromium does not build;
- the patch forces `--pdf-renderer` globally or onto the PDF extension renderer
  instead of preserving Chromium's PDF content-frame process model;
- the fake-GUI or real-GUI stream-info chain regresses;
- HTML DevTools sanity capture fails;
- ordinary non-PDF HTML plugin/mime behavior changes;
- the renderer crashes before producing useful logs;
- the run uses an installed/stable Roamium instead of the repo-built binary.

## Result

**Result:** Partial

Chromium branch: `148.0.7778.97-issue-792-exp27`

Chromium commit: `d3608ed6f5e6d` (`Externalize PDF plugin embeds`)

Patch archive: regenerated at `chromium/patches/issue-792/`.

Build:

```bash
cd chromium/src
export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium
```

Result: success after correcting the design/API name from
`pdf::IsPdfInternalPluginAllowedOrigin()` to the actual Chromium 148 symbol,
`IsPdfInternalPluginAllowedOrigin()`.

Fake-GUI preflight:

- Log directory: `logs/issue-792-exp27-fakegui-20260529-165715`
- Result: pass for all plumbing gates.

Relevant log:

```text
[issue-792-exp18] real-mime-handler-get-stream-info has_stream=1 ... original_url=http://127.0.0.1:9787/bitcoin.pdf
[issue-792-exp27] internal-plugin-external-check document_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html ... mime_type=application/x-google-chrome-pdf allowed_origin=1
[issue-792-exp27] internal-plugin-externalized handled=1
[issue-792-exp14] pdf-map-to-original stream_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/a96d36d6-1bd7-430c-b16a-d946735021f4 original_url=http://127.0.0.1:9787/bitcoin.pdf success=1
[issue-792-exp16] pvs-start frame_tree_node_id=3 url=http://127.0.0.1:9787/bitcoin.pdf is_pdf=1 ...
[issue-792-exp26] internal-plugin-create-check document_url=http://127.0.0.1:9787/bitcoin.pdf mime_type=application/x-google-chrome-pdf ... has_pdf_renderer=1
[issue-792-exp26] internal-plugin-create-result created=1
```

HTML DevTools sanity:

- Log directory: `logs/issue-792-exp27-html-devtools-20260529-165747`
- Screenshot:
  `logs/issue-792-exp27-html-devtools-20260529-165747/devtools-smoke.png`
- Result: pass. The screenshot showed the rendered Example Domain page.

PDF DevTools capture:

- Log directory: `logs/issue-792-exp27-pdf-devtools-20260529-165800`
- Screenshot:
  `logs/issue-792-exp27-pdf-devtools-20260529-165800/devtools-smoke.png`
- Result: PDF renderer crash / gray plugin rectangle. The screenshot no longer
  showed `Couldn't load plugin`, but it did not show recognizable Bitcoin PDF
  content.

Relevant PDF logs:

```text
[issue-792-exp27] internal-plugin-external-check document_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html ... allowed_origin=1
[issue-792-exp27] internal-plugin-externalized handled=1
[issue-792-exp14] pdf-map-to-original stream_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/0e4641dc-c998-4b74-83a8-53b6464f3c61 original_url=http://localhost:9616/bitcoin.pdf success=1
[issue-792-exp16] pvs-start frame_tree_node_id=3 url=http://localhost:9616/bitcoin.pdf is_pdf=1 ...
[issue-792-exp26] internal-plugin-create-check document_url=http://localhost:9616/bitcoin.pdf mime_type=application/x-google-chrome-pdf ... has_pdf_renderer=1
[issue-792-exp26] internal-plugin-create-result created=1
```

Then the PDF renderer crashed after the PDF document finished loading:

```text
FATAL:ui/base/resource/resource_bundle.cc:1284] Check failed: !data->empty(). Unable to find resource: 38115.
...
chrome_pdf::FormatPageSize(...)
chrome_pdf::PdfViewWebPlugin::SendMetadata()
chrome_pdf::PdfViewWebPlugin::DocumentLoadComplete()
chrome_pdf::PDFiumEngine::FinishLoadingDocument()
...
Received signal 6
```

The original Experiment 26 terminal failure,
`internal-plugin-create-skipped reason=missing-pdf-renderer`, is gone. The
renderer process is now a PDF renderer (`has_pdf_renderer=1`), the internal PDF
plugin is created (`created=1`), and PDFium reaches document-load completion.
The remaining failure is a downstream missing localized resource used while
sending PDF metadata, specifically the page-size string path in
`pdf/ui/document_properties.cc`.

## Conclusion

Experiment 27 successfully reproduced the core Electron/Chrome PDF flow:
externalize the trusted internal PDF MIME, map the stream URL to the original
PDF URL, create a PDF renderer process, and instantiate the internal PDF plugin
inside that process.

The issue is now past the plugin/process-model gate. The next experiment should
load or otherwise provide the PDF localized strings needed by
`chrome_pdf::FormatPageSize()`, starting with the missing resource `38115` from
the `IDS_PDF_PROPERTIES_PAGE_SIZE_*` family in `components/pdf_strings.grdp`.
