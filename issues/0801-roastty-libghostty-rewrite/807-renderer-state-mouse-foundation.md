+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "default"
reasoning = "medium"

[review.result]
agent = "codex"
model = "default"
reasoning = "medium"
+++

# Experiment 807: Renderer State Mouse Foundation

## Description

Port the value-level renderer `State` and `Mouse` foundation from upstream
`renderer/State.zig` into Roastty.

Roastty already has the upstream `Preedit` type and range behavior in
`roastty/src/renderer/state.rs`, but the renderer checklist still records
`Render state` as partial because the outer `State` and `Mouse` fields are
missing. This experiment adds the data model that renderers consume without
attempting to wire it into the live renderer thread or terminal snapshot update
loop.

Upstream `State` carries a mutex, terminal pointer, optional inspector pointer,
optional preedit, and mouse state. In Roastty, the terminal/inspector/mutex
fields depend on renderer-thread ownership and frontend integration that are
tracked separately. This experiment should preserve the value fields that can be
ported safely now: optional preedit and renderer-relevant mouse state.

## Changes

- `roastty/src/renderer/state.rs`
  - Update the module description from "preedit only" to renderer state values.
  - Add `Mouse` with:
    - optional viewport `Coordinate`, matching upstream `Mouse.point`;
    - active `crate::input::key_mods::Mods`, matching upstream `Mouse.mods`.
  - Add `State` with:
    - optional `Preedit`;
    - `Mouse`;
    - explicit constructor/update helpers for setting/clearing preedit and mouse
      point/mods.
  - Keep terminal pointer, inspector pointer, and mutex/threading fields out of
    this experiment, documenting that those are renderer-thread integration
    work.
  - Add focused tests for default state, setting/clearing preedit, mouse point
    and modifier updates, preserving a modifier bit that terminal `MouseMods`
    cannot represent such as `super_`, `caps_lock`, `num_lock`, or side state,
    and cloning owned preedit codepoints without aliasing.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the renderer `Render state` checklist row from
    "only `Preedit`" to mention `State`/`Mouse` value foundations while keeping
    full live renderer state integration open.

## Verification

- Inspect:
  - `vendor/ghostty/src/renderer/State.zig`
  - `roastty/src/renderer/state.rs`
  - `roastty/src/terminal/point.rs`
  - `roastty/src/input/key_mods.rs`
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty renderer::state -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/807-renderer-state-mouse-foundation.md`
- Run:
  - `git diff --check`

The experiment passes if Roastty has a tested renderer `State`/`Mouse` value
foundation while the checklist remains partial for live render-thread,
terminal/inspector ownership, and renderer update-loop integration. It is
Partial if `Mouse` lands but the outer `State` needs follow-up. It fails if the
renderer state cannot be cleanly separated from live renderer-thread ownership.

## Design Review

Codex reviewed the design and found one blocking ambiguity: the original plan
said renderer `Mouse.mods` should match upstream, but it did not name the
Roastty type and pointed verification at terminal mouse-reporting modifiers.
Upstream uses input/key modifiers for renderer mouse state, so the plan now
requires `crate::input::key_mods::Mods`, updates verification to
`roastty/src/input/key_mods.rs`, and requires a test that preserves a modifier
bit terminal `MouseMods` cannot represent.

Codex re-reviewed the corrected design and approved it with no findings. The
approval confirmed that `Mouse.point` maps to viewport coordinates, `Mouse.mods`
maps to `crate::input::key_mods::Mods`, omitting mutex/terminal/inspector fields
is properly scoped to later renderer-thread/frontend integration, and the
planned checklist update does not claim live renderer integration.

## Result

**Result:** Pass

Roastty now has the value-level renderer state foundation in
`roastty/src/renderer/state.rs`. The module keeps the existing upstream-derived
`Preedit` behavior and adds:

- `Mouse`, with optional viewport `Coordinate` and
  `crate::input::key_mods::Mods` for renderer-relevant input modifiers;
- `State`, with optional `Preedit`, `Mouse`, and explicit helpers for
  setting/clearing preedit and mouse point/modifier state.

The implementation intentionally does not add upstream's mutex, terminal
pointer, or inspector pointer fields. Those are tied to renderer-thread and
frontend ownership, which remain separate incomplete slices.

The Issue 801 renderer checklist now records render state as partial with
`Preedit` and value-level `State`/`Mouse` foundations present. Live
renderer-thread terminal/inspector ownership and update-loop integration remain
missing.

Verification:

- Inspected `vendor/ghostty/src/renderer/State.zig`.
- Inspected `roastty/src/renderer/state.rs`.
- Inspected `roastty/src/terminal/point.rs`.
- Inspected `roastty/src/input/key_mods.rs`.
- `cargo fmt -p roastty` — passed.
- `cargo test -p roastty renderer::state -- --nocapture --test-threads=1` —
  passed, 8 tests.
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/807-renderer-state-mouse-foundation.md`
  — passed.
- `git diff --check` — passed.

## Conclusion

Experiment 807 moves renderer state beyond preedit-only support by adding the
renderer mouse and outer state value model. The next renderer-state work can
connect these values to the live renderer thread, terminal snapshots, inspector
state, and update-loop ownership model.

## Completion Review

Codex reviewed the staged result and found one blocking documentation
consistency issue: the experiment file recorded `Pass`, but the Issue 801
experiment index still listed Experiment 807 as `Designed`. The README index now
marks Experiment 807 as `Pass · Codex/Codex/Codex`. Codex found no code-level
blocking issues: the implementation uses input key modifiers, preserves
`super_`/lock/side state in tests, covers preedit clone ownership, and keeps
live renderer integration scoped out.

Codex re-reviewed the corrected staged result and approved it with no findings.
The approval confirmed that the README status was fixed, the value-level state
matches upstream `preedit`/`mouse` fields for this slice, tests cover modifier
fidelity and preedit ownership, and the docs do not claim live renderer-thread,
terminal, inspector, or update-loop integration.
