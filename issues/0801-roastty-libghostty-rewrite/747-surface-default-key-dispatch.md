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

# Experiment 747: Surface Default Key Dispatch

## Description

Experiments 745 and 746 taught Roastty to answer whether a key event matches the
static default keybind set and, at the surface boundary, which default flags
that match carries. `roastty_surface_key` still ignores those default keybinds:
it stores the event, encodes it, and writes the encoded bytes to the child PTY
when possible.

Upstream Ghostty handles bindings before terminal encoding. For a matched
ordinary consumed binding, the key event is consumed even if the action cannot
perform. For a matched performable binding, an unperformed action acts as if the
binding does not exist and the key falls through to terminal encoding. Release
events do not trigger bindings, but releases for a consumed press are also
consumed so Kitty keyboard release reporting does not leak a release event after
a consumed press.

This experiment adds a static default key-dispatch foundation for the default
bindings that Roastty can already execute through `parse_binding_action` and
`perform_parsed_binding_action`. It does not add user keybind parsing, keybind
storage, active key tables, key sequences, key remaps, chained bindings, custom
unbinds, global/all binding fanout, or unsupported default actions such as
search navigation.

## Changes

- `roastty/src/lib.rs`
  - Extend the static default key-event matcher so a match can return both flags
    and the default binding-action string.
  - Keep `roastty_config_key_is_binding` and `roastty_surface_key_is_binding`
    using that shared matcher for bool/flags queries.
  - Add surface key dispatch before terminal encoding in `Surface::key`.
  - Dispatch only static defaults whose actions are already supported by
    `parse_binding_action`, including:
    - `open_config`, `reload_config`;
    - `copy_to_clipboard`, `paste_from_clipboard`, `paste_from_selection`;
    - font-size actions;
    - write-screen file actions;
    - selection expansion;
    - tab/window/split actions already supported by the binding-action parser;
    - scrolling and prompt-jump actions;
    - `start_search`, `end_search`, `search_selection`;
    - `clear_screen`, `select_all`, `undo`, `redo`, inspector, fullscreen, and
      natural text-editing `text`/`esc` actions.
  - Leave supported query-only defaults without a supported action parser
    outside dispatch for this experiment.
  - For ordinary consumed defaults, return `true` from `roastty_surface_key`
    even if the action callback or action preconditions make the action return
    `false`.
  - For performable defaults, return `true` only when the action performs; when
    it does not perform, continue to terminal encoding.
  - Track the last consumed static default press/repeat using a normalized
    release identity: physical key plus normalized binding modifiers. This
    intentionally does not depend on UTF-8 or `unshifted_codepoint`, because
    release events for unicode bindings may arrive without text payloads.
  - Preserve existing raw terminal key behavior for nonmatching events and
    performable non-performed events.

- `roastty/tests/abi_harness.c`
  - Add representative C ABI checks for default key dispatch where practical:
    ordinary runtime-action dispatch, performable fallthrough without action
    support, and nonmatching terminal behavior remain covered by Rust tests.

- Tests in `roastty/src/lib.rs`
  - Cover ordinary defaults consuming and forwarding supported runtime actions,
    for example command-D `new_split:right`, command-Home `scroll_to_top`, and
    command-`=` / command-`+` font-size increase.
  - Cover ordinary defaults consuming even when the runtime callback returns
    `false` or is absent.
  - Cover performable defaults performing and consuming when preconditions are
    met, for example command-K `clear_screen`, Escape `end_search`, or
    shift-arrow selection expansion.
  - Cover performable defaults falling through to the terminal when the action
    cannot perform, for example command-C without an active selection/worker or
    command-F without a runtime search callback.
  - Cover natural text-editing defaults writing their legacy text/escape bytes
    instead of the normal encoded key.
  - Cover release suppression after a consumed default press, including a
    unicode default such as command-D whose release event carries the same
    physical key/modifiers but empty UTF-8.
  - Cover release fallthrough for non-consumed/nonmatching presses.
  - Cover an unsupported query-only default such as command-G or shift-command-G
    search navigation falling through because no dispatch action exists yet.
  - Keep existing `surface_key`, `surface_key_is_binding`,
    `config_key_is_binding`, `binding_action`, and ABI harness tests passing.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty surface_key_default -- --nocapture --test-threads=1`
- `cargo test -p roastty surface_key_is_binding -- --nocapture --test-threads=1`
- `cargo test -p roastty surface_key -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness -- --nocapture`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the initial Experiment 747 design and found two technical gaps
that needed to be fixed before the plan commit.

First, release suppression was under-specified. Unicode default presses such as
command-D or command-`=` may be followed by release events that have the same
physical key and modifiers but no UTF-8 or unshifted payload. The plan now
specifies a normalized release identity of physical key plus normalized binding
modifiers, with tests for a consumed unicode default press suppressing an empty
UTF-8 release.

Second, unsupported query-only defaults need explicit fallthrough coverage. The
static matcher includes some defaults whose actions are not yet supported by the
binding-action parser, such as search navigation. The plan now includes a test
proving command-G or shift-command-G remains query-visible but is not consumed
by this dispatch layer until a supported action exists.

## Result

**Result:** Pass

`roastty_surface_key` now checks the static default keybind matcher before
terminal encoding. Matching defaults that have a supported binding-action string
dispatch through the same parsed action semantics used by
`roastty_surface_binding_action`.

Ordinary consumed defaults return `true` and suppress the corresponding release
event even if their runtime callback or action preconditions do not perform.
Performable defaults return `true` only when the action performs; otherwise they
fall through to normal terminal encoding. Unsupported query-only defaults such
as command-G remain visible to `roastty_surface_key_is_binding` but are not
consumed by `roastty_surface_key` until their action parser/dispatcher exists.

The release suppression identity is physical key plus normalized binding
modifiers, so a unicode default press such as command-D suppresses an empty
UTF-8 release with the same physical key and modifiers.

Verification passed:

- `cargo fmt -p roastty`
- `cargo test -p roastty surface_key_default -- --nocapture --test-threads=1`
- `cargo test -p roastty surface_key_is_binding -- --nocapture --test-threads=1`
- `cargo test -p roastty surface_key -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness -- --nocapture`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

Roastty now has a static default key-dispatch foundation: default keybinds can
be queried, report flags, and execute supported actions before terminal
encoding. Remaining keybinding work can build from here toward real keybind
storage, custom config parsing, key tables, sequences, remaps, chained/global
bindings, and unsupported default actions such as search navigation.

## Completion Review

Codex reviewed the completed Experiment 747 diff and found one real technical
bug: release suppression state was not one-shot, so a stale consumed default
could suppress later matching release events. The implementation now clears the
stored release identity when consuming a release and clears stale state before
terminal encoding on non-consumed press/repeat paths.

Codex re-reviewed the fixed diff and reported no remaining blocking technical
issues. The review confirmed ordinary bindings consume even when unperformed,
performable bindings fall through when unperformed, unsupported query-only
command-G remains query-visible but undispatched, stale-release behavior is
covered by tests, and the ABI coverage is representative.
