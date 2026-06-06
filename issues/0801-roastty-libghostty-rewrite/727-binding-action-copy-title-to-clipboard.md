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

# Experiment 727: Binding Action Copy Title To Clipboard

## Description

Experiment 726 completed the surface-local `toggle_readonly` binding action. The
next compact upstream binding-action gap is `copy_title_to_clipboard`.

Upstream Ghostty treats `copy_title_to_clipboard` as a surface-scoped binding
action that forwards to the runtime app with `copy_title_to_clipboard` and no
payload. The runtime decides how to copy the effective title, using the
user-overridden title when present and otherwise the terminal title.

Roastty already has title state and runtime action forwarding for related title
actions, but it does not expose the `copy_title_to_clipboard` runtime action tag
or parse the binding action yet. This experiment adds the narrow forwarding path
only; it does not implement local clipboard copying in Roastty.

## Changes

- `roastty/include/roastty.h`
  - Add `ROASTTY_ACTION_COPY_TITLE_TO_CLIPBOARD = 64`, matching upstream
    `apprt.Action.Key.copy_title_to_clipboard`.
  - Document that the action has zeroed storage.

- `roastty/src/lib.rs`
  - Add the matching Rust action constant.
  - Extend `parse_binding_action` to accept `copy_title_to_clipboard` with no
    parameter and reject empty-colon or non-empty parameters.
  - Forward the action through the existing `RuntimeAction` path with zeroed
    storage.
  - Preserve false-path behavior for null surfaces, detached surfaces, and
    missing runtime action callbacks.

- `roastty/tests/abi_harness.c`
  - Assert the new ABI action tag.
  - Add malformed `copy_title_to_clipboard` rejection checks.
  - Add valid no-callback coverage returning `false`.

- Tests in `roastty/src/lib.rs`
  - Cover parser false paths for `copy_title_to_clipboard:` and
    `copy_title_to_clipboard:now`.
  - Cover null, detached, and missing-callback cases returning `false`.
  - Cover forwarding to the action callback with target surface, action tag 64,
    and zeroed storage.
  - Cover callback result propagation.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty copy_title_to_clipboard -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 727 design and found no technical blockers. The
review approved the surface-targeted runtime forwarding scope, upstream-aligned
action tag 64, zeroed storage, strict no-parameter parsing, callback false
paths, forwarding tests, C ABI harness coverage, and verification steps.

The review found one workflow blocker: this design-review section still said
`Pending.` This section now records the review outcome, and the README tuple is
`Codex/Codex/-`.
