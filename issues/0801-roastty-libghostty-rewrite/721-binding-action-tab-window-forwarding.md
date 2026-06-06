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

# Experiment 721: Binding Action Tab Window Forwarding

## Description

Experiment 720 added title binding actions. Upstream Ghostty's nearby
surface-scoped actions also forward tab and window commands to the app runtime:

- `new_tab`
- `close_tab[:this|other|right]`
- `goto_window:previous|next`
- `toggle_split_zoom`
- `reset_window_size`
- `toggle_maximize`
- `toggle_fullscreen`

Roastty already has the generic runtime action callback and split forwarding.
This experiment adds the next small app-runtime forwarding slice for tab/window
commands. It does not implement the tab model, window UI, fullscreen policy, or
Swift frontend behavior; it only parses binding actions and forwards the
upstream-shaped action tags/storage through the existing callback ABI.

## Changes

- `roastty/include/roastty.h`
  - Add action tags matching upstream `ghostty_action_tag_e` values:
    - `ROASTTY_ACTION_NEW_TAB = 2`
    - `ROASTTY_ACTION_CLOSE_TAB = 3`
    - `ROASTTY_ACTION_TOGGLE_MAXIMIZE = 6`
    - `ROASTTY_ACTION_TOGGLE_FULLSCREEN = 7`
    - `ROASTTY_ACTION_GOTO_WINDOW = 17`
    - `ROASTTY_ACTION_TOGGLE_SPLIT_ZOOM = 20`
    - `ROASTTY_ACTION_RESET_WINDOW_SIZE = 23`
  - Add close-tab mode constants matching upstream
    `ghostty_action_close_tab_mode_e`:
    - `ROASTTY_ACTION_CLOSE_TAB_THIS = 0`
    - `ROASTTY_ACTION_CLOSE_TAB_OTHER = 1`
    - `ROASTTY_ACTION_CLOSE_TAB_RIGHT = 2`
  - Add goto-window constants matching upstream `ghostty_goto_window_e`:
    - `ROASTTY_GOTO_WINDOW_PREVIOUS = 0`
    - `ROASTTY_GOTO_WINDOW_NEXT = 1`
  - Add fullscreen constants matching upstream `ghostty_fullscreen_e`:
    - `ROASTTY_FULLSCREEN_NATIVE = 0`
    - `ROASTTY_FULLSCREEN_MACOS_NON_NATIVE = 1`
    - `ROASTTY_FULLSCREEN_MACOS_NON_NATIVE_VISIBLE_MENU = 2`
    - `ROASTTY_FULLSCREEN_MACOS_NON_NATIVE_PADDED_NOTCH = 3`
  - Document storage conventions:
    - close tab: `storage[0] = roastty_action_close_tab_mode_e`
    - goto window: `storage[0] = roastty_goto_window_e`
    - toggle fullscreen: `storage[0] = roastty_fullscreen_e`
    - no-storage actions leave storage zeroed.

- `roastty/src/lib.rs`
  - Add matching constants.
  - Extend the internal parsed binding-action enum with `CloseTab(c_int)`,
    `GotoWindow(c_int)`, and no-storage runtime action variants as needed.
  - Extend `parse_binding_action` to accept:
    - `new_tab`
    - `close_tab` as the upstream default `this`
    - `close_tab:this`
    - `close_tab:other`
    - `close_tab:right`
    - `goto_window:previous`
    - `goto_window:next`
    - `toggle_split_zoom`
    - `reset_window_size`
    - `toggle_maximize`
    - `toggle_fullscreen`
  - Reject missing, empty, whitespace-padded, unknown, and extra-colon
    parameters where applicable; reject any parameter for no-parameter actions.
  - Forward actions through `action_cb`, returning `false` for null, detached,
    and no-callback surfaces and otherwise returning the callback result.
  - Forward `toggle_fullscreen` with `ROASTTY_FULLSCREEN_NATIVE` because Roastty
    does not yet expose Ghostty's macOS non-native fullscreen config.
  - Keep title, clipboard, font-size, split, close-surface, text/CSI/ESC, reset,
    clear-screen, scroll, prompt-jump, select-all, and adjust-selection
    semantics unchanged.

- `roastty/tests/abi_harness.c`
  - Add C ABI smoke coverage for new action constants and enum constants.
  - Add malformed tab/window action rejection checks.
  - Add no-callback coverage that valid tab/window forwarding actions return
    `false` without crashing.

- Tests in `roastty/src/lib.rs`
  - Cover constant values matching upstream.
  - Cover parser false paths for invalid close-tab, goto-window, and
    no-parameter action forms.
  - Cover null, detached, and no-callback surfaces returning `false`.
  - Cover valid tab/window actions forwarding expected action tags, target,
    storage, and callback result.
  - Re-run existing binding-action tests to prove previous action semantics did
    not change.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty tab_window -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 721 design and found no findings. The review
approved the scope as parser plus app-runtime forwarding only, with action tags,
storage conventions, close-tab and goto-window modes, no-storage actions,
malformed-form tests, and callback-result tests all covered.

The review also accepted the explicit `toggle_fullscreen` scope decision:
Roastty forwards `ROASTTY_FULLSCREEN_NATIVE` for now because the macOS
non-native fullscreen configuration is not exposed yet.
