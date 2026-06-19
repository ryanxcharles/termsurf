# Experiment 5: Restore TermSurf Identity Surfaces

## Description

Experiment 4 proved that the merged macOS app launches, but it also proved that
several identity surfaces are still wrong for Issue 826:

- the app bundle is `TermSurf Ghostboard.app`;
- `CFBundleName` and `CFBundleDisplayName` are `TermSurf Ghostboard`;
- the app executable is `ghostboard`;
- the debug bundle ID is `com.termsurf.ghostboard.debug`;
- local build instructions still point at `TermSurf Ghostboard.app`.

Issue 826 requires the user-facing app identity to remain `TermSurf`, the CLI
command to remain `termsurf`, and the config path to remain
`~/.config/termsurf/config`. This experiment restores and verifies those
identity surfaces without changing protocol behavior, browser overlays, pane
geometry, or Roamium/webtui behavior.

The expected final macOS debug app bundle for this experiment is
`ghostboard/macos/build/Debug/TermSurf.app`.

## Changes

- `ghostboard/macos/Ghostty.xcodeproj/project.pbxproj`
  - Rename the macOS app product reference and product name from
    `TermSurf Ghostboard.app` / `TermSurf Ghostboard` to `TermSurf.app` /
    `TermSurf`.
  - Rename the app executable from `ghostboard` to `termsurf`, unless the build
    proves that upstream requires a different executable name internally. If
    that happens, document the reason and keep the user-facing CLI artifact
    `termsurf`.
  - Update app bundle IDs from `com.termsurf.ghostboard*` to `com.termsurf*`.
    The expected debug bundle ID is `com.termsurf.debug`; the expected release
    bundle ID is `com.termsurf`.
  - Update test host paths that point at the app bundle/executable.
  - Update the Dock Tile plugin display name and bundle ID to avoid `Ghostboard`
    in user-facing metadata.
- `ghostboard/macos/Ghostty.xcodeproj/xcshareddata/xcschemes/Ghostty.xcscheme`
  - Update buildable app names from `TermSurf Ghostboard.app` to `TermSurf.app`.
- `ghostboard/macos/AGENTS.md`
  - Update local macOS build/run instructions to use `TermSurf.app`.
- `ghostboard/HACKING.md`
  - Update the macOS app output path to use `TermSurf.app`.
- `ghostboard/macos/Sources/Features/Settings/SettingsView.swift`
  - If it still says to restart `TermSurf Ghostboard`, update that text to
    `TermSurf` while preserving the documented config path.
- `issues/0826-update-ghostboard-to-latest-ghostty/README.md`
  - Link this experiment and update its status after the result is known.
- `issues/0826-update-ghostboard-to-latest-ghostty/05-restore-termsurf-identity-surfaces.md`
  - Record design, verification, result, reviews, and conclusion.

Do not rename the `ghostboard/` source directory in this experiment. The
directory name is an internal repository boundary and is not the user-facing app
identity.

Do not do broad `ghostty` to `termsurf` rewrites. Internal Ghostty names should
stay intact unless they directly affect the Issue 826 app identity, CLI command,
config path, or user-facing strings listed above.

## Verification

Before changes, capture the current state:

```bash
git status --short
plutil -extract CFBundleName raw \
  "ghostboard/macos/build/Debug/TermSurf Ghostboard.app/Contents/Info.plist"
plutil -extract CFBundleDisplayName raw \
  "ghostboard/macos/build/Debug/TermSurf Ghostboard.app/Contents/Info.plist"
plutil -extract CFBundleIdentifier raw \
  "ghostboard/macos/build/Debug/TermSurf Ghostboard.app/Contents/Info.plist"
plutil -extract CFBundleExecutable raw \
  "ghostboard/macos/build/Debug/TermSurf Ghostboard.app/Contents/Info.plist"
test -x "ghostboard/zig-out/bin/termsurf" || true
```

Build after changes:

```bash
cd ghostboard
zig build -Demit-macos-app=false \
  > ../logs/issue-0826-exp05-zig-core.log 2>&1
macos/build.nu --configuration Debug --action clean \
  > ../logs/issue-0826-exp05-macos-clean.log 2>&1
rm -rf "macos/build/Debug/TermSurf.app" \
  "macos/build/Debug/TermSurf Ghostboard.app"
macos/build.nu --configuration Debug --action build \
  > ../logs/issue-0826-exp05-macos-build.log 2>&1
```

Verify app identity from the rebuilt bundle:

```bash
test -d "ghostboard/macos/build/Debug/TermSurf.app"
test ! -d "ghostboard/macos/build/Debug/TermSurf Ghostboard.app"
test "$(plutil -extract CFBundleName raw \
  "ghostboard/macos/build/Debug/TermSurf.app/Contents/Info.plist")" = "TermSurf"
test "$(plutil -extract CFBundleDisplayName raw \
  "ghostboard/macos/build/Debug/TermSurf.app/Contents/Info.plist")" = "TermSurf"
test "$(plutil -extract CFBundleIdentifier raw \
  "ghostboard/macos/build/Debug/TermSurf.app/Contents/Info.plist")" = "com.termsurf.debug"
test "$(plutil -extract CFBundleExecutable raw \
  "ghostboard/macos/build/Debug/TermSurf.app/Contents/Info.plist")" = "termsurf"
test -x "ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf"
! rg -n "com\\.termsurf\\.ghostboard" \
  ghostboard/macos/Ghostty.xcodeproj/project.pbxproj \
  ghostboard/macos/Ghostty.xcodeproj/xcshareddata/xcschemes/Ghostty.xcscheme
```

Verify that the Zig CLI remains `termsurf`:

```bash
cd ghostboard
zig build -Demit-exe=true -Demit-macos-app=false \
  > ../logs/issue-0826-exp05-zig-exe.log 2>&1
test -x zig-out/bin/termsurf
zig-out/bin/termsurf --version \
  > ../logs/issue-0826-exp05-termsurf-version.log 2>&1
```

Verify the config path remains TermSurf-specific:

```bash
rg -n "\\.config/termsurf|XDG_CONFIG_HOME/termsurf|config-path" \
  ghostboard/src ghostboard/macos/Sources \
  > logs/issue-0826-exp05-config-paths.log
rg -n "\\.config/ghostty|XDG_CONFIG_HOME/ghostty|config/ghostty" \
  ghostboard/src ghostboard/macos/Sources \
  > logs/issue-0826-exp05-ghostty-config-paths.log || true
```

Verify launch still works after the rename:

```bash
APP="$PWD/ghostboard/macos/build/Debug/TermSurf.app"
osascript -e "tell application \"$APP\" to activate" \
  > logs/issue-0826-exp05-launch.log 2>&1
sleep 5
ps -axo pid,comm,args \
  | rg "TermSurf.app/Contents/MacOS/termsurf|$APP" \
  > logs/issue-0826-exp05-process.log
osascript -e "tell application \"$APP\" to quit" \
  > logs/issue-0826-exp05-quit.log 2>&1
sleep 2
ps -axo pid,comm,args \
  | rg "TermSurf.app/Contents/MacOS/termsurf|$APP" \
  | rg -v 'rg|ps -axo|zsh -lc' \
  > logs/issue-0826-exp05-post-quit-process.log || true
```

Run formatting and hygiene checks:

```bash
git diff --name-only -- '*.zig' | xargs -r zig fmt
(cd ghostboard && swiftlint lint --strict --fix)
prettier --write --prose-wrap always --print-width 80 \
  issues/0826-update-ghostboard-to-latest-ghostty/README.md \
  issues/0826-update-ghostboard-to-latest-ghostty/05-restore-termsurf-identity-surfaces.md \
  ghostboard/HACKING.md \
  ghostboard/macos/AGENTS.md
git diff --check
```

Pass criteria:

- The rebuilt macOS bundle is `TermSurf.app`.
- `CFBundleName` and `CFBundleDisplayName` are `TermSurf`.
- The debug app bundle ID is `com.termsurf.debug`; release app bundle IDs use
  `com.termsurf`.
- No macOS app target bundle ID begins with `com.termsurf.ghostboard`.
- The rebuilt app executable is `termsurf`.
- The built app launches by absolute path and quits cleanly.
- The Zig CLI artifact remains `zig-out/bin/termsurf`.
- Config documentation and code continue to point at `~/.config/termsurf/config`
  or `$XDG_CONFIG_HOME/termsurf/config`.
- No `~/.config/ghostty` / `$XDG_CONFIG_HOME/ghostty` config path remains in the
  active Ghostboard app/config code.

Partial criteria:

- Some identity surfaces are fixed, but the app cannot be rebuilt/launched or an
  internal upstream assumption requires keeping a non-target executable or
  bundle ID. The first blocking mismatch must be documented with logs.

Fail criteria:

- The experiment expands into TermSurf protocol, browser overlay, pane geometry,
  webtui, or Roamium behavior before the identity surfaces are restored.
- The build cannot be invoked, or the tree is left with ambiguous app products.

## Design Review

An adversarial Codex subagent reviewed the initial design with fresh context.

**Verdict:** Changes required.

Required findings and fixes:

- Bundle ID acceptance was under-specified. Fixed by adding expected debug and
  release bundle IDs, mechanical `CFBundleIdentifier` equality checks, and pass
  criteria requiring no macOS app target bundle ID to begin with
  `com.termsurf.ghostboard`.
- Stale app artifacts could make verification ambiguous. Fixed by adding
  `macos/build.nu --configuration Debug --action clean` and explicit removal of
  the old and new app product paths before rebuilding.

The optional plist-check finding was also adopted by changing the rebuilt bundle
plist verification from value-printing commands to failing equality checks.

The re-review approved the design with no required findings. It confirmed that
the bundle ID expectations, stale-product cleanup, and exact plist checks now
address the prior findings.

## Result

**Result:** Partial

The macOS app identity surfaces were restored and verified. The standalone
`zig-out/bin/termsurf` check from the design turned out to be ambiguous under
Ghostty's macOS `app_runtime = .none` build mode:
`zig build -Demit-exe=true -Demit-macos-app=false` did not rebuild or replace
the existing `zig-out/bin/termsurf` file. The rebuilt macOS app executable is
correctly named `termsurf` and its `--version` output now says `TermSurf`, but a
later experiment should decide whether Issue 826 also requires a fresh
standalone `zig-out/bin/termsurf` artifact on macOS.

Changes made:

- `ghostboard/macos/Ghostty.xcodeproj/project.pbxproj`
  - Renamed the app product from `TermSurf Ghostboard.app` to `TermSurf.app`.
  - Renamed the app executable from `ghostboard` to `termsurf`.
  - Changed the Debug app bundle ID to `com.termsurf.debug` and release app
    bundle IDs to `com.termsurf`.
  - Changed the Dock Tile plugin display name and bundle ID to
    `TermSurf Dock Tile Plugin` and `com.termsurf.dock-tile`.
  - Updated test host paths to point at
    `TermSurf.app/$(BUNDLE_EXECUTABLE_FOLDER_PATH)/termsurf`.
  - Set `PRODUCT_MODULE_NAME = Ghostty` for the app target so internal Swift
    tests can keep importing the upstream module name while the product name is
    `TermSurf`.
- `ghostboard/macos/Ghostty.xcodeproj/xcshareddata/xcschemes/Ghostty.xcscheme`
  - Updated app buildable names to `TermSurf.app`.
- `ghostboard/macos/AGENTS.md` and `ghostboard/HACKING.md`
  - Updated local build/run docs to point at `TermSurf.app`.
- `ghostboard/macos/Sources/Features/Settings/SettingsView.swift`
  - Updated user-facing settings text from `TermSurf Ghostboard` to `TermSurf`.
- `ghostboard/macos/Sources/Ghostty/Ghostty.Config.swift`
  - Updated the default custom icon path to `~/.config/termsurf/TermSurf.icns`.
- `ghostboard/src/config/url.zig`, `ghostboard/src/config/Config.zig`, and
  `ghostboard/src/build/GhosttyResources.zig`
  - Updated config-path examples/tests/comments from Ghostty config paths to
    TermSurf config paths.
- `ghostboard/src/cli/version.zig`
  - Updated the user-facing version banner to `TermSurf`.
- `ghostboard/src/build/GhosttyXcodebuild.zig`
  - Updated the active Zig macOS app helper to use `TermSurf.app` and
    `Contents/MacOS/termsurf` for `zig build run`.

Verification:

- Pre-change built bundle values were captured:
  - `CFBundleName`: `TermSurf Ghostboard`
  - `CFBundleDisplayName`: `TermSurf Ghostboard`
  - `CFBundleIdentifier`: `com.termsurf.ghostboard.debug`
  - `CFBundleExecutable`: `ghostboard`
  - `ghostboard/zig-out/bin/termsurf` existed before the experiment.
- Clean rebuild passed:
  - `zig build -Demit-macos-app=false`
  - `macos/build.nu --configuration Debug --action clean`
  - removed stale `macos/build/Debug/TermSurf.app` and
    `macos/build/Debug/TermSurf Ghostboard.app`
  - `macos/build.nu --configuration Debug --action build`
- Rebuilt bundle identity checks passed:
  - `ghostboard/macos/build/Debug/TermSurf.app` exists.
  - `ghostboard/macos/build/Debug/TermSurf Ghostboard.app` is absent.
  - `CFBundleName` is `TermSurf`.
  - `CFBundleDisplayName` is `TermSurf`.
  - `CFBundleIdentifier` is `com.termsurf.debug`.
  - `CFBundleExecutable` is `termsurf`.
  - `Contents/MacOS/termsurf` is executable.
  - `Ghostty.swiftmodule` exists, proving the internal Swift module name stayed
    `Ghostty`.
  - `com.termsurf.ghostboard` no longer appears in the macOS app project or
    scheme.
- `macos/build.nu --configuration Debug --action test` passed. This was added
  after the first test attempt failed because the product rename changed the
  Swift module from `Ghostty` to `TermSurf`; setting
  `PRODUCT_MODULE_NAME = Ghostty` fixed it.
- Config path verification found 25 TermSurf config-path hits. The remaining two
  `config/ghostty` grep hits are false positives in
  `share/pkgconfig/ghostty-internal.pc` and
  `share/pkgconfig/ghostty-internal-static.pc`; they are not user config paths.
- The rebuilt macOS executable version check passed:

  ```text
  TermSurf 1.3.2-main-+6e0b74697
  ```

- `zig build run -- --version` passed and printed `TermSurf`, proving the Zig
  macOS app helper uses the renamed app/executable path.
- Launch verification passed:
  - `osascript` launch log was empty.
  - process check found exactly one built-app process:
    `ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf`.
  - `osascript` quit log was empty.
  - post-quit process check was empty.
- Formatting and hygiene passed:
  - `zig fmt` on changed Zig files.
  - `swiftlint lint --strict --fix` in `ghostboard/`.
  - Prettier on changed Markdown docs.
  - `git diff --check`.

Standalone CLI note:

- `ghostboard/zig-out/bin/termsurf --version` initially still printed `Ghostty`,
  but the file timestamp showed it was a stale artifact from before this
  experiment.
- Re-running `zig build -Demit-exe=true -Demit-macos-app=false` under the
  default macOS build mode did not rebuild `zig-out/bin/termsurf`, because
  Ghostty's `app_runtime = .none` path builds the macOS library/app support
  artifacts rather than installing the standalone executable.
- Trying `zig build -Dapp-runtime=gtk -Demit-exe=true -Demit-macos-app=false`
  reached the standalone executable path but failed on missing local GTK
  dependencies (`gtk4` and `libadwaita-1`). This does not affect the verified
  macOS app executable, but it means the standalone `zig-out/bin/termsurf`
  criterion is not solved by this experiment.

## Conclusion

The macOS app identity is restored: the rebuilt bundle is `TermSurf.app`, the
visible name is `TermSurf`, the bundle ID no longer contains `ghostboard`, the
app executable is `termsurf`, the internal Swift module remains `Ghostty`, and
the app still builds, tests, launches, and quits.

Config-path identity is also restored for the active app/config surfaces checked
in this experiment.

The result review initially found one required issue: the active
`GhosttyXcodebuild.zig` helper still pointed at
`TermSurf Ghostboard.app/Contents/MacOS/ghostboard`. This was fixed by updating
the helper to `TermSurf.app/Contents/MacOS/termsurf` and verified with
`zig build run -- --version`.

The next experiment should decide and verify the standalone CLI artifact story:
whether Issue 826 needs `zig-out/bin/termsurf` to be freshly produced on macOS,
whether that should come from the app executable, or whether it belongs to later
packaging/install work rather than the upstream-merge parity issue.

## Result Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Verdict:** Changes required.

Required finding and fix:

- `ghostboard/src/build/GhosttyXcodebuild.zig` still pointed the active Zig
  macOS app helper at `TermSurf Ghostboard.app/Contents/MacOS/ghostboard`. Fixed
  by updating the helper path to `TermSurf.app/Contents/MacOS/termsurf` and the
  run-step label to `run TermSurf app`.

The re-review approved the fix with no findings. It confirmed the helper path,
run executable, result documentation, and `zig build run -- --version` log all
match the restored TermSurf identity.
