# Experiment 31: Present the Normal Roamium Overlay

## Description

Experiment 30 proved that Ghostboard can launch the real repo-built Roamium
artifact, complete the normal `webtui` lifecycle, receive `CaContext`, and keep
the app and browser process lifecycle clean enough for a smoke test. The next
major parity gap is visual: Ghostboard currently logs browser-originated
messages such as `CaContext`, but it does not attach the Chromium
`CAContext`/`CALayerHost` output to the macOS terminal surface.

This experiment will implement native macOS overlay presentation for the normal
Roamium tab only. It will route
`CaContext(tab_id, ca_context_id, pixel_width, pixel_height)` to the owning
terminal pane, ask the AppKit side to create or update a `CALayerHost`, and
position that layer over the terminal cell rectangle from the latest
`SetOverlay`/`Resize` state.

This experiment intentionally stops at visual presentation. Browser keyboard and
mouse input forwarding, DevTools overlay presentation, shutdown crash cleanup,
and richer page-state UI updates are separate experiments.

## Changes

Expected implementation files:

- `ghostboard/src/apprt/termsurf.zig`
  - handle `TERMSURF__TERM_SURF_MESSAGE__MSG_CA_CONTEXT` instead of only logging
    it;
  - map the browser server plus `tab_id` back to the pane recorded by
    `TabReady`;
  - store the latest `ca_context_id`, browser pixel size, and pending overlay
    frame in normal pane state;
  - call a macOS bridge function after `CaContext`, `SetOverlay`, and `Resize`
    updates so Swift can create or reposition the host layer;
  - log successful and rejected overlay presentation attempts with pane id, tab
    id, context id, pixel size, and frame.
- `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`
  - add a new `@_cdecl` bridge, likely `termsurf_present_overlay`, following the
    existing `termsurf_open_split` pattern;
  - resolve the pane id to `Ghostty.SurfaceView` via `AppDelegate.findSurface`;
  - dispatch AppKit layer mutation to the main queue;
  - log accepted/rejected overlay bridge calls.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - own the normal browser overlay layers for a surface;
  - create a root `CALayer` if needed, then a flipped/positioning layer and a
    `CALayerHost` with the received `contextId`;
  - update the frame without implicit animations when `SetOverlay` or `Resize`
    changes the terminal-cell rectangle;
  - replace the hosted context safely if Roamium sends a new `CaContext`;
  - tear down overlay layers when the surface deinitializes or the pane clears.

Possible supporting files, only if required by the existing macOS source layout:

- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView.swift`
  - expose enough geometry or lifecycle state to position the overlay relative
    to the rendered terminal surface.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceScrollView.swift`
  - update overlay frame when scrolling or visible bounds changes if the
    existing `SurfaceView` frame alone is insufficient.

No changes will be made to `webtui`, `roamium`, Chromium,
`proto/termsurf.proto`, config paths, branding, CLI install behavior, DevTools
behavior, browser input forwarding, or browser shutdown in this experiment.

## Verification

Pass criteria:

- `cargo build -p webtui` passes, with command, cwd, and exit status recorded in
  a log.
- `./scripts/build.sh roamium` passes, with command, cwd, and exit status
  recorded in a log.
- The real browser artifact remains
  `/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium`, and the
  runtime harness uses that path rather than `target/debug/roamium`, an
  installed browser, or a fake helper.
- If Zig code is modified, run
  `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  inside `ghostboard/`, with command, cwd, and exit status recorded in a log.
- If Swift code is modified, run the nested Ghostboard SwiftLint fix and
  non-mutating lint checks for the touched Swift files, with command, cwd, and
  exit status recorded in logs. If SwiftLint cannot run in this environment,
  record the exact failure and run the macOS app build as the required compiler
  check.
- The native GhosttyKit framework build passes:
  `zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`,
  with command, cwd, and exit status recorded in a log.
- The macOS app build passes:
  `macos/build.nu --scheme Ghostty --configuration Debug --action build`, with
  command, cwd, and exit status recorded in a log.
- Runtime harness launches `TermSurf.app` with `GHOSTTY_LOG=stderr` and a
  temporary config whose command runs:

  ```text
  /Users/astrohacker/dev/termsurf/target/debug/web --browser /Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium https://example.com
  ```

- Runtime logs still prove the Experiment 30 lifecycle:
  - Ghostboard spawns the Chromium-output Roamium with `--ipc-socket`,
    `--user-data-dir`, and `--listen-socket`;
  - Roamium sends `ServerRegister(profile=default)`;
  - Ghostboard sends `CreateTab`;
  - Roamium sends `TabReady`;
  - Ghostboard sends `BrowserReady`;
  - `webtui` connects to Roamium's direct browser socket;
  - `web last` returns the normal Roamium tab.
- Runtime logs prove the new overlay path:
  - Ghostboard receives `CaContext` with nonzero `ca_context_id`;
  - Ghostboard maps the `CaContext` tab id to the normal pane id;
  - Ghostboard calls the macOS overlay bridge with pane id, context id, and a
    nonzero frame;
  - AppKit creates or updates a `CALayerHost` with that context id;
  - AppKit positions the host over the expected terminal-cell rectangle.
- Visual verification is mandatory. Capture a screenshot of the launched app and
  prove that browser content is visible inside the terminal pane. The screenshot
  check should be automated if possible:
  - capture the app window after `TitleChanged("Example Domain")` or after
    `LoadingState` completes;
  - crop or inspect the expected overlay rectangle;
  - require non-terminal browser pixels or recognizable Example Domain content
    in that rectangle.
- If automated screenshot validation is not reliable in this macOS VM, the
  result must include recorded manual screenshot inspection with the screenshot
  path, the inspected rectangle, and a pass/fail statement that recognizable
  `Example Domain` content or non-terminal browser content is visible inside the
  expected overlay rectangle. App logs proving `CALayerHost` creation are
  necessary but not sufficient for this experiment to pass.
- Runtime cleanup leaves no stale matching
  `TermSurf.app/Contents/MacOS/termsurf`, `target/debug/web`, or
  `chromium/src/out/Default/roamium` processes, and removes the GUI socket.
- `git diff --check` is clean.
- `git diff --name-only` or `git diff --stat` is recorded, and the experiment
  fails if the implementation changes any forbidden path: `webtui/`, `roamium/`,
  `chromium/`, or `proto/termsurf.proto`.

Fail criteria:

- The runtime uses a fake helper, installed browser, or `target/debug/roamium`
  instead of `/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium`.
- `webtui`, `roamium`, Chromium, or `proto/termsurf.proto` are modified to make
  visual presentation work.
- `CaContext` is received but not mapped to the pane that owns the normal tab.
- The AppKit bridge mutates layers off the main thread.
- A `CALayerHost` is created without a nonzero `contextId`.
- The overlay frame ignores the `SetOverlay`/`Resize` cell rectangle or is not
  updated when overlay geometry changes.
- The implementation breaks the Experiment 30 normal Roamium lifecycle.
- The experiment adds browser keyboard/mouse forwarding, DevTools overlay
  presentation, browser shutdown fixes, Chromium changes, `webtui` changes,
  `roamium` changes, or protobuf schema changes.

## Design Review

A fresh-context adversarial Codex subagent reviewed the Experiment 31 design and
returned **CHANGES REQUIRED** with two required findings:

- visual verification could pass with logs only, which would not prove that
  Chromium pixels are visible inside the terminal pane;
- the hygiene checks did not explicitly require `git diff --name-only` or
  `git diff --stat` to prove forbidden paths were untouched.

Both findings were accepted. The design now makes visual proof mandatory through
automated screenshot validation or recorded manual screenshot inspection with
explicit pass/fail criteria, and it requires a recorded diff-name or diff-stat
check that fails if `webtui/`, `roamium/`, `chromium/`, or
`proto/termsurf.proto` changed.

The same reviewer re-reviewed the fixes and returned **APPROVED**. The reviewer
confirmed that visual proof is now mandatory, logs are explicitly necessary but
not sufficient, the forbidden-path diff check is required, no new required
findings were introduced, and the issue README still links Experiment 31 as
`Designed`.

## Result

**Result:** Pass

Experiment 31 implemented normal-tab native overlay presentation for real
Roamium output. Ghostboard now routes `CaContext` from the browser server to the
owning normal pane, records the context id and browser pixel size, and calls a
macOS bridge with the pane id, context id, and current overlay cell rectangle.
The AppKit side resolves the pane id to `Ghostty.SurfaceView`, creates or
updates a `CALayerHost`, and positions it over the terminal surface.

Changes:

- `ghostboard/src/apprt/termsurf.zig`
  - added `termsurf_present_overlay` bridge declaration;
  - added `termsurf_clear_overlay` bridge declaration;
  - added normal-pane `ca_context_id`, `ca_pixel_width`, and `ca_pixel_height`
    state;
  - added `CaContext` handling through the existing browser-fd server state and
    tab-to-pane lookup;
  - added `PresentOverlay` snapshots after `CaContext` and later `SetOverlay`
    geometry updates;
  - clears AppKit overlay layers when a TUI pane with an attached context is
    cleaned up;
  - kept DevTools overlay presentation out of scope.
- `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`
  - added the `@_cdecl("termsurf_present_overlay")` bridge;
  - added the `@_cdecl("termsurf_clear_overlay")` bridge;
  - dispatches AppKit overlay work to the main queue;
  - resolves pane UUIDs through the existing `AppDelegate.findSurface` helper.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - added per-surface overlay layer ownership;
  - dynamically creates `CALayerHost` with the received `contextId`;
  - positions the hosted layer from the overlay grid rectangle and current
    terminal `cellSize`;
  - removes overlay layers during pane cleanup and surface teardown.

Verification:

- `cargo build -p webtui` passed.
  - Log: `logs/ghostboard-exp31-cargo-build-webtui-20260616.log`
- `./scripts/build.sh roamium` passed and used
  `chromium/src/out/Default/roamium`.
  - Log: `logs/ghostboard-exp31-build-roamium-script-20260616.log`
- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
  - Log: `logs/ghostboard-exp31-zig-fmt-20260616.log`
- Native GhosttyKit framework build passed.
  - Log: `logs/ghostboard-exp31-zig-native-xcframework-20260616.log`
- SwiftLint passed on the touched Swift files with zero violations.
  - Log: `logs/ghostboard-exp31-swiftlint-20260616.log`
- macOS app build passed.
  - Log: `logs/ghostboard-exp31-macos-build-debug-20260616.log`
- `git diff --name-only` listed only expected Ghostboard source files and issue
  documentation:
  - `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`
  - `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - `ghostboard/src/apprt/termsurf.zig`
  - `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/31-present-normal-roamium-overlay.md`
  - `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/README.md`
  - no forbidden `webtui/`, `roamium/`, `chromium/`, or `proto/termsurf.proto`
    paths were present.
  - Log: `logs/ghostboard-exp31-git-diff-name-only-20260616.log`
- `git diff --check` passed.

Runtime verification launched
`ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf` with
`GHOSTTY_LOG=stderr` and a temporary config whose command was:

```text
/Users/astrohacker/dev/termsurf/target/debug/web --browser /Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium https://example.com
```

The runtime harness proved the normal Roamium lifecycle still works:

- Ghostboard spawned
  `/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium` with
  `--ipc-socket`, `--user-data-dir`, and `--listen-socket`;
- Roamium sent `ServerRegister(profile=default)`;
- Ghostboard sent `CreateTab`;
- Roamium sent `TabReady`;
- Ghostboard sent `BrowserReady`;
- the real `webtui` process connected to Roamium's direct browser socket;
- `web last` returned the normal Roamium tab;
- cleanup left no stale matching app, `web`, or Roamium processes and removed
  the GUI socket.
- cleanup also cleared the AppKit overlay:

  ```text
  ClearOverlay: pane_id=4109BD34-38F5-44C6-A14E-745CA689E782
  TermSurf overlay cleared pane_id=4109BD34-38F5-44C6-A14E-745CA689E782
  ```

The runtime harness also proved the new overlay path:

- Ghostboard received `CaContext` for `tab_id=1`;
- Ghostboard mapped that tab to pane `4109BD34-38F5-44C6-A14E-745CA689E782`;
- Ghostboard called the macOS bridge:

  ```text
  PresentOverlay: pane_id=4109BD34-38F5-44C6-A14E-745CA689E782 context_id=2036760821 grid=78x16+1+1 pixel=780x320
  ```

- AppKit presented the overlay:

  ```text
  TermSurf overlay presented pane_id=4109BD34-38F5-44C6-A14E-745CA689E782 context_id=2036760821 frame={{8, 17}, {624, 272}} pixel=780x320
  ```

Visual verification was performed from:

```text
logs/ghostboard-exp31-screenshot-20260616.png
```

The screenshot shows recognizable browser content inside the expected terminal
pane overlay rectangle: a white browser page headed `Example Domain`, followed
by the standard explanatory text. This satisfies the mandatory visual proof
criterion; the overlay is not merely logged, it is visibly composited in the
TermSurf window.

Logs:

- Runtime harness: `logs/ghostboard-exp31-runtime-harness-20260616.log`
- App/Roamium: `logs/ghostboard-exp31-runtime-app-20260616.log`
- `web last`: `logs/ghostboard-exp31-querylast-20260616.log`
- Screenshot: `logs/ghostboard-exp31-screenshot-20260616.png`

One residual issue remains unchanged from Experiment 30: Roamium still crashes
during shutdown after the harness terminates Ghostboard and the browser socket
reaches EOF. The crash is still in Chromium compositor shutdown. No stale
process remains, and shutdown hardening remains out of scope for this
experiment.

## Conclusion

Ghostboard can now visually present a normal real-Roamium browser tab through
native macOS `CALayerHost` compositing. This closes the major gap between
protocol lifecycle proof and visible browser output for the normal browsing
path.

The remaining parity work should proceed to browser input forwarding, DevTools
overlay presentation, richer browser state handling, and graceful browser
shutdown.

## Completion Review

A fresh-context adversarial Codex subagent reviewed the completed Experiment 31
result and returned **CHANGES REQUIRED** with one required finding: pane cleanup
cleared TermSurf state without clearing the AppKit `CALayerHost`, so browser
pixels could remain attached if `webtui` exited while the surface view stayed
alive.

The finding was accepted. The implementation now includes a
`termsurf_clear_overlay` bridge, calls it from `cleanupTuiPanes` before pane
state is erased, removes the surface's overlay layers on the main queue, and
logs the clear path. The runtime harness was rerun and now requires:

```text
ClearOverlay: pane_id=4109BD34-38F5-44C6-A14E-745CA689E782
TermSurf overlay clear request pane_id=4109BD34-38F5-44C6-A14E-745CA689E782
TermSurf overlay cleared pane_id=4109BD34-38F5-44C6-A14E-745CA689E782
```

The same reviewer re-reviewed the cleanup fix and returned **APPROVED** with no
findings. The reviewer confirmed that the working-tree diff adds
`termsurf_clear_overlay`, snapshots pane ids before `pane.* = .{}`, calls the
clear bridge after releasing `state_mutex`, dispatches layer removal to the main
queue on the Swift side, has runtime logs proving request and clear, passes
SwiftLint/build checks, and keeps `git diff --check` clean.
