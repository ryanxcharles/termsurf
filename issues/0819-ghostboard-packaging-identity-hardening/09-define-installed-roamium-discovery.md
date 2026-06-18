# Experiment 9: Define Installed Roamium Discovery

## Description

Issue 814 deliberately made debug Ghostboard resolve named/default `roamium`
only through an explicit absolute `TERMSURF_ROAMIUM_PATH`, so debug tests cannot
accidentally pass by using an installed Roamium. Issue 819 now needs the normal
installed-app behavior. A double-clicked installed Ghostboard will not inherit
`TERMSURF_ROAMIUM_PATH`, so named/default `roamium` needs a deliberate installed
discovery path.

This experiment will define the installed Roamium policy, preserve the debug
contract, and add a regression test that distinguishes debug behavior from
release/installed behavior.

## Changes

Planned source changes:

- `ghostboard/src/apprt/termsurf.zig`
  - Import `build_config`.
  - Keep existing behavior for absolute browser paths.
  - Keep existing debug behavior: in debug builds, named/default `roamium`
    resolves only through absolute `TERMSURF_ROAMIUM_PATH` and does not fall
    back to installed paths.
  - In non-debug builds only, if `TERMSURF_ROAMIUM_PATH` is missing, empty, or
    relative, try the installed Roamium path.
  - Define canonical installed Roamium path as
    `/opt/homebrew/opt/termsurf-roamium/roamium`, matching the Homebrew cask and
    existing `AGENTS.md` distribution contract.
  - Add `TERMSURF_INSTALLED_ROAMIUM_PATH` as an optional test override for
    release-app harness tests, accepted only when absolute. This must not alter
    debug fallback behavior.
  - Log whether `roamium` resolved through `TERMSURF_ROAMIUM_PATH`, the
    installed-path override, or the canonical installed path.
- `scripts/install.sh`
  - Align manual Roamium install destination with the canonical installed path
    by installing to `/opt/homebrew/opt/termsurf-roamium` by default.
  - Preserve a test/install override such as `TERMSURF_ROAMIUM_INSTALL_DIR` if
    needed for non-privileged future tests.
  - Keep cleanup of old `/usr/local/roamium`, `/usr/local/bin/roamium`, and
    `/usr/local/lib/roamium`.
- `scripts/uninstall.sh`
  - Remove `/opt/homebrew/opt/termsurf-roamium` for Roamium uninstall.
  - Keep cleanup of old `/usr/local/roamium`, `/usr/local/bin/roamium`, and
    `/usr/local/lib/roamium`.
- `docs/ghostboard-launch-discovery.md`
  - Preserve the Issue 814 debug contract.
  - Add the installed/release contract owned by Issue 819.
- `scripts/ghostboard-geometry-matrix.sh`
  - Add a focused release-app scenario that launches the Release
    `TermSurf Ghostboard.app` without `TERMSURF_ROAMIUM_PATH`, points
    `TERMSURF_INSTALLED_ROAMIUM_PATH` at the repo-built debug Roamium, and
    verifies named/default `roamium` resolves through installed discovery.
  - Keep existing debug scenarios proving stale installed fallback is not used
    in debug builds.

Planned issue-document changes:

- Add `## Result` and `## Conclusion` after verification.
- Update the Issue 819 README experiment status after verification.

Explicitly out of scope:

- Adding Ghostboard to `scripts/release.sh` tarball contents.
- Changing the Homebrew cask.
- Changing Chromium/Roamium binaries.
- Changing absolute `--browser` path behavior.

## Verification

Formatting actions:

```bash
zig fmt ghostboard/src/apprt/termsurf.zig
prettier --write --prose-wrap always --print-width 80 \
  docs/ghostboard-launch-discovery.md \
  issues/0819-ghostboard-packaging-identity-hardening/README.md \
  issues/0819-ghostboard-packaging-identity-hardening/09-define-installed-roamium-discovery.md
```

Static checks:

```bash
bash -n scripts/install.sh scripts/uninstall.sh scripts/ghostboard-geometry-matrix.sh
git diff --check
rg -n 'TERMSURF_ROAMIUM_PATH|TERMSURF_INSTALLED_ROAMIUM_PATH|termsurf-roamium|/usr/local/roamium|/opt/homebrew/opt/termsurf-roamium' \
  ghostboard/src/apprt/termsurf.zig \
  scripts/install.sh \
  scripts/uninstall.sh \
  scripts/ghostboard-geometry-matrix.sh \
  docs/ghostboard-launch-discovery.md
```

Build/runtime checks:

1. Build debug and release Ghostboard:

   ```bash
   scripts/build.sh ghostboard
   scripts/build.sh ghostboard --release
   ```

2. Preserve existing debug resolver tests:

   ```bash
   scripts/ghostboard-geometry-matrix.sh named-roamium-debug-launch
   scripts/ghostboard-geometry-matrix.sh named-roamium-invalid-env
   ```

3. Run the new release installed-Roamium fallback scenario:

   ```bash
   scripts/ghostboard-geometry-matrix.sh installed-roamium-release-launch
   ```

   The scenario must assert that the app log does not contain
   `env=TERMSURF_ROAMIUM_PATH path=` and does contain the installed
   override/canonical resolution log.

4. Verify manual install scripts name the same installed Roamium directory as
   the resolver and Homebrew cask:

   ```bash
   rg -n '/opt/homebrew/opt/termsurf-roamium' scripts/install.sh scripts/uninstall.sh homebrew/Casks/termsurf.rb ghostboard/src/apprt/termsurf.zig
   ```

Pass criteria:

- Debug Ghostboard still requires explicit absolute `TERMSURF_ROAMIUM_PATH` for
  named/default `roamium`; existing debug launch and invalid-env scenarios pass.
- Release Ghostboard can resolve named/default `roamium` through installed
  discovery when `TERMSURF_ROAMIUM_PATH` is unset.
- The installed path is canonical and consistent:
  `/opt/homebrew/opt/termsurf-roamium/roamium`.
- Manual install/uninstall scripts, Homebrew cask, docs, and resolver agree on
  the installed Roamium location.
- Absolute browser executable paths still bypass named-browser discovery.
- No release tarball or Homebrew cask packaging changes are made.

Partial criteria:

- Source and docs define the installed path, but the release-app harness cannot
  run in this VM.
- The release fallback can be proven only through a test override, not through a
  real installed `/opt/homebrew/opt/termsurf-roamium/roamium`.

Fail criteria:

- Debug Ghostboard falls back to installed Roamium when `TERMSURF_ROAMIUM_PATH`
  is missing or invalid.
- Installed/release Ghostboard still requires `TERMSURF_ROAMIUM_PATH` for normal
  named/default `roamium`.
- Script, cask, docs, and resolver paths remain inconsistent.

## Design Review

This experiment is plan-only until a fresh-context adversarial design review
approves it. Record the reviewer verdict here, fix all real findings, and commit
the approved plan before implementation begins.

Fresh-context adversarial design review by Codex subagent `Euler the 2nd`:

- **Verdict:** Approved.
- **Optional finding:** The override name was written loosely as “such as
  `TERMSURF_INSTALLED_ROAMIUM_PATH`.” Accepted; fixed by making
  `TERMSURF_INSTALLED_ROAMIUM_PATH` the explicit required override name.
- **Optional finding:** The release fallback scenario should be harder to fake
  because the existing harness normally exports `TERMSURF_ROAMIUM_PATH`.
  Accepted; fixed by requiring the scenario to assert no
  `env=TERMSURF_ROAMIUM_PATH path=` resolution log appears and that the
  installed override/canonical resolution log does appear.

## Completion Gate

After implementation and verification:

- add `## Result` and `## Conclusion` to this experiment file;
- update the Issue 819 README experiment status from `Designed` to `Pass`,
  `Partial`, or `Fail`;
- request a fresh-context completion review;
- fix all real completion-review findings and record the final verdict in this
  file; and
- commit the reviewed result separately before designing or implementing the
  next experiment.

## Result

**Result:** Pass

Implemented installed Roamium discovery while preserving the debug resolver
contract.

Changed files:

- `ghostboard/src/apprt/termsurf.zig`
  - `roamium` still resolves through an explicit absolute
    `TERMSURF_ROAMIUM_PATH` when that environment variable is valid.
  - Debug Zig builds return `null` after a missing, empty, or relative
    `TERMSURF_ROAMIUM_PATH`, so debug named/default `roamium` does not fall back
    to installed paths.
  - Non-debug Zig builds fall back to installed discovery. The canonical
    installed path is `/opt/homebrew/opt/termsurf-roamium/roamium`.
  - Release harnesses can set the absolute-only
    `TERMSURF_INSTALLED_ROAMIUM_PATH` override to test installed discovery
    without writing to `/opt/homebrew`.
- `scripts/build.sh`
  - Ghostboard builds now refresh `GhosttyKit.xcframework` with matching Zig
    optimization before Xcode runs: `Debug` for debug app builds and
    `ReleaseFast` for release app builds.
  - Release app builds are deep ad-hoc signed after Xcode so embedded Sparkle
    can load when the app is launched directly from the harness.
- `scripts/install.sh`
  - Manual Roamium installs now default to `/opt/homebrew/opt/termsurf-roamium`,
    with `TERMSURF_ROAMIUM_INSTALL_DIR` available as a test/install override.
  - Old `/usr/local/roamium`, `/usr/local/bin/roamium`, and
    `/usr/local/lib/roamium` locations are still cleaned up.
- `scripts/uninstall.sh`
  - Roamium uninstall removes `/opt/homebrew/opt/termsurf-roamium` by default
    and still removes old `/usr/local` locations.
- `docs/ghostboard-launch-discovery.md`
  - Documents the debug resolver contract and the release installed discovery
    contract.
- `scripts/ghostboard-geometry-matrix.sh`
  - Adds `installed-roamium-release-launch`.
  - The release scenario launches the Release app, omits
    `TERMSURF_ROAMIUM_PATH`, writes its harness config through a temporary
    `$XDG_CONFIG_HOME/termsurf/config`, and sets
    `TERMSURF_INSTALLED_ROAMIUM_PATH` to the repo-built Roamium binary.
  - Resolver-only scenarios no longer require the generic initial AppKit
    hit-test prerequisite; mouse/geometry scenarios still keep that assertion.

Verification passed:

```bash
zig fmt ghostboard/src/apprt/termsurf.zig
bash -n scripts/build.sh scripts/install.sh scripts/uninstall.sh scripts/ghostboard-geometry-matrix.sh
git diff --check
rg -n 'TERMSURF_ROAMIUM_PATH|TERMSURF_INSTALLED_ROAMIUM_PATH|termsurf-roamium|/usr/local/roamium|/opt/homebrew/opt/termsurf-roamium' \
  ghostboard/src/apprt/termsurf.zig \
  scripts/build.sh \
  scripts/install.sh \
  scripts/uninstall.sh \
  scripts/ghostboard-geometry-matrix.sh \
  docs/ghostboard-launch-discovery.md \
  homebrew/Casks/termsurf.rb
scripts/build.sh ghostboard
scripts/build.sh ghostboard --release
scripts/ghostboard-geometry-matrix.sh named-roamium-debug-launch
scripts/ghostboard-geometry-matrix.sh named-roamium-invalid-env
scripts/ghostboard-geometry-matrix.sh installed-roamium-release-launch
```

Runtime evidence:

- `named-roamium-debug-launch` passed and proved no `--browser` argument was
  used, `TERMSURF_ROAMIUM_PATH` resolved the named `roamium` browser, the debug
  Roamium path spawned, `BrowserReady` preserved `browser=roamium`, and no stale
  installed path was used.
- `named-roamium-invalid-env` passed and proved a relative
  `TERMSURF_ROAMIUM_PATH=roamium` logs a clear unresolved named-browser error,
  creates no pending `default/roamium` server, and spawns no browser.
- `installed-roamium-release-launch` passed and proved the Release app can load
  the harness config through XDG config, discover `TERMSURF_SOCKET`, receive a
  named/default `roamium` `SetOverlay`, avoid any
  `env=TERMSURF_ROAMIUM_PATH path=` resolution, resolve through
  `TERMSURF_INSTALLED_ROAMIUM_PATH`, spawn that installed override path, and
  preserve `browser=roamium` in `BrowserReady`.

The release scenario initially exposed two packaging/build issues that were
fixed in this experiment:

- the Release app launched directly from the harness could not load Sparkle
  until the build script deep-signed the app bundle after Xcode; and
- Xcode Release builds could link a stale Debug `GhosttyKit.xcframework`, so
  `build_config.is_debug` stayed true and installed fallback was disabled until
  the build script refreshed the xcframework in `ReleaseFast`.

## Conclusion

Issue 819 now has a deliberate installed Roamium discovery contract:

1. Debug Ghostboard keeps deterministic developer behavior: named/default
   `roamium` requires an absolute `TERMSURF_ROAMIUM_PATH`.
2. Release Ghostboard can launch normal named/default `roamium` without
   `TERMSURF_ROAMIUM_PATH` by resolving to the installed Roamium location.
3. Resolver, docs, manual install/uninstall scripts, and the Homebrew cask agree
   on `/opt/homebrew/opt/termsurf-roamium/roamium`.
4. The Ghostboard build script now refreshes the Zig xcframework in the correct
   optimization mode before Xcode app builds, preventing stale debug/release
   resolver behavior from leaking across builds.

The release harness proves installed discovery with an override path rather than
a real `/opt/homebrew/opt/termsurf-roamium/roamium` install, which keeps the
test non-destructive while still exercising the release-only fallback path.

## Completion Review

Fresh-context adversarial completion review by Codex subagent `Boyle the 2nd`:

- **Initial verdict:** Changes required.
- **Required finding:** `scripts/install.sh` accepted
  `TERMSURF_ROAMIUM_INSTALL_DIR`, but a normal non-root
  `scripts/install.sh roamium` would re-exec through `sudo` without preserving
  the override, causing the install to fall back to
  `/opt/homebrew/opt/termsurf-roamium`.
- **Fix:** `scripts/install.sh` and `scripts/uninstall.sh` now treat a
  non-default writable `TERMSURF_ROAMIUM_INSTALL_DIR` like the existing
  `TERMSURF_APPLICATIONS_DIR` override, and both scripts preserve
  `TERMSURF_ROAMIUM_INSTALL_DIR` through `sudo env` if escalation is still
  needed.

Fresh-context adversarial re-review by Codex subagent `Euclid the 2nd`:

- **Final verdict:** Approved.
- **Resolved finding:** The reviewer confirmed `scripts/install.sh roamium` now
  avoids `sudo` for a writable non-default `TERMSURF_ROAMIUM_INSTALL_DIR`, and
  preserves the override through `sudo env` if escalation is required.
- **Additional check:** The reviewer confirmed `scripts/uninstall.sh` mirrors
  the same override handling.
- **Read-only verification rerun by reviewer:**
  `bash -n scripts/build.sh scripts/install.sh scripts/uninstall.sh scripts/ghostboard-geometry-matrix.sh`
  and `git diff --check` passed.
