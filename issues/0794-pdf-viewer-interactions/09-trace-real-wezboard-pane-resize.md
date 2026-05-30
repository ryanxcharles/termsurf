# Experiment 9: Trace Real Wezboard Pane Resize

## Description

Experiment 8 proved that direct TermSurf protocol resize works for PDFs. A
second `Resize` message reaches Roamium, Chromium `ResizeTab()`, PDF plugin
geometry, and PDFium plugin-size propagation. Therefore, if PDF resize still
looks wrong in a real split-pane Wezboard session, the remaining suspect is
outside Roamium/Chromium: the real Wezboard pane/layout path may not be sending
the correct resize message or screen rect when pane geometry changes.

Experiment 9 should measure the real app path:

```text
Wezboard pane layout
  -> paint.rs set_overlay_frame(...)
  -> termsurf::conn::set_overlay_frame(...)
  -> webview_screen_rect_desc(...)
  -> send_resize_with_screen_rect(...)
  -> Roamium Msg::Resize
  -> Chromium/PDF resize traces from Experiment 8
```

The goal is not to change pane layout yet. The goal is to prove whether a real
split or pane resize sends the same kind of second `Resize` message that
Experiment 8 proved works.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Add gated Wezboard resize traces.

   Use the existing `TERMSURF_PDF_INPUT_TRACE` / `TERMSURF_PDF_INPUT_TRACE_FILE`
   convention so one log file can show both Wezboard and Roamium evidence.

   In `wezboard/wezboard-gui/src/termsurf/conn.rs`, add trace lines around:
   - `set_overlay_frame(...)`: pane id, backing x/y/w/h, dpi, computed scale,
     point x/y/w/h, and mux window id presence;
   - `webview_screen_rect_desc(...)`: local frame, converted screen rect,
     top-left screen rect, scale, and input backing rect;
   - `send_resize_with_screen_rect(...)`: pane id, tab id, pixel width/height,
     screen rect, scale, changed/no-change decision, and whether a resize was
     sent or skipped.

   Keep logs append-only, direct-to-file, and gated. Do not log every paint when
   nothing changed; the `changed` decision should keep volume bounded.

2. Add a real-app resize trace runner if practical.

   Prefer a script such as `scripts/test-issue-794-real-pane-resize.sh` that:
   - creates a log directory and exports `TERMSURF_PDF_INPUT_TRACE=1` /
     `TERMSURF_PDF_INPUT_TRACE_FILE=$LOG_DIR/pdf-input.log` before launching any
     TermSurf process;
   - starts a local fixture server for `test-html/public/bitcoin.pdf`, records
     its port, and writes its stdout/stderr or HTTP log into the log directory;
   - uses an absolute log directory and absolute
     `TERMSURF_PDF_INPUT_TRACE_FILE`, because `web` and Roamium may run with a
     different working directory from the runner;
   - starts debug `wezboard-gui` directly from
     `wezboard/target/debug/wezboard-gui` with the trace environment set;
   - starts the debug GUI before the web pane, waits for the debug GUI's
     TermSurf socket, and then spawns debug `web` through `wezboard cli spawn`
     with `TERMSURF_SOCKET` explicitly set to that socket;
   - clears stale `TERMSURF_SOCKET`, `TERMSURF_PANE_ID`, `WEZBOARD_UNIX_SOCKET`,
     and `WEZBOARD_PANE` from the runner's environment before launching the
     debug GUI, so a shell running inside the user's existing Wezboard cannot
     leak its socket into the automated test;
   - launches that debug GUI with a unique `--class` and `--always-new-process`;
   - on macOS, also passes `WEZBOARD_UNIX_SOCKET` pointing at the debug GUI's
     `gui-sock-$pid` socket for every `wezboard cli` command, because class
     discovery alone is not sufficient isolation when an existing user session
     is running;
   - runs debug `web` inside that window with
     `--browser chromium/src/out/Default/roamium` and the local Bitcoin PDF
     fixture;
   - creates a split pane or otherwise changes pane geometry;
   - collects `TERMSURF_PDF_INPUT_TRACE_FILE`, Wezboard stdout/stderr, Roamium
     stdout/stderr if available, and screenshots if automation permissions
     allow.

   If fully driving Wezboard UI automation is too fragile in one pass, the
   experiment may be a trace-plus-manual-reproduction run, but it must still
   produce objective log evidence for the resize messages. Do not mark Pass from
   visual inspection alone.

   The runner must fail fast if debug `wezboard-gui`, debug `web`, or debug
   `chromium/src/out/Default/roamium` is missing. It should also record the
   exact command line used for each process. If `web` is launched inside
   Wezboard manually rather than by script, prefix the in-terminal command with
   the same `TERMSURF_PDF_INPUT_TRACE` environment or prove from the log that
   Roamium inherited it.

3. Reuse Experiment 8 downstream traces.

   The run should keep `TERMSURF_PDF_INPUT_TRACE=1` so the same log captures:
   - Wezboard `set_overlay_frame` / `send_resize_with_screen_rect`;
   - Roamium `resize ... ffi=ts_set_view_size`;
   - Chromium `[issue-794-exp8] resize-tab`;
   - PDF plugin `[issue-794-exp8] plugin-geometry-changed`;
   - PDFium `[issue-794-exp8] pdfium-plugin-size-updated`.

   This connects the real app path to the direct protocol path proven by
   Experiment 8.

4. Add analyzer fields.

   The result should record:
   - number of `set_overlay_frame` changed events;
   - number of Wezboard `Resize` messages sent;
   - first and last pane pixel sizes;
   - first and last screen rects;
   - whether pane split/resize produced a changed screen rect;
   - whether the changed screen rect produced a `Resize` protobuf;
   - whether Roamium received the matching resize;
   - whether Chromium/PDF/PDFium received the matching resize;
   - screenshot paths if available;
   - first failing layer.

   Suggested first failing layers:
   - `wezboard-no-pane-geometry-change`
   - `wezboard-set-overlay-not-called`
   - `wezboard-screen-rect-not-changing`
   - `wezboard-resize-suppressed`
   - `wezboard-resize-not-sent`
   - `roamium-resize-receive-missing`
   - `chromium-resize-missing`
   - `pdf-plugin-resize-missing`
   - `pdfium-plugin-size-missing`
   - `no-failure-observed`

5. Build and format.

   This experiment changes Rust. Build the debug GUI and debug TUI before
   running the real app test:

   ```bash
   cd "$HOME/dev/termsurf"
   PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" cargo-fmt --manifest-path wezboard/Cargo.toml --all
   PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" ./scripts/build.sh wezboard
   PATH="/Users/ryan/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/Cellar/rustup/1.29.0_1/bin:$PATH" ./scripts/build.sh webtui
   ```

   If scripts change, run syntax checks. Run Prettier on this experiment file
   and the issue README.

## Verification

1. Build Wezboard after adding traces.

2. Run the real app trace with `TERMSURF_PDF_INPUT_TRACE=1`.

   Required setup:
   - create `LOG_DIR=logs/issue-794-exp9-real-pane-resize-...`;
   - start the local Bitcoin PDF fixture server and record its port in
     `$LOG_DIR/fixture-server.log`;
   - launch debug `wezboard-gui` with `TERMSURF_PDF_INPUT_TRACE=1` and
     `TERMSURF_PDF_INPUT_TRACE_FILE=$LOG_DIR/pdf-input.log`;
   - ensure the debug `web` command is launched with the same trace environment
     or prove Roamium inherited it by checking for `roamium trace-init` in the
     log.

   The preferred run should use debug components and must pass the debug Roamium
   binary explicitly:

   ```bash
   TERMSURF_PDF_INPUT_TRACE=1 \
   TERMSURF_PDF_INPUT_TRACE_FILE="$LOG_DIR/pdf-input.log" \
   /Users/ryan/dev/termsurf/webtui/target/debug/web \
     --browser /Users/ryan/dev/termsurf/chromium/src/out/Default/roamium \
     "http://127.0.0.1:<fixture-port>/bitcoin.pdf"
   ```

3. Reproduce the real pane geometry change.

   Use the smallest reliable sequence:
   - open one PDF webview;
   - capture the initial resize trace;
   - create a split pane or resize the pane;
   - capture the follow-up resize trace.

4. Required pass evidence:
   - Wezboard logs a changed `set_overlay_frame` / screen rect after the split
     or pane resize;
   - Wezboard sends a `Resize` protobuf with changed pixel size or screen rect;
   - Roamium receives the matching resize and calls `ts_set_view_size`;
   - Chromium/PDF/PDFium Experiment 8 traces receive the matching resize;
   - if screenshots are available, the PDF webview geometry visibly changes.

5. Record the result in this file.

   The result must include:
   - build commands and results;
   - log directory;
   - trace summary table from Wezboard through PDFium;
   - screenshots if available;
   - first failing layer if any;
   - next experiment target.

6. Codex must review the completed output.

   Do not proceed to Experiment 10 until real issues from Codex's review are
   addressed.

## Pass Criteria

Experiment 9 passes if it proves the real Wezboard pane-resize path either:

- sends correct changed resize geometry all the way to PDFium, making resize
  behavior good enough on the real app path; or
- stops at one exact measured layer, giving the next experiment a concrete fix
  target.

## Partial Criteria

Experiment 9 is partial if:

- Wezboard traces are present, but UI automation cannot reliably create the pane
  split/resize;
- screenshots are unavailable and logs do not fully prove the resize path;
- Wezboard sends changed resize geometry, but downstream Roamium/Chromium logs
  are missing due to process environment propagation.

## Failure Criteria

Experiment 9 fails if:

- it changes pane layout, PDF loading, or PDF resize behavior before recording
  real Wezboard resize evidence;
- it relies on visual inspection without objective resize logs;
- it runs an installed/stable Roamium instead of the repo-built debug Roamium;
- it omits `cargo fmt` after Rust changes;
- it omits Codex design or completion review.

## Result

**Result:** Pass

Valid automated run: `logs/issue-794-exp9-real-pane-resize-20260530-100846`.

The first two runner attempts exposed harness isolation bugs, not product
behavior:

- `logs/issue-794-exp9-real-pane-resize-20260530-100340` used `wezboard cli`
  discovery and accidentally targeted the user's existing Wezboard session.
- `logs/issue-794-exp9-real-pane-resize-20260530-100708` isolated the CLI socket
  but still let stale `TERMSURF_SOCKET` environment from the invoking pane leak
  into the `web` startup path.

The final runner fixed both problems by:

- launching debug `wezboard-gui` with stale TermSurf/Wezboard socket environment
  removed;
- waiting for the debug GUI's concrete mux socket and TermSurf socket;
- spawning debug `web` through that debug mux with
  `TERMSURF_SOCKET=$debug_gui_termsurf_socket`;
- passing debug Roamium explicitly with
  `--browser chromium/src/out/Default/roamium`;
- using an absolute trace log path.

Build and formatting:

- `cargo-fmt --manifest-path wezboard/Cargo.toml --all` ran after the Rust trace
  edits.
- `./scripts/build.sh wezboard` passed.
- `./scripts/build.sh webtui` passed.
- `bash -n scripts/test-issue-794-real-pane-resize.sh` passed.
- Prettier ran on this experiment file.

Trace evidence:

| Layer                       | Evidence                                                                                                                                                                   |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fixture server              | `GET /bitcoin.pdf` returned `200` in `fixture-server.log`.                                                                                                                 |
| Web pane isolation          | `cli-list-before.json` contains only the debug test window with initial `sleep` pane `0` and web pane `1`; it does not include the user's existing panes.                  |
| Pane split                  | `cli-list-after.json` shows web pane `1` resized from `2240x2304` to `2212x1088` device pixels and new split pane `2` created below it.                                    |
| Wezboard initial resize     | `pdf-input.log` records `wezboard-resize pane_id=1 tab_id=1 changed=true result=sent pixel_width=2212 pixel_height=2112 ... screen_width=1106 screen_height=1056 scale=2`. |
| Roamium initial resize      | `pdf-input.log` records matching `roamium resize tab_id=1 pane_id=1 pixel_width=2212 pixel_height=2112 ... ffi=ts_set_view_size`.                                          |
| Chromium initial resize     | `wezboard.stderr` records `[issue-794-exp8] resize-tab requested_width=2212 requested_height=2112 logical_width=1106 logical_height=1056 ...`.                             |
| PDF plugin initial geometry | `wezboard.stderr` records `[issue-794-exp8] plugin-geometry-changed plugin_width=1610 plugin_height=2000 ...`.                                                             |
| PDFium initial size         | `wezboard.stderr` records `[issue-794-exp8] pdfium-plugin-size-updated ... new_width=1610 new_height=2000 visible_pages=1`.                                                |
| Wezboard split resize       | `pdf-input.log` records `wezboard-resize pane_id=1 tab_id=1 changed=true result=sent pixel_width=2184 pixel_height=896 ... screen_width=1092 screen_height=448 scale=2`.   |
| Roamium split resize        | `pdf-input.log` records matching `roamium resize tab_id=1 pane_id=1 pixel_width=2184 pixel_height=896 ... ffi=ts_set_view_size`.                                           |
| Chromium split resize       | `wezboard.stderr` records `[issue-794-exp8] resize-tab requested_width=2184 requested_height=896 logical_width=1092 logical_height=448 ...`.                               |
| PDF plugin split geometry   | `wezboard.stderr` records `[issue-794-exp8] plugin-geometry-changed plugin_width=1582 plugin_height=784 ...`.                                                              |
| PDFium split size           | `wezboard.stderr` records `[issue-794-exp8] pdfium-plugin-size-updated old_width=1610 old_height=2000 new_width=1582 new_height=784 visible_pages=1`.                      |

Counts from the final valid run:

- Wezboard changed resize sends: `2`.
- Roamium resize receives for pane `1`: `3`.
- Chromium `resize-tab` events: `3`.
- PDF plugin geometry changes: present before and after split.
- PDFium plugin-size updates: present before and after split.
- First failing layer: `no-failure-observed`.

The third Roamium resize is an intermediate split-size resize with a zero screen
rect:

```text
roamium resize tab_id=1 pane_id=1 pixel_width=2184 pixel_height=896 \
screen_x=0 screen_y=0 screen_width=0 screen_height=0 screen_scale=0
```

Chromium logs a corresponding transient `[issue-794-exp8] resize-tab` before the
corrected Wezboard-originated screen rect arrives. This does not invalidate the
result because the corrected resize follows immediately and reaches Chromium,
the PDF plugin, and PDFium. It is recorded here so the run is not mistaken for a
clean single-message resize path.

No screenshot was captured in this run. That is acceptable for Experiment 9
because the pass condition was objective resize propagation through the real
Wezboard path, and the log chain proves that propagation.

## Conclusion

Experiment 9 proves the real Wezboard pane-resize path is not the remaining
resize blocker. In an actual debug Wezboard session, creating a bottom split
causes Wezboard to recompute the web pane geometry, send a changed `Resize`
protobuf, Roamium to call `ts_set_view_size`, Chromium to resize the tab, and
the PDF plugin/PDFium to receive the changed plugin size.

The earlier suspicion that real split-pane resize was failing somewhere between
Wezboard layout and PDFium is ruled out. The next experiment should move to the
remaining PDF viewer completeness surface, not continue chasing real resize
plumbing unless the user reports a fresh real-app resize symptom that
contradicts this trace.
