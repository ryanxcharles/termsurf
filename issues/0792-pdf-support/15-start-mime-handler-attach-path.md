# Experiment 15: Start the MIME-Handler Attach Path

## Description

Experiment 14 proved the browser-side PDF stream entry now exists: direct
`application/pdf` navigation fires the PDF throttles, maps the PDF MIME type to
the PDF component extension, creates a `TransferrableURLLoader`, and stores a
`StreamContainer` in `pdf::PdfViewerStreamManager`.

The remaining failure is on the viewer side. The stream is stored as an
unclaimed stream keyed by the navigating frame tree node, but no MimeHandlerView
attach path is running far enough to claim it. The browser still falls through
to `ShellDownloadManagerDelegate::ChooseDownloadPath(...)`.

The next Electron/Chrome-shaped step is not to fake a claimed stream. It is to
start the canonical full-page MIME-handler attach flow:

```text
TsPluginResponseInterceptorURLLoaderThrottle
  -> MimeHandlerViewAttachHelper::OverrideBodyForInterceptedResponse(...)
  -> MimeHandlerViewEmbedder::Create(...)
  -> renderer recognizes the injected <embed internalid=...>
  -> Blink creates the named child frame for the full-page PDF wrapper
  -> PdfViewerStreamManager claims the stream during the wrapper commit
```

Experiment 14 only used
`MimeHandlerViewAttachHelper::CreateTemplateMimeHandlerPage(...)`, which returns
the wrapper HTML but does not call `CreateFullPageMimeHandlerView(...)`. That
means the browser-side observer responsible for watching the synthetic wrapper,
setting the internal id, creating the full-page MIME-handler guest, and
advancing the claim path is never created.

This experiment replaces that template-only path with
`OverrideBodyForInterceptedResponse(...)` and wires the minimal renderer hooks
Chrome uses for full-page MIME-handler attach:

- register the internal PDF plugin in `TsContentClient::AddPlugins(...)`, using
  Chrome's `ChromeContentClient::AddPlugins(...)` shape;
- bind `extensions::mojom::MimeHandlerViewContainerManager` on each
  `RenderFrame`;
- implement `ContentRendererClient::IsPluginHandledExternally(...)` enough for
  the injected full-page PDF wrapper to return `true`, so Blink creates the
  child frame that `MimeHandlerViewEmbedder` watches;
- keep `OverrideCreatePlugin(...)` behavior intact for the actual internal PDF
  plugin.

Chrome has two related renderer paths here. The full-page OOPIF PDF wrapper can
return `true` from `IsPluginHandledExternally(...)` for the internal PDF plugin
and let Blink create the named child frame. The embedded BrowserPlugin
MimeHandlerView path can call
`MimeHandlerViewContainerManager::CreateFrameContainer(...)`. Experiment 15 is
about the full-page PDF path first. If implementation proves the
BrowserPlugin/container path is required before the full-page child frame is
created, record that as the next gate instead of faking it.

TermSurf currently does not override `TsContentClient::AddPlugins(...)`, so the
internal PDF plugin registration and any renderer-side plugin-info lookup may be
the first stopping point. That is an acceptable Partial only if the logs prove
that the attach path reached the plugin-info boundary and ordinary HTML still
works.

This is still not a full guest-view port. If the attach path reaches
`MimeHandlerViewEmbedder::CreateMimeHandlerViewGuest(...)` and then fails
because `GuestViewManager`, `MimeHandlerViewGuest`, or
`MimeHandlerStreamManager` is not sufficiently wired, that is a useful Partial.
The experiment should record that exact next gate and stop there.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 14 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp14
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp15
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Switch the intercepted-response body path to the attach helper.

   In
   `content/libtermsurf_chromium/ts_plugin_response_interceptor_url_loader_throttle.cc`,
   replace the direct `CreateTemplateMimeHandlerPage(...)` call with
   `MimeHandlerViewAttachHelper::OverrideBodyForInterceptedResponse(...)`.

   The resume ordering is load-bearing:
   - keep the navigation deferred while the replacement body and stream
     container are prepared;
   - create the `TransferrableURLLoader` exactly as Experiment 14 does;
   - add the `StreamContainer` before resuming the load;
   - pass a single resume closure to `OverrideBodyForInterceptedResponse(...)`
     so `delegate_->Resume()` happens once, after
     `CreateFullPageMimeHandlerView(...)` has been posted and the stream
     container has been added.

   Do not leave a second unconditional `ResumeLoad()` post in place. Double
   resume is a failure.

   Required logs:

   ```text
   [issue-792-exp15] override-body-start frame_tree_node_id=<id> internal_id=<id> stream_id=<id> original_url=<url>
   [issue-792-exp15] stream-container-added-before-resume frame_tree_node_id=<id> internal_id=<id>
   [issue-792-exp15] override-body-resume frame_tree_node_id=<id> internal_id=<id>
   ```

3. Register the internal PDF plugin.

   In `content/libtermsurf_chromium/ts_content_client.{cc,h}`, override
   `AddPlugins(std::vector<content::WebPluginInfo>* plugins)` and add the
   internal PDF plugin entry using Chrome's `ChromeContentClient` shape:
   - name: Chromium PDF Plugin;
   - description: Built-in PDF viewer;
   - extension: `pdf`;
   - MIME type: `pdf::kInternalPluginMimeType`;
   - plugin type: `PLUGIN_TYPE_BROWSER_INTERNAL_PLUGIN`;
   - path: a TermSurf-owned constant equivalent to Chrome's internal PDF plugin
     path.

   Do not register arbitrary plugins.

   Required log:

   ```text
   [issue-792-exp15] internal-pdf-plugin-registered mime_type=<mime> path=<path>
   ```

4. Register the renderer-side container manager associated interface.

   In `content/libtermsurf_chromium/ts_content_renderer_client.cc`, add the
   renderer associated-interface binding Chrome registers:

   ```text
   extensions::mojom::MimeHandlerViewContainerManager
     -> extensions::MimeHandlerViewContainerManager::BindReceiver(render_frame)
   ```

   Use the `RenderFrame`'s associated-interface registry in the same lifecycle
   window Chrome uses during `RenderFrameCreated()`. Do not add unrelated Chrome
   renderer agents.

   Required log:

   ```text
   [issue-792-exp15] mime-handler-container-manager-bound render_frame=<ptr>
   ```

5. Add TermSurf's minimal external-plugin renderer hook.

   In `TsContentRendererClient`, override
   `IsPluginHandledExternally(RenderFrame*, const blink::WebElement&, const GURL&, const std::string&)`.

   For this experiment, handle only the full-page PDF wrapper case:
   - `mime_type` is `application/pdf` or Chromium's internal PDF plugin MIME
     type after plugin lookup if that value is available at this layer;
   - the element has the `internalid` attribute injected by
     `MimeHandlerViewAttachHelper`;
   - the frame belongs to the synthetic wrapper document created by the PDF
     response interceptor.

   When those conditions match and the internal PDF plugin is available, return
   `true` so Blink's `HTMLPlugInElement::RequestObject(...)` creates or
   redirects the named child frame. That child frame is what
   `MimeHandlerViewEmbedder::RenderFrameCreated(...)` watches.

   Do not call `CreateFrameContainer(...)` unconditionally. In Chrome's
   full-page internal PDF path, `IsPluginHandledExternally(...)` can return
   `true` before the BrowserPlugin `CreateFrameContainer(...)` path. Only call
   `CreateFrameContainer(...)` if Chromium's actual plugin lookup reports a
   BrowserPlugin MIME-handler entry rather than the internal PDF plugin. If
   plugin lookup cannot be wired cleanly in this experiment, record that as a
   Partial instead of manufacturing a `WebPluginInfo`.

   Do not make every PDF `<embed>` externally handled globally. This hook should
   be tied to the internal-id wrapper path so ordinary plugin behavior and
   non-PDF content remain untouched.

   Required logs:

   ```text
   [issue-792-exp15] is-plugin-handled-externally mime_type=<mime> url=<url> has_internal_id=<0|1> plugin_lookup=<ok|missing|disallowed> handled=<0|1>
   [issue-792-exp15] create-frame-container result=<0|1> mime_type=<mime> url=<url>
   ```

6. Verify or add the plugin-creation delegation.

   Verify `TsContentRendererClient::OverrideCreatePlugin(...)` delegates to
   `extensions_renderer_client_->OverrideCreatePlugin(...)` before falling back
   to shell plugin creation. If that delegation is missing, add it. If it is
   already present, preserve it.

   The external-plugin hook handles the wrapper/placeholder path. The actual
   internal PDF plugin should still instantiate through the renderer-client path
   already built in earlier experiments.

   Add only diagnostic logs needed to prove the two paths are distinct.

7. Confirm the guest-view dependency boundary before coding through it.

   Before implementing behavior beyond `MimeHandlerViewEmbedder::Create(...)`,
   audit the GN/link boundary:
   - `//extensions/browser` already includes `MimeHandlerViewAttachHelper`,
     `MimeHandlerViewEmbedder`, `MimeHandlerViewGuest`, and
     `MimeHandlerStreamManager` when guest view is enabled;
   - `//extensions/renderer` already includes `MimeHandlerViewContainerManager`;
   - `content/libtermsurf_chromium/BUILD.gn` already depends on both
     `//extensions/browser` and `//extensions/renderer`.

   The design assumption is that these classes can link and
   `MimeHandlerViewEmbedder` can be constructed, but the path may fail later
   when `CreateMimeHandlerViewGuest(...)` needs a fully initialized
   `GuestViewManager` or stream manager. If the build/link audit contradicts
   that assumption, record it in the result and stop rather than broadening the
   slice silently.

8. Instrument the canonical attach and claim ladder.

   Add narrow issue-tagged logs at TermSurf seams first. If direct logs in
   upstream Chromium files are the clearest way to prove progress, keep them
   log-only and issue-tagged.

   Evidence points:

   ```text
   [issue-792-exp15] mhv-embedder-create frame_tree_node_id=<id> resource_url=<url> stream_id=<id> internal_id=<id>
   [issue-792-exp15] mhv-ready-to-commit frame_tree_node_id=<id> url=<url>
   [issue-792-exp15] mhv-render-frame-created parent_matches=<0|1> url_matches=<0|1> owner_type=<type> name_matches=<0|1> ready=<0|1>
   [issue-792-exp15] mhv-create-guest frame_tree_node_id=<id> stream_id=<id>
   [issue-792-exp15] pdf-stream-claim frame_tree_node_id=<id> claimed=<0|1>
   [issue-792-exp15] pdf-extension-about-blank frame_tree_node_id=<id> embedder=<id>
   [issue-792-exp15] pdf-extension-navigate handler_url=<url>
   ```

   `pdf-extension-about-blank` should fire when the placeholder child frame
   commits its initial `about:blank`. `pdf-extension-navigate` should fire when
   `PdfViewerStreamManager` navigates that same frame tree node to the PDF
   extension handler URL.

   The most important proof is whether `pdf-stream-claim claimed=1` appears
   before `ShellDownloadManagerDelegate::ChooseDownloadPath(...)`.

9. Keep the Experiment 12 MIME-handler service diagnostic unless the stream is
   actually claimed.

   If the stream is not claimed, do not replace `BindTsMimeHandlerService()`.
   The right next experiment is still the attach/guest-view layer.

   If the stream is claimed and the extension frame reaches
   `mimeHandlerPrivate.getStreamInfo()` but the current diagnostic service still
   returns `null`, this experiment may replace only that narrow method with a
   real call into the claimed stream manager. Do not broaden into a full
   arbitrary MIME-handler implementation.

   Required log if this gate is reached:

   ```text
   [issue-792-exp15] mime-handler-get-stream-info frame_url=<url> has_stream=<0|1>
   ```

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
    - PDF extension frame binder population;
    - Experiment 14's stream-entry browser-client hooks.

11. Scope guard.

    Forbidden in this experiment:
    - calling `PdfViewerStreamManager::ClaimStreamInfoForTesting()` outside a
      test;
    - mutating `PdfViewerStreamManager` internals to fake a claimed stream;
    - creating synthetic PDF streams not backed by the intercepted response;
    - replacing TermSurf's content-shell base with app_shell or Chrome browser
      startup code;
    - implementing a broad `GuestViewManager`/`MimeHandlerStreamManager` port
      unless the attach path reaches that exact gate and the result is recorded
      as Partial first;
    - manufacturing a fake `WebPluginInfo` instead of registering/querying the
      internal PDF plugin legitimately;
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

2. Run the direct PDF navigation smoke through the committed fake GUI harness:

   ```bash
   LOG_DIR=logs/issue-792-exp15-pdf-$(date +%Y%m%d-%H%M%S)
   scripts/test-issue-792-fake-gui.py \
     http://127.0.0.1:9787/bitcoin.pdf \
     --serve-bitcoin-pdf \
     --log-dir "$LOG_DIR" \
     --seconds 12
   ```

3. Inspect the logs:

   ```bash
   rg '\\[issue-792-exp1[45]\\]|ShellDownloadManagerDelegate|mime-handler-get-stream-info|Stream has been aborted' "$LOG_DIR"
   ```

4. Verify the single-resume invariant.

   For one PDF navigation, `override-body-resume` must fire exactly once:

   ```bash
   COUNT=$(rg --no-filename -c 'override-body-resume' "$LOG_DIR" | awk '{sum += $1} END {print sum + 0}')
   test "$COUNT" -eq 1
   ```

   A count of `0` means the deferred load was never resumed. A count greater
   than `1` means the old unconditional resume path was left in place or a new
   double-resume bug was introduced.

5. Run the normal HTML smoke:

   ```bash
   LOG_DIR=logs/issue-792-exp15-html-$(date +%Y%m%d-%H%M%S)
   scripts/test-issue-792-fake-gui.py \
     http://localhost:9616/index.html \
     --log-dir "$LOG_DIR" \
     --seconds 6
   ```

   Pass condition: ordinary URL/title/loading messages still appear, and no
   `issue-792-exp15` PDF attach logs appear for HTML.

6. Classify the result.

   **Pass:** the intercepted PDF response goes through
   `OverrideBodyForInterceptedResponse(...)`; `MimeHandlerViewEmbedder` is
   created; the renderer recognizes the injected wrapper element; the full-page
   wrapper creates the expected named child frame; the stream is claimed by
   `PdfViewerStreamManager`; the PDF extension frame is navigated toward its
   handler URL; and ordinary HTML does not regress. The PDF still may not
   visibly render if the next gate is the current diagnostic
   `MimeHandlerService.GetStreamInfo()` implementation.

   **Partial:** the branch builds and advances at least one attach-path rung,
   but stops at a newly proven gate. Expected useful Partial outcomes include:
   - `OverrideBodyForInterceptedResponse(...)` runs, but the renderer never
     calls `IsPluginHandledExternally()`;
   - the renderer calls `IsPluginHandledExternally()`, but plugin info lookup is
     missing or disallowed;
   - the renderer returns `true` for the full-page wrapper, but Blink does not
     create the expected named child frame;
   - `CreateFrameContainer(...)` is required for this path and succeeds, but
     `MimeHandlerViewEmbedder` never sees the expected child frame;
   - `MimeHandlerViewEmbedder` tries to create a guest and fails because
     `GuestViewManager`, `MimeHandlerViewGuest`, or `MimeHandlerStreamManager`
     is the next missing Electron-style subsystem;
   - the stream is claimed, but the extension still receives `stream_info=null`
     from the diagnostic MIME-handler service.

   **Fail:** the branch does not build, PDF response interception from
   Experiment 14 regresses, ordinary HTML regresses, `delegate_->Resume()` is
   called twice or never, direct PDF navigation bypasses the attach-path logs
   entirely, the implementation manufactures a fake `WebPluginInfo` instead of
   registering/querying the internal PDF plugin legitimately, or the
   implementation fakes stream claim instead of using the canonical attach path.

7. Record the result in this file and update the README experiment index.

8. Ask Claude to review the implementation, verification artifacts, and result
   language. Fix real issues before proceeding to Experiment 16.

## Result

**Result:** Partial

The branch builds:

```bash
cd chromium/src
export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium
```

Result: `Build Succeeded: 6 steps`.

PDF smoke artifact:

```text
logs/issue-792-exp15-pdf-20260529-133019
```

The run proved several new rungs:

```text
[issue-792-exp15] internal-pdf-plugin-registered mime_type=application/x-google-chrome-pdf path=internal-pdf-viewer
[issue-792-exp15] override-body-start frame_tree_node_id=1 internal_id=FBB655FEF32483A8442F0BBBD35C7588 stream_id=90c20293-edfd-46a5-ba0a-be65ef4cb013 original_url=http://127.0.0.1:9787/bitcoin.pdf
[issue-792-exp15] stream-container-added-before-resume frame_tree_node_id=1 internal_id=FBB655FEF32483A8442F0BBBD35C7588
[issue-792-exp15] mhv-embedder-create frame_tree_node_id=1 resource_url=http://127.0.0.1:9787/bitcoin.pdf stream_id=90c20293-edfd-46a5-ba0a-be65ef4cb013 internal_id=FBB655FEF32483A8442F0BBBD35C7588
[issue-792-exp15] override-body-resume frame_tree_node_id=1
[issue-792-exp15] mime-handler-container-manager-bound render_frame=0x744d7c000
```

The single-resume invariant passed:

```bash
rg --no-filename -c 'override-body-resume' logs/issue-792-exp15-pdf-20260529-133019 | awk '{sum += $1} END {print sum + 0}'
```

Result: `1`.

The failure point is equally clear. The run did **not** log:

```text
[issue-792-exp15] mhv-ready-to-commit ...
[issue-792-exp15] is-plugin-handled-externally ...
[issue-792-exp15] mhv-render-frame-created ...
[issue-792-exp15] pdf-stream-claim ...
```

Instead, the same download delegate still fired after the stream container was
added:

```text
ShellDownloadManagerDelegate::ChooseDownloadPath(...)
```

That means the Experiment 15 attach helper starts, but it starts too late or on
the wrong path for the top-level PDF navigation. Creating
`MimeHandlerViewEmbedder` from the URL-loader response callback does not make it
observe the already-in-progress wrapper commit, so it never calls
`ReadyToCommitNavigation()`, never sets the renderer-side internal id at the
right time, and never reaches the renderer external-plugin hook.

HTML smoke artifact:

```text
logs/issue-792-exp15-html-20260529-133037
```

Ordinary HTML still loaded. The only Experiment 15 logs in that smoke were
process/frame setup logs:

```text
[issue-792-exp15] internal-pdf-plugin-registered ...
[issue-792-exp15] mime-handler-container-manager-bound ...
```

No PDF response interception, attach helper, or stream-container logs fired for
HTML.

## Conclusion

Experiment 15 proves that, for OOPIF PDF, the
`OverrideBodyForInterceptedResponse(...)` branch of
`MimeHandlerViewAttachHelper` is not the missing top-level PDF claim mechanism.
The helper can create `MimeHandlerViewEmbedder`, but by that point the
main-frame navigation is too far along for the embedder to observe the wrapper
`ReadyToCommitNavigation()` event and claim the stream.

This also explains why the Chrome source does not call
`OverrideBodyForInterceptedResponse(...)` for OOPIF PDF. Chrome uses
`CreateTemplateMimeHandlerPage(...)` for OOPIF PDFs and relies on
`PdfViewerStreamManager` to claim the stream during the wrapper navigation
lifecycle. TermSurf's next experiment should return to that canonical OOPIF PDF
path and identify why `PdfViewerStreamManager::ReadyToCommitNavigation()` is not
claiming the unclaimed stream after Experiment 14's `stream-container-added`
event.

The most likely next target is lifecycle ordering: either the stream manager is
created after the relevant `ReadyToCommitNavigation()` notification has already
passed, or the wrapper navigation observed by `PdfViewerStreamManager` is not
the same frame tree node used in `AddStreamContainer(...)`. Experiment 16 should
revert this experiment's `OverrideBodyForInterceptedResponse(...)` change in
`ts_plugin_response_interceptor_url_loader_throttle.cc`, return to
`CreateTemplateMimeHandlerPage(...)` to match Chrome's OOPIF PDF branch, and
instrument `PdfViewerStreamManager`'s `DidStartNavigation()`,
`ReadyToCommitNavigation()`, `AddStreamContainer()`, and `RenderFrameDeleted()`
around the frame tree node and committed URL, without trying another
attach-helper route.
