# Experiment 161: Phase G — committed preedit UTF-8

## Description

Experiments 157, 159, and 160 narrowed the dead-key blocker. The copied app can
synthesize the native `Option-E`, `E` dead-key sequence; AppKit produces marked
text and then commits `é`; the copied app reaches `committedPreeditText text=é`;
and the terminal accessibility oracle can now observe deterministic startup
output from the real copied-app terminal surface.

The remaining blocker is therefore not XCTest discovery and not the app-level
`initial-command` path. The committed preedit text reaches the Swift
`committedPreeditTextAction` path, but the app-visible terminal output still
exposes a replacement character instead of visible `é`.

This experiment diagnoses and fixes that handoff without weakening the copied
app route. It must distinguish these layers:

1. Swift passes the committed `é` to `roastty_surface_key` with the wrong bytes,
   length, keycode, modifiers, or composing state.
2. Rust `input_key_to_event` receives valid UTF-8 but keybinding/default-key
   dispatch encodes something other than the committed text.
3. The PTY receives valid UTF-8, but the spawned shell/input environment echoes
   it back as replacement text.
4. The terminal parser receives valid UTF-8 from the PTY but stores or exposes a
   replacement character through screen text/accessibility.
5. The terminal contains visible `é`, but the accessibility/copy oracle reads a
   stale or lossy value.

Experiment 139 already proved the raw Rust by-value `roastty_surface_key` path
can deliver multi-byte `é` to a child PTY in a focused unit test. This
experiment must therefore use live copied-app evidence, or a unit test that
specifically reproduces the copied-app committed-preedit event shape, before
changing product behavior.

## Changes

- `roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift`
  - Extend the existing `ROASTTY_UI_KEY_TRACE_PATH` hook, only if needed, to
    record bounded byte-level evidence for the committed preedit handoff:
    committed text, UTF-8 bytes, keycode, modifier masks, consumed modifier
    masks, composing state, and whether the event went through
    `committedPreeditTextAction` or direct `sendText`.
  - Keep the hook inert unless the UI-test environment variable is present.
  - Do not replace the native `typeKey` route with `typeText`, paste,
    `sendText`, or direct `setMarkedText`.
- `roastty/src/lib.rs`
  - Add a narrow test or diagnostic only if the Swift trace is insufficient to
    identify the failing layer.
  - If a Rust product bug is proven, fix the smallest responsible layer:
    `input_key_to_event`, key encoding/dispatch, PTY queueing, terminal UTF-8
    decoding, or screen-text extraction.
  - Preserve the existing Experiment 139 guarantee that by-value
    `roastty_surface_key` accepts app-provided UTF-8 text.
- `roastty/macos/RoasttyUITests/RoasttyDeadKeyUITests.swift`
  - Strengthen the focused UI test only after the trace identifies the failing
    layer.
  - If the fix makes app-visible output reliable, replace the final `XCTSkip`
    with a required assertion that the copied app exposes visible `é` through
    terminal accessibility or select-all/copy.
  - If the trace proves a narrower blocker remains, keep the skip but make its
    message name the exact failing layer and evidence.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - Update the Phase G native-key note after the result.

Out of scope:

- Bypassing `SurfaceView_AppKit.keyDown` / `interpretKeyEvents`.
- Claiming dead-key success from route trace alone.
- Broad IME matrix coverage beyond the current deterministic dead-key case.
- Permission-dependent global shortcut installation.
- Rewriting the copied app's input architecture.
- Making UI tests run by default.

## Verification

- Format markdown:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/161-committed-preedit-utf8.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Swift lint if Swift changes:
  - `swiftlint lint roastty/macos/RoasttyUITests/RoasttyDeadKeyUITests.swift 'roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift'`
- Rust formatting and focused tests if Rust changes:
  - `cargo fmt`
  - `cargo test -p roastty surface_key_by_value_utf8_reaches_child_pty`
  - Run any new focused Rust test added by this experiment.
- Focused dead-key UI gate:
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyDeadKeyUITests`
  - Must report exactly 1 executed test. `Executed 0 tests` is a failure.
  - If this experiment claims `Pass`, the selector must have 0 skips and prove
    visible `é` plus the full copied-app route: `setMarkedText string=´`,
    `insertText accumulated=é`, and `committedPreeditText text=é`.
  - If this experiment remains `Partial`, the selector may skip only after
    proving the copied-app route and recording the exact byte/PTY/terminal layer
    that still blocks visible `é`.
- Terminal-output regression gate:
  - `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyTerminalOutputUITests`
  - Must still execute exactly 1 test with 0 skips and find
    `TERMSURF_READY_158`.
- Default hosted app tests still skip UI by default:
  - `cd roastty && macos/build.nu --action test`
- Full Rust suite:
  - `cargo test -p roastty`
  - Record the exact result. If unrelated suite instability recurs, do not
    overclaim; include focused passing tests for this experiment and the
    reproduced unrelated failures.
- Hygiene:
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/161-committed-preedit-utf8.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = the focused copied-app dead-key UI selector executes exactly 1 test
with 0 skips and 0 failures, proves the full native route, and observes visible
`é` through the real terminal accessibility/copy path without bypassing
`keyDown` / `interpretKeyEvents`.

**Partial** = the experiment preserves the live copied-app route and narrows the
remaining failure to a specific Swift byte handoff, Rust key encoding, PTY echo,
terminal UTF-8 decoding, or accessibility/copy layer with concrete evidence.

**Fail** = the selector executes zero tests, the result bypasses the copied app
native key path, weakens the route assertions, claims success from trace alone,
or changes unrelated input architecture without evidence.

## Result

**Result:** Pass

The failing layer was not Swift's UTF-8 handoff and not Rust's by-value
`roastty_surface_key` conversion. The committed preedit path was already passing
valid `é` text into the same by-value key event shape that libroastty can
deliver to a child PTY.

This experiment made the focused dead-key UI test deterministic by running the
copied app surface against a UTF-8 echo command:

```text
stty -echo -icanon min 2 time 0; dd bs=2 count=1 2>/dev/null; sleep 1
```

That command removes the interactive shell's locale, prompt, and echo behavior
from the final oracle while preserving the real copied app, native AppKit
`typeKey` route, `keyDown` / `interpretKeyEvents`, marked-text commit, Swift
`committedPreeditTextAction`, by-value `roastty_surface_key`, PTY write, and
terminal accessibility observation.

The UI trace hook now records bounded byte-level detail when
`ROASTTY_UI_KEY_TRACE_PATH` is present, including UTF-8 bytes, Unicode scalars,
keycode, modifier masks, consumed modifier masks, composing state, and the
committed-preedit path. It remains inert outside UI tests.

The Rust regression
`surface_key_by_value_committed_preedit_shape_reaches_child_pty` now covers the
exact committed-preedit shape Swift sends: `keycode = 0`,
`unshifted_codepoint = 0`, no modifiers, `composing = false`, and text `é`.

Verification:

- `swiftlint lint roastty/macos/RoasttyUITests/RoasttyDeadKeyUITests.swift 'roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift'`
  — 0 violations, 0 serious.
- `cargo fmt -p roastty` — pass.
- `cargo test -p roastty surface_key_by_value -- --nocapture` — pass: 2 passed,
  including the new committed-preedit shape regression and the existing by-value
  UTF-8 regression.
- `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyDeadKeyUITests`
  — pass: exactly 1 executed test, 0 skips, 0 failures. The test observed
  visible `é` in the copied app terminal.
- `cd roastty && macos/build.nu --action test --ui-tests --only-testing RoasttyUITests/RoasttyTerminalOutputUITests`
  — pass: exactly 1 executed test, 0 skips, 0 failures. The test observed
  `TERMSURF_READY_158`.
- `cd roastty && macos/build.nu --action test` — pass. UI tests remain skipped
  by default; hosted app tests passed: 213 tests in 23 suites.
- `cargo test -p roastty` — unrelated failure: 4,848 passed, 2 failed, 4
  ignored. The failures were
  `os::cf_release_thread::tests::pool_flush_releases_on_worker_thread` and
  `os::cf_release_thread::tests::worker_drop_drains_queued_refs`. A focused
  rerun of `cargo test -p roastty os::cf_release_thread::tests -- --nocapture`
  reproduced persistent release-thread state failures in that module, including
  `empty_pool_is_noop`; this is outside the touched committed-preedit,
  key-event, PTY, and UI-test paths.
- `git diff --check` — pass.

## Conclusion

The native macOS dead-key path is now proven end to end in the copied app for
the deterministic UTF-8 terminal case. `Option-E`, `E` produces AppKit marked
text, commits `é`, sends the committed text through `committedPreeditTextAction`
and by-value `roastty_surface_key`, reaches the PTY, and becomes visible through
the real terminal accessibility oracle.

The previous replacement-character observation depended on an interactive
shell/echo environment as the final oracle. The product input path and terminal
UTF-8 path are covered by focused UI and Rust tests; broader shell locale
behavior is not part of this experiment's pass condition.

## Completion Review

**Reviewer:** Codex-native adversarial subagent `Feynman` with fresh context,
using the `adversarial-review` skill's Codex path
(`multi_agent_v1.spawn_agent`), not Claude's named `adversarial-reviewer` agent.

**Verdict:** Approved after fixes.

The reviewer found two Required findings:

1. The Pass-path visible-`é` oracle still used `XCTSkip`, which would have let a
   future visible-text regression report as skipped instead of failed.
2. The experiment file had not yet recorded the completion review.

Both findings were fixed before the result commit. The final visible-text oracle
now fails hard with the same trace and terminal snapshot evidence, and this
section records the completion review outcome. The reviewer also confirmed that
the native AppKit route is preserved and that the Rust regression matches the
committed-preedit event shape.

## Design Review

**Reviewer:** Codex-native adversarial subagent `Cicero` with fresh context,
using the `adversarial-review` skill's Codex path
(`multi_agent_v1.spawn_agent`), not Claude's named `adversarial-reviewer` agent.

**Verdict:** Approved.

The reviewer found no Required findings. It verified that the README links
Experiment 161 as `Designed`, the experiment has the required sections and
concrete Pass/Partial/Fail criteria, the scope follows the Experiment 160
blocker, the design preserves the copied-app native `typeKey` route, and the
diagnostics cover Swift bytes/event shape, Rust conversion/encoding, PTY and
terminal layers, and accessibility/copy observation. It also ran
`git diff --check` and the Prettier check for the Experiment 161 design and
README.
