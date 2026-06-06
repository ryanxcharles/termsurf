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

# Experiment 702: Surface Binding Action ABI Foundation

## Description

Upstream Ghostty exposes an embedded C ABI entry point for invoking a parsed
binding action string on a surface:

- `ghostty_surface_binding_action(surface, action_ptr, action_len)`.

The upstream implementation parses strings with `input.Binding.Action.parse`,
then calls `Surface.performBindingAction`. The action grammar is the same
configuration action grammar used by keybinds. Examples relevant to Roastty's
current ABI are:

- `new_split` and `new_split:right`;
- `goto_split:next`, `goto_split:left`, and the compatibility aliases
  `goto_split:top` / `goto_split:bottom`;
- `resize_split:up,10`;
- `equalize_splits`.

Roastty already has C ABI action tags and runtime callback forwarding for split
actions, but it does not expose `roastty_surface_binding_action`. That leaves a
gap for frontends that want to invoke a config-style binding action string
without calling the split-specific helper functions directly.

This experiment adds a narrow binding-action ABI foundation:

- expose
  `roastty_surface_binding_action(roastty_surface_t, const char*, uintptr_t)`;
- parse only the split actions Roastty can already forward through its runtime
  action callback;
- map upstream's `new_split:auto` default with Roastty's stored surface pixel
  size, choosing `right` when width is greater than height and `down` otherwise;
- reject unsupported, malformed, null, detached, or callback-less calls with
  `false` and no side effects.

This does not implement full `Binding.Action` parsing, keybind storage,
keybinding lookup, app-scoped actions, terminal text/CSI/ESC actions, clipboard
actions, close/tab/window actions, command palette actions, key tables, or real
split-tree mutation in the frontend.

## Changes

- `roastty/include/roastty.h`
  - Add
    `roastty_surface_binding_action(roastty_surface_t, const char*, uintptr_t)`.

- `roastty/src/lib.rs`
  - Add a small parser for the currently supported binding-action strings:
    - `new_split[:right|down|left|up|auto]`;
    - `goto_split:previous|next|up|left|down|right|top|bottom`;
    - `resize_split:up|down|left|right,<u16>`;
    - `equalize_splits`.
  - Add a bool-returning action-forwarding helper that shares the existing
    `Surface::perform_action` callback path but preserves the callback return
    value for `roastty_surface_binding_action`.
  - Keep the split-specific helper functions as `void` ABI wrappers that ignore
    the callback result as they do today.
  - Forward parsed actions through the bool-returning helper and return that
    callback result.
  - Return `false` without forwarding for null surfaces, null action pointers
    with nonzero lengths, invalid UTF-8, empty actions, unsupported action
    names, malformed parameters, detached surfaces, and surfaces whose app has
    no action callback.
  - Treat a null action pointer with length zero as an empty action and return
    `false`.
  - Map `new_split` and `new_split:auto` using `Surface.size.width_px` and
    `Surface.size.height_px`, choosing right when width is greater than height
    and down otherwise. This intentionally makes zero-size default surfaces pick
    down, matching upstream's `width > height` branch condition.

- `roastty/tests/abi_harness.c`
  - Add compile/link smoke coverage for the new symbol on null and default
    surfaces.

- Tests in `roastty/src/lib.rs`
  - Cover null/default/detached/callback-less false paths with no action
    records.
  - Cover unsupported and malformed action strings returning false.
  - Cover each supported split action family forwarding the expected action tag
    and storage values.
  - Cover `new_split` / `new_split:auto` choosing right for wide surfaces, down
    for tall surfaces, and down for default zero-size surfaces.
  - Cover callback false results being returned to the caller.
  - Cover existing split-specific helper functions still ignoring callback false
    results and remaining side-effect compatible with their current `void` ABI.
  - Cover `goto_split:top` / `goto_split:bottom` aliases and invalid resize
    amount handling.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty binding_action -- --nocapture`
- `cargo test -p roastty split -- --nocapture`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the initial Experiment 702 design and blocked the plan commit
until two issues were fixed:

- `new_split:auto` must use Roastty's stored surface geometry instead of a
  hardcoded direction, because upstream chooses right only when width is greater
  than height.
- The design must explicitly add a bool-returning forwarding helper, because the
  existing split-specific `Surface::perform_action` path currently ignores the
  runtime callback return value for `void` helper APIs.

This revised design fixes both findings and adds verification for geometry-based
auto split selection plus preservation of the existing split helper behavior.
