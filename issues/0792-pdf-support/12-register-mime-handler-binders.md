# Experiment 12: Register MIME-handler Binders

## Description

Experiment 11 crossed the extension renderer resource-pack gate. The direct PDF
extension smoke now dies because the PDF viewer's `mimeHandlerPrivate` custom
bindings request a browser-side Mojo interface that TermSurf does not register:

```text
No binder found for interface extensions.mime_handler.MimeHandlerService for the frame/document scope
```

Chromium and Electron both register two frame-scoped MIME-handler interfaces:

- `extensions.mime_handler.MimeHandlerService`
- `extensions.mime_handler.BeforeUnloadControl`

Their real implementations are backed by `MimeHandlerViewGuest` and a
`StreamContainer`. TermSurf does not have those layers yet. Experiment 12 only
crosses the binder-registration gate by adding TermSurf-owned diagnostic
implementations:

- `MimeHandlerService.GetStreamInfo()` returns `null`;
- `MimeHandlerService.SetPdfPluginAttributes(...)` logs and ignores the
  attributes;
- `BeforeUnloadControl.SetShowBeforeUnloadDialog(...)` logs and acknowledges the
  request.

Returning `null` from `GetStreamInfo()` is intentional. Chromium's own
`MimeHandlerServiceImpl` does the same when its stream has been aborted or is
missing. This lets the renderer continue past the bad-Mojo kill and expose the
next real layer without silently inventing a stream, navigation throttle,
guest-view, `PdfViewerStreamManager`, or PDF renderer process model.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 11 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp11
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp12
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Add a TermSurf MIME-handler binder implementation.

   Add a small `content/libtermsurf_chromium` helper, for example:

   ```text
   ts_mime_handler_binders.h
   ts_mime_handler_binders.cc
   ```

   It should expose two binding functions:

   ```text
   BindTsMimeHandlerService(RenderFrameHost*, PendingReceiver<MimeHandlerService>)
   BindTsBeforeUnloadControl(RenderFrameHost*, PendingReceiver<BeforeUnloadControl>)
   ```

   The bind helpers should check that the calling frame belongs to the PDF
   component extension, using the same PDF-extension gate shape as Experiment
   10's help-bubble factory. Non-PDF frames should drop the receiver without
   creating the diagnostic service. This matches Electron/Chrome's implicit
   guest-view gating, where the real service is only created when a
   `MimeHandlerViewGuest` exists.

   Use `extensions/common/api/mime_handler.mojom.h` for the interface types.
   Prefer self-owned Mojo receivers for the concrete diagnostic service objects.
   Do not depend on `MimeHandlerViewGuest`, `MimeHandlerServiceImpl`,
   `StreamContainer`, or any guest-view browser implementation in this
   experiment.

   The TermSurf service classes must implement every pure-virtual method on the
   Mojo interfaces, not only the methods the renderer-side JavaScript currently
   calls. Any unmentioned method should log on invocation and complete its
   callback, or no-op if the method is fire-and-forget, without doing real work.

   For callback-style Mojo methods, the diagnostic implementation must complete
   the callback. For example:
   - `GetStreamInfo()` runs its callback with `nullptr`;
   - `BeforeUnloadControl.SetShowBeforeUnloadDialog(...)` runs its empty
     acknowledgment callback.

   Do not leave renderer promises waiting on an uncompleted callback.

   Required logs:

   ```text
   [issue-792-exp12] mime-handler-service-binder frame_url=<url> site_url=<url>
   [issue-792-exp12] mime-handler-get-stream-info frame_url=<url> stream_info=null
   [issue-792-exp12] mime-handler-set-pdf-plugin-attributes frame_url=<url>
   [issue-792-exp12] before-unload-control-binder frame_url=<url> site_url=<url>
   [issue-792-exp12] before-unload-set-show frame_url=<url> show=<0|1>
   ```

   The service object may capture frame URL strings at bind time for logging. Do
   not hold raw `RenderFrameHost*` pointers after binding.

3. Register both frame-scoped binders.

   In `TsBrowserClient::RegisterBrowserInterfaceBindersForFrame()`, register:

   ```text
   extensions::mime_handler::MimeHandlerService
   extensions::mime_handler::BeforeUnloadControl
   ```

   Register both even if the first smoke only asks for `MimeHandlerService`. The
   renderer resource `mime_handler_private_custom_bindings.js` requests both
   interfaces during module initialization, and Chrome/Electron register both
   together.

   Keep Experiment 10's existing binders intact:
   - `PdfHelpBubbleHandlerFactory`;
   - associated `pdf::mojom::PdfHost`.

4. Keep this slice diagnostic-only.

   Forbidden in this experiment:
   - creating a real `StreamContainer`;
   - wiring `MimeHandlerViewGuest`;
   - wiring guest-view attach helpers;
   - wiring `PdfViewerStreamManager`;
   - changing PDF navigation interception;
   - changing the direct PDF download path;
   - adding `--pdf-renderer` process-model logic;
   - restoring or changing PDF extension manifest permissions;
   - adding Chrome browser UI stacks.

   If returning a null stream causes the viewer to report "stream aborted" or a
   similar renderer-side error, that is the expected useful next gate. Record it
   and stop.

5. Re-check the downstream PDF plugin binders.

   After the MIME-handler binder gate is crossed, inspect whether Experiment
   10's associated PDF host binder fires:

   ```text
   [issue-792-exp10] pdf-host-binder ...
   ```

   The expected result may still be "does not fire" because no real stream or
   plugin frame exists yet. Record the observation either way. Do not alter
   `PdfHost` binding in this experiment.

6. Build and archive only after verification.

   Build:

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp11 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

   If the branch builds and verification passes or produces a useful Partial, do
   the full bookkeeping after Claude after-review accepts the result:
   - commit the Chromium branch;
   - regenerate `chromium/patches/issue-792/`;
   - add the new branch row to `chromium/README.md`;
   - update Experiment 12's line in `issues/0792-pdf-support/README.md` from
     `Designed` to the final status.

## Verification

1. Confirm starting state.

   ```bash
   git status --short
   git -C chromium/src status --short
   git -C chromium/src branch --show-current
   ```

   Chromium should start clean on `148.0.7778.97-issue-792-exp11`.

2. Build the branch.

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp11 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

3. Run the direct PDF extension smoke.

   Reuse the debug screenshot harness against:

   ```text
   chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html
   ```

   Required evidence:
   - Experiment 9 activation remains intact:

     ```text
     [issue-792-exp9] renderer-activate-extension ... active=1
     [issue-792-exp9] pdf-script-context ... context=BLESSED_EXTENSION ... pdfViewerPrivate_available=1
     [issue-792-exp8] schema-request name=pdfViewerPrivate found=1
     ```

   - Experiment 10 help-bubble binder remains intact:

     ```text
     [issue-792-exp10] pdf-help-bubble-binder ...
     [issue-792-exp10] pdf-help-bubble-create-handler ...
     ```

   - Experiment 11 resource-pack load remains intact:

     ```text
     [issue-792-exp11] extensions-renderer-pak ... loaded=1 ...
     ```

   - Experiment 12 binders fire:

     ```text
     [issue-792-exp12] mime-handler-service-binder ...
     [issue-792-exp12] mime-handler-get-stream-info ... stream_info=null
     [issue-792-exp12] before-unload-control-binder ...
     ```

   - The previous bad-Mojo binder failure is gone:

     ```text
     No binder found for interface extensions.mime_handler.MimeHandlerService
     ```

   Record the next observed error exactly. The most likely acceptable next gate
   is a renderer-side `mimeHandlerPrivate.getStreamInfo()` failure because the
   diagnostic service returned a null stream.

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

   Direct PDF navigation is still expected to follow the content_shell download
   path. A browser crash, renderer IPC crash, or hang is a failure.

## Pass Criteria

The experiment passes if:

- `libtermsurf_chromium` builds;
- both MIME-handler browser-side binders are registered;
- the direct PDF extension smoke no longer dies at the missing
  `MimeHandlerService` binder;
- `GetStreamInfo()` is called and returns `null`;
- the result records the next observed PDF viewer gate;
- Experiment 9, 10, and 11 evidence remains intact;
- HTML and unchanged PDF regression smokes do not crash or hang before artifact
  capture.

## Partial Criteria

The experiment is Partial if it builds and registers at least one binder but
does not fully cross the missing-binder gate. Examples:

- `MimeHandlerService` binds but `BeforeUnloadControl` still produces a bad-Mojo
  kill;
- the service binds but the renderer disconnects before `GetStreamInfo()` and
  the logs identify why;
- the generated Mojo interface signatures require a broader dependency or
  ownership shape than the diagnostic helper can provide cleanly.

Every Partial result must record the exact blocker and the next experiment's
target.

## Failure Criteria

The experiment fails if:

- it implements real stream handoff;
- it creates or stores real `StreamContainer` objects;
- it wires `MimeHandlerViewGuest`, guest-view attach helpers, or
  `PdfViewerStreamManager`;
- it changes PDF navigation interception or the direct PDF download path;
- it adds `--pdf-renderer` process-model logic;
- it changes PDF extension manifest permissions;
- it removes or weakens Experiment 9 activation;
- it removes or weakens Experiment 10 PDF viewer binders;
- it removes or weakens Experiment 11 renderer resource-pack loading;
- ordinary HTML pages crash, hang, or lose normal lifecycle messages;
- direct PDF navigation regresses into a crash, hang, or renderer IPC failure;
- the build cannot complete.

## Result

**Result:** Partial

Experiment 12 built and registered both MIME-handler browser-side binders, but
the direct PDF extension smoke stopped at a newly exposed binder gate before
`GetStreamInfo()` could run.

Build:

```text
autoninja -C out/Default libtermsurf_chromium
Build Succeeded: 3 steps
```

Direct PDF extension smoke:

```text
logs/issue-792-exp12-extension-20260529-121137/
```

Experiment 9 activation and API availability remained intact:

```text
[issue-792-exp9] renderer-activate-extension extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai active=1
[issue-792-exp9] pdf-script-context url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html context=BLESSED_EXTENSION effective_context=BLESSED_EXTENSION has_extension=1 active=1 is_webview=0 pdfViewerPrivate_available=1 result=0 message=
[issue-792-exp8] schema-request name=pdfViewerPrivate found=1
```

Experiment 10's help-bubble binder remained intact:

```text
[issue-792-exp10] pdf-help-bubble-binder frame_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html site_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/
[issue-792-exp10] pdf-help-bubble-create-handler frame_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html site_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/
```

Experiment 11's renderer resource pack remained loaded:

```text
[issue-792-exp11] extensions-renderer-pak path=/Users/ryan/dev/termsurf/chromium/src/out/Default/gen/extensions/extensions_renderer_resources.pak found=1 loaded=1 mimeHandlerPrivate_bytes=3766 mime_handler_mojom_bytes=27053
```

The new Experiment 12 binders fired:

```text
[issue-792-exp12] mime-handler-service-binder frame_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html site_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/
[issue-792-exp12] before-unload-control-binder frame_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html site_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/
```

The previous missing `MimeHandlerService` bad-Mojo kill did not recur. The next
gate is now `extensions.KeepAlive`:

```text
Terminating render process for bad Mojo message: Received bad user message: No binder found for interface extensions.KeepAlive for the frame/document scope
```

Because the renderer died at `extensions.KeepAlive`, the expected diagnostic
line did not appear:

```text
[issue-792-exp12] mime-handler-get-stream-info ... stream_info=null
```

`pdf-host-binder` also still did not fire. The viewer is not yet far enough
through startup to create the PDF plugin host path.

Regression checks:

- `logs/issue-792-exp12-html-20260529-121206/`: normal HTML reached
  `UrlChanged`, `TitleChanged`, and `LoadingState`.
- `logs/issue-792-exp12-pdf-20260529-121220/`: direct PDF navigation still
  followed the content_shell download path via
  `ShellDownloadManagerDelegate::ChooseDownloadPath`.

The known teardown `SEGV_ACCERR` after artifact capture still recurred. It did
not prevent the required artifacts from being captured.

Bookkeeping status: Chromium branch commit, patch archive refresh,
`chromium/README.md` branch row, and main-repo commit are deferred until Claude
after-review accepts this result.

## Conclusion

The MIME-handler binder gate is solved. The PDF viewer can now request both
`extensions.mime_handler.MimeHandlerService` and
`extensions.mime_handler.BeforeUnloadControl` without triggering the previous
bad-Mojo termination.

The next missing layer is the frame-scoped `extensions.KeepAlive` binder used by
extension API promise plumbing. Experiment 13 should follow the Chromium and
Electron extension binder patterns for `extensions.KeepAlive`, but should stay
narrow: register only the required keepalive binder, keep the PDF-extension gate
discipline, and do not implement stream handoff, guest-view,
`PdfViewerStreamManager`, or PDF renderer process-model changes until the viewer
reaches `GetStreamInfo()`.
