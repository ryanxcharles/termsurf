# Experiment 14: Complete PDF Viewer Strings and Load-Time Data

## Description

Experiment 13 proved that save/download works in a contained path and that HTTP,
local `file://`, HTTP extensionless, and local extensionless PDFs all enter the
PDF viewer and render. It also surfaced the next concrete product failure:

```text
Uncaught (in promise) Error: Assertion failed: Could not find value for thumbnailPageAriaLabel
```

The toolbar inventory still shows labels such as `$i18n{tooltipDownload}` and
`$i18n{tooltipZoomIn}`. That means Experiment 12's
`resourcesPrivate.getStrings(PDF)` implementation was enough to initialize zoom
presets, but it is not a complete-enough PDF viewer string/data provider.

Chromium's canonical provider is `chrome/browser/pdf/pdf_extension_util.cc`:

- `GetCommonStrings()` includes common PDF viewer strings and
  `presetZoomFactors`;
- `GetPdfViewerStrings()` includes stand-alone viewer strings such as
  `thumbnailPageAriaLabel`, `sidebarLabel`, `downloadEdited`,
  `downloadOriginal`, properties dialog strings, thumbnail/sidebar strings, and
  toolbar strings;
- `GetAdditionalData(browser_context)` includes PDF viewer booleans such as
  `pdfGetSaveDataInBlocks`, `pdfUseShowSaveFilePicker`, and `printingEnabled`.

Experiment 12 already found that linking the full Chrome
`pdf_extension_util`/`resources_private` path pulled too much Chrome browser
stack and duplicated TermSurf's smaller PDF embedder objects. Experiment 14
should therefore keep the TermSurf-owned narrow API provider, but make its PDF
string/load-time data complete enough for the bundled PDF viewer UI.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a new Chromium branch.

   Fork the current good Issue 794 Chromium branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp12
   git checkout -b 148.0.7778.97-issue-794-exp14
   ```

   Add the branch to `chromium/README.md` with a description such as "Complete
   PDF viewer strings and load-time data."

2. Audit every PDF viewer string/load-time key used by Chromium's bundled PDF
   UI.

   Before changing product code, run a source audit over:

   ```bash
   rg -n "loadTimeData\\.(getString|getStringF|getBoolean|getInteger|getValue)|\\$i18n" \
     chrome/browser/resources/pdf \
     -g'*.ts' -g'*.html.ts' -g'*.html' -g'*.js'
   ```

   Compare the discovered keys with:
   - `pdf_extension_util::GetCommonStrings()`;
   - `pdf_extension_util::GetPdfViewerStrings()`;
   - `pdf_extension_util::GetAdditionalData()`;
   - the current TermSurf list in
     `content/libtermsurf_chromium/extensions/ts_resources_private_api.cc`.

   Record in the result:
   - keys already covered by TermSurf;
   - keys newly added;
   - keys intentionally omitted because their feature is compile-time disabled
     or explicitly unsupported by TermSurf;
   - any key still present as `$i18n{...}` after the fix.

3. Expand TermSurf's PDF string provider.

   Update `content/libtermsurf_chromium/extensions/ts_resources_private_api.cc`
   so `GetPdfStringsForTermSurf()` covers Chromium's stand-alone PDF viewer
   string surface without linking the broad Chrome PDF browser stack.

   Preferred implementation:
   - mirror the non-feature-gated entries from Chromium's `GetCommonStrings()`
     and `GetPdfViewerStrings()`;
   - include the load-bearing missing strings from Experiment 13, especially
     `thumbnailPageAriaLabel`, `sidebarLabel`, `tooltipThumbnails`,
     `tooltipDocumentOutline`, `tooltipAttachments`, `downloadEdited`, and
     `downloadOriginal`;
   - preserve `presetZoomFactors` via `zoom::GetPresetZoomFactorsAsJSON()`;
   - call `webui::SetLoadTimeDataDefaults(...)` after all strings/data are in
     the dictionary.

   Do not link `//chrome/browser/pdf` or Chrome's full `resources_private_api`
   path unless the copied table proves impossible. If a key requires a
   buildflag-gated resource ID that is not compiled in this build, gate it with
   the same buildflag as Chromium.

4. Preserve conservative PDF additional data.

   The current TermSurf provider hard-codes conservative feature booleans.
   Experiment 14 must preserve that posture. It may reorganize the values into a
   TermSurf-owned helper, but it must not enable unsupported browser-side
   behaviors:
   - `pdfGetSaveDataInBlocks`: keep `false` unless TermSurf's corresponding
     browser-side save-data-in-blocks path is explicitly verified in this
     experiment, which is not expected;
   - `pdfUseShowSaveFilePicker`: keep `false`; do not enable native file picker
     behavior in a strings experiment;
   - `printingEnabled`: keep `false` until a dedicated print experiment proves a
     contained/non-native print path;
   - `pdfGlicSummarizeEnabled`, `pdfSaveToDrive`, `pdfSearchifySaveEnabled`,
     `pdfInk2Enabled`, and `pdfTextAnnotationsEnabled`: keep false unless their
     browser-side support exists in TermSurf.

   This experiment's goal is to eliminate missing strings and placeholder UI,
   not to enable unsupported features by toggling booleans. If useful, record
   Chromium's feature-flag values separately in diagnostics, but do not expose
   them to the PDF viewer as enabled TermSurf behavior.

5. Improve the Experiment 13 probe's string checks.

   Extend `scripts/probe-pdf-save-print-title-local.mjs` so the summary records:
   - the count and examples of `$i18n{...}` placeholders in visible toolbar and
     sidebar controls;
   - console errors containing `Could not find value for`;
   - whether `thumbnailPageAriaLabel` is available through
     `loadTimeData.getStringF(...)` in the live PDF extension frame;
   - the live values of the additional-data booleans if they are observable.

   The probe should not click print unless the Experiment 13 containment rule is
   still satisfied.

6. Build Chromium and Roamium.

   Use the Chromium skill's build rule:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Then rebuild Roamium:

   ```bash
   cd /Users/ryan/dev/termsurf
   ./scripts/build.sh roamium
   ```

   Never use `ninja` directly.

7. Verify with the Experiment 13 probe.

   Re-run:

   ```bash
   LOG_DIR="logs/issue-794-exp14-strings-$(date +%Y%m%d-%H%M%S)" \
     scripts/test-issue-794-pdf-toolbar.py \
     --probe save-print-title-local \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf
   ```

   Required checks:
   - no `Could not find value for ...` console errors for supported PDF viewer
     UI strings, including `thumbnailPageAriaLabel`;
   - no visible toolbar/sidebar `$i18n{...}` placeholders for the supported PDF
     viewer controls;
   - `presetZoomFactors` remains non-empty;
   - save/download still creates a contained `bitcoin.pdf`;
   - print remains safely classified without opening a native dialog;
   - title classification is recorded again;
   - HTTP, `file://`, HTTP extensionless, and `file://` extensionless PDFs still
     render;
   - page navigation and zoom still work for the multi-page Bitcoin variants.

8. Run prior PDF interaction regressions.

   Re-run the existing automated checks for:
   - PDF wheel scroll;
   - PDF keyboard select/copy;
   - PDF drag selection;
   - PDF resize/reflow;
   - toolbar zoom/final controls from Experiment 12;
   - normal HTML click smoke.

9. Archive Chromium patches only after a coherent branch result.

   If the experiment passes or produces a coherent partial branch, commit the
   Chromium branch and regenerate:

   ```bash
   cd chromium/src
   rm -rf ../../chromium/patches/issue-794-exp14/
   git format-patch 148.0.7778.97..HEAD \
     -o ../../chromium/patches/issue-794-exp14/
   ```

   Update `chromium/README.md` in the main repo.

10. Formatting and review.

    If Markdown changes, run Prettier. If Rust changes, run `cargo fmt` and
    accept its output. If Chromium C++ changes, run Chromium formatting on the
    modified files if practical.

    Codex must review the completed output before Experiment 15 is designed.

## Verification

1. String coverage table:

   | Key group                      | Before | After | Evidence |
   | ------------------------------ | ------ | ----- | -------- |
   | Common PDF strings             |        |       |          |
   | Stand-alone PDF viewer strings |        |       |          |
   | `thumbnailPageAriaLabel`       |        |       |          |
   | Download/save strings          |        |       |          |
   | Sidebar/thumbnail strings      |        |       |          |
   | Properties dialog strings      |        |       |          |
   | Feature-gated strings omitted  |        |       |          |

2. Runtime UI table:

   | Check                                                  | Result | Evidence |
   | ------------------------------------------------------ | ------ | -------- |
   | `$i18n{...}` placeholders visible                      |        |          |
   | missing `Could not find value for ...` console errors  |        |          |
   | `loadTimeData.getStringF("thumbnailPageAriaLabel", 1)` |        |          |
   | `presetZoomFactors` count                              |        |          |
   | additional-data booleans observable                    |        |          |

3. Experiment 13 parity table:

   | Surface                        | Result | Evidence |
   | ------------------------------ | ------ | -------- |
   | Save/download contained        |        |          |
   | Print safely classified        |        |          |
   | Title classification           |        |          |
   | HTTP PDF render                |        |          |
   | `file://` PDF render           |        |          |
   | HTTP extensionless render      |        |          |
   | `file://` extensionless render |        |          |
   | Page navigation                |        |          |
   | Toolbar zoom                   |        |          |

4. Regression table:

   | Check                    | Result | Log directory |
   | ------------------------ | ------ | ------------- |
   | PDF wheel scroll         |        |               |
   | PDF keyboard select/copy |        |               |
   | PDF drag selection       |        |               |
   | PDF resize/reflow        |        |               |
   | Toolbar zoom/final       |        |               |
   | HTML click smoke         |        |               |

5. Codex completion review.

   Do not proceed to Experiment 15 until real issues from Codex's completion
   review are addressed.

## Pass Criteria

Experiment 14 passes if:

- no `Could not find value for ...` console assertions remain for supported PDF
  viewer UI strings;
- supported visible PDF viewer controls no longer expose `$i18n{...}`
  placeholders;
- `resourcesPrivate.getStrings(PDF)` still returns non-empty
  `presetZoomFactors`;
- save/download, local-file rendering, extensionless rendering, page navigation,
  toolbar zoom, and prior PDF interaction regressions remain working;
- print remains safely classified and no native print dialog opens;
- title classification is recorded, but fixing title propagation is not part of
  this string/template experiment;
- the implementation keeps TermSurf's narrow embedder-owned API provider rather
  than linking broad Chrome browser PDF stacks.

## Partial Criteria

Experiment 14 is partial if:

- the known missing strings are fixed, but another string/load-time key is
  discovered and precisely recorded;
- the string provider is complete, but enabling its template replacements
  regresses rendering, save/download, page navigation, toolbar zoom, or prior
  PDF interaction checks;
- the Chromium build proves that a copied table is not viable and identifies the
  smallest safe dependency needed next.

Title propagation and a contained print path are known remaining Issue 794
surfaces, but they are not part of this experiment's string/template completion
goal.

## Failure Criteria

Experiment 14 fails if:

- it hard-codes renderer-side JavaScript strings instead of fixing
  `resourcesPrivate.getStrings(PDF)` and component-extension template
  replacements;
- it links Chrome's broad PDF/browser stack without recording and justifying the
  dependency cost;
- it enables print, native file pickers, Save to Drive, Glic, Ink2, or Searchify
  UI without the corresponding TermSurf browser-side support;
- it opens a native save or print dialog during automation;
- it regresses save/download, zoom, page navigation, PDF input, resize, or
  local/extensionless rendering;
- it uses installed/stable Roamium instead of repo-built debug Roamium;
- it omits Codex design or completion review.

## Result

**Result:** Pass

Experiment 14 completed the PDF viewer string/template surface without linking
Chrome's full PDF browser stack. TermSurf now provides the bundled viewer with:

- the non-feature-gated common PDF strings from Chromium's `GetCommonStrings()`;
- the stand-alone viewer strings from `GetPdfViewerStrings()`;
- build-flag-gated Ink2 and Save-to-Drive string tables when those resource IDs
  exist in this Chromium build;
- standard WebUI load-time defaults such as `textdirection`, `language`,
  `fontfamily`, and `fontsize`;
- template replacements for component-extension HTML/JS resources, not only
  runtime `chrome.resourcesPrivate.getStrings(PDF)`;
- conservative feature booleans for unsupported TermSurf surfaces.

One important implementation detail is that `pdfOopifEnabled` is intentionally
kept empty. The first template-replacement attempt exposed Chromium's enabled
OOPIF attribute, which moved the viewer onto `pdfViewerPrivate.getStreamInfo()`.
TermSurf's current PDF bridge still uses the legacy mime-handler viewer path,
and that OOPIF path is not implemented yet. Keeping the attribute empty restores
the proven working path while still allowing the rest of the strings to be
resolved.

String coverage:

| Key group                      | Before                          | After | Evidence                                                                                                                                 |
| ------------------------------ | ------------------------------- | ----- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Common PDF strings             | Partially covered by Exp 12     | Pass  | `ts_resources_private_api.cc` and `ts_component_extension_resource_manager.cc` mirror Chromium's common entries                          |
| Stand-alone PDF viewer strings | Incomplete                      | Pass  | Same files now include sidebar, thumbnails, properties, password, download, and rotation labels                                          |
| `thumbnailPageAriaLabel`       | Missing runtime error in Exp 13 | Pass  | `logs/issue-794-exp14-strings-20260530-121526/save-print-title-local/save-print-title-local-summary.json` reports `Thumbnail for page 1` |
| Download/save strings          | Incomplete                      | Pass  | Toolbar inventory shows `Download`, `With your changes`, and `Without your changes` instead of placeholders                              |
| Sidebar/thumbnail strings      | Incomplete                      | Pass  | Placeholder scan count is `0`; sidebar labels are resolved                                                                               |
| Properties dialog strings      | Incomplete                      | Pass  | Added from Chromium's PDF viewer string table                                                                                            |
| Feature-gated strings omitted  | N/A                             | Pass  | Ink2 and Save-to-Drive strings are gated with Chromium's build flags; unsupported feature booleans remain false                          |

Runtime UI:

| Check                                                  | Result  | Evidence                                                                                                                           |
| ------------------------------------------------------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `$i18n{...}` placeholders visible                      | Pass    | Placeholder count `0` in `logs/issue-794-exp14-strings-20260530-121526/save-print-title-local/save-print-title-local-summary.json` |
| missing `Could not find value for ...` console errors  | Pass    | `consoleMissingStringErrors=[]` in the same summary                                                                                |
| `loadTimeData.getStringF("thumbnailPageAriaLabel", 1)` | Pass    | Same summary returns `Thumbnail for page 1`                                                                                        |
| `presetZoomFactors` count                              | Pass    | Exp 12 toolbar probe still passes; Exp 14 toolbar regression remains green                                                         |
| additional-data booleans observable                    | Partial | Runtime probe records the API surface; unsupported print/native picker booleans remain deliberately disabled                       |

Experiment 13 parity:

| Surface                        | Result  | Evidence                                                                                                                     |
| ------------------------------ | ------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Save/download contained        | Pass    | `download-file-created`; file at `logs/issue-794-exp14-strings-20260530-121526/save-print-title-local/downloads/bitcoin.pdf` |
| Print safely classified        | Pass    | `print-not-contained`; probe did not click print or open a native dialog                                                     |
| Title classification           | Partial | Still `title-extension-only`; title propagation remains future work                                                          |
| HTTP PDF render                | Pass    | `http-pdf render=True`                                                                                                       |
| `file://` PDF render           | Pass    | `file-pdf render=True`                                                                                                       |
| HTTP extensionless render      | Pass    | `http-extensionless render=True`                                                                                             |
| `file://` extensionless render | Pass    | `file-extensionless render=True`                                                                                             |
| Page navigation                | Pass    | Multi-page Bitcoin variants changed page state                                                                               |
| Toolbar zoom                   | Pass    | Multi-page and untitled variants changed zoom state                                                                          |

Regression checks:

| Check                    | Result                                                                        | Log directory                                                |
| ------------------------ | ----------------------------------------------------------------------------- | ------------------------------------------------------------ |
| PDF wheel scroll         | Pass; `first_failing_hop=no-failure-observed`, state and screenshot changed   | `logs/issue-794-exp14-regression-scroll-20260530-122058`     |
| PDF keyboard select/copy | Pass; `first_failing_hop=no-failure-observed`, clipboard copied `21230` bytes | `logs/issue-794-exp14-regression-key-20260530-122058`        |
| PDF drag selection       | Pass; `first_failing_hop=no-failure-observed`, state and screenshot changed   | `logs/issue-794-exp14-regression-drag-text-20260530-122150`  |
| PDF resize/reflow        | Pass; `first_failing_hop=no-failure-observed`, state and screenshot changed   | `logs/issue-794-exp14-regression-resize-20260530-122058`     |
| Toolbar zoom/final       | Pass; toolbar probe status `pass`                                             | `logs/issue-794-exp14-regression-toolbar-20260530-122123`    |
| HTML click smoke         | Pass; `first_failing_hop=no-failure-observed`, state and screenshot changed   | `logs/issue-794-exp14-regression-html-click-20260530-122234` |

Build and format:

- Chromium build passed: `autoninja -C out/Default libtermsurf_chromium`.
- Roamium debug build passed with the project cargo-path workaround:
  `./scripts/build.sh roamium`.
- Chromium C++ files were formatted with
  `chromium/src/buildtools/mac_arm64-format/clang-format`.
- Chromium branch commit: `c117909804bb8` (`Complete PDF viewer strings`).
- Patch archive: `chromium/patches/issue-794-exp14/`.
- Codex completion review:
  - Initial review: `logs/codex-review/20260530-122502-564890-last-message.md`.
  - Follow-up review after fixing the criteria/docs/archive findings:
    `logs/codex-review/20260530-122904-436340-last-message.md`.
  - Result: no blocking findings remain; Codex accepted Experiment 14 as Pass
    for the string/template objective.

## Conclusion

The missing-string problem had two layers. `resourcesPrivate.getStrings(PDF)`
was necessary for runtime viewer initialization, but the bundled component
extension also needs template replacements while serving `index.html` and the
template-bearing JavaScript resources. Adding only the runtime API left visible
`$i18n{...}` placeholders and missing-string failures; adding the template map
fixed the rendered UI labels.

Experiment 14 also identified a real trap: blindly mirroring Chrome's
`pdfOopifEnabled` value regresses rendering because TermSurf has not implemented
the OOPIF `pdfViewerPrivate.getStreamInfo()` path. For now, TermSurf must expose
the rest of the PDF viewer strings while keeping that attribute disabled.

The next remaining Issue 794 work is not strings. The known unfinished surfaces
are title propagation (`title-extension-only`) and a contained print path.
