# Experiment 28: Validate Webtui DevTools Flow

## Description

Experiments 25 through 27 implemented the protocol pieces needed for DevTools:

- `QueryDevtoolsRequest` can resolve an existing browser tab.
- `SetDevtoolsOverlay` can create a browser-side DevTools tab for an attached
  browser server.
- `TERMSURF_PANE_ID` is propagated into terminal child processes.
- `OpenSplit` can create a native Ghostboard split running a requested command.

Those experiments proved the individual protocol pieces with direct socket
harnesses. The remaining question is whether the real `webtui` binary can drive
the whole workflow inside Ghostboard without any changes to `webtui` or
`roamium`.

This experiment will launch the actual `target/debug/web` binary inside
`TermSurf.app`, using a fake browser helper only in place of Roamium. The helper
will connect as a browser server, receive the normal `CreateTab`, send
`TabReady(tab_id=42)`, and keep the browser socket open. The harness will then
drive the normal `webtui` command flow by sending the user-level `:devtools`
command into the terminal. The expected chain is:

```text
webtui normal pane
  -> QueryDevtoolsRequest(tab_id=42)
  -> OpenSplit(command="<same web binary> --browser <helper> --profile default devtools://42")
  -> Ghostboard native split
  -> webtui DevTools pane
  -> QueryDevtoolsRequest(tab_id=42)
  -> SetDevtoolsOverlay(inspected_tab_id=42)
  -> CreateDevtoolsTab(inspected_tab_id=42)
  -> TabReady(devtools-pane, tab_id=99)
  -> BrowserReady(devtools-pane, tab_id=99)
```

If this flow fails, this experiment may make the smallest necessary Ghostboard
fix to make the existing `webtui` and helper-compatible browser behavior work.
It will not change `webtui`, `roamium`, or `proto/termsurf.proto`.

## Changes

Expected code changes are none unless the runtime validation discovers a
Ghostboard-side defect.

If a fix is needed, keep it limited to the smallest relevant Ghostboard files,
likely one of:

- `ghostboard/src/apprt/termsurf.zig`
  - protocol state, routing, query, `SetDevtoolsOverlay`, or logging fixes;
- `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`
  - native split command propagation fixes;
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
  - pane environment propagation fixes.

No changes will be made to `webtui`, `roamium`, `proto/termsurf.proto`,
branding, icon assets, Xcode project files, CLI install behavior, native browser
overlay presentation, keyboard/mouse browser input forwarding, browser process
lifecycle, or DevTools duplicate detection in this experiment.

## Verification

Pass criteria:

- Build the real `webtui` binary with `cargo build -p webtui`, with the command,
  cwd, and exit status recorded in a log.
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
- Runtime harness launches `TermSurf.app` with a temporary config whose command
  runs the actual `target/debug/web --browser <helper> https://example.com`
  inside the first terminal surface.
- The fake browser helper:
  - receives Ghostboard's browser launch arguments including `--ipc-socket` and
    `--listen-socket`;
  - listens on the requested `--listen-socket`;
  - connects back with `ServerRegister(profile=default)`;
  - receives `CreateTab` for the normal webtui pane;
  - sends `TabReady(normal-pane, tab_id=42)`;
  - accepts the normal webtui process's direct browser connection on
    `--listen-socket` after `BrowserReady(tab_id=42)`;
  - later receives `CreateDevtoolsTab(inspected_tab_id=42)` for the DevTools
    pane;
  - sends `TabReady(devtools-pane, tab_id=99)`;
  - accepts the DevTools webtui process's direct browser connection on
    `--listen-socket` after `BrowserReady(tab_id=99)`.
- The normal `webtui` process receives `BrowserReady(tab_id=42)` and becomes
  ready enough for the `:devtools` command to proceed. This can be proven by the
  following downstream events rather than by screen scraping.
- The harness sends the literal user command `:devtools` or `:devtools right`
  into the normal webtui terminal using the same System Events keyboard
  automation proven in Experiment 26. Keyboard automation is allowed only for
  simulating the user's command entry; the split itself must be caused by
  `OpenSplit`.
- App logs show `QueryDevtoolsRequest` for `tab_id=42` from the normal pane.
- App logs show `OpenSplit` for the normal pane and a successful native split
  bridge log.
- The DevTools split launches the actual `target/debug/web` binary with
  `devtools://42`, `--browser <helper>`, and `--profile default`.
- App logs show the DevTools pane sends
  `SetDevtoolsOverlay(inspected_tab_id=42)`.
- The helper receives `CreateDevtoolsTab(inspected_tab_id=42)` for the DevTools
  pane.
- The DevTools webtui process receives `BrowserReady(tab_id=99)`, proven by the
  app log line for `BrowserReady: pane_id=<devtools-pane> tab_id=99`.
- `QueryLastRequest(profile=default)` still returns the normal browser pane with
  `tab_id=42` after the DevTools pane becomes ready.
- Runtime shutdown removes the GUI socket file and leaves no stale matching
  `TermSurf.app/Contents/MacOS/termsurf`, `target/debug/web`, or fake helper
  processes.
- `git diff --check` is clean.

Fail criteria:

- The real `webtui` binary is not used for the normal pane.
- The DevTools split does not launch the real `webtui` binary.
- The harness sends `OpenSplit` directly instead of issuing the user-level
  `:devtools` command to the normal `webtui`.
- The split is created by System Events keyboard shortcuts instead of Ghostboard
  handling `OpenSplit`.
- `webtui` sends `QueryDevtoolsRequest` but receives an error for an existing
  attached tab.
- `OpenSplit` is not emitted by `webtui` or not handled by Ghostboard.
- The DevTools split launches but does not send `SetDevtoolsOverlay`.
- The browser helper does not receive `CreateDevtoolsTab`.
- DevTools `TabReady` makes `QueryLastRequest(profile=default)` return the
  DevTools pane instead of the normal pane.
- The implementation changes `webtui`, `roamium`, `proto/termsurf.proto`, Xcode
  project files, native browser overlay presentation, browser input forwarding,
  browser process lifecycle, or DevTools duplicate detection in this experiment.

## Design Review

A fresh-context adversarial Codex subagent reviewed the Experiment 28 design and
returned **APPROVED** with one optional finding: the runtime verification should
cover Ghostboard's `--listen-socket` browser launch argument and the direct
browser connections that real `webtui` opens after `BrowserReady`.

The design was updated to require the helper to verify both `--ipc-socket` and
`--listen-socket`, listen on the browser socket, and accept the normal and
DevTools webtui direct browser connections.

## Result

**Result:** Pass

The existing Ghostboard implementation passed the real `webtui` DevTools flow
without source changes.

Verification passed:

- Real `webtui` debug build passed:
  `logs/ghostboard-exp28-cargo-build-webtui-20260616.log`.
- No Rust source files were modified, so `cargo fmt` was not required.
- No Zig source files were modified, so `zig fmt` and the native GhosttyKit
  framework rebuild were not required.
- No Swift source files were modified, so `swiftlint` was not required.
- macOS app build passed:
  `logs/ghostboard-exp28-macos-build-debug-20260616.log`.
- Runtime harness passed: `logs/ghostboard-exp28-runtime-harness-20260616.log`.
- Runtime app log: `logs/ghostboard-exp28-runtime-app-20260616.log`.
- Fake browser helper log: `logs/ghostboard-exp28-helper-20260616.log`.
- `web last` / `QueryLastRequest` log:
  `logs/ghostboard-exp28-querylast-20260616.log`.
- `git diff --check` passed.

Observed successful runtime checks:

```text
PASS: helper launched with ipc and listen sockets
PASS: helper bound --listen-socket
PASS: normal CreateTab and TabReady tab_id 42
PASS: normal direct browser connection accepted
PASS: sent literal :devtools command with System Events
PASS: DevTools CreateDevtoolsTab and TabReady tab_id 99
PASS: DevTools direct browser connection accepted
PASS: QueryLast returned normal pane tab_id 42
PASS: app log contains QueryDevtoolsRequest tab 42
PASS: app log contains OpenSplit request
PASS: app log contains Swift split success
PASS: app log contains SetDevtoolsOverlay
PASS: app log contains CreateDevtoolsTab
PASS: app log contains BrowserReady tab 99
PASS: app exited and GUI socket cleaned up
PASS: no stale app, webtui, or helper processes
runtime verification passed
```

The runtime harness launched `TermSurf.app` with a temporary
`GHOSTTY_CONFIG_PATH` whose `command` started the actual
`/Users/astrohacker/dev/termsurf/target/debug/web` binary:

```text
/Users/astrohacker/dev/termsurf/target/debug/web --browser /tmp/termsurf-exp28-runtime/devtools-helper.py https://example.com
```

The fake browser helper was used only in place of Roamium. Ghostboard launched
that helper with both required browser-server arguments:

```text
--ipc-socket=/var/folders/.../termsurf-ghostboard-82084.sock
--listen-socket=/var/folders/.../devtools-helper.py-82084-default.sock
```

The helper connected back to the GUI socket, sent
`ServerRegister(profile=default)`, received a normal `CreateTab` for pane
`476286F8-BB58-4B6A-9565-D092CF301BA9`, sent `TabReady(tab_id=42)`, and accepted
the normal `webtui` direct browser connection on the listen socket.

The harness then used System Events only to type the literal user command
`:devtools` into the normal `webtui` pane. It did not send `OpenSplit` directly,
and it did not use a native split keyboard shortcut. The app log shows that the
real `webtui` process sent:

```text
TermSurf QueryDevtoolsRequest pane_id=476286F8-BB58-4B6A-9565-D092CF301BA9 inspected_tab_id=42 profile=default browser=/tmp/termsurf-exp28-runtime/devtools-helper.py
OpenSplit: pane_id=476286F8-BB58-4B6A-9565-D092CF301BA9 direction=right command=/Users/astrohacker/dev/termsurf/target/debug/web --browser /tmp/termsurf-exp28-runtime/devtools-helper.py --profile default devtools://42
```

Ghostboard created the native split, and the split launched the real `webtui`
binary with `devtools://42`, the same browser helper, and `--profile default`.
The DevTools webtui pane then sent `SetDevtoolsOverlay(inspected_tab_id=42)`,
causing Ghostboard to send `CreateDevtoolsTab(inspected_tab_id=42)` to the
helper. The helper replied with `TabReady(tab_id=99)`, and Ghostboard sent
`BrowserReady(tab_id=99)` to the DevTools pane.

After the DevTools pane was ready, the harness ran the actual `web last`
subcommand against the normal pane's `TERMSURF_SOCKET` and `TERMSURF_PANE_ID`.
`QueryLastRequest(profile=default)` still returned the normal pane and tab:

```text
profile: default
pane_id: 476286F8-BB58-4B6A-9565-D092CF301BA9
tab_id:  42
```

One verification detail matters for future runtime harnesses: the first runtime
attempt proved the behavior but did not capture Zig `std.log` info-level
protocol markers because `GHOSTTY_LOG` was not set. The final passing harness
sets `GHOSTTY_LOG=stderr`, which makes app-side `termsurf` protocol logs visible
in the captured app log.

## Conclusion

Ghostboard now supports the real `webtui` DevTools workflow through the current
TermSurf protocol without changes to `webtui`, `roamium`, or
`proto/termsurf.proto`. The tested chain is:

```text
webtui normal pane
  -> QueryDevtoolsRequest(tab_id=42)
  -> OpenSplit(command=".../target/debug/web --browser <helper> --profile default devtools://42")
  -> Ghostboard native split
  -> webtui DevTools pane
  -> QueryDevtoolsRequest(tab_id=42)
  -> SetDevtoolsOverlay(inspected_tab_id=42)
  -> CreateDevtoolsTab(inspected_tab_id=42)
  -> TabReady(devtools-pane, tab_id=99)
  -> BrowserReady(devtools-pane, tab_id=99)
```

The next experiment can move from fake-browser DevTools orchestration to
Roamium-backed runtime behavior, because the GUI-side split, DevTools overlay,
browser socket, and `webtui` command path have now been validated together.

## Completion Review

A fresh-context adversarial Codex subagent reviewed the completed Experiment 28
result and returned **APPROVED** with no findings.

The reviewer confirmed that the working tree contained only the expected issue
markdown result updates, the result commit had not already been made,
`git diff --check` was clean, and the logs prove the real
`/Users/astrohacker/dev/termsurf/target/debug/web` binary drove both the normal
and DevTools panes. The reviewer also confirmed that the helper verified
`--ipc-socket`, `--listen-socket`, both direct browser connections, normal tab
42, DevTools tab 99, `QueryLastRequest(profile=default)` returning the normal
pane after DevTools became ready, runtime cleanup, the experiment Result and
Conclusion sections, and the README status update to **Pass**.
