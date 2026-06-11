# Experiment 121: Phase G — sequence control actions

## Description

Add the two sequence-control binding actions that do not require chained action
storage: `ignore` and `end_key_sequence`.

Upstream Ghostty treats `ignore` as a performed action with an ignored input
effect. When it appears inside an active sequence, the sequence is cleared and
queued leader bytes are dropped. Upstream also special-cases invalid sequence
keys when the active table/root catch-all binding is `ignore`: the invalid key
does not flush the queued sequence prefix to the pty.

`end_key_sequence` is a sequence leaf action that ends the active sequence and
flushes only the queued leader keys, excluding the key that triggered
`end_key_sequence`.

This experiment implements those two actions for Roastty's configured surface
key path. It keeps `chain=` out of scope because upstream chained bindings
require storing and dispatching multiple actions per leaf, plus reverse mapping
updates. It also keeps `roastty_app_key`, native keymaps/global shortcuts, and
broader default binding catalog work out of scope.

## Changes

- `roastty/src/lib.rs`
  - Extend `ParsedBindingAction` and canonical action formatting with:
    - `ignore`
    - `end_key_sequence`
  - Teach the config action parser to accept both actions with no parameter.
  - Dispatch `ignore` as a performed, consumed configured action for the surface
    path. When it runs while a sequence is active, drop queued leader bytes and
    emit the inactive/end `ROASTTY_ACTION_KEY_SEQUENCE` notification.
  - Dispatch `end_key_sequence` by ending the active sequence with a flush of
    queued leader bytes and consuming the triggering leaf key without encoding
    it.
  - Switch active sequence lookup from exact-only lookup to full set lookup, so
    sequence-local `catch_all` leaves are found before an input is considered an
    invalid sequence key.
  - Add a helper equivalent to upstream `catchAllIsIgnore()` for invalid
    active-sequence misses. It searches active key tables inner-most to
    outer-most, then the root configured set, matching upstream; it does not
    inspect the active sequence set because sequence-local `catch_all` entries
    are handled by normal active sequence lookup.
  - Use that helper in invalid active-sequence handling so an invalid
    non-modifier key drops the queued prefix and returns ignored when the active
    table/root catch-all binding action is `ignore`; otherwise keep the Exp119 /
    Exp120 behavior of flushing the prefix and encoding the current key.
  - Keep table/root sequence lookup order from Exp120 unchanged.
- Tests in `roastty/src/lib.rs`
  - Parse/canonicalize `ignore` and `end_key_sequence`; reject parameters.
  - `a=ignore` consumes the configured surface key without forwarding bytes or
    firing a runtime action.
  - `a>b=ignore` starts a sequence on `a`, then drops queued `a`, emits inactive
    sequence notification, and does not write `a` or `b`.
  - `a>catch_all=ignore` handles the second key through sequence-local
    `catch_all` lookup, drops queued `a`, emits inactive sequence notification,
    and does not write either key.
  - `a>escape=end_key_sequence` starts a sequence on `a`, then flushes only
    queued `a`, emits inactive sequence notification, and does not encode
    `escape`.
  - While `a>b=quit` is active, a root `catch_all=ignore` causes invalid `x` to
    drop queued `a`, emit inactive notification, and not encode `a` or `x`.
  - The same invalid-catch-all-ignore behavior works from an active table
    sequence, with table catch-all taking precedence over root catch-all.
  - Modifier keys during an active sequence still do not flush or clear the
    active sequence.
  - `roastty_app_key` ignores these sequence-control actions for now.

## Verification

- Run:
  - `cargo test -p roastty sequence`
  - `cargo test -p roastty key_table`
  - `cargo test -p roastty surface_key`
  - `cargo test -p roastty app_key`
  - `cargo test -p roastty parse_config_binding_action`
  - `cargo test -p roastty --test abi_harness`
  - `cargo test -p roastty -- --test-threads=1`
  - `cargo fmt`
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/121-sequence-control-actions.md issues/0802-libroastty-completion-and-mac-app/README.md`

## Design Review

**Reviewer:** Codex-native adversarial reviewer, fresh context
(`multi_agent_v1.spawn_agent`, agent `019eb7a5-6774-7b51-8da8-01f15a3e2dd9`)

**Initial verdict:** Changes required

**Required finding:** The original catch-all-ignore helper scope was wrong. It
said to inspect the active sequence set during invalid active-sequence fallback,
but upstream first performs normal active sequence lookup and only then searches
active key tables inner-to-outer followed by root for `catch_all=ignore`.

**Fix:** Updated the design so active sequence lookup uses full set lookup for
sequence-local `catch_all`, while the invalid-miss helper searches active tables
and then root. Added explicit test coverage for sequence-local, root, and table
`catch_all=ignore` behavior.

**Final verdict:** Approved

**Final findings:** None.
