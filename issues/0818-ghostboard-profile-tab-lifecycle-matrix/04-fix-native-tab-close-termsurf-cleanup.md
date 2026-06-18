# Experiment 4: Fix Native Tab-Close TermSurf Cleanup

## Description

Experiment 3 proved same-profile server reuse through browser A, browser B, and
reopened browser C, but it could not prove normal cleanup after closing browser
C. The failing run reached C input routing, then `Cmd+W` produced AppKit
close-tab key-equivalent logs without a timely `CloseTab` for C. The C pane was
only cleaned up later during teardown, after Roamium's socket was already
closed, which produced `CloseTab send failed`.

This experiment will fix and prove the native tab-close lifecycle path for
TermSurf browser panes. The current source shape suggests split close goes
through `BaseTerminalController.removeSurfaceNode`, which calls
`termsurf_pane_closed`, while native tab close uses
`TerminalController.closeTabImmediately` and only calls `window.close()`.
Therefore a closed native tab can leave its TermSurf pane live until the TUI
process disconnects.

The goal is not to redesign server lifecycle. The narrow goal is to make normal
native tab close notify TermSurf while the browser socket is still writable,
then rerun the same-profile lifecycle scenario far enough to prove browser C's
normal close cleanup. If the run then exposes final profile-server shutdown as a
separate missing behavior, record that as the next issue-local experiment rather
than hiding it.

## Changes

Planned source investigation and likely code changes:

- `ghostboard/macos/Sources/Features/Terminal/TerminalController.swift`
  - inspect `closeTabImmediately`, `closeWindowImmediately`, and
    `windowWillClose`;
  - add the smallest native tab-close cleanup call needed before
    `window.close()` removes the tab window;
  - preserve undo/tab restore behavior and avoid duplicate cleanup when full
    window close or split close already handles the panes.
- `ghostboard/macos/Sources/Features/Terminal/BaseTerminalController.swift`
  - reuse existing surface-tree cleanup semantics if possible instead of adding
    a second per-pane cleanup implementation;
  - keep cross-window drag/move behavior intact, especially the existing
    `closeTermSurfPanes: false` path.
- `ghostboard/src/apprt/termsurf.zig`
  - inspect only unless the Swift lifecycle fix is insufficient;
  - if changed, keep cleanup idempotent for repeated pane close and later TUI
    disconnect cleanup.
- `scripts/ghostboard-geometry-matrix.sh`
  - extend or reuse the existing `same-profile-server-lifecycle` scenario so its
    pass criteria explicitly require browser C `CloseTab` before teardown,
    browser A interactivity after C closes, and final server cleanup evidence if
    the scenario reaches final profile closure;
  - if final profile-server shutdown is still missing after browser C cleanup is
    fixed, split the scenario result so browser C native-close cleanup can be
    recorded without hard-failing before the result documents the remaining
    final-shutdown gap.

No Chromium changes are planned.

## Verification

Static checks:

- Format edited markdown:
  `prettier --write --prose-wrap always --print-width 80 issues/0818-ghostboard-profile-tab-lifecycle-matrix/README.md issues/0818-ghostboard-profile-tab-lifecycle-matrix/04-fix-native-tab-close-termsurf-cleanup.md`
- If Zig changes are needed, run `zig fmt` on each changed `.zig` file.
- If Swift changes are made, run `swiftlint lint --strict` from
  `ghostboard/macos`.
- If any code outside `ghostboard/macos/` changes, run
  `zig build -Demit-macos-app=false` from `ghostboard` before the macOS app
  build.
- Build the macOS app with
  `cd ghostboard && macos/build.nu --configuration Debug --action build`.
- Run `git diff --check`.
- Run `bash -n scripts/ghostboard-geometry-matrix.sh` if the harness changes.

Runtime check:

- Run `scripts/ghostboard-geometry-matrix.sh same-profile-server-lifecycle`.

Pass criteria:

- The scenario proves browser A creates the `default/${ROAMIUM}` server.
- Browser B launched with the same profile reuses the same server/pid and does
  not spawn a duplicate default-profile Roamium.
- Closing browser B sends normal `CloseTab` while Roamium is attached, destroys
  the B tab in Roamium, and leaves browser A interactive.
- Reopened browser C uses the same server/pid and receives a fresh pane/tab or
  context identity distinct from browser A and B.
- Closing browser C sends normal native tab-close cleanup before harness
  teardown: the evidence must include `Pane close cleanup` and `CloseTab` for C
  while Roamium is attached, must destroy the C tab in Roamium, and must not
  rely on `TUI disconnect cleanup` or `CloseTab send failed` as the cleanup
  evidence.
- Browser A remains interactive after browser C closes.
- Final closure of browser A produces either a clean server process exit/removal
  or explicit evidence that profile-server shutdown remains the next missing
  behavior.

Partial criteria:

- Browser C native tab close is fixed and proven, but final server process
  cleanup remains unimplemented or unproven.
- The code fix is correct but the scenario exposes an unrelated harness flake
  that prevents final process-cleanup proof.

Fail criteria:

- Browser C native tab close still lacks a timely `CloseTab`.
- Browser C cleanup works only through a fallback TUI disconnect path rather
  than the normal native tab-close path.
- Same-profile server reuse regresses.
- Browser input routing leaks to closed panes or wrong profile/server contexts.
- The fix breaks split close, close-window behavior, undo/tab restore, or
  cross-window surface movement.

## Design Review

Adversarial design review by `Kant the 2nd` returned `CHANGES REQUIRED`.

Required findings and fixes:

- Missing Swift/macOS hygiene checks. Fixed by adding `swiftlint lint --strict`,
  `macos/build.nu --configuration Debug --action build`, and the required
  underlying Zig build when non-macOS code changes.
- Fallback TUI-disconnect cleanup was allowed as `Partial` even though the
  experiment is specifically about native tab-close cleanup. Fixed by moving
  fallback-only cleanup to `Fail` and requiring C cleanup evidence from the
  native tab-close path before teardown.
- Final server-cleanup criteria conflicted with the current harness, which
  hard-fails if the shared Roamium pid survives final close. Fixed by explicitly
  allowing a harness/result split if final profile-server shutdown is still the
  next missing behavior after C native-close cleanup is proven.

Re-review approved with no required findings. `Kant the 2nd` verified the macOS
hygiene checks, native-close-only cleanup criteria, and final-shutdown criteria
are now aligned with the experiment goal and current harness behavior.

## Result

**Result:** Pass

Implemented the native tab-close cleanup and final profile-server cleanup path.
The passing run was:

```text
scripts/ghostboard-geometry-matrix.sh same-profile-server-lifecycle
timestamp: 20260618-022204
```

Runtime evidence:

- Harness log:
  `logs/ghostboard-geometry-same-profile-server-lifecycle-harness-20260618-022204.log`
- App log:
  `logs/ghostboard-geometry-same-profile-server-lifecycle-app-20260618-022204.log`
- Roamium trace:
  `logs/ghostboard-geometry-same-profile-server-lifecycle-roamium-20260618-022204.log`

Passing observations:

- Browser A created the `default/${ROAMIUM}` server and spawned shared Roamium
  pid `30820`.
- Browser B reused the same server and pid, then native tab close selected
  browser A, sent `CloseTab`, destroyed/removed browser B in Roamium, and did
  not leak input.
- Browser C reopened with the same profile/server/pid, received fresh
  pane/tab/context identity, routed input only to browser C, and native tab
  close sent timely `Pane close cleanup` / `CloseTab` before teardown.
- Late `SetOverlay` messages from already-closed native tabs were ignored for
  the same TUI fd, preventing closed panes from recreating hidden Roamium tabs.
- Browser A remained interactive after browser C closed.
- Final browser A close sent `CloseTab`; Roamium removed the last tab and
  exited; Ghostboard reaped the child; the harness reported
  `PASS: shared Roamium pid exited after final browser close`.

Verification run:

- `prettier --write --prose-wrap always --print-width 80 issues/0818-ghostboard-profile-tab-lifecycle-matrix/README.md issues/0818-ghostboard-profile-tab-lifecycle-matrix/04-fix-native-tab-close-termsurf-cleanup.md`
- `zig fmt src/apprt/termsurf.zig`
- `zig build -Demit-macos-app=false` from `ghostboard`
- `cd ghostboard && macos/build.nu --configuration Debug --action build`
- `swiftlint lint --strict Sources/Features/Terminal/BaseTerminalController.swift Sources/Features/Terminal/TerminalController.swift`
  from `ghostboard/macos`
- `cargo fmt -- roamium/src/dispatch.rs roamium/src/ffi.rs`
- `./scripts/build.sh roamium`
- `git diff --check`
- `bash -n scripts/ghostboard-geometry-matrix.sh`
- `scripts/ghostboard-geometry-matrix.sh same-profile-server-lifecycle`

Notes:

- Full `swiftlint lint --strict` currently reports an unrelated existing
  violation in `Sources/Ghostty/Surface View/SurfaceView_AppKit.swift:2175`. The
  edited Swift files passed strict lint, and the macOS app build succeeded.

## Conclusion

Native tab close now notifies TermSurf before the AppKit tab window disappears,
so browser panes close through the same cleanup semantics that split close
already used. TermSurf also blocks late `SetOverlay` messages from a TUI fd
after the GUI has closed that pane, which prevents hidden browser tab recreation
during terminal teardown.

When a profile server's final pane closes, Ghostboard now shuts down the browser
socket, clears the server state, and reaps the Roamium child process. Roamium
also exits deterministically after its last tab is removed. Together these fixes
prove same-profile reuse, close/reopen, and final process cleanup in the
`same-profile-server-lifecycle` guard.

## Completion Review

Adversarial completion review by `Galileo the 2nd` approved the experiment
result with no findings.
