# Experiment 129: Phase G — command palette UI gate

## Description

Make command-palette UI behavior an explicit, runnable Roastty app verification
gate.

The copied macOS app already contains the upstream command-palette UI path:
`toggle_command_palette` posts a surface-scoped notification, the terminal
controller toggles `commandPaletteIsShowing`, `TerminalCommandPaletteView`
renders the palette over the focused surface, and submitted commands call
`roastty_surface_binding_action`. The copied app also already has
`RoasttyCommandPaletteTests`, but the macOS build wrapper skips UI tests during
normal CLI test runs and the UI test base class only enables tests when
`IDE_DISABLED_OS_ACTIVITY_DT_MODE` is present. That means Issue 802 still cannot
claim command-palette UI behavior is automatically verified from the project
tooling.

This experiment adds a deliberate opt-in UI-test path for the macOS build
wrapper and uses it to run the existing command-palette UI tests against the
copied Roastty app. If the existing tests expose gaps, fix only the smallest
copied-app or Roastty ABI behavior needed for command-palette parity.

## Changes

- `roastty/macos/build.nu`
  - Keep the current default behavior: `macos/build.nu --action test` still
    skips `RoasttyUITests`.
  - Add an explicit flag such as `--ui-tests` that:
    - stops passing `-skip-testing RoasttyUITests`;
    - sets `IDE_DISABLED_OS_ACTIVITY_DT_MODE=1` so
      `RoasttyCustomConfigCase.defaultTestSuite` enables the UI tests under
      CLI-driven `xcodebuild`;
    - allows a focused `-only-testing` selector, or otherwise documents the
      exact xcodebuild selector used for `RoasttyCommandPaletteTests`.
- `roastty/macos/RoasttyUITests/RoasttyCommandPaletteTests.swift`
  - Reuse the copied upstream tests where they already prove behavior.
  - Add mandatory focused coverage for the Roastty-specific action path where
    dismissal alone is not enough proof:
    - Cmd-Shift-P / menu command opens the command palette;
    - Escape and outside click dismiss it;
    - keyboard submission of a filtered command has an observable postcondition
      that fails if the palette merely dismisses without executing through
      `roastty_surface_binding_action`;
    - mouse selection of a command has an observable postcondition that fails if
      the palette merely dismisses without executing through the copied app
      delegate path.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - If the UI gate runs and passes, update the Phase G checklist and operating
    notes so command-palette UI behavior is no longer listed as remaining work.

Out of scope:

- Native keymaps and keyboard-layout reload.
- Native global shortcut registration.
- Rewriting the copied SwiftUI command-palette UI.
- Making all macOS UI tests part of the default `macos/build.nu --action test`
  path.

## Design Review

**Reviewer:** Codex-native adversarial review subagent, fresh context.

**Initial verdict:** Changes required.

**Finding:** The initial design allowed reusing existing command-palette UI
tests unchanged even though they can pass on dismissal alone without proving
selected actions execute through `roastty_surface_binding_action`.

**Fix:** The design now makes additional action-execution coverage mandatory for
both keyboard-submitted and mouse-selected commands, with observable
postconditions that fail on dismissal-only behavior.

**Final verdict:** Approved. The reviewer found no remaining required findings
and confirmed the `--ui-tests` / `--only-testing` path is feasible for the
existing Nushell/xcodebuild wrapper.

## Verification

- Run formatting/linting as applicable:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/129-command-palette-ui-gate.md issues/0802-libroastty-completion-and-mac-app/README.md`
  - `swiftlint` for edited Swift files, if any Swift files change and the
    command is available.
- Build/test the macOS app through project tooling:
  - `cd roastty && macos/build.nu --action build`
  - `cd roastty && macos/build.nu --action test`
- Run the focused command-palette UI gate using the new explicit opt-in path,
  for example:
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyCommandPaletteTests`
- If Rust outside `roastty/macos/` changes, also run:
  - `zig build -Demit-macos-app=false`
  - `cargo test -p roastty command_palette`
- Run `git diff --check`.
- Run the same Prettier command with `--check`.

**Pass** = the copied app's command-palette UI opens, dismisses, filters, and
executes keyboard-submitted and mouse-selected actions through the Roastty
surface binding action path in a CLI-runnable UI test gate, with observable
postconditions that would fail on dismissal-only behavior, and normal non-UI
macOS tests remain unchanged.

**Partial** = the build wrapper can opt into UI tests, but command-palette UI
execution fails because of an app/runtime behavior gap that needs a follow-up
experiment.

**Fail** = command-palette UI verification cannot run from CLI tooling without a
larger macOS test harness or permission redesign.
