# Experiment 15: Open Browser in New Window

## Description

Experiments 12 through 14 proved browser overlay behavior across native tabs in
one Ghostboard window. The next untested matrix row is opening a browser in a
new native window.

This experiment should prove that a second Ghostboard window can host its own
browser overlay without reusing, showing, hiding incorrectly, or stealing input
from the browser overlay in the first window. Browser A starts in window A from
the harness initial command. The harness then invokes the user-visible
`new_window` action, types a real repo-built `web --browser ...` command into
window B's shell, and proves browser B is isolated to window B while browser A
remains isolated to window A.

This experiment intentionally covers exactly two native windows with one browser
overlay in each window. It does not test multiple browser panes per window,
closing windows, moving windows between displays, fullscreen, minimize/hide, or
DevTools.

If current Ghostboard already passes, the experiment should record that and
avoid product source changes. If it fails, the harness must first localize
whether the failure is new-window action dispatch, inherited initial command,
window identity mapping, overlay visibility gating, stale hit testing, keyboard
focus routing, or Roamium browser-tab state before any product fix is designed.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add an `open-browser-in-new-window` scenario;
  - add scenario-local config:
    - keep `window-save-state = never`;
    - keep the first-run wrapper so only window A runs the initial `web` command
      automatically;
    - prove window B starts as a plain terminal shell and does not inherit the
      initial `web` wrapper;
  - add a scenario-local keybinding:
    - `keybind = ctrl+b=new_window`;
  - launch browser A in window A using the repo-built `web` and Roamium
    binaries;
  - record browser A's identity tuple:
    `selected_tab_id + pane_id + browser_tab_id + context_id + window_id + AppKit frame + AppKit pixels`;
  - invoke the public `new_window` action through the configured keybinding;
  - wait for a second native window id after the action boundary, and fail if
    only the original window remains visible;
  - activate and click window B through the same AppKit/CGEvent automation used
    by earlier scenarios;
  - type and submit a full repo-built `web --browser ... https://example.org`
    command in window B's shell;
  - wait for browser B geometry records after the command boundary and require a
    distinct `window_id`, `pane_id`, `browser_tab_id`, and `context_id`;
  - prove browser B's AppKit frame, presented pixels, Roamium resize,
    screenshot, hit-test coordinates, Browse-mode focus, and keyboard input all
    belong to window B;
  - prove browser A is not freshly presented as visible under window B's window
    id and that clicks/typing in window B do not reach browser A;
  - reactivate window A and prove browser A remains visible, hit-testable,
    focusable, and keyboard-routable with its original identity tuple;
  - prove keyboard input after returning to window A reaches browser A only and
    does not reach browser B;
  - capture screenshots for window A before opening window B, window B after
    opening browser B, and window A after reactivation;
  - fail if any assertion accepts browser A records as proof of browser B, or
    accepts pre-new-window records as post-new-window evidence.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if runtime evidence proves the TermSurf pane/browser mapping is
    not window-scoped or sends incorrect `window_id` / selected-tab identity.
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
  - change only if runtime evidence proves new-window creation does not create
    an independently identifiable/focusable terminal surface for TermSurf.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if runtime evidence proves native overlay visibility, layer
    lookup, hit testing, or keyboard focus is not scoped to the active window.
- `roamium/src/dispatch.rs`
  - change only if existing run-specific tracing cannot prove browser B resize,
    focus, and keyboard input. Any such change must be trace-only under the
    existing trace mechanism.
- `issues/0809-ghostboard-viewport-geometry/15-open-browser-in-new-window.md`
  - record the design review, implementation, verification, completion review,
    result, and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 15 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `scripts/ghostty-app/winid.swift`
- `issues/0809-ghostboard-viewport-geometry/13-open-browser-in-new-tab.md`
- `issues/0809-ghostboard-viewport-geometry/14-close-browser-tab.md`
- `ghostboard/src/input/command.zig`
- `ghostboard/src/input/Binding.zig`
- `ghostboard/src/config/Config.zig`
- `ghostboard/src/apprt/termsurf.zig`
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
- `ghostboard/macos/Sources/Features/AppleScript/AppDelegate+AppleScript.swift`
- `roamium/src/dispatch.rs`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/15-open-browser-in-new-window.md
  ```

- Shell syntax is valid:

  ```bash
  bash -n scripts/ghostboard-geometry-matrix.sh
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

- If Rust files are changed:

  ```bash
  cargo fmt
  cargo check -p roamium
  ```

- The new scenario passes:

  ```bash
  scripts/ghostboard-geometry-matrix.sh open-browser-in-new-window
  ```

- Adjacent tab and focus regressions still pass:

  ```bash
  scripts/ghostboard-geometry-matrix.sh open-browser-in-new-tab
  scripts/ghostboard-geometry-matrix.sh split-right-focus-switch
  ```

- The passing `open-browser-in-new-window` run proves:
  - window B is created through the user-visible `new_window` action;
  - window B does not inherit the first-run browser command automatically;
  - browser A and browser B have distinct native window ids, pane ids, browser
    tab ids, and context ids;
  - browser B appears only in window B, with AppKit frame, AppKit pixels,
    Roamium resize, screenshot, and hit-test evidence tied to window B;
  - browser A remains isolated to window A and is not visible, hit-testable, or
    keyboard-routable from window B;
  - returning to window A restores browser A hit testing, Browse-mode focus, and
    keyboard input without sending input to browser B;
  - screenshots show browser A in window A and browser B in window B with no
    stale or cross-window overlay.
- `git diff --check` passes.
- The design review is recorded in this experiment file and the plan is
  committed before implementation begins.
- After implementation, verification, and result recording, the completion
  review is recorded in this experiment file and the result commit is made
  before designing or implementing Experiment 16.

Fail criteria:

- The harness creates browser B through private state mutation instead of the
  public new-window path plus a real typed `web` command.
- Window B inherits the harness initial `web` command automatically.
- Browser B reuses browser A's window id, pane id, browser tab id, or context
  id.
- Browser A is visible, hit-testable, focused, or keyboard-routable while window
  B is the active target.
- Browser B is visible, hit-testable, focused, or keyboard-routable while window
  A is the active target.
- The harness accepts pre-new-window AppKit, Zig, Roamium, or hit-test records
  as proof of window B behavior.
- The experiment expands into closing windows, multiple windows with multiple
  browser panes, display moves, fullscreen, minimize/hide, DevTools, or final
  matrix regression before this one-browser-per-window case is proven.

## Design Review

Fresh-context adversarial review approved the design before implementation.

Initial verdict: **CHANGES REQUIRED**.

Required finding:

- The Rust verification block listed `cargo fmt` and
  `./scripts/build.sh webtui`, but the only planned Rust source file was
  `roamium/src/dispatch.rs`. That would not prove a Roamium trace-only change
  builds.

Fix:

- Changed the Rust verification block to run `cargo fmt` and
  `cargo check -p roamium` if Rust files are changed.

Final verdict: **APPROVED**.

Findings after re-review: none.
