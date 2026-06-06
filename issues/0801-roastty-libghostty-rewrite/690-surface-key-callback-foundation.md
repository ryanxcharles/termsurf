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

# Experiment 690: Surface Key Callback Foundation

## Description

Experiment 689 added the state-only surface mouse callback foundation. The next
remaining frontend surface input boundary is key dispatch: upstream exposes
`ghostty_surface_key(surface, event)` and
`ghostty_surface_key_is_binding(surface, event, flags)`.

Full upstream key behavior is broad. It routes key events through the app,
applies keymaps and key remaps, checks binding tables and trigger sequences,
dispatches actions, writes encoded key reports to the terminal PTY, and returns
whether the event was actually consumed. Roastty already has standalone
`roastty_key_event_t` and key encoder ABI support, plus the surface
key-translation-modifier query from Experiment 687, but it does not yet have the
binding/keymap/action dispatch machinery needed to faithfully implement the
surface callbacks.

This experiment adds a safe C ABI foundation for the two surface key callbacks.
The functions validate handles and store the latest key event on the surface so
future dispatch experiments have the boundary in place. They intentionally do
not yet write encoded key data to the PTY, trigger actions, inspect keybinding
tables, advance trigger sequences, or claim event consumption.

## Changes

- `roastty/include/roastty.h`
  - Add `typedef uint8_t roastty_keybind_flags_t;`, matching upstream's
    `input.Binding.Flags.C` shape.
  - Add public surface key callback functions next to
    `roastty_surface_key_translation_mods`:
    - `ROASTTY_API bool roastty_surface_key(roastty_surface_t, roastty_key_event_t);`
    - `ROASTTY_API bool roastty_surface_key_is_binding(roastty_surface_t, roastty_key_event_t, roastty_keybind_flags_t*);`
- `roastty/src/lib.rs`
  - Add `last_key_event: Option<key::KeyEvent>` to `Surface`.
  - Implement `roastty_surface_key`:
    - null surfaces, detached surfaces, and null or internally unconvertible key
      event handles return `false`;
    - valid calls clone and store the key event;
    - the return value is always `false` until full key dispatch exists, so the
      public ABI does not claim consumption for a stored-only event.
  - Implement `roastty_surface_key_is_binding`:
    - null surfaces, detached surfaces, and null or internally unconvertible key
      event handles return `false`;
    - non-null `flags` output is set to `0` before returning;
    - the return value is always `false` until keybinding tables and trigger
      state exist.
- `roastty/tests/abi_harness.c`
  - Assert `sizeof(roastty_keybind_flags_t) == 1`.
  - Exercise null and live surface key callback calls through `roastty.h`.
- Tests
  - Null and detached surfaces return `false`; binding flags are zeroed when
    supplied.
  - Null key event handles return `false` and leave surface state unchanged.
  - Valid `roastty_surface_key` stores an owned copy of the event and still
    returns `false`.
  - Mutating the source `roastty_key_event_t` after dispatch does not mutate the
    stored surface event.
  - `roastty_surface_key_is_binding` zeroes non-null flags, tolerates a null
    flags pointer, and returns `false`.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/690-surface-key-callback-foundation.md`
- `cargo fmt -p roastty`
- `cargo test -p roastty surface_key`
- `cargo test -p roastty key`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

**Result:** Approved with wording fixes.

Codex found no blocking design issues. It approved the state-only scope as an
incremental foundation before PTY writes, action dispatch, keybinding-table
checks, trigger sequences, and consumed-event returns exist. It also approved
`roastty_surface_key_is_binding` returning `false` while zeroing non-null flags,
because Roastty does not yet have binding tables or trigger state.

Codex asked for two implementation details to stay explicit. First, opaque
handles cannot validate arbitrary dangling pointers, so the design now says
"null or internally unconvertible" handles rather than promising general stale
pointer detection. Second, the test plan now includes a null flags-pointer case
for `roastty_surface_key_is_binding`, matching upstream's optional output
pointer shape.
