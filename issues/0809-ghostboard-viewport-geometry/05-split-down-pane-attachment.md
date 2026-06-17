# Experiment 5: Split-down pane attachment

## Description

Experiment 4 proved the horizontal split-right matrix row: a browser in the
original pane stays attached to that pane, narrows with the left pane, and stops
routing input from the new right sibling pane.

The next matrix row is the vertical pane split:

- start with one pane running `web`;
- create a split below the browser-owning pane;
- prove the browser remains attached to the original pane, which should become
  the top pane;
- prove the browser resizes to the original pane's new shorter viewport instead
  of staying at the pre-split height or moving into the new lower sibling pane;
- prove mouse hit testing uses the post-split browser frame and does not route
  lower-sibling input to the original browser.

This experiment should extend the existing geometry harness with a `split-down`
scenario. It should use normal Ghostty/Ghostboard user behavior to create the
split. The intended automation is to add a scenario-local keybinding to the
generated config:

```text
keybind = ctrl+j=new_split:down
```

Then the harness can inject Control-J with `scripts/ghostty-app/inject.swift`
after the initial browser-open correlation has passed. Control-J uses macOS
virtual key code `38` for the `j` key.

If current Ghostboard already passes, the experiment should record that and
avoid product source changes. If it fails, the harness must first localize which
invariant failed before any Ghostboard fix is designed in this experiment.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add a `split-down` scenario;
  - for this scenario, add `keybind = ctrl+j=new_split:down` to the generated
    Ghostboard config;
  - launch the same repo-built `TermSurf.app`, `target/debug/web`, and Roamium
    trace setup as `initial-open`;
  - wait for the same initial-open AppKit/Zig/Roamium correlation to pass;
  - record the pre-split identity tuple, pane id, browser tab id, AppKit overlay
    frame, AppKit-presented pixel size, and window bounds;
  - record the Roamium trace line count before injecting Control-J so post-split
    browser resize proof cannot reuse pre-split trace evidence;
  - inject Control-J into the focused app window to create a split below the
    original pane;
  - wait for a new AppKit presentation record after the key injection whose
    identity still contains the original pane id and context id;
  - require the post-split AppKit overlay frame height and AppKit-presented
    pixel height to be smaller than the pre-split height, while the frame/pixel
    width remains equal or within a small tolerance expected from split-divider
    or terminal-layout rounding;
  - require Zig to record the post-split AppKit-presented pixel size for the
    original pane id after the split phase;
  - require Roamium's run-specific trace to contain `ffi=ts_set_view_size` after
    the split trace boundary with the post-split AppKit-presented pixel size for
    the original pane id and browser tab id;
  - capture a post-split screenshot;
  - send deterministic mouse input inside the post-split browser frame and
    require a fresh `hit=true` / `web_point` hit-test record after the split;
  - send deterministic mouse input in the lower sibling pane area, at a point
    outside the post-split overlay frame but inside the old pre-split browser
    height/window area, and require it does not route as a hit to the original
    browser overlay/context.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if the harness proves the split-down update path fails;
  - likely candidate fixes should be localized from the geometry logs before
    implementation.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if AppKit does not re-present or report the updated overlay
    frame/pixels for the original pane after the split.
- `issues/0809-ghostboard-viewport-geometry/05-split-down-pane-attachment.md`
  - record the design, implementation, verification, completion review, result,
    and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 5 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `ghostboard/src/build/mdgen/ghostty_5_header.md`
- `ghostboard/macos/Sources/Ghostty/Ghostty.App.swift:846-864`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift:491-614`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift:2152-2169`
- `ghostboard/src/apprt/termsurf.zig:892-944`
- `ghostboard/src/apprt/termsurf.zig:1241-1358`
- `ghostboard-legacy/src/datastruct/split_tree.zig:505-570`
- `ghostboard-legacy/src/Surface.zig:2492-2515`
- `ghostboard-legacy/src/renderer/generic.zig:849-862`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/05-split-down-pane-attachment.md
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
    "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"
  swiftlint lint --strict \
    "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"
  macos/build.nu --scheme Ghostty --configuration Debug --action build
  ```

- If only the harness/docs change, the already-built app may be reused, but the
  final result must still state whether any product build was or was not needed.
- Existing scenarios still pass serially:

  ```bash
  scripts/ghostboard-geometry-matrix.sh initial-open
  scripts/ghostboard-geometry-matrix.sh window-resize
  scripts/ghostboard-geometry-matrix.sh split-right
  ```

- The new scenario passes:

  ```bash
  scripts/ghostboard-geometry-matrix.sh split-down
  ```

- The `split-down` passing run proves all of the following:
  - initial-open still correlates AppKit, Zig, Roamium, screenshot, and hit
    test;
  - the split action is triggered by the scenario-local `ctrl+j` keybinding;
  - after the split, AppKit reports a new overlay frame for the original pane id
    and context id;
  - the post-split overlay frame height and AppKit-presented pixel height are
    smaller than the pre-split height;
  - the post-split overlay frame width and AppKit-presented pixel width remain
    equal to the pre-split width or within a small documented tolerance;
  - Zig records the post-split AppKit-presented pixel size for the original pane
    id after the split phase;
  - Roamium's run-specific trace records `ffi=ts_set_view_size` after the split
    trace boundary on the same line as the post-split AppKit-presented pixel
    size for the original pane id and browser tab id;
  - the post-split screenshot shows the browser content filling only the
    original top pane's resized viewport;
  - hit testing inside the resized browser frame reports `hit=true` and a
    current webview-relative coordinate after the split;
  - hit testing in the lower sibling pane area, outside the post-split overlay
    frame but inside the old pre-split browser height/window area, does not
    route to the original browser overlay/context.
- `git diff --check` passes.

Fail criteria:

- The harness creates a split by calling a private Ghostboard API instead of
  exercising user-visible keybinding behavior.
- The test accepts pre-split AppKit, Zig, Roamium, or hit-test records as proof
  of post-split behavior.
- The test proves the window size changed but not that the original
  browser-owning pane changed.
- The browser moves to the new lower sibling pane, remains at the old full-pane
  height, overlaps the sibling pane, or loses hit-test routing after the split.
- The experiment expands into split-boundary dragging, pane close, tabs,
  fullscreen, or multi-window behavior before split-down is proven.

## Design Review

The design was reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

The reviewer reported no findings and verified that the README links Experiment
5 as `Designed`, the experiment has the required sections, the scope is limited
to the split-down matrix row, the keybinding/injection plan is plausible, and
the verification criteria cover stale-evidence prevention, geometry resize,
Roamium post-boundary resize, positive hit testing, negative lower-sibling hit
testing, serial regressions, and hygiene checks.

## Result

**Result:** Pass.

The `split-down` scenario is implemented in
`scripts/ghostboard-geometry-matrix.sh`. The harness now:

- accepts `split-down` in addition to the prior scenarios;
- adds the scenario-local config line `keybind = ctrl+j=new_split:down`;
- injects Control-J with `scripts/ghostty-app/inject.swift`;
- disables Ghostty window state restoration with `window-save-state = never` in
  every generated harness config, so scenarios start from a clean window layout;
- waits for a post-split AppKit presentation for the original pane/context;
- verifies that the original browser overlay height shrinks while width remains
  stable;
- verifies that AppKit-presented pixel height shrinks while pixel width remains
  stable;
- verifies that Zig records the post-split AppKit-presented pixel size for the
  original pane;
- verifies that Roamium applies the post-split AppKit pixel size with
  `ffi=ts_set_view_size` after the split key injection trace boundary;
- captures a post-split screenshot;
- verifies a positive hit test inside the resized browser overlay;
- verifies an explicit negative hit test in the lower sibling pane area, proving
  the old full-height browser frame is no longer routing input to the original
  browser context.

No Ghostboard product source changes were needed for this row. Current
Ghostboard already keeps the browser attached to the original pane and resizes
it correctly after a downward split.

One intermediate run failed because the app restored a previous split-down
window layout before the harness injected Control-J. Adding
`window-save-state = never` to the generated config fixed that and is now part
of every geometry scenario.

Final passing artifacts:

- split-down app log:
  `logs/ghostboard-geometry-split-down-app-20260617-081424.log`
- split-down harness log:
  `logs/ghostboard-geometry-split-down-harness-20260617-081424.log`
- split-down initial screenshot:
  `logs/ghostboard-geometry-split-down-screenshot-20260617-081424.png`
- split-down post-split screenshot:
  `logs/ghostboard-geometry-split-down-split-screenshot-20260617-081424.png`
- split-down Roamium trace:
  `logs/ghostboard-geometry-split-down-roamium-20260617-081424.log`
- initial-open regression app log:
  `logs/ghostboard-geometry-initial-open-app-20260617-080731.log`
- initial-open regression harness log:
  `logs/ghostboard-geometry-initial-open-harness-20260617-080731.log`
- window-resize regression app log:
  `logs/ghostboard-geometry-window-resize-app-20260617-080739.log`
- window-resize regression harness log:
  `logs/ghostboard-geometry-window-resize-harness-20260617-080739.log`
- split-right regression app log:
  `logs/ghostboard-geometry-split-right-app-20260617-081452.log`
- split-right regression harness log:
  `logs/ghostboard-geometry-split-right-harness-20260617-081452.log`

Key passing evidence from the `split-down` run:

- pre-split AppKit overlay frame: `944x493`, AppKit pixel size: `1888x986`;
- post-split AppKit overlay frame: `944x187`, AppKit pixel size: `1888x374`;
- the original pane id remained `983A12D5-C5D3-406A-9423-652B311DDB80`;
- the original browser tab id remained `1`;
- Roamium applied the post-split resize with `ffi=ts_set_view_size`;
- the post-split positive click reported `hit=true` with a current `web_point`;
- the lower-sibling negative click produced an explicit `hit=false` record for
  the original browser context.

Verification commands run:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
git diff --check
scripts/ghostboard-geometry-matrix.sh split-down
scripts/ghostboard-geometry-matrix.sh initial-open
scripts/ghostboard-geometry-matrix.sh window-resize
scripts/ghostboard-geometry-matrix.sh split-right
```

The post-split screenshot was visually inspected. It shows the browser filling
only the top pane while the new shell split occupies the lower pane.

## Conclusion

The vertical split-down matrix row passes in current Ghostboard. The browser
stays attached to its original pane, resizes to the top pane's shorter viewport,
forwards the corrected size to Roamium, and stops accepting input in the new
lower sibling pane.

## Completion Review

The completed experiment was reviewed by a fresh-context Codex adversarial
subagent.

Initial verdict: **Changes required**.

- Required finding: the negative hit-test helper was vacuous because it passed
  when no post-click hit-test record appeared. The split-down result needed an
  explicit post-click `hit=false` record for the original browser context.
- Optional finding: the app logs show Roamium crashing on teardown with
  `Received signal 11 SEGV_ACCERR`. This does not invalidate the split-down
  geometry proof, but it should be kept as evidence for the later
  cleanup/lifecycle matrix rows.

Fix:

- The negative hit-test helper now fails on any post-click `hit=true`, requires
  an explicit post-click `hit=false` AppKit hit-test record by default, and only
  allows absence for the split-right sibling-pane case where the click lands in
  a different surface and may not invoke the original overlay hit-test.
- The split-down scenario was rerun and passed with explicit `hit=false`
  evidence in `logs/ghostboard-geometry-split-down-app-20260617-081424.log`.

Re-review verdict: **Approved**. The reviewer confirmed the helper now fails on
post-click `hit=true`, requires explicit post-click `hit=false` by default,
keeps only split-right on the `allow-absent` path, verified the fresh split-down
artifacts, and reported no new required findings.
