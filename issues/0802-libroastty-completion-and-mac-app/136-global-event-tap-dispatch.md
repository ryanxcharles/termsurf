# Experiment 136: Phase G — global event tap dispatch

## Description

Validate the native global-key event path that is already present in the copied
macOS app.

Earlier Phase G work wired configured `global:` bindings into Roastty's app-key
dispatcher, and the copied app already has a `GlobalEventTap` that enables a
session event tap when `roastty_app_has_global_keybinds` is true. The remaining
gap is that the captured-event callback is private and untested: no hosted test
proves that a `CGEvent` captured while the app is inactive is converted to an
`NSEvent`, sent through `roastty_app_key`, and suppressed when a configured
global binding handles it.

This experiment adds a narrow testable dispatch seam around the existing event
tap callback. It does not attempt to install a live `CGEventTap` in tests, since
that depends on Accessibility permissions and can be flaky in CI/local
automation.

## Changes

- `roastty/macos/Sources/Features/Global Keybinds/GlobalEventTap.swift`
  - Extract the keydown dispatch body into an internal helper that accepts the
    event type, `CGEvent`, app-active state, and optional `roastty_app_t`.
  - Keep `cgEventFlagsChangedHandler` behavior unchanged: disabled taps are
    re-enabled, non-keydown events pass through, active-app events pass through,
    missing app/delegate/NSEvent pass through, and handled global bindings
    suppress the event by returning `nil`.
  - Preserve the existing event-tap creation, retry timer, and permission
    behavior.
- `roastty/macos/Tests/Roastty/GlobalEventTapTests.swift`
  - Add hosted tests that create temporary Roastty configs and raw
    `roastty_app_t` values without installing a real event tap.
  - Construct `CGEvent` keyboard events for macOS virtual keycode `0` (`KeyA`).
  - Prove an inactive app with `keybind = global:a=ignore` is handled and would
    be suppressed by the event tap callback.
  - Prove the same global binding is not handled while the app is active.
  - Prove a non-global `keybind = a=ignore` is not handled through the global
    tap path while inactive.
  - Prove non-keydown event types pass through.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After implementation, narrow the Phase G global-shortcut notes to say the
    callback dispatch path is hosted-test validated, while permission-dependent
    live tap installation remains outside automated tests unless a later
    experiment adds a stable harness for it.

Out of scope:

- Installing a real `CGEventTap` during tests or requiring Accessibility
  permission.
- Changing when `AppDelegate` enables or disables the shared event tap.
- Changing `roastty_app_key` semantics.
- Supporting `global:` trigger sequences, which the parser still rejects.
- Full Rust-side `KeymapDarwin` text translation or dead-key/preedit handling.

## Verification

- Run formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/136-global-event-tap-dispatch.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Run targeted Rust tests that cover app-key global dispatch:
  - `cargo test -p roastty app_key_global`
  - `cargo test -p roastty app_has_global_keybinds`
- Run the targeted macOS hosted test:
  - `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/GlobalEventTapTests`
- Run broader macOS coverage:
  - `cd roastty && macos/build.nu --action test`
- Run full Roastty tests:
  - `cargo test -p roastty -- --test-threads=1`
- Run checks:
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/136-global-event-tap-dispatch.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = the hosted test proves the event tap dispatch helper consumes an
inactive-app `CGEvent` only when a configured `global:` binding handles it, does
not consume active-app/non-global/non-keydown events, and the existing Rust
global app-key tests still pass.

**Partial** = the dispatch seam works but hosted macOS tests cannot construct a
stable `CGEvent`/`NSEvent` pair for the keybinding path.

**Fail** = the global event tap callback cannot be tested without installing a
real event tap or changing runtime semantics.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Sartre`, fresh context.

**Verdict:** Approved.

**Findings:** None.

The reviewer confirmed the README links Experiment 136 as `Designed`, the
experiment has the required sections, the scope is narrow and does not overclaim
live Accessibility-permission or event-tap installation validation, and the
formatting/diff checks passed.
