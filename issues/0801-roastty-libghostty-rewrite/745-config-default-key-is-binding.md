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

# Experiment 745: Config Default Key Is Binding

## Description

Experiments 742 through 744 added a static default reverse lookup for
`roastty_config_trigger`, but `roastty_config_key_is_binding` still validates
inputs and always returns `false`. Upstream Ghostty's `Config.keyEventIsBinding`
returns `false` for release events, then asks the keybind set whether a press or
repeat key event would trigger a binding. The set checks physical key first,
then single UTF-8 codepoint, then unshifted codepoint.

This experiment adds key-event lookup for the static default keybind set, not
only the menu-visible triggers exposed by `roastty_config_trigger`. Upstream
`keyEventIsBinding` recognizes performable bindings too, so this slice includes
performable defaults such as command-C/command-V, undo/redo, search, clear
screen, selection expansion, and natural text-editing bindings. It does not add
user keybind parsing, keybind storage, key tables, sequences, action dispatch,
or surface keybinding dispatch.

## Changes

- `roastty/src/lib.rs`
  - Add a static default key-event matcher used by
    `roastty_config_key_is_binding`.
  - Return `false` for null config, null event, and release events.
  - Match event modifiers using binding modifiers, ignoring lock keys and
    modifier side bits, as upstream does through `event.mods.binding()`.
  - Match physical-key defaults first, including physical Copy/Paste, selection
    expansion keys, split navigation/resizing arrows, viewport Home/End/PageUp/
    PageDown, prompt-jump arrows, Escape, Enter, Backspace, and natural
    text-editing arrow defaults.
  - Match unicode defaults from a single UTF-8 codepoint when present.
  - Fall back to `unshifted_codepoint` when no physical or UTF-8 match exists.
  - Return `true` for press and repeat events that match static defaults.
  - Include performable default bindings that upstream would treat as bindings:
    command-C/command-V, command-K, undo/redo, search actions, selection
    expansion, scroll-to-selection, and natural text-editing `text`/`esc`
    bindings.
  - Keep user-defined keybinds, key tables, sequences, and custom unbinds out of
    scope until real keybind storage exists.

- `roastty/tests/abi_harness.c`
  - Add representative C ABI checks that default physical, UTF-8, unshifted,
    repeat, release, performable, lock-modifier, and nonmatching events return
    the expected values.

- Tests in `roastty/src/lib.rs`
  - Cover physical-key defaults such as Copy, Paste, command-Arrow split
    navigation, command-Home scrolling, and command-shift-Enter split zoom.
  - Cover unicode defaults from UTF-8 for config/menu/window actions.
  - Cover performable defaults including command-C/command-V, command-K,
    command-Z, command-F, command-G, Escape, shift-arrow selection expansion,
    and natural text-editing command/option arrows.
  - Cover unshifted-codepoint fallback for unicode defaults when UTF-8 is empty.
  - Cover physical precedence over UTF-8 when both are present.
  - Cover repeat events matching and release events returning `false`.
  - Cover lock-key and side-bit modifiers not preventing binding matches.
  - Cover nonmatching key/modifier combinations returning `false`.
  - Keep existing `config_trigger`, `config_key_is_binding`, binding-action, and
    ABI harness tests passing.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty config_key_is_binding -- --nocapture --test-threads=1`
- `cargo test -p roastty config_trigger -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 745 design and found one real technical blocker:
the initial plan limited key-event lookup to the static defaults exposed through
`roastty_config_trigger`, but upstream `Config.keyEventIsBinding` checks whether
an event would trigger any binding in the binding set. That includes performable
defaults that are intentionally excluded from the reverse trigger API.

The plan now includes those performable default key events while still leaving
user keybind storage, key tables, sequences, action dispatch, and custom
configuration out of scope. The review confirmed the lookup order is otherwise
correct: release events return `false`; press/repeat events check physical key
first, then a single UTF-8 codepoint, then `unshifted_codepoint`. It also
approved the modifier normalization plan: compare binding modifiers while
ignoring lock keys and side-specific modifier bits.

The review initially raised a stale process concern that Experiment 744 still
needed completion-review metadata and a result commit. Current git history shows
Experiment 744 has both required commits:
`9bde52e7fce82 Map windows to bright keys` and
`6f7c7eca5a1e4 Give windows their compass`. No Experiment 744 blocker remains.

The remaining workflow requirement from the review was to record
`[review.design]`, this review section, and the README tuple before the
Experiment 745 plan commit; those records are now present.

## Result

**Result:** Pass

`roastty_config_key_is_binding` now recognizes the static default binding set
for press and repeat events. The matcher returns `false` for null config, null
event, and release events; checks physical keys first; checks a single UTF-8
codepoint next; and falls back to `unshifted_codepoint`. Modifier matching uses
binding modifiers, so lock keys and side-specific modifier bits do not prevent
default keybind matches.

The implemented default event set includes the default reverse-trigger actions
from Experiments 742 through 744 and the performable defaults that upstream
`Config.keyEventIsBinding` treats as bindings, including command-C/command-V,
command-K, undo/redo, search, selection expansion, scroll-to-selection, Escape,
and natural text-editing keys.

Verification passed:

- `cargo fmt -p roastty`
- `cargo test -p roastty config_key_is_binding -- --nocapture --test-threads=1`
- `cargo test -p roastty config_trigger -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness -- --nocapture`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

The default-keybind query now has a useful static implementation while still
leaving custom user keybind storage, key tables, sequences, unbinds, and surface
keybinding dispatch for later experiments. The next slice can build on this by
adding real keybind storage or by moving the static defaults into the surface
dispatch path, depending on which upstream behavior is needed next.

## Completion Review

Codex reviewed the completed Experiment 745 diff and found one real technical
gap: command-`=` was missing from the key-event matcher even though upstream
keeps both command-`=` and command-`+` as default font-size bindings. The
implementation now includes command-`=` under `ROASTTY_MODS_SUPER`, with Rust
and C ABI coverage.

Codex re-reviewed the fixed diff and reported no remaining blocking technical
issues. The review confirmed the lookup order, modifier normalization,
repeat/release behavior, physical precedence, unshifted fallback, and
representative performable defaults match the experiment design.
