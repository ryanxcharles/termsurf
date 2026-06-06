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

# Experiment 679: Surface Display ID

## Description

Experiment 678 completed the adjacent surface refresh wakeup ABI. The next
narrow surface lifecycle gap is the macOS display ID setter:
`ghostty_surface_set_display_id(surface, display_id)`. Upstream exposes this as
a Darwin-only embedded ABI that pushes a `.macos_display_id` message into the
renderer thread mailbox and wakes the renderer.

Roastty does not yet have Ghostty's renderer thread, renderer mailbox, or Metal
renderer attachment. This experiment adds the renamed
`roastty_surface_set_display_id(surface, display_id)` ABI and stores the latest
display ID on the surface so later renderer-thread work has a stable state
source. It does not attempt to create renderer messages or wake renderer threads
that do not exist yet.

Roastty is macOS-only for this rewrite, and the current Roastty header does not
use upstream-style `__APPLE__` guards, so this experiment adds the function as a
normal Roastty ABI declaration.

## Changes

- `roastty/include/roastty.h`
  - Add
    `ROASTTY_API void roastty_surface_set_display_id(roastty_surface_t, uint32_t);`
    near the other surface setters.
- `roastty/src/lib.rs`
  - Add a `display_id: u32` field to `Surface`.
  - Initialize new surfaces with `display_id = 0`.
  - Add `roastty_surface_set_display_id(surface, display_id)`.
  - Null surfaces are a no-op.
  - Live attached and detached surfaces store the latest value.
  - Do not mark the surface dirty or invoke `wakeup_cb` in this experiment.
    Upstream display-ID delivery wakes the renderer thread, not the app runtime,
    and Roastty does not have that renderer path yet.
  - Add tests:
    - null display-ID set is a no-op;
    - a new surface starts with display ID `0` internally;
    - setting the display ID updates the stored surface value;
    - repeated sets keep the latest value;
    - setting after `roastty_app_free` still updates the detached live surface
      without dereferencing the cleared app pointer;
    - setting display ID does not mark the surface dirty and does not invoke app
      `wakeup_cb`.
- `roastty/tests/abi_harness.c`
  - Exercise `roastty_surface_set_display_id(surface, value)` through the C
    header on both null and live surfaces to prove the symbol exists and is
    null-safe.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/679-surface-display-id.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Design Review

**Result:** Approved.

Codex approved the scope as a storage-only interim mapping for upstream's Darwin
renderer-mailbox path while Roastty has no renderer thread or mailbox. It agreed
that setting the display ID should not mark the surface dirty or invoke app
`wakeup_cb`, because upstream wakes the renderer thread rather than the app
runtime, and display routing metadata should not be conflated with terminal
render-state changes.

Codex also confirmed the planned tests are sufficient for this slice: null
safety, default value, latest-value overwrite, detached update, no dirty/no
wakeup, and C ABI header/symbol coverage. The result documentation should record
this as display-ID storage only, not renderer mailbox or renderer wakeup parity.

## Result

**Result:** Pass

Implemented `roastty_surface_set_display_id(surface, display_id)` in the public
C header and Rust ABI. The function is null-safe and stores the latest display
ID on live surfaces, including detached surfaces whose app pointer has already
been cleared by `roastty_app_free`.

The implementation initializes new surfaces with display ID `0`, preserves the
latest value across repeated sets, and intentionally does not mark the surface
dirty or invoke app `wakeup_cb`. That keeps this slice aligned with upstream's
semantics: display-ID changes are renderer-routing metadata delivered through
the renderer mailbox, not terminal render-state dirtiness or app runtime wakeup.
Roastty does not have the renderer mailbox yet, so this experiment records only
the stored-state source for later renderer work.

The Rust tests cover null calls, default value, latest-value overwrite,
detached-surface update, and the no-dirty/no-wakeup behavior. The C ABI harness
calls `roastty_surface_set_display_id` through `roastty.h` on both null and live
surfaces to prove the symbol is exported and null-safe.

Verification passed:

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/679-surface-display-id.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Conclusion

Roastty now accepts the macOS display ID setter at the renamed ABI boundary and
keeps the latest display ID on each surface. Renderer-thread work still needs to
consume this state and deliver upstream-equivalent `.macos_display_id` messages
to the Metal renderer path.

## Completion Review

**Result:** Approved after provenance update.

Codex found no code issues. It confirmed that the implementation stores
`display_id` on `Surface`, initializes it to `0`, and only assigns the field in
`roastty_surface_set_display_id` without touching dirty state or app wakeup. It
also confirmed that the header declaration is exposed correctly and the Rust/C
tests cover the intended ABI surface and null safety.

Codex confirmed the result documentation avoids claiming renderer mailbox
parity: it states that Roastty does not have the renderer mailbox yet, and the
README checklist records display-ID storage as done while renderer display-ID
delivery remains missing. The first completion-review pass blocked only because
`[review.result]`, this completion-review section, and the README
`Codex/Codex/Codex` tuple had not yet been recorded.
