# Experiment 19: Enable the Production PDF Print Control

## Description

Experiment 18 proved that the PDF toolbar's print activation reaches
`PdfViewWebPlugin::Print()` and can be safely contained during automation. The
remaining product mismatch is that TermSurf still hides/disables the normal PDF
print control in default production mode because `printingEnabled` is tied to
the test-only intercept guard.

That is now the wrong default. The viewer has a real print path:

1. the toolbar click reaches viewer JavaScript;
2. viewer JavaScript posts `{type: "print"}`;
3. the message reaches `PdfViewWebPlugin::HandlePrintMessage()`;
4. `PdfViewWebPlugin::Print()` is entered;
5. with no test intercept active, Chromium's existing PDF print path continues
   to `OnInvokePrintDialog()` and `client_->Print()`.

Experiment 19 should expose the normal print control in production while keeping
automated tests safe. Automation must never click production print without the
contained intercept. It should only verify:

- the default viewer reports `printingEnabled=true` and the print control is
  visible/enabled;
- the default probe refuses to click production print without an intercept file;
- the contained intercept mode still clicks print and writes `pdf-print.log`;
- existing PDF interaction regressions still pass.

The actual production print click must still be verified before this experiment
can Pass. Because automation must not invoke native print UI, that production
click check is a manual smoke test: click print in the debug app, confirm a
native print UI appears, cancel it, and confirm the pane remains usable. If that
manual smoke is skipped, the experiment result is capped at Partial even if all
automated checks pass.

This experiment does not implement custom print preview, save-to-PDF, PDF
generation, printer job submission, or new TermSurf protocol messages. It only
stops hiding a print control whose Chromium path is now proven reachable.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a new Chromium branch.

   Fork the current Issue 794 Chromium branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp18
   git checkout -b 148.0.7778.97-issue-794-exp19
   ```

   Add the branch to `chromium/README.md` with a description such as "Enable PDF
   print control."

2. Decouple `printingEnabled` from the contained intercept guard while
   preserving print permissions.

   In TermSurf's PDF load-time data providers:
   - `content/libtermsurf_chromium/extensions/ts_resources_private_api.cc`;
   - `content/libtermsurf_chromium/extensions/ts_component_extension_resource_manager.cc`.

   Change the default PDF viewer load-time value so production print
   availability is no longer gated on `TERMSURF_PDF_PRINT_INTERCEPT`. Keep the
   existing `termsurfPdfPrintBridgeTrace` flag separate.

   Do not flatten Chromium/PDFium print permission semantics. The final
   `printingEnabled` value must still respect document/browser print
   restrictions. In practical terms:
   - a normal unrestricted PDF should expose `printingEnabled=true`;
   - a PDF that Chromium/PDFium marks print-restricted must not become printable
     merely because TermSurf decoupled printing from the intercept guard;
   - if the current TermSurf load-time provider cannot see document-level
     restrictions at load-time, record that limitation and keep a follow-up
     guard rather than silently claiming full permission support.

   Do not remove the contained print intercept. It is still needed for automated
   verification. Do not make production print depend on
   `TERMSURF_PDF_PRINT_INTERCEPT`.

3. Keep print containment only in the renderer print path.

   In `pdf/pdf_view_web_plugin.cc`, keep Experiment 18's behavior:
   - when no intercept/bridge trace mode is active, preserve Chromium's normal
     PDF print path;
   - when the contained intercept is active, append `pdf-print.log` and return
     before native print;
   - when bridge/guard probe mode is active and intercept state is missing,
     empty, mismatched, or append-failing, log the guard reason and return
     before native print.

   This preserves automation safety without changing the production path.

4. Update the save/print/title/local probe's default print classification.

   `scripts/probe-pdf-save-print-title-local.mjs` currently treats
   `printingEnabled=true` without `--print-intercept-file` as
   `print-not-contained`. For Experiment 19, that is no longer a failure. It is
   the expected production state.

   Update `probePrint(...)` so when:
   - `printingEnabled=true`;
   - the print control exists;
   - no `--print-intercept-file` was provided;

   it returns a non-clicking status such as
   `print-production-available-not-clicked`. The probe must record that it found
   the control and intentionally did not click it because no contained intercept
   was configured.

   The probe must still classify a missing or disabled print control as a
   problem when `printingEnabled=true`.

5. Keep contained print verification unchanged.

   In `--enable-pdf-print-intercept` mode, the harness still creates/truncates:

   ```text
   $LOG_DIR/pdf-print.log
   $LOG_DIR/pdf-print-bridge.log
   ```

   The probe must click print only in this contained mode and require:
   - `print.status=print-contained-callback`;
   - `bridgeClassification=print-reaches-contained-intercept`;
   - fresh `pdf-print-intercept` lines.

6. Build Chromium and Roamium.

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

7. Verify default production print visibility without clicking print.

   Run:

   ```bash
   LOG_DIR="logs/issue-794-exp19-print-default-$(date +%Y%m%d-%H%M%S)" \
     scripts/test-issue-794-pdf-toolbar.py \
     --probe save-print-title-local \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf
   ```

   Required:
   - `printingEnabled=true`;
   - the print control is present and enabled;
   - print is not clicked;
   - print status is `print-production-available-not-clicked`;
   - no `pdf-print.log`;
   - no `pdf-print-bridge.log`;
   - title/save/local/embedded-title regressions still pass.

8. Verify contained print still works.

   Run:

   ```bash
   LOG_DIR="logs/issue-794-exp19-print-contained-$(date +%Y%m%d-%H%M%S)" \
     scripts/test-issue-794-pdf-toolbar.py \
     --probe save-print-title-local \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf \
     --enable-pdf-print-intercept
   ```

   Required:
   - `printingEnabled=true`;
   - print is clicked;
   - `pdf-print-bridge.log` is written;
   - `pdf-print.log` receives fresh `pdf-print-intercept` lines;
   - bridge classification is `print-reaches-contained-intercept`;
   - no native print dialog opens;
   - no real printer job is submitted;
   - title/save/local/embedded-title regressions still pass.

9. Run focused regressions.

   Re-run:
   - PDF wheel scroll regression;
   - PDF toolbar event regression.

10. Required manual production print smoke test.

    Because automation must not click production print, this experiment requires
    a manual smoke note after automated verification:
    - run debug Wezboard/Roamium;
    - open the Bitcoin PDF;
    - click the PDF print control;
    - confirm a native print UI appears;
    - cancel it;
    - confirm the pane remains usable.

    This manual smoke is required for Pass because the experiment explicitly
    enables a production print control. If it is skipped, record the experiment
    as Partial. If it fails, record Partial or Fail based on severity and design
    the next experiment around native print UI behavior.

11. Archive Chromium patches only after a coherent branch result.

    If the experiment passes or produces a coherent partial branch, commit the
    Chromium branch and regenerate:

    ```bash
    cd chromium/src
    rm -rf ../../chromium/patches/issue-794-exp19/
    git format-patch 148.0.7778.97..HEAD \
      -o ../../chromium/patches/issue-794-exp19/
    ```

12. Formatting and review.

    If Markdown changes, run Prettier. If Rust changes, run `cargo fmt` and
    accept its output. If Chromium C++ changes, run Chromium formatting on the
    modified files if practical.

    Codex must review the completed output before Experiment 20 is designed.

## Verification

| Check                                      | Required result                          |
| ------------------------------------------ | ---------------------------------------- |
| Codex design review completed              | Yes                                      |
| Chromium branch exists and is recorded     | Yes                                      |
| Default `printingEnabled`                  | `true`                                   |
| Default print control                      | Present and enabled                      |
| Default print click                        | Not clicked                              |
| Default print status                       | `print-production-available-not-clicked` |
| Default run writes no print logs           | Yes                                      |
| Contained print status                     | `print-contained-callback`               |
| Contained bridge classification            | `print-reaches-contained-intercept`      |
| Native print dialog avoided in automation  | Yes                                      |
| Real printer job avoided in automation     | Yes                                      |
| Title propagation regression               | Pass                                     |
| Save/download regression                   | Pass                                     |
| Local PDF parity regression                | Pass                                     |
| Embedded PDF title regression              | Pass                                     |
| PDF wheel scroll regression                | Pass                                     |
| PDF toolbar event regression               | Pass                                     |
| Manual production print click/cancel smoke | Pass                                     |
| Codex completion review completed          | Yes                                      |

## Pass Criteria

This experiment passes if the production PDF viewer exposes the normal print
control by default, the automated default probe verifies that control without
clicking it, the contained intercept mode still proves the click path without
native print UI, a manual debug-app click opens and cancels native print UI
cleanly, and existing PDF regressions pass.

## Partial Criteria

This experiment is partial if:

- all automated checks pass but the manual production print smoke is skipped;
- the print control becomes visible but the contained intercept regresses;
- the default probe cannot distinguish "production print available but not
  clicked" from a failed print path;
- automated checks pass but the required manual smoke shows native print UI is
  unusable from Roamium.

Record the first failing layer and design the next experiment around that layer.

## Failure Criteria

This experiment fails if:

- automation clicks production print without the contained intercept;
- automation opens a native print dialog;
- automation submits a real print job;
- the contained intercept no longer fails closed;
- print visibility regresses title propagation, save/download, PDF rendering,
  wheel scrolling, local PDF parity, embedded PDF title safety, or toolbar
  controls;
- it adds Chrome print-preview infrastructure, PDF generation, or new protocol
  messages;
- it omits Codex design or completion review.

## Result

**Result:** Partial

Experiment 19 completed the automated production-print visibility work, but it
cannot be marked Pass until the required manual production print click/cancel
smoke is run in the debug app.

Implemented pieces:

- created Chromium branch `148.0.7778.97-issue-794-exp19`;
- changed TermSurf's PDF load-time data providers so `printingEnabled` is no
  longer gated on `TERMSURF_PDF_PRINT_INTERCEPT`;
- kept `termsurfPdfPrintBridgeTrace` separate from production print visibility;
- kept Experiment 18's contained renderer print intercept and fail-closed guard
  behavior;
- updated `scripts/probe-pdf-save-print-title-local.mjs` so default production
  print availability returns `print-production-available-not-clicked` and does
  not click print unless a contained intercept file is configured.

Codex design review:

- Initial review: `logs/codex-review/20260530-143834-600146-last-message.md`.
- Follow-up review after requiring manual production print smoke and documenting
  permission semantics:
  `logs/codex-review/20260530-144036-912329-last-message.md`.
- Result: no blocking findings remained; Codex said the design was ready for
  implementation.

Build evidence:

- `autoninja -C out/Default libtermsurf_chromium` passed.
- `./scripts/build.sh roamium` passed.
- `node --check scripts/probe-pdf-save-print-title-local.mjs` passed.
- Prettier passed on the edited issue/probe files.

Verification evidence:

- Default production visibility run:
  `logs/issue-794-exp19-print-default-20260530-144237/`
  - `printingEnabled=true`;
  - print control was found;
  - print was not clicked;
  - print status was `print-production-available-not-clicked`;
  - no `pdf-print.log` was created;
  - no `pdf-print-bridge.log` was created;
  - title propagation passed;
  - save/download created `bitcoin.pdf`;
  - HTTP, `file://`, extensionless, and untitled local PDF parity rendered;
  - embedded PDF host-title safety passed.
- Contained print run: `logs/issue-794-exp19-print-contained-20260530-144426/`
  - `printingEnabled=true`;
  - print was clicked;
  - bridge classification was `print-reaches-contained-intercept`;
  - four fresh `pdf-print-intercept` lines were written;
  - no native print dialog was observed;
  - no real printer job was submitted;
  - title propagation passed;
  - save/download created `bitcoin.pdf`;
  - HTTP, `file://`, extensionless, and untitled local PDF parity rendered;
  - embedded PDF host-title safety passed.
- Focused PDF wheel scroll regression:
  `logs/issue-794-exp19-regression-scroll-20260530-144237/`
  - `first_failing_hop=no-failure-observed`;
  - six scroll events were sent;
  - before/after screenshot and state both changed.
- Focused PDF toolbar event regression:
  `logs/issue-794-exp19-regression-toolbar-events-20260530-144826/`
  - `toolbar-events-summary.json` reported `status=pass`.

Permission-semantics note:

- The current TermSurf load-time data provider does not see document-level PDF
  content restrictions before setting `printingEnabled`.
- `PdfViewWebPlugin::Print()` still checks PDFium print permissions before the
  native print path, so a print-restricted PDF should not become actually
  printable from the plugin path.
- The UI-level `printingEnabled` value is therefore correct for normal
  unrestricted PDFs, but a future experiment should add or verify a restricted
  PDF fixture if Issue 794 requires UI hiding for print-restricted documents.

Manual production print smoke:

- Run by the user in debug Wezboard/Roamium after the automated pass.
- Result: failed.
- Observed behavior: clicking the PDF print button did not show native print UI.
  Instead, the PDF's main page viewport turned flat gray while the toolbar and
  thumbnail rail remained visible.
- Screenshot evidence: `screenshot5.png`.
- No printer job was intentionally submitted.
- Because Experiment 19 explicitly requires native print UI click/cancel
  verification before Pass, the result remains Partial.

Chromium branch archive:

- Chromium commit: `00c280eb3bda4 Enable PDF print control`.
- Patch archive: `chromium/patches/issue-794-exp19/`, ending in
  `0060-Enable-PDF-print-control.patch`.

## Conclusion

The production PDF print control is now visible by default for normal PDFs, and
automation can prove that without invoking native print UI. The contained
intercept path still proves that a click reaches `PdfViewWebPlugin::Print()`
without opening a native dialog or submitting a printer job.

This resolves the automated print-control visibility gap but proves production
print is not complete. Manual testing showed that clicking the visible PDF print
button does not present native print UI; the PDF viewport turns gray instead.

The next experiment should target native print invocation after
`PdfViewWebPlugin::Print()`: trace `OnInvokePrintDialog()`,
`PdfViewWebPluginClient::Print()`, and `PrintRenderFrameHelper::PrintNode()`,
then determine whether Roamium lacks the required browser-side printing host or
whether the print helper enters print mode but cannot surface UI from the
TermSurf embedder.

Separately, if print-restricted PDF UI accuracy matters, add a restricted-PDF
fixture and verify the UI does not advertise print for restricted documents.
