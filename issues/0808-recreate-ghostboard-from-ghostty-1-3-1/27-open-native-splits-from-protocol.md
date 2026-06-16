# Experiment 27: Open Native Splits From Protocol

## Description

Experiment 26 gave every macOS Ghostboard terminal surface a `TERMSURF_PANE_ID`
that matches its native `SurfaceView.id`. That makes it possible for `webtui` to
send an `OpenSplit` request whose `pane_id` identifies an actual Ghostboard
surface.

`webtui` uses this path for DevTools: after `QueryDevtoolsRequest` succeeds, it
builds a command like
`<current web binary> --browser <browser> --profile <profile> devtools://<tab>`
and sends `OpenSplit(pane_id, direction, command)`. Fresh Ghostboard currently
decodes the protobuf schema but ignores `OpenSplit`, so the DevTools TUI process
is never launched inside a native terminal split.

This experiment will implement the narrow GUI-side `OpenSplit` bridge on macOS:
Zig will decode `OpenSplit`, copy the request strings, and call a C-callable
Swift bridge. Swift will hop to the main thread, find the target `SurfaceView`
by UUID, create a split `SurfaceConfiguration` inherited from that surface,
override `command` with the requested command, and call the existing
`BaseTerminalController.newSplit(at:direction:baseConfig:)`.

This experiment will prove that an `OpenSplit` protobuf message creates a native
Ghostboard split running the requested command, and that the new split receives
its own `TERMSURF_PANE_ID` plus the same `TERMSURF_SOCKET`. It will not require
or modify `webtui` or `roamium`, and it will not implement native browser
overlay presentation, browser input forwarding, duplicate DevTools detection, or
full `:devtools` end-to-end browser attachment.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - add `OpenSplit` to message dispatch;
  - add `OpenSplit` to `msgTypeName`;
  - validate that `pane_id`, `direction`, and `command` are present;
  - call a macOS bridge with the protobuf strings after logging the request;
  - keep the handler fire-and-forget, because native split creation runs on the
    AppKit main thread and the current protocol has no `OpenSplit` reply.
- `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`
  - add a C-callable Swift entry point such as
    `termsurf_open_split(pane_id, direction, command)`;
  - copy C strings to Swift strings before dispatching to the main thread;
  - parse `pane_id` as a UUID and find the target surface with the existing
    `AppDelegate.findSurface(forUUID:)`;
  - map `right`, `left`, `down`, and `up` to
    `SplitTree<Ghostty.SurfaceView>.NewDirection`;
  - build `Ghostty.SurfaceConfiguration` from
    `ghostty_surface_inherited_config(surface, GHOSTTY_SURFACE_CONTEXT_SPLIT)`;
  - override `config.command` with the requested command;
  - call the target window controller's existing `newSplit` method;
  - log missing app delegate, bad UUID, missing surface, bad direction, missing
    controller, and failed split cases.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, icon assets, Xcode project files, CLI install behavior, native browser
overlay presentation, keyboard/mouse browser input forwarding, DevTools
duplicate detection, or browser process lifecycle in this experiment.

## Verification

Pass criteria:

- Swift formatting/linting follows the nested Ghostboard instructions for the
  touched Swift file. During implementation, run `swiftlint lint --strict --fix`
  for the touched AppDelegate Swift bridge file, then run the non-mutating
  `swiftlint lint --strict` check and record the command, cwd, and exit status
  in logs.
- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passes inside `ghostboard/`, with the command, cwd, and exit status recorded
  in a log.
- The native GhosttyKit framework build passes:
  `zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`,
  with the command, cwd, and exit status recorded in a log. This must prove the
  Zig reference to the Swift bridge does not break the native framework build.
- The macOS app build passes:
  `macos/build.nu --scheme Ghostty --configuration Debug --action build`, with
  the command, cwd, and exit status recorded in a log. This must prove the Swift
  bridge symbol resolves in the app.
- Runtime harness launches `TermSurf.app` with a temporary config that makes the
  first terminal surface write `TERMSURF_PANE_ID` and `TERMSURF_SOCKET` to a
  capture file and then sleep.
- The harness connects to the captured `TERMSURF_SOCKET` and sends a
  length-prefixed `OpenSplit` protobuf message with:
  - `pane_id` equal to the first surface's captured UUID;
  - `direction = right`;
  - `command` set to a shell command that appends the new split's
    `TERMSURF_PANE_ID` and `TERMSURF_SOCKET` to the same capture file.
- The capture file gains a second line without using System Events keyboard
  automation, proving the split was created by the protocol message rather than
  by a UI shortcut.
- The second line's pane id is nonempty, parses as a UUID, and differs from the
  first pane id.
- The second line's socket is identical to the first socket and uses the
  Ghostboard socket namespace.
- App logs include an `OpenSplit` request log and a successful Swift bridge
  split log.
- Runtime shutdown removes the socket file and leaves no stale launched
  `TermSurf.app/Contents/MacOS/termsurf` process.
- Negative runtime checks send malformed or unresolvable `OpenSplit` requests
  and verify:
  - a bad UUID does not create a new capture line and logs a rejection;
  - an unknown direction does not create a new capture line and logs a
    rejection.
- `git diff --check` is clean.

Fail criteria:

- `OpenSplit` remains ignored.
- `OpenSplit` creates a split for the wrong target pane.
- `OpenSplit` creates a split through System Events or keyboard automation
  instead of the protobuf handler.
- The new split launches without the requested command.
- The new split inherits the parent pane id instead of receiving its own
  `TERMSURF_PANE_ID`.
- The new split loses `TERMSURF_SOCKET`.
- A bad UUID or unknown direction creates a split.
- The implementation modifies `webtui`, `roamium`, `proto/termsurf.proto`, Xcode
  project files, native browser overlay presentation, browser input forwarding,
  DevTools duplicate detection, or browser process lifecycle in this experiment.

## Design Review

A fresh-context adversarial Codex subagent reviewed the Experiment 27 design and
returned **APPROVED** with no findings.
