# Experiment 29: Validate Roamium Normal Tab Lifecycle

## Description

Experiment 28 proved that the real `webtui` binary can drive Ghostboard's
DevTools orchestration when a fake helper stands in for Roamium. The next
increment is to replace that helper with the actual `target/debug/roamium`
binary and prove Ghostboard can launch and coordinate a normal browser tab with
the current Roamium process.

This experiment is intentionally a lifecycle smoke test, not a rendering or
input-forwarding experiment. The expected result is that the app built from
`ghostboard/` launches `target/debug/roamium`, Roamium connects back with
`ServerRegister(profile=default)`, Ghostboard sends `CreateTab`, Roamium replies
with `TabReady` and sends browser-originated state such as `CaContext`, and
Ghostboard sends `BrowserReady` to the real `webtui` process.

If this fails, this experiment may make the smallest necessary Ghostboard-side
fix to launch or coordinate the existing `webtui` and `roamium` binaries. It
will not change `webtui`, `roamium`, `proto/termsurf.proto`, Chromium, or the
browser rendering/input surface.

## Changes

Expected code changes are none unless the runtime validation discovers a
Ghostboard-side launch, environment, or protocol lifecycle defect.

If a fix is needed, keep it limited to the smallest relevant Ghostboard files,
likely one of:

- `ghostboard/src/apprt/termsurf.zig`
  - browser launch arguments, server matching, lifecycle state, or logging
    fixes;
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - terminal environment propagation fixes;
- `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`
  - only if the runtime path exposes a split or surface-lookup issue.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
Chromium, branding, icon assets, Xcode project files, CLI install behavior,
native browser overlay presentation, CALayerHost attachment, keyboard/mouse
browser input forwarding, DevTools duplicate detection, browser direct-client
routing, or browser process shutdown in this experiment.

## Verification

Pass criteria:

- Build the real `webtui` binary with `cargo build -p webtui`, with the command,
  cwd, and exit status recorded in a log.
- Build the real `roamium` binary with `cargo build -p roamium`, with the
  command, cwd, and exit status recorded in a log.
- If Rust code is modified, run `cargo fmt` as required by `AGENTS.md`. If no
  Rust code is modified, explicitly record that no Rust formatting was required.
- If Zig code is modified, run
  `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  inside `ghostboard/`, with the command, cwd, and exit status recorded in a
  log.
- If Swift code is modified, run the nested Ghostboard `swiftlint` fix and
  non-mutating lint checks for touched Swift files, with commands, cwd, and exit
  statuses recorded in logs.
- If Ghostboard code is modified, the native GhosttyKit framework build passes:
  `zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`,
  with the command, cwd, and exit status recorded in a log.
- The macOS app build passes:
  `macos/build.nu --scheme Ghostty --configuration Debug --action build`, with
  the command, cwd, and exit status recorded in a log.
- Runtime harness launches `TermSurf.app` with `GHOSTTY_LOG=stderr` and a
  temporary config whose command runs the actual
  `target/debug/web --browser /Users/astrohacker/dev/termsurf/target/debug/roamium https://example.com`
  inside the first terminal surface.
- App logs show Ghostboard spawned
  `/Users/astrohacker/dev/termsurf/target/debug/roamium` with the expected
  browser-server arguments, including `--ipc-socket`, `--user-data-dir`, and
  `--listen-socket`.
- App logs show Roamium connected back as a browser server and sent
  `ServerRegister(profile=default)`.
- App logs show Ghostboard matched the Roamium server and sent `CreateTab` for
  the normal `webtui` pane.
- App logs show Roamium sent `TabReady` for the normal pane.
- App logs show Ghostboard sent `BrowserReady` to the normal `webtui` pane with
  a nonempty browser listen socket and browser path
  `/Users/astrohacker/dev/termsurf/target/debug/roamium`.
- Logs show Roamium sent at least one browser-originated page state message
  after `CreateTab`, preferably `CaContext`. It is acceptable in this experiment
  if Ghostboard logs that message as ignored, because native overlay
  presentation is explicitly out of scope.
- The normal `webtui` process receives `BrowserReady` and connects to Roamium's
  direct browser socket. This can be proven by downstream app/Roamium log
  activity rather than screen scraping.
- A `web status` or `web last` query against the captured normal pane
  `TERMSURF_SOCKET` and `TERMSURF_PANE_ID` returns the normal Roamium tab.
- Runtime shutdown removes the GUI socket file and leaves no stale matching
  `TermSurf.app/Contents/MacOS/termsurf`, `target/debug/web`, or
  `target/debug/roamium` processes.
- `git diff --check` is clean.

Fail criteria:

- The runtime uses a fake helper or installed browser instead of
  `/Users/astrohacker/dev/termsurf/target/debug/roamium`.
- Roamium is modified to accommodate Ghostboard.
- `webtui` is modified to accommodate Ghostboard.
- Ghostboard does not launch Roamium or launches it without the required
  `--ipc-socket` / `--listen-socket` arguments.
- Roamium does not connect back with `ServerRegister(profile=default)`.
- Ghostboard does not send `CreateTab` to the attached Roamium server.
- Roamium does not send `TabReady`.
- Ghostboard does not send `BrowserReady` to `webtui`.
- `web last` / `web status` cannot find the normal Roamium tab after
  `BrowserReady`.
- The implementation adds CALayerHost overlay presentation, keyboard/mouse
  browser input forwarding, DevTools duplicate detection, browser shutdown,
  browser direct-client routing changes, Chromium changes, `webtui` changes,
  `roamium` changes, or protobuf schema changes in this experiment.

## Design Review

A fresh-context adversarial Codex subagent reviewed the Experiment 29 design and
returned **APPROVED** with no required findings.

The reviewer confirmed that the README links Experiment 29 as **Designed**, the
experiment has Description, Changes, and Verification sections, the scope is
limited to Ghostboard-side lifecycle fixes only, the verification requires the
real `/Users/astrohacker/dev/termsurf/target/debug/roamium` binary rather than a
fake helper or installed browser, the lifecycle proof covers spawn arguments,
`ServerRegister`, `CreateTab`, `TabReady`, `BrowserReady`, direct browser socket
connection, query visibility, process cleanup, and the required hygiene checks.
