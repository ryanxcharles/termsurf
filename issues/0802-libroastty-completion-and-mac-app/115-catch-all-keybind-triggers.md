# Experiment 115: Phase G — catch-all keybind triggers

## Description

Add upstream-compatible `catch_all` keybinding trigger support to Roastty's
single-trigger keybinding path.

Upstream Ghostty's `Binding.Trigger` has a third key variant, `catch_all`, and
`Binding.Set.getEvent` resolves key events in this order: physical key, UTF-8
single codepoint, unshifted codepoint, `catch_all` with the event's binding
modifiers, then bare `catch_all` when the event had modifiers. Roastty already
has the C ABI tag `ROASTTY_TRIGGER_CATCH_ALL`, but its config parser only
accepts physical/unicode trigger keys and its configured-binding matcher can
only compare exact physical/unicode triggers.

This experiment wires `catch_all` into the existing configured single-trigger
surface. It does not add multi-key sequences/chords, leader keys, key tables,
native keymaps, native global shortcut registration, or the remaining upstream
action catalog.

## Changes

- `roastty/src/lib.rs`
  - Add a small `catch_all_trigger(mods)` helper returning
    `ROASTTY_TRIGGER_CATCH_ALL`.
  - Extend `ConfigKeybindKey` and `config_keybind_key` so `catch_all` parses as
    a special trigger key, including modifier-prefixed forms such as
    `ctrl+catch_all`.
  - Keep invalid trigger behavior unchanged for empty trigger parts, duplicate
    modifiers, multiple keys, and unknown multi-character keys.
  - Replace the current exact configured-binding scan with explicit configured
    exact and configured catch-all lookup helpers that:
    - ignores release events;
    - check exact physical/unicode triggers first with the event's binding
      modifiers;
    - check a `catch_all` trigger with the event's binding modifiers;
    - if the event had binding modifiers, check bare `catch_all`;
    - preserve the current last-configured-wins behavior by scanning configured
      bindings in reverse for each lookup candidate.
  - Use that helper from `Config::key_event_is_binding`,
    `App::key_event_binding`, and the surface configured-binding paths so config
    queries, app-key dispatch, and surface-key dispatch share the same catch-all
    ordering.
  - Merge configured and built-in default priorities so configured exact
    bindings still override default exact bindings, but configured `catch_all`
    runs only after both configured exact and default exact lookup fail. This
    preserves upstream's "`catch_all` matches otherwise-unbound keys" behavior
    even though Roastty currently stores defaults outside the configured-binding
    vector.
- `roastty/tests/abi_harness.c`
  - Add C ABI coverage proving `roastty_config_trigger` can return
    `ROASTTY_TRIGGER_CATCH_ALL` for configured catch-all bindings without
    reading the inactive trigger union field.
  - Add C-side `roastty_config_key_is_binding_handle` /
    `roastty_surface_key_is_binding_handle` checks for bare and modified
    catch-all fallback.
- Tests in `roastty/src/lib.rs`
  - Parse `catch_all`, `ctrl+catch_all`, and modifier aliases.
  - Reject duplicate/multiple-key forms such as `catch_all+catch_all=ignore` and
    `catch_all+a=ignore`.
  - Prove direct configured bindings take priority over modified catch-all, and
    modified catch-all takes priority over bare catch-all.
  - Prove built-in exact default bindings take priority over configured
    catch-all fallback.
  - Prove bare catch-all matches unmodified unbound keys and is the fallback for
    modified unbound keys when no modifier-specific catch-all exists.
  - Prove release events never match catch-all bindings.

## Verification

- Add the unit and ABI-harness coverage above.
- Run:
  - `cargo test -p roastty catch_all`
  - `cargo test -p roastty parse_config_keybind`
  - `cargo test -p roastty app_key`
  - `cargo test -p roastty surface_key`
  - `cargo test -p roastty --test abi_harness`
  - `cargo test -p roastty -- --test-threads=1`
  - if the known foreground-PID or mouse-reporting races fail, rerun the failing
    test in isolation, then rerun `cargo test -p roastty -- --test-threads=1`
  - `cargo fmt`
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/115-catch-all-keybind-triggers.md issues/0802-libroastty-completion-and-mac-app/README.md`

## Design Review

Codex-native adversarial review ran in a fresh-context subagent
(`multi_agent_v1.spawn_agent`, agent `019eb71c-a4d6-7832-91b6-22e151d493cf`).

Initial verdict: **Changes required.** The reviewer found that the first design
would let configured `catch_all` bindings shadow built-in exact default
bindings. That is not faithful to upstream's "`catch_all` matches otherwise
unbound keys" semantics because Ghostty stores defaults and configured bindings
in one keybind set, where exact triggers are checked before `catch_all`.

Fix: the design now requires merged priority: configured exact bindings override
default exact bindings, but configured `catch_all` runs only after configured
exact and default exact lookup both fail. The verification plan now requires
coverage proving built-in exact default bindings take priority over configured
catch-all fallback.

Final verdict after re-review: **Approved.** The reviewer confirmed the prior
finding was resolved and reported no new required findings.

## Result

**Result:** Pass.

Implemented configured `catch_all` trigger support for Roastty's single-trigger
keybinding path. `catch_all` now parses through the existing keybind parser,
round-trips through the C-facing trigger tag, and participates in config,
app-key, and surface-key lookup.

The lookup order now follows upstream's trigger order for configured bindings:
physical key, UTF-8 single codepoint, unshifted codepoint, modifier-specific
`catch_all`, then bare `catch_all`. Because Roastty still stores built-in
defaults outside the configured binding vector, surface/config queries merge the
priority explicitly: configured exact bindings, built-in exact defaults, then
configured `catch_all`. That preserves upstream's "`catch_all` matches
otherwise-unbound keys" behavior, including the case where an exact default
binding is present but its `performable:` action does not perform.

Verification:

- `cargo test -p roastty catch_all` — passed: 5 unit tests, ABI harness filtered
  out.
- `cargo test -p roastty parse_config_keybind` — passed: 4 unit tests, ABI
  harness filtered out.
- `cargo test -p roastty app_key` — passed: 10 unit tests, ABI harness filtered
  out.
- `cargo test -p roastty surface_key` — passed: 52 unit tests, ABI harness
  filtered out.
- `cargo test -p roastty --test abi_harness` — passed with the existing 10
  enum-conversion warnings from the C harness.
- `cargo test -p roastty -- --test-threads=1` — passed: 4,640 unit tests, the
  ABI harness, and doc tests.
- `cargo fmt` — passed.
- `cargo fmt --check` — passed.
- `git diff --check` — passed.

## Conclusion

Configured single-trigger bindings now support upstream-compatible `catch_all`
fallbacks without letting those fallbacks shadow exact configured or built-in
default bindings. Remaining Phase G work still includes multi-key
sequences/chords, leader keys, key tables, native keymaps, keyboard-layout
reload, native global shortcut registration, broader global/all routing, and the
full upstream action/default binding catalog.

## Completion Review

Codex-native adversarial review ran in a fresh-context subagent
(`multi_agent_v1.spawn_agent`, agent `019eb72c-1402-7d50-9769-2a0b2fe6c14e`).

Verdict: **Approved.** The reviewer reported no required findings.

Optional finding: the reviewer saw the known
`surface_foreground_pid_reports_worker_foreground_pid_after_start` race during
its independent full serial verification; isolated rerun passed. No code change
was made for that unrelated race because the experiment verification plan
already documents the retry exception, and the final local full serial gate
passed.
