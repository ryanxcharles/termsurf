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

# Experiment 720: Binding Action Title

## Description

Experiment 719 added font-size binding actions. Upstream Ghostty's nearby
surface-scoped actions also forward title changes to the app runtime:

- `prompt_surface_title`
- `prompt_tab_title`
- `set_surface_title:<title>`
- `set_tab_title:<title>`

Roastty already has the generic runtime action callback used for split actions,
but the public action tag set currently exposes only split-related tags. This
experiment adds the title action tags and storage conventions needed for the
macOS frontend to handle surface and tab title prompts/overrides.

This does not implement the Swift prompt UI, tab model storage, title override
state, undo/redo, or copy-title behavior. It only parses binding actions and
forwards the appropriate app-runtime action through the existing synchronous
callback ABI.

## Changes

- `roastty/include/roastty.h`
  - Add action tags matching upstream `ghostty_action_tag_e` values:
    - `ROASTTY_ACTION_SET_TITLE = 32`
    - `ROASTTY_ACTION_SET_TAB_TITLE = 33`
    - `ROASTTY_ACTION_PROMPT_TITLE = 34`
  - Add prompt-title selector constants matching upstream
    `ghostty_prompt_title_e` values:
    - `ROASTTY_PROMPT_TITLE_SURFACE = 0`
    - `ROASTTY_PROMPT_TITLE_TAB = 1`
  - Document the title action storage convention:
    - prompt title: `storage[0]` is the prompt-title selector;
    - set title / set tab title: `storage[0]` is a borrowed null-terminated
      `const char *` valid only for the duration of `action_cb`.

- `roastty/src/lib.rs`
  - Add matching action and prompt-title constants.
  - Extend the internal parsed binding-action enum with:
    - `PromptTitle(c_int)`
    - `SetTitle(c_int, Vec<u8>)`
  - Extend `parse_binding_action` to accept:
    - `prompt_surface_title` with no parameter;
    - `prompt_tab_title` with no parameter;
    - `set_surface_title:<bytes>`;
    - `set_tab_title:<bytes>`.
  - Reject any parameter on prompt actions.
  - Require set-title parameters to be valid UTF-8 and NUL-free so they can be
    passed as borrowed C strings.
  - Allow empty set-title parameters, matching upstream's reset/clear behavior.
  - Add dispatcher handling that:
    - returns `false` for null and detached surfaces;
    - returns `false` when no runtime `action_cb` is installed;
    - forwards prompt actions with the prompt selector in `storage[0]`;
    - forwards set-title actions with the borrowed C string pointer in
      `storage[0]`;
    - returns the runtime callback result.
  - Keep clipboard, font-size, split, close, text/CSI/ESC, reset, clear-screen,
    scroll, prompt-jump, select-all, and adjust-selection semantics unchanged.

- `roastty/tests/abi_harness.c`
  - Add C ABI smoke coverage for the new action and prompt-title constants.
  - Add malformed title action rejection checks.
  - Add no-callback coverage that valid title actions return `false` without
    crashing.

- Tests in `roastty/src/lib.rs`
  - Cover constant values matching upstream.
  - Cover parser false paths for `prompt_surface_title:`,
    `prompt_surface_title:now`, `prompt_tab_title:`, and `prompt_tab_title:now`.
  - Cover parser false paths for invalid UTF-8 and NUL-containing set-title
    parameters.
  - Cover null, detached, and no-callback surfaces returning `false`.
  - Cover prompt title actions forwarding the expected action tag, target, and
    selector storage.
  - Cover set-title actions forwarding the expected action tag, target, C string
    bytes, empty-title reset bytes, and callback result.
  - Re-run existing binding-action tests to prove previous action semantics did
    not change.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty title -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 720 design and found the scope otherwise sound:
title actions are limited to parser and app-runtime forwarding behavior, while
Swift prompt UI, persistent title state, and copy-title behavior remain
deferred. The review approved the borrowed C string convention because the plan
documents that the pointer is valid only for the duration of `action_cb` and
requires valid UTF-8 plus NUL-free bytes before constructing the C string.

The review raised two test-plan blockers:

- invalid UTF-8 set-title parameters needed explicit parser false-path coverage;
- prompt action parameter rejection needed to explicitly cover empty and
  non-empty colon forms.

The plan now covers invalid UTF-8 and NUL-containing set-title parameters, plus
`prompt_surface_title:`, `prompt_surface_title:now`, `prompt_tab_title:`, and
`prompt_tab_title:now`.

The review also raised the normal workflow provenance requirement. Design-review
frontmatter and this section are now present, and the README provenance tuple
will be updated to `Codex/Codex/-` before the plan commit. Result-review
provenance will be added only after implementation and completion review.

Codex re-reviewed the revised design and found no remaining blockers. The review
approved the invalid UTF-8 and NUL set-title coverage, explicit empty/non-empty
prompt colon coverage, and provenance record.

## Result

**Result:** Pass

Implemented title binding-action parsing and dispatch for
`prompt_surface_title`, `prompt_tab_title`, `set_surface_title`, and
`set_tab_title`. Roastty now exposes the upstream-matching runtime action tags
for set title, set tab title, and prompt title, plus the prompt-title selector
constants.

Prompt actions forward the selector in `storage[0]`. Set-title actions validate
UTF-8 and NUL-free parameters, allow empty titles for reset/clear behavior, and
forward a borrowed C string pointer in `storage[0]` for the duration of
`action_cb`. Tests copy that string inside the callback to match the documented
lifetime.

The C ABI header now documents the action storage convention for split and title
actions, and the harness covers constants, malformed prompt forms, and valid
no-callback title actions returning `false`.

Verification:

- `cargo fmt -p roastty`
- `cargo test -p roastty title -- --nocapture --test-threads=1` — 11 passed
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1` — 74
  passed
- `cargo test -p roastty --test abi_harness` — 1 passed
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

Roastty now has the binding-action and runtime-action ABI slice needed for the
macOS frontend to prompt for or set surface/tab titles. Remaining title work is
frontend/UI behavior and persistent title override state; this experiment only
establishes parser and runtime callback forwarding parity.

## Completion Review

Codex reviewed the completed Experiment 720 implementation and result record.
The review found one workflow blocker: result-review provenance was not yet
recorded in the experiment frontmatter or README tuple. This file now includes
`[review.result]`, and the README provenance tuple has been updated to
`Codex/Codex/Codex`.

The review found no code blockers. It approved the upstream-matching constants,
header storage convention, borrowed C string lifetime handling, parser behavior
for prompt and set-title actions, null/detached/no-callback false paths,
callback-result propagation, Rust tests, C ABI smoke coverage, and
binding-action regression coverage.
