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

# Experiment 752: Surface CLI Keybind Dispatch

## Description

Make CLI-loaded root keybinds executable through `roastty_surface_key`.
Experiments 749 through 751 made configured keybinds parseable, queryable at the
config level, and queryable at the surface level, but actual key dispatch still
only executes static default bindings. A configured binding such as
`--keybind=ctrl+x=text:hello` is reported as a binding by
`roastty_surface_key_is_binding`, but `roastty_surface_key` still falls through
to terminal encoding.

This experiment dispatches configured root keybinds before static defaults and
terminal encoding, reusing the existing `parse_binding_action` and
`perform_parsed_binding_action` pipeline. Configured matches are treated as
ordinary consumed bindings because Roastty does not yet parse configured keybind
flags or performability.

Unsupported configured actions remain non-consuming for this experiment but
still shadow static defaults. CLI keybind parsing currently stores any non-empty
action bytes and does not yet run action validation or emit diagnostics. Until
that validation exists, matching an unsupported configured action should bypass
static default dispatch and fall through to terminal encoding instead of
swallowing user input or executing the shadowed default.

This remains root-table only. It does not implement key tables, sequences,
`clear`, `unbind`, global/all prefixes, configured performable flags, config
file loading, action validation diagnostics, or chained bindings.

## Changes

- `roastty/src/lib.rs`
  - Add an app-level configured keybind lookup that returns the matching action
    bytes and a normalized release identity.
  - Make `Surface::key` check configured root keybinds after consumed-release
    suppression and before static default dispatch.
  - Dispatch configured keybind actions through the existing
    `parse_binding_action` and `perform_parsed_binding_action` helpers.
  - Treat supported configured actions as ordinary consumed bindings: return
    `true` and suppress the matching release even if the action callback or
    action preconditions make the action return `false`.
  - Preserve configured-over-static precedence. For example, `cmd+c=text:custom`
    should run the configured `text` action instead of the static command-C copy
    action.
  - Leave unsupported or malformed configured action strings non-consuming for
    this experiment, so they fall through to terminal encoding.
  - Preserve configured-over-static precedence even for unsupported configured
    actions. For example, `cmd+c=unknown_action` should bypass static command-C
    copy dispatch and fall through to terminal encoding.
  - Preserve static default dispatch behavior when no configured keybind trigger
    matches the event.
  - Preserve stale-release clearing on non-consumed press/repeat paths.
- `roastty/tests/abi_harness.c`
  - Add C coverage for a CLI-loaded configured key dispatch through
    `roastty_surface_key`, using a representative action such as `text:hello`.
  - Assert configured-over-static precedence with a binding such as
    `cmd+c=text:custom`.
  - Assert unsupported configured actions fall through rather than being
    consumed, and that an unsupported configured action overlapping command-C
    does not attempt the static copy action.
- Tests in `roastty/src/lib.rs`
  - Cover configured `text:` key dispatch writing decoded bytes to a child PTY.
  - Cover configured runtime/app actions dispatching through existing action
    callbacks.
  - Cover configured-over-static precedence, proving command-C can be rebound to
    `text:custom` and no copy action is attempted.
  - Cover ordinary-consumed behavior for supported configured actions whose
    callback returns false or whose action preconditions do not perform.
  - Cover release suppression after a consumed configured press, including a
    Unicode trigger whose release has the same physical key/modifiers but empty
    UTF-8.
  - Cover configured release suppression as one-shot, proving a second matching
    release after the consumed release falls through.
  - Cover stale configured release state clearing on non-consumed press/repeat
    paths before terminal encoding.
  - Cover unsupported configured actions falling through to terminal encoding
    and not suppressing releases.
  - Cover unsupported configured actions shadowing static defaults, such as
    `cmd+c=unknown_action` falling through without attempting command-C copy.
  - Keep existing `config_cli_keybind`, `config_key_is_binding`,
    `surface_key_is_binding`, `surface_key`, `surface_key_default`,
    `binding_action`, and ABI harness tests passing.

## Verification

- `cargo test -p roastty config_cli_keybind -- --nocapture --test-threads=1`
- `cargo test -p roastty config_key_is_binding -- --nocapture --test-threads=1`
- `cargo test -p roastty surface_key_is_binding -- --nocapture --test-threads=1`
- `cargo test -p roastty surface_key_default -- --nocapture --test-threads=1`
- `cargo test -p roastty surface_key -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness -- --nocapture`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the initial Experiment 752 design and found two blocking gaps.

First, unsupported configured-action precedence was ambiguous. The plan said
configured bindings dispatch before static defaults, but also said static
defaults should run when no configured action is dispatched. That left overlap
cases such as `cmd+c=unknown_action` unclear. The design now states that a
matching configured trigger shadows static defaults even when its action is not
supported; unsupported configured actions fall through to terminal encoding and
do not attempt the shadowed static default action.

Second, configured release suppression needed the same regression coverage that
Experiment 747 added for default dispatch. The design now requires one-shot
configured release suppression and stale-state clearing tests when a later
non-consumed press/repeat path falls through to terminal encoding.

Codex re-reviewed the corrected design and approved it for the plan commit with
no remaining blocking findings.

## Result

**Result:** Pass

Roastty now dispatches CLI-loaded root keybinds through `roastty_surface_key`.
The app-level configured keybind lookup reverse-scans app-owned keybinds,
returns the matching action bytes plus a normalized release identity, and lets
`Surface::key` try configured bindings before static defaults. Supported
configured actions reuse the existing `parse_binding_action` and
`perform_parsed_binding_action` pipeline.

Configured actions are ordinary consumed bindings for this experiment: a
supported configured action returns `true` and suppresses the matching release
even when the runtime callback returns false or the action preconditions do not
perform. Unsupported configured actions still shadow static defaults, but they
fall through to terminal encoding without setting release suppression.

Verification passed:

- `cargo test -p roastty config_cli_keybind -- --nocapture --test-threads=1`
- `cargo test -p roastty config_key_is_binding -- --nocapture --test-threads=1`
- `cargo test -p roastty surface_key_is_binding -- --nocapture --test-threads=1`
- `cargo test -p roastty surface_key_default -- --nocapture --test-threads=1`
- `cargo test -p roastty surface_key -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness -- --nocapture`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Completion Review

Codex reviewed the implementation diff and found no blocking issues. The review
confirmed that configured key dispatch runs before static defaults, supported
configured actions dispatch through the existing binding-action parser and
performer, and unsupported configured actions shadow static defaults before
falling through without release suppression.

The review also confirmed no regressions in default dispatch or query behavior:
static defaults still run when no configured trigger matches, surface keybind
queries remain consistent with Experiment 751, and release suppression continues
to use the existing one-shot identity path. The review found no must-fix test
gaps. It noted a non-blocking efficiency issue: `App::key_event_is_binding` now
clones action bytes through `key_event_binding` even for boolean queries.

## Conclusion

Experiment 752 moved configured root keybinds from query-only support into real
surface key dispatch for all actions already supported by Roastty's
binding-action parser. Remaining keybinding work includes action validation and
diagnostics, configured performable flags, key tables, sequences, config-file
loading, `clear`, `unbind`, global/all prefixes, and chained bindings.
