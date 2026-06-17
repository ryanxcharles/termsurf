# Experiment 14: Close Browser Tab

## Description

Experiment 13 proved that Ghostboard can host two independent browser overlays
across two native tabs in one window. Browser A belongs to native tab 1, browser
B belongs to native tab 2, and switching between tabs preserves each browser's
pane id, browser tab id, CA/context id, AppKit frame, hit testing, focus, and
keyboard isolation.

The next matrix row is closing a browser tab. This experiment should close the
native tab that owns browser B and prove the closed tab's browser overlay,
TermSurf pane state, native AppKit layer, and Roamium browser tab are cleaned up
without disturbing browser A in the surviving native tab.

This experiment intentionally covers exactly one window with two native tabs,
where each tab contains one browser overlay. It should close native tab 2 while
tab 2 is selected, then prove tab 1 remains alive and browser A remains visible,
hit-testable, and keyboard-routable. It does not test closing the last tab,
closing multiple tabs, closing tabs to the right, reopening closed tabs,
multiple windows, or DevTools tabs.

If current Ghostboard already passes, the experiment should record that and
avoid product source changes. If it fails, the harness must first localize
whether the failure is tab-close keybinding precedence, TermSurf pane cleanup,
native layer cleanup, Roamium `CloseTab` delivery, stale hit testing, or
surviving-tab focus/visibility before any product fix is designed.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add a `close-browser-tab` scenario;
  - reuse the two-native-tab/two-browser setup shape from
    `open-browser-in-new-tab`;
  - add scenario-local config:
    - `confirm-close-surface = false`;
    - any required close-tab confirmation config if Ghostty distinguishes tab
      close confirmation from surface close confirmation;
  - add scenario-local keybindings:
    - `keybind = ctrl+t=new_tab`;
    - `keybind = ctrl+1=goto_tab:1`;
    - `keybind = ctrl+2=goto_tab:2`;
    - `keybind = ctrl+p=previous_tab`;
    - `keybind = ctrl+n=next_tab`;
    - `keybind = ctrl+w=close_tab`;
  - launch browser A in tab 1 using the repo-built `web` and Roamium binaries;
  - create native tab 2 and launch browser B by typing the real repo-built
    `web --browser ... https://example.org` command into tab 2's shell;
  - record browser A and browser B identity tuples:
    `selected_tab_id + pane_id + browser_tab_id + context_id + AppKit frame`;
  - prove browser B is visible, hit-testable, and keyboard-routable before
    close;
  - ensure browser B is in Control mode before invoking the close-tab keybinding
    so the close key is handled by Ghostboard UI bindings, not forwarded to
    Roamium as browser input;
  - invoke `ctrl+w` while native tab 2 is selected;
  - fail if the only observed effect is a Roamium browser key event for
    Control-W, with no native tab close or TermSurf cleanup evidence;
  - wait for close evidence after the close boundary:
    - selected native tab changes away from browser B's selected tab id;
    - Zig records cleanup for browser B's pane id;
    - Zig sends `CloseTab` for browser B's browser tab id;
    - Roamium trace records `close-tab` receipt/destruction for browser B;
    - AppKit or the bridge records overlay clear/target cleanup for browser B,
      or records a clear rejection that proves the surface was already removed;
  - prove browser B's former selected tab id is gone from the active tab group
    or at least no longer selectable by public keybindings;
  - prove browser A is still visible in tab 1 with its own pane id, browser tab
    id, context id, AppKit frame, and AppKit pixels;
  - prove clicking and typing in browser A after tab B closes routes only to
    browser A and not browser B;
  - capture screenshots before close, after close, and after browser A is
    revalidated;
  - fail if any assertion accepts pre-close AppKit, Zig, Roamium, or hit-test
    records as post-close cleanup proof.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if runtime evidence proves closing a native tab does not call
    `paneClosed` / TUI cleanup, does not send `CloseTab`, leaves stale pane
    state, or leaves stale input routing.
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
  - change only if runtime evidence proves native tab close removes the tab
    without notifying TermSurf about each surface/pane.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if runtime evidence proves close-tab keybindings are swallowed
    by browser key forwarding, native overlay layers are not cleared, or hit
    testing remains live after the owning native tab is closed.
- `roamium/src/dispatch.rs`
  - change only if existing run-specific tracing cannot prove `CloseTab` receipt
    and browser-tab destruction. Any such change must be trace-only under the
    existing trace mechanism.
- `issues/0809-ghostboard-viewport-geometry/14-close-browser-tab.md`
  - record the design review, implementation, verification, completion review,
    result, and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 14 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `issues/0809-ghostboard-viewport-geometry/13-open-browser-in-new-tab.md`
- `issues/0809-ghostboard-viewport-geometry/10-close-browser-pane.md`
- `ghostboard/src/apprt/termsurf.zig`
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
- `ghostboard/src/input/command.zig`
- `ghostboard/src/input/Binding.zig`
- `roamium/src/dispatch.rs`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/14-close-browser-tab.md
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
  swiftlint lint --strict --fix \
    "macos/Sources/Features/Terminal/TerminalController.swift" \
    "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"
  swiftlint lint --strict \
    "macos/Sources/Features/Terminal/TerminalController.swift" \
    "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"
  macos/build.nu --scheme Ghostty --configuration Debug --action build
  ```

- If Rust files are changed:

  ```bash
  cargo fmt
  cargo check -p roamium
  ```

- If only the harness/docs change, the already-built app may be reused, but the
  final result must state whether any product build was or was not needed.
- Existing adjacent scenarios pass serially:

  ```bash
  scripts/ghostboard-geometry-matrix.sh open-browser-in-new-tab
  scripts/ghostboard-geometry-matrix.sh new-terminal-tab-visibility
  ```

- The new scenario passes:

  ```bash
  scripts/ghostboard-geometry-matrix.sh close-browser-tab
  ```

- The passing `close-browser-tab` run proves all of the following:
  - browser A and browser B initially open with distinct pane ids, browser tab
    ids, CA/context ids, selected native tab ids, and AppKit overlay identities;
  - browser B is visible, hit-testable, focusable, and keyboard-routable while
    native tab 2 is selected before close;
  - browser B is returned to Control mode before the close-tab keybinding is
    sent;
  - `ctrl+w` invokes native tab close behavior while tab 2 is selected;
  - the close-tab keybinding is not swallowed as browser input;
  - after the close boundary, Zig cleanup references browser B's pane id and
    browser tab id;
  - after the close boundary, Zig sends `CloseTab` for browser B's browser tab
    id;
  - after the close boundary, Roamium receives and destroys browser B's browser
    tab;
  - after the close boundary, AppKit/bridge cleanup evidence proves browser B's
    native overlay is gone or its surface was already removed;
  - tab 2 is no longer selected or selectable as a live native tab;
  - browser A remains visible, hit-testable, focusable, and keyboard-routable
    with its original identity tuple and tab-bar-adjusted geometry;
  - keyboard input after browser B's tab closes reaches browser A only and does
    not reach browser B;
  - screenshots show browser B before close and browser A after close with no
    stale browser B overlay.
- `git diff --check` passes.
- The design review is recorded in this experiment file and the plan is
  committed before implementation begins.
- After implementation, verification, and result recording, the completion
  review is recorded in this experiment file and the result commit is made
  before designing or implementing Experiment 15.

Fail criteria:

- The harness closes a private Ghostboard state object instead of invoking a
  user-visible native tab close path.
- The harness accepts browser B's pre-close records as proof of post-close
  cleanup.
- The harness accepts browser A's surviving records as proof that browser B was
  cleaned up.
- Browser B remains visible, hit-testable, keyboard-focusable, or present in
  Roamium after its native tab closes.
- Closing browser B's native tab also removes, hides, corrupts, or reroutes
  browser A.
- The experiment expands into closing the last tab, closing multiple tabs,
  reopening closed tabs, closing windows, DevTools, or multi-window behavior
  before this one-tab-close case is proven.

## Design Review

The design was reviewed by a fresh-context Codex adversarial subagent.

Final verdict: **Approved**.

Findings:

- None.

The reviewer confirmed that the README links Experiment 14 as `Designed`, the
experiment has Description, Changes, and Verification sections, the scope
matches the next matrix row rather than redoing browser-pane close, and the
verification covers native tab close cleanup, browser B teardown, browser A
survival, input isolation, stale overlay checks, formatting/build hygiene,
adjacent scenarios, `git diff --check`, the design/plan commit gate, and the
completion/result commit gate.
