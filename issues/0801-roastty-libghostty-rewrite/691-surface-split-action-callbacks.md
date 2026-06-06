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

# Experiment 691: Surface Split Action Callbacks

## Description

Experiment 690 added the state-only surface key callback foundation. Another
remaining surface ABI group is split requests: `ghostty_surface_split`,
`ghostty_surface_split_focus`, `ghostty_surface_split_resize`, and
`ghostty_surface_split_equalize`.

Upstream implements these by asking the app runtime to perform split actions.
Roastty already has a generic `roastty_runtime_action_cb(app, target, action)`
callback and C ABI `roastty_target_s` / `roastty_action_s` structs, but the
surface split functions and split action tags do not exist yet. This experiment
adds the split action ABI and forwards split requests through the existing
runtime action callback.

This does not implement a split tree inside Roastty, create new surfaces, change
focus, or resize any frontend panes. The embedding frontend remains responsible
for interpreting the action callback and performing the actual split operation.

## Changes

- `roastty/include/roastty.h`
  - Add split action tags for `roastty_action_s.tag`:
    - `ROASTTY_ACTION_NEW_SPLIT = 4`
    - `ROASTTY_ACTION_GOTO_SPLIT = 16`
    - `ROASTTY_ACTION_RESIZE_SPLIT = 18`
    - `ROASTTY_ACTION_EQUALIZE_SPLITS = 19`
  - Add upstream-compatible split enums:
    - `roastty_split_direction_e`: `RIGHT = 0`, `DOWN = 1`, `LEFT = 2`, `UP = 3`
    - `roastty_goto_split_e`: `PREVIOUS = 0`, `NEXT = 1`, `UP = 2`, `LEFT = 3`,
      `DOWN = 4`, `RIGHT = 5`
    - `roastty_resize_split_e`: `UP = 0`, `DOWN = 1`, `LEFT = 2`, `RIGHT = 3`
  - Add surface split functions near `roastty_surface_request_close`:
    - `ROASTTY_API void roastty_surface_split(roastty_surface_t, roastty_split_direction_e);`
    - `ROASTTY_API void roastty_surface_split_focus(roastty_surface_t, roastty_goto_split_e);`
    - `ROASTTY_API void roastty_surface_split_resize(roastty_surface_t, roastty_resize_split_e, uint16_t);`
    - `ROASTTY_API void roastty_surface_split_equalize(roastty_surface_t);`
- `roastty/src/lib.rs`
  - Add Rust constants matching the public split action tags and enum values.
  - Add validation helpers for split, goto, and resize enum values.
  - Add `Surface::perform_action(tag, storage)` that calls
    `app.runtime.action_cb` with target
    `{ tag = ROASTTY_TARGET_SURFACE, surface = self }` when the surface is
    attached and the callback exists.
  - Implement each surface split function as a safe no-op for null surfaces,
    detached surfaces, invalid enum values, and apps without `action_cb`.
  - Store payloads in `roastty_action_s.storage`:
    - new split: `storage[0] = direction`
    - goto split: `storage[0] = direction`
    - resize split: `storage[0] = amount`, `storage[1] = direction`, matching
      upstream's `{ amount, direction }` payload order
    - equalize splits: no payload
- `roastty/tests/abi_harness.c`
  - Assert action tag and split enum values.
  - Exercise null and live surface split calls through `roastty.h`.
- Tests
  - Constants match upstream enum values.
  - Null, detached, invalid enum, and no-callback surfaces are safe no-ops.
  - Each valid split function invokes `action_cb` with the expected target,
    action tag, and payload.
  - Resize callback payload order is asserted as amount first, direction second.
  - The callback return value is ignored, matching the upstream fire-and-report
    shape.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/691-surface-split-action-callbacks.md`
- `cargo fmt -p roastty`
- `cargo test -p roastty surface_split`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

**Result:** Approved after ABI fixes.

Codex initially blocked the design on two ABI fidelity issues. First, the split
action tags must use upstream's full action enum values rather than a local
dense range, so the design now uses `NEW_SPLIT = 4`, `GOTO_SPLIT = 16`,
`RESIZE_SPLIT = 18`, and `EQUALIZE_SPLITS = 19`.

Second, resize payload storage must match upstream's `{ amount, direction }`
layout. The design now stores `amount` in `storage[0]` and `direction` in
`storage[1]`, and the tests explicitly assert that order.

Codex approved forwarding the split requests through Roastty's existing runtime
`action_cb` as the right boundary for this slice, with split-tree and frontend
pane mutations left to the embedding frontend.

## Result

**Result:** Pass.

Roastty now exposes split action tags, upstream-compatible split direction
enums, and the four surface split request functions: `roastty_surface_split`,
`roastty_surface_split_focus`, `roastty_surface_split_resize`, and
`roastty_surface_split_equalize`.

The implementation validates enum values and forwards valid attached-surface
requests through the existing runtime `action_cb` with target
`ROASTTY_TARGET_SURFACE`. Payloads use the approved storage layout: new split
and goto split store direction in `storage[0]`, resize split stores amount in
`storage[0]` and direction in `storage[1]`, and equalize splits has no payload.
Null surfaces, detached surfaces, invalid enum values, and apps without
`action_cb` are safe no-ops. The callback return value is ignored.

Verification passed:

- `cargo fmt -p roastty`
- `cargo test -p roastty surface_split -- --nocapture`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

Surface split requests now cross the C ABI and reach the embedding runtime as
explicit actions. Roastty still does not own split-tree/frontend pane mutation;
frontends must interpret the action callback and perform the actual split,
focus, resize, or equalize operation.

## Completion Review

Codex reviewed the staged implementation and result. It found no correctness
blockers: the ABI forwards through `action_cb`, uses the upstream tag values and
approved resize payload order, treats null/detached/invalid/no-callback cases as
no-ops, and ignores the callback return value.
