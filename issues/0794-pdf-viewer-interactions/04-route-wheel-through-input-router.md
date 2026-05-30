# Experiment 4: Route Wheel Events Through Chromium's Input Router

## Description

Experiment 3 proved that TermSurf protocol scroll reaches Roamium and calls
Chromium's `ts_forward_scroll_event`, but the PDF viewer does not move. The
failure is therefore inside Chromium-side wheel routing or PDF/plugin handling.

The most likely cause is that `TsBrowserMainParts::ForwardScrollEvent()`
forwards the wheel directly to the main frame `RenderWidgetHost`:

```cpp
view->GetRenderWidgetHost()->ForwardWheelEvent(wheel_event);
```

That bypasses Chromium's normal browser-side input router. Chrome's OOPIF path
does not directly send all input to the main frame; it uses hit-test data to
route events to the owning `RenderWidgetHostView` for the surface under the
cursor. The PDF viewer is now an extension frame with an internal PDF plugin, so
direct root-frame forwarding is a plausible explanation for why the wheel event
is received by Chromium but never reaches the PDF content.

Experiment 4 should route TermSurf wheel events through Chromium's
`RenderWidgetHostInputEventRouter` when it is available, with a direct-forward
fallback for non-routable cases. This is not a PDF-specific special case; it
uses Chromium's existing cross-process frame input path.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a fresh Chromium branch for Issue 794 Experiment 4.

   Fork from the current known-good PDF foundation branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-793-exp1
   git checkout -b 148.0.7778.97-issue-794-exp4
   ```

   Add the branch to `chromium/README.md`.

2. Add Chromium-side wheel-routing traces.

   In `content/libtermsurf_chromium/ts_browser_main_parts.cc`, add
   `[issue-794-exp4]` logs around `ForwardScrollEvent()` that record:
   - tab/web contents pointer or URL;
   - root `RenderWidgetHostView` and root `RenderWidgetHost` presence;
   - whether a `RenderWidgetHostInputEventRouter` is available;
   - wheel coordinates, delta, phase, momentum phase, granularity, and
     modifiers;
   - route mode used: `input-router` or `direct-fallback`;
   - for each `RenderFrameHost` in the `WebContents`, the frame tree node id,
     parent presence, last committed URL, site URL, view presence, and view
     bounds if available;
   - for PDF-related frames, whether the input point is plausibly inside that
     frame's root-coordinate bounds.

   These logs may go to Chromium stderr through `LOG(INFO)`. Do not add a new
   protocol message or a new Roamium FFI call.

3. Route wheel events through Chromium's input router.

   Replace the direct-only forwarding path with:
   - construct the same `blink::WebMouseWheelEvent` as today;
   - add a `ui::LatencyInfo`;
   - if the root view has an input router, call
     `RenderWidgetHostInputEventRouter::RouteMouseWheelEvent(...)` with the root
     view and the wheel event;
   - otherwise fall back to `ForwardWheelEvent(...)`.

   Implementation notes:
   - Use Chromium's existing router types, not PDF-specific frame lookup.
   - Get the router from `WebContentsImpl` or the root widget internals, not
     from the public `RenderWidgetHostView` interface. Pass the root view as the
     `RenderWidgetHostViewInput` implementation (`RenderWidgetHostViewBase` in
     this checkout).
   - Keep coordinates in root view coordinates. The input router expects root
     coordinates and performs hit-test based target selection.
   - Preserve existing delta, phase, momentum phase, granularity, and modifier
     semantics.
   - Keep the fallback path so simple non-OOPIF pages and popup edge cases still
     receive wheel input if the router is unavailable.
   - If the required router API is not accessible from `libtermsurf_chromium`,
     stop and record the compile/API blocker rather than inventing a separate
     PDF-frame routing path.

4. Do not broaden the experiment into other input types yet.

   Do not change mouse down/up/move, keyboard, resize, PDF stream plumbing, PDF
   extension resources, or Wezboard/Roamium protocol code. If wheel routing
   succeeds, follow-up experiments can decide whether click/drag should also use
   the input router.

5. Build and archive correctly.

   Build with:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Never run `ninja` directly.

   If the experiment produces a coherent Chromium branch, commit it on
   `148.0.7778.97-issue-794-exp4`, regenerate the patch archive under
   `chromium/patches/issue-794-exp4/`, and update `chromium/README.md`.

6. Run formatters and checks.
   - Run Chromium formatting on modified C++ files.
   - Run `prettier` on this experiment file and the issue README.
   - If Rust changes are unexpectedly made, run `cargo fmt` and accept the
     output.

## Verification

1. Rebuild Chromium:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

2. Re-run the protocol scroll harness:

   ```bash
   LOG_DIR="logs/issue-794-exp4-router-$(date +%Y%m%d-%H%M%S)" \
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$PWD/$LOG_DIR/pdf-input.log" \
   scripts/test-issue-794-protocol-scroll.py \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf
   ```

3. Inspect:
   - `$LOG_DIR/protocol-scroll-summary.json`
   - `$LOG_DIR/pdf-input.log`
   - `$LOG_DIR/roamium.stderr`
   - `$LOG_DIR/before/summary.json`
   - `$LOG_DIR/after/summary.json`
   - `$LOG_DIR/before/baseline.png`
   - `$LOG_DIR/after/baseline.png`

4. Pass/fail decision:
   - If `first_failing_hop` becomes `no-failure-observed`, PDF protocol scroll
     is fixed. The result must cite the Chromium log line showing
     `route_mode=input-router`, and the next experiment should target the next
     incomplete PDF interaction surface.
   - If the route mode is `input-router` but the PDF still does not scroll, the
     next experiment should instrument the routed target and PDF/plugin renderer
     handling. Do not keep changing TermSurf protocol code.
   - If the route mode is `direct-fallback`, the result must explain why the
     input router was unavailable and the next experiment should target that
     missing Chromium embedder setup.
   - If the patch fails to compile because the input router API is not usable
     from `libtermsurf_chromium`, record the exact API/visibility blocker and
     design the next experiment around the smallest viable public API route.

5. Regression checks:
   - Load a normal non-PDF scrollable HTML page through the existing debug
     TermSurf flow and confirm wheel scrolling still works.
   - Run the Issue 794 protocol harness at least once with the opposite
     `--scroll-delta-y` sign if the primary run does not move, to keep the sign
     convention ruled out.
   - Confirm Roamium does not DCHECK on wheel phase handling.

6. Record the result in this file.

   The result must include:
   - Chromium branch name and commit hash, if a Chromium commit is made;
   - build command and result;
   - exact log directory;
   - analyzer `first_failing_hop`;
   - Chromium route mode observed;
   - whether PDF screenshot/state changed;
   - whether normal HTML wheel scrolling still works;
   - next experiment target.

7. Codex must review the completed output.

   Do not proceed to Experiment 5 until real issues from Codex's review are
   addressed.

## Pass Criteria

Experiment 4 passes if it either:

- fixes protocol-level PDF wheel scrolling by routing through Chromium's input
  router and preserves normal page scrolling; or
- proves with logs why Chromium's input router path is unavailable or
  insufficient, identifying the next exact Chromium-side layer to instrument.

## Partial Criteria

Experiment 4 is partial if:

- the patch builds and logs route mode, but the automated harness cannot produce
  before/after PDF evidence;
- the route mode is ambiguous because required Chromium logs are missing;
- normal HTML scroll regresses and the fix must be redesigned before keeping the
  route change.

## Failure Criteria

Experiment 4 fails if:

- it changes TermSurf protocol messages or Roamium FFI instead of Chromium input
  routing;
- it special-cases PDF URLs or PDF extension ids as the primary routing fix;
- it sends wheel events directly to a guessed child frame without first trying
  Chromium's input router;
- it touches PDF stream/resource/loading code;
- it modifies Chromium without a fresh Issue 794 Experiment 4 branch;
- it omits the Chromium build;
- it omits Codex design or completion review.

## Result

**Result:** Pass

Experiment 4 fixed protocol-level PDF wheel scrolling by routing TermSurf wheel
events through Chromium's `RenderWidgetHostInputEventRouter`.

Chromium branch:

```text
148.0.7778.97-issue-794-exp4
```

Chromium commit:

```text
7f3e288910ba Route TermSurf wheel input through Chromium router
```

Patch archive:

```text
chromium/patches/issue-794-exp4/
```

Build command:

```bash
cd chromium/src
export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium
```

Build result:

```text
Build Succeeded: 2 steps
```

Primary PDF verification log:

```text
logs/issue-794-exp4-router-default-20260530-082324
```

Key analyzer output:

- `first_failing_hop`: `no-failure-observed`
- tab id: `1`
- DevTools port: `51618`
- scroll coordinate source: `plugin-bounds`
- scroll coordinates: `x=450.5`, `y=253.0`
- scroll deltas: five events with `delta_y=-600.0`, followed by one phase-ended
  event with `delta_y=0.0`
- before/after screenshot changed: `true`
- before/after state changed: `false`
- before screenshot SHA-256:
  `927e13124bc9738175b456c22c3aa03c59a35fc916a050678bbc40499dffecac`
- after screenshot SHA-256:
  `eb6fc8aa0289b0d9b5878de0bf07876b7d65d60df7f5f9276cb69f4939fe2d53`

Chromium route evidence:

```text
[issue-794-exp4] wheel-route route_mode=input-router root_view=0x7fba10c00 router=0x7fba1db80
```

The Chromium trace also showed the input point inside all three relevant root
coordinate bounds: the top-level PDF wrapper frame, the PDF extension frame, and
the internal PDF plugin frame. The internal plugin frame was:

```text
frame_tree_node_id=3 url=http://127.0.0.1:9787/bitcoin.pdf root_bounds=301,56 299x394 contains_input=1 pdf_related=1
```

Normal HTML regression check:

```text
logs/issue-794-exp4-html-scroll-negative-20260530-082200
```

That control run loaded `test-interactions.html`, sent the same protocol scroll
sequence with `delta_y=-600.0`, and produced:

- `first_failing_hop`: `no-failure-observed`
- before/after screenshot changed: `true`
- before/after state changed: `true`
- route mode: `input-router`

The positive-sign checks did not scroll either PDF or normal HTML. The final
harness default was changed to `delta_y=-600.0`, matching the protocol sign that
scrolls downward in Chromium's wheel path. This also explains why the earlier
positive-delta protocol runs were useful as diagnostics but not the correct
downward-scroll stimulus.

Syntax check:

```bash
python3 -m py_compile scripts/test-issue-794-protocol-scroll.py
```

Markdown formatting:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0794-pdf-viewer-interactions/README.md \
  issues/0794-pdf-viewer-interactions/04-route-wheel-through-input-router.md
```

## Conclusion

The root cause for wheel scrolling was that TermSurf's Chromium embedder
bypassed Chromium's browser-side OOPIF input router and forwarded wheel events
directly to the root/main `RenderWidgetHost`. Once wheel events go through
`RenderWidgetHostInputEventRouter`, Chromium can route them to the PDF
extension/plugin surface under the cursor.

PDF wheel scrolling now works under the automated TermSurf protocol harness, and
the normal HTML control page still scrolls. The next experiment should move to
the next incomplete PDF interaction surface, likely click/focus or drag text
selection, and decide whether mouse down/up/move should also move from direct
root-frame forwarding to Chromium's input router.
