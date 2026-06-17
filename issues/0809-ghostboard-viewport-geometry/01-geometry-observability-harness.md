# Experiment 1: Geometry observability harness

## Description

Issue 809 cannot be solved safely by fixing the first visible size bug in
isolation. Before changing overlay behavior, Ghostboard needs durable geometry
observability and a repeatable matrix harness so every later experiment can
prove which invariant failed and whether the fix regressed adjacent cases.

This experiment will add instrumentation and harness scaffolding only. It will
not attempt to fix the 2026-06-17 viewport-fill failure.

The canonical geometry record should capture:

- `window_id`;
- `surface_id`;
- `selected_tab_id`;
- `pane_id`;
- `browser_tab_id`;
- terminal pane viewport rectangle;
- TUI overlay cell rectangle;
- native AppKit/CALayerHost root, positioning, and host frames;
- Roamium/browser pixel viewport size;
- backing scale factor;
- overlay visibility state;
- input hit-test frame and webview-relative coordinates.

## Changes

Planned files:

- `ghostboard/src/apprt/termsurf.zig`
  - add structured `TermSurf geometry` trace logs for Zig-side overlay state:
    `SetOverlay`, `CaContext`, `PresentOverlay`, `ClearOverlay`, tab/pane
    lookup, and resize-relevant snapshots;
  - include the fields available on the Zig side: pane id, browser tab id,
    overlay grid rectangle, browser pixel size, context id, and whether the
    overlay is eligible to present;
  - include every canonical identity field that Zig can know directly, and log
    explicit `unknown:<field>` placeholders with reasons for AppKit-only fields
    such as window or surface ids until bridge/AppKit records fill them in.
  - gate high-volume logs behind an environment variable such as
    `TERMSURF_GEOMETRY_TRACE=1`.
- `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`
  - add structured bridge-level trace logs for overlay clear/present requests;
  - include pane id, context id, overlay cell rectangle, browser pixel size, and
    whether a target surface was found.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - add structured AppKit-side geometry logs when presenting, resizing,
    hiding/showing, clearing, and hit-testing the native overlay;
  - log surface id, window id when available, backing scale factor, bounds, cell
    size, computed overlay frame, root/positioning/host layer frames, browser
    pixel size, context id, visibility, and hit-test coordinates;
  - add a small helper for formatting geometry records consistently.
- `scripts/ghostboard-geometry-matrix.sh`
  - create a repeatable harness entry point that launches the built
    `TermSurf.app`, runs `target/debug/web` with the repo-built Roamium binary,
    captures logs under `logs/`, and records screenshots for named scenarios;
  - initially support the `initial-open` scenario that reproduces the screenshot
    failure;
  - send a deterministic mouse event inside the visible overlay for
    `initial-open` so input hit-test geometry is always exercised;
  - accept a scenario name so later experiments can add `window-resize-larger`,
    `split-horizontal`, `tab-switch`, and the other matrix cases without
    inventing new harnesses.
- `issues/0809-ghostboard-viewport-geometry/01-geometry-observability-harness.md`
  - record the design, implementation notes, verification commands, result, and
    reviewer findings.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 1 to the experiment index.

Reference files to inspect but not necessarily change:

- `ghostboard-legacy/src/Surface.zig`
  - `setOverlay`, `setCAContextId`, and `clearOverlay` show the legacy invariant
    that overlay frame updates happen when overlay grid state or context state
    changes.
- `ghostboard-legacy/src/renderer/Metal.zig`
  - `updateCALayerHostFrame` shows the legacy frame calculation inputs: grid
    rectangle, cell size, padding, and scale.
- `ghostboard-legacy/src/renderer/generic.zig`
  - resize handling calls `updateCALayerHostFrame`, which is a likely reference
    for later fixes after this instrumentation identifies the current gap.

## Verification

Pass criteria:

- Markdown is formatted with:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/01-geometry-observability-harness.md
  ```

- Zig formatting is run if Zig files are changed:

  ```bash
  cd ghostboard
  zig fmt src/apprt/termsurf.zig
  ```

- Swift formatting/linting is run if Swift files are changed, following
  `ghostboard/macos/AGENTS.md`:

  ```bash
  cd ghostboard
  swiftlint lint --strict --fix \
    macos/Sources/App/macOS/AppDelegate+TermSurf.swift \
    "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"
  ```

- The underlying Ghostboard library and macOS app build:

  ```bash
  cd ghostboard
  zig build -Demit-macos-app=false
  macos/build.nu --scheme Ghostty --configuration Debug --action build
  ```

- The harness runs the `initial-open` scenario with geometry tracing enabled:

  ```bash
  scripts/ghostboard-geometry-matrix.sh initial-open
  ```

- The `initial-open` run produces:
  - a Ghostboard app log under `logs/`;
  - a harness log under `logs/`;
  - a screenshot under `logs/`;
  - at least one Zig-side geometry record;
  - at least one bridge-level geometry record;
  - at least one AppKit-side geometry record;
  - at least one input hit-test geometry record from a deterministic mouse event
    inside the overlay.
- The geometry records are comparable by scenario and include the canonical
  identity tuple. If a layer cannot know one tuple field directly, the record
  must log an explicit unavailable marker and reason, and the harness must
  correlate the missing field from another record in the same `initial-open`
  run.
- The harness proves correlation across Zig, bridge, AppKit, screenshot, and log
  artifacts for `initial-open` by matching the pane id, browser tab id, context
  id, overlay rectangle, and timestamp/scenario id.
- The input hit-test record includes the current hit-test frame, the raw event
  point, the hit result, and the webview-relative coordinates.
- The experiment explicitly records whether the 2026-06-17 viewport-fill failure
  is reproduced by the harness, but it does not fix that failure.
- `git diff --check` passes.

Fail criteria:

- The experiment changes overlay geometry behavior instead of adding
  observability and harness support.
- The harness is one-off and cannot be extended to the remaining matrix
  scenarios.
- Geometry logs omit the fields needed to compare Zig overlay state, AppKit
  layer frame state, browser viewport state, and input hit-test state.
- Geometry records cannot be correlated into one canonical
  `window_id + surface_id + selected_tab_id + pane_id + browser_tab_id` identity
  for the `initial-open` run.
- `initial-open` does not exercise and record input hit testing.
- The harness cannot run the `initial-open` scenario against the repo-built
  `TermSurf.app`, `target/debug/web`, and Chromium-output `roamium`.
- The experiment requires changes to `webtui` or Roamium without concrete
  evidence that Ghostboard-side instrumentation cannot observe the needed state.

## Design Review

A fresh-context adversarial reviewer first returned **CHANGES REQUIRED** with
two required findings:

- The design allowed incomplete canonical identity evidence. The reviewer noted
  that Issue 809 requires
  `window_id + surface_id + selected_tab_id + pane_id + browser_tab_id`, while
  the original pass criteria allowed an identity tuple only "when available" and
  did not require cross-layer correlation.
- The design did not prove input hit-test observability. The reviewer noted that
  the original `initial-open` scenario could pass without sending mouse input,
  even though Issue 809 requires hit-test frame and webview-relative coordinate
  evidence.

The design was updated to require explicit unavailable-field markers with
reasons, `initial-open` correlation across Zig, bridge, AppKit, screenshot, and
log artifacts, deterministic mouse input inside the overlay, and a required
hit-test geometry record with frame, raw event point, hit result, and
webview-relative coordinates.

A fresh-context re-review returned **APPROVED**. The reviewer confirmed that the
two required findings were resolved and that the fixes introduced no new
required findings.
