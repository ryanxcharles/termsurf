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

# Experiment 677: Surface Draw Wakeup

## Description

Experiment 676 made no-command surfaces use Roastty's default-shell resolution.
The next surface lifecycle gap is draw/wakeup signaling. The upstream embedded
ABI exposes `ghostty_surface_draw(surface)`, and Roastty's ABI inventory already
lists the renamed `roastty_surface_draw`, but the Roastty header and library do
not implement it yet.

This experiment adds the `roastty_surface_draw(surface)` ABI as a narrow
renderer-wakeup slice. Roastty does not yet have the full renderer-thread
machinery that upstream `Surface.draw()` uses, so this slice maps draw requests
onto the behavior the current library can represent: mark the surface as needing
render and invoke the app runtime `wakeup_cb` when the live surface still has an
attached app with a wakeup callback.

This experiment does not implement renderer frame drawing, renderer mailbox
messages, Metal renderer integration, refresh callbacks, frontend presentation,
or animation scheduling.

## Changes

- `roastty/include/roastty.h`
  - Add `ROASTTY_API void roastty_surface_draw(roastty_surface_t);` alongside
    the other surface lifecycle functions.
- `roastty/src/lib.rs`
  - Add `roastty_surface_draw(surface)`.
  - Null surfaces are a no-op.
  - For a live surface, set `surface.dirty = true`.
  - If `surface.app` is non-null and the app runtime has `wakeup_cb`, invoke it
    with runtime userdata.
  - If the surface has been detached by `roastty_app_free`, keep the call a
    no-op beyond marking the live surface dirty; do not dereference the old app
    pointer.
  - Keep `roastty_surface_render_state_update` as the operation that clears
    dirty state after a successful snapshot.
  - Add tests:
    - null draw is a no-op;
    - drawing a live surface marks `roastty_surface_needs_render(surface)`;
    - drawing a live surface invokes `wakeup_cb` with app userdata;
    - drawing a live surface twice invokes `wakeup_cb` twice, even when the
      surface is already dirty;
    - drawing a detached surface marks it dirty without invoking a wakeup;
    - successful render-state update still clears a draw-requested dirty flag
      when a worker exists. Use `os::pty::PTY_COMMAND_LOCK` for this
      subprocess-backed test.
- `roastty/tests/abi_harness.c`
  - Exercise `roastty_surface_draw(surface)` through the C header and assert
    `roastty_surface_needs_render(surface)` becomes true for the existing
    skeleton surface.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/677-surface-draw-wakeup.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Design Review

**Result:** Approved after amendments.

Codex found two test-plan gaps. First, upstream `Surface.draw()` forces a render
on every call, so the plan should prove repeated draw calls invoke wakeup even
when the surface is already dirty. Second, the worker-backed dirty-clear test
will spawn a PTY process and should explicitly use the shared PTY command lock.

The design now includes a repeated-draw wakeup test and requires
`os::pty::PTY_COMMAND_LOCK` for the worker-backed render-state update test.

## Result

**Result:** Pass

Implemented `roastty_surface_draw(surface)` in the public C header and Rust ABI.
The function is null-safe, marks live surfaces dirty, and invokes the app
runtime `wakeup_cb` with runtime userdata while the surface is still attached to
an app. Detached live surfaces keep the dirty-state behavior without
dereferencing the cleared app pointer.

The Rust tests cover null draw calls, live dirty marking, wakeup userdata,
repeated wakeups while already dirty, detached-surface dirty marking without
wakeup, and render-state update clearing a draw-requested dirty flag when a
worker-backed snapshot succeeds. The C ABI harness now calls the exported draw
function through `roastty.h` and observes
`roastty_surface_needs_render(surface)` become true.

Verification passed:

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/677-surface-draw-wakeup.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Conclusion

Roastty now has the renamed surface draw ABI and a minimal wakeup bridge that
matches the current library architecture. This does not replace Ghostty's full
renderer thread or mailbox behavior yet; it establishes the external surface
draw entry point and preserves dirty-state semantics so later renderer slices
can attach to it.

## Completion Review

**Result:** Approved after documentation fixes.

Codex found no code, ABI/header, or test-reliability blockers. It confirmed the
draw ABI is declared in the public header, implemented as null-safe dirty+wakeup
behavior, and covered by tests for repeated wakeups, detached surfaces, and
worker-backed dirty clearing.

Codex did block the first completion-review pass because the experiment file was
missing `[review.result]` provenance and the README still showed
`Codex/Codex/-`. It also noted that the PTY/termio checklist should not claim
surface draw wakeup as a PTY behavior. The experiment file now records the
result review, the README tuple is updated to `Codex/Codex/Codex`, and the
PTY/termio checklist wording keeps draw wakeup out of that subsystem.
