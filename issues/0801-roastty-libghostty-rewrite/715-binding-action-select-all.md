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

# Experiment 715: Binding Action Select All

## Description

Experiment 714 added `jump_to_prompt:<i16>` binding-action support. Upstream
Ghostty's `performBindingAction` also supports `select_all`, which selects all
nonblank terminal content on the active screen and queues a render when a
selection exists.

Roastty already has the core terminal selection behavior:

- `Terminal::select_all()` returns the active screen's trimmed all-content
  selection;
- `Terminal::set_selection(Some(selection))` installs an active selection;
- `roastty_terminal_select_all` exposes terminal-level C ABI coverage;
- existing tests cover empty, whitespace-only, trimmed-edge, and scrollback
  select-all cases.

This experiment only wires the existing terminal behavior into
`roastty_surface_binding_action("select_all")`.

This does not implement `adjust_selection`, copy/paste actions, search actions,
write-file actions, keybind storage/lookup, frontend selection routing, or
clipboard integration.

## Changes

- `roastty/src/lib.rs`
  - Extend the internal parsed binding-action enum with `SelectAll`.
  - Extend `parse_binding_action` to accept exactly `select_all` and reject
    `select_all:` or any parameterized form.
  - Add a surface helper that:
    - returns `false` for null, detached, and no-worker surfaces;
    - calls `Terminal::select_all()` on attached worker-backed surfaces;
    - installs the returned selection with `Terminal::set_selection`;
    - requests a render and returns `true` when a selection is installed;
    - returns `true` without changing selection or requesting render when the
      terminal has no selectable content, matching upstream's consumed action
      behavior.
  - Keep split, close, `text:`, `csi:`, `esc:`, `reset`, `clear_screen`, scroll,
    and prompt-jump action semantics unchanged.

- `roastty/tests/abi_harness.c`
  - Add C ABI smoke coverage that `select_all:` and `select_all:now` are
    rejected.
  - Add no-worker coverage that `select_all` returns `false` without crashing.

- Tests in `roastty/src/lib.rs`
  - Cover parser false paths for parameterized `select_all`.
  - Cover null, detached, and no-worker surfaces returning `false`.
  - Cover worker-backed empty/whitespace-only terminals consuming `select_all`
    without installing a selection or marking the surface as needing render.
  - Cover worker-backed terminals with text installing the same selection
    returned by `Terminal::select_all()` and marking the surface as needing
    render.
  - Re-run existing binding-action tests to prove previous action semantics did
    not change.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty select_all -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 715 design and found no technical blockers. The
review confirmed that the scope is narrow and upstream-compatible: accept only
bare `select_all`, reject colon/parameter forms, return `false` for null,
detached, and no-worker surfaces, consume worker-backed empty terminals as
`true`, and install the same trimmed selection as `Terminal::select_all()` for
worker-backed terminals with text.

The review raised one workflow blocker before plan commit: record design-review
provenance in this file and update the README provenance tuple to
`Codex/Codex/-`. Those fields are now present. The review also suggested making
the render behavior explicit where the harness can observe it. The plan now
requires tests proving that empty/whitespace-only select-all does not mark the
surface as needing render, while non-empty select-all does.

The review mentioned result-review provenance, but that belongs to the
post-implementation result checkpoint and will be added only after completion
review.
