+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5"
reasoning = "medium"
+++

# Experiment 699: Confirm Close Policy

## Description

Upstream Ghostty's embedded API exposes app and surface quit-confirm checks:
`ghostty_app_needs_confirm_quit` and `ghostty_surface_needs_confirm_quit`. The
surface implementation delegates to `Surface.needsConfirmQuit`, which:

- returns true for read-only surfaces;
- returns false after the child process has exited;
- otherwise follows `confirm-close-surface`:
  - `always` always confirms;
  - `false` never confirms;
  - `true` confirms only when the terminal cursor is not at a semantic prompt.

Roastty already has a parsed `ConfirmCloseSurface` enum in `config/mod.rs`,
surface `process_exited` state, app surface registration, runtime close-surface
callback forwarding, and semantic prompt/cursor state in the terminal. However,
the C ABI currently returns false unconditionally for app-level confirmation,
and the surface-level implementation is a stub. The local C-facing `Config`
skeleton also does not store the `confirm-close-surface` value yet.

This experiment wires the already-modeled pieces into the ABI:

- store `confirm-close-surface` on the local C-facing `Config` and `Surface`;
- copy it through app/surface config update entry points;
- add a terminal `cursor_is_at_prompt` predicate matching upstream's semantic
  prompt behavior;
- make app and surface quit-confirm APIs use the policy;
- pass the computed surface policy through `roastty_surface_request_close`.

This does not implement read-only surfaces, full config file loading, full
runtime config updates beyond this field, or any frontend confirmation UI.

## Changes

- `roastty/src/terminal/screen.rs`
  - Add `Screen::cursor_is_at_prompt`:
    - false on alternate-screen callers through the terminal wrapper;
    - true when the cursor row has a semantic prompt marker other than `None`;
    - true when cursor semantic content is `Prompt` or `Input`;
    - false for output or missing row metadata.

- `roastty/src/terminal/terminal.rs`
  - Add `Terminal::cursor_is_at_prompt` that returns false on the alternate
    screen and delegates to the active primary screen otherwise.
  - Add terminal tests based on upstream's `cursorIsAtPrompt` coverage,
    including prompt, input, output, new prompt, and alternate-screen cases.

- `roastty/src/lib.rs`
  - Extend the local C-facing `Config` skeleton with
    `confirm_close_surface: config::ConfirmCloseSurface`, defaulting to `True`
    and preserved by `roastty_config_clone`.
  - Extend `Surface` with the same field, initialized from the app config when a
    surface is created and updated by `roastty_surface_update_config`.
  - Store the app-level default confirm-close policy and update it through
    `roastty_app_update_config`.
  - Implement `Surface::needs_confirm_quit`:
    - return false for detached surfaces;
    - return false after `process_exited`;
    - return true for `Always`, including no-worker surfaces that are still
      attached and not marked exited;
    - return false for `False`;
    - return false without a worker for `True`, since there is no live terminal
      prompt state to inspect;
    - use `ConfirmCloseSurface::needs_confirm(at_prompt)` for live workers.
  - Implement `roastty_app_needs_confirm_quit` by scanning registered attached
    surfaces.
  - Keep `roastty_surface_request_close` using the same
    `Surface::needs_confirm_quit` result for the runtime close callback.

- Tests in `roastty/src/lib.rs`
  - Cover app-level aggregation across multiple surfaces.
  - Cover surface confirm policy for default `True`, `Always`, and `False`.
  - Cover no-worker `Always` returning true and no-worker `True` returning
    false.
  - Cover child-exited and detached-surface false results.
  - Cover prompt-aware behavior using OSC 133 semantic prompt sequences.
  - Cover `roastty_surface_request_close` passing the computed confirmation
    result to the runtime callback.
  - Cover app/surface config update copying the field and preserving unrelated
    behavior.

- `roastty/tests/abi_harness.c`
  - Extend smoke coverage for app/surface confirm APIs remaining callable
    through the header.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty cursor_is_at_prompt -- --nocapture`
- `cargo test -p roastty confirm_quit -- --nocapture`
- `cargo test -p roastty request_close -- --nocapture`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the amended Experiment 699 design and approved it with no
blocking findings. The review accepted the explicit no-worker semantics:
attached, non-exited `Always` surfaces still require confirmation, while `True`
without a worker returns false because there is no live terminal prompt state to
inspect. The review also accepted the scoped terminal prompt predicate, config
field propagation plan, app/surface policy wiring, request-close forwarding, and
test coverage.

## Result

**Result:** Pass

Implemented app and surface quit-confirm policy:

- Added `Terminal::cursor_is_at_prompt`, backed by `Screen` semantic prompt and
  cursor semantic-content state, with alternate screens returning false.
- Added `confirm_close_surface` to the local C-facing `Config` skeleton, cloned
  it, copied it into `App`, inherited it into new surfaces, and wired
  `roastty_app_update_config` / `roastty_surface_update_config` for this field.
- Implemented `Surface::needs_confirm_quit` with detached/exited false
  short-circuits, `Always` true for attached non-exited surfaces, `False` false,
  and `True` consulting live terminal prompt state.
- Implemented `roastty_app_needs_confirm_quit` by scanning registered attached
  surfaces.
- Kept `roastty_surface_request_close` forwarding the computed surface policy to
  the runtime close callback.

Verification passed:

- `cargo fmt -p roastty`
- `cargo test -p roastty cursor_is_at_prompt -- --nocapture`
- `cargo test -p roastty confirm_quit -- --nocapture`
- `cargo test -p roastty request_close -- --nocapture`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

Roastty now has the upstream-shaped quit-confirm decision path for the current
surface/app model. Remaining work is outside this slice: read-only surfaces,
full config loading/reload semantics, and frontend presentation of confirmation
UI.

## Completion Review

Codex reviewed the staged completed Experiment 699 result. The review found no
code correctness blockers and approved the implementation: detached/exited
surfaces return false, no-worker `Always` returns true, no-worker `True` returns
false, worker-backed `True` uses terminal prompt state, alternate screens are
not prompts, and app aggregation, config propagation, and request-close callback
propagation are covered by tests.

The review initially blocked the result commit only because completion-review
provenance had not yet been recorded. This section, the `[review.result]`
frontmatter, and the README tuple update resolve that workflow finding.
