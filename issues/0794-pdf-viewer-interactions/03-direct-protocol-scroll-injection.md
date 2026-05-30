# Experiment 3: Direct Protocol Scroll Injection

## Description

Experiment 1 proved that Chromium's PDF viewer can scroll when driven through
DevTools `Input.dispatchMouseEvent`. Experiment 2 added Wezboard/Roamium trace
points, but macOS synthetic wheel injection did not reach Wezboard. The user
correctly pointed out that this does not require hardware input: TermSurf
already has a protocol path for scroll events.

Experiment 3 pivots to a fully automated protocol-layer test. It should launch
Roamium against a fake/minimal GUI socket, create a PDF tab, capture a before
DevTools screenshot/state, send one or more length-prefixed
`TermSurfMessage { scroll_event }` messages directly to Roamium, then capture an
after screenshot/state.

This measures the currently important path:

```text
TermSurf ScrollEvent
→ Roamium dispatch
→ ts_forward_scroll_event
→ Chromium input routing
→ PDF extension/plugin scroll
```

It deliberately does **not** test macOS hardware-event delivery into Wezboard.
That full app-path check can return later if needed. For now, the question is
whether a normal TermSurf scroll message makes the PDF viewer scroll.

This experiment is diagnostic first. It may include small diagnostic trace
improvements, but it must not change scroll behavior. It must receive Codex
design review before implementation. After the result is recorded, Codex must
review the completed output before the next experiment is designed.

## Changes

1. Reuse the existing fake-GUI Roamium harness as the base.

   Start from `scripts/test-issue-792-fake-gui.py`, which already:
   - opens a fake GUI Unix socket;
   - launches repo-built Roamium with `--ipc-socket`;
   - receives `ServerRegister`;
   - sends `CreateTab`;
   - waits for `TabReady`;
   - sends `Resize`;
   - can serve the vendored Bitcoin PDF with `application/pdf`.

   Add a new Issue 794 harness, preferably:

   ```text
   scripts/test-issue-794-protocol-scroll.py
   ```

   Do not mutate Issue 792's historical harness in place unless the change is a
   harmless extraction into shared helpers.

2. Add protobuf support for `ScrollEvent`.

   Use the existing hand-encoded protobuf helpers from the fake-GUI harness. The
   existing fake-GUI helper has varint/string/bool encoders only; `ScrollEvent`
   also needs a fixed64 double encoder:

   ```python
   def double_field(number: int, value: float) -> bytes:
       return field(number, 1) + struct.pack("<d", value)
   ```

   Encode:

   ```text
   TermSurfMessage.scroll_event = field 8
   ScrollEvent.tab_id = field 1
   ScrollEvent.x = field 2
   ScrollEvent.y = field 3
   ScrollEvent.delta_x = field 4
   ScrollEvent.delta_y = field 5
   ScrollEvent.phase = field 6
   ScrollEvent.momentum_phase = field 7
   ScrollEvent.precise = field 8
   ScrollEvent.modifiers = field 9
   ```

   `x` and `y` should target the visible PDF plugin/container area. Derive the
   target point from the before-probe PDF bounds reported by
   `scripts/capture-pdf-interactions.mjs`: prefer the center of the visible
   `EMBED#plugin` bounds, then the PDF viewer container bounds, then a fixed
   viewport-relative fallback only if no bounds are available. Record the chosen
   source and exact coordinates.

   Send several scroll messages with `delta_y` values large enough to move the
   first page if Chromium receives them. Preserve the same sign convention that
   existing Wezboard scroll forwarding uses.

3. Capture before/after PDF state through DevTools.

   Reuse `scripts/capture-pdf-interactions.mjs` in `--mode probe` if the fake
   GUI launch exposes a DevTools port in Roamium logs. The harness should:
   - parse the DevTools port from Roamium stderr/stdout;
   - capture `$LOG_DIR/before/summary.json` and `baseline.png`;
   - send protocol `ScrollEvent` messages;
   - wait a short settle interval;
   - capture `$LOG_DIR/after/summary.json` and `baseline.png`.

   If the existing DevTools helper cannot attach in the fake-GUI setup, record
   that as an automation gap and design the next experiment around DevTools
   attachment. Do not fall back to human screenshot inspection as the primary
   result.

4. Add or adapt a direct-protocol analyzer.

   The analyzer should emit a JSON summary such as:

   ```text
   $LOG_DIR/protocol-scroll-summary.json
   ```

   Required fields:
   - whether `ServerRegister` was received;
   - whether `CreateTab` was sent;
   - the `TabReady` tab id;
   - whether `Resize` was sent;
   - how many protocol `ScrollEvent` messages were sent;
   - scroll coordinates and deltas sent;
   - whether scroll coordinates came from plugin bounds, container bounds, or a
     fixed fallback;
   - whether Roamium logged `trace-init`;
   - whether Roamium logged `scroll-event`;
   - whether Roamium logged `ffi=ts_forward_scroll_event`;
   - whether before/after PDF state changed;
   - whether before/after screenshots changed;
   - `first_failing_hop`.

   Use this hop ladder:

   | Hop                         | Pass signal                                          |
   | --------------------------- | ---------------------------------------------------- |
   | Roamium registered          | `ServerRegister` received                            |
   | PDF tab created             | `CreateTab` sent and `TabReady` received             |
   | View sized                  | `Resize` sent                                        |
   | Protocol scroll sent        | one or more `ScrollEvent` messages sent              |
   | Roamium trace initialized   | `roamium trace-init` unless later scroll lines exist |
   | Roamium received scroll     | `roamium scroll-event` trace line                    |
   | Roamium called Chromium FFI | `ffi=ts_forward_scroll_event` trace line             |
   | PDF viewer scrolled         | before/after DevTools state or screenshot differs    |

   Suggested `first_failing_hop` values:
   - `roamium-not-registered`
   - `tab-not-ready`
   - `resize-not-sent`
   - `protocol-scroll-not-sent`
   - `trace-env-not-inherited`
   - `roamium-receive-missing`
   - `chromium-ffi-missing`
   - `chromium-or-pdf-routing`
   - `no-failure-observed`
   - `automation-gap`

5. Remove stale manual-wheel Experiment 3 helpers from the active worktree.

   The direct-protocol experiment should not depend on
   `TERMSURF_REAL_WHEEL_MODE=manual`,
   `scripts/analyze-issue-794-input-trace.mjs`, or new Wezboard
   `send_to_chromium` trace changes. Those can be redesigned later if a full
   app-path experiment is still needed.

6. Update review language to Codex.

   Forward-looking Issue 794 and Experiment 3 language should say Codex review,
   not Claude review. Completed historical experiment files can remain as they
   were unless they need a result correction.

7. Run formatters and checks.
   - Run `prettier` on this markdown file and the issue README.
   - If Rust changes are made, run `cargo fmt` and accept the output.
   - Run syntax checks for any new Python/JavaScript/shell scripts.
   - Build any Rust component that changed.

## Verification

1. Run the protocol-scroll harness:

   ```bash
   LOG_DIR="logs/issue-794-exp3-protocol-scroll-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-scroll.py \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf
   ```

   The exact command may change if the harness chooses positional arguments, but
   it must write all artifacts under `logs/issue-794-exp3-protocol-scroll-*`.

2. Inspect required artifacts:
   - `$LOG_DIR/messages.log`
   - `$LOG_DIR/pdf-input.log`
   - `$LOG_DIR/protocol-scroll-summary.json`
   - `$LOG_DIR/before/summary.json`
   - `$LOG_DIR/after/summary.json`
   - `$LOG_DIR/before/baseline.png`
   - `$LOG_DIR/after/baseline.png`
   - Roamium stdout/stderr logs

3. Pass/fail decision:
   - If `first_failing_hop` is `no-failure-observed`, protocol scroll works for
     PDFs. The next experiment should target any remaining failed interaction
     surface, such as text selection or resize/reflow.
   - If `first_failing_hop` is `chromium-or-pdf-routing`, Roamium receives and
     forwards scroll, but Chromium/PDF does not scroll. The next experiment
     should add Chromium-side scroll/PDF routing instrumentation on a fresh
     Issue 794 Chromium branch.
   - If `first_failing_hop` is before `chromium-or-pdf-routing`, fix that
     harness/protocol/Roamium layer before touching Chromium.

4. Record the result in this file.

   The result must include:
   - exact log directory;
   - tab id;
   - scroll coordinates and deltas sent;
   - analyzer `first_failing_hop`;
   - whether Roamium trace initialization was observed;
   - whether Roamium logged receive and FFI lines;
   - whether before/after screenshot or state changed;
   - next experiment target.

5. Codex must review the completed output.

   Do not proceed to Experiment 4 until real issues from Codex's review are
   addressed.

## Pass Criteria

Experiment 3 passes if it fully automates protocol-level PDF scrolling and
identifies the first failing hop with machine-readable evidence.

Examples of pass outcomes:

- Protocol `ScrollEvent` reaches Roamium and Chromium FFI, but the PDF does not
  scroll. The next target is Chromium/PDF routing.
- Protocol `ScrollEvent` reaches Roamium and the PDF scrolls. The next target is
  the next incomplete PDF interaction surface.
- The fake-GUI harness exposes a lower-level setup bug, such as missing
  `TabReady` or missing DevTools attachment, that must be fixed before
  interpreting PDF scroll behavior.

## Partial Criteria

Experiment 3 is partial if:

- the harness can launch and create a PDF tab but cannot attach DevTools for
  before/after state;
- the harness can send scroll events but cannot produce a reliable screenshot or
  state diff;
- logs prove Roamium forwards the scroll but more Chromium-side instrumentation
  is needed to classify why the PDF does or does not scroll.

## Failure Criteria

Experiment 3 fails if:

- it depends on physical mouse/trackpad input;
- it uses CDP input as a substitute for TermSurf protocol `ScrollEvent`;
- it cannot prove whether the protocol `ScrollEvent` reached Roamium;
- it changes scrolling behavior instead of measuring it;
- it asks the user to inspect raw logs without producing an analyzer summary;
- it modifies Chromium without creating a fresh Issue 794 Chromium branch;
- it omits required formatting, syntax, or build checks.

## Result

**Result:** Pass

The direct protocol harness was implemented as
`scripts/test-issue-794-protocol-scroll.py`. It launches repo-built Roamium
against a fake GUI Unix socket, serves the vendored Bitcoin PDF fixture, creates
a PDF tab, captures before/after DevTools probe artifacts, sends hand-encoded
TermSurf `ScrollEvent` messages, and writes `protocol-scroll-summary.json`.

The primary run wrote artifacts to:

```text
logs/issue-794-exp3-protocol-scroll-20260530-080842
```

Key analyzer output:

- tab id: `1`
- DevTools port: `51405`
- scroll coordinate source: `plugin-bounds`
- scroll coordinates: `x=450.5`, `y=253.0`, plugin bounds `width=299.0`,
  `height=394.0`
- scroll events sent: `6`
- scroll deltas: five events with `delta_y=600.0`, followed by one phase-ended
  event with `delta_y=0.0`
- phases: `1` (`kPhaseBegan`), four `4` (`kPhaseChanged`), then `8`
  (`kPhaseEnded`)
- `roamium_trace_init`: `true`
- `roamium_scroll_event_line`: `true`
- `roamium_ffi_line`: `true`
- before/after screenshot changed: `false`
- before/after state changed: `false`
- `first_failing_hop`: `chromium-or-pdf-routing`

The trace file confirms the protocol message reached Roamium and was forwarded
to Chromium:

```text
roamium scroll-event tab=1 pane=fake-pane ffi=ts_forward_scroll_event coords=(450.50, 253.00) delta=(0.00, 600.00) phase=1 momentum_phase=0 precise=true modifiers=0
```

The before and after screenshots had the same SHA-256:

```text
927e13124bc9738175b456c22c3aa03c59a35fc916a050678bbc40499dffecac
```

I also ran an opposite-sign check:

```text
logs/issue-794-exp3-protocol-scroll-negative-20260530-080532
```

That run sent five `delta_y=-600.0` events plus a phase-ended event. It produced
the same result: Roamium received the scroll, called `ts_forward_scroll_event`,
and the PDF state/screenshot did not change. This rules out a simple
positive-vs-negative wheel sign mistake at this layer.

One harness correction happened during implementation. An initial run sent wheel
events with both `phase=0` and `momentum_phase=0`. Chromium DCHECKed in
`MouseWheelEventQueue` because wheel events must carry a real phase or momentum
phase. The harness now sends a realistic began/changed/ended phase sequence.

Syntax check:

```bash
python3 -m py_compile scripts/test-issue-794-protocol-scroll.py
```

The final positive-delta verification command exited `0`; diagnostic outcomes
such as `chromium-or-pdf-routing` are successful harness classifications, not
shell failures.

## Conclusion

Experiment 3 successfully automated the protocol-level PDF scroll check. The
failure is past the TermSurf socket and Roamium dispatch layers: Roamium
receives `ScrollEvent`, finds the tab, and calls Chromium's
`ts_forward_scroll_event`, but Chromium/PDF does not scroll.

The next experiment should instrument Chromium's wheel routing on a fresh Issue
794 Chromium branch. The immediate question is whether
`TsBrowserMainParts::ForwardScrollEvent()` is forwarding the wheel to the wrong
`RenderWidgetHost` for the PDF viewer's out-of-process extension frame/plugin,
or whether the event reaches the correct renderer and is ignored inside the PDF
viewer/plugin stack.
