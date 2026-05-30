# Experiment 18: Probe the PDF Print Intercept Guard

## Description

Experiment 17 proved that the PDF viewer print message reaches the internal
plugin bridge:

1. `pdf_viewer.ts::onPrint_()` runs.
2. `PluginController.print()` runs.
3. `PluginController.postMessage_({type: "print"})` runs.
4. `PdfViewWebPlugin::OnMessage()` receives `type=print`.
5. `PdfViewWebPlugin::HandlePrintMessage()` runs.

The remaining unproven boundary is the first line of `PdfViewWebPlugin::Print()`
and the contained print-intercept predicate inside that function. Experiment 16
added a guarded intercept at the top of `PdfViewWebPlugin::Print()`, but
Experiment 17 did not log entry to `Print()` itself. Therefore the current
evidence says "print reaches `HandlePrintMessage()` but no intercept line
appears," not yet "the intercept predicate is false inside `Print()`."

This experiment should answer one question:

> When `HandlePrintMessage()` calls `Print()`, what exact guard state does
> `PdfViewWebPlugin::Print()` see?

This is still contained-print work only. Do not implement production print,
print preview, native dialogs, printer jobs, or new TermSurf protocol messages.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a new Chromium branch.

   Fork the current Issue 794 Chromium branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp17
   git checkout -b 148.0.7778.97-issue-794-exp18
   ```

   Add the branch to `chromium/README.md` with a description such as "Probe PDF
   print intercept guard."

2. Add a native print-guard trace record at `PdfViewWebPlugin::Print()`.

   Reuse the existing bridge trace file when `TERMSURF_PDF_PRINT_BRIDGE_TRACE=1`
   and `TERMSURF_PDF_PRINT_BRIDGE_TRACE_FILE=/path/to/pdf-print-bridge.log` are
   active. Do not create a new log file unless the implementation proves the
   bridge trace helper cannot be reused cleanly.

   At the first executable line of `PdfViewWebPlugin::Print()`, append one
   structured trace record such as:

   ```text
   pdf-print-guard pid=... source=pdf_view_web_plugin.cc event=print-enter \
     has_intercept_switch=... has_intercept_file_switch=... \
     intercept_switch_file_empty=... env_intercept=... \
     env_intercept_file_present=... intercept_path_present=... \
     bridge_trace_path_present=... has_engine=... can_print=... url=...
   ```

   The record must avoid arbitrary PDF payloads. URL, process id, booleans, and
   file-path presence/emptiness are enough. For path debugging, do not dump
   arbitrary absolute paths. Instead require non-sensitive comparisons:
   - `intercept_file_basename=pdf-print.log` when an intercept path is present;
   - `bridge_file_basename=pdf-print-bridge.log` when a bridge trace path is
     present;
   - `intercept_dir_matches_bridge_dir=1` when both paths are present and their
     parent directories match.

   The summary should classify `intercept-path-mismatch` if an intercept path is
   present but its basename is not `pdf-print.log` or its directory does not
   match the bridge trace directory during the contained run.

3. Make the intercept helper explain its decision.

   Refactor the existing `GetTermSurfPdfPrintInterceptPath()` logic in
   `pdf/pdf_view_web_plugin.cc` into a small local diagnostic struct or helper
   that can report:
   - whether the renderer command line contains
     `--termsurf-pdf-print-intercept`;
   - whether the renderer command line contains
     `--termsurf-pdf-print-intercept-file`;
   - whether the command-line file value is empty;
   - whether environment fallback has `TERMSURF_PDF_PRINT_INTERCEPT=1`;
   - whether environment fallback has a non-empty
     `TERMSURF_PDF_PRINT_INTERCEPT_FILE`;
   - the final optional intercept path.

   Keep the behavior equivalent unless this experiment's trace proves a simple
   bug in the existing predicate. In particular, the intercept must still be
   active only when both the explicit flag and a non-empty file path are present
   through either renderer command-line switches or environment fallback.

4. If the trace proves a simple guard mismatch, fix it in the same experiment.

   Allowed fixes:
   - `TsBrowserClient::AppendExtraCommandLineSwitches()` is not forwarding the
     intercept switches to the renderer process that runs the PDF plugin;
   - the renderer-side predicate checks a different switch name or file-switch
     name than the browser-side forwarding code appends;
   - the command-line helper treats a present switch with a valid file value as
     missing;
   - `base::AppendToFile()` fails and the code currently hides the write error.

   For an append failure, keep the fail-closed behavior: return before any
   native print call when intercept mode is active, but write an explicit trace
   record identifying the append error.

   The contained verification run must also fail closed when the bridge/guard
   probe mode is active but the print intercept state is missing or invalid. In
   other words: if `PdfViewWebPlugin::Print()` sees the bridge trace guard for
   this experiment and can write a `pdf-print-guard event=print-enter` record,
   then a missing, empty, or mismatched intercept path must be recorded as the
   result and `Print()` must return before any native print path. This is a
   diagnostic safety rule, not production print behavior. The default run still
   has no bridge trace and must keep the existing production/default behavior.

   Do not use this experiment to add Chrome print preview, a native print
   dialog, a custom PDF-generation flow, or a new protocol message. If the trace
   shows that `PdfViewWebPlugin::Print()` is not entered at all despite
   `HandlePrintMessage()` logging, record that contradiction and design the next
   experiment around call-path verification.

5. Keep the client-layer print intercept in place but do not make it the primary
   target.

   `components/pdf/renderer/pdf_view_web_plugin_client.cc` still has the
   Experiment 16 intercept near `PdfViewWebPluginClient::Print()`. Leave it in
   place as a downstream safety net. The expected contained callback for this
   experiment is the earlier `pdf/pdf_view_web_plugin.cc` intercept, because
   Experiment 17 proved `HandlePrintMessage()` reaches that class.

6. Extend the probe summary with guard-state evidence.

   Update `scripts/probe-pdf-save-print-title-local.mjs` only if needed to
   extract the new `pdf-print-guard` records from `pdf-print-bridge.log`.

   The contained print summary should include:
   - whether a `print-enter` guard record exists;
   - whether `intercept_path_present=1` at `print-enter`;
   - whether a fresh `pdf-print.log` intercept line exists;
   - if no intercept line exists, the first guard-state reason, for example:
     - `print-not-entered`;
     - `intercept-switch-missing`;
     - `intercept-file-switch-missing`;
     - `intercept-file-empty`;
     - `env-intercept-missing`;
     - `env-intercept-file-missing`;
     - `intercept-path-mismatch`;
     - `append-failed`;
     - `intercept-present-but-log-missing`.

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
   LOG_DIR="logs/issue-794-exp18-print-default-$(date +%Y%m%d-%H%M%S)" \
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
   - title/save/local/embedded-title regressions still pass.

9. Verify contained print guard state.

   Run:

   ```bash
   LOG_DIR="logs/issue-794-exp18-print-guard-$(date +%Y%m%d-%H%M%S)" \
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
   - the bridge trace still shows `HandlePrintMessage()` for `print`;
   - a `pdf-print-guard event=print-enter` record exists, or the result is
     classified as `print-not-entered`;
   - the summary records the first guard-state reason;
   - if `intercept_path_present=1` and the path matches the expected contained
     log location, the contained intercept writes `pdf-print.log` or the trace
     records an append failure;
   - if the intercept path is missing, empty, or mismatched while bridge/guard
     trace mode is active, `Print()` logs the guard reason and returns before
     native print;
   - no native print dialog opens;
   - no real printer job is submitted;
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
    rm -rf ../../chromium/patches/issue-794-exp18/
    git format-patch 148.0.7778.97..HEAD \
      -o ../../chromium/patches/issue-794-exp18/
    ```

12. Formatting and review.

    If Markdown changes, run Prettier. If Rust changes, run `cargo fmt` and
    accept its output. If Chromium C++ changes, run Chromium formatting on the
    modified files if practical.

    Codex must review the completed output before Experiment 19 is designed.

## Verification

| Check                                  | Required result              |
| -------------------------------------- | ---------------------------- |
| Codex design review completed          | Yes                          |
| Chromium branch exists and is recorded | Yes                          |
| Default print disabled                 | Yes                          |
| Default run writes no print logs       | Yes                          |
| Contained run writes bridge trace      | Yes                          |
| `HandlePrintMessage()` still observed  | Yes                          |
| `Print()` entry guard state classified | One named reason from step 6 |
| Native print dialog avoided            | Yes                          |
| Real printer job avoided               | Yes                          |
| Title propagation regression           | Pass                         |
| Save/download regression               | Pass                         |
| Local PDF parity regression            | Pass                         |
| Embedded PDF title regression          | Pass                         |
| PDF wheel scroll regression            | Pass                         |
| PDF toolbar event regression           | Pass                         |
| Codex completion review completed      | Yes                          |

## Pass Criteria

This experiment passes if it either:

- fixes the contained print intercept so the guarded print click writes a fresh
  `pdf-print.log` line from `PdfViewWebPlugin::Print()` without opening native
  print UI; or
- proves the exact first guard-state failure at `PdfViewWebPlugin::Print()` and
  records enough evidence to make the next experiment mechanical.

Default print must remain disabled, and the existing title/save/local/scroll and
toolbar regressions must pass.

## Partial Criteria

This experiment is partial if it narrows the print failure but cannot classify
the first guard-state reason because required trace records are missing,
contradictory, or ambiguous.

Record the strongest proven boundary and design the next experiment around the
first unproven layer.

## Failure Criteria

This experiment fails if:

- it changes production print behavior;
- it enables print without the explicit intercept guard;
- it opens a native print dialog;
- it submits a real print job;
- it hides an intercept-file append failure and falls through to native print;
- it adds Chrome print-preview infrastructure, PDF generation, or new protocol
  messages;
- it regresses title propagation, save/download, PDF rendering, wheel scrolling,
  or toolbar controls;
- it omits Codex design or completion review.

## Result

**Result:** Pass

Experiment 18 proved the first line of `PdfViewWebPlugin::Print()` and fixed the
remaining contained-print harness failure.

Implemented pieces:

- created Chromium branch `148.0.7778.97-issue-794-exp18`;
- added `pdf-print-guard` trace records at `PdfViewWebPlugin::Print()` entry;
- refactored the PDF print intercept predicate in `pdf/pdf_view_web_plugin.cc`
  into a diagnostic state helper;
- recorded command-line switch presence, file-switch presence, environment
  fallback state, path presence, basename comparisons, bridge-directory match,
  engine presence, and print permission state;
- made bridge/guard probe mode fail closed if the intercept path is missing,
  empty, or mismatched;
- recorded append failures explicitly instead of silently falling through;
- extended `scripts/probe-pdf-save-print-title-local.mjs` to classify guard
  outcomes and include guard state in the summary;
- fixed `scripts/test-issue-794-pdf-toolbar.py` so the contained print run
  creates/truncates `pdf-print.log` before activation, matching the existing
  bridge-log setup.

Codex design review:

- Initial review: `logs/codex-review/20260530-141646-923070-last-message.md`.
- Follow-up review after fixing the fail-closed and path-mismatch findings:
  `logs/codex-review/20260530-141828-786883-last-message.md`.
- Result: no blocking findings remained; Codex said the experiment was ready for
  implementation.

Build evidence:

- `autoninja -C out/Default libtermsurf_chromium` passed.
- `./scripts/build.sh roamium` passed.
- `node --check scripts/probe-pdf-save-print-title-local.mjs` passed.
- `python3 -m py_compile scripts/test-issue-794-pdf-toolbar.py` passed.
- Prettier check passed for the edited issue and probe files.

Verification evidence:

- Default/no-intercept run:
  `logs/issue-794-exp18-print-default-rerun-20260530-143119/`
  - `printingEnabled=false`;
  - print was not clicked;
  - no `pdf-print.log` was created;
  - no `pdf-print-bridge.log` was created;
  - title propagation passed;
  - save/download created `bitcoin.pdf`;
  - embedded PDF host-title safety passed.
- First contained run: `logs/issue-794-exp18-print-guard-20260530-142514/`
  - `PdfViewWebPlugin::Print()` was entered;
  - command-line intercept switch and file switch were both present;
  - intercept path was present;
  - `intercept_file_basename=pdf-print.log`;
  - `bridge_file_basename=pdf-print-bridge.log`;
  - `intercept_dir_matches_bridge_dir=1`;
  - `intercept_path_expected=1`;
  - `has_engine=1`;
  - `can_print=1`;
  - `base::AppendToFile()` failed because the harness had not created
    `pdf-print.log` before activation;
  - no native print dialog was observed, and no real printer job was submitted.
- Contained rerun after harness fix:
  `logs/issue-794-exp18-print-guard-rerun-20260530-142905/`
  - `printingEnabled=true`;
  - print was clicked;
  - bridge classification was `print-reaches-contained-intercept`;
  - `printGuardState.intercept_path_present=1`;
  - `printGuardState.intercept_path_expected=1`;
  - four fresh `pdf-print-intercept` lines were written by
    `PdfViewWebPlugin::Print()`;
  - title propagation passed;
  - save/download created `bitcoin.pdf`;
  - HTTP, `file://`, extensionless, and untitled local PDF parity rendered;
  - embedded PDF host-title safety passed.
  - Note: the broad `save-print-title-local-summary.json` top-level `status`
    still reports `partial` from the legacy aggregate classifier. Experiment
    18's verdict is based on the print-specific fields required by this
    experiment: `print.status=print-contained-callback`,
    `bridgeClassification=print-reaches-contained-intercept`, the expected guard
    state, and fresh `pdf-print-intercept` lines.
- Focused PDF wheel scroll regression:
  `logs/issue-794-exp18-regression-scroll-20260530-143119/`
  - `first_failing_hop=no-failure-observed`;
  - six scroll events were sent;
  - before/after screenshot and state both changed.
- Focused PDF toolbar event regression:
  `logs/issue-794-exp18-regression-toolbar-events-20260530-143313/`
  - `toolbar-events-summary.json` reported `status=pass`.

Chromium branch archive:

- Chromium commit: `304dd08c23984 Probe PDF print intercept guard`.
- Patch archive: `chromium/patches/issue-794-exp18/`, ending in
  `0059-Probe-PDF-print-intercept-guard.patch`.

No native print dialog was observed, and no real printer job was submitted.

## Conclusion

The contained print activation proof now works. The earlier failure was not a
missing viewer bridge, a missing `Print()` call, a missing renderer command-line
switch, or a path mismatch. The PDF plugin did enter `PdfViewWebPlugin::Print()`
with:

- the intercept switch present;
- the intercept file switch present;
- the expected `pdf-print.log` basename;
- the same directory as `pdf-print-bridge.log`;
- `engine_` present;
- print permission available.

The actual issue was mechanical: the harness created the bridge trace file but
did not create `pdf-print.log`, and `base::AppendToFile()` did not create the
missing file. Creating/truncating `pdf-print.log` before activation makes the
contained intercept fire reliably.

Issue 794's remaining print work is no longer "can the toolbar reach print?"
That is proven. The next print experiment should decide and implement the
production print behavior:

- either keep print disabled until a real TermSurf print implementation exists;
- or add a contained user-facing behavior, such as save-to-PDF/download, without
  invoking native print UI from automation.

Before closing Issue 794, the issue still needs a final pass over the complete
viewer interaction checklist: render, scroll, resize/reflow, mouse drag
selection, keyboard selection/copy, toolbar controls, save/download, title,
local PDF parity, embedded PDF safety, and print policy.
