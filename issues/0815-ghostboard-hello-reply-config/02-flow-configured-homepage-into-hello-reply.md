# Experiment 2: Flow Configured Homepage Into HelloReply

## Description

Experiment 1 fixed the empty `HelloReply` by sending deterministic defaults.
Historical Issues 674 and 675 require more: `web` without arguments should open
a configurable homepage supplied by the GUI's live config, not just a hardcoded
fallback. Ghostboard already loads TermSurf config files through the normal
Ghostty config system, and Swift already reads config fields through
`ghostty_config_get`.

This experiment will add a `homepage` config field and bridge the active macOS
app config into Ghostboard's TermSurf IPC state so `HelloReply.homepage`
reflects the configured value. The browser list will remain the deterministic
`roamium` default from Experiment 1.

## Changes

Planned config changes:

- `ghostboard/src/config/Config.zig`
  - Add a `homepage` string field with default `https://termsurf.com/welcome`.
  - Document that this is the default page opened by `web` when no URL is
    supplied.

- `ghostboard/macos/Sources/Ghostty/Ghostty.Config.swift`
  - Add a `homepage` computed property using `ghostty_config_get`.
  - Fall back to `https://termsurf.com/welcome` if the field is unavailable or
    empty.

Planned TermSurf bridge changes:

- `ghostboard/include/ghostty.h`
  - Declare a small exported function for updating the TermSurf hello config,
    such as `termsurf_hello_config_changed(const char *homepage)`.

- `ghostboard/src/main_c.zig`
  - Export the new C ABI function and forward the homepage string into
    `apprt/termsurf.zig`.

- `ghostboard/src/apprt/termsurf.zig`
  - Store the current HelloReply homepage in TermSurf IPC state.
  - Initialize it to the documented default.
  - Update it from the exported bridge function by copying the incoming C string
    into Zig-owned fixed storage with a defined maximum length. Never retain the
    Swift-owned `const char *`.
  - Guard both bridge updates and `sendHelloReply` reads with `state_mutex`,
    matching the existing shared IPC state pattern.
  - Reject empty or overlong input deterministically by falling back to the
    default and logging the reason.
  - Use the stored homepage in `sendHelloReply`.
  - During `sendHelloReply`, copy the locked homepage snapshot into stack
    storage and pass only that pointer through the synchronous `sendProtobuf`
    call.
  - Log both default and configured homepage values clearly enough for the
    harness to assert.

- `ghostboard/macos/Sources/App/macOS/AppDelegate.swift`
  - Call the bridge when the app starts with its loaded config and whenever
    `ghosttyConfigDidChange(config:)` runs.

Planned harness changes:

- `scripts/ghostboard-geometry-matrix.sh`
  - Add a focused custom-homepage scenario that writes a temporary config file
    containing `homepage = https://example.net/issue-815-homepage`.
  - Launch the debug app with `GHOSTTY_CONFIG_PATH` pointing at that config.
  - Override command generation to run exactly `exec "$WEB"` with no positional
    URL and no `--browser`, proving webtui uses `HelloReply.homepage` rather
    than a CLI URL.
  - Assert the generated command lacks both `--browser` and the test URL.
  - Verify Ghostboard sends `HelloReply` with the custom homepage.
  - Verify the following `SetOverlay` URL is the custom homepage and
    `browser=roamium`.

Planned issue-doc changes:

- Record the configured homepage behavior, fallback behavior, verification
  commands, runtime logs, and reviewer verdict.

## Verification

Formatting actions:

1. `prettier --write --prose-wrap always --print-width 80 issues/0815-ghostboard-hello-reply-config/README.md issues/0815-ghostboard-hello-reply-config/02-flow-configured-homepage-into-hello-reply.md`.
2. `zig fmt ghostboard/src/config/Config.zig ghostboard/src/apprt/termsurf.zig ghostboard/src/main_c.zig`.

Static/build checks:

1. `prettier --check --prose-wrap always --print-width 80 issues/0815-ghostboard-hello-reply-config/README.md issues/0815-ghostboard-hello-reply-config/02-flow-configured-homepage-into-hello-reply.md`.
2. `zig fmt --check ghostboard/src/config/Config.zig ghostboard/src/apprt/termsurf.zig ghostboard/src/main_c.zig`.
3. `bash -n scripts/ghostboard-geometry-matrix.sh`.
4. `shellcheck scripts/ghostboard-geometry-matrix.sh` if available.
5. `cd ghostboard && zig build -Demit-macos-app=false`.
6. `cd ghostboard && macos/build.nu --scheme Ghostty --configuration Debug --action build`.
7. `git diff --check`.

Runtime checks:

1. Run `scripts/ghostboard-geometry-matrix.sh named-roamium-debug-launch` to
   confirm the default homepage/browser path still works.
2. Run the new custom-homepage scenario.
3. Verify the app log shows the loaded configured homepage.
4. Verify `HelloReply` includes the configured homepage and `roamium`.
5. Verify webtui consumes the configured homepage by sending `SetOverlay` with
   `url=https://example.net/issue-815-homepage`.
6. Verify `BrowserReady` still preserves `browser=roamium`.

Pass criteria:

- A custom `homepage` config value flows into `HelloReply.homepage`.
- `web` without arguments uses that configured homepage through the hello reply.
- Missing or empty homepage config falls back to `https://termsurf.com/welcome`.
- The browser list remains `["roamium"]`, and omitted-`--browser` launch still
  works.
- The app builds and the runtime scenarios pass.

Partial criteria:

- The homepage config flows into `HelloReply`, but broader browser-list
  configurability needs a separate experiment.

Fail criteria:

- `HelloReply.homepage` remains hardcoded despite a custom config file.
- The custom homepage reaches the log but webtui still opens the default URL.
- The bridge introduces stale pointer lifetime, data race, or build failures.

## Design Review

Fresh-context adversarial review by Codex subagent `Epicurus`:

- **Initial verdict:** Changes required.
- **Required finding:** The bridge plan did not specify the synchronization and
  owned-storage contract needed to avoid data races and dangling pointers
  between AppKit/config-change paths and `sendHelloReply` client threads.
- **Optional finding:** The custom-homepage harness scenario must generate a
  `web` command with no positional URL, not merely no `--browser`, so it proves
  webtui consumes `HelloReply.homepage`.
- **Nit:** Separate mutating `prettier --write` formatting from final
  non-mutating `prettier --check` verification.
- **Resolution:** Accepted all findings. The design now requires Zig-owned fixed
  storage, `state_mutex` synchronization for updates and reads, deterministic
  fallback for empty/overlong homepage values, a no-URL/no-browser command
  assertion in the harness, and separate formatting/check steps.
- **Re-review verdict:** Approved. The reviewer confirmed all prior findings
  were resolved and no new required findings were introduced.

## Result

**Result:** Pass

Implemented the configured homepage path end to end:

- Added `homepage` to `ghostboard/src/config/Config.zig`, with default
  `https://termsurf.com/welcome`.
- Added Swift access through `Ghostty.Config.homepage`.
- Added `termsurf_hello_config_changed` through the public C header, bridging
  header, `main_c.zig`, and `AppDelegate`.
- Stored the active homepage in Zig-owned fixed storage guarded by
  `state_mutex`, and copied a locked snapshot for each synchronous `HelloReply`.
- Added C config getter support and a regression test for non-optional sentinel
  strings (`[:0]const u8`), which `homepage` uses.
- Added the `hello-config-homepage` harness scenario. It runs `web` with no
  positional URL and no `--browser`, writes a temporary `homepage` config value,
  and verifies that `HelloReply`, `SetOverlay`, and `BrowserReady` all carry the
  configured homepage/default-browser behavior.

The first focused runtime attempt failed because the new harness scenario wrote
the custom homepage before the shared config fixture was composed, and the
shared fixture overwrote it. The final harness writes the scenario command early
but appends the `homepage` line after the shared `initial-command` config, so
the launched app reads one final config containing both values.

Verification run:

- `zig fmt ghostboard/src/config/c_get.zig ghostboard/src/config/CApi.zig ghostboard/src/config/Config.zig ghostboard/src/apprt/termsurf.zig ghostboard/src/main_c.zig`
  — pass.
- `bash -n scripts/ghostboard-geometry-matrix.sh` — pass.
- `cd ghostboard && ./zig-out/bin/termsurf +validate-config --config-file=<tmp>`
  with `homepage = "https://example.net/issue-815-homepage"` — pass.
- `cd ghostboard && ./zig-out/bin/termsurf +validate-config --config-file=<tmp>`
  with `homepage = https://example.net/issue-815-homepage` — pass.
- `cd ghostboard && zig build -Demit-macos-app=false` — pass.
- `cd ghostboard && macos/build.nu --scheme Ghostty --configuration Debug --action build`
  — pass.
- `git diff --check` — pass.
- `scripts/ghostboard-geometry-matrix.sh hello-config-homepage` — pass.
  - Harness log:
    `logs/ghostboard-geometry-hello-config-homepage-harness-20260617-213553.log`
  - App log:
    `logs/ghostboard-geometry-hello-config-homepage-app-20260617-213553.log`
  - Roamium trace:
    `logs/ghostboard-geometry-hello-config-homepage-roamium-20260617-213553.log`
- `scripts/ghostboard-geometry-matrix.sh named-roamium-debug-launch` — pass.
  - Harness log:
    `logs/ghostboard-geometry-named-roamium-debug-launch-harness-20260617-213611.log`
  - App log:
    `logs/ghostboard-geometry-named-roamium-debug-launch-app-20260617-213611.log`
  - Roamium trace:
    `logs/ghostboard-geometry-named-roamium-debug-launch-roamium-20260617-213611.log`

Known broad-test limitation:

- `cd ghostboard && zig build test -Demit-macos-app=false` failed in the macOS
  Swift test target before reaching the new Zig C API test:
  `@testable import Ghostty` could not resolve module dependency `Ghostty`. This
  is the same broad local test-target class already documented on earlier
  Ghostboard issues, so the experiment used the successful Zig build, macOS app
  build, config validation, and focused runtime harnesses as the authoritative
  verification.

## Conclusion

Ghostboard now exposes a configurable homepage through its normal config system
and sends that value in `HelloReply.homepage`. The focused runtime harness
proves that `web` without arguments consumes the GUI-provided homepage and
default browser list, and the named-Roamium regression keeps Experiment 1's
default behavior intact.

## Completion Review

Fresh-context adversarial result review by Codex subagent `Wegener`:

- **Verdict:** Approved.
- **Required findings:** None.
- **Evidence reviewed:** The reviewer confirmed the result commit had not yet
  been made, the README marked Experiment 2 as `Pass`, the experiment file had
  `Result` and `Conclusion`, Swift only passes a temporary C string through a
  synchronous bridge call, Zig copies that value into owned storage immediately,
  `sendHelloReply` snapshots the homepage under `state_mutex`, the harness
  command is exactly `exec "$WEB"`, and the logs show the configured
  `HelloReply` followed by `SetOverlay` with
  `url=https://example.net/issue-815-homepage`.
- **Independent checks run by reviewer:** `git diff --check`,
  `bash -n scripts/ghostboard-geometry-matrix.sh`, `zig fmt --check ...`, and
  `prettier --check ...` all passed.
- **Unavailable optional check:** `shellcheck` was unavailable.
