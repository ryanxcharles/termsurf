# Experiment 23: Browser Navigation Geometry

## Description

Experiment 22 proved that terminal scrollback movement does not move, resize, or
stale the browser overlay. The next matrix row is browser navigation.

Browser navigation should change the page loaded inside the existing browser
tab, not the ownership or geometry of the native overlay. After navigating from
the initial URL to a second URL through the normal `webtui` URL edit workflow,
Ghostboard should keep the same window, surface, selected tab, pane, browser
tab, context id, AppKit frame, AppKit pixels, backing scale, and input routing.

This experiment should isolate one window with one browser overlay. It must
drive navigation through public user behavior: Control mode `i` to edit the URL,
real typed URL text, and Enter to submit. It should not inject a protocol
message directly, call private Roamium APIs, change the window, change splits,
change font size, change scrollback, or use DevTools. If current Ghostboard
already passes, the experiment should record that and avoid product changes. If
it fails, the harness must first localize whether the failure is URL edit input,
`Navigate` delivery, Roamium navigation, `UrlChanged` propagation, overlay
geometry drift, stale hit testing, or keyboard focus routing before any product
fix is designed.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add a `browser-navigation-geometry` scenario;
  - launch one browser in one Ghostboard window using the repo-built `web` and
    Roamium binaries;
  - record the baseline canonical identity tuple:
    `window_id + surface_id + selected_tab_id + pane_id + browser_tab_id`, plus
    `context_id + grid + cell size + AppKit frame + AppKit pixels + backing_scale`;
  - capture a baseline screenshot;
  - enter URL edit mode using real keyboard input (`i` from Control mode);
  - replace or select the current URL through normal editor input, type a second
    URL, and submit it with Enter;
  - wait for fresh navigation evidence after the submit boundary, such as
    Roamium `title-changed`/URL trace, `UrlChanged` handling, or existing
    Ghostboard protocol logs that prove the same browser tab loaded the second
    URL;
  - require the same canonical identity and context id after navigation;
  - require AppKit frame, AppKit pixels, backing scale, and Roamium view size to
    stay equal to baseline after navigation;
  - fail if navigation emits a fresh Roamium resize, unless the resize exactly
    matches the already-current AppKit pixel size and can be explained by normal
    idempotent synchronization;
  - click inside the current overlay frame after navigation and prove hit
    testing still uses the baseline AppKit frame, surface id, selected tab id,
    context id, and web-relative coordinates;
  - enter Browse mode after navigation and prove keyboard input reaches the same
    browser tab and pane;
  - capture a post-navigation screenshot;
  - fail if assertions accept pre-navigation records as post-navigation proof.
- `roamium/src/dispatch.rs`
  - change only if existing trace output cannot prove navigation happened on the
    intended tab and pane;
  - any change must be trace-only under the existing trace mechanism.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if runtime evidence proves Ghostboard fails to route valid
    `Navigate` or `UrlChanged` messages for the existing browser tab.
- `webtui/src/main.rs`
  - change only if runtime evidence proves the public URL edit workflow cannot
    be automated deterministically or cannot send a valid `Navigate` message;
  - any change must be user-visible and not a hidden test hook.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if runtime evidence proves AppKit frame/pixel or hit-test state
    changes incorrectly during valid browser navigation.
- `issues/0809-ghostboard-viewport-geometry/23-browser-navigation-geometry.md`
  - record the design review, implementation, verification, completion review,
    result, and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 23 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `webtui/src/main.rs`
- `roamium/src/dispatch.rs`
- `ghostboard/src/apprt/termsurf.zig`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
- `issues/0809-ghostboard-viewport-geometry/22-terminal-scrollback-movement.md`
- `issues/0809-ghostboard-viewport-geometry/21-tui-overlay-resize-command.md`
- `issues/0809-ghostboard-viewport-geometry/03-window-resize-follow.md`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/23-browser-navigation-geometry.md
  ```

- Shell syntax is valid:

  ```bash
  bash -n scripts/ghostboard-geometry-matrix.sh
  ```

- If Rust files are changed:

  ```bash
  cargo fmt
  cargo check -p webtui
  cargo check -p roamium
  cargo build -p webtui
  cargo build -p roamium
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
  scripts/ghostboard-geometry-matrix.sh browser-navigation-geometry
  ```

- The passing run proves:
  - navigation is invoked through public `webtui` URL edit behavior using real
    keyboard input;
  - the second URL loads in the same browser tab and pane, with fresh
    post-submit evidence after the navigation boundary;
  - the browser keeps the same window id, surface id, selected tab id, pane id,
    browser tab id, and context id after navigation;
  - AppKit frame, AppKit pixels, backing scale, and Roamium view size remain
    equal to baseline after navigation;
  - any resize trace after navigation is either absent or explicitly proven to
    be an idempotent resize to the current AppKit pixel size;
  - mouse hit testing after navigation uses the current/baseline overlay frame
    and routes to the correct context with web-relative coordinates;
  - Browse-mode keyboard input after navigation reaches the same browser tab and
    pane;
  - screenshots show baseline and post-navigation states.
- Adjacent geometry regressions still pass:

  ```bash
  scripts/ghostboard-geometry-matrix.sh terminal-scrollback-movement
  scripts/ghostboard-geometry-matrix.sh window-resize
  ```

- `git diff --check` passes.
- The design review is recorded in this experiment file and the plan is
  committed before implementation begins.
- After implementation, verification, and result recording, the completion
  review is recorded in this experiment file and the result commit is made
  before designing or implementing Experiment 24.

Fail criteria:

- The harness fakes navigation by injecting TermSurf protobuf directly, calling
  private browser APIs, changing the initial launch URL instead of navigating,
  or using DevTools.
- Navigation cannot be proven with fresh evidence after the submit boundary.
- The browser changes window id, surface id, selected tab id, pane id, browser
  tab id, or context id across navigation.
- AppKit frame, AppKit pixels, backing scale, or Roamium size drift during
  navigation.
- Mouse or keyboard input after navigation reaches the wrong browser, no
  browser, or stale coordinates.
- The experiment expands into DevTools, tab/window switching, split changes,
  scrollback movement, or final matrix regression before browser navigation is
  isolated.

## Design Review

Fresh-context adversarial design review returned **APPROVED**.

Findings: none.
