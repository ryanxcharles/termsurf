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

# Experiment 673: Surface TTY Name

## Description

Experiment 672 made surface worker launch honor command, cwd, env vars, and
initial input. The surface ABI still reports a placeholder TTY name. On macOS,
the PTY slave fd reports the real `/dev/ttys...` path before the slave is
closed, while the master fd does not reliably provide that name. The next slice
is to capture the slave tty name at PTY open time and expose it through the
existing `roastty_surface_tty_name(surface)` ABI when a surface has an attached
worker.

This experiment does not implement foreground PID, renderer wakeups, grid
resize, or the broader draw/refresh lifecycle.

## Changes

- `roastty/src/os/pty.rs`
  - Store an owned tty name on `Pty`, captured from the slave fd immediately
    after `openpty`.
  - Add `Pty::tty_name() -> Option<&str>` and `PtyChild::tty_name()`.
  - If tty-name capture fails or yields invalid UTF-8, keep opening the PTY and
    store `None`.
  - Keep the tty name available after `Pty::close_slave`.
  - Tests:
    - opening a PTY records a `/dev/` tty path;
    - a spawned `PtyChild` keeps the tty name after the slave fd is closed.
- `roastty/src/termio.rs`
  - Add `Termio::tty_name() -> Option<&str>` forwarding to the child.
  - Add a focused test that a spawned termio exposes a `/dev/` tty path.
- `roastty/src/lib.rs`
  - Update `roastty_surface_tty_name(surface)`:
    - return empty string for null surfaces, preserving current null behavior;
    - return an allocated/copy `RoasttyString` for the attached worker's tty
      name when available; never return a borrowed pointer into `Pty` or
      `TermioWorker` state;
    - fall back to the existing placeholder when an attached worker has no
      captured tty name;
    - keep the existing placeholder string for a non-null surface without an
      attached worker, so existing skeleton behavior remains stable until worker
      launch is universal.
  - Tests:
    - a surface without a worker still returns the placeholder;
    - after `roastty_surface_start`, `roastty_surface_tty_name` returns a
      sentinel string beginning with `/dev/`;
    - after `roastty_app_free` detaches and clears the worker, the live surface
      falls back to the placeholder.
    - use `os::pty::PTY_COMMAND_LOCK` for spawned child, termio, and surface
      subprocess tests.

## Design Review

**Result:** Approved after amendments.

Codex found three gaps: `roastty_surface_tty_name` needed to state that it
returns an allocated copy rather than a borrowed worker string, tty capture
failure needed fallback semantics, and subprocess tests needed to explicitly use
the shared PTY command lock.

The design now keeps PTY open successful when capture fails, stores `None` for
unavailable names, makes surface tty-name fall back to the placeholder when no
captured worker name exists, returns an allocated `RoasttyString` copy for real
tty names, and requires the shared PTY lock for spawned tests.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/673-surface-tty-name.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty os::pty`
- `cargo test -p roastty termio`
- `cargo test -p roastty surface`
- `git diff --check`

## Result

**Result:** Pass.

Roastty now captures the PTY slave tty name at `Pty::open` time, stores it as
owned Rust state, and keeps it available after the slave fd is closed during
child spawn. `Pty::tty_name`, `PtyChild::tty_name`, and `Termio::tty_name`
expose the captured name internally without making tty-name capture required for
PTY creation.

The surface ABI now returns an allocated `RoasttyString` copy of the attached
worker's tty name when available. Null surfaces still return an empty string,
and non-null surfaces without an attached worker, or without a captured worker
tty name, keep returning the existing `roastty-skeleton-tty` placeholder.

Focused tests cover PTY open capture, `PtyChild` retention after slave close,
Termio tty-name exposure, surface tty-name after `roastty_surface_start`, and
fallback behavior after `roastty_app_free` detaches the worker.

Verification passed:

- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty os::pty` — 16 passed, 0 failed
- `cargo test -p roastty termio` — 17 passed, 0 failed
- `cargo test -p roastty surface` — 27 passed, 0 failed
- `git diff --check`

## Conclusion

Surface worker launch now provides the real PTY tty path through
`roastty_surface_tty_name` once a worker exists, while preserving the skeleton
placeholder for dormant or detached surfaces. The remaining PTY/frontend launch
gaps are configured shell policy beyond `/bin/sh`, foreground PID, renderer
wakeups, terminal grid resize, and the broader draw/refresh lifecycle.

## Completion Review

**Result:** Approved after provenance and verification-record fixes.

Codex found no code bugs or regressions in the staged implementation. It
confirmed that the PTY tty name is captured from the slave before
`Pty::close_slave`, stored as owned state, forwarded through `PtyChild` and
`Termio`, and returned from `roastty_surface_tty_name` as an allocated
`RoasttyString` copy with the designed placeholder fallback.

The review required two issue-record fixes: add result-review provenance and the
completion-review section, and record the already-run `cargo fmt --check` and
`git diff --check` verification commands. Those fixes are now included.
