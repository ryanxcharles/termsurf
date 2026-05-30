# Experiment 5: Route Mouse Events Through Chromium's Input Router

## Description

Experiment 4 fixed PDF wheel scrolling by routing TermSurf wheel events through
Chromium's `RenderWidgetHostInputEventRouter`. The remaining user-visible PDF
interaction failures include click/focus and drag text selection.

The mouse path still has the same architectural problem wheel had before
Experiment 4:

```cpp
view->GetRenderWidgetHost()->ForwardMouseEvent(mouse_event);
```

Both `TsBrowserMainParts::ForwardMouseEvent()` and
`TsBrowserMainParts::ForwardMouseMove()` forward directly to the root/main
`RenderWidgetHost`. That bypasses Chromium's browser-side OOPIF hit-test router.
For a PDF, the visible interactive content lives inside the PDF extension frame
and internal PDF plugin frame, not purely in the root frame.

Experiment 5 should route TermSurf mouse down/up/move events through Chromium's
input router, preserve the direct fallback, and verify behavior with direct
TermSurf protocol mouse input. This should be treated as a focused mouse-input
experiment, not a general PDF feature sweep.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a fresh Chromium branch for Issue 794 Experiment 5.

   Fork from the passing Experiment 4 branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp4
   git checkout -b 148.0.7778.97-issue-794-exp5
   ```

   Add the branch to `chromium/README.md`.

2. Add Chromium-side mouse-routing traces.

   In `content/libtermsurf_chromium/ts_browser_main_parts.cc`, add
   `[issue-794-exp5]` logs around `ForwardMouseEvent()` and `ForwardMouseMove()`
   that record:
   - event kind (`down`, `up`, `move`);
   - tab/web contents URL;
   - root view/root widget/router presence;
   - mouse coordinates, button, click count, modifiers, and computed
     `web_modifiers`;
   - route mode used: `input-router` or `direct-fallback`;
   - frame inventory equivalent to Experiment 4 for PDF-related frames, so the
     result can say whether the input point was inside the PDF extension/plugin
     root bounds.

   Keep these logs gated by `TERMSURF_PDF_INPUT_TRACE`.

3. Route mouse down/up/move through Chromium's input router.

   Replace direct-only forwarding in both functions with:
   - construct the same `blink::WebMouseEvent` as today;
   - add `ui::LatencyInfo`;
   - if `WebContentsImpl::GetInputEventRouter()` is available and the root view
     can be passed as `RenderWidgetHostViewInput`, call
     `RenderWidgetHostInputEventRouter::RouteMouseEvent(...)`;
   - otherwise fall back to `ForwardMouseEvent(...)`.

   Implementation notes:
   - Reuse the same internal API pattern as Experiment 4.
   - Preserve button/modifier semantics. In particular, drag `MouseMove` must
     carry `blink::WebInputEvent::kLeftButtonDown` while the button is held.
   - Keep coordinates in root view coordinates. The router owns target selection
     and coordinate transforms.
   - Do not special-case PDF URLs, the PDF extension id, or plugin frame ids as
     the primary fix.
   - Do not change wheel routing, keyboard routing, resize, PDF stream plumbing,
     PDF resources, Roamium dispatch behavior, Wezboard input forwarding, or
     `termsurf.proto`. The only Roamium change allowed is the gated
     diagnostic-only mouse trace from Step 5.

4. Add a protocol mouse harness.

   Add a new script, preferably:

   ```text
   scripts/test-issue-794-protocol-mouse.py
   ```

   It may reuse helpers from `scripts/test-issue-794-protocol-scroll.py` by
   extraction or local copy. The harness should launch Roamium against the fake
   GUI socket, create a tab, capture before/after DevTools probe artifacts, send
   hand-encoded protocol mouse input, and write:

   ```text
   $LOG_DIR/protocol-mouse-summary.json
   ```

   Encode:

   ```python
   def double_field(number: int, value: float) -> bytes:
       return field(number, 1) + struct.pack("<d", value)
   ```

   `MouseEvent.x`, `MouseEvent.y`, `MouseMove.x`, and `MouseMove.y` are `double`
   fields in `termsurf.proto`, so they must use protobuf fixed64 wire type `1`,
   exactly like the scroll harness uses for `ScrollEvent.x/y`.

   ```text
   TermSurfMessage.mouse_event = field 6
   MouseEvent.tab_id = field 1
   MouseEvent.type = field 2 ("down" or "up")
   MouseEvent.button = field 3 ("left")
   MouseEvent.x = field 4 (double/fixed64)
   MouseEvent.y = field 5 (double/fixed64)
   MouseEvent.click_count = field 6
   MouseEvent.modifiers = field 7

   TermSurfMessage.mouse_move = field 7
   MouseMove.tab_id = field 1
   MouseMove.x = field 2 (double/fixed64)
   MouseMove.y = field 3 (double/fixed64)
   MouseMove.modifiers = field 4
   ```

   During drag, every `MouseMove` between the `down` and `up` must set
   `MouseMove.modifiers = 64`, which is the existing TermSurf left-button-down
   bit used by Wezboard. A malformed drag without this modifier is a harness
   failure, not PDF evidence.

   The harness should support at least:
   - `--action click`
   - `--action drag`

   For normal HTML:
   - click should target `#click-target` and verify the counter/state changes;
   - drag should target `#selection-target` and verify selection text changes.

   For PDF:
   - click should target the visible `EMBED#plugin` or PDF viewer container and
     record any focus/state/screenshot change;
   - drag should derive points from `EMBED#plugin` or the visible PDF page area,
     send down/move/up with left-button modifiers, and record selection,
     clipboard, screenshot, and Chromium route logs.

   The harness may use DevTools only for observation and clipboard/selection
   inspection. It must not use CDP input as the mouse stimulus.

5. Add gated Roamium mouse traces.

   In `roamium/src/dispatch.rs`, add diagnostic-only `trace_pdf_input(...)`
   lines for `Msg::MouseEvent` and `Msg::MouseMove`, gated by the existing
   `TERMSURF_PDF_INPUT_TRACE` helper. The lines should record:
   - tab id;
   - pane id when found;
   - result (`ffi=ts_forward_mouse_event`, `ffi=ts_forward_mouse_move`, or
     `no-tab`);
   - event type/button/click count for down/up;
   - coordinates;
   - modifiers.

   This is required so the analyzer can distinguish bad protobuf encoding or
   Roamium dispatch failure from Chromium routing failure. Do not add a new FFI
   call or protocol message.

6. Add analyzer fields.

   `protocol-mouse-summary.json` should include:
   - `server_register_received`;
   - `create_tab_sent`;
   - `tab_ready_id`;
   - `resize_sent`;
   - action (`click` or `drag`);
   - target coordinate source and exact points;
   - number and types of protocol mouse messages sent;
   - whether Roamium logged mouse receive/FFI lines;
   - whether Chromium logged mouse route mode;
   - whether any route mode was `input-router`;
   - whether before/after screenshot changed;
   - whether before/after state changed;
   - selected text length and clipboard text length where available;
   - `first_failing_hop`.

   Suggested `first_failing_hop` values:
   - `roamium-not-registered`
   - `tab-not-ready`
   - `resize-not-sent`
   - `protocol-mouse-not-sent`
   - `trace-env-not-inherited`
   - `roamium-receive-missing`
   - `roamium-ffi-missing`
   - `chromium-route-missing`
   - `chromium-route-direct-fallback`
   - `pdf-focus-or-selection`
   - `no-failure-observed`
   - `automation-gap`

7. Build and archive correctly.

   Build with:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Never run `ninja` directly.

   If the experiment produces a coherent Chromium branch, commit it on
   `148.0.7778.97-issue-794-exp5`, regenerate the patch archive under
   `chromium/patches/issue-794-exp5/`, and update `chromium/README.md`.

8. Run formatters and checks.
   - Run Chromium formatting on modified C++ files.
   - Run syntax checks for new Python/JavaScript/shell scripts.
   - Run `prettier` on this experiment file and the issue README.
   - Run `cargo fmt` for the Roamium Rust trace changes and accept the output.

## Verification

1. Build Chromium:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

2. Build Roamium so the gated mouse traces are present in the test binary:

   ```bash
   cd "$HOME/dev/termsurf"
   ./scripts/build.sh roamium
   ```

3. Run normal HTML click:

   ```bash
   python3 -m http.server 9791 --bind 127.0.0.1 --directory test-html/public

   LOG_DIR="logs/issue-794-exp5-html-click-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-mouse.py \
     "http://127.0.0.1:9791/test-interactions.html" \
     --url-contains test-interactions.html \
     --action click \
     --log-dir "$LOG_DIR"
   ```

   Required: HTML click state changes and Chromium logs show
   `route_mode=input-router`.

4. Run normal HTML drag:

   ```bash
   LOG_DIR="logs/issue-794-exp5-html-drag-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-mouse.py \
     "http://127.0.0.1:9791/test-interactions.html" \
     --url-contains test-interactions.html \
     --action drag \
     --log-dir "$LOG_DIR"
   ```

   Required: HTML selection state or clipboard text changes and Chromium logs
   show `route_mode=input-router`.

5. Run PDF click:

   ```bash
   LOG_DIR="logs/issue-794-exp5-pdf-click-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-mouse.py \
     --serve-bitcoin-pdf \
     --action click \
     --log-dir "$LOG_DIR"
   ```

   Required: the route mode reaches `input-router`; focus/state may remain
   inconclusive if Chromium does not expose plugin focus through the current
   probe.

6. Run PDF drag:

   ```bash
   LOG_DIR="logs/issue-794-exp5-pdf-drag-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-mouse.py \
     --serve-bitcoin-pdf \
     --action drag \
     --log-dir "$LOG_DIR"
   ```

   Pass/fail interpretation:
   - If selected text or clipboard text appears, PDF drag selection is fixed.
   - If routing is `input-router` but selection is still empty, the next
     experiment should target PDF plugin focus/selection APIs, not TermSurf
     mouse routing.
   - If HTML drag fails, stop and fix the harness or mouse routing before
     interpreting PDF drag.

7. Regression check wheel after mouse changes:

   Re-run the Experiment 4 PDF scroll command and confirm
   `first_failing_hop = no-failure-observed`.

8. Record the result in this file.

   The result must include:
   - Chromium branch and commit hash;
   - Chromium and Roamium build commands and results;
   - exact log directories for HTML click, HTML drag, PDF click, PDF drag, and
     wheel regression;
   - route modes observed;
   - selected text/clipboard lengths for drag runs;
   - whether PDF click/focus and PDF drag selection passed, failed, or remained
     inconclusive;
   - next experiment target.

9. Codex must review the completed output.

   Do not proceed to Experiment 6 until real issues from Codex's review are
   addressed.

## Pass Criteria

Experiment 5 passes if it either:

- fixes protocol-level PDF click/focus or drag selection by routing mouse events
  through Chromium's input router while preserving HTML click/drag and PDF wheel
  scrolling; or
- proves with HTML controls and Chromium route logs that mouse routing works,
  but PDF selection still needs a deeper PDF plugin/focus/selection experiment.

## Partial Criteria

Experiment 5 is partial if:

- mouse routing builds and logs route mode, but the protocol mouse harness
  cannot reliably verify HTML click/drag;
- PDF click/drag evidence is inconclusive because selection/clipboard state is
  not observable;
- the route mode is ambiguous because required Chromium logs are missing.

## Failure Criteria

Experiment 5 fails if:

- it uses CDP input as a substitute for TermSurf protocol mouse input;
- it changes TermSurf protocol messages or Roamium FFI;
- it special-cases PDF URLs, PDF extension ids, or plugin frame ids as the
  primary routing fix;
- it touches wheel behavior beyond preserving Experiment 4;
- it touches PDF stream/resource/loading code;
- it modifies Chromium without a fresh Issue 794 Experiment 5 branch;
- it omits HTML click/drag controls;
- it omits the PDF wheel regression check;
- it omits Codex design or completion review.

## Result

**Result:** Pass

Chromium branch: `148.0.7778.97-issue-794-exp5`

Chromium commit: `3764d958e358f`
(`Route TermSurf mouse input through Chromium router`)

Patch archive: `chromium/patches/issue-794-exp5/`

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

- HTML click: `logs/issue-794-exp5-html-click-20260530-084234`
- HTML drag: `logs/issue-794-exp5-html-drag-20260530-084122`
- PDF click: `logs/issue-794-exp5-pdf-click-20260530-084319`
- PDF drag: `logs/issue-794-exp5-pdf-drag-20260530-084344`
- PDF wheel regression: `logs/issue-794-exp5-wheel-regression-20260530-084658`

Observed results:

- HTML click passed. The harness sent two TermSurf protocol `MouseEvent`
  messages, Roamium logged `ffi=ts_forward_mouse_event`, Chromium logged
  `[issue-794-exp5] mouse-route kind=down/up route_mode=input-router`, and the
  before/after HTML state and screenshot changed. The summary reported
  `first_failing_hop = no-failure-observed`.
- HTML drag passed. The harness sent one down event, eight `MouseMove` messages
  with `modifiers = 64`, and one up event. Roamium logged both mouse-event and
  mouse-move FFI calls, Chromium logged `route_mode=input-router`, and the
  selected text length became `104`. The summary reported
  `first_failing_hop = no-failure-observed`.
- PDF click reached the input router and the PDF frame inventory. The PDF click
  target was inside the plugin bounds at `(450.5, 253.0)`. Roamium logged the
  FFI mouse events, Chromium logged `route_mode=input-router`, and the PDF frame
  inventory showed the root frame, PDF extension frame, and PDF plugin frame all
  containing the input point. The summary reported
  `first_failing_hop = no-failure-observed`, with a state change but no visible
  screenshot change. This proves routing, but does not by itself prove a
  user-visible PDF focus affordance.
- PDF drag reached the input router and the PDF frame inventory, but did not
  produce selected text. The harness sent a valid drag with left-button-down
  move modifiers, Roamium and Chromium both logged the full path, and every
  mouse event used `route_mode=input-router`. The summary still reported
  `first_failing_hop = pdf-focus-or-selection`, with selected text length `0`
  and no screenshot change.
- PDF wheel scrolling still passed after the mouse-routing changes. The
  regression log reported `first_failing_hop = no-failure-observed`, PDF
  screenshot changed, and Chromium logged
  `[issue-794-exp4] wheel-route route_mode=input-router`.

## Conclusion

Experiment 5 fixed and verified the general TermSurf protocol mouse-routing
layer. TermSurf mouse down/up/move messages now reach Chromium's
`RenderWidgetHostInputEventRouter`, normal HTML click and drag still work, PDF
clicks and drags are routed into the PDF frame tree, and the Experiment 4 wheel
fix was preserved.

The remaining PDF drag-selection failure is deeper than root mouse routing. The
next experiment should target PDF plugin focus/selection behavior: whether the
PDF extension frame/internal plugin receives focus in the way Chromium's PDF
selection code expects, whether selection state is exposed outside the plugin
frame, and whether additional PDF viewer/browser binders or focus handoff are
missing.
