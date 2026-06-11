# Experiment 111: Phase G — configured binding consumption

## Description

Use the configured-keybind flag metadata from Experiment 110 in the surface key
runtime path. Upstream `Surface.keyEvent` distinguishes keybind actions from key
encoding with two binding flags:

- `unconsumed:` runs the action but still lets the key encode into the child
  process.
- `performable:` only consumes when the action actually performs; otherwise the
  key behaves as if no binding existed and encodes normally.

Roastty currently performs configured bindings and always consumes them. This
experiment ports the surface-local consumption decision for configured
single-key bindings. It does not implement multi-key sequences/chords, key
tables, global shortcut registration, all-surface action routing, app-scoped
`roastty_app_key`, native keymaps, or the full upstream binding table.

## Changes

- `roastty/src/lib.rs`
  - Update `Surface::dispatch_configured_binding` so it uses
    `ConfiguredBindingMatch.flags` after parsing and performing the action.
  - Return `None` to the existing `Surface::key` fallback path when a configured
    binding should not consume, so VT KAM and terminal encoding keep using the
    already-remapped event.
  - Preserve upstream-compatible behavior for this slice:
    - bindings with the consumed bit set consume after a parsed action, matching
      the current default;
    - `unconsumed:` bindings perform the action but fall through to normal key
      encoding;
    - `performable:` bindings consume only when the action returns performed;
    - `performable:` bindings whose action returns false fall through to normal
      key encoding;
    - `global:` and `all:` bindings remain consumed in the surface path, but
      their real app/all-surface dispatch semantics remain later work.
  - Keep release suppression only for consumed configured bindings, matching the
    existing `last_consumed_default_binding` mechanism.

## Verification

- Add surface-key tests for:
  - `unconsumed:` configured bindings perform the action and still write the key
    to the child PTY;
  - default configured bindings still consume and suppress key encoding;
  - `performable:` configured bindings with an unperformed action fall through
    to key encoding;
  - `performable:` configured bindings with a performed action consume and
    suppress key encoding;
  - `global:`/`all:` plus `unconsumed:` still consume in the current surface
    path.
- Run:
  - `cargo test -p roastty keybind`
  - `cargo test -p roastty surface_key`
  - `cargo test -p roastty -- --test-threads=1`
  - if the known foreground-PID race fails, rerun
    `cargo test -p roastty -- --test-threads=1 --skip surface_foreground_pid_reports_worker_foreground_pid_after_start`
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/111-configured-binding-consumption.md issues/0802-libroastty-completion-and-mac-app/README.md`

## Design Review

Codex-native adversarial review ran in a fresh-context subagent
(`multi_agent_v1.spawn_agent`, agent `019eb6c7-6a2c-7e72-8ac1-912d27b6f7df`).

**Verdict:** Approved.

Findings: none.

## Result

**Result:** Pass.

Implemented configured binding consumption in
`Surface::dispatch_configured_binding`. Configured bindings now use their stored
flag byte after the action is parsed and performed:

- default consumed bindings keep consuming and suppressing the matching release;
- `unconsumed:` bindings perform the action, then fall through to VT KAM and
  terminal key encoding;
- `performable:` bindings whose action returns false fall through as if no
  binding existed;
- `performable:` bindings whose action returns true consume;
- `global:` and `all:` bindings still consume in the current surface path even
  when combined with `unconsumed:` and unperformed `performable:`.

This keeps the experiment scoped to surface-local consumption. Real global
shortcut registration, all-surface action routing, app-scoped `roastty_app_key`,
sequences/chords, key tables, native keymaps, and the full upstream binding
table remain later work.

Verification:

- `cargo test -p roastty keybind` — pass: 20 unit tests passed; ABI harness
  filtered pass.
- `cargo test -p roastty surface_key` — pass: 50 unit tests passed; ABI harness
  filtered pass.
- `cargo test -p roastty -- --test-threads=1` — pass: 4621 unit tests passed,
  ABI harness passed with the known 10 enum-conversion warnings, doc tests
  passed.
- `cargo fmt --check` — pass.
- `git diff --check` — pass.
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/111-configured-binding-consumption.md issues/0802-libroastty-completion-and-mac-app/README.md`
  — pass after result formatting.

## Completion Review

Codex-native adversarial review ran in a fresh-context subagent
(`multi_agent_v1.spawn_agent`, agent `019eb6cf-8dc0-7093-a4cf-d1f07b784c6e`).

Initial verdict: **Changes required.**

Required finding: `global:` / `all:` bindings combined with `performable:` could
fall through when the action returned false, because the first implementation
returned `None` before enforcing global/all consumption.

Fix: `Surface::dispatch_configured_binding` now treats global/all as consumed
before applying the unperformed-`performable:` fallthrough, and the regression
test
`surface_key_configured_global_all_consume_even_when_performable_unperformed`
covers both `global:performable:unconsumed:` and `all:performable:unconsumed:`.

Re-review verdict: **Approved.** The reviewer independently verified the fix
with
`cargo test -p roastty surface_key_configured_global_all_consume_even_when`,
`cargo fmt --check`, and `git diff --check`, and reported no remaining required
findings.

## Conclusion

Surface configured bindings now honor the first useful runtime behavior carried
by upstream binding flags. This unblocks later app-level/global work from a
cleaner base: normal surface bindings already know when to consume, when to fall
through, and when an unperformed performable action should behave as absent.
