# Experiment 13: Open Browser in New Tab

## Description

Experiment 12 proved that an existing browser overlay hides when a new plain
terminal tab is selected, that mouse and keyboard input in that new terminal tab
do not reach the hidden browser, and that switching back restores the original
browser with correct tab-bar-adjusted geometry.

The next matrix row is opening a browser in a new tab. This experiment should
start from the Experiment 12 shape, then launch a second `webtui` instance from
inside the new terminal tab. The result must prove two independent browser
overlays can coexist across native tabs without leaking visibility, geometry, or
input routing across tab boundaries:

- browser A belongs to the original tab and original pane;
- browser B belongs to the second tab and second pane;
- browser A and browser B have distinct browser tab ids, pane ids, CA/context
  ids, and native overlay identities;
- only the selected tab's browser is visible and hit-testable;
- switching between tabs restores the correct browser, pane id, browser tab id,
  context id, frame, and input routing for that tab.

This experiment intentionally covers exactly two native tabs in one window, each
with one browser overlay. It does not close tabs, move tabs, open additional
windows, or test multiple browser panes in the same tab. Those remain separate
matrix rows.

If current Ghostboard already passes, this experiment should record that and
avoid product source changes. If it fails, the harness must first localize
whether the failure is browser-session identity, AppKit tab visibility, stale
layer attachment, hit testing, keyboard focus, or Roamium process/context
routing before any product fix is designed.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add an `open-browser-in-new-tab` scenario;
  - reuse the repo-built `TermSurf.app`, `target/debug/web`, and Roamium trace
    setup used by existing scenarios;
  - add scenario-local keybindings:
    - `keybind = ctrl+t=new_tab`;
    - `keybind = ctrl+1=goto_tab:1`;
    - `keybind = ctrl+2=goto_tab:2`;
    - `keybind = ctrl+p=previous_tab`;
    - `keybind = ctrl+n=next_tab`;
  - launch browser A in tab 1 using the existing initial `webtui` command;
  - record browser A's pane id, browser tab id, context id, selected tab id,
    overlay frame, AppKit pixel size, and hit-test coordinates;
  - create and select a new plain terminal tab with `ctrl+t` and `ctrl+2`;
  - prove browser A is hidden and not hit-testable while tab 2 is selected;
  - type and submit a full repo-built `web` command in the tab 2 shell to launch
    browser B, using a distinct URL marker from browser A if practical;
  - wait for browser B's Zig `tab_ready`, Zig `ca_context`, bridge presentation,
    AppKit presentation, AppKit pixels, Roamium resize, screenshot, and hit-test
    correlation;
  - prove browser B has a different pane id, browser tab id, CA/context id, and
    native overlay identity from browser A;
  - prove browser B is attached to tab 2's selected tab id and tab-bar-adjusted
    frame;
  - prove browser A is not freshly presented as visible in tab 2 after browser B
    opens;
  - click browser B and type a browser marker after entering Browse mode, then
    prove Roamium receives the key event for browser B only;
  - switch back to tab 1 and prove browser A becomes focused/hit-testable again
    with its own identity and geometry, while browser B does not receive input;
  - switch forward to tab 2 and prove browser B becomes focused/hit-testable
    again with its own identity and geometry, while browser A does not receive
    input;
  - capture screenshots for browser A selected, browser B selected, browser A
    restored, and browser B restored;
  - fail if any assertion accepts pre-switch AppKit, Zig, Roamium, or hit-test
    records as post-switch proof.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if runtime evidence proves native layer visibility or hit
    testing is tab-global instead of tab-local.
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
  - change only if runtime evidence proves tab switching does not deliver the
    focus/visibility behavior needed for independent tab overlays.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if runtime evidence proves browser session identity or
    pane-to-tab routing is incomplete.
- `issues/0809-ghostboard-viewport-geometry/13-open-browser-in-new-tab.md`
  - record the design review, implementation, verification, completion review,
    result, and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 13 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `issues/0809-ghostboard-viewport-geometry/12-new-terminal-tab-visibility.md`
- `issues/0809-ghostboard-viewport-geometry/11-same-tab-focus-switch.md`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
- `ghostboard/src/apprt/termsurf.zig`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/13-open-browser-in-new-tab.md
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
  final result must state whether any product build was or was not needed.
- Existing adjacent scenarios pass serially:

  ```bash
  scripts/ghostboard-geometry-matrix.sh new-terminal-tab-visibility
  scripts/ghostboard-geometry-matrix.sh split-right-focus-switch
  ```

- The new scenario passes:

  ```bash
  scripts/ghostboard-geometry-matrix.sh open-browser-in-new-tab
  ```

- The passing `open-browser-in-new-tab` run proves all of the following:
  - browser A initially correlates AppKit, Zig, Roamium, screenshot, and
    hit-test evidence in tab 1;
  - tab 2 is created and selected with public keybindings;
  - before browser B opens, browser A is hidden from tab 2 and is not
    hit-testable from tab 2;
  - browser B opens from a typed command in tab 2's shell, not by private
    harness state manipulation;
  - browser B receives its own pane id, browser tab id, CA context id, AppKit
    presentation, AppKit pixel size, Roamium resize, screenshot, and hit-test;
  - browser A and browser B have distinct pane ids, browser tab ids, CA/context
    ids, and native overlay identities;
  - browser B is visible and hit-testable only while tab 2 is selected;
  - browser A is visible and hit-testable only while tab 1 is selected;
  - switching tab 1 -> tab 2 -> tab 1 -> tab 2 preserves each browser's identity
    tuple and tab-bar-adjusted geometry;
  - keyboard input reaches only the visible selected browser after Browse mode
    focus is established;
  - no click or keyboard marker typed in one selected tab reaches the hidden
    browser in the other tab;
  - screenshots show the expected browser in the selected tab and no overlay
    bleed from the hidden tab.
- `git diff --check` passes.
- The design review is recorded in this experiment file and the plan is
  committed before implementation begins.
- After implementation, verification, and result recording, the completion
  review is recorded in this experiment file and the result commit is made
  before designing or implementing Experiment 14.

Fail criteria:

- The harness launches browser B by mutating private Ghostboard state instead of
  typing a real command into the tab 2 shell.
- The test accepts browser A's pre-tab-switch records as proof for browser B, or
  browser B's records as proof for browser A.
- Browser A or browser B remains visible, hit-testable, or keyboard-focusable
  while its owning native tab is hidden.
- The two browsers share a pane id, browser tab id, CA/context id, or native
  overlay identity.
- Roamium receives input for the hidden browser.
- Switching between tabs loses either browser's pane id, browser tab id, context
  id, or tab-bar-adjusted geometry.
- The experiment expands into tab close, tab move, multiple windows, or multiple
  browser panes inside one tab before the basic two-tab/two-browser case is
  proven.

## Design Review

The design was reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes required**.

Findings:

- Required: the verification did not require browser A and browser B to have
  distinct CA/context ids or native overlay identities. It only required
  distinct pane ids and browser tab ids, which could allow the experiment to
  pass even if both browser overlays reused one native layer/context.
- Required: the result-review/result-commit gate was incomplete. The design
  mentioned the design review and plan commit, but did not explicitly require
  the completion review and separate result commit before moving to the next
  experiment.

Fixes:

- Added distinct CA/context id and native overlay identity requirements to the
  design goal, planned changes, pass criteria, and fail criteria.
- Added an explicit completion-review and result-commit pass criterion before
  designing or implementing Experiment 14.

The fixed design was re-reviewed by the same fresh-context Codex adversarial
subagent.

Final verdict: **Approved**.

The reviewer confirmed both Required findings were resolved and no new Required
findings were introduced.

## Result

**Result:** Pass.

The `open-browser-in-new-tab` scenario was implemented in
`scripts/ghostboard-geometry-matrix.sh` and passed after two product fixes in
Ghostboard:

- `ghostboard/src/apprt/termsurf.zig` now sends `CreateTab` immediately when a
  new pane calls `SetOverlay` for an already-attached browser server. Before
  this fix, browser B's `SetOverlay` was recorded for the second terminal tab,
  but no `CreateTab` was sent to Roamium, so browser B never reached `TabReady`.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift` now
  synchronizes the current SurfaceView focus state after overlay presentation
  and lets Escape pass to the terminal/TUI instead of forwarding it to Roamium.
  Before these fixes, browser B could open, but entering Browse mode did not
  focus Roamium if the surface had become focused before TermSurf knew about the
  pane, and Escape could not return `webtui` from Browse mode to Control mode.

The passing run proved:

- browser A opened in native tab 1 with pane id
  `89FB8BCF-6049-4D25-BBF1-FC1E03A83C92`, browser tab id `1`, and CA/context id
  `676350157`;
- browser A hid when native tab 2 was selected, and terminal mouse/keyboard
  input in tab 2 did not route to browser A;
- browser B was launched by typing the real repo-built `web` command into tab
  2's shell;
- browser B opened in native tab 2 with pane id
  `5B460647-526A-4A85-856C-6131CEE8A887`, browser tab id `2`, and CA/context id
  `3990711887`;
- browser A and browser B had distinct pane ids, browser tab ids, CA/context
  ids, and AppKit overlay identities;
- browser B's AppKit frame was `{{8, 17}, {944, 459}}` with AppKit pixel size
  `1888x918`, and Roamium applied that resize;
- browser B was visible, hit-testable, focusable, and keyboard-routable only
  while tab 2 was selected;
- switching tab 2 -> tab 1 restored browser A with the tab-bar-adjusted frame
  and AppKit pixels, and browser A accepted keyboard input after Browse mode was
  re-entered;
- switching tab 1 -> tab 2 restored browser B with its original frame and AppKit
  pixels, and browser B accepted keyboard input after Browse mode was
  re-entered;
- keyboard markers sent in one browser tab did not reach the hidden browser in
  the other tab.

Verification commands run:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
cd ghostboard && zig fmt src/apprt/termsurf.zig
cd ghostboard && zig build -Demit-macos-app=false
cd ghostboard && swiftlint lint --strict --fix \
  "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"
cd ghostboard && swiftlint lint --strict \
  "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"
cd ghostboard && macos/build.nu --scheme Ghostty --configuration Debug --action build
scripts/ghostboard-geometry-matrix.sh open-browser-in-new-tab
scripts/ghostboard-geometry-matrix.sh new-terminal-tab-visibility
scripts/ghostboard-geometry-matrix.sh split-right-focus-switch
git diff --check
```

Verification output:

- `bash -n scripts/ghostboard-geometry-matrix.sh` passed.
- `zig fmt src/apprt/termsurf.zig` passed.
- `zig build -Demit-macos-app=false` passed.
- `swiftlint lint --strict --fix ...` completed with no remaining changes
  required.
- `swiftlint lint --strict ...` passed with 0 violations.
- `macos/build.nu --scheme Ghostty --configuration Debug --action build` passed.
- `scripts/ghostboard-geometry-matrix.sh open-browser-in-new-tab` passed.
  - Harness log:
    `logs/ghostboard-geometry-open-browser-in-new-tab-harness-20260617-114038.log`
  - App log:
    `logs/ghostboard-geometry-open-browser-in-new-tab-app-20260617-114038.log`
  - Roamium trace:
    `logs/ghostboard-geometry-open-browser-in-new-tab-roamium-20260617-114038.log`
- `scripts/ghostboard-geometry-matrix.sh new-terminal-tab-visibility` passed.
  - Harness log:
    `logs/ghostboard-geometry-new-terminal-tab-visibility-harness-20260617-114146.log`
- `scripts/ghostboard-geometry-matrix.sh split-right-focus-switch` passed.
  - Harness log:
    `logs/ghostboard-geometry-split-right-focus-switch-harness-20260617-114238.log`
- `git diff --check` passed.

## Conclusion

Ghostboard can now host one browser overlay in tab 1 and a second independent
browser overlay in tab 2 using one shared Roamium profile server. The important
product learning is that a second `SetOverlay` for an already-attached browser
server must immediately emit `CreateTab`; waiting for `ServerRegister` only
works for the first pane. A second learning is that TermSurf focus state must be
synchronized from AppKit after overlay presentation, because a new SurfaceView
can already be focused before TermSurf has a pane record for it.

The next experiment should move to the next matrix row, closing a browser tab,
and prove both native-layer cleanup and Roamium tab cleanup for the owning
browser pane without regressing the two-tab visibility behavior proven here.

## Completion Review

The completed experiment was reviewed by a fresh-context Codex adversarial
subagent.

Final verdict: **Approved**.

Findings:

- No Required findings.

Evidence checked by the reviewer:

- scope matches Experiment 13 and the implementation is limited to the harness,
  the Ghostboard TermSurf routing path, the AppKit focus/key bridge, and issue
  docs;
- the result commit had not yet been made;
- the README marks Experiment 13 as `Pass`, and this experiment file has
  `Result` and `Conclusion`;
- the passing harness log proves distinct browser A/B pane ids, browser tab ids,
  and CA/context ids, plus tab switching, hit-testing, focus, Escape-to-Control,
  and keyboard isolation;
- the Roamium trace supports routed key/focus events for tab 1 and tab 2 with no
  cross-tab key-event evidence in the checked windows;
- `bash -n scripts/ghostboard-geometry-matrix.sh` passed;
- `git diff --check` passed.

The reviewer did not rerun the full runtime/build scenarios because they may
create or modify generated artifacts; it verified the supplied logs and
read-only checks instead.
