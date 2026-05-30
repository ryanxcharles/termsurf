# Experiment 16: Prove Contained PDF Print Activation

## Description

Experiment 15 left one Issue 794 product gap: print. The current automated
save/print/title/local probe classifies print as `print-not-contained` because
it can see a print control, but it cannot safely click it. TermSurf should not
mark print support complete by opening a native print dialog during automation,
and it should not enable a user-facing print button that silently does nothing.

Chromium's current PDF viewer print path is:

1. `viewer-toolbar` dispatches a `print` event.
2. `pdf_viewer.ts::onPrint_()` calls `this.currentController.print()`.
3. `controller.ts::print()` posts `{type: 'print'}` to the internal plugin.
4. `pdf_viewer.ts::handleScriptingMessage()` receives `type: 'print'` and calls
   `this.pluginController_.print()`.
5. `PdfViewWebPluginClient::Print()` calls
   `printing::PrintRenderFrameHelper::Get(render_frame_)->PrintNode(element)`
   when printing is enabled.

TermSurf currently keeps `printingEnabled` conservative, and the probe refuses
to click print unless a contained/non-native proof path exists. Experiment 16
should add that proof path first: an explicit TermSurf print-intercept mode that
lets automation enable the print control, click it, and prove that the PDF
plugin print method was reached, while preventing the native print dialog or a
real printer job.

This is not yet the final user-facing print implementation. It is the controlled
activation experiment that makes the remaining print layer measurable. A later
experiment can decide whether the production behavior should open a native
dialog, generate a PDF file, or integrate a custom print preview flow.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a new Chromium branch.

   Fork the current good Issue 794 Chromium branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp15
   git checkout -b 148.0.7778.97-issue-794-exp16
   ```

   Add the branch to `chromium/README.md` with a description such as "Prove
   contained PDF print activation."

2. Add a gated TermSurf PDF print intercept.

   In Chromium, add a narrow helper for the PDF renderer print path. The helper
   should use one shared "intercept active" predicate. The predicate is true
   only when all required automation/debug inputs are present, for example:

   ```text
   TERMSURF_PDF_PRINT_INTERCEPT=1
   TERMSURF_PDF_PRINT_INTERCEPT_FILE=/path/to/pdf-print.log
   ```

   The predicate must require both the flag and a non-empty intercept-file path.
   The harness is responsible for passing a path under the test run's log
   directory. If the path is missing or empty, the intercept is not active and
   `printingEnabled` must remain false. Do not create a split-brain state where
   the browser UI enables print but the renderer print intercept is absent.

   The intercept should live as close as possible to the actual PDF print call,
   preferably in `components/pdf/renderer/pdf_view_web_plugin_client.cc` around
   `PdfViewWebPluginClient::Print()`.

   Required behavior when the intercept is enabled:
   - append one structured log line when `PdfViewWebPluginClient::Print()` is
     reached;
   - include enough data to identify the document/plugin context, such as the
     plugin document URL if available, renderer process id, and whether printing
     support was compiled in;
   - return before calling `PrintRenderFrameHelper::PrintNode(...)`;
   - never open a native print dialog;
   - never submit a real print job.

   The renderer must fail closed. If intercept mode is on but writing the
   intercept file fails, still return before `PrintRenderFrameHelper::PrintNode`
   and make the automated run fail because the fresh intercept line is missing.
   Never treat a logging failure as permission to fall through to native print.

   Required behavior when the intercept is disabled:
   - preserve the existing Chromium behavior;
   - do not suppress production print accidentally;
   - do not write logs.

3. Enable the PDF viewer print control only under the same contained test mode.

   Update TermSurf's PDF load-time data provider in:
   - `content/libtermsurf_chromium/extensions/ts_resources_private_api.cc`
   - `content/libtermsurf_chromium/extensions/ts_component_extension_resource_manager.cc`

   Keep `printingEnabled=false` by default. Expose `printingEnabled=true` only
   when the same shared intercept-active predicate from step 2 is true. This
   means both `TERMSURF_PDF_PRINT_INTERCEPT=1` and a valid
   `TERMSURF_PDF_PRINT_INTERCEPT_FILE` must be present before the toolbar print
   control becomes test-clickable.

   This avoids the dangerous state where automation enables the toolbar but the
   production browser can unexpectedly open a native dialog from a PDF pane.

4. Extend the save/print/title/local probe to click print in contained mode.

   Update `scripts/test-issue-794-pdf-toolbar.py` with an explicit opt-in flag,
   for example:

   ```text
   --enable-pdf-print-intercept
   ```

   When that flag is present for the `save-print-title-local` probe, the harness
   should set:

   ```text
   TERMSURF_PDF_PRINT_INTERCEPT=1
   TERMSURF_PDF_PRINT_INTERCEPT_FILE=$LOG_DIR/pdf-print.log
   ```

   The flag must be absent in the default/no-intercept verification run. Pass
   the print intercept file path to
   `scripts/probe-pdf-save-print-title-local.mjs`.

   Update `probePrint(...)` so it:
   - verifies that `printingEnabled` is true in the PDF extension frame;
   - verifies that the print control is present and enabled;
   - clicks the print control;
   - waits briefly;
   - reads the intercept file;
   - classifies the result as `print-contained-callback` only if a fresh
     intercept line appears after the click;
   - classifies any native-dialog signal, timeout, missing control, or missing
     intercept as Partial/Fail evidence, not Pass.

5. Preserve title/save/local checks.

   The Experiment 15 title checks should remain active. This experiment should
   not regress:
   - `titlePropagationPass=true`;
   - save/download `download-file-created`;
   - HTTP, `file://`, extensionless, and untitled PDF rendering;
   - embedded PDF host-title safety.

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
   CARGO_BIN="$(/opt/homebrew/bin/rustup which cargo)"
   PATH="$(dirname "$CARGO_BIN"):$PATH" ./scripts/build.sh roamium
   ```

   Never use `ninja` directly. Run `cargo fmt` if any Rust changes are made and
   accept its output.

7. Verify default no-intercept behavior first.

   Before the contained print run, run the save/print/title/local probe without
   setting the print-intercept environment:

   ```bash
   LOG_DIR="logs/issue-794-exp16-print-default-$(date +%Y%m%d-%H%M%S)" \
     scripts/test-issue-794-pdf-toolbar.py \
     --probe save-print-title-local \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf
   ```

   Required checks:
   - `printingEnabled` is false or not enabled;
   - the probe does not click print;
   - no `pdf-print.log` file is created;
   - the summary does not classify print as `print-contained-callback`;
   - title propagation and save/download still pass.

   This proves the production/default path is not accidentally enabled by the
   experiment.

8. Verify the contained print path.

   Re-run:

   ```bash
   LOG_DIR="logs/issue-794-exp16-print-$(date +%Y%m%d-%H%M%S)" \
     scripts/test-issue-794-pdf-toolbar.py \
     --probe save-print-title-local \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf \
     --enable-pdf-print-intercept
   ```

   Required print checks:
   - `printingEnabled` is observable as `true` only during the intercept run
     where both the env flag and intercept file path are present;
   - the print control is present and enabled;
   - clicking print creates a fresh `pdf-print.log` intercept line;
   - the summary classifies print as `print-contained-callback`;
   - no native print dialog opens;
   - no real printer job is submitted.

   Required regression checks:
   - `titlePropagationPass=true`;
   - save/download remains `download-file-created`;
   - local parity cases still render;
   - no missing-string console errors;
   - embedded PDF title safety still passes.

9. Run focused regressions.

   Re-run:
   - PDF wheel scroll regression;
   - PDF toolbar event regression;
   - one normal HTML click/title smoke test, if available.

10. Archive Chromium patches only after a coherent branch result.

If the experiment passes or produces a coherent partial branch, commit the
Chromium branch and regenerate:

```bash
cd chromium/src
rm -rf ../../chromium/patches/issue-794-exp16/
git format-patch 148.0.7778.97..HEAD \
  -o ../../chromium/patches/issue-794-exp16/
```

Update `chromium/README.md` in the main repo.

11. Formatting and review.

    If Markdown changes, run Prettier. If Rust changes, run `cargo fmt` and
    accept its output. If Chromium C++ changes, run Chromium formatting on the
    modified files if practical.

    Codex must review the completed output before Experiment 17 is designed.

## Verification

| Check                                   | Required result            |
| --------------------------------------- | -------------------------- |
| Codex design review completed           | Yes                        |
| Chromium branch exists and is recorded  | Yes                        |
| Print intercept disabled by default     | Yes                        |
| `printingEnabled` false by default      | Yes                        |
| No-intercept run avoids print click     | Yes                        |
| No-intercept run writes no print log    | Yes                        |
| `printingEnabled` true in intercept run | Yes                        |
| Print control clicked                   | Yes                        |
| Fresh print intercept line written      | Yes                        |
| Native print dialog avoided             | Yes                        |
| Real printer job avoided                | Yes                        |
| Print classification                    | `print-contained-callback` |
| Title propagation regression            | Pass                       |
| Save/download regression                | Pass                       |
| Local PDF parity regression             | Pass                       |
| Embedded PDF title regression           | Pass                       |
| PDF wheel scroll regression             | Pass                       |
| PDF toolbar event regression            | Pass                       |
| Codex completion review completed       | Yes                        |

## Pass Criteria

This experiment passes if the PDF print toolbar control can be enabled in an
explicit contained test mode, clicked automatically, and proven to reach
`PdfViewWebPluginClient::Print()` without opening a native print dialog or
submitting a real print job, while the existing title/save/local/interaction
regressions continue to pass.

## Partial Criteria

This experiment is partial if it identifies the exact remaining print layer but
does not produce a contained callback. Examples:

- `printingEnabled=true`, but the print control remains hidden or disabled;
- the print control clicks, but no plugin print intercept line appears;
- the plugin print intercept fires, but a native print dialog also appears;
- the contained print path works, but title/save/local regress.

Record the first failing layer and design the next experiment around that layer.

## Failure Criteria

This experiment fails if:

- it enables production print without the intercept guard;
- it opens a native print dialog during automated verification;
- it submits a real print job;
- it treats a disabled or unclicked print button as success;
- it adds broad Chrome print-preview infrastructure instead of the contained
  print activation proof;
- it regresses title propagation, save/download, PDF rendering, wheel scrolling,
  or toolbar controls.

## Result

**Result:** Partial

Experiment 16 proved the safety half of the design, but not the contained print
callback.

Implemented pieces:

- added a gated PDF print intercept in
  `components/pdf/renderer/pdf_view_web_plugin_client.cc`;
- added a second earlier gated intercept in `pdf/pdf_view_web_plugin.cc` after
  the first run showed the client layer was never reached;
- forwarded the contained print guard from the browser process to renderer
  command lines through `TsBrowserClient::AppendExtraCommandLineSwitches()`;
- kept `printingEnabled=false` by default and enabled it only when the explicit
  print-intercept environment variables are present;
- extended the save/print/title/local probe with `--enable-pdf-print-intercept`,
  print-intercept log checking, and runtime diagnostics for the PDF
  viewer/controller/plugin objects.

Verification evidence:

- Chromium build passed: `autoninja -C out/Default libtermsurf_chromium`.
- Roamium debug build passed: `./scripts/build.sh roamium`.
- Default/no-intercept run:
  `logs/issue-794-exp16-print-default-20260530-132404/`
  - `printingEnabled=false`;
  - print control was found but not clicked;
  - no `pdf-print.log` was created;
  - title propagation passed;
  - save/download created `bitcoin.pdf`;
  - HTTP, `file://`, extensionless, and untitled PDF local parity rendered;
  - embedded PDF host-title safety passed.
- Intercept run: `logs/issue-794-exp16-print-20260530-133929/`
  - `printingEnabled=true`;
  - print control was visible and enabled;
  - probe activated print via the toolbar event, `currentController.print()`,
    `pluginController_.print()`, and the underlying plugin element
    `postMessage({type: "print"})`;
  - runtime diagnostics showed `currentController.print`,
    `pluginController_.print`, and plugin `postMessage` all exist and
    `pluginController_.isActive=true`;
  - no `pdf-print.log` was created;
  - summary remained `print-intercept-missing`;
  - title propagation, save/download, local parity, and embedded title safety
    still passed.
- Focused PDF wheel scroll regression:
  `logs/issue-794-exp16-regression-scroll-20260530-134654/`
  - `first_failing_hop=no-failure-observed`;
  - six scroll events were sent;
  - before/after screenshot and state both changed.
- Focused PDF toolbar event regression:
  `logs/issue-794-exp16-regression-toolbar-events-20260530-134725/`
  - `toolbar-events-summary.json` reported `status=pass`.

The earlier DOM-click attempt briefly hung until the run was terminated. That
run did not create `pdf-print.log`, and the later guarded runs exited cleanly.
No native print output or real printer job was observed.

## Conclusion

The print toolbar can be exposed only under the explicit contained-test guard,
and the default path remains safe. However, the visible PDF viewer's print
activation does not currently reach the internal PDF plugin print handler.

The first failing layer is now narrower than "print": it is the viewer-to-plugin
message bridge for the `print` message. The viewer-side objects exist and report
active state, but `postMessage({type: "print"})` does not reach
`PdfViewWebPlugin::HandlePrintMessage()` / `PdfViewWebPlugin::Print()`.

The next experiment should instrument the PDF plugin message bridge directly:

- log when `PluginController.bindMessageHandler()` runs and whether delayed
  messages are flushed;
- log every viewer-side `postMessage_()` call for `type: "print"`;
- log every plugin-side message dispatch in `PdfViewWebPlugin` before the
  message-handler table;
- compare `print` with a known-working message such as rotate or viewport.

Do not proceed to a production print behavior until that bridge is proven.
