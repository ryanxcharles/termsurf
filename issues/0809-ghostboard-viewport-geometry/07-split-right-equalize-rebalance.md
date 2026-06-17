# Experiment 7: Split-right equalize rebalance

## Description

Experiment 6 proved that a browser attached to the left pane of a split-right
layout follows the pane when the divider is moved with `resize_split:right,20`.
The next viewport matrix row is equalizing or rebalancing panes after a layout
reflow.

This experiment should start from the same proven split-right setup,
deliberately make the layout unequal, then invoke normal Ghostty/Ghostboard
equalize behavior. It must prove the browser follows the original pane back
toward the equalized pane rectangle:

- open a browser in a single pane;
- create a right-side split from the browser-owning pane;
- record the post-split frame and AppKit-presented pixels as the equal-split
  baseline;
- invoke a scenario-local `resize_split:right,20` keybinding to make the
  split-right layout unequal;
- record the divider-resized frame and AppKit-presented pixels as the unequal
  baseline;
- invoke a scenario-local `equalize_splits` keybinding;
- prove the original browser pane returns to the post-split equal baseline
  within a small documented tolerance, while height remains stable;
- prove Zig and Roamium receive the equalized AppKit-presented pixel size after
  the equalize action;
- prove positive hit testing still works inside the equalized browser frame;
- prove a sibling-pane click outside the equalized browser frame does not route
  to the original browser.

This experiment intentionally covers only equalizing a two-pane split-right
layout after the divider has been moved. Zoom/maximize, pane close, tab, window,
fullscreen, and multi-window behavior remain later matrix rows.

If current Ghostboard already passes, the experiment should record that and
avoid product source changes. If it fails, the harness must first localize which
invariant failed before any Ghostboard fix is designed in this experiment.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add a `split-right-equalize` scenario;
  - add scenario-local keybindings:
    - `keybind = ctrl+d=new_split:right`;
    - `keybind = ctrl+l=resize_split:right,20`;
    - `keybind = ctrl+e=equalize_splits`;
  - launch the same repo-built `TermSurf.app`, `target/debug/web`, and Roamium
    trace setup as the existing scenarios;
  - wait for the initial-open AppKit/Zig/Roamium correlation to pass;
  - inject Control-D to create the right split and wait for the same post-split
    proof used by Experiments 4 and 6;
  - record the post-split frame, AppKit-presented pixel size, pane id, browser
    tab id, context id, app-log line count, and Roamium trace line count as the
    equal-split baseline;
  - inject Control-L to move the vertical divider right and wait for the same
    divider-resize proof used by Experiment 6;
  - record the divider-resized frame and AppKit-presented pixel size as the
    unequal baseline;
  - inject Control-E to invoke `equalize_splits`;
  - wait for a new AppKit presentation record after Control-E for the original
    pane/context whose frame width moves back from the unequal baseline to the
    post-split equal baseline within a documented tolerance, and whose height
    remains equal or within a small documented tolerance;
  - wait for a new AppKit-presented pixel record after Control-E whose pixel
    width moves back from the unequal baseline to the post-split equal baseline
    within a documented tolerance, and whose pixel height remains stable;
  - require Zig to record the equalized AppKit-presented pixel size after the
    equalize phase;
  - require Roamium's run-specific trace to contain `ffi=ts_set_view_size` after
    the equalize trace boundary with the equalized AppKit-presented pixel size
    for the original pane id and browser tab id;
  - capture a post-equalize screenshot;
  - send deterministic mouse input inside the equalized browser frame and
    require a fresh `hit=true` / `web_point` hit-test record after equalize;
  - send deterministic mouse input in the right sibling pane area, outside the
    equalized overlay frame but inside the window/sibling region, and require it
    does not route as a hit to the original browser overlay/context.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if the harness proves equalize updates fail;
  - likely candidate fixes should be localized from the geometry logs before
    implementation.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if AppKit does not re-present or report the updated overlay
    frame/pixels for the original pane after equalize.
- `issues/0809-ghostboard-viewport-geometry/07-split-right-equalize-rebalance.md`
  - record the design, implementation, verification, completion review, result,
    and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 7 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `ghostboard/src/input/Binding.zig:3378-3388`
- `ghostboard/src/apprt/action.zig:146-149`
- `ghostboard/src/config/Config.zig:6611-6629`
- `ghostboard/macos/Sources/Ghostty/Ghostty.App.swift:1262-1312`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift:491-614`
- `ghostboard/src/apprt/termsurf.zig:892-944`
- `ghostboard/src/apprt/termsurf.zig:1241-1358`
- `ghostboard/src/datastruct/split_tree.zig:759-787`
- `ghostboard-legacy/src/datastruct/split_tree.zig:813-890`
- `issues/0809-ghostboard-viewport-geometry/06-split-right-divider-resize.md`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/07-split-right-equalize-rebalance.md
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
  scripts/ghostboard-geometry-matrix.sh split-down
  scripts/ghostboard-geometry-matrix.sh split-right-resize
  ```

- The new scenario passes:

  ```bash
  scripts/ghostboard-geometry-matrix.sh split-right-equalize
  ```

- The `split-right-equalize` passing run proves all of the following:
  - initial-open still correlates AppKit, Zig, Roamium, screenshot, and hit
    test;
  - the split action is triggered by the scenario-local `ctrl+d` keybinding;
  - the divider-resize action is triggered by the scenario-local `ctrl+l`
    keybinding;
  - the equalize action is triggered by the scenario-local `ctrl+e` keybinding;
  - after equalize, AppKit reports a new overlay frame for the original pane id
    and context id;
  - the equalized overlay frame width moves back from the divider-resized
    unequal width to the post-split equal width within a small documented
    tolerance;
  - the equalized AppKit-presented pixel width moves back from the
    divider-resized unequal width to the post-split equal width within a small
    documented tolerance;
  - the equalized overlay frame height and AppKit-presented pixel height remain
    equal to the post-split baseline or within a small documented tolerance;
  - Zig records the equalized AppKit-presented pixel size for the original pane
    id after the equalize phase;
  - Roamium's run-specific trace records `ffi=ts_set_view_size` after the
    equalize trace boundary on the same line as the equalized AppKit-presented
    pixel size for the original pane id and browser tab id;
  - the post-equalize screenshot shows browser content filling only the original
    pane's equalized viewport;
  - hit testing inside the equalized browser frame reports `hit=true` and a
    current webview-relative coordinate after equalize;
  - hit testing in the right sibling pane area outside the equalized overlay
    frame does not route to the original browser overlay/context.
- `git diff --check` passes.

Fail criteria:

- The harness equalizes panes by calling a private Ghostboard API instead of
  exercising user-visible keybinding behavior.
- The test accepts pre-equalize AppKit, Zig, Roamium, or hit-test records as
  proof of equalize behavior.
- The test proves divider resize but not a subsequent equalize-induced pane size
  change.
- The browser remains at the unequal divider-resized size, overlaps the sibling
  pane, or loses hit-test routing after equalize.
- The experiment expands into zoom/maximize, pane close, tabs, fullscreen, or
  multi-window behavior before split-right equalize/rebalance is proven.

## Design Review

The design was reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

The reviewer reported no findings.

## Result

**Result:** Pass.

The `split-right-equalize` scenario is implemented in
`scripts/ghostboard-geometry-matrix.sh`. The harness now:

- accepts `split-right-equalize` in addition to the prior scenarios;
- adds scenario-local keybindings:
  - `keybind = ctrl+d=new_split:right`;
  - `keybind = ctrl+l=resize_split:right,20`;
  - `keybind = ctrl+e=equalize_splits`;
- creates a right-side split from the browser-owning pane;
- records the post-split frame/pixels as the equal-split baseline;
- resizes the divider right to make the split unequal;
- records the divider-resized frame/pixels as the unequal baseline;
- invokes equalize through Control-E;
- waits for AppKit frame and presented-pixel records after equalize that return
  to the equal-split baseline within tolerance;
- verifies that Zig records the equalized AppKit-presented pixel size;
- verifies that Roamium applies the equalized AppKit pixel size with
  `ffi=ts_set_view_size` after the equalize trace boundary;
- captures a post-equalize screenshot;
- verifies a positive hit test inside the equalized browser overlay;
- verifies the right sibling pane area does not route input to the original
  browser context.

No Ghostboard product source changes were needed for this row. Current
Ghostboard already keeps the browser attached to the original pane and resizes
it correctly after equalizing a previously moved split-right divider.

Final passing artifacts:

- split-right-equalize app log:
  `logs/ghostboard-geometry-split-right-equalize-app-20260617-083757.log`
- split-right-equalize harness log:
  `logs/ghostboard-geometry-split-right-equalize-harness-20260617-083757.log`
- split-right-equalize initial screenshot:
  `logs/ghostboard-geometry-split-right-equalize-screenshot-20260617-083757.png`
- split-right-equalize post-equalize screenshot:
  `logs/ghostboard-geometry-split-right-equalize-split-screenshot-20260617-083757.png`
- split-right-equalize Roamium trace:
  `logs/ghostboard-geometry-split-right-equalize-roamium-20260617-083757.log`
- initial-open regression app log:
  `logs/ghostboard-geometry-initial-open-app-20260617-083854.log`
- initial-open regression harness log:
  `logs/ghostboard-geometry-initial-open-harness-20260617-083854.log`
- window-resize regression app log:
  `logs/ghostboard-geometry-window-resize-app-20260617-083903.log`
- window-resize regression harness log:
  `logs/ghostboard-geometry-window-resize-harness-20260617-083903.log`
- split-right regression app log:
  `logs/ghostboard-geometry-split-right-app-20260617-083919.log`
- split-right regression harness log:
  `logs/ghostboard-geometry-split-right-harness-20260617-083919.log`
- split-down regression app log:
  `logs/ghostboard-geometry-split-down-app-20260617-084004.log`
- split-down regression harness log:
  `logs/ghostboard-geometry-split-down-harness-20260617-084004.log`
- split-right-resize regression app log:
  `logs/ghostboard-geometry-split-right-resize-app-20260617-084018.log`
- split-right-resize regression harness log:
  `logs/ghostboard-geometry-split-right-resize-harness-20260617-084018.log`

Key runtime evidence from the passing `split-right-equalize` run:

- initial frame: `944x493`, AppKit pixel size `1888x986`;
- post-split equal baseline: `456x493`, AppKit pixel size `912x986`;
- divider-resized unequal baseline: `480x493`, AppKit pixel size `960x986`;
- post-equalize frame: `456x493`, AppKit pixel size `912x986`;
- pane id: `DF05A487-4FC3-4E08-AEBA-4F9BF5571AC1`;
- browser tab id: `1`;
- context id: `3095360314`;
- positive post-equalize hit point: `276,414`;
- sibling negative post-equalize point: `756,414`.

Verification commands run:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
git diff --check
scripts/ghostboard-geometry-matrix.sh split-right-equalize
scripts/ghostboard-geometry-matrix.sh initial-open
scripts/ghostboard-geometry-matrix.sh window-resize
scripts/ghostboard-geometry-matrix.sh split-right
scripts/ghostboard-geometry-matrix.sh split-down
scripts/ghostboard-geometry-matrix.sh split-right-resize
```

The existing-scenario regression sweep was run serially. No product build was
needed because the only implementation change was to the shell harness.

## Completion Review

The completed experiment was reviewed by a fresh-context Codex adversarial
subagent.

Verdict: **Approved**.

The reviewer reported no findings.

## Conclusion

The equalize/rebalance matrix row passes for a two-pane split-right layout. A
browser in the original left pane follows the owning pane through split
creation, divider resize, and equalization back to the equal-split baseline, and
the AppKit, Zig, Roamium, screenshot, and input-hit evidence all agree.

The next experiment should move to the next untested matrix row, most likely
zoom/maximize pane behavior, while continuing to re-run adjacent split geometry
scenarios serially as regressions.
