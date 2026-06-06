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

# Experiment 674: Surface Foreground PID

## Description

Experiment 673 exposed the surface PTY name through the worker-backed surface
ABI. The neighboring ABI, `roastty_surface_foreground_pid(surface)`, still
returns `0` for every surface. Upstream Ghostty returns the foreground process
group attached to the PTY, using `tcgetpgrp(master)` and falling back to `0`
when the information is unavailable.

This experiment exposes that same macOS behavior through Roastty's existing PTY
and Termio layers, then wires it into `roastty_surface_foreground_pid`. The API
contract is the current PTY foreground process group, not necessarily the
original child process id; the tests compare against the child id only for
simple spawn-time fixtures where the initial child is the session and process
group leader.

This experiment does not implement configured shell policy beyond `/bin/sh`,
renderer wakeups, terminal grid resize, or the broader draw/refresh lifecycle.

## Changes

- `roastty/src/os/pty.rs`
  - Add `PtyChild::foreground_pid() -> Option<u64>`.
  - Implement it with `libc::tcgetpgrp(self.master_fd())`; return `Some(pid)`
    only when the returned process group id is positive, and return `None` for
    errors or non-positive values.
  - Keep `PtyCommand` spawn behavior unchanged: `setsid()` plus `TIOCSCTTY`
    already make the child process group the controlling foreground process
    group for the PTY.
  - Add a focused subprocess test that a spawned `PtyChild` reports a positive
    foreground pid, and that it equals the child id for the simple child fixture
    created by this test.
- `roastty/src/termio.rs`
  - Add `Termio::foreground_pid() -> Option<u64>` forwarding to `PtyChild`.
  - Extend the existing accessor test, or add a focused test, to assert a
    spawned Termio reports a positive foreground pid matching the child id for
    the simple child fixture.
- `roastty/src/lib.rs`
  - Update `roastty_surface_foreground_pid(surface)`:
    - return `0` for null surfaces;
    - return the attached worker Termio foreground pid when available;
    - return `0` for non-null surfaces without an attached worker, preserving
      the current skeleton fallback.
  - Add surface tests:
    - a surface without a worker still returns `0`;
    - after `roastty_surface_start`, the surface returns a positive foreground
      pid matching the worker child id for the simple initial worker fixture;
    - after `roastty_app_free` detaches and clears the worker, the live surface
      returns `0`.
  - Use `os::pty::PTY_COMMAND_LOCK` for subprocess and surface worker tests.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/674-surface-foreground-pid.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty os::pty`
- `cargo test -p roastty termio`
- `cargo test -p roastty surface`
- `git diff --check`

## Design Review

**Result:** Approved after amendment.

Codex found one contract wording issue: the foreground PID ABI reports the
current foreground process group from `tcgetpgrp(master)`, which is not
guaranteed to equal the original child PID after shell job control changes the
foreground process group.

The design now states that the API contract is a positive foreground process
group id when available, with `None` internally and `0` at the C ABI when
unavailable. Tests may compare the value to `child.id()` only in deterministic
simple-child fixtures where the initial child is the session and process group
leader.

## Result

**Result:** Pass.

Roastty now reports the PTY foreground process group through the worker-backed
surface ABI. `PtyChild::foreground_pid` calls `tcgetpgrp(master_fd)` and returns
`Some(pid)` only for positive process group ids. `Termio::foreground_pid`
forwards that value internally. `roastty_surface_foreground_pid(surface)`
returns the attached worker foreground pid when available and keeps returning
`0` for null, dormant, or detached surfaces.

Focused tests cover PTY-level foreground pid reporting, Termio forwarding, a
surface without a worker returning `0`, a started surface returning the simple
worker fixture's foreground pid, and a live surface returning `0` after
`roastty_app_free` detaches and clears the worker.

Verification passed:

- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty os::pty` — 17 passed, 0 failed
- `cargo test -p roastty termio` — 17 passed, 0 failed
- `cargo test -p roastty surface` — 30 passed, 0 failed
- `git diff --check`

## Conclusion

Surface worker launch now exposes the real PTY foreground process group through
`roastty_surface_foreground_pid`, while preserving the Ghostty-style `0`
fallback when the value is unavailable. The remaining PTY/frontend launch gaps
are configured shell policy beyond `/bin/sh`, renderer wakeups, terminal grid
resize, and the broader draw/refresh lifecycle.

## Completion Review

**Result:** Approved after provenance fix.

Codex found no implementation bugs or test-scope regressions in the staged
result. It confirmed that the `tcgetpgrp(master_fd)` path, positive-pid `Option`
handling, C ABI `0` fallback, and simple-child equality tests match the amended
design. It also confirmed the README checklist update removing foreground PID
from the remaining PTY gaps is accurate.

The only result-review finding was missing provenance and completion-review
recording. The experiment frontmatter and README agent tuple now record the
result review, and this completion-review section records the review outcome.
