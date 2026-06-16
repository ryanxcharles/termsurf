# Experiment 4: Reproduce the Upstream macOS Baseline

## Description

Establish the pristine Ghostty `v1.3.1` macOS build and run baseline without
making any source changes under `ghostboard/`. Experiment 3 failed because it
tried to fix a local link failure by modifying imported Ghostty code. This
experiment treats build and launch failures as environment, cache, permission,
or invocation problems until the upstream baseline is proven.

The goal is to make the imported `ghostboard/` tree build and run as plain
Ghostty `v1.3.1` on macOS. Ghostboard branding, config paths, CLI names, icons,
protocol code, Xcode project edits, Zig build edits, and vendored source edits
remain out of scope.

## Changes

No source changes under `ghostboard/` are allowed in this experiment. The only
repository changes should be this experiment's result documentation and README
status updates.

The experiment will use upstream Ghostty's own instructions and CI evidence:

- `ghostboard/HACKING.md` says development builds use `zig build`, and macOS app
  builds require full Xcode, the macOS SDK, the iOS SDK, and the Metal
  Toolchain.
- `ghostboard/macos/AGENTS.md` says the macOS app is built with
  `macos/build.nu`, and `zig build -Demit-macos-app=false` is only the
  underlying-library step.
- `ghostboard/.github/workflows/test.yml` builds the macOS app by first running
  the Zig build through Nix with `--system`, then running
  `xcodebuild -target Ghostty` outside Nix because Nix breaks `xcodebuild`.
- `ghostboard/.github/workflows/release-tag.yml` uses the same split:
  `nix develop -c zig build ... -Demit-macos-app=false`, followed by native
  `xcodebuild -target Ghostty -configuration Release`.

Steps:

1. Verify no source changes are present under `ghostboard/`:

   ```bash
   git diff -- ghostboard
   git status --short ghostboard
   ```

2. Record the relevant environment:

   ```bash
   command -v zig
   zig version
   command -v nix || true
   command -v nu || true
   command -v swiftlint || true
   xcode-select -p
   xcodebuild -version
   xcrun --sdk macosx --show-sdk-version
   xcrun --sdk iphoneos --show-sdk-version
   xcrun metal -v
   ```

3. Clean only ignored generated outputs:

   ```bash
   rm -rf ghostboard/.zig-cache \
     ghostboard/zig-out \
     ghostboard/macos/GhosttyKit.xcframework \
     ghostboard/macos/build
   ```

4. If Nix is available, reproduce upstream CI's dependency path first:

   ```bash
   cd ghostboard
   nix build -L .#deps
   GHOSTTY_DEPS="$(readlink ./result)"
   nix develop -c zig build --system "$GHOSTTY_DEPS" -Demit-macos-app=false
   cd macos
   xcodebuild -target Ghostty \
     COMPILATION_CACHE_CAS_PATH="$HOME/Library/Developer/Xcode/DerivedData/CompilationCache.noindex" \
     COMPILATION_CACHE_KEEP_CAS_DIRECTORY=YES
   ```

5. If Nix is not available, record that as an environment gap and run the
   imported local instructions without editing source:

   ```bash
   cd ghostboard
   zig build -Demit-macos-app=false
   macos/build.nu --configuration Debug --action build
   ```

6. If the app builds, launch it by absolute path and verify it runs as upstream
   Ghostty:

   ```bash
   osascript -e 'tell application "'"$(pwd)"'/macos/build/Debug/Ghostty.app" to activate'
   sleep 5
   pgrep -fl Ghostty
   osascript -e 'tell application "'"$(pwd)"'/macos/build/Debug/Ghostty.app" to quit'
   ```

   If the upstream-style `xcodebuild` command produces
   `macos/build/Release/Ghostty.app`, use that absolute path instead.

7. Record all logs under `logs/`, record whether any environment changes were
   required, and verify `git diff -- ghostboard` remains empty.

## Verification

Pass criteria:

- `git diff -- ghostboard` is empty before and after the experiment.
- No tracked or untracked source file is added, edited, or removed under
  `ghostboard/`.
- The environment has full Xcode selected with macOS, iOS, and Metal toolchains
  visible.
- The imported Ghostty `v1.3.1` tree builds the macOS app without source edits.
- The built app exists at the expected upstream output path.
- The built app launches by absolute path, runs long enough to create a terminal
  window/process, and quits cleanly.
- The issue records the exact invocation that worked so future Ghostboard
  modifications start from a proven pristine baseline.

Fail criteria:

- Any source change under `ghostboard/` is made or required.
- The experiment attempts to fix the baseline by modifying Ghostty source, Xcode
  project files, Zig build files, vendored code, branding, config paths, CLI
  names, icons, protocol code, `webtui`, or `roamium`.
- Required macOS build tools are missing and cannot be treated as an environment
  gap.
- The app still fails to build or fails to launch.
- The result does not identify whether the remaining blocker is environment,
  cache, permission, or invocation related.

## Notes

This experiment deliberately stops before any Ghostboard porting work. A passing
result means the imported tree is proven as upstream Ghostty on this VM. Only
then can later experiments modify `ghostboard/` for TermSurf branding and
protocol support.

## Design Review

Fresh-context adversarial review returned `APPROVED`.

- The reviewer checked the issue README, this experiment design, the prior
  failed Experiment 3, and upstream/imported Ghostty docs and CI workflows.
- The reviewer confirmed the README links Experiment 4 as `Designed`, the
  experiment has the required sections, and the scope makes zero source changes
  under `ghostboard/`.
- The reviewer confirmed the design follows the user instruction to prove the
  pristine macOS build and run baseline before any Ghostboard-specific code
  changes.
- No required findings were reported.
