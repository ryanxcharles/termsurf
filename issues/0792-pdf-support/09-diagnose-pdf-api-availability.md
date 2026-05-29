# Experiment 9: Diagnose PDF API Availability Gates

## Description

Experiment 8 loaded Chrome's common API feature resource pak, composed
`ChromeExtensionsAPIProvider`, and kept the PDF component extension's
`pdfViewerPrivate` / `resourcesPrivate` permissions. The direct PDF extension
smoke still failed at the same point:

```text
Uncaught TypeError: Cannot read properties of undefined (reading 'SaveRequestType')
source: chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/pdf_viewer_wrapper.js
```

The decisive new clue is that no
`[issue-792-exp8] schema-request name=pdfViewerPrivate ...` line appeared. That
means the renderer binding system never asked for the `pdfViewerPrivate` schema.
The break is therefore somewhere between "Chrome's API provider is present" and
"the PDF viewer script context is allowed to bind `chrome.pdfViewerPrivate`."

Experiment 8's conclusion identified renderer activation as the leading
hypothesis, but Claude's post-review correctly noted that several gates remain
unproven:

1. Chrome API feature JSON may load but fail to parse into the feature map.
2. The PDF extension may be registered in the browser but not broadcast to the
   renderer.
3. The extension may be loaded in the renderer but not activated in the PDF
   renderer process.
4. The script context may be classified as an unprivileged extension context
   instead of `privileged_extension`.
5. `pdfViewerPrivate` may fail a feature gate such as extension type, manifest
   version, location, platform, allowlist, or permission.

This experiment diagnoses that full chain first. If the first broken gate is
identified conclusively and the fix is a single canonical missing call in the
same layer, the experiment may apply that one fix and re-run the direct
extension smoke. It must not guess across layers.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 8 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp8
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp9
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

   Steps 3, 4, and 5 add diagnostic log statements directly to Chromium core
   files (`extensions/browser/renderer_startup_helper.cc` and
   `extensions/renderer/dispatcher.cc`). These are temporary experiment patches,
   not TermSurf-owned permanent architecture. They must be archived in
   `chromium/patches/issue-792/` like every other Chromium modification. The
   result must say which diagnostic logs are worth keeping as future
   debuggability and which should be removed in a later cleanup slice after the
   broken gate is fixed.

2. Diagnose Chrome API feature-map availability.

   Add low-volume logs with prefix:

   ```text
   [issue-792-exp9]
   ```

   The logs must answer:
   - Does the Chrome API feature provider contain `pdfViewerPrivate` after the
     provider setup runs?
   - Does the Chrome permission feature provider contain `pdfViewerPrivate`?
   - Does the generated schema registry contain `pdfViewerPrivate`?

   Preferred implementation:
   - after extension feature providers are initialized in the narrowest
     available TermSurf-owned hook, query:

     ```cpp
     extensions::FeatureProvider::GetAPIFeature("pdfViewerPrivate")
     extensions::FeatureProvider::GetPermissionFeature("pdfViewerPrivate")
     extensions::api::ChromeGeneratedSchemas::IsGenerated("pdfViewerPrivate")
     ```

   - log once per process:

     ```text
     [issue-792-exp9] feature-map api_pdfViewerPrivate=<0|1> permission_pdfViewerPrivate=<0|1> schema_pdfViewerPrivate=<0|1>
     ```

   If there is no clean TermSurf-owned hook after feature-provider
   initialization, add a temporary PDF-only diagnostic near the feature-provider
   initialization path, but do not permanently fork Chromium's extension feature
   loader for TermSurf.

3. Diagnose browser-side PDF extension load broadcast.

   Instrument the browser-side PDF extension registration path:
   - in `RegisterTsPdfComponentExtension()`, log whether
     `RendererStartupHelperFactory::GetForBrowserContext()` returned a helper
     and whether TermSurf called `OnExtensionLoaded(*extension)`;
   - in `extensions/browser/renderer_startup_helper.cc`, add PDF-id-only logs in
     `RendererStartupHelper::OnExtensionLoaded()` for whether the PDF extension
     enters `extension_process_map_`.

   Required logs:

   ```text
   [issue-792-exp9] pdf-extension-load-broadcast helper=<0|1> called=<0|1>
   [issue-792-exp9] renderer-startup-on-loaded extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_count=<n>
   ```

   These logs prove whether the browser knows the PDF extension should be sent
   to renderers.

4. Diagnose browser-side renderer activation.

   Instrument the activation path for the PDF extension:
   - in `TsBrowserClient::SiteInstanceGotProcessAndSite()`, after the Experiment
     6 `ProcessMap::Insert()` succeeds, log whether a `RendererStartupHelper`
     exists for the `BrowserContext`;
   - in `extensions/browser/renderer_startup_helper.cc`, add PDF-id-only logs in
     `RendererStartupHelper::ActivateExtensionInProcess()`, including whether a
     renderer remote already exists or activation was queued in
     `pending_active_extensions_`.

   Required logs:

   ```text
   [issue-792-exp9] pdf-process-map-helper extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=<id> helper=<0|1>
   [issue-792-exp9] renderer-startup-activate extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=<id> remote=<0|1> pending=<0|1>
   ```

   This step is diagnostic by default. If the smoke proves
   `ActivateExtensionInProcess()` is never called for the PDF renderer process,
   the only allowed fix in this experiment is to call the existing canonical
   helper from the point where TermSurf already inserts the PDF extension into
   `ProcessMap`. Do not invent a parallel activation mechanism.

5. Diagnose renderer-side load/activation receipt.

   Add PDF-id-only renderer logs in `extensions/renderer/dispatcher.cc`:
   - when `Dispatcher::LoadExtensions()` receives the PDF extension;
   - when `Dispatcher::ActivateExtension()` receives the PDF extension;
   - after activation, whether `IsExtensionActive(kPdfExtensionId)` returns
     true.

   Required logs:

   ```text
   [issue-792-exp9] renderer-load-extension extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai
   [issue-792-exp9] renderer-activate-extension extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai active=<0|1>
   ```

   These logs prove whether browser-side activation actually reaches the
   renderer before the viewer script context needs bindings.

6. Diagnose script-context classification and API availability.

   Add a PDF-id-only renderer log when a script context is created for
   `chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/...`.

   The log must include:
   - context URL;
   - context type string;
   - effective context type string;
   - whether the extension pointer exists;
   - whether the PDF extension is active in the renderer;
   - whether the context was classified through the `is_webview` path;
   - `context->GetAvailability("pdfViewerPrivate")` result, availability bool,
     and message.

   Required log:

   ```text
   [issue-792-exp9] pdf-script-context url=<url> context=<type> effective_context=<type> has_extension=<0|1> active=<0|1> is_webview=<0|1> pdfViewerPrivate_available=<0|1> result=<n> message=<text>
   ```

   This is the load-bearing diagnostic. It identifies whether the API is absent
   because of context classification, missing activation, missing permission,
   manifest type/version, allowlist, or another feature gate.

7. Apply only the first proven fix, if one is conclusively identified.

   If the logs show the first broken gate, apply only the canonical fix for that
   specific gate, in the smallest possible diff that does not modify unrelated
   diagnostic stages or unrelated behavior. Then re-run the direct extension
   smoke.

   Examples of allowed fixes:
   - If browser registration calls `OnExtensionLoaded()` but no activation is
     ever requested for the PDF renderer process, call
     `RendererStartupHelper::ActivateExtensionInProcess(*extension, process)`
     from the existing `SiteInstanceGotProcessAndSite()` PDF extension branch
     after `ProcessMap::Insert()`.
   - If the PDF extension is active but the context is still unprivileged, add
     the smallest TermSurf-specific context-classification hook only if the logs
     prove Chromium's normal classification path cannot see the active PDF
     extension.
   - If the feature map and context are correct but the manifest fails a
     `pdfViewerPrivate` extension-type, manifest-version, location, or allowlist
     gate, fix only the PDF component extension metadata needed for that exact
     gate.

   Examples of forbidden fixes:
   - manually defining `chrome.pdfViewerPrivate` in JavaScript;
   - bypassing feature availability checks globally;
   - exposing `pdfViewerPrivate` to web pages or non-PDF extensions;
   - adding `ChromeExtensionsRendererAPIProvider` without a log-proven need;
   - implementing `resourcesPrivate` or `pdfViewerPrivate` browser functions;
   - adding PDF navigation, streams, guest-view, MimeHandlerView,
     `PdfViewerStreamManager`, or `--pdf-renderer`.

8. Build and archive only after verification.

   Build:

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp8 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

   If the branch builds and verification passes or produces a useful Partial, do
   the full bookkeeping after Claude after-review accepts the result:
   - commit the Chromium branch;
   - regenerate `chromium/patches/issue-792/`;
   - add the new branch row to `chromium/README.md`;
   - update Experiment 9's line in `issues/0792-pdf-support/README.md` from
     `Designed` to the final status.

## Verification

1. Confirm starting state.

   ```bash
   git status --short
   git -C chromium/src status --short
   git -C chromium/src branch --show-current
   ```

   Chromium should start clean on `148.0.7778.97-issue-792-exp8`.

2. Build the branch.

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp8 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

3. Run the direct PDF extension smoke.

   Reuse the debug screenshot harness against:

   ```text
   chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html
   ```

   Required diagnostic evidence:

   ```text
   [issue-792-exp9] feature-map ...
   [issue-792-exp9] pdf-extension-load-broadcast ...
   [issue-792-exp9] renderer-startup-on-loaded ...
   [issue-792-exp9] pdf-process-map-helper ...
   [issue-792-exp9] renderer-load-extension ...
   [issue-792-exp9] renderer-activate-extension ...
   [issue-792-exp9] pdf-script-context ...
   ```

   The result must identify the first broken gate in the chain. If a fix is
   applied, re-run the smoke and compare before/after logs.

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

   Unless this experiment explicitly applies a proven fix that changes only PDF
   extension API availability, direct PDF navigation is still expected to follow
   the content_shell download path. A browser crash, renderer IPC crash, or hang
   is a failure.

## Pass Criteria

The experiment passes if it either:

- identifies the first broken gate and records the exact cause with the required
  diagnostic logs; or
- identifies the first broken gate, applies the one allowed canonical fix for
  that gate, and proves the `SaveRequestType` error is gone with
  `schema-request name=pdfViewerPrivate found=1` appearing in the direct
  extension smoke.

In both cases, HTML and unchanged PDF regression smokes must not crash or hang
before artifact capture.

## Partial Criteria

The experiment is Partial if it builds and some diagnostics fire, but the logs
do not yet isolate the first broken gate. Examples:

- browser-side logs prove extension load/activation state, but renderer-side
  logs do not fire because the renderer process exits too early;
- renderer-side context classification logs fire, but feature availability is
  unavailable because of missing include/build access;
- all diagnostic logs fire with positive load, activation, classification, and
  availability signals, but the Experiment 8
  `schema-request name=pdfViewerPrivate` line still does not appear, pointing at
  a later binding-system gate such as manifest-version or extension-type
  handling;
- the direct smoke reaches a new crash before the diagnostic sequence completes,
  and the crash stack identifies the next layer.

Every Partial result must record which diagnostic stage was missing and why.

## Failure Criteria

The experiment fails if:

- it skips the diagnostic chain and guesses a fix;
- it exposes PDF private APIs globally or to non-PDF contexts;
- it implements browser-side `pdfViewerPrivate`, `resourcesPrivate`, or
  `metricsPrivate` functions;
- it adds PDF navigation, streams, guest-view, MimeHandlerView,
  `PdfViewerStreamManager`, or `--pdf-renderer`;
- it adds broad Chrome browser UI stacks;
- ordinary HTML pages crash, hang, or lose normal lifecycle messages;
- direct PDF navigation regresses into a crash, hang, or renderer IPC failure;
- the build cannot complete.

## Result

**Result:** Pass

Experiment 9 identified the first broken gate, applied the single allowed
canonical fix for that gate, and exposed the next missing layer.

Before the fix, the direct PDF extension smoke against:

```text
chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html
```

produced these decisive logs in:

```text
logs/issue-792-exp9-extension-20260529-112609/
```

```text
[issue-792-exp9] feature-map api_pdfViewerPrivate=1 permission_pdfViewerPrivate=1 schema_pdfViewerPrivate=1
[issue-792-exp9] pdf-extension-load-broadcast helper=1 called=1
[issue-792-exp9] renderer-startup-on-loaded extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_count=0
[issue-792-exp9] pdf-process-map-helper extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=5 helper=1
[issue-792-exp9] renderer-load-extension extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai
[issue-792-exp9] pdf-script-context url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html context=UNBLESSED_EXTENSION effective_context=UNBLESSED_EXTENSION has_extension=1 active=0 is_webview=0 pdfViewerPrivate_available=0 result=4 message='pdfViewerPrivate' is only allowed to run in privileged pages, but this is a extension iframe
Uncaught TypeError: Cannot read properties of undefined (reading 'SaveRequestType')
```

That proved the feature map and generated schema were present, and the renderer
had loaded the PDF extension, but the PDF extension renderer process had not
been activated. Because `SiteInstanceGotProcessAndSite()` already inserted the
PDF extension into `ProcessMap`, the canonical missing call was
`RendererStartupHelper::ActivateExtensionInProcess(*extension, process)` at that
same point.

After adding that call, the direct PDF extension smoke in:

```text
logs/issue-792-exp9-extension-after-20260529-112706/
```

advanced through the previously broken gate:

```text
[issue-792-exp9] renderer-startup-activate extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=5 remote=1 pending=0
[issue-792-exp9] pdf-activate-request extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=5 called=1
[issue-792-exp9] renderer-activate-extension extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai active=1
[issue-792-exp9] pdf-script-context url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html context=BLESSED_EXTENSION effective_context=BLESSED_EXTENSION has_extension=1 active=1 is_webview=0 pdfViewerPrivate_available=1 result=0 message=
[issue-792-exp8] schema-request name=pdfViewerPrivate found=1
```

The original `SaveRequestType` error disappeared. The smoke then hit the next
missing browser-side PDF viewer layer:

```text
Terminating render process for bad Mojo message: Received bad user message: No binder found for interface help_bubble.mojom.PdfHelpBubbleHandlerFactory for the frame/document scope
```

Regression checks:

- `logs/issue-792-exp9-html-after-20260529-112752/`: normal HTML reached
  `UrlChanged`, `TitleChanged`, and `LoadingState`.
- `logs/issue-792-exp9-pdf-after-20260529-112802/`: direct PDF navigation still
  followed the content_shell download path.

The known teardown `SEGV_ACCERR` after artifact capture still recurred in all
smokes. That is the pre-existing cleanup crash from earlier PDF experiments and
did not prevent the required artifacts from being captured.

Bookkeeping status: Chromium branch commit, patch archive refresh,
`chromium/README.md` branch row, and main-repo commit are deferred until Claude
after-review accepts this result. Claude accepted the result on 2026-05-29, with
only low-severity documentation notes.

## Conclusion

The API availability gate is solved. TermSurf now registers the PDF extension in
the renderer process map and also activates it in the extension renderer startup
path. That is enough for the PDF extension page to become a `BLESSED_EXTENSION`
context and bind `chrome.pdfViewerPrivate`.

The activation call is intentionally PDF-extension-only in this slice. A future
cleanup can generalize it to all extension site instances if TermSurf registers
additional component extensions.

The next missing layer is not API schema exposure. It is browser-side Mojo
binder support for PDF viewer UI services, starting with
`help_bubble.mojom.PdfHelpBubbleHandlerFactory`. Experiment 10 should follow
Electron/Chrome's embedder pattern for registering the minimal PDF viewer Mojo
binders needed by the component extension, beginning with a diagnostic map of
which binders the viewer requests before it can continue.

The following diagnostics are worth keeping until the PDF viewer reaches a
stable render path:

- `pdf-process-map-helper`
- `pdf-activate-request`
- `renderer-startup-activate`
- `renderer-activate-extension`
- `pdf-script-context`

The feature-map and load-broadcast diagnostics are useful for this experiment's
archive but can be removed in a later cleanup slice once the extension system is
stable.

The Chromium patch archive for this slice will also include harmless
`git cl format --full` style churn in files that received diagnostics. Any later
diagnostic-cleanup slice should distinguish formatter churn from the diagnostic
lines themselves.
