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

# Experiment 686: Surface Inherited Config

## Description

Experiment 685 added the surface mouse-captured query. Another upstream surface
API that Roastty still lacks is
`ghostty_surface_inherited_config(surface, context)`, which returns the
configuration options to use when creating a child surface from an existing
surface.

Roastty's config system is still skeletal, but surfaces already store a context
and worker-backed terminals can expose current terminal PWD state. This
experiment exposes a conservative upstream-faithful inherited config for the
pieces Roastty can currently support: requested child context and current
worker-terminal PWD when available. Unsupported fields remain at their defaults.
It does not implement full upstream conditional config inheritance,
font/theme/keybind inheritance, platform view inheritance, launch command/env
inheritance, initial input inheritance, userdata inheritance, or config reload
semantics.

## Changes

- `roastty/include/roastty.h`
  - Add
    `ROASTTY_API roastty_surface_config_s roastty_surface_inherited_config(roastty_surface_t, roastty_surface_context_e);`
    next to `roastty_surface_app`.
- `roastty/src/lib.rs`
  - Implement `roastty_surface_inherited_config(surface, context)`:
    - null surface returns `roastty_surface_config_new()` with the requested
      context applied when the context value is valid;
    - worker-backed surfaces return the current terminal PWD as
      `working_directory` when the terminal has one;
    - no-worker surfaces, detached surfaces, or worker-backed surfaces without a
      PWD return a null `working_directory`;
    - the returned context is the requested context when valid, otherwise the
      surface's stored context;
    - command, environment variables, initial input, userdata, platform fields,
      scale factor, font size, and wait-after-command remain default values
      because upstream does not replay launch command/env/input into child
      surfaces and Roastty does not yet implement full inherited config policy.
  - Add tests:
    - null surface returns default config and preserves a valid requested
      context;
    - inherited config does not inherit command, env vars, initial input, or
      userdata from the parent surface;
    - no-worker and no-PWD surfaces return null working directory;
    - worker-backed surfaces return the current terminal PWD;
    - inherited config uses the requested child context;
    - invalid requested context falls back to the source surface context;
    - the returned PWD pointer remains stable across unrelated surface mutations
      while the source surface is alive.
- `roastty/tests/abi_harness.c`
  - Exercise null and live-surface `roastty_surface_inherited_config` through
    the public C header.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/686-surface-inherited-config.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Design Review

**Result:** Approved after amendments.

Codex found that the first design inherited too much parent launch config.
Upstream `newSurfaceOptions` does not replay command, environment variables,
initial input, or userdata into child surfaces, so the design now keeps those
fields at their defaults and adds verification that they are not inherited.

Codex also noted that upstream derives the inherited working directory from the
current terminal PWD, not the source surface's launch-time working directory.
The design now uses the current worker-terminal PWD when available and returns a
null working directory otherwise. Removing env inheritance also removes the
unnecessary env-array lifetime risk from the slice.

Codex approved the amended design in follow-up review and found no remaining
substantive design blockers.

## Result

**Result:** Pass.

Implemented `roastty_surface_inherited_config(surface, context)` as a
conservative inherited surface-config snapshot. Null surfaces return default
surface config with a valid requested context applied. Live surfaces return
default unsupported fields, use the requested valid child context, and fall back
to the source surface context for invalid context values.

Worker-backed surfaces now expose current terminal PWD through
`working_directory` when available. The returned pointer is borrowed from an
owned cache on the source surface and remains valid across unrelated surface
mutations while that source surface is alive. No-worker surfaces, worker-backed
surfaces without PWD, and unsupported launch fields return defaults: command,
env vars, initial input, userdata, platform, scale factor, font size, and
wait-after-command are not inherited.

The C header now declares the API, and the C ABI harness exercises null and live
no-worker calls through the public header. Rust tests cover null/default
behavior, unsupported launch-field defaults, valid requested context, invalid
context fallback, current worker-terminal PWD inheritance, PWD pointer
stability, detached surfaces ignoring worker PWD, and no-worker/no-PWD null
working-directory results.

Verification:

- `cargo fmt -p roastty`
- `cargo test -p roastty surface_text_bracketed_mode_wraps_paste_markers -- --nocapture`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

Note: an initial `cargo test -p roastty surface` run hit the existing PTY timing
flake in `surface_text_bracketed_mode_wraps_paste_markers`, which then poisoned
the shared PTY test lock. The exact failing test passed when rerun alone, and
the full `surface` filter passed cleanly afterward.

## Conclusion

Roastty now exposes the upstream surface inherited-config entry point for the
subset it can support faithfully today: requested context plus current
worker-terminal PWD. Full inherited config policy remains future work with the
larger config system: conditional config inheritance, font/theme/keybind
inheritance, platform view inheritance, and reload semantics are still missing.

## Completion Review

**Result:** Approved after fixes and workflow updates.

Codex first found that detached surfaces could still return a worker PWD,
contradicting the approved design. The implementation now checks
`self.app.is_null()` before reading worker PWD, clears the cached inherited
working directory on detach, and includes a regression test for a detached
surface with a manually attached worker PWD.

Codex then reviewed the fixed staged diff and found no remaining code, ABI,
pointer-lifetime, regression, or test blockers. It approved the code result and
blocked the result commit only until review provenance was recorded. This
section, the `[review.result]` frontmatter, and the README reviewer tuple are
the requested workflow updates.
