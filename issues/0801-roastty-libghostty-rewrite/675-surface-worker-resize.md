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

# Experiment 675: Surface Worker Resize

## Description

Experiment 674 exposed the PTY foreground process group through the surface ABI.
The next IO lifecycle gap is resize forwarding. `roastty_surface_set_size`
currently stores only the surface pixel size, while `TermioWorker` already has a
resize command that applies `TIOCSWINSZ` to the PTY. Upstream Ghostty's embedded
surface size setter updates surface size and queues resize notifications to the
PTY and renderer.

This experiment wires Roastty's existing surface size setter to the existing
worker resize command. It keeps renderer and font/grid calculation out of scope:
Roastty still lacks the renderer/font metrics needed to compute rows and columns
from pixels, so the worker resize uses the current `Surface::pty_size()`
fallbacks and any internally stored rows/columns.

This experiment does not implement configured shell policy beyond `/bin/sh`,
renderer wakeups, renderer resize/reflow, font metrics, frontend grid
calculation, or the broader draw/refresh lifecycle.

## Changes

- `roastty/src/lib.rs`
  - Add an internal `Surface::set_size(width, height)` helper, or equivalent,
    that:
    - updates `surface.size.width_px` and `surface.size.height_px`;
    - computes the current `Surface::pty_size()` after storing the new pixels;
    - if a `TermioWorker` is attached, queues `worker.resize_pty(size)`;
    - records any resize queue failure in `last_termio_error`, marks the surface
      dirty/process-exited consistently with existing worker error handling, and
      keeps the ABI returning `void`.
  - Update `roastty_surface_set_size` to call that helper and preserve null
    surface behavior as a no-op.
  - Keep the existing `roastty_surface_size` ABI unchanged.
  - Add surface tests:
    - setting size without a worker updates `roastty_surface_size`;
    - setting size after `roastty_surface_start` reaches the child PTY;
    - resize queue failure on an attached but stopped/disconnected worker
      records `last_termio_error`, sets process-exited state, and marks the
      surface dirty;
    - setting size after `roastty_app_free` detaches the worker remains a no-op
      beyond stored size updates.
- `roastty/src/os/pty.rs` and `roastty/src/termio.rs`
  - If needed for deterministic tests, add internal `PtyChild`/`Termio` access
    to the current PTY winsize using `TIOCGWINSZ`.
  - Keep this accessor internal to Roastty tests and surface plumbing; it is not
    a new C ABI.

For the worker resize test, avoid `SIGWINCH`/shell-trap timing. The test should
call `roastty_surface_set_size`, then poll the attached worker's current PTY
winsize through the internal Termio/Pty accessor until it matches the expected
size. Because the current public size setter has no font metrics or row/column
arguments, the test may set `surface.size.rows` and `surface.size.columns`
internally before calling `roastty_surface_set_size` to prove the worker
receives the current PTY grid size. Use `os::pty::PTY_COMMAND_LOCK` for
subprocess-backed tests.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/675-surface-worker-resize.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty os::pty`
- `cargo test -p roastty termio`
- `cargo test -p roastty surface`
- `git diff --check`

## Design Review

**Result:** Approved after amendments.

Codex found two gaps. First, a shell `SIGWINCH`/`stty size` fixture could miss
the resize if the trap is not installed before `roastty_surface_set_size`, and
could be delayed by shell job-control behavior. Second, the planned resize queue
failure semantics needed a focused test.

The design now avoids trap-dependent tests by polling the attached worker's PTY
winsize through an internal Termio/Pty accessor. It also requires a
disconnected-worker resize test that verifies `last_termio_error`,
process-exited state, and dirty/needs-render state match existing worker error
handling.

## Result

**Result:** Pass.

Roastty now forwards surface size changes to an attached Termio worker. The
internal PTY layer exposes the current winsize with `TIOCGWINSZ`, Termio
forwards that internally, and `roastty_surface_set_size` updates stored pixel
size before queueing `TermioWorker::resize_pty(surface.pty_size())` when a
worker is attached. Queue failure is mapped through the existing worker-error
state path, setting `last_termio_error`, process-exited state, and dirty state
while keeping the C ABI `void`.

Focused tests cover stored size updates without a worker, deterministic attached
worker PTY winsize forwarding, disconnected-worker resize error state, and
detached surfaces updating stored size without a worker. The worker resize test
polls the attached worker's PTY winsize directly instead of relying on a
`SIGWINCH` shell trap.

Verification passed:

- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty os::pty` — 17 passed, 0 failed
- `cargo test -p roastty termio` — 17 passed, 0 failed
- `cargo test -p roastty surface` — 34 passed, 0 failed
- `git diff --check`

## Conclusion

Surface size updates now reach the active PTY worker through the existing resize
command, so process-side terminal sizing is wired for running surfaces. The
remaining resize/frontend gaps are renderer wakeups, renderer/font grid
calculation, renderer reflow, and the broader draw/refresh lifecycle.

## Completion Review

**Result:** Approved after provenance and README wording fixes.

Codex found no implementation bugs in the staged result. It confirmed that the
direct `TIOCGWINSZ` accessors, `Surface::set_size` forwarding, async resize
polling test, disconnected-worker error test, and detached-surface behavior
match the amended design.

The review required two issue-record fixes: add result-review provenance and
completion-review recording, and reword the README checklist so configured shell
policy remains listed as missing while only surface worker resize forwarding is
marked done. Those fixes are now included.
