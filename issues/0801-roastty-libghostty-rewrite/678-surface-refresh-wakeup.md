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

# Experiment 678: Surface Refresh Wakeup

## Description

Experiment 677 added `roastty_surface_draw(surface)` as the immediate
dirty+wakeup surface render request. The adjacent upstream embedded ABI also
exports `ghostty_surface_refresh(surface)`, documented as telling the surface
that it needs to schedule a render. In upstream Ghostty, embedded
`Surface.refresh()` calls the core surface `refreshCallback()`, and that
callback queues a render.

Roastty does not yet have Ghostty's renderer thread, render mailbox, or frame
pacing. This experiment adds the renamed `roastty_surface_refresh(surface)` ABI
as the scheduled-render counterpart to draw, mapped to the behavior the current
library can represent: mark the surface dirty and invoke the app runtime
`wakeup_cb` when the live surface still has an attached app with a wakeup
callback.

This experiment deliberately keeps draw and refresh behavior equivalent until a
later renderer-thread experiment can distinguish immediate frame requests from
scheduled refresh requests.

## Changes

- `roastty/include/roastty.h`
  - Add `ROASTTY_API void roastty_surface_refresh(roastty_surface_t);` alongside
    `roastty_surface_draw`.
- `roastty/src/lib.rs`
  - Add `roastty_surface_refresh(surface)`.
  - Null surfaces are a no-op.
  - For a live surface, set `surface.dirty = true`.
  - If `surface.app` is non-null and the app runtime has `wakeup_cb`, invoke it
    with runtime userdata.
  - If the surface has been detached by `roastty_app_free`, keep the call a
    no-op beyond marking the live surface dirty; do not dereference the old app
    pointer.
  - Share the implementation path with draw where possible so the two interim
    render-request APIs stay consistent until renderer pacing exists.
  - Add tests:
    - null refresh is a no-op;
    - refreshing a live surface marks `roastty_surface_needs_render(surface)`;
    - refreshing a live surface invokes `wakeup_cb` with app userdata;
    - refreshing a live surface twice invokes `wakeup_cb` twice, even when the
      surface is already dirty;
    - refreshing a detached surface marks it dirty without invoking a wakeup;
    - successful render-state update still clears a refresh-requested dirty flag
      when a worker exists. Use `os::pty::PTY_COMMAND_LOCK` for this
      subprocess-backed test.
- `roastty/tests/abi_harness.c`
  - Exercise `roastty_surface_refresh(surface)` through the C header and assert
    `roastty_surface_needs_render(surface)` becomes true for an existing
    skeleton surface.
  - Do not run the refresh check on a surface that `roastty_surface_draw` has
    already made dirty. Use a fresh surface or otherwise assert a false
    `roastty_surface_needs_render(surface)` precondition immediately before
    calling refresh.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/678-surface-refresh-wakeup.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Design Review

**Result:** Approved after amendment.

Codex found one ABI harness test-plan gap. Experiment 677 already added a draw
check that leaves its surface dirty, so testing refresh on the same surface
would not prove that refresh changed render state. The plan now requires the C
harness refresh check to use a fresh surface or assert a false
`roastty_surface_needs_render(surface)` precondition immediately before calling
`roastty_surface_refresh`.

Codex otherwise approved the scope and agreed that mapping refresh to the same
dirty+wakeup path as draw is a defensible interim behavior while Roastty lacks
Ghostty's renderer queue.

## Result

**Result:** Pass

Implemented `roastty_surface_refresh(surface)` in the public C header and Rust
ABI. The function is null-safe, marks live surfaces dirty, and invokes the app
runtime `wakeup_cb` with runtime userdata while the surface is still attached to
an app. Detached live surfaces keep the dirty-state behavior without
dereferencing the cleared app pointer.

The implementation shares the same internal render-request path as
`roastty_surface_draw(surface)`, preserving equivalent interim behavior until
Roastty has Ghostty's renderer thread, mailbox, and frame pacing. The Rust tests
cover null refresh calls, live dirty marking, wakeup userdata, repeated wakeups
while already dirty, detached-surface dirty marking without wakeup, and
render-state update clearing a refresh-requested dirty flag when a worker-backed
snapshot succeeds. The C ABI harness now calls the exported refresh function
through `roastty.h` on a fresh surface and observes
`roastty_surface_needs_render(surface)` become true.

Verification passed:

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/678-surface-refresh-wakeup.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Conclusion

Roastty now exposes both renamed embedded render-request ABIs:
`roastty_surface_refresh(surface)` for scheduled refresh requests and
`roastty_surface_draw(surface)` for immediate draw requests. For now both map to
the same dirty+wakeup bridge because the library has no renderer queue to encode
their timing difference. Later renderer-thread work can split the behavior
behind these stable ABI entry points.

## Completion Review

**Result:** Approved after provenance update.

Codex found no code, ABI/header, or test-reliability issues. It confirmed that
the public header declares `roastty_surface_refresh`, the implementation shares
the draw render-request path, and the Rust tests cover null refresh calls, dirty
marking, wakeup userdata, repeated wakeups, detached surfaces, and worker-backed
dirty clearing.

Codex also confirmed the C ABI harness avoids the design-review tautology by
creating a fresh `refresh_surface`, asserting it is not dirty, calling refresh,
and then asserting it is dirty. The first completion-review pass blocked only
because `[review.result]`, this completion-review section, and the README
`Codex/Codex/Codex` tuple had not yet been recorded.
