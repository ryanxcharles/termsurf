# Experiment 7: Trace PDF Drag Selection Inside PDFium

## Description

Experiment 6 proved that PDF selection itself works: after protocol focus and
protocol `Cmd+A` / `Cmd+C`, Chromium routes keys to the focused internal PDF
plugin widget and the clipboard receives the Bitcoin PDF text. That rules out
missing selected-text propagation, copy plumbing, and basic PDF plugin focus.

The remaining failure is narrower: protocol mouse drag reaches Chromium's input
router and the PDF frame tree, but dragging across the visible PDF does not
produce a PDFium text selection.

Experiment 7 should instrument the renderer-side PDF pointer path and run a
small automated drag sweep. The goal is to answer exactly where the drag stops:

- Does `PdfViewWebPlugin::HandleWebInputEvent()` receive mouse down/move/up?
- What coordinates does it pass to `PDFiumEngine` after the plugin transform?
- Does `PDFiumEngine::OnLeftMouseDown()` see `PDFiumPage::TEXT_AREA`?
- Does `OnMouseMove()` extend a selection while the left button is down?
- Does `OnSelectionTextChanged()` fire with non-empty text?

If the trace shows TermSurf events reach PDFium text areas but selection still
does not change, the next fix belongs in the PDFium mouse-selection path. If the
trace shows the automated drag never touches text, update the harness target
points and rerun before drawing product conclusions.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a fresh Chromium branch for Issue 794 Experiment 7.

   Fork from the passing Experiment 6 branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp6
   git checkout -b 148.0.7778.97-issue-794-exp7
   ```

   Add the branch to `chromium/README.md`.

2. Add renderer-side PDF plugin mouse traces.

   In `pdf/pdf_view_web_plugin.cc`, add `[issue-794-exp7]` logs gated by
   `TERMSURF_PDF_INPUT_TRACE` inside `PdfViewWebPlugin::HandleWebInputEvent()`
   for mouse down/move/up. Each line should record:
   - original event type;
   - original position in widget;
   - transformed position passed to `engine_->HandleInputEvent()`;
   - button, click count, and modifiers;
   - `available_area_`;
   - `viewport_to_dip_scale_`, `device_scale_`, and computed
     `viewport_to_device_scale`;
   - whether `engine_->HandleInputEvent()` returned true.

   Do not change behavior in this step.

3. Add PDFium selection traces.

   In `pdf/pdfium/pdfium_engine.cc`, add `[issue-794-exp7]` logs gated by
   `TERMSURF_PDF_INPUT_TRACE` around:
   - `PDFiumEngine::OnLeftMouseDown()`;
   - `PDFiumEngine::OnMouseMove()`;
   - `PDFiumEngine::OnMouseUp()`;
   - `PDFiumEngine::OnSelectionTextChanged()`.

   The mouse logs should record:
   - event type;
   - `PositionInWidget()`;
   - left-button-down state before/after if available;
   - `selecting_` before and after the handler;
   - handler return value;
   - `point_data.page_index`;
   - `point_data.char_index`;
   - `point_data.area` as both numeric value and a coarse label (`text`, `link`,
     `form`, `page-non-text`, `outside-page`, or `unknown`);
   - `point_data.pdf_point`;
   - current selection text length after the handler.

   `OnMouseMove()` must additionally record whether it returned before selection
   because `selecting_` was false, whether it reached
   `ExtendSelection(point_data)`, and the return value from `ExtendSelection()`.
   These fields are load-bearing: without them, the result cannot distinguish
   "mousedown never entered selection mode" from "move hit non-text" from
   "ExtendSelection failed."

   The selection-change log should record the selected text length and a short
   escaped sample. Keep the sample small (for example, 80 characters) to avoid
   huge logs.

4. Extend the protocol mouse harness with a PDF drag sweep.

   Extend `scripts/test-issue-794-protocol-mouse.py` with an action such as:

   ```text
   --action pdf-drag-sweep
   ```

   The sweep should:
   - require a PDF URL / `--serve-bitcoin-pdf`;
   - derive the visible `EMBED#plugin` bounds from the DevTools probe;
   - send `FocusChanged(true)`;
   - run several horizontal drags across likely text rows inside the plugin
     bounds, not just one diagonal drag;
   - include the existing left-button-down move modifier (`64`) on every move;
   - clear or baseline the clipboard before each drag attempt;
   - after each drag, send protocol `Cmd+C` and capture selected text and
     clipboard if available;
   - count clipboard text as evidence only when that attempt's after-copy
     clipboard hash differs from that attempt's baseline hash;
   - stop early if selection/copy succeeds, but still record which drag path
     succeeded.

   Suggested initial sweep rows, expressed as fractions of plugin height:

   ```text
   y = 0.16, 0.22, 0.28, 0.34, 0.40
   x = 0.12 -> 0.88
   ```

   The exact fractions may be adjusted if the trace shows they are not hitting
   PDF text on the Bitcoin fixture. Record the final fractions in the result.

5. Add analyzer fields.

   The sweep summary should include:
   - all Experiment 5 mouse routing fields;
   - focus sent / focus logged fields from Experiment 6;
   - number of drag attempts;
   - exact drag points for every attempt;
   - clipboard baseline and after-copy hashes for every attempt;
   - whether any attempt selected text;
   - selected text and clipboard lengths;
   - whether `PdfViewWebPlugin::HandleWebInputEvent()` logged mouse events;
   - whether `PDFiumEngine::OnLeftMouseDown()` logged `TEXT_AREA`;
   - whether `OnLeftMouseDown()` set `selecting_` true;
   - whether `PDFiumEngine::OnMouseMove()` logged selection extension;
   - whether `ExtendSelection()` was reached and whether it returned true;
   - whether `OnSelectionTextChanged()` logged non-empty text;
   - first failing layer.

   Suggested first failing layers:
   - `protocol-focus-not-sent`
   - `protocol-drag-not-sent`
   - `chromium-router-missing`
   - `pdf-plugin-input-missing`
   - `pdfium-mousedown-missing`
   - `pdfium-not-text-area`
   - `pdfium-move-missing`
   - `pdfium-selection-not-changing`
   - `selection-not-observable`
   - `no-failure-observed`

6. Build and archive correctly.

   Build Chromium with:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Never run `ninja` directly.

   Build Roamium if the harness or Roamium traces change:

   ```bash
   cd "$HOME/dev/termsurf"
   PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" ./scripts/build.sh roamium
   ```

   If the experiment produces a coherent Chromium branch, commit it on
   `148.0.7778.97-issue-794-exp7`, regenerate the patch archive under
   `chromium/patches/issue-794-exp7/`, and update `chromium/README.md`.

7. Run formatters and checks.
   - Run Chromium formatting on modified C++ files.
   - Run syntax checks for new or modified Python/JavaScript/shell scripts.
   - Run `prettier` on this experiment file and the issue README.
   - Run `cargo fmt` if any Rust files change.

## Verification

1. Build Chromium:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

2. Run PDF drag sweep:

   ```bash
   LOG_DIR="logs/issue-794-exp7-pdf-drag-sweep-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-mouse.py \
     --serve-bitcoin-pdf \
     --action pdf-drag-sweep \
     --log-dir "$LOG_DIR"
   ```

3. Run the Experiment 6 PDF key-selection control.

   Required: `first_failing_hop = no-failure-observed`,
   `chromium_key_target_classification = pdf-plugin`,
   `clipboard_before_text_length = 0` or an explicitly recorded non-empty
   baseline, and `clipboard_after_sha256 != clipboard_before_sha256` after
   protocol `Cmd+A` / `Cmd+C`.

4. Run the Experiment 4 PDF wheel regression.

   Required: `first_failing_hop = no-failure-observed`.

5. Interpret the drag sweep:
   - If one sweep path selects text, record the successful path and update the
     normal drag harness or app-level reproduction instructions to use a real
     text row. Then rerun the single-drag PDF test with those points.
   - If no sweep path selects text, but `OnLeftMouseDown()` never reports
     `TEXT_AREA`, the next experiment should fix coordinate mapping or page/text
     hit testing.
   - If `OnLeftMouseDown()` reports `TEXT_AREA` and `OnMouseMove()` runs with
     the left button down, but `OnSelectionTextChanged()` remains empty, the
     next experiment should fix PDFium selection state or the event sequence
     sent during drag.
   - If `OnSelectionTextChanged()` reports non-empty text but the harness cannot
     observe/copy it, the next experiment should fix selection observability for
     mouse-created selections.

6. Record the result in this file.

   The result must include:
   - Chromium branch and commit hash;
   - build command and result;
   - exact log directories for PDF drag sweep, PDF key-selection control, and
     PDF wheel regression;
   - successful or failed sweep paths;
   - PDF plugin and PDFium trace summary;
   - selected text and clipboard lengths;
   - next experiment target.

7. Codex must review the completed output.

   Do not proceed to Experiment 8 until real issues from Codex's review are
   addressed.

## Pass Criteria

Experiment 7 passes if it either:

- identifies and records a drag path that selects/copies PDF text through
  TermSurf protocol mouse input while preserving keyboard selection and wheel
  scrolling; or
- produces renderer/PDFium trace evidence that identifies the exact remaining
  layer blocking mouse drag selection.

## Partial Criteria

Experiment 7 is partial if:

- Chromium builds and traces are present, but the sweep harness cannot reliably
  run multiple drag attempts;
- PDF plugin logs are present but PDFium logs are missing due to a gating or
  process-inheritance issue;
- selected text appears visually but cannot be observed or copied by the
  harness.

## Failure Criteria

Experiment 7 fails if:

- it changes PDF behavior without first recording renderer/PDFium trace
  evidence;
- it uses CDP input as a substitute for TermSurf protocol mouse input;
- it regresses Experiment 4 PDF wheel scrolling;
- it regresses Experiment 6 PDF key selection/copy;
- it changes PDF loading, stream, extension, or resource plumbing;
- it modifies Chromium without a fresh Issue 794 Experiment 7 branch;
- it omits Codex design or completion review.

## Result

**Result:** Pass

Chromium branch: `148.0.7778.97-issue-794-exp7`

Chromium commit: `560a9e889f905` (`Trace PDF drag selection`)

Patch archive: `chromium/patches/issue-794-exp7/`

Build:

```bash
cd chromium/src
export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium
```

Result: build succeeded.

Verification logs:

- Initial PDF drag sweep with margin-starting paths:
  `logs/issue-794-exp7-pdf-drag-sweep-20260530-092403`
- PDF drag sweep: `logs/issue-794-exp7-pdf-drag-sweep-20260530-092530`
- PDF single-drag regression:
  `logs/issue-794-exp7-pdf-single-drag-20260530-092741`
- PDF key-selection control:
  `logs/issue-794-exp7-pdf-key-control-20260530-092600`
- PDF wheel regression: `logs/issue-794-exp7-wheel-regression-20260530-092632`
- HTML drag regression:
  `logs/issue-794-exp7-html-drag-regression-20260530-092816`

What happened:

- The first sweep run, `logs/issue-794-exp7-pdf-drag-sweep-20260530-092403`,
  proved that protocol mouse input reached the PDF plugin and PDFium, but every
  mousedown started in non-text margin. Later mouse moves crossed PDF text, but
  `selecting_` was already false, so `ExtendSelection()` was never reached.
- The harness sweep was adjusted to start drags inside the Bitcoin PDF text
  column. The successful geometry was: `fraction_x1 = 0.32`,
  `fraction_x2 = 0.72`, `fraction_y = 0.28`.
- With that path, the drag sweep reported
  `first_failing_hop = no-failure-observed`, `drag_sweep_selected = true`,
  `pdf_plugin_input_line = true`, `pdfium_mousedown_text_area_line = true`,
  `pdfium_mousedown_selecting_true_line = true`,
  `pdfium_mousemove_extend_line = true`, `pdfium_extend_reached_line = true`,
  `pdfium_extend_return_true_line = true`, and
  `pdfium_selection_nonempty_line = true`.
- Clipboard proof from the successful sweep attempt copied 62 characters from
  the PDF, with a sample beginning:
  `be sent directly from one party to another without going th`.
- The normal single-drag PDF harness was updated to use the successful text-row
  geometry. It then reported `first_failing_hop = no-failure-observed` and
  `pdfium_selection_nonempty_line = true`.
- The Experiment 6 PDF key-selection control still passed:
  `first_failing_hop = no-failure-observed`,
  `chromium_key_target_classification = pdf-plugin`,
  `clipboard_before_text_length = 0`, and the clipboard hash changed after
  protocol `Cmd+A` / `Cmd+C`.
- The Experiment 4 PDF wheel regression still passed with
  `first_failing_hop = no-failure-observed`.
- The HTML drag regression still passed with
  `first_failing_hop = no-failure-observed` and `selected_text_length = 104`.

## Conclusion

PDF mouse drag selection was not blocked in Chromium, PDFium, focus, selected
text propagation, or copy plumbing after Experiments 4-6. The failed automated
drag path was starting in the left PDF margin. PDFium correctly refused to enter
selection mode on that mousedown; later moves over text could not extend a
selection because `selecting_` was false.

Starting the drag on actual PDF text makes PDFium enter selection mode, extend
selection during mouse moves, emit non-empty selected text, and allow the
selection to be copied through TermSurf protocol keyboard input.

The remaining Issue 794 work should move away from mouse-selection routing and
target the other completeness requirements: resize/reflow behavior, toolbar
controls, save/print paths, titles, local-file parity, and any remaining
viewer-state gaps.
