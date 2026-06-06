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

# Experiment 676: Surface Default Shell

## Description

Experiment 675 wired surface size changes to the active PTY worker. The next
remaining PTY launch gap is that a surface without an explicit command still
always starts `/bin/sh`. Upstream Ghostty finalization resolves a default shell
from the environment or passwd database before falling back to `sh`; Roastty
already has the macOS `os::passwd` and `os::desktop` helpers needed for a
bounded version of that policy.

This experiment replaces the hardcoded no-command `/bin/sh` surface launch with
a default-shell resolver:

1. If `SHELL` is set, non-empty, and the process was not launched from the
   desktop, use it.
2. Otherwise use the current user's passwd shell when present and non-empty.
3. Otherwise fall back to `/bin/sh`.

Explicit `RoasttySurfaceConfig.command` strings still run through
`/bin/sh -lc <command>` in this experiment. Full `Config.command` /
`initial-command` parsing, direct-command execution, shell integration
injection, login-shell argv decoration, and wait-after-command behavior remain
deferred.

## Changes

- `roastty/src/lib.rs`
  - Add an internal default-shell resolver, or equivalent:
    - a pure helper that accepts `SHELL`, passwd shell, and desktop-launch state
      for deterministic unit tests;
    - a runtime helper that reads `std::env::var_os("SHELL")`,
      `os::passwd::get().shell`, and `os::desktop::launched_from_desktop()`.
  - Update `Surface::start_termio` so the no-command branch starts the resolved
    shell program instead of hardcoded `/bin/sh`.
  - Preserve the explicit command branch as `/bin/sh -lc <command>`.
  - Add tests for the pure resolver:
    - non-desktop non-empty `SHELL` wins over passwd shell;
    - desktop launch ignores `SHELL` and uses passwd shell;
    - empty `SHELL` is ignored;
    - passwd shell is used when env shell is unavailable;
    - empty passwd shell is ignored;
    - missing env and passwd shell falls back to `/bin/sh`.
  - Add a surface integration test that temporarily sets `SHELL` to an
    executable test script, starts a no-command surface with no initial command,
    and verifies the script output reaches the render snapshot. Make this test
    deterministic by forcing the resolver's desktop-launch state to non-desktop
    through a test-only helper or override rather than depending on the host
    process launch source. Use a shared env lock plus
    `os::pty::PTY_COMMAND_LOCK` so global environment mutation and PTY
    subprocess setup are serialized. Use an RAII env guard that restores the
    previous `SHELL` value, or unsets it if it was previously absent, even when
    the test panics.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/676-surface-default-shell.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `git diff --check`

## Design Review

**Result:** Approved after amendments.

Codex found three gaps. First, a surface integration test that only sets `SHELL`
would be host-launch dependent because the runtime resolver ignores `SHELL` when
`os::desktop::launched_from_desktop()` is true. Second, global environment
mutation needs restoration, not only serialization. Third, the pure resolver
tests needed an empty-passwd-shell case.

The design now requires deterministic non-desktop resolver state for the surface
integration test, an RAII guard that restores or unsets `SHELL` on drop, and a
pure resolver test that empty passwd shell falls back instead of being used.
