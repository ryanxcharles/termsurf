# Experiment 11: Load Extension Renderer Resources

## Description

Experiment 10 moved past the PDF help-bubble Mojo binder. The direct PDF
extension smoke then failed in the renderer resource map:

```text
FATAL:extensions/renderer/resource_bundle_source_map.cc:72] NOTREACHED hit. Module resource registered as "mimeHandlerPrivate" not found
```

This is a narrower problem than the first reading suggested. Chromium's
`CoreExtensionsRendererAPIProvider::PopulateSourceMap()` already registers:

```text
mimeHandlerPrivate -> IDR_MIME_HANDLER_PRIVATE_CUSTOM_BINDINGS_JS
extensions/common/api/mime_handler.mojom -> IDR_MIME_HANDLER_MOJOM_JS
```

The crash says the module name is registered but the resource bytes are missing
from the renderer's `ui::ResourceBundle`. The generated pack exists at:

```text
out/Default/gen/extensions/extensions_renderer_resources.pak
```

Experiment 11 loads that pack into TermSurf's resource bundle, verifies that
`mimeHandlerPrivate` can be read, and then reruns the direct PDF extension smoke
to expose the next gate. It must not implement `mimeHandlerPrivate` browser API
functions, `PdfViewerStreamManager`, guest-view, MimeHandlerView, PDF navigation
interception, or `--pdf-renderer`.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 10 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp10
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp11
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Load `extensions_renderer_resources.pak`.

   Extend `LoadTsPdfResourceBundle()` in
   `content/libtermsurf_chromium/extensions/ts_pdf_resource_bundle.cc`.

   Load:

   ```text
   gen/extensions/extensions_renderer_resources.pak
   ```

   Use the same direct
   `ui::ResourceBundle::GetSharedInstance().AddDataPackFromPath(...)` pattern
   already used for:
   - `gen/chrome/pdf_resources.pak`;
   - `gen/chrome/common_resources.pak`.

   Required log:

   ```text
   [issue-792-exp11] extensions-renderer-pak path=<path> found=<0|1> loaded=<0|1> mimeHandlerPrivate_bytes=<n> mime_handler_mojom_bytes=<n>
   ```

   `loaded=1` requires both resources to be non-empty.

   Verify the bytes by reading these constants with
   `ui::ResourceBundle::LoadDataResourceString(...)`:
   - `IDR_MIME_HANDLER_PRIVATE_CUSTOM_BINDINGS_JS`
   - `IDR_MIME_HANDLER_MOJOM_JS`

   They are defined in `extensions/grit/extensions_renderer_resources.h`, from
   the allowed `//extensions/renderer/resources` target. These are the canonical
   resource IDs for the two module names that
   `CoreExtensionsRendererAPIProvider::PopulateSourceMap()` already registers.
   Loading the pack makes the bytes available; it does not register new module
   names.

   `LoadTsPdfResourceBundle()` is now carrying more than PDF resources because
   prior slices added Chrome common resources and this slice adds extension
   renderer resources. Do not rename it in this experiment; note the helper name
   as future cleanup once the PDF path is stable.

3. Add only the required dependency.

   If the existing `libtermsurf_chromium` deps do not make
   `extensions_renderer_resources.pak` available in
   `out/Default/gen/extensions/`, add the narrow GN dependency that generates
   that pack.

   Allowed dependency family:
   - `//extensions/renderer/resources`

   Forbidden dependencies:
   - `//chrome/renderer/extensions`
   - `//chrome/renderer`
   - broad `//chrome/browser/*`
   - `//chrome/browser/pdf`
   - `//chrome/browser/pdf:pdf`

   Do not add `ChromeExtensionsRendererAPIProvider`. The core renderer provider
   already registers the `mimeHandlerPrivate` module name; this experiment is
   about missing resource bytes, not broad Chrome renderer APIs.

4. Keep Experiment 10's binders intact.

   Do not remove or weaken:
   - `pdf-help-bubble-binder`;
   - `pdf-help-bubble-create-handler`;
   - `pdf-host-binder` registration;
   - Experiment 9 extension activation.

5. Diagnose the next gate after the module bytes load.

   After loading the resource pack, rerun the direct PDF extension smoke and
   inspect logs.

   If the `mimeHandlerPrivate` resource fatal disappears, record the next actual
   failure. Likely outcomes:
   - `mimeHandlerPrivate.getStreamInfo` or `pdfViewerPrivate.getStreamInfo`
     reaches an unimplemented browser API path;
   - `chrome.mimeHandlerPrivate.getStreamInfo` is still undefined because
     Experiment 3 stripped the `mimeHandlerPrivate` manifest permission;
   - `pdf-host-binder` finally fires;
   - the viewer reaches a new missing Mojo binder;
   - the viewer reaches the PDF stream handoff / `PdfViewerStreamManager` layer.

   Do not implement any of those next layers in this experiment.

6. Build and archive only after verification.

   Build:

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp10 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

   If the branch builds and verification passes or produces a useful Partial, do
   the full bookkeeping after Claude after-review accepts the result:
   - commit the Chromium branch;
   - regenerate `chromium/patches/issue-792/`;
   - add the new branch row to `chromium/README.md`;
   - update Experiment 11's line in `issues/0792-pdf-support/README.md` from
     `Designed` to the final status.

## Verification

1. Confirm starting state.

   ```bash
   git status --short
   git -C chromium/src status --short
   git -C chromium/src branch --show-current
   ```

   Chromium should start clean on `148.0.7778.97-issue-792-exp10`.

2. Build the branch.

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp10 --full
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

   - Experiment 11 resource log appears:

     ```text
     [issue-792-exp11] extensions-renderer-pak ... loaded=1 ...
     ```

   - The previous fatal is gone:

     ```text
     Module resource registered as "mimeHandlerPrivate" not found
     ```

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
   path unless this resource-load change unexpectedly changes direct navigation.
   A browser crash, renderer IPC crash, or hang is a failure.

## Pass Criteria

The experiment passes if:

- `libtermsurf_chromium` builds;
- `extensions_renderer_resources.pak` is found and loaded;
- `mimeHandlerPrivate` and `extensions/common/api/mime_handler.mojom` resource
  bytes are non-empty;
- the direct PDF extension smoke no longer dies at
  `Module resource registered as "mimeHandlerPrivate" not found`;
- Experiment 9 and 10 evidence remains intact;
- the result records the next observed PDF viewer gate, if any;
- HTML and unchanged PDF regression smokes do not crash or hang before artifact
  capture.

## Partial Criteria

The experiment is Partial if it builds and proves some part of the resource-pack
diagnosis, but does not fully cross the `mimeHandlerPrivate` module-resource
gate. Examples:

- the generated pack is not present unless a broader dependency is added, and
  the narrow `//extensions/renderer/resources` dependency is insufficient;
- the pack loads but one of the two required resources is still empty;
- the fatal changes to a different resource name in the same pack;
- the direct smoke reaches a new crash whose stack identifies the next layer.

Every Partial result must record the exact blocker and the next experiment's
target.

## Failure Criteria

The experiment fails if:

- it implements browser-side `mimeHandlerPrivate`, `pdfViewerPrivate`,
  `resourcesPrivate`, or stream APIs;
- it implements PDF navigation interception, streams, guest-view,
  MimeHandlerView, `PdfViewerStreamManager`, or `--pdf-renderer`;
- it adds `ChromeExtensionsRendererAPIProvider`;
- it adds `//chrome/renderer/extensions`, `//chrome/renderer`,
  `//chrome/browser/pdf`, `//chrome/browser/pdf:pdf`, `//chrome/browser/ui`, or
  broad Chrome browser UI stacks;
- it removes or weakens Experiment 9's extension activation fix;
- it removes or weakens Experiment 10's PDF viewer binder slice;
- ordinary HTML pages crash, hang, or lose normal lifecycle messages;
- direct PDF navigation regresses into a crash, hang, or renderer IPC failure;
- the build cannot complete.

## Result

**Result:** Pass

Experiment 11 loaded the extension renderer resource pack and crossed the
`mimeHandlerPrivate` module-resource gate.

Direct PDF extension smoke:

```text
logs/issue-792-exp11-extension-20260529-115559/
```

The new resource-pack log proves the generated pack exists and both required
resources are readable:

```text
[issue-792-exp11] extensions-renderer-pak path=/Users/ryan/dev/termsurf/chromium/src/out/Default/gen/extensions/extensions_renderer_resources.pak found=1 loaded=1 mimeHandlerPrivate_bytes=3766 mime_handler_mojom_bytes=27053
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

The previous fatal did not recur:

```text
Module resource registered as "mimeHandlerPrivate" not found
```

The next gate is now a missing browser-side binder:

```text
Terminating render process for bad Mojo message: Received bad user message: No binder found for interface extensions.mime_handler.MimeHandlerService for the frame/document scope
```

Regression checks:

- `logs/issue-792-exp11-html-20260529-115624/`: normal HTML reached
  `UrlChanged`, `TitleChanged`, and `LoadingState`.
- `logs/issue-792-exp11-pdf-20260529-115634/`: direct PDF navigation still
  followed the content_shell download path.

The known teardown `SEGV_ACCERR` after artifact capture still recurred in all
smokes. That is the pre-existing cleanup crash from earlier PDF experiments and
did not prevent the required artifacts from being captured.

Bookkeeping status: Chromium branch commit, patch archive refresh,
`chromium/README.md` branch row, and main-repo commit are deferred until Claude
after-review accepts this result. Claude accepted the result on 2026-05-29, with
only a low-severity documentation note.

## Conclusion

The `mimeHandlerPrivate` JavaScript module resource problem is solved. The
module name was already registered by `CoreExtensionsRendererAPIProvider`; the
missing layer was the generated `extensions_renderer_resources.pak` data pack.

The next missing layer is the browser-side
`extensions.mime_handler.MimeHandlerService` binder. Experiment 12 should follow
Electron's pattern for registering only the MIME-handler service binders needed
by the PDF viewer, without implementing stream handoff or
`PdfViewerStreamManager` until the binder gate is crossed and the logs prove the
next failure.

Experiment 12 should also re-check whether `pdf-host-binder` fires after
`MimeHandlerService` is bound. It still did not fire in Experiment 11 because
the renderer died before reaching the PDF plugin host path.
