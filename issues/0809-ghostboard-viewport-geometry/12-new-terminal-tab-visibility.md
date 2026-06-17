# Experiment 12: New terminal tab visibility

## Description

Experiment 11 proved same-tab focus switching between a browser-owning pane and
a sibling terminal pane. The next viewport matrix rows are tab visibility:

- creating a new terminal tab must hide the existing browser overlay because its
  owning AppKit/native tab is no longer selected;
- switching back to the browser tab must show the same browser overlay again in
  the same pane, with the same browser tab id, context id, overlay frame, and
  AppKit-presented pixel size.

This experiment intentionally covers only one existing browser overlay and one
new plain terminal tab in the same native macOS tab group. The harness must make
the global `initial-command` safe for tab creation: the first surface runs
`webtui`, but later inherited-tab launches run a plain shell/marker command so
they do not start a second browser. It does not open a second browser in the new
tab, close tabs, move tabs, or test multiple windows. Those are separate matrix
rows.

If current Ghostboard already passes, this experiment should record that and
avoid product source changes. If it fails, the harness must first localize
whether the failure is AppKit view visibility, selected-tab identity, stale
native layer visibility, hit testing, or keyboard routing before any product fix
is designed.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add a `new-terminal-tab-visibility` scenario;
  - add scenario-local keybindings:
    - `keybind = ctrl+t=new_tab`;
    - `keybind = ctrl+1=goto_tab:1`;
    - `keybind = ctrl+2=goto_tab:2`;
    - `keybind = ctrl+p=previous_tab`;
  - launch the same repo-built `TermSurf.app`, `target/debug/web`, and Roamium
    trace setup as existing scenarios;
  - replace the ordinary one-line `run-web.sh` with a first-run wrapper for this
    scenario:
    - the wrapper atomically creates a marker file in `RUN_DIR`;
    - the first invocation execs `target/debug/web --browser ...`;
    - later invocations append a marker to a new-tab command log and exec a
      plain shell that stays alive long enough for keyboard testing;
  - record the new-tab marker command/log path in the harness output;
  - wait for the initial-open AppKit/Zig/Roamium correlation to pass;
  - record the original browser pane id, browser tab id, context id, overlay
    frame, AppKit-presented pixel size, selected tab id, app-log line count, and
    Roamium trace line count as the selected browser-tab baseline;
  - inject Control-T to create a new terminal tab;
  - wait until AppKit logs show the original browser surface is no longer the
    selected tab, or until the original surface logs `view_did_hide`;
  - prove the new terminal tab starts as a plain login shell and does not
    inherit the first-run `webtui` wrapper;
  - prove no second browser tab/context is created after the new-tab boundary;
  - capture a screenshot while the new terminal tab is selected;
  - prove the hidden-tab state does not present the original browser overlay as
    visible in the selected tab;
  - click inside the old browser overlay's former screen rectangle and prove it
    does not route to the original browser context while the new terminal tab is
    selected;
  - type a deterministic marker in the new terminal tab and prove Roamium does
    not receive browser key events for the original browser tab and pane after
    the new-tab boundary;
  - inject Control-P to switch back to the browser tab with the public
    `previous_tab` action;
  - wait for the original browser pane to regain focus;
  - prove no post-switch AppKit presentation or pixel record changed the
    tab-bar-adjusted baseline geometry;
  - prove the original browser overlay reappears with the same pane id, browser
    tab id, context id, and tab-bar-adjusted overlay frame using a fresh
    hit-test on the restored tab;
  - click inside the restored browser overlay and prove hit testing routes to
    the original browser context with the same overlay frame and a current
    webview-relative point;
  - enter Browse mode if needed, type a deterministic browser marker, and prove
    Roamium receives key events for the original browser tab and pane only after
    the browser tab is selected again;
  - capture a post-switch-back screenshot;
  - fail if any assertion accepts pre-tab-switch AppKit, Zig, Roamium, or
    hit-test records as post-switch proof.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if the harness proves native layer visibility is not updated on
    tab hide/show, or hit testing still reaches a hidden-tab overlay.
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
  - change only if the harness proves tab selection does not trigger the
    visibility/focus notifications needed by the existing SurfaceView layer
    lifecycle.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if the harness proves Zig pane state must be told about
    tab-selected visibility to stop stale browser focus or input routing.
- `issues/0809-ghostboard-viewport-geometry/12-new-terminal-tab-visibility.md`
  - record the design review, implementation, verification, completion review,
    result, and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 12 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
- `ghostboard/macos/Sources/Ghostty/Ghostty.App.swift`
- `ghostboard/src/apprt/termsurf.zig`
- `issues/0809-ghostboard-viewport-geometry/01-geometry-observability-harness.md`
- `issues/0809-ghostboard-viewport-geometry/11-same-tab-focus-switch.md`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/12-new-terminal-tab-visibility.md
  ```

- Shell syntax is valid:

  ```bash
  bash -n scripts/ghostboard-geometry-matrix.sh
  ```

- If Swift files are changed:

  ```bash
  cd ghostboard
  swiftlint lint --strict --fix \
    "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift" \
    "macos/Sources/Features/Terminal/TerminalController.swift"
  swiftlint lint --strict \
    "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift" \
    "macos/Sources/Features/Terminal/TerminalController.swift"
  macos/build.nu --scheme Ghostty --configuration Debug --action build
  ```

- If Zig files are changed:

  ```bash
  cd ghostboard
  zig fmt src/apprt/termsurf.zig
  zig build -Demit-macos-app=false
  ```

- If only the harness/docs change, the already-built app may be reused, but the
  final result must still state whether any product build was or was not needed.
- Existing adjacent scenarios still pass serially:

  ```bash
  scripts/ghostboard-geometry-matrix.sh initial-open
  scripts/ghostboard-geometry-matrix.sh split-right-focus-switch
  ```

- The new scenario passes:

  ```bash
  scripts/ghostboard-geometry-matrix.sh new-terminal-tab-visibility
  ```

- The `new-terminal-tab-visibility` passing run proves all of the following:
  - initial-open still correlates AppKit, Zig, Roamium, screenshot, and hit
    test;
  - the new tab is created by the scenario-local `ctrl+t` keybinding;
  - the new terminal tab starts a plain login shell and does not inherit the
    first-run `webtui` wrapper;
  - no second browser tab id, pane id, or CA context is created after the
    new-tab boundary;
  - while the new terminal tab is selected, the original browser tab's native
    surface is hidden or no longer selected;
  - while the new terminal tab is selected, the original browser overlay is not
    visible in the selected tab;
  - clicking the former browser-overlay screen rectangle while the new terminal
    tab is selected does not route to the original browser context;
  - keyboard input typed in the new terminal tab does not reach Roamium as
    browser input for the original browser context;
  - switching back with `ctrl+p` / `previous_tab` focuses the original browser
    pane again;
  - after switching back, the original browser overlay is visible again with the
    same pane id, browser tab id, context id, and tab-bar-adjusted frame as the
    baseline, and no AppKit log records a conflicting frame or pixel size;
  - clicking the restored browser overlay produces a fresh `hit=true` hit-test
    for the original browser context with a current webview-relative point and
    the same overlay frame as the baseline;
  - browser keyboard input reaches Roamium only after the browser tab is
    selected again and webtui is in Browse mode;
  - the post-new-tab screenshot shows no browser overlay in the new terminal
    tab, and the post-switch-back screenshot shows the browser visible again in
    the original tab.
- `git diff --check` passes.
- The design review is recorded in this experiment file and the plan is
  committed before implementation begins.

Fail criteria:

- The harness manipulates private Ghostboard state instead of exercising real
  tab keybindings or public app behavior.
- The test accepts pre-tab-switch AppKit, Zig, Roamium, or hit-test records as
  proof of post-switch behavior.
- The browser overlay remains visible or hit-testable while the new terminal tab
  is selected.
- A second browser overlay is created in the new tab.
- Keyboard input typed in the new terminal tab reaches Roamium as browser input
  for the original browser context.
- Switching back to the browser tab loses the original pane id, browser tab id,
  context id, frame, or AppKit-presented pixel size.
- The experiment expands into opening a second browser, tab close, tab move,
  window switching, or multi-window behavior before basic new-tab hide/show is
  proven.

## Design Review

The first design review was performed by a fresh-context Codex adversarial
subagent.

Verdict: **Changes required**.

Findings:

- Required: the design assumed the new tab would be an empty terminal tab, but
  the current harness config uses a global `initial-command = direct:$COMMAND`.
  Ghostty inherits tab configuration when creating a new tab, so the new tab
  would likely run `webtui` again and create a second browser. That would make
  the hidden-overlay proof ambiguous and expand the experiment scope.
- Optional: the planned issue-doc update mentioned completion review but did not
  explicitly mention recording the design review and the separate plan commit
  gate.

Fixes:

- Added a required first-run wrapper for `new-terminal-tab-visibility`: the
  first invocation runs `webtui`, while inherited later tab launches run a plain
  shell/marker command.
- Added verification that the second command invocation is plain terminal work
  and that no second browser tab, pane, or CA context is created after the
  new-tab boundary.
- Added explicit design-review recording and plan-commit expectations.

The fixed design was re-reviewed by the same fresh-context Codex adversarial
subagent.

Final verdict: **Approved**.

The reviewer confirmed the required inherited-command issue is resolved by the
first-run wrapper requirement, the plan-commit/design-review workflow concern is
resolved, and no Required findings remain.

## Result

**Result:** Pass

Implemented `new-terminal-tab-visibility` in
`scripts/ghostboard-geometry-matrix.sh`. No Ghostboard product source changes
were needed.

The passing run was:

```bash
scripts/ghostboard-geometry-matrix.sh new-terminal-tab-visibility
```

Run evidence:

- harness log:
  `logs/ghostboard-geometry-new-terminal-tab-visibility-harness-20260617-110857.log`
- app log:
  `logs/ghostboard-geometry-new-terminal-tab-visibility-app-20260617-110857.log`
- Roamium trace:
  `logs/ghostboard-geometry-new-terminal-tab-visibility-roamium-20260617-110857.log`
- initial screenshot:
  `logs/ghostboard-geometry-new-terminal-tab-visibility-screenshot-20260617-110857.png`
- new terminal tab screenshot:
  `logs/ghostboard-geometry-new-terminal-tab-visibility-new-tab-screenshot-20260617-110857.png`
- restored browser tab screenshot:
  `logs/ghostboard-geometry-new-terminal-tab-visibility-back-tab-screenshot-20260617-110857.png`

The passing run proved:

- initial browser geometry still correlates across Zig, bridge, AppKit, Roamium,
  screenshot, and hit-test evidence;
- `ctrl+t` dispatches the public `.new_tab` action;
- the new native terminal tab starts `/usr/bin/login` as a plain shell and does
  not inherit the first-run `webtui` wrapper;
- the browser tab's geometry shrinks from `944x493` / `1888x986` to `944x459` /
  `1888x918` when the native tab bar appears, and that tab-bar-adjusted geometry
  becomes the correct restore baseline;
- selecting the second native tab with `ctrl+2` changes `selected_tab_id` from
  `11748` to `11761`;
- no second Zig browser pane/context and no second Roamium browser context are
  created after opening the new terminal tab;
- the original browser overlay is not freshly presented as visible in the
  selected new terminal tab;
- clicking the old browser rectangle while the new terminal tab is selected does
  not route to the original browser context;
- typing in the new terminal tab does not reach Roamium as a key event for the
  original browser context;
- switching back with `ctrl+p` / `previous_tab` focuses the original browser
  pane again;
- after switching back, no AppKit log records a conflicting frame or pixel size;
- a fresh restored-tab hit-test has the original `selected_tab_id`, the original
  browser context id, a current `web_point`, and the tab-bar-adjusted `944x459`
  overlay frame;
- after entering Browse mode, Roamium receives `focused=true` and then receives
  the restored browser keyboard marker.

Verification commands run:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
scripts/ghostboard-geometry-matrix.sh new-terminal-tab-visibility
scripts/ghostboard-geometry-matrix.sh split-right-focus-switch
scripts/ghostboard-geometry-matrix.sh initial-open
```

`split-right-focus-switch` passed in run
`logs/ghostboard-geometry-split-right-focus-switch-harness-20260617-110958.log`.
`initial-open` passed when run serially in
`logs/ghostboard-geometry-initial-open-harness-20260617-111019.log`.

An earlier parallel `initial-open` run at
`logs/ghostboard-geometry-initial-open-harness-20260617-110958.log` failed to
observe the initial hit-test while another GUI automation scenario was running
at the same time. The serial rerun passed, so viewport geometry did not regress;
the harness scenarios should be run serially when they depend on global macOS
keyboard/mouse focus.

## Conclusion

The new-tab visibility matrix row is covered. Current Ghostboard hides the
browser from the selected new terminal tab, prevents mouse and keyboard input in
that tab from reaching the old browser context, and restores the original
browser tab with correct tab-bar-adjusted geometry and input routing.

Two useful implementation learnings should inform later tab experiments:

- Ghostboard/Ghostty macOS `new_tab` in this path starts a plain login shell and
  does not inherit the harness `initial-command`; the first-run wrapper remains
  harmless guardrail evidence but is not used by the new tab.
- Switching back to an already-presented browser tab does not necessarily emit a
  fresh AppKit `presented` record. The durable restore proof is pane focus,
  absence of conflicting geometry logs, screenshot evidence, and a fresh
  restored-tab hit-test with the expected selected tab id and overlay frame.

## Completion Review

The completed experiment was reviewed by a fresh-context Codex adversarial
subagent.

Verdict: **Approved**.

Findings: none.

The reviewer independently verified:

- the result commit had not yet been made;
- the diff only touches the harness and Issue 809 documentation, with no product
  source changes;
- `bash -n scripts/ghostboard-geometry-matrix.sh` passed;
- `git diff --check` passed for the reviewed diff;
- the README marks Experiment 12 as `Pass`, and this experiment file has
  `Result` and `Conclusion`;
- the claimed logs support the pass, including the serial rerun of
  `initial-open`;
- the screenshots match the claims: the new terminal tab has no browser overlay,
  and the restored tab shows the browser overlay again.
