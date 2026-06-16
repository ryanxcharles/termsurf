# Experiment 2: Build the Pristine Ghostty Import

## Description

Build the freshly imported upstream Ghostty `v1.3.1` tree before making any
Ghostboard-specific branding, config-path, icon, CLI, build-system, or protocol
changes. The goal is to prove the imported baseline builds in this VM and to
record any toolchain or source-build blocker before we start modifying the tree.

This experiment deliberately does not rename the app, change user-facing text,
change the config path, copy the Wezboard icon, implement TermSurf protocol
messages, launch the app, or modify `webtui` or `roamium`.

## Changes

No source changes are planned for this experiment. The only repository changes
should be this experiment's result documentation after the build attempt.

The build flow follows the imported Ghostty macOS instructions:

1. Verify the build tools are available:

   ```bash
   command -v zig
   zig version
   command -v nu
   xcode-select -p
   xcodebuild -version
   ```

2. Verify the working tree is clean before building:

   ```bash
   git status --short
   ```

3. Build the underlying library from the Ghostty subtree root:

   ```bash
   cd ghostboard
   zig build -Demit-macos-app=false
   ```

4. Build the macOS app using Ghostty's wrapper script, as required by
   `ghostboard/macos/AGENTS.md`:

   ```bash
   cd ghostboard
   macos/build.nu --configuration Debug --action build
   ```

5. Record the result in this experiment file.

Do not edit files under `ghostboard/` in this experiment. If the build fails,
record the error and conclusion without changing source.

## Verification

Pass criteria:

- `command -v zig` succeeds.
- `zig version` satisfies the imported Ghostty minimum Zig version in
  `ghostboard/build.zig.zon`.
- `command -v nu` succeeds because `ghostboard/macos/build.nu` requires Nushell.
- `xcode-select -p` points at full Xcode, not only Command Line Tools.
- `xcodebuild -version` reports the selected Xcode version.
- `git status --short` is clean before building.
- `cd ghostboard && zig build -Demit-macos-app=false` succeeds.
- `cd ghostboard && macos/build.nu --configuration Debug --action build`
  succeeds.
- The debug app bundle exists at `ghostboard/macos/build/Debug/Ghostty.app`.
- Any generated build output is ignored or otherwise does not leave source
  changes in `git status --short`.
- The issue README lists this experiment as `Pass` only after the result is
  recorded.

Fail criteria:

- Required tooling is missing.
- Xcode selection is invalid for building Ghostty.
- The underlying `zig build -Demit-macos-app=false` fails.
- The macOS `build.nu` app build fails.
- The build modifies tracked source files.
- The experiment makes branding, config-path, icon, CLI, build-system, protocol,
  `webtui`, or `roamium` changes.

## Notes

If the build fails for environmental reasons, the result should be `Fail` or
`Partial` with the exact blocker and the next experiment should address that
blocker. Do not silently turn a build failure into a source edit inside this
experiment.

## Design Review

Fresh-context adversarial review returned `APPROVED`.

- Optional: the original design required `command -v zig` to report a
  Homebrew-installed executable, but imported Ghostty only requires a compatible
  Zig version. Fixed by adding `zig version` to the tool checks and changing the
  pass criterion to require a version satisfying `ghostboard/build.zig.zon`.
- The reviewer confirmed the README links Experiment 2 as `Designed`, the
  experiment has the required sections, the scope is a clean build baseline, the
  macOS commands match `ghostboard/macos/AGENTS.md`, and the expected app output
  path matches `ghostboard/macos/build/Debug/Ghostty.app`.
- No required findings were reported.
