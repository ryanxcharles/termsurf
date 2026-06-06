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

# Experiment 680: Surface Request Close

## Description

Experiment 679 added display-ID storage. The next narrow surface lifecycle gap
is `roastty_surface_request_close(surface)`, which already exists in the header
and Rust ABI but is currently a no-op.

Upstream embedded `ghostty_surface_request_close(surface)` calls core
`Surface.close()`. Core close then invokes the embedded runtime close callback
with the surface's current `needsConfirmQuit()` value. In Roastty, full
confirmation policy is not implemented yet and
`roastty_surface_needs_confirm_quit(surface)` currently returns `false`, but the
runtime callback already exists in `roastty_runtime_config_s` as
`close_surface_cb`.

This experiment makes `roastty_surface_request_close(surface)` forward close
requests to the attached app runtime callback using the current surface userdata
and current confirmation value. It does not free the surface, remove it from the
app registry, implement keybinding-triggered close, or implement full
confirm-close configuration policy.

## Changes

- `roastty/src/lib.rs`
  - Add an internal surface close-request helper.
  - Update `roastty_surface_request_close(surface)`:
    - null surfaces are a no-op;
    - detached surfaces are a no-op beyond preserving the live surface;
    - attached surfaces with no `close_surface_cb` are a no-op;
    - attached surfaces with `close_surface_cb` invoke it with
      `surface.userdata` and the current `roastty_surface_needs_confirm_quit`
      value, which is currently `false`;
    - do not free the surface or unregister it from the app.
  - Add tests:
    - null request close is a no-op;
    - request close invokes `close_surface_cb` with surface userdata;
    - the callback receives the current confirmation value (`false` for this
      slice);
    - request close leaves the surface registered/alive;
    - request close is a no-op when the runtime has no callback;
    - request close after `roastty_app_free` does not invoke the callback or
      dereference the cleared app pointer.
- `roastty/tests/abi_harness.c`
  - Track `close_surface_cb` calls and arguments.
  - Exercise `roastty_surface_request_close(surface)` through the C header and
    assert the callback receives the configured surface userdata and `false`.
  - Keep the existing null request-close call as a null-safety check.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/680-surface-request-close.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Design Review

**Result:** Approved.

Codex approved the mapping as narrow and faithful enough for Roastty's current
state. Upstream `ghostty_surface_request_close` calls core `Surface.close()`,
and core close forwards `needsConfirmQuit()` into the runtime close callback.
Because Roastty's current `roastty_surface_needs_confirm_quit` is still stubbed
to `false`, forwarding `false` is the right interim value.

Codex also approved the constrained scope: callback forwarding only, no
freeing/unregistering, no keybinding close path, and no full confirm-close
policy. The planned tests cover the important boundaries: null, no callback,
attached callback arguments, surface remains alive/registered, and detached app
safety.

## Result

**Result:** Pass

Implemented `roastty_surface_request_close(surface)` as callback forwarding
instead of a no-op. Null surfaces, detached surfaces, and attached surfaces
whose runtime has no `close_surface_cb` remain no-ops. Attached surfaces with a
close callback invoke it with the surface userdata and the current
`roastty_surface_needs_confirm_quit(surface)` value, which remains `false` in
this slice.

The implementation does not free or unregister the surface. That keeps request
close aligned with upstream's embedded model: the runtime callback starts the
close process, and ownership cleanup is still explicit through
`roastty_surface_free` in Roastty's current ABI.

The Rust tests cover null request close, callback arguments, the current
confirmation value, preserving the live registered surface after callback
delivery, no-callback no-op behavior, and detached-surface safety. The C ABI
harness now records the runtime close callback and asserts that
`roastty_surface_request_close(surface)` forwards the configured surface
userdata and `false`.

Verification passed:

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/680-surface-request-close.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Conclusion

Roastty now forwards explicit surface close requests to the embedded runtime
callback. Full confirm-close configuration, keybinding-triggered close, and
event/mailbox-driven close routing remain separate future slices.

## Completion Review

**Result:** Approved after provenance update.

Codex found no code issues. It confirmed that `roastty_surface_request_close`
resolves the live surface and forwards to `Surface::request_close`, and that the
helper calls `close_surface_cb(surface.userdata, surface.needs_confirm_quit())`
without freeing or unregistering the surface. It also confirmed that
`needs_confirm_quit` remains the intended `false` stub for this slice.

Codex confirmed the tests cover callback arguments, no-callback behavior,
detached-surface no-op behavior, and ownership preservation. It also confirmed
the result documentation accurately states callback forwarding only and leaves
full confirm-close policy for future work. The first completion-review pass
blocked only because `[review.result]`, this completion-review section, and the
README `Codex/Codex/Codex` tuple had not yet been recorded.
