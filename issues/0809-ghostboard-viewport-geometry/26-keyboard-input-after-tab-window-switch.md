# Experiment 26: Keyboard Input After Tab and Window Switch

## Description

Experiment 25 proved mouse hit testing and click coordinate forwarding after
representative geometry changes. The final explicit input row in the viewport
matrix is keyboard input after tab/window switch.

The goal of this experiment is to prove that keyboard input reaches only the
currently visible, focused browser pane after tab and window selection changes.
It should not re-test every geometry path. It should isolate keyboard routing
across the selection boundaries most likely to leave stale focus:

- switching from a browser tab to a plain terminal tab and back;
- switching between two tabs that each own a browser;
- switching between two windows that each own a browser;
- returning to the first browser after the second tab/window had focus.

For each active-browser step, the harness must enter Browse mode, type a unique
marker, and prove Roamium receives `key-event` records for only the active
browser tab/pane. For each inactive-browser step, the harness must type a
different marker into the active terminal/browser context and prove the inactive
browser does not receive a key event after the relevant trace boundary.

The experiment should prefer extending `scripts/ghostboard-geometry-matrix.sh`
with one focused scenario, for example `keyboard-after-tab-window-switch`,
reusing helpers and behavior already proven by Experiments 12, 13, 15, 16,
and 24. If existing scenarios already contain most of the mechanics, this
experiment should extract only small reusable helpers needed to keep the new
scenario readable.

If current Ghostboard already passes, record that and avoid product changes. If
it fails, localize whether the stale state is in AppKit focus, Zig
pane/tab/window routing, `webtui` mode handling, or Roamium event delivery
before designing any product fix.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add a `keyboard-after-tab-window-switch` scenario;
  - launch one browser in the first tab/window with repo-built `web` and
    Roamium;
  - create a plain terminal tab and prove keyboard input there does not reach
    the first browser;
  - switch back to the first browser tab, focus its current overlay frame, enter
    Browse mode, type a marker, and prove only browser A receives it;
  - create a second browser in another tab, enter Browse mode, type a marker,
    and prove only browser B receives it;
  - switch back to browser A, type a marker, and prove only browser A receives
    it;
  - create a second window with a browser, type a marker in browser C, and prove
    only browser C receives it;
  - return to the original browser window, type a marker in browser A, and prove
    only browser A receives it;
  - bracket every keyboard assertion with Roamium trace-line counts so previous
    key events cannot satisfy later assertions;
  - require matching AppKit or Ghostboard focus/mode evidence before accepting a
    Roamium key-event as active-browser proof;
  - capture screenshots at representative tab/window switch points;
  - fail if any inactive browser receives a key event after the relevant trace
    boundary.
- `roamium/src/dispatch.rs`
  - change only if current trace output cannot distinguish key-event tab id,
    pane id, and marker content or timing;
  - any change must be trace-only under the existing trace mechanism.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if runtime evidence proves Ghostboard routes keyboard events to
    a stale browser tab, pane, or window.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if runtime evidence proves AppKit focus or key dispatch is stale
    after tab/window switching.
- `issues/0809-ghostboard-viewport-geometry/26-keyboard-input-after-tab-window-switch.md`
  - record the design review, implementation, verification, completion review,
    result, and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 26 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `issues/0809-ghostboard-viewport-geometry/12-new-terminal-tab-visibility.md`
- `issues/0809-ghostboard-viewport-geometry/13-open-browser-in-new-tab.md`
- `issues/0809-ghostboard-viewport-geometry/15-open-browser-in-new-window.md`
- `issues/0809-ghostboard-viewport-geometry/16-multiple-windows-with-browsers.md`
- `issues/0809-ghostboard-viewport-geometry/24-devtools-split-geometry.md`
- `ghostboard/src/apprt/termsurf.zig`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
- `roamium/src/dispatch.rs`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/26-keyboard-input-after-tab-window-switch.md
  ```

- Shell syntax is valid:

  ```bash
  bash -n scripts/ghostboard-geometry-matrix.sh
  ```

- If Rust files are changed:

  ```bash
  cargo fmt
  cargo check -p roamium
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

- If the copied Roamium binary is needed by the harness after Rust changes:

  ```bash
  scripts/build.sh roamium
  ```

- The new keyboard scenario passes:

  ```bash
  scripts/ghostboard-geometry-matrix.sh keyboard-after-tab-window-switch
  ```

- The passing run proves:
  - keyboard markers typed in a plain terminal tab do not reach browser A;
  - after switching back to browser A, marker A reaches only browser A;
  - after switching to browser B in another tab, marker B reaches only browser
    B;
  - after switching back to browser A from browser B, marker A2 reaches only
    browser A;
  - after switching to browser C in another window, marker C reaches only
    browser C;
  - after returning to browser A's window, marker A3 reaches only browser A;
  - every positive and negative keyboard assertion is bounded by a fresh trace
    line count;
  - focus/mode evidence exists before accepting a positive browser key event.
- Adjacent regressions still pass:

  ```bash
  scripts/ghostboard-geometry-matrix.sh new-terminal-tab-visibility
  scripts/ghostboard-geometry-matrix.sh open-browser-in-new-tab
  scripts/ghostboard-geometry-matrix.sh open-browser-in-new-window
  ```

- `git diff --check` passes.
- The design review is recorded in this experiment file and the plan is
  committed before implementation begins.
- After implementation, verification, and result recording, the completion
  review is recorded in this experiment file and the result commit is made
  before designing or implementing the next experiment.

Fail criteria:

- The scenario accepts a Roamium key-event from before the tab/window switch as
  evidence for the current active browser.
- Keyboard evidence does not distinguish browser tab id and pane id.
- The scenario proves only positive delivery and does not prove inactive
  browsers stayed silent.
- The scenario relies on installed apps/binaries instead of repo-built `web` and
  Roamium.
- The experiment expands into mouse behavior, final full-matrix regression, or
  unrelated product changes before keyboard routing after tab/window switch is
  isolated.

## Design Review

Fresh-context adversarial design review returned **APPROVED**.

Findings: none.

Read-only checks performed by the reviewer:

- `git status --short`
- `git diff --check`
- `bash -n scripts/ghostboard-geometry-matrix.sh`
- targeted inspection of the issue README, experiment design, harness patterns,
  Roamium key trace format, Zig forwarding, and Swift key forwarding
