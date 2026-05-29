# Experiment 7: Serve Chrome Resources To The PDF Viewer

## Description

Experiment 6 proved the PDF viewer shell is now running as a recognized
extension renderer process. The viewer advanced past the previous
`main.js`/`pdf_viewer_wrapper.js` CSP failures and began loading its own
extension scripts. The next blocker is shared WebUI resource access:

```text
Not allowed to load local resource: chrome://resources/css/text_defaults_md.css
Not allowed to load local resource: chrome://resources/js/assert.js
Not allowed to load local resource: chrome://resources/lit/v3_0/lit.rollup.js
Not allowed to load local resource: chrome://resources/js/load_time_data.js
Not allowed to load local resource: chrome://resources/mojo/mojo/public/js/bindings.js
```

Electron's
`ElectronBrowserClient::RegisterNonNetworkSubresourceURLLoaderFactories()` adds
a `chrome://resources` WebUI URL-loader factory for component extensions. Chrome
does the same in `ChromeContentBrowserClient` and also grants the extension
renderer process request access to the `chrome://resources/` origin in
`ChromeExtensionWebContentsObserver::SetUpRenderFrameHost()`.

This experiment copies that minimal embedder substrate into TermSurf:

1. grant the PDF component extension renderer process access to
   `chrome://resources/`;
2. register a `chrome://resources` subresource URL-loader factory for the PDF
   component extension frame.

It does not wire PDF navigation, `PdfNavigationThrottle`,
`PdfViewerStreamManager`, guest-view, MimeHandlerView, `--pdf-renderer`, PDF
viewer private APIs, stream handoff, or broad Chrome browser UI stacks.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 6 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp6
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp7
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Add a PDF component-extension predicate.

   Add a small local helper in the narrowest appropriate file, likely
   `ts_browser_client.cc` and mirrored in `ts_extensions_browser_client.cc` if
   needed:
   - true only when the extension id is `mhjfbmdgcfjbbpaeojofohoefgiehjai`;
   - true only when
     `extensions::Manifest::IsComponentLocation(extension->location())` is true;
   - false for every other extension, hosted app, normal page, worker context,
     and browser context.

   The PDF id check keeps this experiment narrower than Chrome's general
   component-extension path. A later cleanup can generalize the helper if
   TermSurf needs more component extensions. This is intentionally stricter than
   Electron's `IsComponentLocation()`-only predicate so no future non-PDF
   component extension accidentally receives `chrome://resources` access from
   this PDF-specific slice.

3. Grant `chrome://resources/` request access to the PDF extension renderer.

   Extend Chromium 148's per-`RenderFrameHost` initialization hook on
   `extensions::ExtensionWebContentsObserver`, in
   `content/libtermsurf_chromium/extensions/ts_extensions_browser_client.cc`.
   Verify the exact override point against
   `extensions/browser/extension_web_contents_observer.h` before coding. In
   Chrome this pattern lives in
   `ChromeExtensionWebContentsObserver::SetUpRenderFrameHost()`.

   Pattern:
   - call
     `ExtensionWebContentsObserver::SetUpRenderFrameHost(render_frame_host)`
     first to preserve the base extension setup;
   - get the extension with `GetExtensionFromFrame(render_frame_host, false)`;
   - if the extension is the PDF component extension, call:

     ```cpp
     content::ChildProcessSecurityPolicy::GetInstance()->GrantRequestOrigin(
         process_id,
         url::Origin::CreateFromNormalizedTuple(
             content::kChromeUIScheme,
             content::kChromeUIResourcesHost,
             0));
     ```

   Construct the grant origin from the same `content::kChromeUIScheme` and
   `content::kChromeUIResourcesHost` constants used by the factory so the two
   hooks cannot drift.

   Do not grant `chrome://theme/`, `chrome://favicon/`,
   `chrome://extension-icon/`, or broad `chrome://` access in this experiment.
   The failing logs only name `chrome://resources/...`.

4. Register a `chrome://resources` subresource factory for the PDF extension
   frame.

   Extend `TsBrowserClient::RegisterNonNetworkSubresourceURLLoaderFactories()`.

   Pattern:
   - preserve the existing base call and `chrome-extension` factory from
     Experiment 5;
   - look up the `RenderFrameHost` from `render_process_id` and
     `render_frame_id`;
   - get the owning `WebContents`;
   - get TermSurf's `ExtensionWebContentsObserver`;
   - resolve the extension with `GetExtensionFromFrame(frame_host, false)`;
   - if the extension is the PDF component extension, add:

     ```cpp
     factories->emplace(
         content::kChromeUIScheme,
         content::CreateWebUIURLLoaderFactory(
             frame_host,
             content::kChromeUIScheme,
             {content::kChromeUIResourcesHost}));
     ```

   Do not register a `chrome://resources` factory for ordinary HTML pages,
   normal extension-like URLs without a matching enabled component extension,
   workers, service workers, or non-PDF extensions.

5. Add minimal diagnostics.

   Use Chromium `LOG(INFO)` lines with this exact prefix:

   ```text
   [issue-792-exp7]
   ```

   Required lines:

   ```text
   [issue-792-exp7] chrome-resources-grant extension_id=<id> process_id=<id>
   [issue-792-exp7] chrome-resources-factory extension_id=<id> process_id=<id> frame_id=<id>
   ```

   If the factory helper sees an extension frame but declines to add the
   factory, log only for the PDF extension id mismatch or non-component location
   cases. Do not log ordinary HTTP pages.

6. Do not widen the experiment.

   Forbidden in this experiment:
   - PDF navigation or MIME interception;
   - `PdfViewerStreamManager`;
   - guest-view or MimeHandlerView;
   - `--pdf-renderer`;
   - PDF viewer private API bindings;
   - stream handoff;
   - restoring `web_accessible_resources`;
   - broad Chrome browser UI/resource stacks;
   - granting general `chrome://` access.

   If the viewer advances past `chrome://resources` and then fails on Mojo,
   private APIs, localization, or stream data, record that exact new blocker and
   design the next experiment around it.

7. Build and archive only after verification.

   Build:

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp6 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

   If the branch builds and verification passes or produces a useful Partial, do
   the full bookkeeping after Claude after-review accepts the result:
   - commit the Chromium branch;
   - regenerate `chromium/patches/issue-792/`;
   - add the new branch row to `chromium/README.md`;
   - update Experiment 7's line in `issues/0792-pdf-support/README.md` from
     `Designed` to the final status.

## Verification

1. Confirm starting state.

   ```bash
   git status --short
   git -C chromium/src status --short
   git -C chromium/src branch --show-current
   ```

   Chromium should start clean on `148.0.7778.97-issue-792-exp6`.

2. Build the branch.

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp6 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

3. Run the direct extension-resource smoke.

   Reuse the debug screenshot harness against:

   ```text
   chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html
   ```

   Pass requires:
   - Experiment 6 still inserts the PDF extension process:

     ```text
     [issue-792-exp6] process-map-insert extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=<id> site_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/
     ```

   - Experiment 7 grants and serves `chrome://resources`:

     ```text
     [issue-792-exp7] chrome-resources-grant extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=<id>
     [issue-792-exp7] chrome-resources-factory extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=<id> frame_id=<id>
     ```

   - the Experiment 6 `chrome://resources/...` "Not allowed to load local
     resource" errors for the named resources are gone or materially changed;
   - no `FATAL`, `NOTREACHED`, renderer IPC crash, or hang occurs before the
     screenshot artifact is captured.

   Record the new first blocker after the resources load. Expected possibilities
   include missing Mojo JS bootstrap, missing PDF private APIs, missing
   localization data, or missing stream/document data.

4. Run normal HTML regression smoke.

   Load:

   ```text
   http://localhost:9616/index.html
   ```

   Pass requires the page to render or lifecycle logs to reach `TitleChanged`
   and `LoadingState`, with no extension IPC crash.

5. Run the PDF unchanged smoke.

   Load:

   ```text
   http://localhost:9616/bitcoin.pdf
   ```

   The PDF is still expected to take the default content_shell download path
   because this experiment does not install PDF navigation or stream handling. A
   browser crash, renderer IPC crash, or hang is a failure.

6. Run Claude review after recording the result.

   Provide Claude with the experiment file, Chromium diff, build output summary,
   runtime logs, screenshot artifact paths, and the recorded result. Fix all
   real findings before proceeding.

## Pass Criteria

- Chromium branch `148.0.7778.97-issue-792-exp7` builds `libtermsurf_chromium`.
- Direct navigation to the PDF component extension still serves `index.html` and
  its own extension JS resources from `ui::ResourceBundle`.
- The PDF extension process is still inserted into `extensions::ProcessMap`.
- The PDF component extension renderer receives explicit request access to
  `chrome://resources/`.
- The PDF component extension frame receives a `chrome://resources` WebUI
  subresource factory.
- The named Experiment 6 `chrome://resources/...` "Not allowed to load local
  resource" errors are gone or replaced by a later, different blocker.
- Normal HTML browsing still works through the debug TermSurf path.
- Loading `bitcoin.pdf` does not crash; rendering is not required.
- Claude reviews the completed result and agrees it is good enough to proceed.

## Partial Criteria

Partial if:

- the branch builds but the grant hook does not fire for the PDF extension
  frame;
- the grant hook fires but the subresource factory hook does not fire;
- both hooks fire, but `chrome://resources/...` still reports "Not allowed to
  load local resource";
- the named `chrome://resources` errors disappear, but the next blocker is Mojo,
  localization, private API, or stream data;
- direct extension loading improves, but normal HTML or PDF unchanged smoke
  exposes a regression that needs a narrow follow-up.

## Failure Criteria

- The experiment changes PDF navigation, stream handling, guest-view,
  MimeHandlerView, or `--pdf-renderer`.
- The experiment grants general `chrome://` access instead of only
  `chrome://resources/` for the PDF component extension frame.
- The experiment registers `chrome://resources` for ordinary pages or non-PDF
  extensions.
- The experiment imports broad Chrome browser UI/resource stacks.
- The experiment changes TermSurf protocol, Wezboard, Roamium Rust, or webtui.
- The experiment regresses normal HTML browsing or reintroduces the extension
  renderer IPC crash.
- The experiment proceeds without Claude design review or ignores real Claude
  findings.

## Result

**Result:** Pass

Chromium branch `148.0.7778.97-issue-792-exp7` builds `libtermsurf_chromium`
successfully after `git cl format`:

```text
Build Succeeded: 3 steps
```

The direct PDF-extension smoke advanced past the `chrome://resources` blocker.
Artifacts:

```text
logs/issue-792-exp7-extension-20260529-104251/
```

Required evidence from the log:

```text
[issue-792-exp6] process-map-insert extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=5 site_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/
[issue-792-exp7] chrome-resources-grant extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=5
[issue-792-exp7] chrome-resources-factory extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=5 frame_id=1
[issue-792-exp7] chrome-resources-renderer-origin extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai
```

The old Experiment 6 blocker is gone. The direct extension smoke no longer
prints:

```text
Not allowed to load local resource: chrome://resources/...
```

The implementation needed four pieces:

1. browser-side request access for the PDF extension process;
2. browser-side `chrome://resources` WebUI subresource factory registration for
   the PDF extension frame;
3. browser-side extension-origin access metadata for the PDF component
   extension;
4. renderer-side `WebSecurityPolicy` origin access from the PDF extension origin
   to `chrome://resources`.

The original two-hook design was necessary but incomplete: request access and
the URL-loader factory routed the requests, but Blink still rejected the URLs in
`SecurityOrigin::CanDisplay()` until the PDF extension origin was allowed to
access `chrome://resources` at the renderer security-policy layer. The renderer
allowlist is process-wide setup, but the rule is keyed to
`chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai`, so ordinary pages and
non-PDF extensions cannot use it. This is still scoped to the PDF extension id
and `chrome://resources`; it does not grant general `chrome://` access.

The new first blocker is later and different:

```text
Uncaught TypeError: Cannot read properties of undefined (reading 'SaveRequestType')
source: chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/pdf_viewer_wrapper.js
```

That points at the PDF viewer API surface, likely the missing
`mimeHandlerPrivate`/PDF viewer private constants and bindings.

Regression smokes:

```text
logs/issue-792-exp7-html-20260529-103646/
logs/issue-792-exp7-pdf-20260529-103659/
```

Normal HTML reached `UrlChanged`, `TitleChanged`, and `LoadingState`. Loading
`bitcoin.pdf` reached `TabReady` and `LoadingState`; it still does not render,
as expected, because this experiment did not wire PDF navigation or stream
handoff. The known teardown `SEGV_ACCERR` still occurs after artifact capture;
that crash predates this experiment and remains out of scope.

## Conclusion

Experiment 7 completed the `chrome://resources` layer for the direct PDF viewer
extension. The PDF viewer shell now loads past shared WebUI resources and fails
at the next missing embedder API surface. The next experiment should wire the
minimal PDF viewer private API constants/bindings needed by
`pdf_viewer_wrapper.js`, starting with the missing `SaveRequestType` value,
while staying out of PDF navigation, stream handoff, guest-view, and
MimeHandlerView unless that API work proves they are the next necessary
dependency.
