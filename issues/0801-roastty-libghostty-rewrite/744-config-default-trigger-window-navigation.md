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

# Experiment 744: Config Default Trigger Window Navigation

## Description

Experiments 742 and 743 moved `roastty_config_trigger` from an empty-trigger
stub to a static default lookup for config, clipboard, font-size, and
write-screen-file menu actions. Upstream Ghostty's macOS default keybind set
also exposes non-performable defaults for windowing, tabs, splits, viewport
scrolling, prompt jumps, fullscreen, command palette, and selection clipboard
paste through the same reverse lookup.

This experiment adds that next coherent macOS default-trigger slice. It still
does not add user keybind parsing, keybind storage, key tables, sequences,
performable reverse lookup, natural text-editing bindings, search bindings,
`roastty_config_key_is_binding`, or surface key dispatch.

## Changes

- `roastty/src/lib.rs`
  - Extend `default_config_trigger` with upstream macOS non-performable default
    triggers:
    - `quit` -> unicode `q` + super
    - `select_all` -> unicode `a` + super
    - `goto_tab:1` through `goto_tab:8` -> unicode `1` through `8` + super
    - `last_tab` -> unicode `9` + super
    - `new_window` -> unicode `n` + super
    - `close_surface` -> unicode `w` + super
    - `close_tab` and `close_tab:this` -> unicode `w` + alt + super
    - `close_window` -> unicode `w` + shift + super
    - `close_all_windows` -> unicode `w` + shift + alt + super
    - `new_tab` -> unicode `t` + super
    - `previous_tab` -> unicode `[` + shift + super
    - `next_tab` -> unicode `]` + shift + super
    - `new_split:right` -> unicode `d` + super
    - `new_split:down` -> unicode `d` + shift + super
    - `goto_split:previous` -> unicode `[` + super
    - `goto_split:next` -> unicode `]` + super
    - `goto_split:up/down/left/right` -> physical arrow keys + alt + super
    - `resize_split:up/down/left/right,10` -> physical arrow keys + ctrl + super
    - `equalize_splits` -> unicode `=` + ctrl + super
    - `toggle_split_zoom` -> physical Enter + shift + super
    - `toggle_fullscreen` -> unicode `f` + ctrl + super, matching the later
      macOS alternate binding that wins the reverse map over the earlier Enter
      binding
    - `toggle_command_palette` -> unicode `p` + shift + super
    - `scroll_to_top` / `scroll_to_bottom` -> physical Home / End + super
    - `scroll_page_up` / `scroll_page_down` -> physical PageUp / PageDown +
      super
    - `jump_to_prompt:-1` / `jump_to_prompt:1` -> physical ArrowUp / ArrowDown
      with `ROASTTY_MODS_SUPER` only, matching the later macOS binding that wins
      over the earlier shift-super binding
    - `inspector:toggle` -> unicode `i` + alt + super
    - `paste_from_selection` -> unicode `v` + shift + super
  - Keep performable defaults excluded from the lookup, including
    `clear_screen`, `undo`, `redo`, `scroll_to_selection`, `start_search`,
    `end_search`, `search_selection`, and natural text-editing `text`/`esc`
    actions.
  - Keep unsupported variants such as `goto_tab:0`, `goto_tab:9`,
    `resize_split:up,5`, and `toggle_fullscreen:native` returning the empty
    trigger.
  - Keep `roastty_config_key_is_binding` unchanged.

- `roastty/tests/abi_harness.c`
  - Add C ABI checks for a representative set of the new defaults covering
    unicode, physical, alias, later-binding-wins, and empty-trigger fallback
    cases.

- Tests in `roastty/src/lib.rs`
  - Cover every new default trigger and its exact tag/key/modifier shape.
  - Cover aliases such as `close_tab` and `close_tab:this`.
  - Cover later-binding-wins cases for `toggle_fullscreen` and `jump_to_prompt`.
  - Cover representative excluded performable actions and unsupported variants
    returning the empty trigger.
  - Keep existing `config_trigger`, `config_key_is_binding`, binding-action, and
    ABI harness tests passing.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty config_trigger -- --nocapture --test-threads=1`
- `cargo test -p roastty config_key_is_binding -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 744 design. It approved the scope as a coherent
static default reverse-lookup expansion that does not add keybind storage,
config parsing, dispatch, or `roastty_config_key_is_binding`.

The review found one real design ambiguity: the prompt-jump entries originally
used unclear wording for the modifier shape. The plan now states the exact
expected value: physical ArrowUp / ArrowDown with `ROASTTY_MODS_SUPER` only,
matching upstream's later macOS binding that wins over the earlier shift-super
binding.

The review also confirmed the test plan is strong: Rust tests cover every
default exactly, aliases, later-binding-wins cases, excluded performable
actions, unsupported variants, and regressions, while representative ABI harness
coverage is acceptable because exhaustive coverage is in Rust.

The review initially raised a stale process concern that Experiment 743 still
needed completion-review metadata and a result commit. Current git history shows
Experiment 743 has both required commits: `1ca3ab40afd29 Hang keys on the menu`
and `6a61c40cc4aed Set menu bells to keys`. No Experiment 743 blocker remains.

The remaining workflow requirement from the review was to record
`[review.design]`, this review section, and the README tuple before the
Experiment 744 plan commit; those records are now present.
