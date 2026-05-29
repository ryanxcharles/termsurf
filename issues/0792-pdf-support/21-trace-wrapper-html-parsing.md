# Experiment 21: Trace Wrapper HTML Parsing

## Description

Experiment 20 fixed the missing wrapper resource. The intercepted PDF response
now receives the expected OOPIF PDF wrapper HTML:

```text
[issue-792-exp20] components-resource-pak ... loaded=1 pdf_embedder_bytes=463 has_iframe=1 has_about_blank=1
[issue-792-exp19] wrapper-payload ... bytes=536 has_template=1 has_iframe=1 has_about_blank=1 has_internal_id=1 has_pdf_extension_url=1
```

But the wrapper still does not produce a child `about:blank` frame:

```text
[issue-792-exp19] pvs-finish frame_tree_node_id=1 url=http://127.0.0.1:9787/bitcoin.pdf ... has_parent=0 ...
[issue-792-exp19] pvs-finish-no-parent frame_tree_node_id=1 url=http://127.0.0.1:9787/bitcoin.pdf
```

No child `pvs-finish` appears, so `PdfViewerStreamManager` never reaches
`pdf-extension-about-blank` or `pdf-extension-navigate`.

The next question is whether Blink parses the wrapper response as HTML and
creates the iframe inside the declarative shadow root. The response head still
carries `application/pdf`, but Chromium's OOPIF PDF path relies on the PDF MIME
type becoming an HTML wrapper document rather than a `PluginDocument`. Blink can
arrive there through two different paths:

```text
application/pdf
  -> Chrome path: plugin data supports MIME and says MIME is external
  -> TermSurf possible path: plugin data does not support application/pdf,
     so ComputeDocumentType() falls through to the default HTML result
  -> DocumentInit type is kHTML either way
  -> wrapper body is parsed as HTML
  -> declarative shadow root attaches
  -> iframe inside shadow root is inserted
  -> iframe navigates to about:blank
```

In Chrome's full browser path, PDF plugin data can set
`is_for_external_handler=true`. TermSurf's Experiment 15 internal plugin
registration covered `application/x-google-chrome-pdf`, so `application/pdf` may
instead reach `Type::kHTML` through Blink's default fallthrough with
`is_for_external_handler=false`. That can still be acceptable if the wrapper is
parsed as HTML. The diagnostic must distinguish these paths rather than assume
`is_for_external_handler=true`.

Experiment 21 adds diagnostics across exactly those gates. It must not change
the MIME type, wrapper body, stream container, plugin registration, or
navigation behavior. The result should tell the next experiment whether to fix
plugin MIME registration, HTML parsing, declarative shadow-root attachment, or
iframe frame creation.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 20 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp20
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp21
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Instrument Blink document-type selection for PDF.

   In `third_party/blink/renderer/core/dom/document_init.cc`, add narrow logs in
   or around `DocumentInit::ComputeDocumentType(...)` and
   `DocumentInit::WithTypeFrom(...)` when `mime_type == "application/pdf"` or
   when `is_for_external_handler` becomes true.

   Required logs:

   ```text
   [issue-792-exp21] document-type-check mime_type=<mime> has_frame=<0|1> allow_plugins=<0|1> has_plugin_data=<0|1> supports_mime=<0|1> is_external=<0|1> result=<html|plugin|text|other>
   [issue-792-exp21] document-type-selected mime_type=<mime> result=<html|plugin|text|other> is_for_external_handler=<0|1>
   ```

   Do not change the result. This log answers whether Blink classifies
   `application/pdf` as an external-handler HTML wrapper document.

3. Instrument wrapper document commit and parser creation.

   In `third_party/blink/renderer/core/loader/document_loader.cc`, add
   issue-tagged logs around the wrapper navigation path:
   - after `InstallNewDocument(...)` in `DocumentLoader::CommitNavigation()`;
   - after `document->OpenForNavigation(...)` in the body-loading path that
     creates the parser.

   Required logs:

   ```text
   [issue-792-exp21] document-commit url=<url> mime_type=<mime> document_class=<html|plugin|text|other> is_for_external_handler=<0|1> child_count=<n>
   [issue-792-exp21] document-parser-open url=<url> mime_type=<mime> parser=<html|text|other>
   ```

   If `Document` does not expose `is_for_external_handler`, omit that field from
   `document-commit` rather than adding new state. This is diagnostic-only.

4. Instrument declarative shadow-root attachment.

   In `third_party/blink/renderer/core/html/parser/html_construction_site.cc`,
   add a narrow log around the declarative shadow-root branch that calls
   `AttachDeclarativeShadowRoot(...)`.

   Required log:

   ```text
   [issue-792-exp21] declarative-shadow-root url=<document-url> host_tag=<tag> mode=<open|closed|none> success=<0|1> should_attach_template=<0|1>
   ```

   This answers whether the wrapper's `<template shadowrootmode="closed">`
   attaches to the body host or remains an inert template.

5. Instrument iframe/frame-owner creation.

   In `third_party/blink/renderer/core/html/html_frame_owner_element.cc`, add
   issue-tagged logs around:
   - `HTMLFrameOwnerElement::InsertedInto(...)`;
   - `HTMLFrameOwnerElement::LoadOrRedirectSubframe(...)`.

   Required logs:

   ```text
   [issue-792-exp21] frame-owner-inserted document_url=<url> tag=<tag> name=<name> src=<src> type=<type> internalid=<value> is_connected=<0|1>
   [issue-792-exp21] load-or-redirect-subframe document_url=<url> tag=<tag> frame_name=<name> url=<url> internalid=<value> result=<0|1>
   ```

   These logs answer whether the iframe element exists and whether Blink starts
   the `about:blank` subframe load.

   If these logs are noisy on non-PDF pages, gate them on the element having a
   non-empty `internalid` attribute. The PDF wrapper iframe has that attribute,
   while ordinary page iframes usually do not.

6. Preserve prior PDF diagnostics.

   Keep the Experiment 19 and 20 logs. The result needs the full chain:

   ```text
   components-resource-pak loaded=1
   wrapper-payload has_iframe=1
   document-type-selected ...
   document-parser-open ...
   declarative-shadow-root ...
   frame-owner-inserted ...
   load-or-redirect-subframe ...
   pvs-finish-about-blank / or first missing gate
   ```

7. Build and archive only after the result is accepted.

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
   LOG_DIR="logs/issue-792-exp21-pdf-$(date +%Y%m%d-%H%M%S)"
   scripts/test-issue-792-fake-gui.py \
     http://127.0.0.1:9787/bitcoin.pdf \
     --serve-bitcoin-pdf \
     --log-dir "$LOG_DIR" \
     --seconds 18
   ```

3. Inspect `roamium.stderr`.

   The required setup chain is:

   ```text
   [issue-792-exp20] components-resource-pak ... loaded=1 ...
   [issue-792-exp19] wrapper-payload ... has_iframe=1 ...
   ```

   Then classify the first missing parser/frame gate:
   - If `document-type-selected ... result=plugin`, the external PDF MIME
     registration is not visible to Blink; the next experiment should fix
     renderer plugin registration / external MIME visibility.
   - If `document-type-selected ... result=html` appears but
     `document-parser-open ... parser=html` does not, the next experiment should
     inspect the document-loader parser path.
   - If the HTML parser opens but `declarative-shadow-root` is absent, the
     wrapper body is not reaching the declarative shadow-root parser branch.
   - If `declarative-shadow-root ... success=0`, the body host is not a valid
     declarative shadow-root host or the template is parsed in the wrong
     location.
   - If the shadow root attaches but `frame-owner-inserted` is absent for the
     iframe, the iframe inside the shadow root is not becoming a live frame
     owner.
   - If `frame-owner-inserted` appears but `load-or-redirect-subframe` is
     absent, iframe insertion is not triggering subframe load.
   - If `load-or-redirect-subframe ... result=0`, the subframe load is blocked;
     the next experiment should inspect that return path.
   - If `load-or-redirect-subframe ... result=1` appears but no child
     `pvs-finish` appears, the browser-side navigation for the child is being
     lost after Blink starts it.
   - If `pvs-finish-about-blank` and `pdf-extension-navigate` appear, the parser
     gate is fixed and the next gate is extension viewer startup / stream-info.

4. Run the normal HTML smoke test:

   ```bash
   LOG_DIR="logs/issue-792-exp21-html-$(date +%Y%m%d-%H%M%S)"
   scripts/test-issue-792-fake-gui.py \
     http://localhost:9616/index.html \
     --log-dir "$LOG_DIR" \
     --seconds 8
   ```

   Non-PDF HTML may emit generic `document-commit` or parser logs if the
   implementation cannot cheaply filter them to PDF/external-handler documents.
   It must not emit PDF-wrapper logs (`wrapper-payload`, `pvs-claim`,
   `pdf-extension-*`) for normal HTML.

5. Record the result in this file.

   The result must include:
   - the exact PDF and HTML log directories;
   - whether `application/pdf` is classified as HTML or plugin;
   - whether the wrapper opens an HTML parser;
   - whether the declarative shadow root attaches;
   - whether the iframe is inserted and attempts `about:blank` load;
   - the first missing transition;
   - the concrete next experiment implied by that transition.

## Result

**Result:** Pass

Build:

```text
autoninja -C out/Default libtermsurf_chromium
Build Succeeded: 3 steps
```

The first PDF run showed no Experiment 21 Blink logs even though the markers
were present in `libblink_core.dylib`. That proved the original log gate was too
narrow: the wrapper document does not arrive at `DocumentLoader` as an
`application/pdf` / external-handler document. The diagnostic was broadened to
also log the original `bitcoin.pdf` URL and the PDF extension ID. That is still
trace-only; it does not change navigation, MIME handling, response bodies, or
frame creation.

Final PDF log:

```text
logs/issue-792-exp21-pdf-20260529-152702
```

HTML control log:

```text
logs/issue-792-exp21-html-20260529-152725
```

Important PDF trace lines:

```text
[issue-792-exp20] components-resource-pak ... loaded=1 pdf_embedder_bytes=463 has_iframe=1 has_about_blank=1
[issue-792-exp19] wrapper-payload ... bytes=536 has_template=1 has_iframe=1 has_about_blank=1 has_internal_id=1 has_pdf_extension_url=1
[issue-792-exp21] document-commit url=http://127.0.0.1:9787/bitcoin.pdf mime_type=text/html document_class=html is_for_external_handler=0 child_count=0
[issue-792-exp21] document-parser-open url=http://127.0.0.1:9787/bitcoin.pdf mime_type=text/html parser=html
[issue-792-exp19] pvs-finish frame_tree_node_id=1 url=http://127.0.0.1:9787/bitcoin.pdf has_committed=1 is_error_page=0 is_pdf=0 has_parent=0 parent_frame_tree_node_id=none stream_count=1
[issue-792-exp19] pvs-finish-no-parent frame_tree_node_id=1 url=http://127.0.0.1:9787/bitcoin.pdf
```

Absent from the PDF log:

```text
[issue-792-exp21] document-type-selected ...
[issue-792-exp21] declarative-shadow-root ...
[issue-792-exp21] frame-owner-inserted ...
[issue-792-exp21] load-or-redirect-subframe ...
```

The HTML control emitted no PDF wrapper logs.

## Conclusion

Experiment 21 proved that Experiment 20 got past the missing-resource problem:
the wrapper payload is non-empty, and Blink commits the original PDF navigation
as an HTML document with an HTML parser.

The MIME transition is important. At the browser-side throttle, the response is
still classified as `application/pdf`; by the time `DocumentLoader` commits the
same original PDF URL, Blink reports `mime_type=text/html`. That is a positive
signal: the wrapper body is being recognized as HTML, so the next layer is not
PDF MIME classification or `DocumentInit::ComputeDocumentType`.

The first missing transition is after `document-parser-open` and before
`declarative-shadow-root`. The wrapper response has an HTML parser, but the
`<template shadowrootmode="closed">` branch is never reached, so no shadow root
attaches, no iframe frame owner is inserted, no `about:blank` child frame is
created, and `PdfViewerStreamManager::DidFinishNavigation` still sees only the
top-level original PDF frame.

The next experiment should inspect why the generated wrapper body is not
reaching the normal HTML tree-builder path despite being delivered as the body
for an HTML document. The likely gates are the substituted data pipe/body-loader
handoff and the MIME-handler wrapper response completion ordering, not PDF
extension startup or resource loading.
