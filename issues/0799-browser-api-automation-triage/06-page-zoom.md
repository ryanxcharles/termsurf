# Experiment 6: Add Automated Page Zoom

## Description

Experiment 1 classified page zoom as `Automatable now`. Experiments 4 and 5
completed generic downloads and JavaScript dialogs, so the next small,
deterministic browser feature is normal web page zoom: Cmd+=, Cmd+-, and Cmd+0.

This experiment adds TermSurf support for Chromium page zoom shortcuts on the
focused tab and proves it with the Issue 799 fake-GUI harness. The fix should
use Chromium's page zoom machinery, not a resize trick, CSS injection, PDF
viewer zoom, terminal font zoom, or a TermSurf-specific scale layer.

The expected user behavior is:

- Cmd+= / Cmd++ zooms the current web page in by one Chromium preset step;
- Cmd+- zooms the current web page out by one Chromium preset step;
- Cmd+0 resets the current web page to the default zoom level;
- ordinary key input still reaches the page when it is not one of those zoom
  shortcuts;
- PDF viewer zoom remains owned by the PDF viewer and is not part of this
  experiment.

## Changes

1. Create a new Chromium branch.

   In `chromium/src`, fork from:

   ```text
   148.0.7778.97-issue-799-exp5
   ```

   Name the new branch:

   ```text
   148.0.7778.97-issue-799-exp6
   ```

   Add it to `chromium/README.md` with a description such as:

   ```text
   Add automated page zoom shortcuts.
   ```

2. Implement page zoom shortcut handling in Chromium.

   Extend `TsBrowserMainParts::ForwardKeyEvent()` in:

   ```text
   chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.cc
   ```

   The existing function already handles TermSurf's Chromium-side browser
   commands for Cmd+[ (back), Cmd+] (forward), and Cmd+R (reload). Add page zoom
   there, in the same command-handling area.

   Before calling `zoom::PageZoom::Zoom(...)`, ensure the tab's `WebContents`
   has a `zoom::ZoomController`. `PageZoom::Zoom()` returns immediately when
   `ZoomController::FromWebContents(web_contents)` is null, and TermSurf cannot
   assume Content Shell created one. Create it with:

   ```text
   zoom::ZoomController::CreateForWebContents(web_contents)
   ```

   Do this either when the tab is created or immediately before the first zoom
   command. Prefer a small helper such as `EnsureTermSurfZoomController(...)`
   near the key-command code so the requirement is explicit and local.

   Required shortcuts:
   - `ui::VKEY_OEM_PLUS` with Meta: call
     `zoom::PageZoom::Zoom(web_contents, content::PAGE_ZOOM_IN)`;
   - `ui::VKEY_ADD` with Meta: same as zoom in;
   - `ui::VKEY_OEM_MINUS` with Meta: call
     `zoom::PageZoom::Zoom(web_contents, content::PAGE_ZOOM_OUT)`;
   - `ui::VKEY_SUBTRACT` with Meta: same as zoom out;
   - `ui::VKEY_0` with Meta: call
     `zoom::PageZoom::Zoom(web_contents, content::PAGE_ZOOM_RESET)`;
   - `ui::VKEY_NUMPAD0` with Meta: same as reset.

   Use Chromium's existing helper:

   ```text
   components/zoom/page_zoom.h
   content/public/common/page_zoom.h
   ```

   `libtermsurf_chromium` already depends on `//components/zoom` for recent PDF
   work, so this should not require a new GN dependency. If the build proves
   otherwise, add only the narrow dependency required by the helper.

   The shortcut recognition should happen before raw key forwarding. Apply the
   zoom action only for key down/repeat events, but consume the matching key-up
   event too. In other words:
   - Meta+zoom key down/repeat: perform the zoom command and return;
   - Meta+zoom key up: do not perform another zoom command, but still return;
   - non-zoom keys: keep the current forwarding behavior.

   This prevents the page from seeing half of a consumed browser shortcut.

3. Preserve routing semantics for non-zoom keys.

   The zoom shortcuts must be consumed by Chromium and must not also be
   forwarded to the page as raw key events. Non-zoom Meta key combinations and
   all ordinary key events must keep the current forwarding behavior.

   Do not add a protocol message for page zoom unless the direct key-command
   path proves insufficient. The user-visible feature is the standard browser
   shortcut, and the protocol already carries key modifiers.

4. Extend the Issue 799 harness key-event helper.

   Update:

   ```text
   scripts/test-issue-799-browser-api-audit.py
   ```

   `KeyEvent` already has field 5 for modifiers in `termsurf.proto`, but the
   harness helper currently does not encode it. Add a `modifiers` argument to
   `key_event_payload(...)` and include field 5.

   Keep all existing callers working by defaulting `modifiers` to `0`.

5. Add a deterministic page zoom probe.

   Add a probe such as:

   ```text
   page-zoom-shortcuts
   ```

   The local fixture should render measurable content, listen for key events,
   and report viewport/layout metrics to the harness. The harness should:
   1. load the page and wait for a baseline report;
   2. send focus for the tab;
   3. send Cmd+= as a real TermSurf key down/up pair with `modifiers = 8`;
   4. wait for a report showing the zoom-in effect;
   5. send Cmd+- as a real TermSurf key down/up pair with `modifiers = 8`;
   6. wait for a report showing movement back toward the baseline;
   7. send Cmd+0 as a real TermSurf key down/up pair with `modifiers = 8`;
   8. wait for a report showing reset to the baseline/default state.

   Use the same modifier bit that Wezboard sends for `Modifiers::SUPER`:

   ```text
   8
   ```

   Use exact Windows virtual key codes in the harness so the fake-GUI path
   matches what Roamium receives from Wezboard:

   | Shortcut         | Chromium key         | Windows VK |
   | ---------------- | -------------------- | ---------- |
   | Cmd+= / Cmd++    | `ui::VKEY_OEM_PLUS`  | `187`      |
   | Cmd+-            | `ui::VKEY_OEM_MINUS` | `189`      |
   | Cmd+0            | `ui::VKEY_0`         | `48`       |
   | Cmd+keypad plus  | `ui::VKEY_ADD`       | `107`      |
   | Cmd+keypad minus | `ui::VKEY_SUBTRACT`  | `109`      |
   | Cmd+keypad 0     | `ui::VKEY_NUMPAD0`   | `96`       |

   The focused probe must cover the primary keyboard shortcuts `VKEY_OEM_PLUS`,
   `VKEY_OEM_MINUS`, and `VKEY_0`. Keypad variants are required in the
   implementation, but may be verified by a second focused probe or by explicit
   log/harness evidence if adding all keypad variants to the main sequence makes
   the probe noisy.

   Prefer page-observable metrics over screenshots, but require metrics that
   prove real browser zoom. A fixed CSS element's `getBoundingClientRect()` is
   not enough because it may remain constant in CSS pixels under browser zoom.
   The verifier must require:
   - `window.devicePixelRatio` changes in the expected direction after Cmd+=;
   - at least one CSS viewport metric, such as `window.innerWidth`,
     `document.documentElement.clientWidth`, or `visualViewport.width`, moves in
     the opposite expected direction;
   - after Cmd+-, those metrics move back toward baseline;
   - after Cmd+0, those metrics return within a small tolerance of baseline.

   Record the raw metric snapshots in `probe-result.json` so failures are
   diagnosable without rerunning the app.

   Also prove key routing:
   - the page should record any received `keydown`/`keyup` events;
   - the zoom shortcut key down/up events must not be observed by the page;
   - after reset, the harness should send a normal `a` key down/up event with
     `modifiers = 0`, and the page must report receiving it.

   This makes the "consume browser shortcut, preserve ordinary page input"
   behavior mechanically verifiable.

6. Classify page zoom distinctly in the harness.

   Add a result verifier for the page zoom probe and classify success as:

   ```text
   page_zoom_completed
   ```

   If the page runs but the metric never changes after Cmd+=, classify as:

   ```text
   page_zoom_failed
   ```

   Update `coverage-map.md` and `reference-coverage-map.md` output so the new
   classification has an accurate next-action message.

7. Run formatters.

   Run the required formatters after edits:

   ```bash
   prettier --write --prose-wrap always --print-width 80 \
     issues/0799-browser-api-automation-triage/README.md \
     issues/0799-browser-api-automation-triage/06-page-zoom.md
   ```

   If any Rust files are edited unexpectedly, run `cargo fmt` and accept its
   output. This experiment is expected to edit Python, C++, markdown, and
   Chromium metadata, not Rust.

8. Build and run automated verification.

   Build Chromium:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Build Roamium if the Chromium output or Rust side requires it:

   ```bash
   PATH="/Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin:$PATH" \
     ./scripts/build.sh roamium
   ```

   Run the focused page zoom probe:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py \
     --probe page-zoom-shortcuts \
     --seconds 10
   ```

   Then run the full Issue 799 harness:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py --seconds 10
   ```

9. Archive Chromium only after a passing implementation.

   If the experiment passes, commit the Chromium branch and regenerate:

   ```bash
   cd chromium/src
   rm -rf ../../chromium/patches/issue-799/
   git format-patch 148.0.7778.97..HEAD -o ../../chromium/patches/issue-799/
   ```

   Commit the main repo changes, including the updated patch archive and issue
   result. If the experiment is Partial or Fail, record the result first and do
   not archive an incoherent Chromium branch unless the partial branch is the
   intended base for the next experiment.

10. Get Codex review before and after implementation.

    Before implementation, run `codex-review` against this experiment design and
    fix all real findings before starting code changes.

    After implementation and result recording, run `codex-review` again against
    the diff, test output, and recorded result. Do not mark the experiment Pass
    until Codex agrees there are no blocking issues or all real issues are
    fixed.

## Verification

This experiment passes if:

- Chromium builds with `autoninja -C out/Default libtermsurf_chromium`;
- the focused `page-zoom-shortcuts` probe classifies as `page_zoom_completed`;
- the focused probe records baseline, zoom-in, zoom-out, and reset metric
  snapshots in `probe-result.json`;
- the metric snapshots prove real browser zoom with `devicePixelRatio` and a CSS
  viewport metric moving in the expected directions;
- Cmd+= / Cmd++ / Cmd+- / Cmd+0 are handled through the real TermSurf key event
  path with Meta modifier `8`;
- zoom shortcut key down/up events are not delivered to the page, while a normal
  `a` key still is;
- the full Issue 799 harness still completes with the previously passing
  download and JavaScript-dialog probes green;
- no renderer bad-Mojo or crash signatures appear in the page zoom probe logs;
- non-zoom key forwarding remains unchanged for existing probes;
- Codex reviews the completed experiment and no blocking findings remain.

This experiment is partial if:

- Chromium page zoom works from C++ logs or direct helper calls, but the harness
  cannot observe a stable page metric yet;
- the focused page zoom probe passes but the full Issue 799 harness regresses in
  an unrelated existing probe and the cause is diagnosed;
- Cmd shortcuts work only for one keyboard representation, such as
  `VKEY_OEM_PLUS`, but keypad variants are not handled yet;
- the build passes but the implementation needs a follow-up protocol or harness
  adjustment to prove the user-visible behavior.

This experiment fails if:

- it fakes zoom by resizing the webview, changing terminal font size, injecting
  CSS, using page scale as a replacement for browser zoom, or touching the PDF
  viewer zoom controls;
- it changes Wezboard pane zoom or terminal grid sizing;
- it adds broad browser UI, Chrome zoom bubble UI, or a product settings stack;
- it consumes ordinary key input that should still reach the page;
- it cannot prove Cmd+=, Cmd+-, and Cmd+0 through automated verification.
