# Experiment 20: Install the Renderer Print Helper

## Description

Experiment 19 made the production PDF print button visible and proved the click
reaches `PdfViewWebPlugin::Print()`. The manual smoke test then showed the next
failure: clicking print does not open native print UI. Instead, the PDF document
viewport turns gray while the toolbar and thumbnail rail remain visible.

That means the toolbar and PDF plugin print entry point are no longer the
problem. The failure is downstream of:

```text
PdfViewWebPlugin::Print()
  -> PdfViewWebPlugin::OnInvokePrintDialog()
  -> PdfViewWebPluginClient::Print()
  -> printing::PrintRenderFrameHelper::Get(render_frame_)->PrintNode(element)
```

Chrome creates a `printing::PrintRenderFrameHelper` for each renderer frame in
`ChromeContentRendererClient::RenderFrameCreated()`. TermSurf's
`TsContentRendererClient::RenderFrameCreated()` currently sets up extension and
MimeHandlerView renderer helpers, but it does not create a print helper.

Experiment 20 tests and fixes that specific missing layer. It adds targeted
trace logging around the PDF native-print handoff, installs a minimal
TermSurf-owned `PrintRenderFrameHelper::Delegate`, and verifies whether that is
enough to surface native print UI from the PDF plugin.

This experiment must not add Chrome print preview, printer job management,
PDF-generation UI, new TermSurf protocol messages, or a custom print dialog. If
installing the renderer helper gets the print path past the current gray-view
failure but still does not produce native UI, record the exact next missing
layer, likely the browser-side `PrintManagerHost`/`PrintViewManager` plumbing.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a new Chromium branch.

   Fork the current Issue 794 Chromium branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp19
   git checkout -b 148.0.7778.97-issue-794-exp20
   ```

   Add the branch to `chromium/README.md` with a description such as "Install
   renderer print helper."

2. Add a native-print trace gate.

   Add a narrow trace helper for this experiment, gated by:

   ```text
   TERMSURF_PDF_NATIVE_PRINT_TRACE=1
   TERMSURF_PDF_NATIVE_PRINT_TRACE_FILE=/absolute/path/to/pdf-native-print.log
   ```

   The trace should append one-line records to the configured file. If the file
   env var is absent, tracing can also use `LOG(INFO)`, but the manual
   verification must prefer direct file records so the result is not hidden in
   large Chromium logs.

   Trace at least:
   - `PdfViewWebPlugin::Print()` after permission checks and before the posted
     `OnInvokePrintDialog()` task;
   - `PdfViewWebPlugin::OnInvokePrintDialog()`;
   - `PdfViewWebPluginClient::Print()`;
   - whether `printing::PrintRenderFrameHelper::Get(render_frame_)` returns a
     helper;
   - whether `PrintNode(element)` is about to be called;
   - the document URL, process id, renderer frame pointer, and `ENABLE_PRINTING`
     state where available.

   Also add gated trace points inside
   `components/printing/renderer/print_render_frame_helper.cc`, because a trace
   that stops at "about to call `PrintNode()`" cannot distinguish a renderer
   helper success from a browser-side print-host failure. Trace at least:
   - `PrintRenderFrameHelper::PrintNode()` entry;
   - the result of `InitPrintSettings(...)`;
   - calls that request or update print settings through
     `GetPrintManagerHost()`;
   - calls that notify the browser side, including `DidShowPrintDialog()`,
     `ScriptedPrint(...)`, `DidPrintDocument(...)`, or the closest equivalent
     callsites reached on Chromium 148;
   - failure exits before printing reaches a browser-side host.

   Keep Experiment 18's contained print intercept untouched. The new trace must
   not make automation click production print.

3. Make the PDF print client handle a missing print helper explicitly.

   In `components/pdf/renderer/pdf_view_web_plugin_client.cc`, avoid blindly
   dereferencing the result of `PrintRenderFrameHelper::Get(render_frame_)`.

   If the helper is missing:
   - write a trace line such as `helper=missing`;
   - return without invoking native print;
   - do not crash the renderer;
   - do not synthesize a fake successful print result.

   This is both a diagnostic improvement and a guard against the gray-view
   failure hiding the actual missing layer.

   Do not require a separate runtime baseline that clicks production print
   before installing the helper. The static code audit already shows TermSurf
   does not create `PrintRenderFrameHelper` in
   `TsContentRendererClient::RenderFrameCreated()`. If `helper=missing` is ever
   observed after the helper install, treat it as a wiring failure in this
   experiment.

4. Install a minimal print helper from TermSurf's renderer client.

   In `content/libtermsurf_chromium/ts_content_renderer_client.cc`, create
   `printing::PrintRenderFrameHelper` in
   `TsContentRendererClient::RenderFrameCreated()`, matching Chrome's
   renderer-client pattern.

   Use a small TermSurf-owned delegate rather than linking Chrome's
   `ChromePrintRenderFrameHelperDelegate`. The delegate should be intentionally
   conservative:
   - `GetPdfElement(blink::WebLocalFrame*)` may return a null
     `blink::WebElement()` unless implementation proves the PDF plugin requires
     Chrome's extension-parent lookup;
   - `IsPrintPreviewEnabled()` should not enable Chrome print preview for this
     experiment;
   - `IsScriptedPrintEnabled()` should return `false` unless implementation
     proves scripted printing is required for the PDF toolbar path;
   - `OverridePrint(...)` should return `false`;
   - `ShouldGenerateTaggedPDF()` can return `false` unless Chromium requires
     otherwise.

   This experiment is about the explicit PDF toolbar print command, which calls
   `PrintNode(element)` directly. It must not accidentally enable arbitrary
   page-level `window.print()` behavior for non-PDF pages.

   Add the smallest required GN dependency, likely
   `//components/printing/renderer`, only if `libtermsurf_chromium` does not
   already get it transitively.

5. Build Chromium and Roamium.

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Then:

   ```bash
   cd /Users/ryan/dev/termsurf
   CARGO_BIN="$(/opt/homebrew/bin/rustup which cargo)"
   PATH="$(dirname "$CARGO_BIN"):$PATH" ./scripts/build.sh roamium
   ```

6. Re-run safe automated regressions.

   Re-run the same checks that Experiment 19 used:
   - default production print visibility, without clicking print;
   - contained print intercept, which clicks print only with the intercept file;
   - PDF wheel scroll regression;
   - PDF toolbar event regression;
   - a basic non-PDF page smoke test.

   Required:
   - the print control remains visible in default production mode;
   - default automation still refuses to click production print;
   - contained print still reaches `print-contained-callback`;
   - scroll and toolbar regressions still pass;
   - a normal non-PDF page still loads, scrolls, clicks, and accepts text input.

7. Run the required manual production print smoke with trace enabled.

   Start debug Wezboard/Roamium with:

   ```bash
   export TERMSURF_PDF_NATIVE_PRINT_TRACE=1
   export TERMSURF_PDF_NATIVE_PRINT_TRACE_FILE=/Users/ryan/dev/termsurf/logs/issue-794-exp20-manual/pdf-native-print.log
   ```

   Then:
   - open the Bitcoin PDF in the debug app;
   - click the PDF print button;
   - if native print UI appears, cancel it;
   - confirm the PDF pane remains usable;
   - capture or preserve a screenshot if the PDF turns gray again;
   - record the trace file lines in this experiment's result.

   This is the only verification step allowed to click production print without
   the contained intercept.

8. Classify the native print result.

   Use the trace to classify the first failing layer:

   | Trace outcome                                              | Meaning                                                                    | Result direction                                     |
   | ---------------------------------------------------------- | -------------------------------------------------------------------------- | ---------------------------------------------------- |
   | `helper=missing` after the helper install                  | Renderer-helper wiring did not reach the PDF frame                         | Partial/Fail based on regression severity            |
   | `helper=present` and `PrintNode` called, native UI appears | Renderer helper was the missing layer                                      | Pass                                                 |
   | `helper=present`, `PrintNode` called, no print-host trace  | Renderer layer fixed; browser-side print host binding is missing           | Partial                                              |
   | Print-host trace starts, then PDF still turns gray         | Browser-side print manager/UI path is entered but cannot surface native UI | Partial                                              |
   | `OnInvokePrintDialog()` not reached                        | Posted task or plugin lifetime issue                                       | Partial; design next experiment around task/lifetime |
   | Renderer crash or sad-tab                                  | Capture crash/logs and mark Partial or Fail based on regression severity   | Partial/Fail                                         |

9. Archive Chromium patches only after a coherent branch result.

   If the experiment passes or produces a coherent partial branch, commit the
   Chromium branch and regenerate:

   ```bash
   cd chromium/src
   rm -rf ../../chromium/patches/issue-794-exp20/
   git format-patch 148.0.7778.97..HEAD \
     -o ../../chromium/patches/issue-794-exp20/
   ```

10. Formatting and review.

    If Markdown changes, run Prettier. If Rust changes, run `cargo fmt` and
    accept its output. If Chromium C++ changes, run Chromium formatting on the
    modified files if practical.

    Codex must review the completed output before Experiment 21 is designed.

## Verification

| Check                                      | Required result |
| ------------------------------------------ | --------------- |
| Codex design review completed              | Yes             |
| Chromium branch exists and is recorded     | Yes             |
| Native print trace gate works              | Yes             |
| Missing helper behavior is loggable if hit | Yes             |
| TermSurf renderer print helper installed   | Yes             |
| `PrintNode(element)` path traced           | Yes             |
| Print helper internal path traced          | Yes             |
| Default print visibility regression        | Pass            |
| Contained print intercept regression       | Pass            |
| PDF wheel scroll regression                | Pass            |
| PDF toolbar event regression               | Pass            |
| Basic non-PDF page smoke                   | Pass            |
| Manual production print click/cancel smoke | Pass or Partial |
| Codex completion review completed          | Yes             |

## Pass Criteria

This experiment passes if the renderer print helper is installed, the production
PDF print button opens native print UI during the manual debug-app smoke test,
the user can cancel it without leaving the PDF pane gray or unusable, and all
safe automated regressions still pass.

## Partial Criteria

This experiment is partial if:

- it confirms the renderer print helper was missing but native print UI still
  does not appear after installing it;
- `PrintNode(element)` is called and the next missing layer is browser-side
  `PrintManagerHost`/`PrintViewManager` wiring;
- the renderer helper path enters browser-side print code but the browser-side
  manager/UI path still cannot surface native print UI;
- the trace identifies a plugin lifetime or posted-task issue that needs a
  separate experiment;
- all automated regressions pass but the manual production print smoke cannot be
  run.

Record the exact first failing trace line and design the next experiment around
that layer.

## Failure Criteria

This experiment fails if:

- it regresses PDF rendering, scrolling, toolbar controls, title propagation,
  local PDF parity, or contained print automation;
- automation clicks production print without the contained intercept;
- it opens a native print dialog during automated verification;
- it submits a real printer job;
- it adds Chrome print preview, printer job submission UI, PDF-generation UI, or
  new TermSurf protocol messages;
- it hides the print button again instead of fixing or classifying the native
  print path;
- it omits Codex design or completion review.

## Result

**Result:** Pass under the revised Issue 794 scope.

Experiment 20 installed the renderer-side print helper and preserved all safe
automated PDF regressions. The manual smoke test no longer opens native print
UI, but it also no longer turns the PDF viewport gray. The user accepted this as
sufficient for Issue 794 and explicitly deferred native PDF printing to a
separate follow-up issue.

Relative to the original Experiment 20 pass criteria, native PDF printing
remains incomplete. This result is Pass only because Issue 794's scope was
revised to exclude native print.

Implemented pieces:

- created Chromium branch `148.0.7778.97-issue-794-exp20`;
- forwarded `TERMSURF_PDF_NATIVE_PRINT_TRACE` and
  `TERMSURF_PDF_NATIVE_PRINT_TRACE_FILE` to renderer processes;
- added native-print trace points in `PdfViewWebPlugin::Print()`,
  `PdfViewWebPlugin::OnInvokePrintDialog()`, `PdfViewWebPluginClient::Print()`,
  and `PrintRenderFrameHelper`'s print path;
- guarded the PDF print client against a missing `PrintRenderFrameHelper`;
- installed a conservative TermSurf-owned `PrintRenderFrameHelper::Delegate`
  from `TsContentRendererClient::RenderFrameCreated()`;
- kept scripted page-level printing disabled in that delegate so this experiment
  does not enable arbitrary `window.print()` behavior.

Design review evidence:

- Initial Codex review:
  `logs/codex-review/20260530-150055-207435-last-message.md`.
- Follow-up Codex review:
  `logs/codex-review/20260530-150300-731303-last-message.md`.
- Final Codex design review:
  `logs/codex-review/20260530-150349-811516-last-message.md`.
- Result: Codex found no remaining blockers after requiring internal
  `PrintRenderFrameHelper` trace, removing the impossible pre-install baseline,
  adding a non-PDF smoke test, and requiring `IsScriptedPrintEnabled() = false`.
- Completion review after the user revised the scope:
  `logs/codex-review/20260530-152122-475125-last-message.md`.
- Completion review result: no blocking findings remained; Codex agreed that
  closing Issue 794 under the revised non-print scope is coherent and that Issue
  795 is a sufficient follow-up for native PDF printing.

Build evidence:

- `autoninja -C out/Default libtermsurf_chromium` passed.
- `./scripts/build.sh roamium` passed with the cargo symlink workaround.
- Chromium formatting was applied with Chromium's `clang-format`.

Automated verification evidence:

- Default production print visibility:
  `logs/issue-794-exp20-print-default-20260530-150921/`
  - print control was present and enabled;
  - default automation did not click production print;
  - print status was `print-production-available-not-clicked`;
  - title, save/download, local PDF parity, extensionless PDF parity, and
    embedded title checks remained usable.
- Contained print intercept:
  `logs/issue-794-exp20-print-contained-20260530-151122/`
  - print was clicked only with the contained intercept active;
  - print status was `print-contained-callback`;
  - fresh `pdf-print-intercept` lines were written;
  - no native print dialog opened in automation;
  - no real printer job was submitted.
- PDF wheel scroll regression:
  `logs/issue-794-exp20-regression-scroll-20260530-151446/`
  - `first_failing_hop=no-failure-observed`;
  - six scroll events were sent;
  - before/after state and screenshot changed.
- PDF toolbar event regression:
  `logs/issue-794-exp20-regression-toolbar-events-20260530-151446/`
  - toolbar event summary reported `status=pass`.
- Non-PDF click smoke:
  `logs/issue-794-exp20-regression-nonpdf-click-20260530-151541/`
  - `first_failing_hop=no-failure-observed`;
  - before/after state and screenshot changed.

Manual production print smoke:

- Run by the user in debug Wezboard/Roamium.
- Result: clicking the PDF print button did not open native print UI.
- Improvement over Experiment 19: the PDF viewport did not turn gray.
- Accepted outcome: non-functioning PDF print is acceptable for now and should
  remain a separate follow-up rather than blocking Issue 794.

Chromium branch archive:

- Chromium commit: `e64e70c265cdb Install PDF print helper`.
- Patch archive: `chromium/patches/issue-794-exp20/`, ending in
  `0061-Install-PDF-print-helper.patch`.

## Conclusion

The renderer print helper work removed the visible gray-viewport regression from
the PDF print button and did not regress the completed PDF interaction surface.
Native print UI still does not appear, which means PDF printing is not complete.

Because the user explicitly deferred PDF printing, this experiment closes the
Issue 794 interaction work under the revised scope. Native PDF printing should
continue in a separate issue focused only on Chromium/Roamium print plumbing.
