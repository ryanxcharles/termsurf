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

# Experiment 726: Binding Action Toggle Readonly

## Description

Experiment 725 completed upstream's surface-local `toggle_mouse_reporting`
binding action. The next compact surface-local binding-action gap is
`toggle_readonly`.

Upstream Ghostty keeps a `readonly: bool` on each surface. The `toggle_readonly`
binding action flips that flag, emits a surface-targeted runtime `readonly`
notification with `.on` or `.off`, and returns `true`. Readonly mode prevents
input writes from reaching the PTY while still allowing terminal-level UI
operations such as selection, copy, paste requests, scrolling, and other
non-input binding actions.

Roastty does not currently have readonly surface state or a readonly runtime
action tag. This experiment adds the smallest useful parity slice: the
surface-local flag, the parameterless binding action, readonly on/off runtime
notification, and gating for PTY input write paths already exposed by Roastty.

## Changes

- `roastty/include/roastty.h`
  - Add `ROASTTY_ACTION_READONLY = 63`, matching upstream
    `apprt.Action.Key.readonly`.
  - Add `roastty_readonly_e` with `ROASTTY_READONLY_ON = 0` and
    `ROASTTY_READONLY_OFF = 1`.
  - Document that `ROASTTY_ACTION_READONLY` stores the readonly enum in
    `storage[0]`.

- `roastty/src/lib.rs`
  - Add matching Rust constants for `ROASTTY_ACTION_READONLY`,
    `ROASTTY_READONLY_ON`, and `ROASTTY_READONLY_OFF`.
  - Add `readonly: bool` to `Surface`, initialized to `false`.
  - Extend `parse_binding_action` to accept `toggle_readonly` with no parameter
    and reject empty-colon or non-empty parameters.
  - Add local binding-action handling that toggles `Surface::readonly`, sends
    `ROASTTY_ACTION_READONLY` with `storage[0]` set to on/off via
    `perform_action_result`, and returns `true` after the local toggle. The
    runtime notification is best-effort, matching upstream's local-state action:
    missing or false callbacks do not roll back the readonly flag and do not
    make the binding unconsumed.
  - Return `false` for null or detached surfaces before toggling.
  - Suppress PTY input writes while readonly is active. Add a small internal
    queue-write helper or equivalent low-level gate so all current surface PTY
    write sites honor readonly:
    - initial-input writes remain unaffected because surfaces start with
      `readonly = false`;
    - paste/text writes from `Surface::text`;
    - raw writes from `Surface::raw_text`, including `text`, `csi`, and `esc`
      binding actions;
    - clear-screen form-feed writes while still allowing the local clear-screen
      operation to complete;
    - mouse report writes;
    - encoded key writes from `Surface::key`.
  - Keep non-input binding actions, selections, scroll actions, and clipboard
    read requests unchanged. Clipboard paste request actions may start while
    readonly is enabled, but any completed paste text is dropped at the
    low-level write gate.

- `roastty/tests/abi_harness.c`
  - Assert the new ABI constants and enum values.
  - Add malformed `toggle_readonly` rejection checks.
  - Add valid `toggle_readonly` coverage that returns `true` even when the
    runtime action callback is absent.

- Tests in `roastty/src/lib.rs`
  - Cover parser false paths for `toggle_readonly:` and `toggle_readonly:now`.
  - Cover null and detached cases returning `false`.
  - Cover missing/false callback cases still returning `true` after the local
    toggle.
  - Cover toggling readonly on/off and readonly notification storage values.
  - Cover key, text, raw text, and clear-screen form-feed writes being
    suppressed while readonly is active.
  - Cover clipboard paste requests still starting while completed paste writes
    are suppressed.
  - Cover mouse report writes being suppressed while readonly is active while
    preserving stored mouse state.
  - Re-run existing binding-action, key/text, mouse, and ABI harness tests.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty toggle_readonly -- --nocapture --test-threads=1`
- `cargo test -p roastty readonly -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty mouse -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the initial Experiment 726 design and found three blockers:

- The plan incorrectly made `toggle_readonly` return the runtime callback result
  even though upstream flips local state and treats notification as secondary.
- The write-gating scope was ambiguous and omitted paste/completion and other
  low-level PTY write paths.
- The design review result and `[review.design]` frontmatter were not recorded.

The design now makes readonly toggling a local action that returns `true` after
state mutation, treats the readonly runtime action as best-effort notification,
and gates every current surface PTY write site through a low-level readonly
check while leaving non-input terminal operations and clipboard read requests
available.
