# Experiment 30: Validate Chromium-Output Roamium Lifecycle

## Description

Experiment 29 proved that `target/debug/roamium` is not a runnable browser path
on macOS because Chromium runtime resources are not beside that binary.
TermSurf's established Roamium build flow is different:
`./scripts/build.sh roamium` builds the Cargo binary and copies it into
`chromium/src/out/Default/roamium`, next to Chromium resources such as
`icudtl.dat`, `.pak` files, and `libtermsurf_chromium.dylib`.

This experiment will repeat the normal-tab lifecycle smoke test with the correct
repo-built browser artifact:
`/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium`.

The goal is to prove that Ghostboard can launch and coordinate the real Roamium
browser process without modifying `webtui`, `roamium`, Chromium, or the protobuf
schema. This remains a lifecycle experiment, not a native rendering or browser
input-forwarding experiment.

## Changes

Expected code changes are none unless the runtime validation discovers a
Ghostboard-side launch, environment, or protocol lifecycle defect.

If a fix is needed, keep it limited to the smallest relevant Ghostboard files,
likely one of:

- `ghostboard/src/apprt/termsurf.zig`
  - browser launch arguments, server matching, lifecycle state, or logging
    fixes;
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - terminal environment propagation fixes.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
Chromium, branding, icon assets, Xcode project files, CLI install behavior,
native browser overlay presentation, CALayerHost attachment, keyboard/mouse
browser input forwarding, DevTools duplicate detection, browser direct-client
routing changes, or browser process shutdown in this experiment.

## Verification

Pass criteria:

- Build the real `webtui` binary with `cargo build -p webtui`, with the command,
  cwd, and exit status recorded in a log.
- Build and place the real Roamium runtime artifact with
  `./scripts/build.sh roamium`, with the command, cwd, and exit status recorded
  in a log.
- Verify that `/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium`
  exists, is executable, and is at least as new as
  `/Users/astrohacker/dev/termsurf/target/debug/roamium`.
- Verify Chromium runtime resources exist beside the browser artifact, including
  `chromium/src/out/Default/icudtl.dat`,
  `chromium/src/out/Default/content_shell.pak`,
  `chromium/src/out/Default/shell_resources.pak`, and
  `chromium/src/out/Default/libtermsurf_chromium.dylib`.
- Record the timestamp and `otool -L` output for
  `chromium/src/out/Default/roamium` and
  `chromium/src/out/Default/libtermsurf_chromium.dylib`. This experiment assumes
  the existing Chromium output is the current repo build; it does not rebuild
  Chromium unless the runtime proves the existing output is stale or
  incompatible.
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
  `target/debug/web --browser /Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium https://example.com`
  inside the first terminal surface.
- App logs show Ghostboard spawned
  `/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium` with the
  expected browser-server arguments, including `--ipc-socket`,
  `--user-data-dir`, and `--listen-socket`.
- App logs show Roamium connected back as a browser server and sent
  `ServerRegister(profile=default)`.
- App logs show Ghostboard matched the Roamium server and sent `CreateTab` for
  the normal `webtui` pane.
- App logs show Roamium sent `TabReady` for the normal pane.
- App logs show Ghostboard sent `BrowserReady` to the normal `webtui` pane with
  a nonempty browser listen socket and browser path
  `/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium`.
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
  `chromium/src/out/Default/roamium` processes.
- `git diff --check` is clean.

Fail criteria:

- The runtime uses a fake helper, installed browser, or `target/debug/roamium`
  instead of `/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium`.
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

A fresh-context adversarial Codex subagent reviewed the Experiment 30 design and
returned **APPROVED** with no required findings.

The reviewer confirmed that the README links Experiment 30 as **Designed**, the
experiment has Description, Changes, and Verification sections, the design uses
`./scripts/build.sh roamium` and
`/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium`, avoids
`webtui`, `roamium`, Chromium, and protobuf changes, includes lifecycle proof
against the real repo-built Roamium path, checks for Chromium resources,
includes Rust/Zig/Swift hygiene, and requires `git diff --check`.

The reviewer had one optional finding: resource checks prove the Experiment 29
missing-ICU issue is avoided, but should also clarify whether the existing
Chromium output is assumed current. The design was updated to record timestamps
and `otool -L` output for the copied Roamium artifact and
`libtermsurf_chromium.dylib`, and to state that this experiment assumes the
existing Chromium output is current unless runtime evidence proves it stale or
incompatible.

## Result

**Result:** Pass

Experiment 30 validated the real repo-built Roamium artifact at
`/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium` with the
actual `target/debug/web` binary and without modifying `webtui`, `roamium`,
Chromium, or `proto/termsurf.proto`.

Build and artifact verification passed:

- `cargo build -p webtui` passed.
  - Log: `logs/ghostboard-exp30-cargo-build-webtui-20260616.log`
- `./scripts/build.sh roamium` passed and placed Roamium at
  `chromium/src/out/Default/roamium`.
  - Log: `logs/ghostboard-exp30-build-roamium-script-20260616.log`
- The artifact/resource check verified:
  - `chromium/src/out/Default/roamium` exists and is executable;
  - `chromium/src/out/Default/roamium` is newer than `target/debug/roamium`;
  - `icudtl.dat`, `content_shell.pak`, `shell_resources.pak`, and
    `libtermsurf_chromium.dylib` exist beside the Chromium-output Roamium
    binary;
  - `otool -L` output was recorded for `roamium` and
    `libtermsurf_chromium.dylib`.
  - Log: `logs/ghostboard-exp30-roamium-artifacts-20260616.log`
- The native GhosttyKit framework build passed after the Ghostboard logging
  change.
  - Log: `logs/ghostboard-exp30-zig-native-xcframework-20260616.log`
- The macOS app build passed after the Ghostboard logging change.
  - Log: `logs/ghostboard-exp30-macos-build-debug-after-logging-20260616.log`

The only source changes were Ghostboard-side diagnostic logging improvements in
`ghostboard/src/apprt/termsurf.zig`:

- `msgTypeName` now names every current TermSurf protobuf message case instead
  of logging browser-originated messages as `Other`. This was needed because the
  first runtime attempt reached Roamium page state, but the harness could not
  prove which page-state messages were received while they were logged as
  `Other`.
- The browser spawn log now includes the full browser argv so runtime logs prove
  `--ipc-socket`, `--user-data-dir`, and `--listen-socket`.

Formatting and hygiene:

- `zig fmt src/apprt/termsurf.zig src/main_c.zig src/build/SharedDeps.zig`
  passed.
  - Log: `logs/ghostboard-exp30-zig-fmt-20260616.log`
- No Rust code was modified, so no Rust formatting was required.
- No Swift code was modified, so no Swift linting was required.
- `git diff --check` passed.

The runtime harness launched
`ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf` with
`GHOSTTY_LOG=stderr` and a temporary config whose command was:

```text
/Users/astrohacker/dev/termsurf/target/debug/web --browser /Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium https://example.com
```

The harness proved the normal Roamium lifecycle:

- Ghostboard spawned
  `/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium` with the
  expected browser-server argv:

  ```text
  --ipc-socket=/var/folders/vx/wbmx10nd7tx8259xgg3v4vf80000gn/T/termsurf/termsurf-ghostboard-84821.sock
  --user-data-dir=/Users/astrohacker/.local/share/termsurf/chromium-profiles/default
  --listen-socket=/var/folders/vx/wbmx10nd7tx8259xgg3v4vf80000gn/T/termsurf/roamium-84821-default.sock
  ```

- Roamium bound the listen socket.
- Roamium connected back to Ghostboard and sent
  `ServerRegister(profile=default)`.
- Ghostboard matched the server key and sent `CreateTab` for
  `https://example.com`.
- Roamium sent `TabReady(tab_id=1)`.
- Ghostboard sent `BrowserReady` to the real `webtui` process with the Roamium
  listen socket and browser path.
- The real `webtui` process connected to Roamium's direct browser socket, proven
  by `[Roamium] client connected`.
- Roamium sent browser-originated page-state messages after `CreateTab`,
  including `CaContext`, `UrlChanged`, `TargetUrlChanged`, `LoadingState`, and
  `TitleChanged`.
- `web last` against the captured normal pane returned the normal Roamium tab:

  ```text
  profile: default
  pane_id: 4109BD34-38F5-44C6-A14E-745CA689E782
  tab_id:  1
  ```

- Runtime cleanup left no stale matching `TermSurf.app/Contents/MacOS/termsurf`,
  `target/debug/web`, or `chromium/src/out/Default/roamium` processes, and the
  GUI socket was removed.
  - Runtime harness log: `logs/ghostboard-exp30-runtime-harness-20260616.log`
  - App/Roamium log: `logs/ghostboard-exp30-runtime-app-20260616.log`
  - `web last` log: `logs/ghostboard-exp30-querylast-20260616.log`

One residual issue remains: Roamium still crashes during shutdown after the
harness terminates Ghostboard and the browser socket reaches EOF. The crash is
inside Chromium compositor shutdown: `cc::TileTaskManagerImpl::Shutdown()`. No
stale process remains, and the normal-tab launch/control lifecycle passed, so
this experiment passes. Shutdown crash cleanup is a separate lifecycle hardening
issue and should be addressed by a later experiment.

## Conclusion

Experiment 30 fixes the Experiment 29 mistake: Ghostboard must launch Roamium
from `chromium/src/out/Default/roamium`, not from `target/debug/roamium`,
because the Chromium-output directory contains the runtime resources and dylibs
Roamium needs.

With the correct browser path, Ghostboard can launch and coordinate the real
repo-built Roamium process for a normal `webtui` tab. The remaining major parity
work is no longer the basic Roamium lifecycle; it is native browser overlay
presentation, browser input forwarding, richer browser state handling, and
graceful browser shutdown.

## Completion Review

A fresh-context adversarial Codex subagent reviewed the completed Experiment 30
result and returned **CHANGES REQUIRED** with two required findings:

- The recorded runtime logs did not prove the browser argv contained
  `--ipc-socket`, `--user-data-dir`, and `--listen-socket`.
- The `zig fmt` and native GhosttyKit framework build logs did not record the
  command and cwd required by the experiment's pass criteria.

Both findings were accepted as real verification gaps.

Fixes:

- `ghostboard/src/apprt/termsurf.zig` now logs the full browser argv after
  successful spawn.
- `logs/ghostboard-exp30-zig-fmt-20260616.log` and
  `logs/ghostboard-exp30-zig-native-xcframework-20260616.log` were regenerated
  with `cwd:`, `cmd:`, and `exit_status=` entries.
- The native GhosttyKit framework build, macOS app build, and runtime harness
  were rerun after the argv logging fix.
- The final runtime logs now contain the required browser-server args and the
  harness asserts their presence before passing.

The same reviewer re-reviewed only the fixes and returned **APPROVED**. The
reviewer confirmed that:

- `ghostboard/src/apprt/termsurf.zig` now logs full browser argv after spawn;
- `logs/ghostboard-exp30-runtime-harness-20260616.log` and
  `logs/ghostboard-exp30-runtime-app-20260616.log` include `--ipc-socket`,
  `--user-data-dir`, and `--listen-socket`;
- the `zig fmt` and native GhosttyKit build logs now include `cwd:`, `cmd:`, and
  `exit_status=0`;
- `git diff --check` is clean;
- no new required findings were introduced.
