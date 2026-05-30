# Experiment 17: Trace the PDF Print Message Bridge

## Description

Experiment 16 narrowed print failure to the viewer-to-plugin message bridge. The
default path stayed safe, and the contained print guard could expose the toolbar
only in explicit test mode. But even after activating the visible print control,
dispatching the toolbar `print` event, calling `currentController.print()`,
calling `pluginController_.print()`, and directly posting `{type: "print"}` to
the plugin element, no intercept fired in `PdfViewWebPlugin::Print()` or
`PdfViewWebPluginClient::Print()`.

That means the next experiment should not add another print implementation. It
should trace the exact bridge used by PDF viewer messages and compare `print`
with a known-working message such as `rotateCounterclockwise` or `viewport`.

The goal is to answer one concrete question:

> Where does the `print` message stop between PDF viewer JavaScript and
> `PdfViewWebPlugin::OnMessage()`?

This is diagnostic-first. Do not change production print behavior in this
experiment.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a new Chromium branch.

   Fork the current Issue 794 Chromium branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp16
   git checkout -b 148.0.7778.97-issue-794-exp17
   ```

   Add the branch to `chromium/README.md` with a description such as "Trace PDF
   print message bridge."

2. Add a single trace gate for the PDF message bridge.

   Use explicit environment variables:

   ```text
   TERMSURF_PDF_PRINT_BRIDGE_TRACE=1
   TERMSURF_PDF_PRINT_BRIDGE_TRACE_FILE=/path/to/pdf-print-bridge.log
   ```

   The harness owns native trace-file creation. It should truncate/create
   `$LOG_DIR/pdf-print-bridge.log` before launching Roamium or before the
   contained activation sequence. Native Chromium trace sites must append only.

   The native trace helper must:
   - require both variables;
   - append structured one-line records;
   - never change message behavior;
   - avoid logging full PDF data or arbitrary payloads.

   JavaScript cannot read process environment variables or append directly to
   the native trace file. JS-side trace records must use a separate transport:
   add a narrow PDF load-time flag such as `termsurfPdfPrintBridgeTrace`, push
   structured records into an in-memory ring buffer such as
   `window.__termsurfPdfPrintBridgeTrace`, and have the DevTools probe collect
   that array after activation. The probe should merge the JS records with the
   native `pdf-print-bridge.log` records in its JSON summary.

3. Instrument viewer-side JavaScript message send points.

   In Chromium PDF viewer resources, trace only when the load-time bridge trace
   flag is active:
   - `pdf_viewer.ts::onPrint_()`
     - record that the toolbar-level print handler ran;
     - record whether `currentController` exists.
   - `controller.ts::PluginController.print()`
     - record that the controller attempted to send `{type: "print"}`.
   - `controller.ts::PluginController.postMessage_()`
     - record the message `type` for `print`, `rotateCounterclockwise`,
       `rotateClockwise`, and `viewport`;
     - record whether `plugin_` exists;
     - record whether `delayedMessages_` is still active.
   - `controller.ts::PluginController.bindMessageHandler()`
     - record that the message port was bound;
     - record how many delayed messages were flushed;
     - record the message types flushed.

   Keep the logging narrow. The comparison messages are included only so the
   result can say whether print is uniquely broken or whether the bridge is
   generally unavailable.

4. Instrument plugin-side message receipt.

   In `pdf/pdf_view_web_plugin.cc`:
   - log at the start of `PdfViewWebPlugin::OnMessage()` before the handler
     table lookup;
   - record the incoming `type` when it is one of `print`,
     `rotateCounterclockwise`, `rotateClockwise`, or `viewport`;
   - log immediately before `HandlePrintMessage()` calls `Print()`;
   - keep the Experiment 16 contained print intercept in place so any successful
     print message remains non-native during the test.

5. Extend the save/print/title/local probe to capture bridge traces.

   Update `scripts/test-issue-794-pdf-toolbar.py` so
   `--enable-pdf-print-intercept` also sets:

   ```text
   TERMSURF_PDF_PRINT_BRIDGE_TRACE=1
   TERMSURF_PDF_PRINT_BRIDGE_TRACE_FILE=$LOG_DIR/pdf-print-bridge.log
   ```

   Pass the bridge trace file path to
   `scripts/probe-pdf-save-print-title-local.mjs`. The harness should create or
   truncate the native bridge trace file before activation. The probe should
   collect both:
   - native append-only records from `pdf-print-bridge.log`;
   - JS records from `window.__termsurfPdfPrintBridgeTrace` in the PDF extension
     frame.

   Update the probe summary so the `print` result includes:
   - bridge trace path;
   - ordered native bridge trace lines after the print activation;
   - ordered JS bridge trace records after the print activation;
   - first missing bridge hop.

   Every JS and native trace record should include an activation label, such as
   `rotate-1` or `print-1`, so delayed message flushes cannot be mistaken for
   the wrong action.

6. Add an explicit compare step.

   In the contained run, perform this sequence:
   1. activate rotate and capture bridge lines;
   2. activate print and capture bridge lines;
   3. compare the two paths.

   The rotate comparison is usable only if the plugin-side native trace observes
   the rotate message reaching `PdfViewWebPlugin::OnMessage()`. If rotate does
   not reach the plugin trace, classify the result as Partial because the
   known-working comparison path was not proven.

   The result must classify one of:
   - `print-stops-before-viewer-handler`;
   - `print-stops-before-controller-print`;
   - `print-stops-before-post-message`;
   - `print-stops-in-delayed-message-queue`;
   - `print-posted-but-not-received-by-plugin`;
   - `print-received-by-plugin-but-not-dispatched`;
   - `print-received-by-plugin-but-contained-intercept-missing`;
   - `print-reaches-contained-intercept`.

7. Build Chromium and Roamium.

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

8. Verify default behavior remains safe.

   Run the save/print/title/local probe without intercept mode:

   ```bash
   LOG_DIR="logs/issue-794-exp17-print-default-$(date +%Y%m%d-%H%M%S)" \
     scripts/test-issue-794-pdf-toolbar.py \
     --probe save-print-title-local \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf
   ```

   Required:
   - `printingEnabled=false`;
   - print is not clicked;
   - no `pdf-print.log`;
   - no `pdf-print-bridge.log`;
   - title/save/local regressions still pass.

9. Verify contained bridge tracing.

   Run:

   ```bash
   LOG_DIR="logs/issue-794-exp17-print-bridge-$(date +%Y%m%d-%H%M%S)" \
     scripts/test-issue-794-pdf-toolbar.py \
     --probe save-print-title-local \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf \
     --enable-pdf-print-intercept
   ```

   Required:
   - `printingEnabled=true`;
   - the print control is visible and enabled;
   - `pdf-print-bridge.log` is written;
   - the summary records the first missing bridge hop;
   - if the message reaches `PdfViewWebPlugin::Print()`, the contained intercept
     writes `pdf-print.log` and no native print occurs;
   - title/save/local/embedded-title regressions still pass.

10. Run focused regressions.

    Re-run:
    - PDF wheel scroll regression;
    - PDF toolbar event regression.

11. Archive Chromium patches only after a coherent branch result.

    If the experiment passes or produces a coherent partial branch, commit the
    Chromium branch and regenerate:

    ```bash
    cd chromium/src
    rm -rf ../../chromium/patches/issue-794-exp17/
    git format-patch 148.0.7778.97..HEAD \
      -o ../../chromium/patches/issue-794-exp17/
    ```

12. Formatting and review.

    If Markdown changes, run Prettier. If Rust changes, run `cargo fmt` and
    accept its output. If Chromium C++ changes, run Chromium formatting on the
    modified files if practical.

    Codex must review the completed output before Experiment 18 is designed.

## Verification

| Check                                     | Required result                      |
| ----------------------------------------- | ------------------------------------ |
| Codex design review completed             | Yes                                  |
| Chromium branch exists and is recorded    | Yes                                  |
| Default print disabled                    | Yes                                  |
| Default run writes no bridge trace        | Yes                                  |
| Contained run writes bridge trace         | Yes                                  |
| Rotate comparison bridge trace exists     | Yes                                  |
| Print bridge first missing hop identified | One named classification from step 6 |
| Native print dialog avoided               | Yes                                  |
| Real printer job avoided                  | Yes                                  |
| Title propagation regression              | Pass                                 |
| Save/download regression                  | Pass                                 |
| Local PDF parity regression               | Pass                                 |
| Embedded PDF title regression             | Pass                                 |
| PDF wheel scroll regression               | Pass                                 |
| PDF toolbar event regression              | Pass                                 |
| Codex completion review completed         | Yes                                  |

## Pass Criteria

This experiment passes if it identifies the exact first missing bridge hop for
`print` by comparing it against a known-working message, while preserving the
default no-print safety behavior and all existing PDF interaction regressions.

If `print` reaches the contained intercept and writes `pdf-print.log`, this
experiment may also pass as the contained print activation proof that Experiment
16 attempted.

## Partial Criteria

This experiment is partial if the trace narrows the failure but cannot name one
exact hop, for example because multiple expected trace points are absent or the
bridge trace itself fails to initialize.

Record the strongest proven boundary and design the next experiment around the
first unproven layer.

## Failure Criteria

This experiment fails if:

- it changes production print behavior;
- it enables print without the explicit intercept guard;
- it opens a native print dialog;
- it submits a real print job;
- it adds broad Chrome print-preview infrastructure;
- it records verbose PDF payloads or arbitrary document data;
- it regresses title propagation, save/download, PDF rendering, wheel scrolling,
  or toolbar controls.

## Result

**Result:** Partial

Experiment 17 disproved Experiment 16's working hypothesis. The viewer-to-plugin
message bridge is not the missing layer for print.

Implemented pieces:

- added the `TERMSURF_PDF_PRINT_BRIDGE_TRACE` /
  `TERMSURF_PDF_PRINT_BRIDGE_TRACE_FILE` native trace gate;
- forwarded that trace gate to renderer command lines from `TsBrowserClient`;
- added a PDF load-time flag, `termsurfPdfPrintBridgeTrace`;
- added JS-side in-memory bridge records in `pdf_viewer.ts` and `controller.ts`;
- added native bridge records at `PdfViewWebPlugin::OnMessage()` and
  `HandlePrintMessage()`;
- extended the save/print/title/local probe to label activations (`rotate-1`,
  `print-1`), collect JS/native traces, and classify the first missing hop.

Verification evidence:

- Chromium build passed: `autoninja -C out/Default libtermsurf_chromium`.
- Roamium debug build passed: `./scripts/build.sh roamium`.
- Default/no-intercept run:
  `logs/issue-794-exp17-print-default-20260530-140319/`
  - `printingEnabled=false`;
  - print was not clicked;
  - no `pdf-print.log` was created;
  - no `pdf-print-bridge.log` was created;
  - title propagation, save/download, local parity, and embedded title safety
    passed.
- Contained bridge run: `logs/issue-794-exp17-print-bridge-20260530-140833/`
  - `printingEnabled=true`;
  - bridge trace was written;
  - rotate comparison reached plugin-side native trace (`rotateNativeCount=6`);
  - print produced JS trace records (`printJsCount=7`);
  - print reached plugin-side native trace (`printNativeCount=12`);
  - native trace included `event=handle-print type=print activationId=print-1`;
  - no fresh `pdf-print.log` line was created;
  - classification: `print-received-by-plugin-but-contained-intercept-missing`;
  - title propagation, save/download, local parity, and embedded title safety
    passed.
- Focused PDF wheel scroll regression:
  `logs/issue-794-exp17-regression-scroll-20260530-141030/`
  - `first_failing_hop=no-failure-observed`;
  - six scroll events were sent;
  - before/after screenshot and state both changed.
- Focused PDF toolbar event regression:
  `logs/issue-794-exp17-regression-toolbar-events-20260530-141055/`
  - `toolbar-events-summary.json` reported `status=pass`.

No native print dialog was observed, and no real printer job was submitted.

## Conclusion

The PDF viewer's print activation path reaches the internal plugin:

1. `pdf_viewer.ts::onPrint_()` runs.
2. `PluginController.print()` runs.
3. `PluginController.postMessage_({type: "print"})` runs.
4. `PdfViewWebPlugin::OnMessage()` receives `type=print`.
5. `PdfViewWebPlugin::HandlePrintMessage()` runs.

The next missing layer is the contained print intercept inside
`PdfViewWebPlugin::Print()`. The most likely explanation is that the renderer
process running the PDF plugin receives the bridge trace guard but not the print
intercept guard, or that the intercept path check is not shared correctly
between the two renderer-side helpers.

The next experiment should stop tracing the viewer bridge and instead prove the
renderer-side guard state at `PdfViewWebPlugin::Print()`:

- log whether `termsurf-pdf-print-intercept` and
  `termsurf-pdf-print-intercept-file` are present on the renderer command line;
- log whether `GetTermSurfPdfPrintInterceptPath()` returns a path at the top of
  `PdfViewWebPlugin::Print()`;
- compare that with the bridge trace path, which is known to reach the same
  renderer process;
- if the guard is missing, fix the forwarding/predicate mismatch;
- if the guard is present but append fails, record the file write error and fix
  that path.

Do not proceed to production print behavior until this contained intercept fires
reliably.
