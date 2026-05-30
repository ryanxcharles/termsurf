# Experiment 6: Route Keyboard Selection to the Focused PDF Widget

## Description

Experiment 5 proved that TermSurf mouse events now enter Chromium's
`RenderWidgetHostInputEventRouter`, and that the PDF frame tree contains the
mouse points being sent. It did not make PDF drag selection work. The next
question is whether the PDF plugin can focus and expose selected text at all
through TermSurf's current keyboard path.

Chromium's own PDF tests separate these layers:

- Mouse/touch input is delivered after waiting for PDF hit-test data.
- Keyboard selection is verified through the PDF child frame's
  `RenderWidgetHostView::GetSelectedText()`.
- `Cmd+A`/`Ctrl+A` selection flows through the focused render widget, not
  blindly through the root widget.

TermSurf currently forwards keys directly to the root/main
`RenderWidgetHostImpl`:

```cpp
auto* rwhi = static_cast<RenderWidgetHostImpl*>(view->GetRenderWidgetHost());
rwhi->ForwardKeyboardEventWithCommands(...);
```

That bypasses Chromium's focused-frame keyboard targeting for OOPIF pages. The
equivalent Chromium pattern is
`WebContentsImpl::GetFocusedRenderWidgetHost(receiving_widget)`, which returns
the focused subframe widget when a main-frame view receives a keyboard event.

Experiment 6 should route TermSurf key events to Chromium's focused render
widget, add enough tracing to prove which widget receives `Cmd+A`/`Cmd+C`, and
verify whether PDF selected text/copy now works. If selection still fails while
keyboard routing targets the PDF child widget, the next experiment should move
inside the PDF plugin/selection implementation.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a fresh Chromium branch for Issue 794 Experiment 6.

   Fork from the passing Experiment 5 branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp5
   git checkout -b 148.0.7778.97-issue-794-exp6
   ```

   Add the branch to `chromium/README.md`.

2. Add Chromium-side key-routing traces.

   In `content/libtermsurf_chromium/ts_browser_main_parts.cc`, add
   `[issue-794-exp6]` logs around `ForwardKeyEvent()` that record:
   - event type (`down`, `up`, or `repeat`);
   - key code, UTF-8 text length, modifiers, and computed `web_modifiers`;
   - root view/root widget presence;
   - focused widget chosen by `WebContentsImpl::GetFocusedRenderWidgetHost()`;
   - the frame tree node id, URL, site URL, and classification for the focused
     widget when it maps to a `RenderFrameHost` view (`root`, `pdf-extension`,
     `pdf-plugin`, or `other-subframe`);
   - whether the route mode is `focused-widget`, `root-direct`, or `drop`;
   - frame inventory equivalent to Experiments 4 and 5, including which frame is
     currently focused if this can be obtained cleanly.

   Keep these logs gated by `TERMSURF_PDF_INPUT_TRACE`.

3. Route keys to Chromium's focused render widget.

   In `ForwardKeyEvent()`:
   - build the same `input::NativeWebKeyboardEvent` and edit commands as today;
   - keep the existing navigation command handling (`Cmd+[`, `Cmd+]`, `Cmd+R`);
   - get the receiving/root `RenderWidgetHostImpl` from the root view;
   - call `WebContentsImpl::GetFocusedRenderWidgetHost(receiving_widget)`;
   - forward key down/up/repeat and synthesized char events to the focused
     widget when non-null;
   - fall back to the receiving/root widget only if the focused-widget lookup is
     unavailable in a non-crashed state;
   - do not special-case PDF URLs, extension ids, or plugin frame ids.

   This mirrors Chromium's own OOPIF keyboard routing behavior: keyboard events
   arrive at the main-frame view, but the focused subframe widget consumes them.

4. Add gated focus and key traces.

   In `roamium/src/dispatch.rs`, add diagnostic-only `trace_pdf_input(...)`
   lines for `Msg::FocusChanged` and `Msg::KeyEvent`, gated by the existing
   `TERMSURF_PDF_INPUT_TRACE` helper.

   For `Msg::FocusChanged`, record:
   - tab id;
   - pane id when found;
   - result (`ffi=ts_set_focus` or `no-tab`);
   - focused value.

   For `Msg::KeyEvent`, record:
   - tab id;
   - pane id when found;
   - result (`ffi=ts_forward_key_event` or `no-tab`);
   - event type;
   - Windows key code;
   - UTF-8 length;
   - modifiers.

   In `ts_browser_main_parts.cc::SetFocus()`, add a matching gated
   `[issue-794-exp6] focus` log that records the WebContents URL, root view, and
   focused value before calling `view->Focus()` / `SetActive(true)`.

   This is diagnostic only. Do not add a new protocol message or FFI function.

5. Extend the protocol harness for keyboard selection.

   Extend `scripts/test-issue-794-protocol-mouse.py` or add a focused companion
   script. The harness should still use TermSurf protocol messages as the
   stimulus; DevTools may only observe state and read clipboard/selection.

   Encode key messages with explicit protobuf wire types:

   ```text
   TermSurfMessage.key_event = field 9
   KeyEvent.tab_id = field 1 (varint)
   KeyEvent.type = field 2 (length-delimited string: "down", "up", or "repeat")
   KeyEvent.windows_key_code = field 3 (varint)
   KeyEvent.utf8 = field 4 (length-delimited string)
   KeyEvent.modifiers = field 5 (varint)
   ```

   Encode focus setup before key testing:

   ```text
   TermSurfMessage.focus_changed = field 10
   FocusChanged.tab_id = field 1 (varint)
   FocusChanged.focused = field 2 (varint bool)
   ```

   For macOS command shortcuts, send `modifiers = 8`, which is TermSurf's mapped
   `blink::WebInputEvent::kMetaKey` bit:

   ```text
   Cmd+A: key down A (65, modifiers 8), key up A (65, modifiers 8)
   Cmd+C: key down C (67, modifiers 8), key up C (67, modifiers 8)
   ```

   Before any key-selection test, the harness must send `FocusChanged(true)` for
   the test tab. A click alone is not equivalent to the real Wezboard path,
   because the real GUI also informs Roamium/Chromium that the tab is focused.

   The harness should support at least:
   - click target, then `Cmd+A`, then observe selected text;
   - clear or baseline the clipboard, click target, then `Cmd+A`, then `Cmd+C`,
     then read clipboard text if the DevTools clipboard permission can be
     granted;
   - normal HTML control page verification;
   - PDF verification.

6. Add analyzer fields.

   The keyboard summary should include:
   - `server_register_received`;
   - `create_tab_sent`;
   - `tab_ready_id`;
   - `resize_sent`;
   - `focus_sent`;
   - target coordinate source and exact click point;
   - protocol mouse messages sent for focus/click setup;
   - protocol key messages sent;
   - whether Roamium logged focus receive/FFI lines;
   - whether Chromium logged focus receipt;
   - whether Roamium logged key receive/FFI lines;
   - Chromium key route mode;
   - whether the selected key target was the focused widget or root widget;
   - focused key target frame tree node id, URL, site URL, and classification
     (`root`, `pdf-extension`, `pdf-plugin`, `other-subframe`, or `unknown`);
   - selected text length after `Cmd+A`;
   - clipboard baseline length/hash/sample and after-copy length/hash/sample if
     available;
   - before/after state and screenshot change;
   - `first_failing_hop`.

   Suggested `first_failing_hop` values:
   - `roamium-not-registered`
   - `tab-not-ready`
   - `resize-not-sent`
   - `focus-not-sent`
   - `protocol-click-not-sent`
   - `protocol-key-not-sent`
   - `trace-env-not-inherited`
   - `roamium-focus-receive-missing`
   - `roamium-focus-ffi-missing`
   - `chromium-focus-missing`
   - `roamium-key-receive-missing`
   - `roamium-key-ffi-missing`
   - `chromium-key-route-missing`
   - `chromium-key-root-target`
   - `chromium-key-target-ambiguous`
   - `pdf-focus-or-selection`
   - `clipboard-unavailable`
   - `no-failure-observed`
   - `automation-gap`

7. Build and archive correctly.

   Build Chromium with:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Never run `ninja` directly.

   Build Roamium so the key traces are present:

   ```bash
   cd "$HOME/dev/termsurf"
   PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" ./scripts/build.sh roamium
   ```

   If the experiment produces a coherent Chromium branch, commit it on
   `148.0.7778.97-issue-794-exp6`, regenerate the patch archive under
   `chromium/patches/issue-794-exp6/`, and update `chromium/README.md`.

8. Run formatters and checks.
   - Run Chromium formatting on modified C++ files.
   - Run syntax checks for new or modified Python/JavaScript/shell scripts.
   - Run `prettier` on this experiment file and the issue README.
   - Run `cargo fmt` for the Roamium Rust trace changes and accept the output.

## Verification

1. Build Chromium:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

2. Build Roamium:

   ```bash
   cd "$HOME/dev/termsurf"
   PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" ./scripts/build.sh roamium
   ```

3. Run normal HTML select-all/copy:

   ```bash
   python3 -m http.server 9791 --bind 127.0.0.1 --directory test-html/public

   LOG_DIR="logs/issue-794-exp6-html-key-selection-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-mouse.py \
     "http://127.0.0.1:9791/test-interactions.html" \
     --url-contains test-interactions.html \
     --action key-select-copy \
     --log-dir "$LOG_DIR"
   ```

   Required:
   - protocol focus is sent before key input;
   - protocol click and key messages are sent;
   - Roamium logs `ffi=ts_set_focus`, `ffi=ts_forward_mouse_event`, and
     `ffi=ts_forward_key_event`;
   - Chromium logs `[issue-794-exp6] key-route route_mode=focused-widget` or a
     root target that is correct for the single-frame HTML page;
   - selected text is non-empty, or clipboard text changes from the baseline and
     matches the expected HTML selected text.

4. Run PDF select-all/copy:

   ```bash
   LOG_DIR="logs/issue-794-exp6-pdf-key-selection-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-mouse.py \
     --serve-bitcoin-pdf \
     --action key-select-copy \
     --log-dir "$LOG_DIR"
   ```

   Required interpretation:
   - If selected text or clipboard text appears, PDF keyboard selection/copy is
     fixed only if the clipboard was cleared/baselined first and the copied text
     changes from the baseline.
   - If key routing targets the root widget, the fix is incomplete and the next
     step remains focused-widget routing.
   - If key routing targets the PDF extension frame but not the internal PDF
     plugin frame, record that distinction. It may indicate a remaining focus
     hop between the extension document and plugin.
   - If key routing targets the PDF child widget but selected text is still
     empty, the next experiment should instrument the PDF plugin's
     `UpdateFocus()`, `HandleInputEvent()`, `SetSelectedText()`, and
     `HandleGetSelectedTextMessage()` paths.

5. Re-run Experiment 5 mouse regressions.

   Run:
   - HTML click from Experiment 5;
   - HTML drag from Experiment 5;
   - PDF click from Experiment 5;
   - PDF drag from Experiment 5.

   HTML click and drag must still pass. PDF click/drag must still reach
   `route_mode=input-router`; PDF drag may still report
   `pdf-focus-or-selection`.

6. Re-run Experiment 4 PDF wheel regression.

   Required: `first_failing_hop = no-failure-observed`.

7. Record the result in this file.

   The result must include:
   - Chromium branch and commit hash;
   - Chromium and Roamium build commands and results;
   - exact log directories for HTML key selection, PDF key selection, PDF drag
     regression, HTML click/drag regressions, PDF click regression, and PDF
     wheel regression;
   - route modes observed;
   - selected text and clipboard lengths;
   - clipboard baseline/change evidence for any copy pass claim;
   - whether PDF click/focus, keyboard selection/copy, and drag selection
     passed, failed, or remained inconclusive;
   - next experiment target.

8. Codex must review the completed output.

   Do not proceed to Experiment 7 until real issues from Codex's review are
   addressed.

## Pass Criteria

Experiment 6 passes if it either:

- fixes protocol-level PDF keyboard selection/copy by routing keys to Chromium's
  focused render widget while preserving HTML key selection, PDF mouse routing,
  and PDF wheel scrolling; or
- proves that TermSurf keys now target the focused PDF widget, but selected text
  still requires deeper PDF plugin/selection instrumentation.

## Partial Criteria

Experiment 6 is partial if:

- keyboard routing builds and logs route mode, but the harness cannot reliably
  verify normal HTML selection/copy;
- PDF selected text is not observable because clipboard permission or DevTools
  selection inspection is unavailable;
- the route mode is ambiguous because required Chromium logs are missing.

## Failure Criteria

Experiment 6 fails if:

- it uses CDP input as a substitute for TermSurf protocol key input;
- it changes TermSurf protocol messages or Roamium FFI;
- it special-cases PDF URLs, PDF extension ids, or plugin frame ids as the
  primary keyboard routing fix;
- it regresses Experiment 4 PDF wheel scrolling;
- it regresses Experiment 5 HTML click/drag or PDF mouse routing;
- it touches PDF stream/resource/loading code;
- it modifies Chromium without a fresh Issue 794 Experiment 6 branch;
- it omits HTML key-selection controls;
- it omits the PDF wheel regression check;
- it omits Codex design or completion review.

## Result

**Result:** Pass

Chromium branch: `148.0.7778.97-issue-794-exp6`

Chromium commit: `878fed10ace41`
(`Route TermSurf keyboard input to focused widget`)

Patch archive: `chromium/patches/issue-794-exp6/`

Builds:

- Chromium build passed:

  ```bash
  cd chromium/src
  export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
  autoninja -C out/Default libtermsurf_chromium
  ```

- Roamium build passed:

  ```bash
  cd "$HOME/dev/termsurf"
  PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" ./scripts/build.sh roamium
  ```

Verification logs:

- HTML key selection/copy:
  `logs/issue-794-exp6-html-key-selection-20260530-090419`
- PDF key selection/copy:
  `logs/issue-794-exp6-pdf-key-selection-20260530-090458`
- HTML click regression:
  `logs/issue-794-exp6-html-click-regression-20260530-090533`
- HTML drag regression:
  `logs/issue-794-exp6-html-drag-regression-20260530-090559`
- PDF click regression:
  `logs/issue-794-exp6-pdf-click-regression-20260530-090622`
- PDF drag regression: `logs/issue-794-exp6-pdf-drag-regression-20260530-090645`
- PDF wheel regression: `logs/issue-794-exp6-wheel-regression-20260530-090708`

Observed results:

- HTML key selection/copy passed. The harness sent protocol
  `FocusChanged(true)`, clicked `#click-target`, sent protocol `Cmd+A` and
  `Cmd+C`, and cleared/baselined the clipboard first. Summary result:
  `first_failing_hop = no-failure-observed`, selected text length `389`,
  clipboard length changed from `0` to `401`, and the copied text was the
  expected HTML page text. Since the page is single-frame HTML, Chromium
  correctly routed keys to the root widget
  (`chromium_key_target_classification = root`).
- PDF key selection/copy passed. The harness sent protocol focus, clicked the
  visible PDF plugin bounds, then sent protocol `Cmd+A` and `Cmd+C`. Chromium
  routed keys to the focused PDF plugin widget:
  `chromium_key_focused_widget_line = true`,
  `chromium_key_root_direct_line = false`, and
  `chromium_key_target_classification = pdf-plugin`. The clipboard was baselined
  empty, then changed to length `21230` beginning with
  `Bitcoin: A Peer-to-Peer Electronic Cash System`. The summary reported
  `first_failing_hop = no-failure-observed`.
- HTML click regression passed. Summary result:
  `first_failing_hop = no-failure-observed`, `route_mode=input-router`,
  before/after state changed, and screenshot changed.
- HTML drag regression passed. Summary result:
  `first_failing_hop = no-failure-observed`, `route_mode=input-router`, and
  selected text length `104`.
- PDF click regression passed. Summary result:
  `first_failing_hop = no-failure-observed`, target `plugin-bounds`,
  `route_mode=input-router`, and before/after state changed.
- PDF drag regression still did not produce selected text. Summary result:
  `first_failing_hop = pdf-focus-or-selection`, target `plugin-bounds`,
  `route_mode=input-router`, selected text length `0`, and no screenshot change.
  This means keyboard selection/copy is fixed, but mouse-drag selection remains
  a separate pointer/PDFium selection problem.
- PDF wheel regression passed. Summary result:
  `first_failing_hop = no-failure-observed`, six scroll events sent, Roamium
  scroll FFI logged, and the before/after PDF screenshot changed.

## Conclusion

Experiment 6 fixed protocol-level PDF keyboard selection and copy. TermSurf now
sends focus and key events through the path Chromium expects for OOPIF PDF
content: the root view receives the event, `WebContentsImpl` selects the focused
render widget, and `Cmd+A` / `Cmd+C` reach the internal PDF plugin frame.

This proves the PDF plugin can focus, select all text, expose selected text to
Chromium's copy path, and place PDF text on the clipboard. The remaining
interaction gap is specifically mouse drag selection: protocol mouse events
route to the PDF frame tree, but a drag over the plugin does not create a PDFium
text selection. The next experiment should instrument or fix the pointer-to-PDF
selection path inside the PDF plugin/PDFium engine, using Experiment 6's
keyboard-selection success as the control.
