# Experiment 15: Wire PDF Title Propagation

## Description

Experiment 13 and Experiment 14 both classified PDF title behavior as
`title-extension-only`: the PDF extension frame has a useful title such as
`bitcoin.pdf`, but the top-level TermSurf tab title remains empty. The current
result is user-visible because webtui/TermSurf title state follows the top-level
`TitleChanged` path, not the child extension frame's `document.title`.

The current Chromium branch already exposes the `pdfViewerPrivate` JavaScript
API schema to the PDF extension. However, TermSurf's browser-side PDF extension
API provider currently registers only `resourcesPrivate.getStrings`. It does not
register the Chrome browser implementation for
`pdfViewerPrivate.setPdfDocumentTitle`. Chromium's canonical implementation of
that function updates the top-level `WebContents` title for full-page PDFs and
rejects embedded PDFs by checking that the sender `WebContents` MIME type is
`application/pdf`.

Do not register Chrome's whole `pdfViewerPrivate` implementation unit as the
default path. The Chrome class is title-specific at the function-registry layer,
but it lives beside the broader `pdfViewerPrivate` backend implementation for
stream info, save-to-drive, plugin attributes, and other browser features. The
preferred fix is a TermSurf-owned title-only extension function that mirrors the
canonical title update logic and guard without importing unrelated behavior.

Experiment 15 should wire the smallest title-specific slice of Chrome's
`pdfViewerPrivate` browser API into TermSurf, prove that the PDF viewer calls
it, and prove that the resulting title reaches the TermSurf title callback and
the top-level DevTools target. This is not a print experiment, not an OOPIF
experiment, and not a general `pdfViewerPrivate` enablement sweep.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a new Chromium branch.

   Fork the current good Issue 794 Chromium branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp14
   git checkout -b 148.0.7778.97-issue-794-exp15
   ```

   Add the branch to `chromium/README.md` with a description such as "Wire PDF
   title propagation."

2. Add title-path diagnostics before making the functional change.

   Instrument the title path narrowly enough to answer:
   - Does the PDF extension call `pdfViewerPrivate.setPdfDocumentTitle(...)`?
   - If the browser function runs, what title does it receive?
   - What `WebContents` does `GetSenderWebContents()` return?
   - What is that `WebContents` MIME type?
   - Is the last committed entry present before and after the title update?
   - Does `TsTabObserver::TitleWasSet()` fire after the update?
   - Does `TsNotifyTitleChanged(...)` send the title to Roamium?

   Use a consistent log prefix such as `[issue-794-exp15]`. Keep the logs
   specific to this path; do not add broad extension dispatch logging.

3. Add and register only a TermSurf-owned title function backend.

   Add a narrow TermSurf browser extension function, for example:
   - `content/libtermsurf_chromium/extensions/ts_pdf_viewer_private_api.h`
   - `content/libtermsurf_chromium/extensions/ts_pdf_viewer_private_api.cc`

   It should declare the same extension function name:

   ```cpp
   DECLARE_EXTENSION_FUNCTION("pdfViewerPrivate.setPdfDocumentTitle", ...)
   ```

   Its `Run()` implementation should mirror Chromium's canonical title logic
   from
   `chrome/browser/extensions/api/pdf_viewer_private/pdf_viewer_private_api.cc`:
   - get `content::WebContents* web_contents = GetSenderWebContents()`;
   - reject missing `WebContents`;
   - validate `web_contents->GetContentsMimeType() == pdf::kPDFMimeType`;
   - parse `api::pdf_viewer_private::SetPdfDocumentTitle::Params`;
   - call `web_contents->UpdateTitleForEntry(...)` with the parsed title;
   - return `RespondNow(NoArguments())`.

   Then register this TermSurf-owned function in
   `content/libtermsurf_chromium/extensions/ts_extensions_browser_client.cc`.

   Do not register Chrome's full browser API provider and do not register every
   `pdfViewerPrivate` function in this experiment. Also do not directly register
   Chrome's `PdfViewerPrivateSetPdfDocumentTitleFunction` unless a dependency
   audit proves it does not pull the broader Chrome PDF/private API stack into
   TermSurf; the expected path is the TermSurf-owned title-only function.

4. Preserve Chromium's full-page PDF guard.

   Keep Chromium's existing full-page-only rule:

   ```cpp
   web_contents->GetContentsMimeType() == pdf::kPDFMimeType
   ```

   This prevents embedded PDFs inside ordinary HTML pages from overwriting the
   host page title. If the canonical Chrome function is used directly, do not
   relax or bypass this check. If the TermSurf-owned function is used, copy this
   guard exactly.

5. Ensure TermSurf emits a `TitleChanged` message.

   First, verify whether `web_contents->UpdateTitleForEntry(...)` causes
   `TsTabObserver::TitleWasSet()` to fire in the TermSurf embedder. If it does,
   use that existing path.

   If the entry title changes but `TitleWasSet()` does not fire, add a
   TermSurf-owned narrow bridge at the title function call site so full-page PDF
   title updates call `TsNotifyTitleChanged(...)` for the same top-level
   `WebContents` handle that Roamium uses. Do not add a new protobuf message;
   use the existing `TitleChanged` protocol path.

   If a direct notification is needed, keep it co-located with the title
   function and document why it exists: PDF title updates originate from the PDF
   extension frame, and TermSurf's observer path may not see extension-frame
   `document.title` changes.

6. If the viewer never calls the title API, wire the full-page viewer path.

   The first diagnostic run may prove that the browser function is registered
   but never called. In that case, inspect
   `chrome/browser/resources/pdf/pdf_viewer.ts::setDocumentMetadata_()`.

   TermSurf deliberately keeps `pdfOopifEnabled` false because enabling it
   switches the viewer to the still-unimplemented
   `pdfViewerPrivate.getStreamInfo()` OOPIF path. Chromium's bundled viewer
   currently calls `setPdfDocumentTitle(...)` only when `pdfOopifEnabled` is
   true and the PDF is not embedded. TermSurf's viewer is also hosted in an
   extension child frame when `pdfOopifEnabled` is false, so `document.title`
   updates only the child frame.

   If this is the observed stopping layer, patch the viewer narrowly:
   - always keep `document.title = this.title_` so the extension child target
     still has the correct local title;
   - for `!this.embedded_`, call
     `PdfViewerPrivateProxyImpl.getInstance().setPdfDocumentTitle(this.title_)`;
   - do not enable `pdfOopifEnabled`;
   - do not call the title API for embedded PDFs.

7. Extend the automated probe for title evidence.

   Update `scripts/probe-pdf-save-print-title-local.mjs` so the title section
   records:
   - top-level DevTools target title;
   - top-level `document.title`;
   - PDF extension child target title;
   - PDF extension `document.title`;
   - whether `pdfViewerPrivate.setPdfDocumentTitle` exists in the extension
     frame;
   - console errors mentioning `pdfViewerPrivate.setPdfDocumentTitle`,
     `Unknown Extension API`, or extension function dispatch failures;
   - the title classification.

   Preserve the existing `title-propagated`, `title-extension-only`,
   `title-api-missing`, and `title-unobserved` classification names, but tighten
   `title-propagated` for this experiment. A PDF title is propagated only when
   all three authoritative user-visible layers agree on the PDF title or
   expected filename fallback:
   - the PDF extension child title;
   - the top-level DevTools target title;
   - TermSurf `TitleChanged` evidence from logs/protocol.

   Top-level `document.title` is useful supporting evidence, but it is not
   required for `title-propagated` because the top-level PDF document may be a
   viewer/wrapper document rather than ordinary HTML.

8. Add an embedded-PDF title regression check.

   Add a small local HTML fixture to the existing test server with:

   ```html
   <title>Embedded PDF Host</title>
   <embed src="/bitcoin.pdf" type="application/pdf" />
   ```

   The probe should load this page after the full-page PDF title check. It must
   prove both that the embedded PDF path was actually exercised and that the
   host title was preserved:
   - the `<embed>` exists and has a non-empty rendered rectangle;
   - an embedded PDF extension child/frame/plugin is present, or the probe
     records an equivalent positive signal that the PDF viewer path loaded;
   - the PDF extension child title is recorded if available;
   - the top-level title remains `Embedded PDF Host`;
   - no top-level TermSurf `TitleChanged` event changes the title to the PDF
     title/fallback;
   - if the title function is called from the embedded case, the
     `[issue-794-exp15]` logs show the full-page MIME guard rejected it.

   If the embedded fixture never enters the PDF viewer path, this check is
   inconclusive and the experiment cannot claim embedded-title safety.

9. Build Chromium and Roamium.

   Use the Chromium skill's build rule:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Then rebuild Roamium:

   ```bash
   cd /Users/ryan/dev/termsurf
   CARGO_BIN="$(/opt/homebrew/bin/rustup which cargo)"
   PATH="$(dirname "$CARGO_BIN"):$PATH" ./scripts/build.sh roamium
   ```

   Never use `ninja` directly. Run `cargo fmt` if any Rust changes are made and
   accept its output.

10. Verify with the save/print/title/local probe.

Re-run:

```bash
LOG_DIR="logs/issue-794-exp15-title-$(date +%Y%m%d-%H%M%S)" \
  scripts/test-issue-794-pdf-toolbar.py \
  --probe save-print-title-local \
  --log-dir "$LOG_DIR" \
  --serve-bitcoin-pdf
```

Required full-page title checks:

- the baseline Bitcoin PDF classification is `title-propagated`;
- `title-propagated` means the PDF extension child title, top-level DevTools
  target title, and TermSurf `TitleChanged` evidence all match the PDF title or
  expected filename fallback;
- top-level `document.title` matches the propagated title if observable, but
  absence of this signal is not a failure by itself;
- the PDF extension child title still matches the propagated title;
- no console error reports an unknown or failed
  `pdfViewerPrivate.setPdfDocumentTitle` call;
- the `[issue-794-exp15]` logs show the title function was called and the
  TermSurf title callback path was reached.

Required embedded-PDF regression check:

- the embedded fixture enters the PDF viewer path;
- the host HTML page title remains `Embedded PDF Host`;
- the embedded PDF does not overwrite the top-level TermSurf title;
- if the embedded PDF's own extension frame has a title, that remains local to
  the extension frame.

11. Run focused regressions.

    Re-run enough existing automated checks to prove this title-only change did
    not disturb the PDF viewer:
    - PDF render/string placeholder check from Experiment 14;
    - PDF toolbar zoom/final controls from Experiment 12;
    - PDF wheel scroll from Experiment 4;
    - normal HTML page title behavior using an ordinary HTML page with a
      `<title>` element.

12. Archive Chromium patches only after a coherent branch result.

    If the experiment passes or produces a coherent partial branch, commit the
    Chromium branch and regenerate:

    ```bash
    cd chromium/src
    rm -rf ../../chromium/patches/issue-794-exp15/
    git format-patch 148.0.7778.97..HEAD \
      -o ../../chromium/patches/issue-794-exp15/
    ```

    Update `chromium/README.md` in the main repo.

13. Formatting and review.

    If Markdown changes, run Prettier. If Rust changes, run `cargo fmt` and
    accept its output. If Chromium C++ changes, run Chromium formatting on the
    modified files if practical.

    Codex must review the completed output before Experiment 16 is designed.

## Verification

| Check                                                       | Required result                |
| ----------------------------------------------------------- | ------------------------------ |
| Codex design review completed                               | Yes                            |
| Chromium branch exists and is recorded                      | Yes                            |
| `pdfViewerPrivate.setPdfDocumentTitle` browser function run | Yes                            |
| Full-page PDF title classification                          | `title-propagated`             |
| Top-level DevTools target title                             | PDF title or expected fallback |
| TermSurf `TitleChanged` callback/protocol evidence          | Present                        |
| Embedded PDF viewer path exercised                          | Yes                            |
| Embedded PDF host page title                                | Preserved                      |
| Unknown extension API errors                                | None for title                 |
| PDF toolbar/string regression                               | Pass                           |
| PDF wheel scroll regression                                 | Pass                           |
| Normal HTML title regression                                | Pass                           |
| Codex completion review completed                           | Yes                            |

## Pass Criteria

This experiment passes if full-page PDFs propagate their title through the
top-level TermSurf title path, embedded PDFs do not overwrite their host page
title, and the focused PDF/HTML regressions still pass.

## Partial Criteria

This experiment is partial if the title function is now registered and called,
but title propagation still stops at a specifically identified layer, such as:

- `UpdateTitleForEntry(...)` updates the `NavigationEntry`, but `TitleWasSet()`
  does not fire;
- `TsNotifyTitleChanged(...)` fires, but Roamium does not forward
  `TitleChanged`;
- Roamium forwards `TitleChanged`, but webtui still displays the old title;
- full-page title propagation works, but an embedded PDF incorrectly overwrites
  the host HTML title.

Record the exact stopping layer and design the next experiment around that layer
only.

## Failure Criteria

This experiment fails if:

- it registers Chrome's full browser API provider or broad `pdfViewerPrivate`
  surface instead of the title-specific backend;
- it directly links Chrome's broad `pdf_viewer_private_api.cc` implementation
  without first proving that doing so does not import unrelated
  `pdfViewerPrivate` behavior;
- it relaxes the full-page PDF MIME guard and lets embedded PDFs change host
  page titles;
- it adds a new TermSurf protobuf title message instead of using the existing
  `TitleChanged` path;
- it enables OOPIF PDF mode or print behavior as part of a title fix;
- PDF rendering, toolbar controls, or wheel scroll regress.

## Result

**Result:** Pass

Experiment 15 fixed PDF title propagation.

The first implementation pass registered a TermSurf-owned
`pdfViewerPrivate.setPdfDocumentTitle` browser function, but the probe still
reported `title-extension-only` and Chromium logged
`Unknown Extension API - pdfViewerPrivate.setPdfDocumentTitle`. The missing
registration layer was `TsExtensionsBrowserClient::Init()`, which directly
registers concrete extension functions. Adding the title function there made the
browser backend reachable.

The next run proved another hidden layer: Chromium's bundled PDF viewer only
calls `setPdfDocumentTitle(...)` when `pdfOopifEnabled` is true and the PDF is
not embedded. TermSurf keeps `pdfOopifEnabled` false because enabling it moves
the viewer to the unimplemented OOPIF `getStreamInfo()` path. The fix therefore
keeps `document.title = this.title_` for the extension child frame and calls the
title API for `!this.embedded_` full-page PDFs without enabling OOPIF.

Implemented changes:

- added `TsPdfViewerPrivateSetPdfDocumentTitleFunction`, a TermSurf-owned
  title-only extension function that mirrors Chromium's full-page MIME guard;
- registered that function in both TermSurf's PDF extension API provider and the
  concrete `TsExtensionsBrowserClient::Init()` registry path;
- patched `pdf_viewer.ts::setDocumentMetadata_()` so full-page, non-embedded
  PDFs call `setPdfDocumentTitle(...)` even when `pdfOopifEnabled` is false;
- added trace evidence in `TsTabObserver::TitleWasSet()` and Roamium's
  `on_title_changed()` path;
- extended the save/print/title/local probe to require agreement between the PDF
  extension child title, top-level DevTools target title, and TermSurf
  `TitleChanged` trace evidence;
- added an embedded-PDF host-title regression fixture.

Verification:

| Check                                 | Result | Evidence                                                                                                                                                                                   |
| ------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Chromium build                        | Pass   | `autoninja -C out/Default libtermsurf_chromium` succeeded                                                                                                                                  |
| Roamium build                         | Pass   | `./scripts/build.sh roamium` succeeded with the cargo workaround                                                                                                                           |
| Full-page HTTP PDF title              | Pass   | `logs/issue-794-exp15-title-20260530-125456/save-print-title-local/save-print-title-local-summary.json` reports `title-propagated`, top title `bitcoin.pdf`, extension title `bitcoin.pdf` |
| Full-page `file://` PDF title         | Pass   | same summary reports `file-pdf` as `title-propagated`                                                                                                                                      |
| HTTP extensionless PDF title          | Pass   | same summary reports `http-extensionless` as `title-propagated`                                                                                                                            |
| `file://` extensionless PDF title     | Pass   | same summary reports `file-extensionless` as `title-propagated`                                                                                                                            |
| HTTP untitled/fallback PDF title      | Pass   | same summary reports `http-untitled` as `title-propagated` with `untitled.pdf`                                                                                                             |
| `file://` untitled/fallback PDF title | Pass   | same summary reports `file-untitled` as `title-propagated` with `untitled.pdf`                                                                                                             |
| TermSurf `TitleChanged` evidence      | Pass   | `pdf-input.log` contains matching `title-changed ... title=...` lines                                                                                                                      |
| Embedded PDF host title               | Pass   | same summary reports embedded status `pass`, top title `Embedded PDF Host`, `embeddedPluginLogged=true`, `overwrittenByPdf=false`                                                          |
| Unknown title API errors              | Pass   | final run has no `Unknown Extension API - pdfViewerPrivate.setPdfDocumentTitle` errors                                                                                                     |
| Save/download regression              | Pass   | same summary reports `download-file-created`                                                                                                                                               |
| PDF wheel scroll regression           | Pass   | `logs/issue-794-exp15-regression-scroll-20260530-125656/protocol-scroll-summary.json` reports `first_failing_hop=no-failure-observed`                                                      |
| PDF toolbar event regression          | Pass   | `logs/issue-794-exp15-regression-toolbar-events-20260530-125820/toolbar-events/toolbar-events-summary.json` reports `status=pass`                                                          |
| Codex completion review               | Pass   | `logs/codex-review/20260530-130050-540860-last-message.md` reports no blocking findings                                                                                                    |

The broad save/print/title/local probe's top-level `status` remains `partial`
because print is still classified as `print-not-contained`; print is explicitly
out of scope for this title experiment. The title-specific field
`titlePropagationPass` is `true`.

## Conclusion

PDF title propagation is now complete for the current TermSurf PDF viewer path.
The important lesson is that TermSurf's current PDF viewer is a non-OOPIF child
frame, so Chrome's upstream title condition was too narrow for us. The fix keeps
OOPIF disabled, avoids broad `pdfViewerPrivate` enablement, and only uses the
new title backend for full-page PDFs. Embedded PDFs preserve the host page
title.

The remaining Issue 794 product gap is contained print behavior.
