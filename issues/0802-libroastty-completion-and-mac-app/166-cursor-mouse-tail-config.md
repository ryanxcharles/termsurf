# Experiment 166: Phase F — cursor mouse tail config

## Description

Remove `cursor-click-to-move` and `mouse-hide-while-typing` from the remaining
Phase F public-config tail.

Both are upstream bool config fields. This experiment wires their
parser/formatter/storage behavior and keeps runtime behavior out of scope:
prompt-click cursor movement and platform mouse hiding remain separate
terminal/app integration work.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config` fields `cursor_click_to_move` and `mouse_hide_while_typing`.
  - Use upstream defaults: `cursor-click-to-move = true` and
    `mouse-hide-while-typing = false`.
  - Format both fields in upstream declaration order immediately after
    `cursor-text` and before `scroll-to-bottom`.
  - Route `Config::set` for both keys using upstream bool semantics:
    bare/missing bool values set `true`, empty values reset to defaults, and
    invalid bool strings report `InvalidValue`.
  - Update config field-order/default tests and add focused
    parse/format/reset/load/clone coverage.

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Mark Experiment 166 as `Designed`.
  - After result, update the Phase F remaining-public-options count from 24 to
    22 and remove cursor-click/mouse-hide wording if this passes.

## Verification

Before implementation:

- Codex-native adversarial design review approves this experiment.
- Commit the reviewed plan separately from the result.

After implementation:

- `cargo test -p roastty cursor_mouse_tail_config`
- `cargo test -p roastty config_format_config_emits_fields_in_upstream_order`
- `cargo test -p roastty`
- `cargo fmt --check -p roastty`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/166-cursor-mouse-tail-config.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `git diff --check`

**Pass** = both keys parse, format, reset, load, and report diagnostics with
upstream defaults/order/bool semantics, and the full roastty test suite passes.

**Partial** = the direct parser/formatter fields land, but ordering, load/replay
behavior, diagnostics, or full-suite verification remains incomplete.

**Fail** = the fields cannot be added without conflicting with existing cursor,
mouse, or config storage behavior.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Peirce`, fresh context.

**Verdict:** Approved with no findings.

The reviewer verified that the README links Experiment 166 as `Designed`, the
experiment has the required sections, upstream default/order semantics match
`Config.zig`, local bool helper semantics match the plan, and no implementation
was done before design review.
