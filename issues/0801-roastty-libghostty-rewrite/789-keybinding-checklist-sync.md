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

# Experiment 789: Keybinding Checklist Sync

## Description

The Issue 801 checklist still says the keybinding system is missing, but the
current Roastty tree already has a focused foundation for configured keybinds,
default key bindings, action string parsing, and surface/app action dispatch.
The implementation is not a full Ghostty binding subsystem yet: the dedicated
Ghostty `Binding` type model, full config export/remap behavior, platform
keymaps/layouts, and frontend/global menu integration are still incomplete or
tracked by adjacent checklist rows.

This experiment verifies the existing keybinding foundation and updates the
checklist wording from "missing" to a scoped partial state. It does not add new
keybinding code and does not close the separate keymaps, Kitty keyboard,
configuration, frontend, or app lifecycle rows.

## Changes

- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Change the keybinding-system checklist row from "missing" to a partial
    foundation summary.
  - Keep the row unchecked because the full Ghostty binding model and adjacent
    integrations remain incomplete.
  - Add the Experiment 789 index entry.
- `issues/0801-roastty-libghostty-rewrite/789-keybinding-checklist-sync.md`
  - Record the verification evidence and review result.

## Verification

- Inspect current keybinding and action-dispatch code:
  - `roastty/src/input/key.rs`
  - `roastty/src/input/key_mods.rs`
  - `roastty/src/input/key_encode.rs`
  - `roastty/src/lib.rs`
- Run focused configured/default binding checks:
  - `cargo test -p roastty keybind -- --nocapture --test-threads=1`
  - `cargo test -p roastty key_is_binding -- --nocapture --test-threads=1`
- Run representative action-dispatch checks:
  - `cargo test -p roastty surface_binding_action_app_runtime -- --nocapture --test-threads=1`
  - `cargo test -p roastty surface_binding_action_text -- --nocapture --test-threads=1`
  - `cargo test -p roastty surface_binding_action_forwards_supported_split_actions -- --nocapture --test-threads=1`
  - `cargo test -p roastty surface_binding_action_forwards_supported_runtime_ui_actions -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/789-keybinding-checklist-sync.md`
- Run:
  - `git diff --check`

The experiment passes if the existing keybinding foundation is present, focused
tests pass, and the README row is updated to a scoped partial state without
overclaiming the full Ghostty binding model or adjacent keymap/config/frontend
work. It is Partial if verification shows only action parsing or only keybind
matching exists. It fails if the original "missing" wording is still accurate.

## Design Review

Codex reviewed the design and found no blocking findings. The review approved
the docs-only scope, the unchecked partial README row, the explicit open work
for the full Ghostty `Binding` model and adjacent integrations, and the
non-empty focused test filters.
