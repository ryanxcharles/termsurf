+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "default"
reasoning = "medium"
+++

# Experiment 776: Surface Key Dispatch Checklist Sync

## Description

Audit and sync the Issue 801 C ABI checklist wording for surface key dispatch
and binding-action parsing.

The C ABI checklist still says the app/surface item is missing
`keybinding/action dispatch` and `full binding-action parsing`. Current code and
tests appear to have moved beyond that text: `roastty_surface_key` dispatches
configured and default bindings through `parse_binding_action` /
`perform_parsed_binding_action`, and `roastty_surface_binding_action` exposes
the same parser/executor through the public C ABI.

This experiment only updates issue documentation if verification confirms those
specific missing-work phrases are stale. It does not mark the whole app/surface
C ABI item complete, because frontend selection routing, split tree/frontend
mutations, and other surface lifecycle work are still listed as missing.

## Changes

- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Remove or rewrite the stale `keybinding/action dispatch` and
    `full binding-action parsing` missing-work phrases only if current code and
    tests prove those areas are complete enough for the checklist.
  - Keep the app/surface C ABI item unchecked and preserve the remaining missing
    work that is still true.

## Verification

- Inspect `roastty/include/roastty.h` to confirm the public C ABI still exposes
  `roastty_surface_key`, `roastty_surface_key_is_binding`, and
  `roastty_surface_binding_action`.
- Inspect `roastty/src/lib.rs` to confirm:
  - `Surface::key` dispatches configured keybinds and static default keybinds;
  - `dispatch_configured_binding` and `dispatch_default_binding` route through
    `parse_binding_action` / `perform_parsed_binding_action`;
  - `roastty_surface_binding_action` uses the same parser/executor surface;
  - tests exist for configured dispatch, default dispatch, binding-action parser
    false paths, and supported action families.
- Run:
  - `cargo test -p roastty surface_key_default -- --nocapture --test-threads=1`
  - `cargo test -p roastty surface_key_configured -- --nocapture --test-threads=1`
  - `cargo test -p roastty surface_key_is_binding -- --nocapture --test-threads=1`
  - `cargo test -p roastty surface_binding_action_ -- --nocapture --test-threads=1`
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/776-surface-key-dispatch-checklist-sync.md`
  - `git diff --check`

The experiment passes if the README update is documentation-only, removes only
verified-stale missing-work text, leaves the app/surface C ABI item unchecked,
and all verification commands pass.

## Design Review

Codex reviewed the design and found no blockers. The review confirmed the scope
is narrow and documentation-only, the app/surface C ABI checklist item remains
unchecked, and the planned inspections and test filters are sufficient for
auditing the `keybinding/action dispatch` and `full binding-action parsing`
phrases.

The design is approved for the plan commit.
