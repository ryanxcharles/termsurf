# Experiment 158: Phase G — terminal output UI oracle

## Description

Experiment 157 proved that XCTest can drive the copied app's native macOS
dead-key route through `SurfaceView_AppKit.keyDown`, AppKit marked text,
`insertText`, and committed-preedit delivery to libroastty. It remained Partial
because the focused UI test could not observe the committed `é` through the
terminal accessibility value or select-all/copy path.

This experiment targets that missing app-visible oracle. The goal is to make a
focused UI test reliably observe deterministic terminal output from the copied
app, then use that oracle to turn the dead-key route proof into an app-visible
commit proof if the oracle is strong enough.

The first diagnostic target is not dead keys. It is a deterministic startup
screen such as `printf ready; cat`, launched through the existing config
`command` field. The experiment must prove which layer is failing if `ready`
does not become visible:

1. The UI-test config file is not being loaded or the surface command is not
   inheriting the parsed config.
2. The child process writes output, but the terminal screen is not updated or
   pumped in the copied-app host.
3. The terminal screen updates, but `SurfaceView_AppKit.accessibilityValue()` /
   `roastty_surface_read_text` reads the wrong range or stale data.
4. XCTest queries the wrong accessibility element, even though the actual
   `SurfaceView` has the content.

If the deterministic oracle works, update
`RoasttyDeadKeyUITests.testDeadKeyCompositionCommitsText` so the final assertion
requires app-visible `é` instead of throwing `XCTSkip`. If the oracle reveals a
product bug or host limitation that cannot be fixed narrowly, record `Partial`
with that exact layer identified and keep the dead-key output gap explicit.

## Changes

- `roastty/macos/RoasttyUITests/RoasttyTerminalOutputUITests.swift`
  - Add a focused UI test using `RoasttyCustomConfigCase`.
  - Launch the copied app with a deterministic `command = printf ready; cat`
    config, or an equivalent command that writes stable ASCII output and stays
    alive for later input.
  - Prove the UI-test config path is active with an independent visible signal
    such as the configured window title, not only by expecting terminal output.
  - Poll the actual terminal text area/accessibility element, not only the
    SwiftUI wrapper group, and report a useful hierarchy/value snapshot on
    failure.
- `roastty/macos/RoasttyUITests/RoasttyDeadKeyUITests.swift`
  - If the output oracle is reliable, replace the current Partial `XCTSkip` path
    with a required app-visible `é` assertion after the full route trace.
  - If the oracle is not reliable, leave the dead-key test as a route-only
    Partial gate and document the exact blocker.
- `roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift`
  - Fix only narrowly identified product bugs in the accessibility/read-text
    path.
  - Any additional UI-test observation hook must be inert unless enabled by a
    launch environment variable and must not change normal app behavior.
- `roastty/src/lib.rs`
  - Touch only if the diagnostic proves `roastty_surface_read_text`,
    termio-pump, or command-start behavior is wrong in the embedded surface
    path.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Update the Phase G native-key checklist and Experiment 158 index line with
    `Pass` or `Partial`.

Out of scope:

- Broad shell-command configuration rewrites.
- Permission-dependent global shortcut installation.
- Changing the copied app's production UI hierarchy solely for test convenience.
- Replacing UI automation with unit-test-only screen snapshots.
- Claiming dead-key `Pass` from route trace alone.

## Verification

- Format markdown:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/158-terminal-output-ui-oracle.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Swift lint for edited Swift files:
  - `swiftlint lint roastty/macos/RoasttyUITests/RoasttyTerminalOutputUITests.swift`
  - Include `RoasttyDeadKeyUITests.swift`, `SurfaceView_AppKit.swift`, or any
    optional edited helper/source files in the same lint run.
- Rust formatting if Rust changes:
  - `cargo fmt`
- Full Rust library tests if `roastty/src/lib.rs` or other Rust library code
  changes:
  - `cargo test -p roastty`
- Default hosted app tests still skip UI by default:
  - `cd roastty && macos/build.nu --action test`
- Focused terminal-output UI gate:
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyTerminalOutputUITests`
  - The selector must report real `RoasttyTerminalOutputUITests` execution. A
    process success with `Executed 0 tests` is not acceptable.
  - If the experiment adds one test method, the class selector must report
    exactly 1 executed test. If it adds more, the exact expected count must be
    stated in the Result.
- Focused dead-key UI gate if its final assertion changes:
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyDeadKeyUITests`
  - If the experiment claims `Pass`, this selector must execute one real test
    with 0 skips and prove visible `é` plus the full route trace.
  - If the experiment remains `Partial`, the selector may still skip, but only
    after proving the full route trace through `committedPreeditText text=é`.
- Hygiene:
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/158-terminal-output-ui-oracle.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = a focused copied-app UI test proves deterministic terminal output is
observable through an app-visible accessibility/copy path, and the dead-key UI
test uses that oracle to prove a visible committed `é` with 0 skips.

**Partial** = the focused UI test identifies the failing layer in the output
oracle, but the fix is larger than this experiment or depends on a host
limitation; the dead-key UI test remains a route-only Partial gate.

**Fail** = the focused selector executes zero tests, bypasses the copied app's
terminal surface, relies only on unit-test snapshots, or claims dead-key success
without app-visible output.

## Design Review

**Reviewer:** Codex-native adversarial subagent `Gibbs` with fresh context,
using the `adversarial-review` skill's Codex path
(`multi_agent_v1.spawn_agent`), not Claude's named `adversarial-reviewer` agent.

**Initial verdict:** Changes required.

**Required findings:**

- The focused terminal-output UI gate did not require a concrete real test count
  beyond rejecting `Executed 0 tests`.
- The design allowed Rust library edits without requiring the full
  `cargo test -p roastty` gate that keeps the ABI harness compiling.

**Fixes:**

- The focused terminal-output selector must report exactly one executed test if
  this experiment adds one test method; if it adds more, the Result must state
  the exact expected count.
- Any Rust library change now requires `cargo test -p roastty` in addition to
  `cargo fmt`.

**Final verdict:** Approved. The re-review confirmed both Required findings are
resolved and found no new Required findings.

## Result

**Result:** Partial

The experiment added a focused copied-app UI selector for deterministic terminal
output and broadened the terminal-text helper used by the dead-key UI test:

- `RoasttyTerminalOutputUITests` launches the copied app through
  `RoasttyCustomConfigCase`, configures a deterministic
  `initial-command = direct:echo TERMSURF_READY_158`, and requires the
  configured window title to become visible before it can require
  `TERMSURF_READY_158` through the terminal accessibility text path.
- `RoasttyTerminalText` collects terminal text from the wrapper group and every
  accessible text view, and reports a useful per-text-view snapshot on failure.
- `RoasttyDeadKeyUITests` now uses the shared terminal-text helper and disables
  oh-my-zsh auto-update prompts for the launched test app before its existing
  route trace.

The focused terminal-output selector executed exactly one real test, but it
still skipped. The first failing layer is the independent config/title proof:
the launched copied app reported the default `👻` window title instead of the
configured `RoasttyTerminalOutputUITests` title, so the test cannot honestly
claim the configured initial command reached the first surface:

```text
Test skipped - Configured window title was not visible; actual title: 👻
Test Suite 'RoasttyTerminalOutputUITests' passed
Executed 1 test, with 1 test skipped and 0 failures
```

Earlier runs with typed input also exposed the host shell's oh-my-zsh update
prompt as a `TextView` query, proving XCTest can see a terminal text element in
some states, but not a deterministic command marker through the current helper.
Switching from typed `echo TERMSURF_READY_158` to the app config
`initial-command = direct:echo TERMSURF_READY_158` did not produce a visible
marker, but the missing configured title means Experiment 158 must stop at layer
1 from the design: the copied-app UI-test config path or the first surface's
inheritance from that config is not proven active in this selector.

The dead-key test therefore remains a route-only Partial gate. It still requires
the native route trace through `setMarkedText`, `insertText accumulated=é`, and
`committedPreeditText text=é` before it may skip the final app-visible output
check.

Verification run:

- `swiftlint lint roastty/macos/RoasttyUITests/RoasttyTerminalText.swift roastty/macos/RoasttyUITests/RoasttyTerminalOutputUITests.swift roastty/macos/RoasttyUITests/RoasttyDeadKeyUITests.swift`
  — pass, 0 violations.
- `git diff --check` — pass.
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/158-terminal-output-ui-oracle.md issues/0802-libroastty-completion-and-mac-app/README.md`
  — pass.
- `cd roastty && macos/build.nu --action test` — pass, 213 tests in 23 suites,
  UI skipped by default.
- `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyTerminalOutputUITests`
  — pass with exactly 1 executed test, 1 skip, 0 failures; the skip occurs at
  the configured-title proof.
- `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyDeadKeyUITests`
  — pass with exactly 1 executed test, 1 skip, 0 failures after proving the
  trace through `committedPreeditText text=é`.

Rust code was not edited, so `cargo fmt` and `cargo test -p roastty` were not
required.

## Conclusion

Experiment 158 improved the UI-test oracle plumbing but did not close the
app-visible terminal-output gap. The next experiment should target the copied
app config/initial-surface path directly: prove why the focused selector sees
the default `👻` title instead of `title = "RoasttyTerminalOutputUITests"`, then
prove whether `initial-command` reaches the first surface. Only after that
should it add a narrow product/test hook that exposes
`roastty_surface_read_text` for the active `SurfaceView` without relying on
XCTest's lossy text-view query behavior.

## Completion Review

**Reviewer:** Codex-native adversarial subagent `Rawls` with fresh context,
using the `adversarial-review` skill's Codex path
(`multi_agent_v1.spawn_agent`), not Claude's named `adversarial-reviewer` agent.

**Initial verdict:** Changes required.

**Required finding:**

- The completed test did not prove the UI-test config path was active with an
  independent visible signal, so the result overclaimed that the failure pointed
  past layer 1 from the design.

**Fixes:**

- `RoasttyTerminalOutputUITests` now checks the configured window title before
  waiting for terminal output and skips with the actual title when it is not
  visible.
- The result now records the first failing layer as the config/title proof,
  includes the exact `actual title: 👻` skip evidence, and says the config path
  or first-surface inheritance is not proven active.

**Final verdict:** Approved. The re-review confirmed the Required finding is
resolved and found no new Required findings.
