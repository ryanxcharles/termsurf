# Experiment 17: Display Move and Backing Scale

## Description

Experiment 16 proved multiple live browser windows remain isolated by native
window, AppKit surface, pane, browser tab, CA/context, mouse hit-testing, and
keyboard routing. The next matrix row is moving a window between displays.

This experiment should prove Ghostboard keeps a browser overlay aligned and
scaled when its owning native window moves between displays. If the current VM
exposes two or more displays to macOS, the harness should move one browser
window from the primary display to a different display and back. If the VM
exposes only one display, the experiment should record that the physical
cross-display transition is not available in this environment and still add
durable display inventory/backing-scale evidence for the single visible display.

The purpose is to test the display/backing-scale boundary without expanding into
fullscreen, minimize/hide, multiple browser windows, split panes, DevTools, or
final matrix regression.

If current Ghostboard already passes in the available display environment, the
experiment should record that and avoid product source changes. If it fails, the
harness must first localize whether the failure is accessibility window moving,
NSScreen/backing-scale reporting, AppKit layer resizing, Roamium resize
delivery, stale hit testing, or keyboard routing before any product fix is
designed.

## Changes

Planned files:

- `scripts/ghostboard-geometry-matrix.sh`
  - add a `display-move-backing-scale` scenario;
  - add a generated Swift display inventory helper that prints each visible
    display/screen id, frame, visible frame, and backing scale;
  - log the display inventory at scenario start;
  - launch one browser in one Ghostboard window using the repo-built `web` and
    Roamium binaries;
  - record the browser's canonical identity tuple:
    `window_id + surface_id + selected_tab_id + pane_id + browser_tab_id`, plus
    `context_id + AppKit frame + AppKit pixels + backing_scale`;
  - if two or more displays exist:
    - move the native window to a visible point on a non-primary display through
      accessibility APIs;
    - wait for AppKit geometry records after the move boundary;
    - require the browser to keep the same surface id, pane id, browser tab id,
      and context id;
    - require the overlay frame to stay aligned with the pane viewport;
    - require AppKit pixel size and/or backing scale to match the destination
      display's scale;
    - require Roamium to receive a resize if the AppKit pixel size changes;
    - click the moved browser and prove hit testing uses the moved window's
      current frame, surface id, selected tab id, context id, and web-relative
      coordinates;
    - enter Browse mode and prove keyboard input reaches the same browser tab
      after the display move;
    - move the window back to the original display and re-prove geometry,
      hit-testing, focus, and keyboard routing;
    - capture screenshots on the destination display and after return.
  - if only one display exists:
    - record a **Partial** result with the display inventory showing why a true
      cross-display move could not run in this VM;
    - still prove the current display's backing scale, AppKit frame, AppKit
      pixels, Roamium resize, hit-test coordinates, and keyboard routing for the
      browser on that display;
    - do not mark the full matrix row complete unless a later environment with
      multiple displays runs the cross-display path.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - change only if runtime evidence proves backing-scale changes or window moves
    do not recompute native overlay frame/pixels or hit-test coordinates.
- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
  - change only if runtime evidence proves window movement between displays is
    not visible to the terminal surface or accessibility automation.
- `ghostboard/src/apprt/termsurf.zig`
  - change only if runtime evidence proves the TermSurf resize/update messages
    omit data needed to track display scale changes.
- `roamium/src/dispatch.rs`
  - change only if existing trace evidence cannot prove resize/focus/key input
    after display movement. Any such change must be trace-only under the
    existing trace mechanism.
- `issues/0809-ghostboard-viewport-geometry/17-display-move-backing-scale.md`
  - record the design review, implementation, verification, completion review,
    result, and conclusion.
- `issues/0809-ghostboard-viewport-geometry/README.md`
  - add Experiment 17 to the experiment index.

Reference files:

- `scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostty-app/inject.swift`
- `scripts/ghostty-app/winid.swift`
- `issues/0809-ghostboard-viewport-geometry/03-window-resize-follow.md`
- `issues/0809-ghostboard-viewport-geometry/15-open-browser-in-new-window.md`
- `issues/0809-ghostboard-viewport-geometry/16-multiple-windows-with-browsers.md`
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
    issues/0809-ghostboard-viewport-geometry/17-display-move-backing-scale.md
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

- The new scenario runs:

  ```bash
  scripts/ghostboard-geometry-matrix.sh display-move-backing-scale
  ```

- If two or more displays are available, the scenario passes and proves:
  - the source and destination displays are distinct;
  - the browser keeps the same surface id, pane id, browser tab id, and context
    id after moving to the destination display and after moving back;
  - AppKit frame/pixels/backing scale agree with the current display after each
    move;
  - Roamium receives resize evidence when AppKit pixels change;
  - mouse hit-testing and Browse-mode keyboard input still route to the browser
    after each display move;
  - screenshots show the browser on the destination display and after return.
- If only one display is available, the scenario records **Partial** and proves:
  - display inventory evidence shows exactly one available display;
  - the browser's single-display AppKit frame, AppKit pixels, backing scale,
    Roamium resize, hit-test coordinates, and keyboard routing are valid;
  - the experiment file explicitly states that the cross-display matrix row
    remains incomplete until run on a multi-display environment.
- Adjacent geometry regressions still pass:

  ```bash
  scripts/ghostboard-geometry-matrix.sh window-resize
  scripts/ghostboard-geometry-matrix.sh open-browser-in-new-window
  ```

- `git diff --check` passes.
- The design review is recorded in this experiment file and the plan is
  committed before implementation begins.
- After implementation, verification, and result recording, the completion
  review is recorded in this experiment file and the result commit is made
  before designing or implementing Experiment 18.

Fail criteria:

- The harness fakes a display move by changing private geometry state instead of
  moving the native window.
- The harness claims the cross-display matrix row is complete in a one-display
  VM.
- A moved browser changes surface id, pane id, browser tab id, or context id.
- AppKit frame/pixels/backing scale or Roamium resize evidence contradicts the
  current display geometry.
- Mouse or keyboard input after a move reaches the wrong browser or no browser.
- The experiment expands into fullscreen, minimize/hide, DevTools, split panes,
  or final matrix regression before display movement/backing-scale behavior is
  isolated.

## Design Review

Fresh-context adversarial review approved the design before implementation.

Verdict: **APPROVED**.

Findings: none.

## Result

**Result:** Partial

Experiment 17 added the `display-move-backing-scale` scenario to
`scripts/ghostboard-geometry-matrix.sh`. No Ghostboard, Roamium, or `webtui`
product source changes were needed.

The current macOS VM exposes exactly one display to macOS:

```text
display_count=1
display=1 0 0 1779 1275 0 82 1779 1163 2.0 main
```

Because there is no destination display, the scenario correctly refused to claim
the cross-display matrix row as complete. It recorded the single-display limit
and still proved the available backing-scale path:

- browser A opened with native window id `481`, AppKit surface id
  `9D747774-AA19-45FC-88C2-73C45BD40BB6`, pane id
  `9D747774-AA19-45FC-88C2-73C45BD40BB6`, browser tab id `1`, context id
  `3691807747`, AppKit pixel size `1712x1020`, and backing scale `2.0`;
- Roamium received the initial AppKit-pixel resize;
- the scenario captured a single-display screenshot;
- clicking the browser on the available display produced a hit-test with the
  expected window id, surface id, selected tab id, AppKit frame, and
  web-relative coordinates;
- Browse-mode keyboard input reached the same Roamium browser tab/pane.

The harness includes a multi-display path for a machine that exposes two or more
screens. That path inventories displays, moves the native window to a different
display through accessibility APIs, revalidates frame/pixel/hit-test and
keyboard routing, moves the window back, and revalidates again. It remains
unexercised in this VM because no second display exists.

The implementation also hardened browser identity parsing in adjacent
multi-browser scenarios. Browser process permission logs can interleave with Zig
`tab_ready` lines in the combined app log, so second-and-later browser identity
extraction now keys off the later Zig `ca_context` records. Those records
include the pane id, browser tab id, and context id required by the geometry
proof, while AppKit remains the authoritative source for `surface_id`.

Verification:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
git diff --check
scripts/ghostboard-geometry-matrix.sh display-move-backing-scale
scripts/ghostboard-geometry-matrix.sh window-resize
scripts/ghostboard-geometry-matrix.sh open-browser-in-new-window
```

Passing/partial evidence:

- Display/backing-scale scenario:
  `logs/ghostboard-geometry-display-move-backing-scale-harness-20260617-130154.log`
- Display/backing-scale app log:
  `logs/ghostboard-geometry-display-move-backing-scale-app-20260617-130154.log`
- Display/backing-scale Roamium trace:
  `logs/ghostboard-geometry-display-move-backing-scale-roamium-20260617-130154.log`
- Single-display screenshot:
  `logs/ghostboard-geometry-display-move-backing-scale-moved-screenshot-20260617-130154.png`
- Adjacent `window-resize` regression:
  `logs/ghostboard-geometry-window-resize-harness-20260617-125743.log`
- Adjacent `open-browser-in-new-window` regression:
  `logs/ghostboard-geometry-open-browser-in-new-window-harness-20260617-125800.log`

No product build was needed because only the shell harness and issue documents
changed.

## Completion Review

Fresh-context adversarial completion review requested one fix before the result
commit.

Initial verdict: **CHANGES REQUIRED**.

Required finding:

- The unexercised multi-display branch could falsely pass without proving the
  display/backing-scale contract. It read the destination display scale but did
  not assert that the moved window landed on that display, did not validate
  moved AppKit `presented`/`presented_pixels` records, did not compare
  `backing_scale` to the destination scale, and did not require a Roamium resize
  when pixel size changed.

Fix:

- Tightened the multi-display branch so it now:
  - asserts the moved window center is inside the destination display;
  - waits for moved AppKit presentation and pixel records;
  - compares moved `backing_scale` to the destination display scale;
  - computes expected AppKit pixels from the moved frame and scale;
  - requires a matching Roamium resize if moved pixels differ from the original;
  - repeats source-display containment, scale, pixel, and resize checks after
    moving the window back.

Final verdict after re-review: **APPROVED**.

Findings after re-review: none.

## Conclusion

The VM cannot prove the physical cross-display matrix row because macOS reports
only one display. The experiment therefore leaves this row incomplete for final
issue closure, but it added durable display inventory logging and single-display
backing-scale coverage. A later run on a multi-display macOS environment should
reuse the same scenario to complete the cross-display path.

The useful harness learning is that display-related tests must report their
available screen topology before drawing conclusions. In a single-display VM, a
passing hit-test and keyboard-routing result proves only the current display's
scale path, not cross-display movement.
