# Experiment 20: Load Components Resource Pak

## Description

Experiment 19 proved that the OOPIF PDF wrapper body is empty:

```text
[issue-792-exp19] wrapper-payload ... bytes=0 has_template=0 has_iframe=0 has_about_blank=0
```

That explains why the pipeline stops after stream claim. The wrapper HTML never
contains the child `about:blank` iframe, so `PdfViewerStreamManager` has no
child frame to observe and never calls `NavigateToPdfExtensionUrl(...)`.

The missing resource is known. Chromium declares `IDR_PDF_EMBEDDER_HTML` in
`components/resources/pdf_resources.grdp`, and the generated header confirms it
lives in:

```text
out/Default/gen/components/grit/components_resources.h
```

The corresponding resource pack is:

```text
out/Default/gen/components/components_resources.pak
```

TermSurf currently loads:

```text
gen/chrome/pdf_resources.pak
gen/chrome/common_resources.pak
gen/extensions/extensions_renderer_resources.pak
```

It does not load `gen/components/components_resources.pak`. Experiment 20 loads
that pak in the existing PDF resource-bundle setup. This is the smallest
mechanical fix implied by Experiment 19.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 19 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp19
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp20
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Add the narrow resource dependency if needed.

   In `content/libtermsurf_chromium/BUILD.gn`, add the narrowest dependency that
   guarantees `gen/components/components_resources.pak` and
   `gen/components/grit/components_resources.h` are generated for
   `libtermsurf_chromium`.

   Expected dependency:

   ```gn
   "//components/resources:components_resources",
   ```

   Do not add broad Chrome browser-resource dependencies. The target resource is
   a components resource, not a PDF extension resource.

3. Load `components_resources.pak`.

   In `content/libtermsurf_chromium/extensions/ts_pdf_resource_bundle.cc`:
   - include `components/grit/components_resources.h`;
   - add the new load inside `LoadTsPdfResourceBundle()`, alongside the existing
     PDF, common, and extensions-renderer pak loads;
   - construct the path:

     ```text
     <DIR_ASSETS>/gen/components/components_resources.pak
     ```

   - if present, call `AddDataPackFromPath(..., ui::kScaleFactorNone)`;
   - verify loading by reading `IDR_PDF_EMBEDDER_HTML`;
   - log:

     ```text
     [issue-792-exp20] components-resource-pak path=<path> found=<0|1> loaded=<0|1> pdf_embedder_bytes=<n> has_iframe=<0|1> has_about_blank=<0|1>
     ```

   `LoadTsPdfResourceBundle()` runs from `TsMainDelegate::PreSandboxStartup()`,
   before any URL loader throttle can invoke
   `MimeHandlerViewAttachHelper::CreateTemplateMimeHandlerPage(...)`.

4. Preserve Experiment 19 diagnostics.

   Do not remove the `wrapper-payload` log yet. It is the verification gate for
   this fix.

5. Do not change PDF navigation behavior directly.

   This experiment is a resource-loading fix only. Do not modify
   `PdfViewerStreamManager`, `TsPluginResponseInterceptorURLLoaderThrottle`, the
   renderer plugin predicates, or stream-info binders except for compile fallout
   caused by the resource include/dependency.

6. Build and archive only after the result is accepted.

   Build with:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   If the experiment passes or produces a coherent partial branch, commit the
   Chromium branch and regenerate:

   ```bash
   rm -rf ../../chromium/patches/issue-792/
   git format-patch 148.0.7778.97..HEAD -o ../../chromium/patches/issue-792/
   ```

## Verification

1. Build `libtermsurf_chromium` with `autoninja`.

2. Run the fake-GUI PDF smoke test against the local bitcoin PDF fixture:

   ```bash
   LOG_DIR="logs/issue-792-exp20-pdf-$(date +%Y%m%d-%H%M%S)"
   scripts/test-issue-792-fake-gui.py \
     http://127.0.0.1:9787/bitcoin.pdf \
     --serve-bitcoin-pdf \
     --log-dir "$LOG_DIR" \
     --seconds 18
   ```

3. Inspect `roamium.stderr`.

   The required resource-loading proof is:

   ```text
   [issue-792-exp20] components-resource-pak ... found=1 loaded=1 pdf_embedder_bytes=<nonzero> has_iframe=1 has_about_blank=1
   ```

   The required wrapper proof is:

   ```text
   [issue-792-exp19] wrapper-payload ... bytes=<nonzero> has_template=1 has_iframe=1 has_about_blank=1 has_internal_id=1 has_pdf_extension_url=1
   ```

4. Classify the PDF pipeline after the wrapper is non-empty.
   - If `wrapper-payload` is still empty, this experiment fails; the pak path,
     dependency, or load timing is wrong.
   - If the wrapper is non-empty but no child `pvs-finish` appears, the next
     blocker is wrapper parsing or declarative shadow DOM child-frame creation.
   - If `pvs-finish-about-blank` and `pdf-extension-navigate` appear, the fix
     worked and the pipeline advanced to extension navigation.
   - If `pdf-extension-navigate` appears but no stream-info API is requested,
     the next blocker is extension viewer startup after navigation.
   - If stream-info is requested, continue following the existing Experiment 18
     stream-info logs.

5. Run the normal HTML smoke test:

   ```bash
   LOG_DIR="logs/issue-792-exp20-html-$(date +%Y%m%d-%H%M%S)"
   scripts/test-issue-792-fake-gui.py \
     http://localhost:9616/index.html \
     --log-dir "$LOG_DIR" \
     --seconds 8
   ```

   Non-PDF navigation should not emit `wrapper-payload`, `pvs-claim`, or other
   PDF-specific transition logs.

6. Record the result in this file.

   The result must include:
   - the exact PDF and HTML log directories;
   - whether `components_resources.pak` was found and loaded;
   - whether `IDR_PDF_EMBEDDER_HTML` produced a non-empty wrapper;
   - the first PDF pipeline transition after wrapper creation;
   - whether the result is Pass, Partial, or Fail;
   - the concrete next experiment implied by the evidence.

## Result

Not run yet.

## Conclusion

Pending implementation.
