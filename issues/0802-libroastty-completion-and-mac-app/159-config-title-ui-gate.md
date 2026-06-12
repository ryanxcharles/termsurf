# Experiment 159: Phase G — config title UI gate

## Description

Experiment 158 stopped at layer 1 of the terminal-output oracle: the focused
copied-app UI selector could not prove that the UI-test config reached the
visible first window/surface. The new `RoasttyTerminalOutputUITests` skipped
with `Configured window title was not visible; actual title: 👻`, and a
follow-up diagnostic run of the pre-existing `RoasttyTitleUITests` failed with
an empty window title instead of `RoasttyUITestsLaunchTests`.

This experiment targets that missing prerequisite only. The goal is to make the
copied-app UI test harness reliably prove that `ROASTTY_CONFIG_PATH` is loaded
and applied to the first visible window title. Once that gate is trustworthy,
the terminal-output oracle can be attempted again in a later experiment.

The likely failing layers are:

1. The UI test leaves or restores app state from a prior launch, so the first
   window is not a fresh config-derived window.
2. `RoasttyCustomConfigCase` writes the config file and passes
   `ROASTTY_CONFIG_PATH`, but launch/restoration opens a window before the
   configured title is applied.
3. The config is loaded, but the visible `NSWindow.title` is overwritten by
   surface/title restoration or by an early default terminal title before the
   test can observe it.
4. The XCTest title query is looking at the wrong window when restored or
   transient windows exist.

## Changes

- `roastty/macos/RoasttyUITests/RoasttyCustomConfigCase.swift`
  - Make the custom-config UI harness isolate test state strongly enough that a
    focused test observes a fresh config-derived window.
  - Prefer existing app/test knobs first, such as clearing the test defaults
    suite or suppressing state restoration, before adding new hooks.
  - If a new debug-only launch environment hook is necessary, it must be inert
    unless enabled by the test and must not change release behavior.
- `roastty/macos/RoasttyUITests/RoasttyTitleUITests.swift`
  - Strengthen the focused title test so it waits for the main window and fails
    with useful window/debug information if the configured title is absent.
  - The test must execute exactly one real test and pass with 0 skips.
- `roastty/macos/RoasttyUITests/RoasttyTerminalOutputUITests.swift`
  - Keep its title gate aligned with the fixed harness, but do not require the
    terminal marker to pass in this experiment unless the title fix naturally
    proves it.
- `roastty/macos/Sources/...`
  - Touch app code only if the failure is a product/testability bug in
    restoration or first-window config application. Keep any production change
    narrowly tied to first-launch config correctness.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Add the experiment index line and update the Phase G native-key note with
    the result.

Out of scope:

- Fixing terminal output visibility beyond the title/config gate.
- Turning the dead-key UI test into a 0-skip Pass.
- Reworking window restoration broadly.
- Changing release behavior solely for UI-test convenience.
- Replacing the copied-app UI test with a hosted unit test.

## Verification

- Format markdown:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/159-config-title-ui-gate.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Swift lint for edited Swift files:
  - `swiftlint lint roastty/macos/RoasttyUITests/RoasttyCustomConfigCase.swift roastty/macos/RoasttyUITests/RoasttyTitleUITests.swift`
  - Include `RoasttyTerminalOutputUITests.swift` or any edited app source files
    in the same lint run.
- Default hosted app tests still skip UI by default:
  - `cd roastty && macos/build.nu --action test`
- Focused title UI gate:
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyTitleUITests`
  - Must report exactly 1 executed test, 0 skips, and 0 failures.
  - Must prove the configured title `RoasttyUITestsLaunchTests` is visible on
    the copied app window.
- Focused terminal-output UI diagnostic:
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyTerminalOutputUITests`
  - Must report exactly 1 executed test. If it still skips, the skip must occur
    after the configured-title gate, not before it.
- Hygiene:
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/159-config-title-ui-gate.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = the focused title UI selector proves the copied app loads the custom
config title with exactly 1 executed test, 0 skips, and 0 failures; the
terminal-output selector reaches at least the title gate.

**Partial** = the experiment identifies a narrower title/config blocker but
cannot fix it without a larger restoration/config change.

**Fail** = the focused selector executes zero tests, relies on a hosted unit
test instead of the copied app, or weakens the config-title assertion into a
skip-only diagnostic.

## Design Review

**Reviewer:** Codex-native adversarial subagent `Dewey` with fresh context,
using the `adversarial-review` skill's Codex path
(`multi_agent_v1.spawn_agent`), not Claude's named `adversarial-reviewer` agent.

**Verdict:** Approved.

The reviewer found no Required findings. The review confirmed that the README
links Experiment 159 as `Designed`, the design has Description, Changes,
Verification, and Pass/Partial/Fail criteria, the scope follows directly from
Experiment 158's title/config blocker, it does not claim terminal-output or
dead-key success, and it includes concrete UI test counts plus hygiene checks.

## Result

**Result:** Pass

The copied-app custom-config title gate is now reliable.

Changes made:

- `roastty/macos/RoasttyUITests/RoasttyCustomConfigCase.swift`
  - Sets `ROASTTY_CLEAR_USER_DEFAULTS=YES` for custom-config UI launches so the
    shared helper starts from a clean test defaults suite.
- `roastty/macos/RoasttyUITests/RoasttyTitleUITests.swift`
  - Writes the title config from `setUpWithError`, after the helper's empty
    config reset, instead of relying on the async setup ordering.
  - Waits for the first window and polls all window titles, failing with the
    observed titles plus the accessibility tree if the configured title is not
    visible.
- `roastty/macos/Sources/Features/Terminal/BaseTerminalController.swift`
  - Carries `config.title` into the controller's derived config and reapplies
    the configured title after config reloads.
  - Keeps explicit runtime title overrides first, then configured title, then
    the focused surface title.

Verification:

- `swiftlint lint roastty/macos/RoasttyUITests/RoasttyCustomConfigCase.swift roastty/macos/RoasttyUITests/RoasttyTitleUITests.swift roastty/macos/Sources/Features/Terminal/BaseTerminalController.swift`
  - Pass, 0 violations.
- `git diff --check`
  - Pass.
- `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyTitleUITests`
  - Pass. The focused copied-app selector executed exactly 1 test with 0 skips
    and 0 failures.
- `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyTerminalOutputUITests`
  - Pass with 1 executed test and 1 skip. The skip now occurs after the
    configured-title gate; the accessibility tree shows the visible window and
    Window menu item titled `RoasttyTerminalOutputUITests`.
- `cd roastty && macos/build.nu --action test`
  - The first run was invalid because it ran in parallel with the focused UI
    diagnostic and collided while copying `libroastty.a` into the xcframework.
    The rerun by itself passed: 213 tests in 23 suites.

## Conclusion

Experiment 159 restores layer 1 of the copied-app UI oracle. The UI harness now
proves that `ROASTTY_CONFIG_PATH` reaches the visible copied-app window, and the
terminal-output diagnostic proceeds past that prerequisite. Terminal text is
still not exposed through the current UI automation tree, so the next experiment
can focus on the actual terminal-output accessibility layer instead of
config/title setup.

## Completion Review

**Reviewer:** Codex-native adversarial subagent `Zeno` with fresh context, using
the `adversarial-review` skill's Codex path (`multi_agent_v1.spawn_agent`), not
Claude's named `adversarial-reviewer` agent.

**Verdict:** Approved.

The reviewer found no Required, Optional, or Nit findings. It independently
checked `git diff --check`, markdown formatting with `prettier --check`,
SwiftLint for the edited Swift files, the requested working-tree diff,
`git status --short`, and `git log -1 --oneline`. It also confirmed that the
latest commit was still the Experiment 159 plan commit, so the result commit had
not been made before completion review.
