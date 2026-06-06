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

# Experiment 706: Binding Action Reset

## Description

Experiments 702–705 added binding-action invocation for split actions,
`close_surface`, `text:`, `csi:`, and `esc:`. Upstream Ghostty's
`performBindingAction` also supports the parameterless `reset` action by
resetting the terminal state in-place:

- `reset` has no parameter;
- `reset:*` is malformed because `reset` is a void action;
- the action resets screen, modes, tab stops, title, PWD, DCS state, Kitty
  graphics state, flags, and related terminal state;
- the binding action is consumed and returns `true`.

Roastty already has `Terminal::reset` and a public `roastty_terminal_reset` ABI
for standalone terminal handles. Surface-backed terminals live inside the
`TermioWorker`, and tests already use `with_termio_mut` to mutate worker
terminal state. This experiment wires `reset` through
`roastty_surface_binding_action` for attached surfaces.

This does not implement `clear_screen`, scrolling actions, search actions,
clipboard actions, cursor-key actions, full keybind storage/lookup, or
app-scoped actions.

## Changes

- `roastty/src/lib.rs`
  - Extend the internal parsed binding-action enum with `Reset`.
  - Extend `parse_binding_action` to accept parameterless `reset` and reject
    `reset:*`.
  - Add/use a surface helper that locks the active termio worker and calls
    `Terminal::reset`.
  - Return `true` for attached parsed `reset` actions, even when no termio
    worker exists, matching action-consumed semantics.
  - Return `false` for null or detached surfaces.
  - Keep split, close, `text:`, `csi:`, and `esc:` semantics unchanged.

- `roastty/tests/abi_harness.c`
  - Add C ABI smoke coverage that `reset:now` is rejected and `reset` can be
    invoked through the public ABI.

- Tests in `roastty/src/lib.rs`
  - Cover `reset:now` returning false.
  - Cover `reset:` returning false, proving parameterless void action parsing
    rejects even empty colon parameters.
  - Cover null and detached surfaces returning false.
  - Cover attached no-worker surfaces returning true without side effects.
  - Cover reset clearing visible terminal text through a surface-backed worker.
  - Cover reset clearing terminal title and PWD metadata through a
    surface-backed worker.
  - Re-run existing binding-action tests to prove previous action semantics did
    not change.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty binding_action -- --nocapture`
- `cargo test -p roastty terminal_reset -- --nocapture`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the initial Experiment 706 design and approved the core reset
approach with one test fix. The review confirmed that upstream `reset` is a
parameterless void action, calls a terminal full reset, and returns `true` when
performed. It also confirmed that Roastty's `Terminal::reset` covers the
expected terminal state reset surface for this slice.

The required fix was adding explicit coverage that `reset:` returns false.
Upstream void-action parsing rejects any colon-bearing form, including an empty
parameter, and `reset:` is easy to regress because `text:`, `csi:`, and `esc:`
intentionally accept empty parameters. The review also required recording this
section and updating the README provenance tuple before the plan commit.

## Result

**Result:** Pass

Implemented parameterless `reset` binding-action support for attached surfaces.
`parse_binding_action` now accepts only the exact `reset` form and rejects
colon-bearing variants like `reset:` and `reset:now`. Dispatch returns `false`
for null or detached surfaces, returns `true` for attached surfaces, and resets
the worker-backed terminal through `Terminal::reset` when a worker exists.

The Rust tests cover malformed reset forms, null/detached surfaces, no-worker
attached surfaces, visible text clearing, title/PWD clearing, and unchanged
binding-action behavior around split, close, `text:`, `csi:`, and `esc:`. The C
ABI harness now rejects `reset:`/`reset:now` and accepts `reset`.

Verification:

- `cargo fmt -p roastty` passed.
- `cargo test -p roastty binding_action -- --nocapture` passed: 24 tests.
- `cargo test -p roastty terminal_reset -- --nocapture` passed, but matched 0
  tests because no current test name contains `terminal_reset`.
- `cargo test -p roastty reset -- --nocapture` passed: 83 tests.
- `cargo test -p roastty --test abi_harness` passed.
- `cargo fmt -p roastty -- --check` passed.
- `git diff --check` passed.

## Conclusion

The reset slice now matches the upstream void-action parsing contract and uses
Roastty's existing terminal reset path for surface-backed workers. The remaining
binding-action work can continue with the next upstream action family instead of
special-casing reset outside the shared parser/dispatcher path.

## Completion Review

Codex reviewed the completed Experiment 706 diff and found no code correctness
blockers. The review confirmed that the parser accepts only parameterless
`reset`, rejects colon-bearing reset forms, returns `false` for null/detached
surfaces, returns `true` for attached surfaces, and routes worker-backed
surfaces through `Terminal::reset`.

The only required fix was workflow provenance: replacing the pending result
review metadata, adding this completion-review note, and updating the README
provenance tuple to `Codex/Codex/Codex`.
