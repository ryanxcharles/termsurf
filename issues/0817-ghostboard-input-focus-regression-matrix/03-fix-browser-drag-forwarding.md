# Experiment 3: Fix Browser Drag Forwarding

## Description

Experiment 2 added the `browser-input-granularity` scenario and proved ordinary
browser text input, special keys, caret/focus state, click counts, and
modifier-click. It failed only at browser drag selection. The logs showed
Roamium received the drag down/up and mouse move events, but the final drag move
arrived with `modifiers=0`; Chromium therefore did not see an active left-button
drag and the page reported an empty browser selection.

This experiment will make the smallest Ghostboard app fix for that scoped
failure: TermSurf mouse moves generated from AppKit drag events must preserve
the active mouse-button modifier before forwarding to Roamium.

## Changes

Planned source changes:

- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - In `forwardTermSurfMouseMove`, preserve explicit button-state bits for
    AppKit drag events:
    - `.leftMouseDragged` keeps the left-button modifier bit;
    - `.rightMouseDragged` keeps the right-button modifier bit;
    - `.otherMouseDragged` keeps the middle/other-button modifier bit.
  - Keep ordinary hover/move forwarding behavior unchanged.
  - If existing AppKit geometry logs are not enough to prove terminal-selection
    suppression, add a narrow TermSurf/AppKit trace log for forwarded overlay
    mouse events and drag moves so the harness can assert that browser-drag
    down/move/up events were consumed by the overlay path and did not fall
    through to terminal selection handling.

Planned issue-document changes:

- Record the result in this experiment file.
- Update the Issue 817 README status for Experiment 3 after verification.

## Verification

Formatting actions:

1. `prettier --write --prose-wrap always --print-width 80 issues/0817-ghostboard-input-focus-regression-matrix/README.md issues/0817-ghostboard-input-focus-regression-matrix/03-fix-browser-drag-forwarding.md`.

Static checks:

1. `git diff --check`.
2. `bash -n scripts/ghostboard-geometry-matrix.sh`.
3. `swiftc -typecheck scripts/ghostty-app/inject.swift`.

Build checks:

1. From `ghostboard/macos`, run
   `./build.nu --configuration Debug --action build`.

Runtime checks:

1. `scripts/ghostboard-geometry-matrix.sh browser-input-granularity`.

Pass criteria:

- The app build succeeds.
- `browser-input-granularity` passes.
- The passing Roamium trace shows the browser drag move carries the active
  left-button modifier instead of `modifiers=0`.
- The page reports non-empty browser drag selection for
  `ISSUE817_BROWSER_DRAG_TEXT`.
- Browse-mode `Cmd+C` copies `ISSUE817_BROWSER_DRAG_TEXT` to the clipboard after
  the browser drag.
- Terminal-selection suppression is directly proven by a reliable observable
  that would fail if terminal selection were created during the browser drag,
  such as AppKit/Ghostboard trace logs showing the drag down/move/up were
  consumed by TermSurf overlay forwarding with no terminal fallback, or an
  equivalent selection-state/screenshot assertion.
- Existing text input, special-key, caret/focus, click-count, and modifier-click
  assertions in the scenario still pass.

Partial criteria:

- The drag move carries the active button modifier and browser drag selection is
  proven, but terminal-selection suppression still lacks a reliable observable.
- The app fix builds, but `browser-input-granularity` exposes a different
  already-existing failure unrelated to drag forwarding.

Fail criteria:

- The app build fails.
- Drag moves still reach Roamium without active button modifiers.
- Browser drag selection remains empty.
- The fix changes ordinary hover/move routing or regresses the already-passing
  keyboard/click rows.

## Design Review

This experiment is plan-only until a fresh-context adversarial design review
approves it. Record the reviewer verdict here, fix all real findings, then
commit the approved plan before implementation begins.

Fresh-context adversarial design review by Codex subagent `Banach`:

- **Initial verdict:** Changes required.
- **Finding 1:** The pass criteria treated Browse-mode clipboard copy as proof
  of terminal-selection suppression. Fixed by requiring a direct suppression
  observable that would fail if terminal selection were created during browser
  drag, such as AppKit/Ghostboard forwarded-overlay logs with no terminal
  fallback or an equivalent selection-state/screenshot assertion.
- **Final verdict:** Approved. The reviewer confirmed the prior Required finding
  was resolved and no new Required findings were introduced.

## Completion Gate

After implementation and verification:

- add `## Result` and `## Conclusion` to this experiment file;
- update the Issue 817 README experiment status from `Designed` to `Pass`,
  `Partial`, or `Fail`;
- request a fresh-context completion review;
- fix all real completion-review findings and record the final verdict in this
  file; and
- commit the reviewed result separately before designing or implementing the
  next experiment.
