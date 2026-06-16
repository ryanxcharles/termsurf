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

## Result

**Result:** Fail

The experiment failed because the designed browser path
`/Users/astrohacker/dev/termsurf/target/debug/roamium` is not a runnable
Chromium resource layout on macOS.

Verification completed:

- Real `webtui` debug build passed:
  `logs/ghostboard-exp29-cargo-build-webtui-20260616.log`.
- Real `roamium` Cargo build passed:
  `logs/ghostboard-exp29-cargo-build-roamium-20260616.log`.
- No Rust source files were modified, so `cargo fmt` was not required.
- No Zig source files were modified, so `zig fmt` and the native GhosttyKit
  framework rebuild were not required.
- No Swift source files were modified, so `swiftlint` was not required.
- macOS app build passed:
  `logs/ghostboard-exp29-macos-build-debug-20260616.log`.
- Runtime harness failed: `logs/ghostboard-exp29-runtime-harness-20260616.log`.
- Runtime app log: `logs/ghostboard-exp29-runtime-app-20260616.log`.
- `git diff --check` passed.

Observed runtime failure:

```text
saw_spawn: True
saw_server_register: False
saw_tab_ready: False
saw_browser_ready: False
saw_resource_error: True
runtime verification failed: target/debug/roamium did not complete lifecycle
exit_status: 1
```

The app log proves Ghostboard did launch exactly the browser path required by
this experiment:

```text
spawned browser path=/Users/astrohacker/dev/termsurf/target/debug/roamium pid=82937 profile=default listen_socket=/var/folders/.../termsurf/roamium-82933-default.sock
```

Roamium then failed before connecting back to the Ghostboard GUI socket:

```text
icudtl.dat not found in bundle
Invalid file descriptor to ICU data received.
```

This is not evidence that Ghostboard cannot launch Roamium. It is evidence that
the experiment picked the wrong runnable path. The established TermSurf build
script does not run Roamium from `target/debug/roamium` directly; it copies the
Cargo-built binary into Chromium's output directory:

```text
cp "$REPO_DIR/target/debug/roamium" "$CHROMIUM_OUT/roamium"
```

That `chromium/src/out/Default/` directory contains the Chromium runtime
resources Roamium needs, including `icudtl.dat`, `.pak` files, and
`libtermsurf_chromium.dylib`.

## Conclusion

Experiment 29 eliminated `target/debug/roamium` as the correct runtime browser
path for this issue. The next experiment should use the established repo-built
Roamium layout by running `./scripts/build.sh roamium` and launching
`/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium`.

This preserves the important requirement that Ghostboard use a repo-built
Roamium binary without modifying `webtui`, `roamium`, Chromium, or the protocol.

## Completion Review

A fresh-context adversarial Codex subagent reviewed the completed Experiment 29
result and returned **APPROVED** with no findings.

The reviewer confirmed that the working tree contained only the expected issue
markdown updates, the README marks Experiment 29 as **Fail**, the experiment has
Result and Conclusion sections, the runtime logs prove Ghostboard launched
`/Users/astrohacker/dev/termsurf/target/debug/roamium`, the runtime logs prove
Roamium failed before lifecycle completion with missing Chromium resources, the
build logs show `cargo build -p webtui`, `cargo build -p roamium`, and the macOS
app build all exited 0, `git diff --check` is clean, and the next-experiment
conclusion is technically supported by `scripts/build.sh` copying
`target/debug/roamium` into `chromium/src/out/Default/roamium`.
