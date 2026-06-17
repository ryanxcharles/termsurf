# Experiment 18: Fullscreen and Unfullscreen

## Description

Experiment 17 added display inventory and backing-scale coverage, but this VM
has only one display, so the physical cross-display row remains partial. The
next matrix row is fullscreen and unfullscreen.

This experiment should prove Ghostboard keeps a browser overlay aligned and
interactive while its owning native window transitions into fullscreen and back
to windowed mode. The harness should use a user-visible fullscreen path, not
private geometry mutation, and should verify that the same browser identity
survives both transitions.

This experiment intentionally covers one window with one browser overlay. It
does not test display moves, minimize/hide, split panes, multiple windows,
DevTools, or final matrix regression.

If current Ghostboard already passes, the experiment should record that and
avoid product source changes. If it fails, the harness must first localize
whether the failure is fullscreen action dispatch, AppKit frame/pixel update,
backing-scale reporting, Roamium resize delivery, stale hit testing, or keyboard
routing before any product fix is designed.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add a `fullscreen-unfullscreen` scenario;
  - add a generated Swift helper, or reuse an existing key/action path, to
    trigger native fullscreen through the user-visible macOS window fullscreen
    mechanism;
  - launch one browser in one Ghostboard window using the repo-built `web` and
    Roamium binaries;
  - record the baseline canonical identity tuple:
    `window_id + surface_id + selected_tab_id + pane_id + browser_tab_id`, plus
    `context_id + AppKit frame + AppKit pixels + backing_scale`;
  - enter fullscreen through the public/native fullscreen path;
  - independently read native fullscreen state after the transition, such as
    `AXFullScreen`, and fail if the window is not fullscreen;
  - wait for post-fullscreen AppKit presentation and pixel records after the
    fullscreen boundary;
  - require the expected canonical identity behavior:
    - if macOS preserves the same AppKit/CG `window_id`, require the same
      `window_id + surface_id + selected_tab_id + pane_id + browser_tab_id`;
    - if native fullscreen changes the CG window id, record the new current
      window id and require the overlay to be rebound to that current window
      with the same `surface_id + selected_tab_id + pane_id + browser_tab_id`
      and context id, with no stale hit-testing or presentation records accepted
      from the pre-fullscreen window id;
  - require the fullscreen AppKit frame and pixels to reflect the larger
    fullscreen content area, allowing for menu-bar/dock variations in the VM;
  - require Roamium to receive the fullscreen AppKit pixel size via resize;
  - click inside the fullscreen overlay and prove hit testing uses the
    fullscreen frame, surface id, selected tab id, context id, and web-relative
    coordinates;
  - enter Browse mode and prove keyboard input reaches the same browser tab
    after fullscreen;
  - return to Control mode before leaving fullscreen;
  - exit fullscreen through the public/native path;
  - independently read native fullscreen state after the transition and fail if
    the window is still fullscreen;
  - wait for post-unfullscreen AppKit presentation and pixel records after the
    unfullscreen boundary;
  - require the expected window identity behavior again: either the original
    window id is restored/preserved, or a current window id is recorded and all
    post-unfullscreen AppKit and hit-test evidence is tied to that current
    window id rather than stale fullscreen records;
  - require the browser returns to the original windowed AppKit frame and pixel
    size, or record any legitimate window-manager-adjusted dimensions as the new
    windowed baseline with evidence;
  - require Roamium to receive the unfullscreen AppKit pixel size via resize
    when pixels change;
  - re-prove hit testing and keyboard routing after unfullscreen;
  - capture screenshots before fullscreen, during fullscreen, and after
    unfullscreen;
  - fail if assertions accept baseline records as fullscreen proof or fullscreen
    records as unfullscreen proof.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if runtime evidence proves fullscreen window transitions do not
    recompute native overlay frame/pixels or hit-test coordinates.
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
  - change only if runtime evidence proves native fullscreen transitions are not
    visible to the terminal surface or window controller.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if runtime evidence proves TermSurf resize/update messages omit
    data needed to track fullscreen transitions.
- `roamium/src/dispatch.rs`
  - change only if existing trace evidence cannot prove resize/focus/key input
    after fullscreen. Any such change must be trace-only under the existing
    trace mechanism.
- `issues/0809-ghostboard-viewport-geometry/18-fullscreen-unfullscreen.md`
  - record the design review, implementation, verification, completion review,
    result, and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 18 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `scripts/ghostty-app/winid.swift`
- `issues/0809-ghostboard-viewport-geometry/03-window-resize-follow.md`
- `issues/0809-ghostboard-viewport-geometry/08-split-right-zoom-restore.md`
- `issues/0809-ghostboard-viewport-geometry/17-display-move-backing-scale.md`
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
- `ghostboard/src/apprt/termsurf.zig`
- `roamium/src/dispatch.rs`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0809-ghostboard-viewport-geometry/README.md \
    issues/0809-ghostboard-viewport-geometry/18-fullscreen-unfullscreen.md
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
  macos/build.nu --scheme Ghostty --configuration Debug --action build
  ```

- If Rust files are changed:

  ```bash
  cargo fmt
  cargo check -p roamium
  ```

- The new scenario passes:

  ```bash
  scripts/ghostboard-geometry-matrix.sh fullscreen-unfullscreen
  ```

- The passing run proves:
  - fullscreen is entered through a public/native user-visible path;
  - native fullscreen state is independently proven true after entering
    fullscreen and false after returning to windowed mode;
  - the browser proves the expected canonical window identity behavior after
    entering fullscreen and after returning to windowed mode: either the same
    AppKit/CG `window_id` survives, or a new current window id is explicitly
    recorded and all AppKit/hit-test evidence is tied to that current window id
    with no stale records accepted;
  - the browser keeps the same surface id, selected tab id, pane id, browser tab
    id, and context id after entering fullscreen and after returning to windowed
    mode;
  - AppKit frame, AppKit pixels, and backing scale are recorded after each
    transition and are not stale baseline/fullscreen records;
  - Roamium receives the fullscreen and unfullscreen AppKit pixel size when
    pixels change;
  - mouse hit-testing and Browse-mode keyboard input still route to the browser
    after fullscreen and after unfullscreen;
  - screenshots show baseline, fullscreen, and unfullscreen states.
- Adjacent geometry regressions still pass:

  ```bash
  scripts/ghostboard-geometry-matrix.sh window-resize
  scripts/ghostboard-geometry-matrix.sh display-move-backing-scale
  ```

- `git diff --check` passes.
- The design review is recorded in this experiment file and the plan is
  committed before implementation begins.
- After implementation, verification, and result recording, the completion
  review is recorded in this experiment file and the result commit is made
  before designing or implementing Experiment 19.

Fail criteria:

- The harness fakes fullscreen by directly resizing private state instead of
  invoking a native/user-visible fullscreen path.
- Native fullscreen state is not independently proven true after fullscreen and
  false after unfullscreen.
- The browser changes surface id, selected tab id, pane id, browser tab id, or
  context id across fullscreen/unfullscreen.
- Window id behavior is ambiguous: post-transition evidence is not tied either
  to the preserved window id or to an explicitly recorded current replacement
  window id.
- AppKit frame/pixels/backing scale or Roamium resize evidence is missing or
  stale after either transition.
- Mouse or keyboard input after fullscreen/unfullscreen reaches the wrong
  browser or no browser.
- The experiment expands into minimize/hide, DevTools, split panes, multiple
  windows, or final matrix regression before fullscreen/unfullscreen behavior is
  isolated.

## Design Review

Fresh-context adversarial review requested two design fixes before
implementation.

Initial verdict: **CHANGES REQUIRED**.

Required findings:

- The identity proof omitted `window_id` behavior, even though the issue's
  canonical identity tuple includes `window_id`.
- The verification could pass a large resize without independently proving the
  window actually entered and exited native macOS fullscreen.

Fixes:

- Added explicit fullscreen and unfullscreen `window_id` expectations: either
  the same AppKit/CG window id survives, or the harness must record the current
  replacement window id and prove all post-transition AppKit and hit-test
  evidence is tied to that current window without accepting stale records.
- Added independent native fullscreen-state proof, such as `AXFullScreen`, after
  both enter and exit boundaries.

Final verdict after re-review: **APPROVED**.

Findings after re-review: none.

## Result

**Result:** Pass

Experiment 18 added the `fullscreen-unfullscreen` scenario to
`scripts/ghostboard-geometry-matrix.sh`. No Ghostboard, Roamium, or `webtui`
product source changes were needed.

The scenario uses accessibility automation against the native window's
`AXFullScreen` attribute, not private TermSurf state mutation. It launches one
browser in one Ghostboard window, enters native fullscreen, independently proves
`AXFullScreen=true`, verifies browser geometry/input in fullscreen, exits
fullscreen, independently proves `AXFullScreen=false`, and verifies browser
geometry/input again in windowed mode.

The passing run proved:

- baseline browser identity was native window id `495`, AppKit surface id
  `B99C145A-7B80-4739-81CF-CFC6AE7D13F8`, selected tab id `495`, pane id
  `B99C145A-7B80-4739-81CF-CFC6AE7D13F8`, browser tab id `1`, and context id
  `2888334459`;
- native fullscreen state became true after entering fullscreen and false after
  exiting fullscreen;
- the same AppKit/CG window id `495` survived both fullscreen and unfullscreen
  in this VM;
- the same surface id, selected tab id, pane id, browser tab id, and context id
  survived both transitions;
- fullscreen geometry grew from `856x510` / `1712x1020` pixels to `1752x1122` /
  `3504x2244` pixels at backing scale `2.0`;
- Roamium received the fullscreen AppKit pixel size through `ts_set_view_size`;
- fullscreen hit testing used the fullscreen frame, current window id, surface
  id, selected tab id, context id, and web-relative coordinates;
- Browse-mode keyboard input reached the same browser tab while fullscreen;
- unfullscreen geometry returned to `856x510` / `1712x1020` pixels at backing
  scale `2.0`;
- Roamium received the unfullscreen AppKit pixel size through
  `ts_set_view_size`;
- unfullscreen hit testing and Browse-mode keyboard input still routed to the
  same browser.

Verification:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
git diff --check
scripts/ghostboard-geometry-matrix.sh fullscreen-unfullscreen
scripts/ghostboard-geometry-matrix.sh window-resize
scripts/ghostboard-geometry-matrix.sh display-move-backing-scale
```

Passing evidence:

- Fullscreen/unfullscreen scenario:
  `logs/ghostboard-geometry-fullscreen-unfullscreen-harness-20260617-131025.log`
- Fullscreen/unfullscreen app log:
  `logs/ghostboard-geometry-fullscreen-unfullscreen-app-20260617-131025.log`
- Fullscreen/unfullscreen Roamium trace:
  `logs/ghostboard-geometry-fullscreen-unfullscreen-roamium-20260617-131025.log`
- Baseline screenshot:
  `logs/ghostboard-geometry-fullscreen-unfullscreen-screenshot-20260617-131025.png`
- Fullscreen screenshot:
  `logs/ghostboard-geometry-fullscreen-unfullscreen-fullscreen-screenshot-20260617-131025.png`
- Unfullscreen screenshot:
  `logs/ghostboard-geometry-fullscreen-unfullscreen-unfullscreen-screenshot-20260617-131025.png`
- Adjacent `window-resize` regression:
  `logs/ghostboard-geometry-window-resize-harness-20260617-131051.log`
- Adjacent `display-move-backing-scale` regression:
  `logs/ghostboard-geometry-display-move-backing-scale-harness-20260617-131107.log`

No product build was needed because only the shell harness and issue documents
changed.

## Completion Review

Fresh-context adversarial completion review approved the result before the
result commit.

Verdict: **APPROVED**.

Findings: none.

The reviewer inspected the working-tree diff for
`scripts/ghostboard-geometry-matrix.sh`, this experiment file, and the issue
README; checked the issue workflow, result documentation, README status,
fullscreen-state proof, current-window identity evidence, AppKit/resize/input
evidence, and verification logs; and confirmed the result commit had not yet
been made.

## Conclusion

Current Ghostboard preserves browser overlay geometry, identity, hit testing,
Roamium resize delivery, and keyboard routing through native macOS fullscreen
and unfullscreen transitions. The durable harness coverage proves native
fullscreen state independently with `AXFullScreen`, so a large ordinary resize
cannot satisfy this matrix row by accident.

In this VM the AppKit/CG window id was stable across fullscreen and
unfullscreen. The harness still records and validates the current window id
after each transition, so a future macOS environment that changes CG window ids
will have to prove the overlay is rebound to the current window rather than
passing on stale records.
