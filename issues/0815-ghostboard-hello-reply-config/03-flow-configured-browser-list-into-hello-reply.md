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
