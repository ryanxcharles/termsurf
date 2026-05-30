# Experiment 8: Trace Protocol Resize and PDF Reflow

## Description

Experiments 4-7 proved the core input path for PDF interaction:

- wheel scrolling reaches the PDF viewer through Chromium's input router;
- mouse clicks and drags reach the PDF plugin;
- keyboard events route to the focused PDF widget;
- PDF drag selection works when the drag starts on actual PDF text.

The next unproven user-visible requirement is resize/reflow. Manual testing
earlier in Issue 794 reported that the first page initially fit the webview
height, but later pane resizes did not resize/reflow the rendered PDF pages.

Experiment 8 should not guess whether this is a Wezboard layout issue, Roamium
protocol issue, Chromium `WebContents` resize issue, PDF viewer JavaScript
issue, or PDFium plugin geometry issue. It should first build an automated
resize ladder using the TermSurf protocol path:

1. fake GUI sends `CreateTab` with an initial webview size;
2. Roamium loads the Bitcoin PDF fixture;
3. DevTools captures viewer/plugin/PDF element geometry;
4. fake GUI sends a second `Resize` protobuf for the same tab;
5. DevTools captures geometry again;
6. gated logs show whether the resize reached Roamium, Chromium
   `TsBrowserMainParts::ResizeTab()`, `PdfViewWebPlugin::UpdateGeometry()`,
   `PdfViewWebPlugin::OnGeometryChanged()`, and
   `PDFiumEngine::PluginSizeUpdated()`.

If direct protocol resize works, the next experiment should move to the real
Wezboard path and prove whether Wezboard sends the correct resize when panes
change. If direct protocol resize fails, the next fix belongs in whichever layer
the ladder identifies.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a fresh Chromium branch for Issue 794 Experiment 8 if Chromium tracing
   is needed.

   Fork from the passing Experiment 7 branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp7
   git checkout -b 148.0.7778.97-issue-794-exp8
   ```

   Add the branch to `chromium/README.md`.

   If implementation discovers that existing logs and DevTools state are enough
   and no Chromium code change is needed, do not create a Chromium branch just
   for bookkeeping.

2. Add or extend a protocol resize harness.

   Add `scripts/test-issue-794-protocol-resize.py`, or extend the existing
   protocol harness only if doing so stays clear. The harness should:
   - serve the local Bitcoin PDF fixture;
   - launch `chromium/src/out/Default/roamium` against a fake GUI Unix socket;
   - send `CreateTab` with an initial size, for example `900x700`;
   - send the initial `Resize` for that tab, as the current harnesses do;
   - wait for the PDF viewer to render;
   - capture a `before` DevTools probe and screenshot;
   - send a second `Resize` for the same tab, for example `1300x900`;
   - wait for the viewer to settle;
   - capture an `after` DevTools probe and screenshot;
   - optionally send a third resize back down to a smaller size to prove both
     grow and shrink behavior.

   Use TermSurf protocol `Resize` messages as the stimulus. DevTools may only
   observe state and screenshots.

3. Record geometry from both the top-level PDF extension page and child frames.

   The existing `scripts/capture-pdf-interactions.mjs` already records:
   - `viewport.innerWidth`;
   - `viewport.innerHeight`;
   - scroll metrics;
   - `PDF-VIEWER#viewer`;
   - `DIV#container`;
   - `DIV#sizer`;
   - `EMBED#plugin`;
   - toolbar and control element bounds;
   - child frame states when available.

   Reuse that data. The resize summary must compare at least:
   - top-level viewport width/height;
   - `PDF-VIEWER#viewer` width/height;
   - `DIV#container` width/height;
   - `EMBED#plugin` width/height;
   - `DIV#sizer` width/height;
   - any observable page, page container, canvas, or page-like element
     width/height;
   - zoom, fit mode, or viewport state if exposed by the viewer page;
   - screenshot hash before/after.

   If `capture-pdf-interactions.mjs` cannot identify page dimensions beyond the
   plugin bounds, record that explicitly rather than inventing a proxy. In that
   case the experiment may prove "the plugin resized," but it must not claim
   "PDF pages reflowed" unless PDFium/plugin traces establish that rendered page
   geometry changed.

4. Add gated resize traces if existing evidence is insufficient.

   Keep all new logs gated by `TERMSURF_PDF_INPUT_TRACE`.

   Candidate trace points:
   - `roamium/src/dispatch.rs` in `Msg::Resize`, recording tab id, pane id,
     width, height, screen rect, scale, and whether `ts_set_view_size` was
     called.
   - `content/libtermsurf_chromium/ts_browser_main_parts.cc` in
     `TsBrowserMainParts::ResizeTab()`, recording requested logical size, screen
     rect, scale, root WebContents URL, and whether `ResizeWebContentForTests()`
     was called.
   - `pdf/pdf_view_web_plugin.cc` in `PdfViewWebPlugin::UpdateGeometry()` and
     `PdfViewWebPlugin::OnGeometryChanged()`, recording plugin rect, available
     area, zoom, device scale, viewport scale, and whether
     `engine_->PluginSizeUpdated()` is called.
   - `pdf/pdfium/pdfium_engine.cc` in `PDFiumEngine::PluginSizeUpdated()`,
     recording old and new plugin size.

   Do not modify PDF loading, PDF stream plumbing, input routing, or the PDF
   extension resource path in this experiment.

   These traces may be skipped only if the grow/shrink geometry checks fully
   pass. If any resize/reflow check fails or is ambiguous, add the gated
   Roamium, Chromium, PDF plugin, and PDFium resize traces and rerun before
   recording a failure-layer conclusion. A non-pass result without enough trace
   evidence to identify the failing layer is Partial, not Pass or Fail.

5. Add analyzer fields.

   The resize summary should include:
   - `resize_messages_sent`;
   - initial size and final size;
   - Roamium resize receive / FFI evidence;
   - Chromium `ResizeTab()` evidence if traced;
   - PDF plugin `UpdateGeometry()` / `OnGeometryChanged()` evidence if traced;
   - PDFium `PluginSizeUpdated()` evidence if traced;
   - before/after viewport size;
   - before/after viewer/container/plugin bounds;
   - before/after sizer and page/page-like bounds when observable;
   - before/after fit/zoom/viewport state when observable;
   - whether each dimension changed in the expected direction;
   - screenshot hash before/after;
   - first failing layer.

   Suggested first failing layers:
   - `protocol-resize-not-sent`
   - `roamium-resize-receive-missing`
   - `roamium-resize-ffi-missing`
   - `chromium-resize-missing`
   - `webcontents-viewport-not-resized`
   - `pdf-viewer-bounds-not-resized`
   - `pdf-plugin-bounds-not-resized`
   - `pdfium-plugin-size-not-updated`
   - `page-reflow-not-observable`
   - `no-failure-observed`

6. Build and archive correctly.

   If Chromium files change, build Chromium with:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Never run `ninja` directly.

   If Rust files change, run `cargo fmt` and rebuild Roamium:

   ```bash
   cd "$HOME/dev/termsurf"
   PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" cargo-fmt --manifest-path roamium/Cargo.toml --all
   PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" ./scripts/build.sh roamium
   ```

   If the experiment produces a coherent Chromium branch, commit it on
   `148.0.7778.97-issue-794-exp8`, regenerate the patch archive under
   `chromium/patches/issue-794-exp8/`, and update `chromium/README.md`.

7. Run formatters and checks.
   - Run Chromium formatting on modified C++ files.
   - Run syntax checks for new or modified Python/JavaScript/shell scripts.
   - Run `prettier` on this experiment file and the issue README.
   - Run `cargo fmt` if any Rust files change.

## Verification

1. Build any modified components.

2. Run direct protocol PDF resize:

   ```bash
   LOG_DIR="logs/issue-794-exp8-pdf-resize-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-resize.py \
     --serve-bitcoin-pdf \
     --initial-width 900 \
     --initial-height 700 \
     --final-width 1300 \
     --final-height 900 \
     --log-dir "$LOG_DIR"
   ```

   Required for pass on the direct protocol path:
   - `first_failing_hop = no-failure-observed`;
   - after viewport width/height match the expected CSS viewport size within a
     small tolerance. The expected CSS size is derived from the observed initial
     viewport and the protocol resize ratio because the protocol uses physical
     pixels while DevTools reports CSS pixels;
   - PDF viewer/container/plugin bounds change in the expected direction;
   - page/content geometry changes in the expected direction, using `#sizer`,
     page/page-like bounds, fit/zoom state, or PDFium/plugin geometry traces;
   - screenshot hash changes;
   - if traced, PDFium plugin size changes.

   If only the viewport/viewer/container/plugin bounds change, but no
   page/content geometry is observable, record `page-reflow-not-observable` or
   Partial. Do not call that proof of PDF reflow.

3. Run shrink-back resize if the first resize passes:

   ```bash
   LOG_DIR="logs/issue-794-exp8-pdf-resize-shrink-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-resize.py \
     --serve-bitcoin-pdf \
     --initial-width 1300 \
     --initial-height 900 \
     --final-width 900 \
     --final-height 700 \
     --log-dir "$LOG_DIR"
   ```

4. Run normal HTML resize regression.

   Use the same harness against `test-html/public/test-interactions.html` or an
   equivalent local HTML page. Required: viewport and ordinary HTML content
   bounds change with the protocol resize.

5. Run PDF wheel, PDF drag, and PDF key-selection regressions from Experiments
   4, 6, and 7.

   Required:
   - wheel still reports `first_failing_hop = no-failure-observed`;
   - drag still reports `first_failing_hop = no-failure-observed`;
   - key-selection still reports `first_failing_hop = no-failure-observed`,
     `chromium_key_target_classification = pdf-plugin`, a recorded clipboard
     baseline, and `clipboard_after_sha256 != clipboard_before_sha256`.

6. Record the result in this file.

   The result must include:
   - branch and commit hash if Chromium changed;
   - build commands and results;
   - log directories for grow, shrink, HTML resize, wheel regression, drag
     regression, and key-selection regression;
   - before/after geometry table;
   - first failing layer if any;
   - next experiment target.

7. Codex must review the completed output.

   Do not proceed to Experiment 9 until real issues from Codex's review are
   addressed.

## Pass Criteria

Experiment 8 passes if it either:

- proves direct TermSurf protocol resize correctly resizes/reflows the PDF
  viewer and records the evidence, making the next target the real Wezboard
  pane-resize path; or
- identifies the exact layer where direct protocol resize stops and records the
  next fix target.

## Partial Criteria

Experiment 8 is partial if:

- the resize harness runs but cannot observe enough PDF geometry to distinguish
  viewer bounds from plugin/page bounds;
- direct grow resize works but shrink resize fails;
- PDF geometry changes but screenshots remain ambiguous;
- traces are present in Roamium/Chromium but missing in the PDF plugin/PDFium
  renderer process due to an environment propagation issue.

## Failure Criteria

Experiment 8 fails if:

- it uses DevTools resize or browser-window resize as the stimulus instead of a
  TermSurf protocol `Resize`;
- it changes PDF loading, stream, extension, resource, wheel, mouse, or keyboard
  routing paths;
- it treats a top-level viewport resize as sufficient proof when PDF
  viewer/plugin bounds did not change;
- it modifies Chromium without a fresh Issue 794 Experiment 8 branch;
- it omits Codex design or completion review.

## Result

**Result:** Pass

Chromium branch: `148.0.7778.97-issue-794-exp8`

Chromium commit: `7538ea05fee1f` (`Trace PDF resize reflow`)

Patch archive: `chromium/patches/issue-794-exp8/`

Builds:

```bash
cd chromium/src
export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium
```

Result: build succeeded.

```bash
cd "$HOME/dev/termsurf"
PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" cargo-fmt --manifest-path roamium/Cargo.toml --all
PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" ./scripts/build.sh roamium
```

Result: build succeeded.

Verification logs:

- PDF grow resize: `logs/issue-794-exp8-pdf-resize-20260530-094919`
- PDF shrink resize: `logs/issue-794-exp8-pdf-resize-shrink-20260530-094951`
- HTML resize regression: `logs/issue-794-exp8-html-resize-20260530-095022`
- PDF wheel regression: `logs/issue-794-exp8-wheel-regression-20260530-094411`
- PDF drag regression: `logs/issue-794-exp8-pdf-drag-regression-20260530-094440`
- PDF key-selection regression:
  `logs/issue-794-exp8-pdf-key-regression-20260530-094509`

Results:

- PDF grow resize reported `first_failing_hop = no-failure-observed`,
  `resize_messages_sent = 2`, `roamium_resize_line = true`,
  `roamium_resize_ffi_line = true`, `chromium_resize_line = true`,
  `pdf_plugin_update_geometry_line = true`,
  `pdf_plugin_geometry_changed_line = true`,
  `pdfium_plugin_size_updated_line = true`, and
  `before_after_screenshot_changed = true`.
- PDF grow viewport scaling was reconciled explicitly: the protocol resize was
  `900x700` physical pixels to `1300x900` physical pixels, DevTools reported
  `devicePixelRatio = 2`, and the expected CSS viewport was `650x450`. The
  observed after viewport was `650x450`.
- PDF grow geometry changed as expected:
  - viewport: `+200` width, `+100` height;
  - viewer: `+200` width, `+100` height;
  - container: `+200` width, `+100` height;
  - plugin: `-101` width, `+100` height, `+301` x.
- PDF shrink resize reported `first_failing_hop = no-failure-observed` with the
  same Roamium/Chromium/PDF plugin/PDFium trace ladder present.
- PDF shrink viewport scaling was also reconciled: the expected CSS viewport was
  `450x350`, and the observed after viewport was `450x350`.
- PDF shrink geometry changed in the opposite direction:
  - viewport: `-200` width, `-100` height;
  - viewer: `-200` width, `-100` height;
  - container: `-200` width, `-100` height;
  - plugin: `+101` width, `-100` height, `-301` x.
- DOM page-like elements were not observable from the extension page, but the
  PDF plugin and PDFium traces proved the resize reached
  `PdfViewWebPlugin::OnGeometryChanged()` and
  `PDFiumEngine::PluginSizeUpdated()`. The experiment therefore proves the
  direct TermSurf protocol resize path reaches the PDF rendering layer, not
  merely the outer `WebContents`.
- The HTML resize regression reported `first_failing_hop = no-failure-observed`,
  viewport delta `+200` width / `+100` height, ordinary HTML content delta
  `+200` width / `-62` height, and a changed screenshot.
- The PDF wheel regression reported `first_failing_hop = no-failure-observed`.
- The PDF drag regression reported `first_failing_hop = no-failure-observed` and
  `pdfium_selection_nonempty_line = true`.
- The PDF key-selection regression reported
  `first_failing_hop = no-failure-observed`,
  `chromium_key_target_classification = pdf-plugin`,
  `clipboard_before_text_length = 0`, and a changed clipboard hash after
  protocol `Cmd+A` / `Cmd+C`.

## Conclusion

Direct TermSurf protocol resize works. A second `Resize` message for an existing
PDF tab reaches Roamium, calls `ts_set_view_size`, reaches Chromium
`TsBrowserMainParts::ResizeTab()`, changes the PDF viewer/container/plugin
geometry, triggers `PdfViewWebPlugin::OnGeometryChanged()`, and reaches
`PDFiumEngine::PluginSizeUpdated()`. Grow and shrink both work.

This means the earlier manual resize complaint is not caused by Roamium's
protocol dispatch, Chromium `WebContents` resizing, the PDF extension frame, or
PDFium plugin-size propagation on the direct protocol path. If resize remains
wrong in a real split-pane session, the next experiment should move outward to
the real Wezboard pane-resize path and prove whether Wezboard sends the correct
second `Resize` message and screen rect when panes are split or resized.
