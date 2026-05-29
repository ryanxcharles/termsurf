# Experiment 8: Expose The PDF Viewer Private API Surface

## Description

Experiment 7 proved the PDF viewer can now load its extension resources and
shared `chrome://resources` dependencies. The first new blocker is not resource
loading; it is the Chrome extension API surface the viewer expects:

```text
Uncaught TypeError: Cannot read properties of undefined (reading 'SaveRequestType')
source=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/pdf_viewer_wrapper.js
```

The failing code reads `chrome.pdfViewerPrivate.SaveRequestType`. TermSurf's
extension system currently installs `CoreExtensionsAPIProvider`, but the
TermSurf-specific provider is still an empty shell:

- no Chrome API feature metadata;
- no Chrome permission feature metadata;
- no Chrome generated schemas;
- no Chrome private API permission registrations.

Experiment 3 also stripped the PDF component extension's `permissions` field
entirely to get the component extension loading before the Chrome permission
registry existed. That was correct for the earlier slice, but it now prevents
the PDF viewer from receiving `pdfViewerPrivate` and `resourcesPrivate`.

This experiment wires only the Chrome API schema/permission substrate needed for
the PDF component extension to see the generated private API objects. It does
not implement the browser-side functions behind those APIs. The expected next
blocker is therefore a real API call such as
`chrome.resourcesPrivate.getStrings()` or
`chrome.pdfViewerPrivate.getStreamInfo()`, not the absence of
`chrome.pdfViewerPrivate.SaveRequestType`.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 7 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp7
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp8
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Load the Chrome common resources pak in every process.

   `ChromeExtensionsAPIProvider::AddAPIJSONSources()` loads
   `IDR_CHROME_EXTENSION_API_FEATURES`, which comes from
   `chrome/common/common_resources.grd` and is packaged as:

   ```text
   gen/chrome/common_resources.pak
   ```

   TermSurf currently loads `gen/chrome/pdf_resources.pak` in
   `LoadTsPdfResourceBundle()` from `TsMainDelegate::PreSandboxStartup()`. That
   hook runs in every Chromium process and is the right place to load the Chrome
   common pak too, because both browser and renderer code can instantiate
   `TsExtensionsClient` and ask for API feature data.

   Extend the existing resource-bundle helper or add a sibling helper that:
   - builds the path from `base::DIR_ASSETS` to
     `gen/chrome/common_resources.pak`;
   - calls `ui::ResourceBundle::GetSharedInstance().AddDataPackFromPath(...)`
     when the pak exists;
   - verifies loading by reading `IDR_CHROME_EXTENSION_API_FEATURES` with
     `ui::ResourceBundle::LoadDataResourceString(...)`, which transparently
     decompresses the gzipped JSON resource;
   - logs:

     ```text
     [issue-792-exp8] chrome-common-pak path=<path> found=<0|1> loaded=<0|1>
     ```

   Do not rely on a successful C++ link as proof that the API feature JSON is
   available at runtime. Without this pak, the generated schemas can be linked
   while the feature provider still registers no Chrome API availability data.

3. Add the Chrome common extension API provider as a sibling provider.

   Extend `content/libtermsurf_chromium/extensions/ts_extensions_client.cc` so
   `TsExtensionsClient` composes Chromium's Chrome common API provider directly:

   ```cpp
   TsExtensionsClient::TsExtensionsClient() {
     AddAPIProvider(std::make_unique<extensions::CoreExtensionsAPIProvider>());
     AddAPIProvider(std::make_unique<extensions::ChromeExtensionsAPIProvider>());
     AddAPIProvider(std::make_unique<TsExtensionsAPIProvider>());
   }
   ```

   The source reference is
   `chrome/common/extensions/chrome_extensions_api_provider.cc`. Sibling
   provider composition matches Chromium's extension-client model and avoids
   reimplementing provider dispatch inside TermSurf's currently empty
   `TsExtensionsAPIProvider`.

   Do not replace `TsExtensionsClient` with `ChromeExtensionsClient`. TermSurf
   still owns the embedder client, permission-message provider, scriptability
   policy, origin-access policy, and product identity.

4. Keep the manifest and behavior surface narrow.

   Update
   `content/libtermsurf_chromium/extensions/ts_pdf_component_extension.cc` so
   the PDF component extension keeps only the private permissions needed by the
   currently loaded viewer shell:

   ```json
   "permissions": ["pdfViewerPrivate", "resourcesPrivate"]
   ```

   Implement this by filtering the existing `permissions` list instead of
   removing the entire key. Continue stripping unrelated Chrome permissions and
   host permissions from the static manifest snapshot, including:
   - `chrome://resources/`;
   - `chrome://webui-test/`;
   - `contentSettings`;
   - `metricsPrivate`;
   - `tabs`;
   - `fileSystem.write`;
   - `web_accessible_resources`.

   Use `manifest->FindList("permissions")`, build a new `base::Value::List` with
   only the two kept string entries, collect strings and dict entries that were
   stripped for diagnostics, then replace the original array with
   `manifest->Set("permissions", std::move(kept_permissions))`.

   Remove or replace the old Experiment 3 log that said the whole permissions
   key was stripped and listed `pdfViewerPrivate`/`resourcesPrivate` among the
   removed values. After this experiment, those two permissions are kept, so the
   old log would be false. The already-landed Experiment 7 browser and renderer
   access grants remain the source of truth for `chrome://resources` loading. Do
   not reintroduce broad host permissions as a shortcut.

5. Add only the necessary build dependencies.

   Update `content/libtermsurf_chromium/BUILD.gn` with the smallest deps needed
   for the Chrome common API provider and generated schemas. Likely candidates:
   - `//chrome/common/extensions`;
   - `//chrome/common/extensions/api`.

   Do not add `//chrome/browser/extensions`, Chrome profiles, Chrome browser UI,
   guest-view, MimeHandlerView, `PdfViewerStreamManager`, or PDF navigation
   dependencies in this experiment.

6. Do not add Chrome renderer custom API providers unless the common-provider
   path proves insufficient.

   The `SaveRequestType` failure is a generated schema/availability failure, not
   evidence that custom renderer bindings are missing. Keep
   `TsContentRendererClient` on `CoreExtensionsRendererAPIProvider` only.

   If the implementation builds and the direct viewer smoke still reports
   `chrome.pdfViewerPrivate` or `SaveRequestType` as undefined, record the
   result as Partial and identify whether the missing layer is:
   - renderer API-provider registration;
   - extension API availability;
   - stripped manifest permissions;
   - allowlist/location mismatch.

   Do not silently add `ChromeExtensionsRendererAPIProvider` in this experiment.

7. Add minimal diagnostics.

   Use Chromium `LOG(INFO)` lines with this exact prefix:

   ```text
   [issue-792-exp8]
   ```

   Required low-volume lines:

   ```text
   [issue-792-exp8] chrome-common-pak path=<path> found=<0|1> loaded=<0|1>
   [issue-792-exp8] chrome-api-provider-enabled provider=ChromeExtensionsAPIProvider
   [issue-792-exp8] pdf-permissions-kept values=pdfViewerPrivate,resourcesPrivate
   ```

   If the extension API system asks for `pdfViewerPrivate` or
   `resourcesPrivate`, log one line per API name per process:

   ```text
   [issue-792-exp8] schema-request name=pdfViewerPrivate found=<0|1>
   [issue-792-exp8] schema-request name=resourcesPrivate found=<0|1>
   ```

   Do not log every schema lookup. Chromium asks for many APIs during extension
   startup.

8. Do not widen the experiment.

   Forbidden in this experiment:
   - implementing `chrome.pdfViewerPrivate.*` browser functions;
   - implementing `chrome.resourcesPrivate.*` browser functions;
   - PDF navigation or MIME interception;
   - `PdfViewerStreamManager`;
   - guest-view or MimeHandlerView;
   - `--pdf-renderer`;
   - stream handoff;
   - broad Chrome browser UI/resource stacks;
   - restoring the full PDF extension manifest.

   If the viewer advances from the enum failure to a missing API function,
   localization, stream, Mojo, plugin, or process-model failure, record that new
   blocker and design the next experiment around it.

9. Build and archive only after verification.

   Build:

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp7 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

   If the branch builds and verification passes or produces a useful Partial, do
   the full bookkeeping after Claude after-review accepts the result:
   - commit the Chromium branch;
   - regenerate `chromium/patches/issue-792/`;
   - add the new branch row to `chromium/README.md`;
   - update Experiment 8's line in `issues/0792-pdf-support/README.md` from
     `Designed` to the final status.

## Verification

1. Confirm starting state.

   ```bash
   git status --short
   git -C chromium/src status --short
   git -C chromium/src branch --show-current
   ```

   Chromium should start clean on `148.0.7778.97-issue-792-exp7`.

2. Build the branch.

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp7 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

3. Run the direct extension-resource smoke.

   Reuse the debug screenshot harness against:

   ```text
   chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html
   ```

   Pass requires:
   - Experiment 7 still grants and serves `chrome://resources`:

     ```text
     [issue-792-exp7] chrome-resources-grant extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai
     [issue-792-exp7] chrome-resources-factory extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai
     ```

   - Experiment 8 proves the Chrome API provider and PDF permissions are wired:

     ```text
     [issue-792-exp8] chrome-common-pak path=<path> found=1 loaded=1
     [issue-792-exp8] chrome-api-provider-enabled provider=ChromeExtensionsAPIProvider
     [issue-792-exp8] pdf-permissions-kept values=pdfViewerPrivate,resourcesPrivate
     [issue-792-exp8] schema-request name=pdfViewerPrivate found=1
     ```

   - the Experiment 7 `SaveRequestType` undefined error is gone;
   - no `FATAL`, `NOTREACHED`, renderer IPC crash, or hang occurs before the
     screenshot artifact is captured.

   If `chrome-api-provider-enabled` fires but the `schema-request` line for
   `pdfViewerPrivate` never appears, record the result as Partial and identify
   whether the Chrome API feature JSON failed to load, the feature map did not
   make `pdfViewerPrivate` available to the PDF component extension, or the
   manifest permissions did not parse.

   In that case, inspect `chrome/common/extensions/api/_api_features.json` for
   the `pdfViewerPrivate` entry and record which extension type, allowlist,
   platform, or manifest-version gate TermSurf failed to satisfy.

   Record the new first blocker. Expected possibilities include
   `chrome.resourcesPrivate.getStrings()` lacking a browser implementation,
   `chrome.pdfViewerPrivate.getStreamInfo()` lacking a browser implementation,
   missing localization data, or stream/document data still being absent.

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

## Pass Criteria

The experiment passes if:

- `gen/chrome/common_resources.pak` is found and
  `IDR_CHROME_EXTENSION_API_FEATURES` loads from it in the relevant browser and
  renderer processes;
- `ChromeExtensionsAPIProvider` is composed into `TsExtensionsClient` without
  replacing TermSurf's embedder-owned `TsExtensionsClient`;
- the PDF component extension keeps exactly `pdfViewerPrivate` and
  `resourcesPrivate` from the original permissions list;
- the direct PDF extension smoke no longer reports the Experiment 7
  `SaveRequestType` undefined error;
- the smoke records the next first blocker as a later layer, such as a missing
  `resourcesPrivate`/`pdfViewerPrivate` browser-side function, localization
  data, stream data, Mojo path, plugin path, or process-model path;
- normal HTML still reaches its lifecycle logs with no extension IPC crash;
- direct PDF navigation still follows the unchanged content_shell path without
  crashing or hanging.

## Partial Criteria

The experiment is Partial if it builds and produces useful diagnostic evidence
but does not advance past the `SaveRequestType` failure. Useful Partial outcomes
include:

- `chrome-common-pak found=0` or `loaded=0`, proving the API feature JSON is not
  available at runtime;
- `chrome-api-provider-enabled` fires, but the `schema-request` line for
  `pdfViewerPrivate` never appears, pointing at API feature gating or manifest
  permission parsing;
- the schema request appears with `found=0`, pointing at generated-schema or
  build-dependency wiring;
- the PDF extension is created but its kept permissions do not include both
  `pdfViewerPrivate` and `resourcesPrivate`;
- `SaveRequestType` resolves, but the viewer immediately fails on an
  unimplemented `chrome.resourcesPrivate.*` or `chrome.pdfViewerPrivate.*`
  function.

Every Partial result must record the exact first failing layer and the next
experiment should target only that layer.

## Failure Criteria

The experiment fails if:

- it implements browser-side `pdfViewerPrivate` or `resourcesPrivate` functions
  instead of stopping at API surface availability;
- it pulls in `//chrome/browser/extensions`, Chrome profiles, Chrome browser UI,
  guest-view, MimeHandlerView, `PdfViewerStreamManager`, PDF navigation, stream
  handoff, or `--pdf-renderer`;
- it restores the full PDF component extension manifest or broad
  `chrome://resources`/`chrome://webui-test` host permissions;
- it adds `ChromeExtensionsRendererAPIProvider` without first recording evidence
  that the common-provider path is insufficient;
- ordinary HTML pages crash, hang, or lose normal lifecycle messages;
- direct PDF navigation regresses from the unchanged content_shell download path
  into a crash, hang, or renderer IPC failure;
- the build cannot complete.

## Result

**Result:** Partial

The branch `148.0.7778.97-issue-792-exp8` builds `libtermsurf_chromium`, loads
the Chrome common resource pak, composes `ChromeExtensionsAPIProvider`, and
keeps the PDF viewer's two narrow private permissions. That proved the intended
API-provider and manifest-permission substrate can be added without breaking the
build or the existing HTML/PDF smoke paths.

Verification artifacts:

- Direct PDF extension smoke: `logs/issue-792-exp8-extension-20260529-110318/`
- Normal HTML smoke: `logs/issue-792-exp8-html-20260529-110513/`
- Unchanged PDF navigation smoke: `logs/issue-792-exp8-pdf-20260529-110523/`

Build:

```text
autoninja -C out/Default libtermsurf_chromium
Build Succeeded: 14 steps
```

Required Experiment 8 evidence appeared:

```text
[issue-792-exp8] chrome-common-pak path=/Users/ryan/dev/termsurf/chromium/src/out/Default/gen/chrome/common_resources.pak found=1 loaded=1
[issue-792-exp8] chrome-api-provider-enabled provider=ChromeExtensionsAPIProvider
[issue-792-exp8] pdf-permissions-kept values=pdfViewerPrivate,resourcesPrivate
[issue-792-exp8] pdf-permissions-stripped values=chrome://resources/,chrome://webui-test/,contentSettings,metricsPrivate,tabs,fileSystem.write
```

Experiment 7's resource-serving evidence also still appeared in the direct
extension smoke:

```text
[issue-792-exp7] chrome-resources-grant extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=5
[issue-792-exp7] chrome-resources-factory extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=5 frame_id=1
```

However, the direct extension smoke did **not** advance past the previous
blocker:

```text
Uncaught TypeError: Cannot read properties of undefined (reading 'SaveRequestType')
source: chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/pdf_viewer_wrapper.js
```

No `[issue-792-exp8] schema-request name=pdfViewerPrivate ...` line appeared.
That means the renderer binding system never tried to fetch the
`pdfViewerPrivate` schema, even though the pak was loaded, the provider was
present, and the manifest kept the permission.

The regression smokes were acceptable:

- Normal HTML reached `UrlChanged`, `TitleChanged`, and `LoadingState`.
- Direct PDF navigation still followed the unchanged content_shell download path
  and did not hang before artifact capture.

The known teardown `SEGV_ACCERR` recurred after screenshot/log capture. This is
the same residual automation cleanup crash seen in prior PDF experiments and did
not block evidence collection.

Bookkeeping status before final commit:

- Chromium branch: `148.0.7778.97-issue-792-exp8`
- Chromium build: passed
- Main issue README status: updated to `Partial`
- Patch archive: regenerated in `chromium/patches/issue-792/`
- `chromium/README.md`: updated to current branch

## Conclusion

Experiment 8 ruled out the next obvious wrong layer. The missing
`SaveRequestType` is not simply because TermSurf lacked Chrome's generated
schemas, Chrome's API feature JSON pak, or the PDF viewer's private permissions.
Those are now wired.

The next break is somewhere between "Chrome API provider is present" and
"renderer binding system asks for the `pdfViewerPrivate` schema." Renderer
activation is the leading hypothesis because Experiment 6 inserted the PDF
extension into the browser-side `ProcessMap`, but the renderer may still not
activate that extension before the PDF viewer script context is created. If the
renderer context is classified as an unprivileged extension context instead of
`privileged_extension`, APIs gated to `"contexts": ["privileged_extension"]`
will not be exposed, and the binding system will never ask for the
`pdfViewerPrivate` schema.

That is plausible, but not yet proven. Other gates could still explain the same
symptom: Chrome API feature JSON could load but not parse into the feature map,
the PDF extension might not be broadcast to the renderer, the renderer might
receive but not activate it, the script context might be classified incorrectly,
or `pdfViewerPrivate` might fail an extension-type, manifest-version, location,
or allowlist gate.

The next experiment should therefore diagnose the full chain before fixing one
link. It should add targeted logs for:

- Chrome API feature JSON parse / feature count;
- browser-side PDF extension load broadcast;
- browser-side `RendererStartupHelper::ActivateExtensionInProcess()`;
- renderer-side extension load and activation receipt;
- PDF viewer script-context classification;
- `pdfViewerPrivate` feature availability result and reason.

After that diagnostic identifies the first broken gate, the experiment should
fix only that gate and re-run the direct extension smoke. Success for the next
slice means `schema-request name=pdfViewerPrivate found=1` appears and the
`SaveRequestType` error disappears. The next blocker after that may be a missing
browser-side implementation of `chrome.resourcesPrivate.*`,
`chrome.pdfViewerPrivate.*`, or `chrome.metricsPrivate.*`.
