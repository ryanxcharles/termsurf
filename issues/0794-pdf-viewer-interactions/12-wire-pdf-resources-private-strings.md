# Experiment 12: Wire PDF ResourcesPrivate Strings for Zoom

## Description

Experiment 11 proved that zoom-in and zoom-out are not failing at the toolbar,
event, or PDF plugin hit-test layers. Both controls reach Chromium's PDF viewer
`Viewport.zoomIn()` / `Viewport.zoomOut()`, and both throw from
`Viewport.mightZoom_`.

The important state from Experiment 11 is:

- `viewer.viewport_.presetZoomFactors_` is an empty array;
- `viewer.viewport_.presetZoomFactors` reports length `0`;
- `Viewport.zoomIn()` and `Viewport.zoomOut()` both assert that preset zoom
  factors are non-empty before choosing the next zoom level.

Chromium normally initializes this state in
`chrome/browser/resources/pdf/pdf_viewer_base.ts`:

```text
chrome.resourcesPrivate.getStrings(PDF)
  -> PdfViewerBaseElement.handleStrings(strings)
  -> loadTimeData.data = strings
  -> JSON.parse(loadTimeData.getString("presetZoomFactors"))
  -> viewport_.setZoomFactorRange(presetZoomFactors)
```

The browser-side provider is
`chrome/browser/extensions/api/resources_private/resources_private_api.cc`. For
the PDF component it calls:

```text
pdf_extension_util::GetStrings(PdfViewerContext::kAll)
pdf_extension_util::GetAdditionalData(browser_context())
webui::SetLoadTimeDataDefaults(...)
```

`pdf_extension_util::GetStrings()` adds `presetZoomFactors` via
`zoom::GetPresetZoomFactorsAsJSON()`.

Experiment 12 should wire TermSurf's PDF extension to Chromium's canonical
`resourcesPrivate.getStrings(PDF)` implementation, verify that
`presetZoomFactors` reaches the live PDF viewer, and then verify that toolbar
zoom works. It should not patch `viewport.ts` or hard-code zoom factors in the
renderer as the primary fix.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a new Chromium branch.

   Fork the current good Issue 794 Chromium branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp8
   git checkout -b 148.0.7778.97-issue-794-exp12
   ```

   Add the branch to `chromium/README.md` with a description such as "Wire PDF
   resourcesPrivate strings for zoom."

2. Verify the current API/data failure before changing product code.

   Extend `scripts/probe-pdf-toolbar-events.mjs` or add a small companion probe
   mode that records, from the PDF extension child target:
   - whether `chrome.resourcesPrivate` exists;
   - whether `chrome.resourcesPrivate.getStrings` exists;
   - the callback result for
     `chrome.resourcesPrivate.getStrings(chrome.resourcesPrivate.Component.PDF, ...)`;
   - whether the returned object contains `presetZoomFactors`;
   - the exact `presetZoomFactors` string and parsed array length;
   - `chrome.runtime.lastError`, if any;
   - whether the natural PDF viewer initialization path calls
     `handleStrings(strings)`;
   - `viewer.viewport_.presetZoomFactors_` before and after the natural
     `handleStrings(strings)` callback.

   This probe must run before the Chromium fix and be recorded in the result. If
   the API already returns non-empty `presetZoomFactors` but the viewport array
   remains empty, stop and redesign around `handleStrings()` ordering instead of
   continuing with a browser-side API fix.

   Do not manually call `handleStrings(strings)` and treat that as proof of the
   real initialization path. A manual diagnostic call is allowed only as a
   separate probe, clearly labeled as manual, to show what would happen if the
   browser-side data were delivered.

3. Wire Chromium's `resourcesPrivate` implementation into TermSurf.

   Preferred implementation:
   - add `//chrome/browser/extensions/api/resources_private` to
     `content/libtermsurf_chromium/BUILD.gn`;
   - make the PDF component extension's `resourcesPrivate` permission usable by
     TermSurf's extension feature/API plumbing;
   - verify and, if needed, wire browser-side registration for
     `resourcesPrivate.getStrings`;
   - preserve the existing TermSurf-owned extension system and PDF component
     resource loader;
   - do not link broad Chrome browser feature stacks beyond the narrow
     `resources_private` API target unless the build proves it is required.

   Linking Chromium's `resources_private` source set is not sufficient by itself
   unless the extension function is registered. Chrome registers this through
   `ChromeExtensionsBrowserAPIProvider::RegisterExtensionFunctions()`. TermSurf
   should not blindly add Chrome's full browser API provider. Instead:
   - first verify whether `resourcesPrivate.getStrings` appears in the live
     extension function registry;
   - if it is missing, add the narrowest TermSurf-side registration that maps
     `resourcesPrivate.getStrings` to
     `extensions::ResourcesPrivateGetStringsFunction`;
   - only widen to Chrome's provider if a narrow registration proves impossible,
     and record the dependency cost before doing so.

   If linking or registering Chromium's implementation is insufficient because
   TermSurf's extension API provider does not expose the generated
   `resourcesPrivate` schema, wire the narrow missing schema/provider piece.
   Keep this limited to the `resourcesPrivate.getStrings` path needed by the PDF
   viewer.

   Do not add a TermSurf-specific replacement API unless the canonical Chromium
   implementation cannot be used. If a fallback is required, it must call the
   same underlying Chromium helpers:
   - `pdf_extension_util::GetStrings(PdfViewerContext::kAll)`;
   - `pdf_extension_util::GetAdditionalData(browser_context())`;
   - `webui::SetLoadTimeDataDefaults(...)`.

   A hard-coded `presetZoomFactors` string in PDF viewer JavaScript is a
   failure.

4. Add focused diagnostics around the browser-side call.

   Add temporary or small permanent debug logs gated by the existing PDF debug
   convention if available. They should answer:
   - did the `resourcesPrivate.getStrings` extension function run?
   - was the requested component `PDF`?
   - did the returned dictionary include `presetZoomFactors`?
   - what was the parsed preset array length in the live viewer?

   Logs should go to normal Roamium/Chromium stderr like the Issue 792 PDF logs.
   Do not add a new protocol message or a new log file unless necessary.

5. Build Chromium and Roamium.

   Use the Chromium skill's build rule:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Then rebuild Roamium if the Rust wrapper or binary linkage requires it:

   ```bash
   cd /Users/ryan/dev/termsurf
   ./scripts/build.sh roamium
   ```

   Never use `ninja` directly.

6. Verify with the Experiment 11 toolbar event harness.

   Re-run:

   ```bash
   LOG_DIR="logs/issue-794-exp12-toolbar-zoom-$(date +%Y%m%d-%H%M%S)" \
     scripts/test-issue-794-pdf-toolbar.py \
     --probe events \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf
   ```

   Required checks:
   - `viewer.viewport_.presetZoomFactors_` is non-empty before clicking zoom;
   - `zoomIn` is `no-failure-observed`;
   - `zoomOut` is `no-failure-observed`;
   - zoom text or `viewer.viewportZoom_` changes after zoom;
   - fit mode remains `no-failure-observed`;
   - rotate remains `no-failure-observed`;
   - no save/download/print UI is opened.

7. Run regression checks from prior successful PDF interaction experiments.

   Re-run the existing automated checks for:
   - PDF wheel scroll;
   - PDF keyboard select/copy;
   - PDF drag selection;
   - PDF resize/reflow;
   - normal HTML click smoke.

   If any prior passing interaction regresses, the experiment is Partial or Fail
   even if zoom works.

8. Archive Chromium patches only after a coherent branch result.

   If the experiment passes or produces a coherent partial branch, commit the
   Chromium branch and regenerate:

   ```bash
   cd chromium/src
   rm -rf ../../chromium/patches/issue-794-exp12/
   git format-patch 148.0.7778.97..HEAD \
     -o ../../chromium/patches/issue-794-exp12/
   ```

   Update `chromium/README.md` in the main repo.

9. Formatting and review.

   If Markdown changes, run Prettier. If Rust changes, run `cargo fmt` and
   accept its output. If Chromium C++ changes, run Chromium formatting on the
   modified files if practical.

## Verification

1. Required pre-fix evidence:

   | Check                                    | Result |
   | ---------------------------------------- | ------ |
   | `chrome.resourcesPrivate` exists         |        |
   | `resourcesPrivate.getStrings` exists     |        |
   | callback fires                           |        |
   | `chrome.runtime.lastError`               |        |
   | returned `presetZoomFactors`             |        |
   | parsed preset zoom factor count          |        |
   | viewport preset zoom factor count before |        |
   | viewport preset zoom factor count after  |        |

2. Required post-fix toolbar table:

   | Feature  | Presets non-empty | First failing hop | Zoom/state changed |
   | -------- | ----------------- | ----------------- | ------------------ |
   | Zoom in  |                   |                   |                    |
   | Zoom out |                   |                   |                    |
   | Fit mode |                   |                   |                    |
   | Rotate   |                   |                   |                    |

3. Required regression table:

   | Check                    | Result | Log directory |
   | ------------------------ | ------ | ------------- |
   | PDF wheel scroll         |        |               |
   | PDF keyboard select/copy |        |               |
   | PDF drag selection       |        |               |
   | PDF resize/reflow        |        |               |
   | HTML click smoke         |        |               |

4. Codex must review the completed output.

   Do not proceed to Experiment 13 until real issues from Codex's completion
   review are addressed.

## Pass Criteria

Experiment 12 passes if:

- the pre-fix probe proves the missing/broken `resourcesPrivate.getStrings(PDF)`
  data path is why `presetZoomFactors_` is empty;
- the fix wires the canonical Chromium PDF strings/additional-data provider or a
  TermSurf wrapper around the same helpers;
- the live PDF viewer receives a non-empty preset zoom factor array;
- toolbar zoom-in and zoom-out work without `Viewport.mightZoom_` assertions;
- fit, rotate, wheel scroll, keyboard copy, drag selection, resize/reflow, and
  normal HTML click smoke remain working.

## Partial Criteria

Experiment 12 is partial if:

- the pre-fix probe shows the failure is not `resourcesPrivate.getStrings`, but
  identifies a different exact initialization gap;
- the API/data path is fixed and presets become non-empty, but zoom still fails
  at a later hop;
- the Chromium build requires a larger extension API registration change than
  can be safely completed in this experiment.

## Failure Criteria

Experiment 12 fails if:

- it patches `viewport.ts` or PDF viewer JavaScript to hard-code zoom factors;
- it bypasses `resourcesPrivate.getStrings(PDF)` instead of fixing the missing
  browser-side data path;
- it makes toolbar zoom appear to work only through direct method calls rather
  than real toolbar clicks;
- it opens native save/download/print UI;
- it regresses any prior passing PDF interaction check without recording the
  regression and stopping;
- it uses installed/stable Roamium instead of repo-built debug Roamium;
- it omits Codex design or completion review.

## Result

**Result:** Pass

Experiment 12 fixed the missing PDF `resourcesPrivate.getStrings(PDF)` path and
preserved the prior passing PDF interaction checks.

Pre-fix evidence:

| Check                                    | Result                                                  |
| ---------------------------------------- | ------------------------------------------------------- |
| `chrome.resourcesPrivate` exists         | Yes                                                     |
| `resourcesPrivate.getStrings` exists     | Yes                                                     |
| callback fires                           | No; the direct probe timed out                          |
| `chrome.runtime.lastError`               | `null` before the call and after timeout                |
| returned `presetZoomFactors`             | None, because the callback never fired                  |
| parsed preset zoom factor count          | `0`                                                     |
| viewport preset zoom factor count before | `0`                                                     |
| viewport preset zoom factor count after  | `0`                                                     |
| Log directory                            | `logs/issue-794-exp12-prefix-resources-20260530-105106` |

Implementation findings:

- Linking `//chrome/browser/extensions/api/resources_private` directly pulled
  broad Chrome PDF/browser dependencies, duplicated TermSurf's existing
  `PdfViewerStreamManager` object code, and introduced unrelated unresolved
  Chrome symbols. That was not a viable narrow embedder fix.
- A TermSurf-owned `resourcesPrivate.getStrings` extension function was added
  instead. It uses Chromium's generated `resourcesPrivate` schema, returns PDF
  toolbar strings from Chromium's localized resources, uses
  `zoom::GetPresetZoomFactorsAsJSON()` for the preset zoom list, applies
  `webui::SetLoadTimeDataDefaults(...)`, and keeps currently unsupported PDF
  feature flags disabled.
- The browser-side extension API dispatcher was not reachable until TermSurf
  attached `ExtensionWebContentsObserver` to created `WebContents` and replaced
  the inherited `LocalFrameHost` associated binder with an explicit TermSurf
  binding. Without that, the renderer sent the request but the browser never
  dispatched it.
- `generated_resources_en-US.pak` is now loaded into TermSurf's PDF resource
  bundle so PDF toolbar strings can be resolved without crashing
  `ResourceBundle`.

Post-fix toolbar verification:

| Feature  | Presets non-empty | First failing hop     | State changed |
| -------- | ----------------- | --------------------- | ------------- |
| Zoom in  | Yes, 17 presets   | `no-failure-observed` | Yes           |
| Zoom out | Yes, 17 presets   | `no-failure-observed` | Yes           |
| Fit mode | Yes, 17 presets   | `no-failure-observed` | Yes           |
| Rotate   | Yes, 17 presets   | `no-failure-observed` | Yes           |

Final toolbar log: `logs/issue-794-exp12-final-toolbar-20260530-111754`.

Regression verification:

| Check                    | Result                                                                                                     | Log directory                                            |
| ------------------------ | ---------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| PDF wheel scroll         | Pass; `first_failing_hop=no-failure-observed`, screenshot changed, six wheel events sent via input router  | `logs/issue-794-exp12-wheel-regression-20260530-111900`  |
| PDF keyboard select/copy | Pass; `first_failing_hop=no-failure-observed`, clipboard copied `21230` bytes from the PDF                 | `logs/issue-794-exp12-key-regression-20260530-111900`    |
| PDF drag selection       | Pass; `drag_sweep_selected=true`, clipboard copied `62` bytes from PDF text                                | `logs/issue-794-exp12-drag-regression-20260530-111900`   |
| PDF resize/reflow        | Pass; `first_failing_hop=no-failure-observed`, viewport, viewer, container, plugin, and screenshot changed | `logs/issue-794-exp12-resize-regression-20260530-111900` |
| HTML click smoke         | Pass; `first_failing_hop=no-failure-observed`, click count changed                                         | `logs/issue-794-exp12-html-click-20260530-111900`        |

Build and archive:

- Chromium build: `autoninja -C out/Default libtermsurf_chromium` passed.
- Roamium debug build: `./scripts/build.sh roamium` passed.
- Chromium branch commit: `dba20bee6c5d6` (`Wire PDF resourcesPrivate strings`).
- Patch archive: `chromium/patches/issue-794-exp12/`.

Codex completion review:

- Review log: `logs/codex-review/20260530-112351-173970-last-message.md`.
- Result: no blocking findings.
- Caveat: the TermSurf-owned `resourcesPrivate.getStrings` implementation is a
  narrow provider for the PDF toolbar strings and data needed by this viewer
  path, not a full drop-in replacement for Chrome's entire PDF string surface.

## Conclusion

Toolbar zoom was blocked because the PDF viewer received a generated
`chrome.resourcesPrivate.getStrings` JavaScript function, but TermSurf had not
provided the browser-side extension API dispatch path or PDF string provider
behind it. Once `resourcesPrivate.getStrings(PDF)` returned Chromium PDF toolbar
strings plus the preset zoom factors, the live viewer initialized
`presetZoomFactors_` with 17 values and real toolbar clicks for zoom-in,
zoom-out, fit, and rotate all worked.

Experiment 12 closes the toolbar zoom gap without hard-coding values in PDF
viewer JavaScript and without regressing the prior interaction fixes. The next
experiment should target the next remaining incomplete PDF viewer surface rather
than revisiting toolbar zoom.
