# Experiment 127: Phase G — surface all-key routing

## Description

Wire configured `all:` and `global:` keybindings reached through the surface key
path to upstream-style app-wide action dispatch.

Upstream `Surface.maybeHandleBinding` treats any leaf whose flags include
`global` or `all` as app-wide: app-scoped actions perform once on the app, while
surface-scoped actions perform on every surface. Roastty currently parses and
stores both flags, reports them through the keybinding query APIs, and consumes
matching surface keys, but `Surface::dispatch_configured_binding` still performs
the action only on the initiating surface. That leaves plain `all:` bindings
short of the upstream semantics and also makes focused surface delivery of
`global:` bindings less complete than upstream.

This experiment adds app-wide dispatch for configured direct and chained leaves
on the surface path when their flags include `all:` or `global:`. It does not
add native platform global shortcut registration; it only fixes the behavior
once a matching key event reaches Roastty's surface key path.

## Changes

- `roastty/src/lib.rs`
  - Add an internal app-wide configured-action dispatcher that can be called
    from a surface binding leaf.
  - For configured leaves whose flags include `ROASTTY_KEYBIND_FLAG_ALL` or
    `ROASTTY_KEYBIND_FLAG_GLOBAL`:
    - classify each action without freezing target-surface-local semantics;
    - dispatch app-scoped actions once to the app target;
    - keep surface-scoped action bytes raw until each target surface parses and
      performs them, so actions such as `new_split` without an explicit
      direction still resolve `auto` from the target surface's own geometry;
    - dispatch surface-scoped actions to every live surface owned by the app;
    - skip stale or detached surface registrations whose `surface.app` no longer
      matches the app handle;
    - avoid taking a second mutable reference to the initiating surface through
      the app's surface list.
  - Keep unprefixed and plain `unconsumed:` / `performable:` surface behavior
    unchanged.
  - Preserve existing app-key behavior: `roastty_app_key` still handles
    `global:` platform captures, focused app-scoped non-global leaves, and does
    not treat plain `all:` as a platform global shortcut.
  - Preserve parser behavior that rejects `all:` / `global:` trigger sequences.
- `roastty/src/lib.rs` tests
  - Add direct `all:` surface-key coverage proving a surface-scoped action fans
    out to multiple live surfaces.
  - Add direct `all:` coverage for an app-scoped action proving it dispatches
    once to the app target, not once per surface.
  - Add mixed `all:chain=` coverage proving app-scoped actions dispatch once
    while surface-scoped actions fan out to live surfaces in order.
  - Add `all:new_split` or `all:chain=new_split` coverage across differently
    sized surfaces proving each target surface resolves the implicit `auto`
    split direction independently.
  - Add `global:` surface-key coverage proving focused surface delivery also
    uses app-wide dispatch when the event reaches the surface path.
  - Add stale/detached surface coverage for `all:` fanout.
  - Keep or update the existing consumption tests so `all:` / `global:` still
    consume even with `unconsumed:` and even when a `performable:` action would
    be unperformed.

Out of scope:

- Native keymaps and keyboard-layout reload.
- Native global shortcut registration.
- App-key sequence/table ownership.
- Supporting `all:` or `global:` trigger sequences.
- Command-palette UI behavior.
- Full upstream default binding table/action catalog completion.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/127-surface-all-key-routing.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Run targeted tests:
  - `cargo test -p roastty surface_key_configured_global_all`
  - `cargo test -p roastty surface_key_all`
  - `cargo test -p roastty app_key`
  - `cargo test -p roastty key_sequence`
- Run full Roastty tests:
  - `cargo test -p roastty -- --test-threads=1`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run the same Prettier command with `--check`.

**Pass** = configured direct and chained `all:` / `global:` leaves reached
through `roastty_surface_key` consume the event and dispatch app-scoped actions
once plus surface-scoped actions across all live app surfaces, without
regressing target-surface-local parsing, app-key behavior, or sequence-prefix
rejection.

**Partial** = direct `all:` fanout works but chained or stale-surface behavior
needs a follow-up.

**Fail** = correct app-wide dispatch requires a larger app/surface ownership
redesign before it can be implemented safely.

## Design Review

**Reviewer:** Codex-native adversarial reviewer, fresh context
(`multi_agent_v1.spawn_agent`, agent `019eb836-cc5d-7181-ac49-345f5105ecff`)

**Initial verdict:** Changes required.

**Required finding:** The first design said to parse every action in an
`all:`/`global:` leaf up front. The reviewer pointed out that this would freeze
surface-local semantics for actions such as implicit `new_split:auto`, which
Roastty currently resolves from the parsing surface's geometry while upstream
keeps `auto` unresolved until each target surface performs the action.

**Fix:** The design now classifies actions without freezing target-surface-local
semantics, keeps surface-scoped action bytes raw until each target parses and
performs them, and requires `all:new_split` or `all:chain=new_split` coverage
across differently sized surfaces.

**Final verdict:** Approved. The reviewer reported no remaining required,
optional, or nit findings.
