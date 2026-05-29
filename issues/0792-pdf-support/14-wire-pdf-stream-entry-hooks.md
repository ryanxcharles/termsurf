# Experiment 14: Wire PDF Stream Entry Hooks

## Description

Experiment 13 proved the PDF extension frame now gets Chromium's canonical
extension-frame binders. The direct PDF extension smoke no longer dies on
`extensions.KeepAlive`; it reaches Experiment 12's diagnostic
`MimeHandlerService.GetStreamInfo()` implementation, which intentionally returns
`null`:

```text
[issue-792-exp12] mime-handler-get-stream-info ... stream_info=null
Unchecked runtime.lastError: Stream has been aborted.
```

That is the expected next gate. The PDF viewer is alive enough to ask for its
stream, but TermSurf has not yet wired the browser-side PDF response path that
creates and stores that stream.

Chrome and Electron both enter the stream path through three browser-client
hooks:

- `CreateURLLoaderThrottles()` adds
  `PluginResponseInterceptorURLLoaderThrottle`, which intercepts
  `application/pdf` responses and creates a `TransferrableURLLoader`.
- `CreateThrottlesForNavigation()` adds `pdf::PdfNavigationThrottle`, which maps
  PDF stream navigations back to the original PDF URL.
- `WillCreateURLLoaderRequestInterceptors()` adds
  `pdf::PdfURLLoaderRequestInterceptor`, which serves the plugin wrapper
  response from `PdfStreamDelegate::GetStreamInfo()`.

TermSurf currently overrides none of these hooks in `TsBrowserClient`. That
means direct `application/pdf` navigation still follows content_shell's download
path, and the PDF extension viewer has no stream to retrieve.

Experiment 14 wires these entry hooks into `TsBrowserClient` using the
Chrome/Electron shape, then records exactly how far the canonical stream path
gets. The implementation should follow Electron's embedder approach: reuse
allowed `//components/pdf/browser` primitives, but add TermSurf-owned glue where
Chrome's browser layer assumes `Profile`, Chrome prefs, or
`chrome/browser/plugins` helpers that content_shell does not have.

In particular, do not add broad `//chrome/browser/pdf` or
`//chrome/browser/plugins` dependencies just to reuse Chrome's concrete browser
classes. Electron patches the plugin response interceptor to call Electron
helpers instead of Chrome's profile-backed helpers; TermSurf should use the same
idea with TermSurf-owned helpers.

This experiment should not replace the current diagnostic MIME-handler binders
with a full service yet, and it should not implement
guest-view/container-manager postMessage plumbing. Without the
guest-view/container-manager claim step, the highest expected result for this
slice may be a registered `StreamContainer` that is not yet claimed. That is
still useful progress and should be recorded as the next gate, not expanded
in-place.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 13 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp13
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp14
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Add stream-entry hook declarations to `TsBrowserClient`.

   In `content/libtermsurf_chromium/ts_browser_client.h`, override the same
   browser-client hooks Chrome and Electron use:

   ```text
   CreateThrottlesForNavigation(...)
   CreateURLLoaderThrottles(...)
   WillCreateURLLoaderRequestInterceptors(...)
   GetPluginMimeTypesWithExternalHandlers(...)
   ```

   Keep the signatures exactly aligned with Chromium 148's
   `content::ContentBrowserClient`.

3. Add TermSurf-owned stream helper classes.

   Add small TermSurf-owned classes modeled on the Chrome/Electron sources, all
   under `content/libtermsurf_chromium/`:

   ```text
   ts_pdf_stream_delegate.{cc,h}
   ts_plugin_response_interceptor_url_loader_throttle.{cc,h}
   ts_plugin_utils.{cc,h}
   ts_pdf_iframe_navigation_throttle.{cc,h}
   ```

   `TsPdfStreamDelegate` implements `pdf::PdfStreamDelegate` from
   `//components/pdf/browser`. It should model Chrome's
   `ChromePdfStreamDelegate` where the logic is generic, but replace
   `Profile`/Chrome-pref reads with TermSurf defaults. The initial defaults:
   - `use_skia`: use the Chromium feature default;
   - `allow_xfa_forms`: use the Chromium feature default;
   - `allow_javascript`, `full_frame`, background color, stream URL, original
     URL, COEP header, and injected wrapper script: derive from the
     `StreamContainer` just as Chrome does.

   `TsPluginResponseInterceptorURLLoaderThrottle` should model Chrome's
   `PluginResponseInterceptorURLLoaderThrottle` plus Electron's patch shape:
   call TermSurf's plugin utility and TermSurf's stream registration helper
   instead of Chrome's `PluginUtils` and Chrome's
   `extensions::mime_handlers::SendExecuteMimeTypeHandlerEvent` dependency.

   `TsPluginUtils` should be the Electron-style minimal MIME-type helper. It
   reads the enabled PDF component extension from `ExtensionRegistry` and its
   `MimeTypesHandler`, and maps only PDF MIME types such as `application/pdf` to
   `extension_misc::kPdfExtensionId`. Do not implement the legacy
   `MimeHandlerStreamManager` path for arbitrary extensions in this experiment.

   `TsPdfIframeNavigationThrottle` should be a minimal copy/adaptation of
   Chrome's iframe PDF throttle only if needed for embedded PDF frames. It must
   avoid Chrome plugin-service/profile dependencies. If the direct full-page PDF
   path does not require it yet, the implementation may add only the hook and
   log that iframe handling is deferred.

   Mine the parked Issue 790 Experiment 6 branch for prior TermSurf stream
   wiring where it helps, but adapt any old navigation-throttle code to Chromium
   148's `NavigationThrottleRegistry&` API.

4. Add the PDF navigation throttle.

   In `TsBrowserClient::CreateThrottlesForNavigation()`, call
   `ShellContentBrowserClient::CreateThrottlesForNavigation(registry)` first,
   then add:

   ```text
   TsPdfIframeNavigationThrottle or a deferred iframe log
   pdf::PdfNavigationThrottle(..., TsPdfStreamDelegate)
   ```

   Follow the Chrome/Electron ordering: iframe handling first, PDF navigation
   throttle second. Use the modern Chromium 148 `NavigationThrottleRegistry&`
   signature, not the older vector-return shape used by some parked PDF
   branches.

   Required log:

   ```text
   [issue-792-exp14] navigation-throttles frame_tree_node_id=<id> url=<url> pdf_throttle=1 iframe_throttle=1
   ```

5. Add the plugin response interceptor throttle.

   In `TsBrowserClient::CreateURLLoaderThrottles()`, start from the base shell
   throttles and append:

   ```text
   TsPluginResponseInterceptorURLLoaderThrottle(request.destination,
                                                frame_tree_node_id)
   ```

   This throttle should handle only the PDF extension case. It should intercept
   `application/pdf`, create the `TransferrableURLLoader`, and add the
   `StreamContainer` to `PdfViewerStreamManager`. It should not implement
   arbitrary legacy MIME-handler extension support.

   Required logs:

   ```text
   [issue-792-exp14] url-loader-throttle destination=<destination> frame_tree_node_id=<id> plugin_interceptor=1
   [issue-792-exp14] plugin-mime-map mime_type=application/pdf extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai
   ```

6. Add the PDF URL loader request interceptor.

   In `TsBrowserClient::WillCreateURLLoaderRequestInterceptors()`, append:

   ```text
   pdf::PdfURLLoaderRequestInterceptor::MaybeCreateInterceptor(
       frame_tree_node_id, std::make_unique<TsPdfStreamDelegate>())
   ```

   Required log:

   ```text
   [issue-792-exp14] pdf-request-interceptor frame_tree_node_id=<id> created=<0|1>
   ```

7. Expose plugin MIME types with external handlers.

   In `TsBrowserClient::GetPluginMimeTypesWithExternalHandlers()`, return the
   PDF extension MIME type(s) exposed by the enabled PDF component extension's
   `MimeTypesHandler`. At minimum, `application/pdf` must be present when the
   PDF component extension is registered and enabled.

   Required log:

   ```text
   [issue-792-exp14] external-plugin-mime-types count=<n> has_pdf=<0|1>
   ```

8. Instrument stream creation and claiming without changing ownership.

   Add narrow logs around the existing canonical stream path, preferably at
   TermSurf-owned seams. If direct edits to allowed `//components/pdf/browser`
   files are the cleanest way to prove the path, keep them log-only and
   issue-gated.

   Required evidence points:

   ```text
   [issue-792-exp14] plugin-response-intercept mime_type=application/pdf extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai frame_tree_node_id=<id>
   [issue-792-exp14] stream-container-added frame_tree_node_id=<id> internal_id=<id> original_url=<url> handler_url=<url>
   [issue-792-exp14] stream-container-claimed frame_tree_node_id=<id> original_url=<url>
   [issue-792-exp14] pdf-map-to-original stream_url=<url> original_url=<url> success=<0|1>
   [issue-792-exp14] pdf-get-stream-info frame_url=<url> has_stream=<0|1>
   ```

   `stream-container-claimed`, `pdf-map-to-original`, and
   `pdf-get-stream-info has_stream=1` may not fire in this slice because the
   full guest-view/container-manager claim step is explicitly out of scope.
   Their absence is an acceptable Partial outcome if the logs prove the stream
   was added and identify the missing claim layer.

9. Keep the Experiment 12 diagnostic MIME-handler service in place.

   Do not replace `BindTsMimeHandlerService()` with Chromium's
   `MimeHandlerServiceImpl` in this experiment. The PDF viewer's JavaScript API
   path and the browser-side navigation/stream path are two different halves of
   the system. This slice wires the browser-side stream entry path first.

   If the canonical stream path reaches `pdf::PdfURLLoaderRequestInterceptor`
   and `TsPdfStreamDelegate::GetStreamInfo()` with a real stream, but the
   renderer still receives `stream_info=null` from TermSurf's diagnostic
   `MimeHandlerService`, record that as the next gate. Do not quietly replace
   the service in the same experiment.

10. Preserve existing PDF foundation work.

    Do not remove or weaken:
    - PDF component extension registration;
    - PDF extension resource serving;
    - `chrome://resources` serving for the PDF viewer;
    - extension renderer activation;
    - PDF viewer private API provider wiring;
    - `PdfHelpBubbleHandlerFactory`;
    - `pdf::mojom::PdfHost`;
    - MIME-handler binders;
    - PDF extension frame binder population.

11. Keep this experiment out of the full guest-view layer.

    Forbidden in this experiment:
    - implementing a full `MimeHandlerViewGuest`;
    - implementing a full `MimeHandlerViewContainerManager`;
    - replacing the current MIME-handler diagnostic service with a production
      implementation;
    - adding synthetic or fake PDF streams that do not originate from the
      intercepted `application/pdf` response;
    - changing `webtui`, `roamium`, `termsurf.proto`, or Wezboard.

12. Build and archive only after an accepted result.

    Build:

    ```bash
    cd chromium/src
    export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
    autoninja -C out/Default libtermsurf_chromium
    ```

    If the branch reaches a coherent Pass or Partial, commit the Chromium
    branch, regenerate `chromium/patches/issue-792/`, update
    `chromium/README.md`, and commit those main-repo changes. If it fails in a
    way that leaves the branch incoherent, record the failure in this file and
    do not archive that branch.

## Verification

1. Build `libtermsurf_chromium` with `autoninja`.

2. Run the existing automated HTML smoke to confirm ordinary pages still load:

   ```bash
   LOG_DIR=logs/issue-792-exp14-html-$(date +%Y%m%d-%H%M%S) \
   TERMSURF_PDF_SETTLE_SECONDS=8 \
   scripts/test-issue-776-pdf.sh http://localhost:9616/index.html
   ```

   Pass condition: the screenshot shows the HTML fixture, and logs still include
   normal `UrlChanged`, `TitleChanged`, and `LoadingState` messages.

3. Run the direct PDF navigation smoke:

   ```bash
   LOG_DIR=logs/issue-792-exp14-pdf-$(date +%Y%m%d-%H%M%S) \
   TERMSURF_PDF_SETTLE_SECONDS=8 \
   scripts/test-issue-776-pdf.sh http://localhost:9616/bitcoin.pdf
   ```

4. Inspect the logs for the stream-entry ladder:

   ```bash
   rg '\\[issue-792-exp14\\]|mime-handler-get-stream-info|Stream has been aborted|ShellDownloadManagerDelegate' logs/issue-792-exp14-*
   ```

5. Classify the result.

   **Pass:** direct PDF navigation no longer follows the content_shell download
   path; the `application/pdf` response is intercepted; the MIME type maps to
   the PDF component extension; a `StreamContainer` is added to
   `PdfViewerStreamManager` with the original URL and handler URL populated; and
   no new regression appears in the HTML smoke. The stream may still remain
   unclaimed because the full guest-view/container-manager layer is out of
   scope.

   **Partial:** at least one of the new hooks fires and the logs identify the
   next missing layer, but the full stream-entry ladder is not complete. Example
   acceptable Partial outcomes:
   - the plugin response interceptor fires, but MIME type lookup does not map
     `application/pdf` to the PDF extension;
   - `StreamContainer` is added, but `PdfViewerStreamManager` never claims it;
   - the stream is claimed, but `PdfNavigationThrottle` never maps the stream
     URL to the original URL;
   - `TsPdfStreamDelegate::GetStreamInfo()` has a stream, but the diagnostic
     `MimeHandlerService.GetStreamInfo()` still returns `null`, proving the next
     experiment must replace the diagnostic service.

   **Fail:** the branch does not build, ordinary HTML regresses, direct PDF
   navigation still goes straight to `ShellDownloadManagerDelegate` with none of
   the new hooks firing, or the implementation uses synthetic streams or broad
   guest-view code in violation of the scope.

6. After recording the result, ask Claude to review the implementation,
   verification artifacts, and result language. Fix real issues before
   proceeding to Experiment 15.

## Result

**Result:** Partial

The branch builds and the new browser-side PDF stream entry hooks fire. Build
verification:

```bash
cd chromium/src
export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium
```

Result: `Build Succeeded: 0 steps`.

The old screenshot smoke script was not reliable for this slice because it
created a Wezboard process with no pane, so the `web` TUI did not get a real
terminal environment and Roamium did not start. For this experiment, the useful
proof is the browser-side stream ladder, so I used a minimal fake-GUI Unix
socket harness that launches repo-built Roamium directly, sends `CreateTab`,
serves `bitcoin.pdf` with `Content-Type: application/pdf`, and records Roamium's
logs. After this result, that harness was committed as
`scripts/test-issue-792-fake-gui.py` so future PDF stream experiments can re-run
the same browser-side proof without depending on screenshot automation. The
committed harness was smoke-tested with the same PDF URL and reproduced the same
`stream-container-added` / download-delegate sequence in
`logs/issue-792-exp14-script-pdf-20260529-130937`.

PDF artifact:

```text
logs/issue-792-exp14-fakegui-20260529-130155
```

The fake GUI saw Roamium register, accepted a tab, and served the PDF:

```text
t=0.000 top_field=12 size=11
sent CreateTab
t=0.123 top_field=13 size=15
tab_ready id=1
sent Resize
t=0.396 top_field=16 size=13
t=0.480 top_field=14 size=16
"GET /bitcoin.pdf HTTP/1.1" 200 -
```

The Experiment 14 stream-entry ladder reached the intended new browser-side
state:

```text
[issue-792-exp14] navigation-throttles frame_tree_node_id=1 url=http://127.0.0.1:9787/bitcoin.pdf pdf_throttle=1 iframe_throttle=0
[issue-792-exp14] pdf-request-interceptor frame_tree_node_id=1 created=1
[issue-792-exp14] pdf-get-stream-info frame_url= has_stream=0
[issue-792-exp14] url-loader-throttle destination=3 frame_tree_node_id=1 plugin_interceptor=1
[issue-792-exp14] plugin-mime-map mime_type=application/pdf extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai
[issue-792-exp14] plugin-response-intercept mime_type=application/pdf extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai frame_tree_node_id=1
[issue-792-exp14] stream-container-added frame_tree_node_id=1 internal_id=BCB00BFB6DB2A370E2A49E4090446BA1 original_url=http://127.0.0.1:9787/bitcoin.pdf handler_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html
```

After the stream container was added, the navigation still reached
content_shell's download delegate:

```text
ShellDownloadManagerDelegate::ChooseDownloadPath(...)
```

That means Experiment 14 succeeded at wiring the response-interception and
stream-container creation half, but the stream is not yet claimed by a
viewer-side guest/MimeHandlerView container. This is the next missing layer.

HTML smoke artifact:

```text
logs/issue-792-exp14-fakegui-html-20260529-130300
```

The same harness loaded `http://localhost:9616/index.html`. It produced normal
tab, URL, title, and loading messages:

```text
t=0.000 top=12
sent CreateTab
t=0.129 top=13
t=0.446 top=14
t=0.621 top=15
t=0.621 top=32
t=0.623 top=16
t=0.634 top=17
t=0.672 top=16
```

The HTML smoke did not log `plugin-response-intercept` or
`stream-container-added`, so ordinary HTML was not converted into the PDF stream
path.

One designed log did not fire in either smoke:
`[issue-792-exp14] external-plugin-mime-types ...`. The override is present, but
content_shell does not appear to query
`GetPluginMimeTypesWithExternalHandlers()` without the MimeHandlerView/guest
layer. That log should become useful once the next experiment wires the
viewer-side claim path.

### Design Exception: `PdfViewerStreamManager`

This experiment intentionally compiles two narrow Chrome source files into
`libtermsurf_chromium`:

```text
//chrome/browser/pdf/pdf_viewer_stream_manager.cc
//chrome/browser/pdf/pdf_viewer_stream_manager.h
```

This is a contained exception to the usual "do not pull Chrome browser code"
rule. The reason is structural: the allowed canonical component
`pdf::PdfNavigationThrottle` hard-couples to
`pdf::PdfViewerStreamManager::FromWebContents()`. To use Chromium's standard PDF
navigation throttle, the linker needs `PdfViewerStreamManager`'s implementation.

The alternative would have been to fork both `pdf::PdfNavigationThrottle` and
`PdfViewerStreamManager` into TermSurf-owned `Ts*` copies. That would create a
larger code surface and a higher upgrade burden, because both copies would need
to be kept in sync with Chromium's PDF navigation semantics. The narrow source
inclusion keeps the exception to one class and preserves Chromium's canonical
stream-entry path.

Longer term, keep this narrow inclusion unless Chromium's PDF stream
architecture changes or a later experiment proves a TermSurf-owned manager is
necessary.

## Conclusion

Experiment 14 added the correct stream-entry hooks to `TsBrowserClient` and
proved the direct `application/pdf` response can now be intercepted and stored
as a `StreamContainer` for the PDF extension. The experiment did not make PDFs
render yet, and that is expected for this slice.

The next experiment should wire the viewer-side claim path: the PDF extension
wrapper/guest side needs to claim the `PdfViewerStreamManager` stream by
`frame_tree_node_id`/internal id, then connect that claimed stream to the
existing `mimeHandlerPrivate.getStreamInfo()` path. Until that exists,
content_shell still treats the top-level PDF as a download even though TermSurf
has created the stream container.
