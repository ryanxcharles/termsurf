# Experiment 3: Implement macOS Bundle Identity

## Description

Experiment 2 decided that Ghostboard should ship as `TermSurf Ghostboard.app`
with bundle identifiers under `com.termsurf.ghostboard` and executable/CLI name
`ghostboard`. Current Ghostboard build outputs still produce `TermSurf.app`,
`com.termsurf.debug`, and executable `termsurf`.

This experiment will implement only the macOS app bundle identity portion of
that contract in Ghostboard's Xcode/build outputs. It will not add repo-level
release packaging, Homebrew packaging, config-path changes, installed Roamium
discovery, or broad source-symbol renames.

## Changes

Planned source changes:

- `ghostboard/macos/Ghostty.xcodeproj/project.pbxproj`
  - Change the macOS app target display/product identity from `TermSurf` to
    `TermSurf Ghostboard`.
  - Change the macOS app executable name from `termsurf` to `ghostboard`.
  - Change macOS app bundle identifiers from `com.termsurf` /
    `com.termsurf.debug` to `com.termsurf.ghostboard` /
    `com.termsurf.ghostboard.debug`.
  - Change the Dock Tile plugin display name and bundle id to the
    `TermSurf Ghostboard` / `com.termsurf.ghostboard.*` family.
- `ghostboard/macos/Ghostty.xcodeproj/xcshareddata/xcschemes/Ghostty.xcscheme`
  - Update Ghostty app `BuildableName` references from `TermSurf.app` to
    `TermSurf Ghostboard.app`.
- Build-output or launch helper references, only if required by the app rename:
  - `ghostboard/macos/build.nu`
  - `ghostboard/src/build/GhosttyXcodebuild.zig`
  - `scripts/ghostboard-geometry-matrix.sh`

Planned issue-document changes:

- Add `## Result` and `## Conclusion` after verification.
- Update the Issue 819 README experiment status after verification.

Explicitly out of scope:

- AppleScript dictionary text and `Ghostty.sdef` resource naming.
- Settings UI/config-path text.
- Repo-level `scripts/build.sh`, `scripts/install.sh`, `scripts/release.sh`, and
  Homebrew packaging.
- iOS target identity.
- Broad project, target, source directory, or implementation symbol renames.

## Verification

Formatting actions:

1. `prettier --write --prose-wrap always --print-width 80 issues/0819-ghostboard-packaging-identity-hardening/README.md issues/0819-ghostboard-packaging-identity-hardening/03-implement-macos-bundle-identity.md`.

Static checks:

1. `git diff --check`.

Build and bundle checks:

1. Remove stale debug build outputs for both old and new app names before
   building:

   ```bash
   rm -rf ghostboard/macos/build/Debug/TermSurf.app
   rm -rf ghostboard/macos/build/Debug/TermSurf\\ Ghostboard.app
   ```

2. Build the debug Ghostboard macOS app using the existing Ghostboard build
   path.
3. Inspect the generated app bundle with `PlistBuddy`:

   ```bash
   /usr/libexec/PlistBuddy -c 'Print :CFBundleName' ghostboard/macos/build/Debug/TermSurf\\ Ghostboard.app/Contents/Info.plist
   /usr/libexec/PlistBuddy -c 'Print :CFBundleDisplayName' ghostboard/macos/build/Debug/TermSurf\\ Ghostboard.app/Contents/Info.plist
   /usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' ghostboard/macos/build/Debug/TermSurf\\ Ghostboard.app/Contents/Info.plist
   /usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' ghostboard/macos/build/Debug/TermSurf\\ Ghostboard.app/Contents/Info.plist
   test -x ghostboard/macos/build/Debug/TermSurf\\ Ghostboard.app/Contents/MacOS/ghostboard
   ```

4. Confirm the old debug app identity was not recreated by the clean rebuild:

   ```bash
   test ! -e ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf
   ```

5. Run a focused existing Ghostboard runtime smoke scenario with the renamed app
   path. Prefer the cheapest scenario that proves the harness can launch the
   generated app and discover `TERMSURF_SOCKET`.

Pass criteria:

- The debug macOS app builds as `TermSurf Ghostboard.app`.
- The rebuilt bundle reports:
  - `CFBundleName = TermSurf Ghostboard`;
  - `CFBundleDisplayName = TermSurf Ghostboard`;
  - `CFBundleIdentifier = com.termsurf.ghostboard.debug`;
  - `CFBundleExecutable = ghostboard`.
- The generated executable exists at
  `TermSurf Ghostboard.app/Contents/MacOS/ghostboard`.
- No active rebuilt debug output still exposes `TermSurf.app` with executable
  `termsurf`.
- A focused Ghostboard runtime smoke test launches the renamed app successfully.
- Changes are limited to macOS bundle identity and any required launch/build
  helper references.

Partial criteria:

- The bundle metadata builds correctly, but an existing harness script still
  needs a follow-up update to locate the renamed app.
- The app builds and launches manually, but the runtime smoke cannot run because
  of an unrelated local environment problem.

Fail criteria:

- The app cannot build after the identity change.
- The rebuilt app still uses `TermSurf.app`, `com.termsurf.debug`, or executable
  `termsurf`.
- The experiment changes out-of-scope config, Homebrew, release packaging,
  AppleScript dictionary, or broad source names.

## Design Review

This experiment is plan-only until a fresh-context adversarial design review
approves it. Record the reviewer verdict here, fix all real findings, and commit
the approved plan before implementation begins.

Fresh-context adversarial design review by Codex subagent `Godel the 2nd`:

- **Initial verdict:** Changes required.
- **Required finding:** The planned source changes omitted the shared Xcode
  scheme even though `Ghostty.xcscheme` hardcoded
  `BuildableName = "TermSurf.app"`. Fixed by adding the scheme file and
  requiring the `BuildableName` references to become `TermSurf Ghostboard.app`.
- **Required finding:** The old-output verification could falsely fail or fail
  to prove the active output because a stale `TermSurf.app` already existed.
  Fixed by making verification remove stale old/new debug app outputs before
  rebuilding and then checking that the old executable was not recreated.
- **Re-review verdict:** Approved. The reviewer confirmed the scheme file and
  clean-output verification fixes resolve the required findings and introduce no
  new Required finding.

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
