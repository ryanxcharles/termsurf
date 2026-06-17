# Experiment 10: Close browser-owning pane

## Description

Experiment 9 proved the resize side of pane close: when a non-browser sibling
closes, the browser-owning pane expands and continues to receive input in the
reclaimed space. The next viewport matrix row is the opposite cleanup case:
closing the pane that owns the browser.

This experiment should start with a browser in the left pane of a split-right
layout. It should focus the browser-owning pane, close that pane through normal
Ghostty/Ghostboard keybinding behavior, and prove the browser overlay and
browser tab are cleaned up instead of leaving a stale native layer, stale pane
mapping, or stale input route:

- open a browser in a single pane;
- create a right-side split from the browser-owning pane;
- focus the browser-owning pane with deterministic mouse input;
- invoke a scenario-local `close_surface` keybinding while the browser-owning
  pane is focused;
- prove the close keybinding is handled as Ghostty/Ghostboard UI input instead
  of being forwarded to Roamium as browser input, or localize and fix the
  keybinding-precedence bug before cleanup assertions are allowed to pass;
- prove the close action uses user-visible keybinding behavior with close
  confirmation disabled by scenario-local config;
- prove Zig sends a `clear_overlay_call` for the closed browser pane;
- prove the Swift bridge receives the clear request and either finds the target
  surface or records a reason why the surface was already gone;
- prove AppKit clears the native overlay when the target surface is still
  present;
- prove Zig sends `CloseTab` to Roamium for the closed browser tab;
- prove Roamium receives and handles `CloseTab` for the closed browser tab;
- prove the former browser area no longer routes input to the old browser
  context after close;
- prove the remaining sibling pane is still present and usable enough for the
  scenario to distinguish "browser pane closed" from "whole window died."

This experiment intentionally covers only closing the browser-owning pane in a
two-pane split-right layout. Tab close, window close, undo/redo restore,
fullscreen, multi-window behavior, and cleanup of multiple simultaneous browser
overlays remain later matrix rows.

If current Ghostboard already passes, the experiment should record that and
avoid product source changes. If it fails, the harness must first localize which
cleanup invariant failed before any Ghostboard fix is designed in this
experiment. Relevant reference audits, if a fix is needed, should include the
current upstream Ghostty close path, `ghostboard-legacy/`, and the
already-solved Wezboard cleanup behavior.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add a `split-right-close-browser-pane` scenario;
  - add scenario-local close behavior:
    - `confirm-close-surface = false`;
  - add scenario-local keybindings:
    - `keybind = ctrl+d=new_split:right`;
    - `keybind = ctrl+k=close_surface`;
  - launch the same repo-built `TermSurf.app`, `target/debug/web`, and Roamium
    trace setup as the existing scenarios;
  - wait for the initial-open AppKit/Zig/Roamium correlation to pass;
  - inject Control-D to create the right split and wait for the same post-split
    proof used by prior split-right scenarios;
  - focus the original browser-owning pane with real mouse input before invoking
    close, because `new_split:right` leaves focus on the newly created sibling
    pane;
  - record the browser pane id, browser tab id, context id, app-log line count,
    and Roamium trace line count before close;
  - record a key-forwarding boundary before close, then fail if Control-K is
    forwarded to Roamium as a browser `KeyEvent` instead of producing close
    cleanup evidence;
  - inject Control-K to invoke `close_surface` on the focused browser-owning
    pane;
  - if the first run proves Control-K is swallowed by browser input forwarding,
    record that failure and localize the fix to configured keybinding precedence
    before browser-key forwarding for TermSurf browsing panes;
  - wait for cleanup evidence after the close-key boundary:
    - Zig `clear_overlay_call` for the browser pane id;
    - Swift bridge `clear_request` for the browser pane id;
    - Swift bridge `clear_target_found` plus AppKit `event=clear` when the
      surface still exists, or a documented bridge `clear_rejected` reason when
      the close path removes the surface before the async clear reaches AppKit;
    - Zig `CloseTab` for the browser pane id and browser tab id;
    - Roamium run-specific trace evidence that the closed tab was received and
      destroyed;
  - capture a post-close-browser-pane screenshot;
  - send deterministic mouse input in the former browser-pane area and require
    no fresh hit-test route to the old browser context after close;
  - send deterministic mouse input in the remaining sibling-pane area and
    require no fresh hit-test route to the old browser context after close;
  - prove the remaining sibling pane is still alive with a positive signal such
    as a focus-change record to the sibling surface, terminal input echo in the
    post-close screenshot, or another concrete app-log event tied to the sibling
    surface after the browser pane closes;
  - fail if the harness accepts any pre-close AppKit, Zig, Roamium, or hit-test
    record as post-close cleanup proof.
- `roamium/src/dispatch.rs`
  - change only if existing tracing does not expose `CloseTab` receipt and tab
    destruction;
  - any change must be trace-only under the existing run-specific
    `TERMSURF_PDF_INPUT_TRACE` mechanism, with no behavior change to tab
    cleanup.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if the harness proves Zig state cleanup, clear-overlay,
    CloseTab, or stale input routing fails;
  - likely candidate fixes should be localized from the geometry logs before
    implementation.
- `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`
  - change only if the bridge cannot reliably report clear-target/clear-rejected
    cleanup evidence.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if configured close keybindings are swallowed by TermSurf
    browser key forwarding, AppKit does not clear the native overlay, or AppKit
    leaves stale hit-test routing after the browser-owning pane closes.
- `issues/0809-ghostboard-viewport-geometry/10-close-browser-pane.md`
  - record the design, implementation, verification, completion review, result,
    and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 10 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `ghostboard/src/input/Binding.zig`
- `ghostboard/src/config/Config.zig:6514`
- `ghostboard/src/config/Config.zig:6880`
- `ghostboard/src/Surface.zig:5807`
- `ghostboard/src/apprt/termsurf.zig:1423-1447`
- `ghostboard/src/apprt/termsurf.zig:1579-1590`
- `ghostboard/src/apprt/termsurf.zig:1852-1920`
- `roamium/src/dispatch.rs:207-213`
- `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift:31-66`
- `ghostboard/macos/Sources/Features/Terminal/BaseTerminalController.swift:390-470`
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift:148-176`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift:620-632`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift:1665-1681`
- `issues/0809-ghostboard-viewport-geometry/09-close-split-right-sibling.md`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/10-close-browser-pane.md
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

- If Rust files are changed:

  ```bash
  cargo fmt
  cargo check -p roamium
  ```

- If Swift files are changed:

  ```bash
  cd ghostboard
  swiftlint lint --strict --fix \
    "macos/Sources/App/macOS/AppDelegate+TermSurf.swift" \
    "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"
  swiftlint lint --strict \
    "macos/Sources/App/macOS/AppDelegate+TermSurf.swift" \
    "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"
  macos/build.nu --scheme Ghostty --configuration Debug --action build
  ```

- If only the harness/docs change, the already-built app may be reused, but the
  final result must still state whether any product build was or was not needed.
- Existing adjacent scenarios still pass serially:

  ```bash
  scripts/ghostboard-geometry-matrix.sh initial-open
  scripts/ghostboard-geometry-matrix.sh split-right
  scripts/ghostboard-geometry-matrix.sh split-right-close-sibling
  ```

- The new scenario passes:

  ```bash
  scripts/ghostboard-geometry-matrix.sh split-right-close-browser-pane
  ```

- The `split-right-close-browser-pane` passing run proves all of the following:
  - initial-open still correlates AppKit, Zig, Roamium, screenshot, and hit
    test;
  - close confirmation is disabled by scenario-local config
    `confirm-close-surface = false`, so Control-K closes the focused browser
    pane directly instead of opening a confirmation dialog;
  - the split action is triggered by the scenario-local `ctrl+d` keybinding;
  - the browser pane is focused by deterministic mouse input before close;
  - the browser-pane close action is triggered by the scenario-local `ctrl+k`
    keybinding;
  - the close keybinding is not swallowed by browser input forwarding: after the
    close-key boundary the run must show close cleanup evidence and must not
    show a Roamium browser-key trace for Control-K as the only effect;
  - Zig records `clear_overlay_call` for the closed browser pane id after the
    close-key boundary;
  - the Swift bridge records `clear_request` for the same pane id after the
    close-key boundary;
  - AppKit records `event=clear` for the same pane id, or the bridge records a
    post-close `clear_rejected` reason proving the surface was already gone
    before the async clear could target it;
  - Zig records `CloseTab` for the same pane id and browser tab id after the
    close-key boundary;
  - the run-specific Roamium trace records `CloseTab` receipt and tab
    destruction for the same browser tab id after the close-key boundary;
  - the post-close-browser-pane screenshot shows the remaining sibling terminal
    pane, not the stale browser overlay;
  - hit testing in the former browser-pane area does not route to the old
    browser context after close;
  - hit testing in the remaining sibling-pane area does not route to the old
    browser context after close.
  - the remaining sibling pane emits a positive post-close survival signal, such
    as focus-change evidence, terminal input echo in the screenshot, or another
    concrete app-log event tied to the sibling surface.
- `git diff --check` passes.

Fail criteria:

- The harness closes the browser pane by calling a private Ghostboard API
  instead of exercising user-visible keybinding behavior.
- The harness leaves close confirmation enabled and therefore risks proving a
  confirmation dialog rather than a pane close.
- The test accepts pre-close AppKit, Zig, Roamium, or hit-test records as proof
  of browser-pane cleanup.
- The browser pane remains visible after close, leaves a stale AppKit layer,
  leaves a stale browser tab mapping, or still receives hit-test/input routing
  after close.
- Control-K is forwarded to Roamium as browser input and no browser-pane close
  cleanup follows.
- The test proves only Zig-side `CloseTab` send intent without Roamium-side
  receipt/destruction evidence.
- The whole window closes before the harness can distinguish browser-pane close
  from sibling-pane survival.
- The experiment expands into tab close, window close, undo/redo restore,
  fullscreen, or multi-window behavior before browser-pane close cleanup is
  proven.

## Design Review

The first design review was performed by a fresh-context Codex adversarial
subagent.

Verdict: **Changes required**.

Findings:

- Required: the plan assumed `ctrl+k=close_surface` would reach Ghostty while
  the browser-owning pane was focused, but browser-focused keydown handling can
  forward the event to Roamium before Ghostty keybindings run. Evidence cited by
  the reviewer: `SurfaceView_AppKit.swift:1326-1329` returns immediately when
  `forwardTermSurfKeyDown` succeeds, and `termsurf.zig:1423-1447` forwards key
  events for browsing panes.
- Required: Roamium-side browser tab cleanup was optional even though the
  experiment claims to prove browser tab cleanup. Evidence cited by the
  reviewer: `roamium/src/dispatch.rs:207-213` handles `Msg::CloseTab` silently
  with no trace.
- Optional: the sibling-survival proof was weaker than the stated goal because
  negative hit testing only proves "not the old browser context," not that the
  remaining terminal pane is alive.

Fixes:

- Added an explicit keybinding-precedence boundary: the run must prove Control-K
  is handled as the configured `close_surface` keybinding rather than being
  swallowed as browser input, or the experiment must record that failure and fix
  keybinding precedence before cleanup assertions can pass.
- Made Roamium-side `CloseTab` receipt/destruction evidence mandatory, with any
  needed Roamium change constrained to trace-only instrumentation under the
  existing run-specific trace mechanism.
- Added a positive sibling-survival requirement after browser-pane close, such
  as focus-change evidence, terminal input echo in the screenshot, or another
  concrete app-log event tied to the sibling surface.

The fixed design was re-reviewed by the same fresh-context Codex adversarial
subagent.

Final verdict: **Approved**.

The reviewer confirmed that the keybinding-precedence proof, mandatory
Roamium-side `CloseTab` evidence, and positive sibling-survival requirement
resolved the prior findings and reported no new Required findings.
