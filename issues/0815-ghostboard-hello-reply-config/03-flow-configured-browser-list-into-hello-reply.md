# Experiment 3: Flow Configured Browser List Into HelloReply

## Description

Experiments 1 and 2 made `HelloReply` deterministic and then configurable for
`homepage`. Issue 815 still names browser-list configuration as part of the
target behavior. webtui consumes the first `HelloReply.browsers` entry as the
default browser when `--browser` is omitted, so Ghostboard should advertise a
configurable browser list, with `roamium` as the deterministic fallback.

This experiment will add a small browser-list config path and prove that
Ghostboard sends the configured list in `HelloReply` while webtui still launches
the first configured browser in the no-`--browser` path.

## Changes

Planned config changes:

- `ghostboard/src/config/Config.zig`
  - Add a repeatable `browser` string field using the existing
    `RepeatableString` config type.
  - Document that each `browser = ...` entry adds one browser name advertised to
    webtui in `HelloReply.browsers`, and that the first configured browser acts
    as the default when webtui is launched without `--browser`.
  - Preserve the existing `RepeatableString` reset convention: `browser = ""`
    clears the configured list, causing Ghostboard to fall back to `roamium`.

- `ghostboard/macos/Sources/Ghostty/Ghostty.Config.swift`
  - Add a `browserList` computed property that reads the repeatable `browser`
    entries through a narrow C API for `RepeatableString` values.
  - Fall back to `["roamium"]` if the field is unavailable or empty.

- `ghostboard/src/config/CApi.zig`, `ghostboard/src/config/c_get.zig`, and
  `ghostboard/include/ghostty.h`
  - Add a small read-only C accessor for repeatable string config values, such
    as count-by-key and item-by-key/index. Keep it generic to `RepeatableString`
    fields instead of adding a browser-only parser.
  - Add a C API regression test proving the new accessor can read multiple
    `browser` entries and reports zero entries after `browser = ""`.

Planned TermSurf bridge changes:

- `ghostboard/include/ghostty.h` and
  `ghostboard/macos/Sources/App/macOS/ghostty-bridging-header.h`
  - Extend the existing hello-config bridge to pass both homepage and browser
    list. Prefer one combined bridge if it keeps AppDelegate update ordering
    simple.
  - The user-facing config remains repeatable `browser = ...` entries. If Swift
    serializes the parsed list for the C bridge, use a private lossless
    delimiter such as newline only for the bridge payload.

- `ghostboard/src/main_c.zig`
  - Forward the browser-list bridge payload into `apprt/termsurf.zig`.

- `ghostboard/src/apprt/termsurf.zig`
  - Store the current browser list in Zig-owned fixed storage guarded by
    `state_mutex`.
  - Parse the bridge payload into a bounded list of non-empty browser names.
  - Fall back to `roamium` if the configured list is empty, contains no
    non-empty entries, exceeds the supported browser count, or has an overlong
    browser name.
  - During `sendHelloReply`, copy the locked browser-list snapshot into stack
    storage, build a stack pointer array for protobuf-c, and pass only that
    stack-owned snapshot through the synchronous `sendProtobuf` call.
  - Log the advertised browser list as a comma-separated string so the harness
    can assert it.

- `ghostboard/macos/Sources/App/macOS/AppDelegate.swift`
  - Pass both the active homepage and browser list when the app starts and when
    `ghosttyConfigDidChange(config:)` runs.

Planned harness changes:

- `scripts/ghostboard-geometry-matrix.sh`
  - Add a focused custom-browser-list scenario that writes `browser = roamium`
    and `browser = debug-roamium` into the temporary config.
  - Run `web` with no `--browser` but with a normal URL, so webtui must choose
    the first configured browser from `HelloReply.browsers`.
  - Assert the generated command lacks `--browser`.
  - Verify Ghostboard logs `HelloReply` with `browsers=roamium,debug-roamium`.
  - Verify `SetOverlay` still uses `browser=roamium`, proving webtui consumed
    the first configured browser and the named-Roamium resolver still succeeds.
  - Add a focused empty-browser-list scenario that writes `browser = ""`,
    launches the same no-`--browser` command, and verifies Ghostboard falls back
    to `browsers=roamium` and webtui still sends `SetOverlay browser=roamium`.

Planned issue-doc changes:

- Record the browser-list config behavior, fallback behavior, verification
  commands, runtime logs, and reviewer verdict.

## Verification

Formatting actions:

1. `prettier --write --prose-wrap always --print-width 80 issues/0815-ghostboard-hello-reply-config/README.md issues/0815-ghostboard-hello-reply-config/03-flow-configured-browser-list-into-hello-reply.md`.
2. `zig fmt ghostboard/src/config/Config.zig ghostboard/src/config/CApi.zig ghostboard/src/config/c_get.zig ghostboard/src/apprt/termsurf.zig ghostboard/src/main_c.zig`.

Static/build checks:

1. `prettier --check --prose-wrap always --print-width 80 issues/0815-ghostboard-hello-reply-config/README.md issues/0815-ghostboard-hello-reply-config/03-flow-configured-browser-list-into-hello-reply.md`.
2. `zig fmt --check ghostboard/src/config/Config.zig ghostboard/src/config/CApi.zig ghostboard/src/config/c_get.zig ghostboard/src/apprt/termsurf.zig ghostboard/src/main_c.zig`.
3. `bash -n scripts/ghostboard-geometry-matrix.sh`.
4. `shellcheck scripts/ghostboard-geometry-matrix.sh` if available.
5. `cd ghostboard && zig build -Demit-macos-app=false`.
6. `cd ghostboard && macos/build.nu --scheme Ghostty --configuration Debug --action build`.
7. `git diff --check`.

Runtime checks:

1. Run `scripts/ghostboard-geometry-matrix.sh named-roamium-debug-launch` to
   confirm the default browser-list path still advertises and launches
   `roamium`.
2. Run the new custom-browser-list scenario.
3. Verify the app log shows the loaded configured browser list.
4. Verify `HelloReply` includes `browsers=roamium,debug-roamium`.
5. Verify webtui consumes the first configured browser by sending `SetOverlay`
   with `browser=roamium`.
6. Verify `BrowserReady` still preserves `browser=roamium`.
7. Run the new empty-browser-list scenario.
8. Verify `browser = ""` resets to the fallback list by sending
   `HelloReply browsers=roamium` and `SetOverlay browser=roamium`.

Pass criteria:

- Custom repeatable `browser` config values flow into `HelloReply.browsers`.
- `web` without `--browser` uses the first configured browser through the hello
  reply.
- Missing or empty browser config falls back to `roamium`.
- Existing configured-homepage behavior remains intact.
- The app builds and the default/custom runtime scenarios pass.

Partial criteria:

- The browser list is configurable and sent in `HelloReply`, but only the
  default/fallback path is runtime-proven.

Fail criteria:

- `HelloReply.browsers` remains hardcoded despite a custom config file.
- The configured list reaches the log but webtui does not use the first browser
  as the default.
- The bridge introduces stale pointer lifetime, data race, or build failures.

## Design Review

Fresh-context adversarial review by Codex subagent `Dalton`:

- **Initial verdict:** Changes required.
- **Required finding:** The planned comma-separated `browser-list` string
  invented parallel list semantics even though Ghostty already has
  `RepeatableString` for repeatable string lists.
- **Required finding:** The pass criteria required empty-list fallback behavior,
  but the verification plan did not prove the empty or invalid fallback path.
- **Resolution:** Accepted both findings. The design now uses repeatable
  `browser = ...` entries backed by `RepeatableString`, adds a narrow read-only
  C accessor for repeatable string config values, treats `browser = ""` as the
  existing repeatable-string reset convention, and adds an empty-browser-list
  harness scenario that proves fallback to `roamium`.
- **First re-review verdict:** Changes required.
- **First re-review resolution:** The reviewer confirmed both substantive
  findings were resolved, then found that the formatting commands omitted the
  newly planned Zig files `ghostboard/src/config/CApi.zig` and
  `ghostboard/src/config/c_get.zig`. Accepted and added both files to the
  mutating `zig fmt` and check-only `zig fmt --check` commands.
- **Second re-review verdict:** Approved. The reviewer confirmed the formatting
  action and check-only command now include `ghostboard/src/config/CApi.zig` and
  `ghostboard/src/config/c_get.zig`, with no new required findings.

## Result

**Result:** Pass

Implemented configurable browser-list flow end to end:

- Added repeatable `browser = ...` config entries using Ghostty's existing
  `RepeatableString` semantics.
- Added generic C accessors for repeatable string config values and covered them
  with a C API regression test.
- Added `Ghostty.Config.browserList`, falling back to `["roamium"]` when the
  repeatable config is absent or reset to empty.
- Extended the hello-config bridge to pass homepage and the parsed browser list
  together.
- Stored the current browser list in Zig-owned fixed storage guarded by
  `state_mutex`.
- Updated `sendHelloReply` to snapshot homepage and browsers into stack storage
  before the synchronous protobuf send.
- Added `hello-config-browser-list` and `hello-empty-browser-list` runtime
  harness scenarios.

Verification run:

- `prettier --check --prose-wrap always --print-width 80 issues/0815-ghostboard-hello-reply-config/README.md issues/0815-ghostboard-hello-reply-config/03-flow-configured-browser-list-into-hello-reply.md`
  — pass.
- `zig fmt --check ghostboard/src/config/Config.zig ghostboard/src/config/CApi.zig ghostboard/src/config/c_get.zig ghostboard/src/apprt/termsurf.zig ghostboard/src/main_c.zig`
  — pass.
- `bash -n scripts/ghostboard-geometry-matrix.sh` — pass.
- `cd ghostboard && zig build -Demit-macos-app=false` — pass.
- `cd ghostboard && macos/build.nu --scheme Ghostty --configuration Debug --action build`
  — pass.
- `git diff --check` — pass.
- `scripts/ghostboard-geometry-matrix.sh named-roamium-debug-launch` — pass.
  - Harness log:
    `logs/ghostboard-geometry-named-roamium-debug-launch-harness-20260617-214938.log`
  - App log:
    `logs/ghostboard-geometry-named-roamium-debug-launch-app-20260617-214938.log`
  - Roamium trace:
    `logs/ghostboard-geometry-named-roamium-debug-launch-roamium-20260617-214938.log`
- `scripts/ghostboard-geometry-matrix.sh hello-config-browser-list` — pass.
  - Harness log:
    `logs/ghostboard-geometry-hello-config-browser-list-harness-20260617-214949.log`
  - App log:
    `logs/ghostboard-geometry-hello-config-browser-list-app-20260617-214949.log`
  - Roamium trace:
    `logs/ghostboard-geometry-hello-config-browser-list-roamium-20260617-214949.log`
- `scripts/ghostboard-geometry-matrix.sh hello-empty-browser-list` — pass.
  - Harness log:
    `logs/ghostboard-geometry-hello-empty-browser-list-harness-20260617-215051.log`
  - App log:
    `logs/ghostboard-geometry-hello-empty-browser-list-app-20260617-215051.log`
  - Roamium trace:
    `logs/ghostboard-geometry-hello-empty-browser-list-roamium-20260617-215051.log`
- `scripts/ghostboard-geometry-matrix.sh hello-config-homepage` — pass.
  - Harness log:
    `logs/ghostboard-geometry-hello-config-homepage-harness-20260617-215102.log`
  - App log:
    `logs/ghostboard-geometry-hello-config-homepage-app-20260617-215102.log`
  - Roamium trace:
    `logs/ghostboard-geometry-hello-config-homepage-roamium-20260617-215102.log`

Skipped optional check:

- `shellcheck scripts/ghostboard-geometry-matrix.sh` was not run because
  `shellcheck` is not installed on this VM.

## Conclusion

Ghostboard now advertises browser names from repeatable `browser = ...` config
entries in `HelloReply.browsers`. The runtime harness proves that webtui uses
the first configured browser when `--browser` is omitted, that `browser = ""`
falls back to `roamium`, and that the configured-homepage path from Experiment 2
still works after the bridge change.

## Completion Review

Fresh-context adversarial result review by Codex subagent `McClintock`:

- **Initial verdict:** Changes required.
- **Required finding:** `ghostboard/include/ghostty.h` still declared
  `termsurf_hello_config_changed(const char*)`, while `main_c.zig` and the Swift
  bridging header used the new two-argument ABI.
- **Resolution:** Accepted. Updated `ghostboard/include/ghostty.h` to declare
  `termsurf_hello_config_changed(const char*, const char*)`.
- **Post-fix verification:** `prettier --check`, `zig fmt --check`,
  `bash -n scripts/ghostboard-geometry-matrix.sh`, `git diff --check`,
  `cd ghostboard && zig build -Demit-macos-app=false`, and
  `cd ghostboard && macos/build.nu --scheme Ghostty --configuration Debug --action build`
  all passed. A parallel rerun of the Zig and macOS app builds briefly failed
  because Xcode saw `GhosttyKit.xcframework` while the Zig build was
  regenerating it; rerunning the macOS build by itself passed.
- **Re-review verdict:** Approved. The reviewer confirmed the public header,
  Swift bridging header, and Zig export now all use the same two-argument ABI,
  with no new required findings.
