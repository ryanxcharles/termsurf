+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5"
reasoning = "medium"
+++

# Experiment 729: Binding Action App Runtime Forwarding

## Description

Experiment 728 completed the surface-triggered `undo` / `redo` forwarding
special cases. The next upstream binding-action gap is the simple app-scoped
runtime actions.

Upstream Ghostty classifies these actions as app-scoped and, when a binding is
triggered from a surface, forwards them to the app runtime rather than the
surface runtime target:

- `quit`
- `close_all_windows`
- `toggle_quick_terminal`
- `toggle_visibility`
- `show_gtk_inspector`
- `open_config`
- `reload_config`
- `check_for_updates`

Roastty already exposes a runtime action callback with a target tag, but current
binding-action forwarding always uses `ROASTTY_TARGET_SURFACE`. This experiment
adds the app target path and parser support for the zero-storage app-scoped
actions above.

`new_window` is intentionally out of scope: upstream special-cases it by calling
`app.newWindow(..., .{ .parent = self })` rather than forwarding a zero-storage
runtime action. That parent-surface creation behavior needs its own experiment.

## Changes

- `roastty/include/roastty.h`
  - Explicitly document the callback target shape for app-scoped forwarded
    actions: `target.tag = ROASTTY_TARGET_APP` (`0`) and
    `target.surface = NULL`.
  - Add upstream-aligned action tags:
    - `ROASTTY_ACTION_QUIT = 0`
    - `ROASTTY_ACTION_CLOSE_ALL_WINDOWS = 5`
    - `ROASTTY_ACTION_TOGGLE_QUICK_TERMINAL = 10`
    - `ROASTTY_ACTION_TOGGLE_VISIBILITY = 12`
    - `ROASTTY_ACTION_SHOW_GTK_INSPECTOR = 29`
    - `ROASTTY_ACTION_OPEN_CONFIG = 40`
    - `ROASTTY_ACTION_RELOAD_CONFIG = 47`
    - `ROASTTY_ACTION_CHECK_FOR_UPDATES = 53`
  - Document that all of these actions have zeroed storage.

- `roastty/src/lib.rs`
  - Add matching Rust action constants and `ROASTTY_TARGET_APP = 0`.
  - Add an app-target action forwarding helper that calls the existing runtime
    action callback with `target.tag = ROASTTY_TARGET_APP` and a null surface.
  - Add a parsed binding-action variant, or equivalent handling, for app-target
    runtime actions.
  - Extend `parse_binding_action` to accept the eight actions above with no
    parameter and reject empty-colon or non-empty parameters.
  - Return `false` for null surfaces, detached surfaces, missing callbacks, and
    false callbacks.
  - Preserve surface-target forwarding for all existing runtime actions.

- `roastty/tests/abi_harness.c`
  - Assert `ROASTTY_TARGET_APP == 0`.
  - Assert the new ABI action tags.
  - Add malformed app action rejection checks.
  - Add valid no-callback coverage returning `false`.

- Tests in `roastty/src/lib.rs`
  - Cover parser false paths for empty-colon and non-empty parameters for each
    new app-scoped action.
  - Cover null, detached, and missing-callback cases returning `false`.
  - Cover forwarding to the action callback with app target, null surface, each
    action tag, and zeroed storage.
  - Cover callback result propagation.
  - Cover that existing surface-targeted runtime actions still use
    `ROASTTY_TARGET_SURFACE`.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty app_runtime -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the initial Experiment 729 design and found one technical
blocker: the plan treated `ROASTTY_TARGET_APP` as an internal Rust constant even
though the target tag is callback ABI surface. The design now explicitly
requires header documentation, ABI harness assertion of
`ROASTTY_TARGET_APP == 0`, and app callback target shape
`target.tag = ROASTTY_TARGET_APP` with `target.surface = NULL`.

The review found no other technical blockers. It approved the action set,
excluded `new_window` scope, zero-storage parser plan, app-target forwarding
tests, callback-result tests, and regression coverage for existing
surface-targeted actions.

The review also found one workflow blocker: this design-review section still
said `Pending.` This section now records the review outcome, and the README
tuple is `Codex/Codex/-`.
