# Experiment 26: Propagate Pane IDs to Surfaces

## Description

Experiments 20 through 25 made Ghostboard's GUI socket capable of tracking TUI
and browser state, creating normal browser tabs, and creating DevTools browser
tabs once a DevTools TUI process exists. The next prerequisite for real `webtui`
workflows is pane identity propagation.

`webtui` reads `TERMSURF_PANE_ID` from its process environment. When that
variable is missing, `webtui` does not create a TermSurf compositor connection,
so it cannot send `SetOverlay`, `QueryLast`, `QueryDevtools`, or `OpenSplit`.
Ghostboard Legacy generated a UUID for each surface and propagated it to child
processes as `TERMSURF_PANE_ID`. The fresh Ghostty 1.3.1 based `ghostboard/`
tree already assigns each macOS `SurfaceView` a `UUID`, and
`SurfaceConfiguration` already supports per-surface environment variables, but
new surfaces currently do not inject `TERMSURF_PANE_ID`.

This experiment will make each Ghostboard surface set `TERMSURF_PANE_ID` to its
own `SurfaceView.id.uuidString` before creating the underlying Ghostty surface.
That gives any shell or command launched inside the terminal a stable pane id
that matches the native surface identity. It also ensures future `OpenSplit`
work can target a real surface by UUID.

This experiment will not implement `OpenSplit`, native split creation from the
TermSurf protocol, browser overlay presentation, input forwarding, or changes to
`webtui`, `roamium`, `proto/termsurf.proto`, app branding, CLI installation, or
browser process lifecycle.

## Changes

- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - make the surface configuration mutable while constructing a macOS
    `SurfaceView`;
  - set `surface_cfg.environmentVariables["TERMSURF_PANE_ID"]` to
    `self.id.uuidString` before calling `ghostty_surface_new`;
  - preserve all existing inherited environment variables and only override
    `TERMSURF_PANE_ID`, because a split or restored surface must receive its own
    current UUID rather than inheriting a parent pane id.

No changes will be made to `ghostboard/src/apprt/termsurf.zig`, `webtui`,
`roamium`, `proto/termsurf.proto`,
`ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_UIKit.swift`, app
branding, icon assets, Xcode project files, Zig build files, CLI install
behavior, native overlay presentation, input forwarding, `OpenSplit`, or browser
process shutdown in this experiment.

## Verification

Pass criteria:

- Swift formatting/linting follows the nested Ghostboard instructions for the
  touched Swift file. During implementation, run
  `swiftlint lint --strict --fix "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"`
  inside `ghostboard/`, then run the non-mutating check
  `swiftlint lint --strict "macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift"`
  and record the command, cwd, and exit status in logs.
- The native GhosttyKit framework build passes:
  `zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`,
  with the command, cwd, and exit status recorded in a log.
- The macOS app build passes:
  `macos/build.nu --scheme Ghostty --configuration Debug --action build`, with
  the command, cwd, and exit status recorded in a log.
- Runtime harness launches `TermSurf.app` with a temporary config or launch
  setup that runs a command inside the first terminal surface and writes the
  values of `TERMSURF_PANE_ID` and `TERMSURF_SOCKET` to a temporary file.
- The captured `TERMSURF_PANE_ID` is nonempty and parses as a UUID string.
- The captured `TERMSURF_SOCKET` is nonempty and points at the expected
  Ghostboard GUI socket namespace.
- The runtime harness creates a second terminal surface by using Ghostty's
  existing native split action and captures its environment.
- The second surface's `TERMSURF_PANE_ID` is nonempty, parses as a UUID string,
  and is different from the first surface's pane id.
- The second surface still receives the same `TERMSURF_SOCKET`, proving pane ids
  are per surface while the GUI socket remains per app instance.
- App logs do not show surface-creation, environment-variable, or terminal
  command launch errors.
- The runtime harness verifies shutdown cleanup removes the socket file and
  leaves no stale `TermSurf.app/Contents/MacOS/termsurf` process.
- `git diff --check` is clean.

Fail criteria:

- A terminal command launched inside Ghostboard does not receive
  `TERMSURF_PANE_ID`.
- `TERMSURF_PANE_ID` is empty or not a UUID string.
- Two different terminal surfaces inherit the same `TERMSURF_PANE_ID`.
- Setting `TERMSURF_PANE_ID` drops existing inherited environment variables such
  as `TERMSURF_SOCKET`.
- The implementation edits UIKit without an iOS verification path.
- The implementation changes TermSurf protocol handling, `OpenSplit`, browser
  process lifecycle, `webtui`, `roamium`, or the protobuf schema in this
  experiment.

## Design Review

A fresh-context adversarial Codex subagent reviewed the initial design and
returned **CHANGES REQUIRED** with two required findings:

- The verification section omitted the Swift formatting/linting gate required by
  `ghostboard/AGENTS.md` and `ghostboard/macos/AGENTS.md`.
- The experiment included a UIKit source edit even though the verification plan
  only proved the macOS app build and runtime behavior.

The design was updated to keep this experiment macOS-only, leave
`SurfaceView_UIKit.swift` out of scope, and add both the mutating `swiftlint`
format pass and a non-mutating `swiftlint` verification check for the touched
AppKit Swift file.

The reviewer re-reviewed those fixes and approved the design with no remaining
required findings.

## Result

**Result:** Pass

Implemented pane id propagation for macOS Ghostboard surfaces in
`ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`.

The implementation now makes the per-surface `SurfaceConfiguration` mutable
while constructing a macOS `SurfaceView`, then sets
`TERMSURF_PANE_ID = self.id.uuidString` before `ghostty_surface_new` starts the
underlying terminal IO. Existing inherited environment variables are preserved,
and `TERMSURF_PANE_ID` is intentionally overwritten at construction time so a
new split receives its own UUID instead of inheriting the parent surface's pane
id.

Verification passed:

- Swift format/lint fix pass succeeded:
  `logs/ghostboard-exp26-swiftlint-fix-20260616.log`.
- Swift non-mutating lint check succeeded with 0 violations:
  `logs/ghostboard-exp26-swiftlint-check-20260616.log`.
- Native GhosttyKit framework build passed:
  `logs/ghostboard-exp26-zig-native-xcframework-20260616.log`.
- macOS app build passed:
  `logs/ghostboard-exp26-macos-build-debug-20260616.log`.
- Runtime harness passed: `logs/ghostboard-exp26-runtime-harness-20260616.log`.
- Runtime app log: `logs/ghostboard-exp26-runtime-app-20260616.log`.
- `git diff --check` passed.

Observed successful runtime checks:

```text
PASS: socket namespace is Ghostboard
PASS: first pane id 9FE1B730-7822-4AAA-8FCE-ACEFF0CC50BC
PASS: second pane id 3F05DE57-06D5-42E3-AE5C-5A277E211A47
PASS: shared socket /var/folders/vx/wbmx10nd7tx8259xgg3v4vf80000gn/T/termsurf/termsurf-ghostboard-78130.sock
PASS: app exited and socket cleaned up
PASS: app log has no targeted surface/env/launch error markers
runtime verification passed
```

The runtime harness launched the built `TermSurf.app` with a temporary
`GHOSTTY_CONFIG_PATH`. The config made every terminal surface append
`TERMSURF_PANE_ID` and `TERMSURF_SOCKET` to a temporary capture file. The first
surface wrote a UUID pane id and the expected Ghostboard GUI socket. The harness
then used System Events to send the native `Cmd-D` split shortcut, causing a
second terminal surface to launch the same command. The second surface wrote a
different UUID pane id and the same GUI socket, proving pane identity is per
surface while the socket remains per app instance.

During verification, direct custom AppleScript commands such as
`count terminals` and app-level `quit` against the built app hung in this VM.
System Events keyboard automation did work and successfully triggered Ghostty's
native split action. This is useful for future experiments that need to drive
native Ghostboard UI state without depending on custom AppleScript handlers.

## Conclusion

Ghostboard now provides `TERMSURF_PANE_ID` to terminal child processes on macOS.
This unblocks real `webtui` compositor connections from processes launched
inside Ghostboard surfaces and gives future `OpenSplit` work a concrete surface
UUID to target. The next experiment should implement the GUI-side `OpenSplit`
bridge that uses those pane ids to create a native split running the requested
command.

## Completion Review

A fresh-context adversarial Codex subagent reviewed the completed Experiment 26
result and returned **APPROVED** with no findings.

The reviewer confirmed that the implementation stayed within the approved
AppKit-only scope, did not edit UIKit or protocol code, set `TERMSURF_PANE_ID`
to the current macOS `SurfaceView` UUID before `ghostty_surface_new`, preserved
inherited environment variables, and recorded the required Swift lint, native
framework build, macOS app build, runtime two-surface verification, shutdown
cleanup, and `git diff --check` evidence.
