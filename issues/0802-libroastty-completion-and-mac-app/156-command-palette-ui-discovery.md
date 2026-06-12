# Experiment 156: Phase G — command-palette UI discovery

## Description

Experiment 155 proved the command-palette command-entry/delegate action path
with hosted macOS tests, but it also exposed a remaining gap in the full UI
gate:

```text
cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyCommandPaletteTests
```

now exits successfully in this environment, but reports `Executed 0 tests`. That
is not a pass. It means the UI-test target builds and starts, but XCTest does
not discover or schedule the selected command-palette tests.

The likely cause is `RoasttyCustomConfigCase.defaultTestSuite`: the copied app
used that override to suppress UI tests outside Xcode, but Experiment 129
already moved that policy into `macos/build.nu` by skipping `RoasttyUITests` in
normal CLI runs and enabling them only with `--ui-tests`. The class-level suite
override is now redundant and appears to break focused CLI discovery.

This experiment makes the focused command-palette UI selector execute real test
bodies again. It does not need to make every UI test part of the default test
path; the default hosted test path must still skip UI tests.

## Changes

- `roastty/macos/RoasttyUITests/RoasttyCustomConfigCase.swift`
  - Remove the `defaultTestSuite` override, or replace it with a mechanism that
    does not interfere with XCTest discovery.
  - Preserve the intended policy through `macos/build.nu`: ordinary
    `--action test` skips `RoasttyUITests`; explicit `--ui-tests` runs them.
  - Keep the temporary config file behavior, user defaults suite, launch
    environment, and teardown behavior unchanged.
- `roastty/macos/build.nu`
  - Adjust comments or environment setup if needed so the test policy is clear:
    non-UI test runs skip the UI target; explicit `--ui-tests` runs the UI
    target; focused `RoasttyUITests/...` selectors skip unit-test execution.
  - Do not make UI tests part of the default test action.
- `roastty/macos/RoasttyUITests/RoasttyCommandPaletteTests.swift`
  - Only change these tests if discovery reveals a small selector/signature
    issue that prevents execution.
  - Keep the action-execution assertions from Experiment 129: keyboard and mouse
    command selection must have an observable postcondition, not just dismissal.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - If the focused UI selector executes real command-palette tests, update the
    operating notes and Phase G command-palette checklist to distinguish hosted
    action-path coverage from full focused UI coverage.
  - If test bodies execute but fail on product behavior, record the result as
    `Partial` and leave the product gap visible for the next experiment.

Out of scope:

- Running the entire UI-test suite by default.
- Fixing unrelated UI tests outside `RoasttyCommandPaletteTests`.
- Rewriting the command-palette UI or replacing XCTest UI automation.
- Native keymaps, global shortcut installation, or unrelated Phase G work.

## Verification

- Format markdown:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/156-command-palette-ui-discovery.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Swift lint for edited Swift files:
  - `swiftlint lint roastty/macos/RoasttyUITests/RoasttyCustomConfigCase.swift roastty/macos/RoasttyUITests/RoasttyCommandPaletteTests.swift roastty/macos/build.nu`
    if `swiftlint` is available; omit `build.nu` if SwiftLint rejects non-Swift
    files.
- Hosted macOS tests still skip UI by default:
  - `cd roastty && macos/build.nu --action test`
- Focused command-palette UI gate:
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyCommandPaletteTests`
  - The result must report real `RoasttyCommandPaletteTests` test execution. A
    process success with `Executed 0 tests` is not acceptable.
- If the focused class selector remains ambiguous, run individual selectors:
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyCommandPaletteTests/testDismissingCommandPalette`
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyCommandPaletteTests/testSelectCommandWithMouse`
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyCommandPaletteTests/testSelectCommandWithKeyboard`
- Hygiene:
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/156-command-palette-ui-discovery.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = normal `macos/build.nu --action test` still skips UI tests and
passes, while the focused command-palette UI selector executes real
`RoasttyCommandPaletteTests` test bodies and those tests pass, proving
open/dismiss/filter/keyboard-submit/mouse-select behavior through XCTest UI
automation.

**Partial** = the selector now executes real test bodies, but one or more tests
fail on command-palette product behavior or environment constraints that need a
follow-up experiment.

**Fail** = focused `RoasttyCommandPaletteTests` still executes zero tests, or
the only way to discover them is to make UI tests run by default.

## Design Review

**Reviewer:** Codex-native adversarial subagent `Hubble` with fresh context,
using the `adversarial-review` skill's Codex path
(`multi_agent_v1.spawn_agent`), not Claude's named `adversarial-reviewer` agent.

**Verdict:** Approved.

**Findings:** No Required, Optional, or Nit findings.

**Final verdict:** Approved.
