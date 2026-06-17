# Experiment 21: TUI Overlay Resize Command

## Description

Experiment 20 proved browser overlay geometry follows terminal font/cell metric
changes. The next matrix row is a TUI-requested overlay resize.

Today `webtui` sends `SetOverlay` whenever its rendered viewport rectangle
changes, but it does not expose a deliberate user command that changes the
viewport rectangle independently of terminal window, pane, tab, font, or split
geometry. This experiment should add and prove a narrow user-visible command
path that lets the TUI request a smaller browser viewport and then reset back to
the full viewport.

This experiment should prove the whole protocol chain follows the TUI-requested
rectangle: `webtui` command state, `SetOverlay` cells, Ghostboard Zig geometry,
AppKit frame/pixels, Roamium resize, hit testing, and Browse-mode keyboard
routing.

This experiment intentionally covers one window with one browser overlay. It
does not test scrollback, browser navigation, DevTools, splits, tabs, multiple
windows, fullscreen, display moves, or final matrix regression.

If current Ghostboard already passes once the TUI command exists, the experiment
should record that and avoid Ghostboard product changes. If it fails, the
harness must first localize whether the failure is TUI command dispatch,
`SetOverlay` emission, Zig overlay update, AppKit frame/pixel recomputation,
Roamium resize delivery, stale hit testing, or keyboard routing before any
product fix is designed.

## Changes

Planned files:

- `webtui/src/main.rs`
  - add a minimal command-mode command, for example `:viewport height <rows>`
    with aliases such as `:vp height <rows>`, plus `:viewport reset`;
  - keep the command user-visible and deterministic, not a hidden test-only
    shortcut;
  - store an optional viewport inner-height override in TUI state;
  - apply the override only to the viewport block height, leaving URL/status UI
    below it and blank filler in the remaining terminal area;
  - cap the requested height to the available terminal area so text/layout does
    not underflow on small windows;
  - clear the override on reset so the viewport returns to normal fill behavior;
  - emit `SetOverlay` naturally through the existing "viewport rect changed"
    path when the command changes the rendered viewport rectangle;
  - keep DevTools behavior unchanged unless the implementation path naturally
    shares the same viewport layout helper.
- `scripts/ghostboard-geometry-matrix.sh`
  - add a `tui-overlay-resize-command` scenario;
  - launch one browser in one Ghostboard window using the repo-built `web` and
    Roamium binaries;
  - record the baseline canonical identity tuple:
    `window_id + surface_id + selected_tab_id + pane_id + browser_tab_id`, plus
    `context_id + grid + cell size + AppKit frame + AppKit pixels + backing_scale`;
  - enter the TUI command mode using real keyboard input, type the viewport
    height command, and submit it;
  - wait for fresh post-command `SetOverlay`/geometry records after the command
    boundary;
  - require the same canonical identity and context id after the TUI-requested
    shrink;
  - require the overlay cell height, AppKit frame height, and AppKit pixel
    height to shrink while the change is tied to a fresh `SetOverlay` update,
    not to a window or pane resize;
  - require Zig to record the fresh AppKit pixel size and Roamium to receive it
    through `ts_set_view_size`;
  - click inside the shrunken browser frame and prove hit testing uses the
    current AppKit frame, surface id, selected tab id, context id, and
    web-relative coordinates;
  - click in the former lower browser area now outside the TUI-requested
    viewport and fail if it routes to the browser context;
  - enter Browse mode and prove keyboard input reaches the same browser after
    the TUI-requested shrink;
  - return to Control mode;
  - enter command mode again, run the viewport reset command, and wait for fresh
    post-reset geometry records after that command boundary;
  - require the overlay cell geometry, AppKit frame/pixels, Roamium resize, hit
    testing, and Browse-mode keyboard routing to return to the baseline/full
    viewport behavior;
  - capture screenshots before shrink, after shrink, and after reset;
  - fail if assertions accept baseline records as post-shrink proof or
    post-shrink records as post-reset proof.
- `webtui/src/ipc.rs`
  - change only if the existing `send_set_overlay` helper cannot express the
    required TUI-requested cell rectangle.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if runtime evidence proves Ghostboard ignores or mishandles
    valid `SetOverlay` updates for an existing pane.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if runtime evidence proves AppKit frame/pixel or hit-test state
    does not update for a valid TUI-requested overlay rectangle.
- `roamium/src/dispatch.rs`
  - change only if existing trace evidence cannot prove resize/focus/key input
    after TUI overlay resize. Any such change must be trace-only under the
    existing trace mechanism.
- `issues/0809-ghostboard-viewport-geometry/21-tui-overlay-resize-command.md`
  - record the design review, implementation, verification, completion review,
    result, and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 21 to the experiment index.

Reference files:

- `webtui/src/main.rs`
- `webtui/src/ipc.rs`
- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `issues/0809-ghostboard-viewport-geometry/03-window-resize-follow.md`
- `issues/0809-ghostboard-viewport-geometry/20-font-size-cell-metrics.md`
- `ghostboard/src/apprt/termsurf.zig`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
- `roamium/src/dispatch.rs`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/21-tui-overlay-resize-command.md
  ```

- Shell syntax is valid:

  ```bash
  bash -n scripts/ghostboard-geometry-matrix.sh
  ```

- If `webtui` Rust files are changed:

  ```bash
  cargo fmt
  cargo check -p webtui
  ```

- If Roamium Rust files are changed:

  ```bash
  cargo fmt
  cargo check -p roamium
  ```

- If Zig files are changed:

  ```bash
  cd ghostboard
  zig fmt src/apprt/termsurf.zig
  zig build -Demit-macos-app=false
  ```

- If Swift files are changed:

  ```bash
  cd ghostboard
  macos/build.nu --scheme Ghostty --configuration Debug --action build
  ```

- The new scenario passes:

  ```bash
  scripts/ghostboard-geometry-matrix.sh tui-overlay-resize-command
  ```

- The passing run proves:
  - viewport shrink and reset are invoked through user-visible `webtui` command
    mode;
  - fresh post-shrink and post-reset `SetOverlay`/geometry records exist after
    their respective command boundaries;
  - the browser keeps the same window id, surface id, selected tab id, pane id,
    browser tab id, and context id after shrink and after reset;
  - post-shrink overlay cell height, AppKit frame height, and AppKit pixel
    height shrink from baseline;
  - the former lower browser area outside the shrunken viewport does not route a
    fresh hit-test to the browser context;
  - post-reset overlay cell geometry, AppKit frame/pixels, and Roamium resize
    return to baseline/full viewport behavior;
  - AppKit frame, AppKit pixels, and backing scale evidence is current and is
    not stale baseline or previous-phase evidence;
  - Roamium receives the current AppKit pixel size via `ts_set_view_size` after
    shrink and reset;
  - mouse hit-testing and Browse-mode keyboard input still route to the browser
    after shrink and after reset;
  - screenshots show baseline, shrunken TUI viewport, and reset/full viewport
    states.
- Adjacent geometry regressions still pass:

  ```bash
  scripts/ghostboard-geometry-matrix.sh font-size-cell-metrics
  scripts/ghostboard-geometry-matrix.sh window-resize
  ```

- `git diff --check` passes.
- The design review is recorded in this experiment file and the plan is
  committed before implementation begins.
- After implementation, verification, and result recording, the completion
  review is recorded in this experiment file and the result commit is made
  before designing or implementing Experiment 22.

Fail criteria:

- The harness fakes the resize by changing terminal window, pane, split,
  font-size, or private AppKit state instead of invoking a TUI command that
  emits a new `SetOverlay`.
- The browser changes window id, surface id, selected tab id, pane id, browser
  tab id, or context id across TUI overlay resize/reset.
- Current post-command geometry is ambiguous: SetOverlay, AppKit frame/pixels,
  backing scale, or Roamium resize evidence is missing or stale.
- The former lower browser area remains hit-testable after the TUI-requested
  shrink.
- Mouse or keyboard input after either TUI resize transition reaches the wrong
  browser or no browser.
- The experiment expands into scrollback, browser navigation, DevTools, or final
  matrix regression before TUI-requested overlay resizing is isolated.

## Design Review

Fresh-context adversarial review initially returned **CHANGES REQUIRED**.

Required finding:

- The design allowed trace-only `roamium/src/dispatch.rs` changes but only
  required `cargo check -p webtui`, so verification could miss Roamium Rust
  breakage.

Fix:

- Split Rust verification by crate: `cargo check -p webtui` when `webtui` Rust
  files change, and `cargo check -p roamium` when Roamium Rust files change.

Fresh-context adversarial re-review approved the design before implementation.

Verdict: **APPROVED**.

Findings: none.

## Result

**Result:** Pass

Implemented a user-visible `webtui` command-mode viewport resize path:

- `:viewport height <rows>` / `:vp height <rows>` stores an optional viewport
  inner-height override.
- `:viewport reset` / `:vp reset` clears the override and returns to full-height
  fill behavior.
- The override changes only the rendered viewport block height; the URL bar and
  status bar remain below it, with blank terminal background filling any unused
  area.
- `webtui` still emits `SetOverlay` through the existing
  `viewport_rect != last_viewport` path, so no protocol or Ghostboard product
  changes were needed.

Added `tui-overlay-resize-command` to `scripts/ghostboard-geometry-matrix.sh`.
The scenario launches one browser, drives real keyboard input through command
mode, shrinks the viewport, verifies the full geometry/input chain, resets the
viewport, and verifies the chain returns to baseline.

Passing evidence:

- Main scenario:
  - Harness log:
    `logs/ghostboard-geometry-tui-overlay-resize-command-harness-20260617-135933.log`
  - App log:
    `logs/ghostboard-geometry-tui-overlay-resize-command-app-20260617-135933.log`
  - Roamium trace:
    `logs/ghostboard-geometry-tui-overlay-resize-command-roamium-20260617-135933.log`
  - Screenshots:
    - `logs/ghostboard-geometry-tui-overlay-resize-command-screenshot-20260617-135933.png`
    - `logs/ghostboard-geometry-tui-overlay-resize-command-tui-shrink-screenshot-20260617-135933.png`
    - `logs/ghostboard-geometry-tui-overlay-resize-command-tui-reset-screenshot-20260617-135933.png`
- Adjacent regression, font metrics:
  - Harness log:
    `logs/ghostboard-geometry-font-size-cell-metrics-harness-20260617-135953.log`
- Adjacent regression, window resize:
  - Harness log:
    `logs/ghostboard-geometry-window-resize-harness-20260617-140011.log`

Key runtime facts from the passing main scenario:

- Baseline identity stayed stable: `window_id=843`,
  `surface_id=AE0CAEE2-8775-47F3-A33A-66EB3165DD30`, `selected_tab_id=843`,
  `pane_id=AE0CAEE2-8775-47F3-A33A-66EB3165DD30`, `browser_tab_id=1`,
  `context_id=1162960851`.
- Baseline grid/frame/pixels: `127x37+1+1`, `{{8, 17}, {1016, 629}}`,
  `2032x1258`.
- `:viewport height 12` produced a fresh Zig `set_overlay_update` after the
  command boundary with grid `127x12+1+1`.
- AppKit then presented the shrunken frame `{{8, 17}, {1016, 204}}` and pixels
  `2032x408`.
- Roamium received the shrunken AppKit pixel size through `ts_set_view_size`.
- Clicking inside the shrunken frame hit the same browser context and used the
  current AppKit frame.
- Clicking in the former lower browser area produced an explicit AppKit
  `hit=false` for the same context, proving stale lower-area hit testing did not
  route to the browser.
- Browse-mode keyboard input after shrink reached the same Roamium browser tab.
- `:viewport reset` produced a fresh Zig `set_overlay_update` after the reset
  boundary with grid `127x37+1+1`.
- AppKit returned to the baseline frame `{{8, 17}, {1016, 629}}` and pixels
  `2032x1258`.
- Roamium received the reset AppKit pixel size through `ts_set_view_size`.
- Mouse hit testing and Browse-mode keyboard input still worked after reset.

Verification run:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
cargo fmt
cargo check -p webtui
cargo build -p webtui
git diff --check
scripts/ghostboard-geometry-matrix.sh tui-overlay-resize-command
scripts/ghostboard-geometry-matrix.sh font-size-cell-metrics
scripts/ghostboard-geometry-matrix.sh window-resize
```

The first `window-resize` adjacent regression was accidentally run in parallel
with `font-size-cell-metrics` and failed before its resize phase because no
initial hit-test record was captured. Running `window-resize` by itself passed.
The useful learning is that GUI automation scenarios which activate apps and
drive global mouse/keyboard input should be run sequentially, not in parallel.

## Conclusion

TUI-requested overlay resizing works end to end. `webtui` can now deliberately
change its viewport rectangle through a real command-mode command, Ghostboard
accepts the resulting `SetOverlay` update for the existing pane/browser, AppKit
recomputes the native frame and pixel size, Roamium resizes to the current
AppKit pixels, stale lower-area hit testing is rejected, and keyboard routing
survives both shrink and reset.

No Ghostboard, Swift, Zig, Roamium, or protocol changes were required for this
matrix row.

## Completion Review

Fresh-context adversarial completion review approved the result before the
result commit.

Verdict: **APPROVED**.

Findings: none.

Reviewer verification:

- `bash -n scripts/ghostboard-geometry-matrix.sh` passed.
- `cargo check -p webtui` passed.
- `git diff --check` passed.
- The result commit had not been made before review: `HEAD` was still
  `c634b3047 Plan TUI viewport resize`.
- The experiment file had `## Result` and `## Conclusion`, and the issue README
  marked Experiment 21 as `Pass`.
- The command path, runtime evidence, fresh shrink/reset geometry boundaries,
  AppKit frame/pixel updates, Roamium resize, negative hit-test after shrink,
  and keyboard routing after shrink/reset matched the approved scope.
