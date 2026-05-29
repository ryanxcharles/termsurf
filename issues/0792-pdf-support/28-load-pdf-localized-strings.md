# Experiment 28: Load PDF Localized Strings

## Description

Experiment 27 reached the PDF renderer and created the internal PDF plugin:

```text
[issue-792-exp16] pvs-start ... url=http://localhost:9616/bitcoin.pdf is_pdf=1
[issue-792-exp26] internal-plugin-create-check ... has_pdf_renderer=1
[issue-792-exp26] internal-plugin-create-result created=1
```

The remaining failure moved downstream into PDFium metadata:

```text
FATAL:ui/base/resource/resource_bundle.cc:1284] Check failed: !data->empty(). Unable to find resource: 38115.
chrome_pdf::FormatPageSize(...)
chrome_pdf::PdfViewWebPlugin::SendMetadata()
chrome_pdf::PdfViewWebPlugin::DocumentLoadComplete()
chrome_pdf::PDFiumEngine::FinishLoadingDocument()
```

Resource `38115` is `IDS_PDF_PROPERTIES_PAGE_SIZE_PORTRAIT` in the generated
`out/Default/gen/components/strings/grit/components_strings.h`. The resource is
present in:

```text
out/Default/gen/components/strings/components_strings_en-US.pak
```

The pre-check that proved this:

```bash
python3 - <<'PY'
import sys
sys.path.insert(0, 'tools/grit')
from grit.format import data_pack
pak='out/Default/gen/components/strings/components_strings_en-US.pak'
p=data_pack.ReadDataPack(pak)
print(38115 in p.resources, len(p.resources.get(38115,b'')), p.resources.get(38115,b'')[:40])
PY
```

Output:

```text
True 8 b'portrait'
```

TermSurf already loads the PDF viewer resource pak, Chrome common resources,
extensions renderer resources, and `components_resources.pak` in
`LoadTsPdfResourceBundle()`. Experiment 28 adds the missing components strings
pak to that same resource-bundle setup. The goal is to let
`chrome_pdf::FormatPageSize()` resolve the PDF metadata strings instead of
crashing before the first page can draw.

This experiment deliberately loads `en-US` only. Full locale negotiation can be
a follow-up; the current debug runs and automated fixtures are English, so
`en-US` is enough to unblock PDFium metadata for this slice.

This experiment must receive Claude design review before it runs. After the
result is recorded, Claude must review the completed output before any cleanup,
closure, or next experiment.

## Changes

1. Create a new Chromium branch from Experiment 27.

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-792-exp27
   git checkout -b 148.0.7778.97-issue-792-exp28
   ```

   Add the branch to `chromium/README.md`.

2. Update `content/libtermsurf_chromium/BUILD.gn`.

   Add the dependency that ensures the components localized strings pak and
   generated header are available:

   ```gn
   "//components/strings:components_strings",
   ```

3. Update `content/libtermsurf_chromium/extensions/ts_pdf_resource_bundle.cc`.

   Add:

   ```cpp
   #include "components/strings/grit/components_strings.h"
   ```

   In `LoadTsPdfResourceBundle()`, after loading `components_resources.pak`,
   load:

   ```text
   gen/components/strings/components_strings_en-US.pak
   ```

   using `ui::ResourceBundle::GetSharedInstance().AddDataPackFromPath(...)`.

   Then probe:

   ```cpp
   ui::ResourceBundle::GetSharedInstance().GetLocalizedString(
       IDS_PDF_PROPERTIES_PAGE_SIZE_PORTRAIT)
   ```

   and log:

   ```text
   [issue-792-exp28] components-strings-pak path=<path> found=<0/1> loaded=<0/1> portrait_bytes=<n>
   ```

   `loaded=1` means the string exists and is non-empty.

4. Keep the change narrowly scoped.

   Do not:
   - change `FormatPageSize()` or bypass metadata sending;
   - hard-code resource `38115` or hard-code the string `portrait`;
   - suppress the crash by catching or ignoring the failed resource lookup;
   - change PDF navigation, stream handling, plugin creation, or process
     assignment;
   - change Roamium Rust, Wezboard, webtui, or the TermSurf protocol.

5. Build Chromium:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

6. Regenerate the Issue 792 Chromium patch archive only after the Chromium
   branch commit:

   ```bash
   cd chromium/src
   rm -rf ../../chromium/patches/issue-792/
   git format-patch 148.0.7778.97..HEAD -o ../../chromium/patches/issue-792/
   ```

## Verification

1. Run the fake-GUI stream-info preflight:

   ```bash
   LOG_DIR="logs/issue-792-exp28-fakegui-$(date +%Y%m%d-%H%M%S)"
   scripts/test-issue-792-fake-gui.py \
     http://127.0.0.1:9787/bitcoin.pdf \
     --serve-bitcoin-pdf \
     --log-dir "$LOG_DIR" \
     --seconds 18
   ```

   Required:

   ```text
   [issue-792-exp28] components-strings-pak ... found=1 loaded=1
   real-mime-handler-get-stream-info has_stream=1
   [issue-792-exp27] internal-plugin-externalized handled=1
   [issue-792-exp16] pvs-start ... is_pdf=1
   [issue-792-exp26] internal-plugin-create-result created=1
   ```

   The fake-GUI run must not crash with missing resource `38115`.

2. Run the real-GUI DevTools HTML sanity check:

   ```bash
   TERMSURF_PDF_SETTLE_SECONDS=8 \
   LOG_DIR="logs/issue-792-exp28-html-devtools-$(date +%Y%m%d-%H%M%S)" \
   scripts/test-issue-792-devtools-screenshot.sh https://example.com
   ```

   The DevTools screenshot must show rendered `example.com`.

3. Run the real-GUI PDF DevTools capture:

   ```bash
   TERMSURF_PDF_SETTLE_SECONDS=18 \
   LOG_DIR="logs/issue-792-exp28-pdf-devtools-$(date +%Y%m%d-%H%M%S)" \
   scripts/test-issue-792-devtools-screenshot.sh http://localhost:9616/bitcoin.pdf
   ```

4. Inspect the PDF DevTools PNG with `view_image`.

   Classify it as:
   - **Rendered PDF:** recognizable Bitcoin PDF content is visible.
   - **Gray plugin rectangle:** the plugin exists but still does not draw PDF
     content.
   - **New missing resource:** another `Unable to find resource: <id>` crash
     appears.
   - **Renderer crash:** a non-resource crash occurs.
   - **Wrong target:** DevTools captured the wrong page.
   - **Automation failure:** no reliable DevTools PNG was produced.

5. Inspect PDF logs.

   Required for Pass:

   ```text
   [issue-792-exp28] components-strings-pak ... found=1 loaded=1
   [issue-792-exp26] internal-plugin-create-result created=1
   ```

   The logs must not contain:

   ```text
   Unable to find resource: 38115
   ```

6. Record the result in this file.

   Include:
   - Chromium branch name and commit;
   - build command and result;
   - fake-GUI log directory and resource/string result;
   - HTML DevTools screenshot path and classification;
   - PDF DevTools screenshot path and classification;
   - whether missing resource `38115` is gone;
   - whether a new missing resource appears;
   - whether recognizable PDF content renders;
   - Pass/Partial/Fail status;
   - next action.

## Pass Criteria

Experiment 28 passes only if:

- Chromium builds;
- `components_strings_en-US.pak` is found and loaded;
- resource `38115` resolves to a non-empty localized string;
- fake-GUI stream-info and plugin creation remain healthy;
- HTML DevTools sanity capture passes;
- real-GUI PDF logs no longer show `Unable to find resource: 38115`;
- the PDF DevTools screenshot shows recognizable Bitcoin PDF content;
- logs do not contradict the run.

## Partial Criteria

Experiment 28 is partial if:

- the components strings pak loads and resource `38115` is resolved;
- the PDF renderer gets past `FormatPageSize()`;
- but another missing resource, renderer crash, gray plugin rectangle, or
  non-rendered PDF state remains.

In that case, the next experiment should target the first new downstream layer
shown by the logs.

## Failure Criteria

Experiment 28 fails if:

- Chromium does not build;
- the patch bypasses `FormatPageSize()` or hard-codes PDF metadata strings
  instead of loading the proper resource pak;
- the fake-GUI or real-GUI stream-info/plugin chain regresses;
- HTML DevTools sanity capture fails;
- the run uses an installed/stable Roamium instead of the repo-built binary.

## Result

**Result:** Pass

Chromium branch: `148.0.7778.97-issue-792-exp28`

Chromium commit: `a40294c71f32d469f07cee437d763cc371c7e491`
(`Load PDF localized strings`)

Patch archive: regenerated at `chromium/patches/issue-792/`.

Build:

```bash
cd chromium/src
export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium
```

Result:

```text
Build Succeeded: 0 steps
```

The fake-GUI preflight passed:

```text
logs/issue-792-exp28-fakegui-20260529-170907
```

Important lines:

```text
[issue-792-exp28] components-strings-pak path=/Users/ryan/dev/termsurf/chromium/src/out/Default/gen/components/strings/components_strings_en-US.pak found=1 loaded=1 portrait_bytes=8
[issue-792-exp18] real-mime-handler-get-stream-info has_stream=1 ... original_url=http://127.0.0.1:9787/bitcoin.pdf
[issue-792-exp27] internal-plugin-externalized handled=1
[issue-792-exp16] pvs-start ... url=http://127.0.0.1:9787/bitcoin.pdf is_pdf=1 ...
[issue-792-exp26] internal-plugin-create-result created=1
```

The run did not contain:

```text
Unable to find resource
FATAL
Received signal
```

The real-GUI HTML DevTools sanity check passed:

```text
logs/issue-792-exp28-html-devtools-20260529-170938/devtools-smoke.png
```

Classification: rendered `example.com`.

The real-GUI PDF DevTools capture passed:

```text
logs/issue-792-exp28-pdf-devtools-20260529-171104/devtools-smoke.png
```

Classification: rendered PDF. The screenshot shows recognizable Bitcoin paper
content: title, author block, abstract heading, and body text.

The PDF run used the repo-built debug path:

```text
web_command=/Users/ryan/dev/termsurf/webtui/target/debug/web --browser /Users/ryan/dev/termsurf/chromium/src/out/Default/roamium http://localhost:9616/bitcoin.pdf
```

The PDF run's Chromium log confirmed the localized strings pak loaded:

```text
[issue-792-exp28] components-strings-pak path=/Users/ryan/dev/termsurf/chromium/src/out/Default/gen/components/strings/components_strings_en-US.pak found=1 loaded=1 portrait_bytes=8
```

The PDF run did not contain:

```text
Unable to find resource
FATAL
Received signal
```

## Conclusion

Loading `components_strings_en-US.pak` fixed the PDF metadata crash introduced
by the first successful plugin-renderer path. Resource `38115`
(`IDS_PDF_PROPERTIES_PAGE_SIZE_PORTRAIT`) now resolves, `FormatPageSize()` no
longer aborts, and the PDF plugin renders visible PDF content in the DevTools
capture.

Experiment 28 is the first experiment in this issue to prove recognizable inline
PDF rendering in Roamium. Claude reviewed the completed experiment and agreed
that Pass is correct. The issue can now close after the README conclusion
records the current remaining limitations separately from the core rendering
success.
