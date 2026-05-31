# Experiment 13: Add Session Isolation and Incognito Coverage

## Description

Experiment 1 left session isolation and incognito behavior as the last explicit
in-scope Issue 799 queue item. Named profiles already exist at the webtui and
Wezboard level, and Wezboard already launches one Roamium process per
`profile + browser` key with a profile-specific `--user-data-dir`. What is not
yet proved is whether Chromium actually persists and isolates state correctly
for those paths, and there is no exposed incognito/private mode path.

This experiment adds deterministic automated coverage for:

1. regular profile persistence across Roamium restarts;
2. regular profile isolation between two different profile directories;
3. incognito/private state isolation from regular state;
4. incognito/private non-persistence across Roamium restarts.

The implementation should use the smallest explicit incognito surface that fits
TermSurf's current architecture. Prefer a Roamium `--incognito` flag that
selects Chromium's already-created off-the-record `TsBrowserContext`. Then
expose it through webtui with a `--incognito` flag that maps to a reserved
profile name for server routing, so the protocol does not need a new field for
this experiment.

The reserved profile name should be `incognito`. This is conventional, already
valid under webtui's lowercase-alphanumeric profile validation, and lets
Wezboard keep its existing one-server-per-profile routing model. The trade-off
is that persistent user profiles named `incognito` become reserved. Document
that behavior in the CLI help and issue result.

Regular profile persistence should be treated as an implementation fact to
prove, not an assumption. Today Roamium passes `--user-data-dir` into Chromium's
process command line, while `ts_create_browser_context(ptr::null())` returns the
default context created by `ShellBrowserMainParts`. That may already be correct
because Content Shell derives its default context storage from the process
`--user-data-dir`. This experiment must explicitly test that behavior. Only if
that test fails should the implementation change the profile-path plumbing by
passing Roamium's parsed `--user-data-dir` into
`ts_create_browser_context(path)` and making
`TsBrowserMainParts::CreateBrowserContext(path)` honor it.

## Changes

1. **Create the Chromium experiment branch.**

   Create a fresh Chromium branch for this experiment before editing Chromium:

   ```bash
   git -C chromium/src checkout -b 148.0.7778.97-issue-799-exp13
   ```

   Branch from the most recent known-good Issue 799 Chromium branch, currently
   `148.0.7778.97-issue-799-exp12`, unless a newer committed Issue 799 branch
   exists when implementation begins. Add the new branch to
   `chromium/README.md`.

2. **Audit current profile behavior before editing.**
   - Confirm whether regular named profiles already persist by launching Roamium
     directly with a temporary `--user-data-dir`, loading a local fixture that
     writes `localStorage` and a cookie, restarting Roamium with the same path,
     and reading those values back.
   - Confirm that a second temporary `--user-data-dir` does not see profile A's
     state.
   - Confirm how persistence is achieved. If it already works, record that the
     process-level `--user-data-dir` plus default Shell context is sufficient.
     If it fails, fix the explicit path plumbing rather than papering over the
     failure in the test.
   - Record the before-change result in this experiment's Result section after
     implementation. Do not assume persistence works just because Wezboard
     passes a profile-specific path.

3. **Add a Chromium C API for the off-the-record browser context.**

   Files:
   - `chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.h`
   - `chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.cc`
   - `chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.h`
   - `chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.cc`

   Add a function such as:

   ```c
   TS_EXPORT ts_browser_context_t ts_create_incognito_browser_context(void);
   ```

   The implementation should return the existing
   `off_the_record_browser_context()` created by `TsBrowserMainParts`. Do not
   create ad hoc temporary directories for incognito. Chromium already knows
   this context is off the record, and existing code such as
   `ConfigureNetworkContextParamsForShell()` already avoids persistent network
   paths for off-the-record contexts.

4. **Select the incognito context in Roamium.**

   Files:
   - `roamium/src/ffi.rs`
   - `roamium/src/main.rs`

   Add FFI for `ts_create_incognito_browser_context()` and parse a `--incognito`
   command-line flag. When the flag is present, initialize `BROWSER_CONTEXT`
   from the incognito API instead of the regular context.

   Preserve `--user-data-dir` parsing for logging and server registration. The
   profile name sent in `ServerRegister` must still match what Wezboard expects,
   so an incognito Roamium launched by Wezboard should register as `incognito`.

   If the regular profile pre-check proves that explicit path plumbing is
   required, also store Roamium's parsed `--user-data-dir` and pass it to
   `ts_create_browser_context(path)` for non-incognito launches. Do not pass a
   persistent path to the incognito context.

5. **Expose incognito from webtui and Wezboard without protocol churn.**

   Files:
   - `webtui/src/main.rs`
   - `wezboard/wezboard-gui/src/termsurf/conn.rs`

   Add a global webtui `--incognito` flag. When present:
   - reject simultaneous `--profile <name>` unless `<name>` is exactly
     `incognito`;
   - set the outbound profile to `incognito`;
   - show clear CLI help that `--incognito` opens an ephemeral private profile.

   In Wezboard's `spawn_server()`, if the profile is `incognito`, append
   `--incognito` to the Roamium command line. Keep the server key as
   `incognito + browser`, so multiple incognito panes share one ephemeral
   session while the incognito Roamium process is alive. This matches ordinary
   browser incognito-window behavior and avoids protocol changes.

   Do not change `termsurf.proto` for this experiment.

   Add an automated launch-path assertion. Use the least invasive option that
   can prove the argv:
   - a browser-wrapper script supplied to Wezboard/webtui that records its argv
     and then exits; or
   - a deterministic Wezboard spawn log/process-argv check.

   The assertion must prove that `web --incognito` results in a Roamium launch
   containing both `--incognito` and a profile/server route of `incognito`.

6. **Add automated session probes.**

   Prefer extending `scripts/test-issue-799-browser-api-audit.py` if the
   existing direct-Roamium harness can drive the needed navigation and
   evaluation. If the session flow is cleaner as a separate script, add:
   - `scripts/test-issue-799-session-isolation.py`

   The script must use only local fixtures and temporary directories under
   `logs/issue-799-browser-api-audit/`. It should launch the repo-built Roamium
   directly with explicit `--user-data-dir` paths and `--listen-socket`/protocol
   control, not the installed app.

   Required probe cases:
   - `profile-persistence`: write `localStorage` and a cookie in profile A,
     restart with the same `--user-data-dir`, and assert both values are still
     visible.
   - `profile-isolation`: start profile B with a different `--user-data-dir` and
     assert profile A's values are absent.
   - `reserved-incognito-profile-isolation`: create persistent state in a
     regular profile directory whose basename is `incognito`, then launch
     Roamium with both that `--user-data-dir` and `--incognito`. Assert the
     persistent `incognito` profile state is not visible and is not modified by
     the private session.
   - `incognito-isolation`: start incognito, assert regular profile A's values
     are absent, write incognito-only values, and assert they are visible during
     that live incognito process.
   - `incognito-non-persistence`: restart incognito and assert the
     incognito-only values are absent.

   Use deterministic local HTTP origins. Cookies are origin-scoped, so the same
   local server origin should be reused across profile A, profile B, and
   incognito checks. Cookie assertions must include server-observed `Cookie`
   headers after the relevant restart/isolation navigation, not only
   `document.cookie`, so the test proves Chromium's network cookie jar behavior.

   The live incognito continuity check must include at least one navigation,
   reload, or second same-origin tab after the write. An immediate read after
   write is not enough evidence that the live private session is functioning.

7. **Regenerate the Chromium patch archive.**

   After the Chromium branch builds and passes verification, commit the Chromium
   changes on `148.0.7778.97-issue-799-exp13`, regenerate the Issue 799 patch
   archive under `chromium/patches/issue-799/`, and include the new patch in the
   main repo commit.

8. **Update Issue 799 tracking.**

   Files:
   - `issues/0799-browser-api-automation-triage/13-session-isolation-incognito.md`
   - `issues/0799-browser-api-automation-triage/README.md`

   After implementation, append Result and Conclusion to this experiment file
   and update the README experiment status. If this completes the final in-scope
   queue item, close Issue 799 with a README Conclusion that lists completed
   automated surfaces and explicitly keeps deferred/manual surfaces out of
   scope.

## Verification

Run all verification from the repo without installing over the stable app.

1. Format changed Rust code:

   ```bash
   PATH="/Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin:$PATH" cargo fmt
   ```

2. Format changed Chromium C++ files:

   ```bash
   chromium/src/buildtools/mac_arm64-format/clang-format -i \
     chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.h \
     chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.cc \
     chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.h \
     chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.cc
   ```

3. Build Chromium:

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

4. Build Roamium, Wezboard, and webtui debug binaries:

   ```bash
   ./scripts/build.sh roamium
   ./scripts/build.sh wezboard
   ./scripts/build.sh webtui
   ```

5. Run the focused session-isolation automation:

   ```bash
   python3 scripts/test-issue-799-session-isolation.py
   ```

   Or, if implemented as probes in the existing harness:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py \
     --probe profile-persistence \
     --probe profile-isolation \
     --probe reserved-incognito-profile-isolation \
     --probe incognito-isolation \
     --probe incognito-non-persistence
   ```

6. Run the full Issue 799 browser API audit harness:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py --seconds 8
   ```

7. Run static checks:

   ```bash
   python3 -m py_compile scripts/test-issue-799-browser-api-audit.py
   test ! -f scripts/test-issue-799-session-isolation.py || \
     python3 -m py_compile scripts/test-issue-799-session-isolation.py
   git diff --check
   git -C chromium/src diff --check
   ```

8. Verify the Chromium branch and patch archive state:

   ```bash
   git -C chromium/src branch --show-current
   ls chromium/patches/issue-799/
   ```

   The Chromium branch must be `148.0.7778.97-issue-799-exp13`, and the patch
   archive must include the committed Experiment 13 Chromium patch.

9. Verify command-line behavior without launching the GUI:

   ```bash
   ./webtui/target/debug/web --help | rg -- '--incognito'
   ./webtui/target/debug/web --incognito --profile default https://example.test
   ```

   The second command should fail fast with a clear error before contacting
   Wezboard. `--incognito --profile incognito` should be accepted.

## Pass Criteria

- Regular profile A persists localStorage and cookies across a Roamium restart.
- Regular profile B does not see profile A's localStorage or cookies.
- Incognito does not see regular profile A's localStorage or cookies.
- Incognito state is visible within the same live incognito session.
- Incognito state is gone after restarting the incognito Roamium process.
- `web --incognito` routes through the reserved `incognito` profile and causes
  Wezboard to launch Roamium with `--incognito`, proven by recorded argv or an
  equivalent deterministic process/spawn assertion.
- A preexisting persistent profile directory named `incognito` is not visible to
  or mutated by an incognito Roamium launch.
- Existing Issue 799 probes still pass with no new missing Mojo interface,
  renderer crash, or regression in downloads, dialogs, permissions, WebAuthn,
  file upload, HTTP auth, crash recovery, console capture, or zoom.

## Failure Criteria

- The implementation only uses a temporary persistent `--user-data-dir` for
  incognito instead of Chromium's off-the-record browser context.
- The implementation adds protocol fields before proving the reserved-profile
  approach is insufficient.
- Incognito state persists across a Roamium process restart.
- Incognito can read regular profile cookies or localStorage.
- Incognito can read or mutate preexisting persistent state from a reserved
  `incognito` profile directory.
- Regular named profiles fail to persist or fail to isolate from each other.
- The test only checks `document.cookie` and never proves server-observed cookie
  headers.
- The webtui/Wezboard routing path is inferred from CLI parsing without proving
  Roamium is actually launched with `--incognito`.
- Existing Issue 799 automated probes regress.

## Codex Review

Before implementation, run Codex review on this design and fix all real
findings. After implementation and result recording, run Codex review again on
the completed experiment and relevant diffs before closing or moving to the next
experiment.
