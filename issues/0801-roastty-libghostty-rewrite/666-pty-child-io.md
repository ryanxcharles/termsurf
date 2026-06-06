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

# Experiment 666: PTY Child IO

## Description

Experiment 665 can spawn a child process attached to a PTY. The next termio
building block is master-side IO on that child without introducing a persistent
background read thread or mailbox yet.

This experiment adds bounded, synchronous PTY child operations:

- mark the PTY master nonblocking;
- write bytes to the child through the PTY master;
- drain up to a caller-provided byte limit of currently available output into a
  caller-provided buffer;
- poll for readable/hangup readiness with a timeout;
- resize the PTY after spawn;
- check or wait for child exit.

This mirrors the pieces upstream Ghostty's `Exec` backend combines in its
read/write/resize paths, but keeps the threading and mailbox work for a later
experiment.

## Changes

- `roastty/src/os/pty.rs`
  - Add `PtyChild::set_nonblocking()` using `fcntl(F_GETFL/F_SETFL)` and
    `O_NONBLOCK` on the master fd.
  - Add `PtyChild::write(&[u8]) -> io::Result<usize>` using `libc::write` on the
    master fd. This is a single syscall: it may write fewer bytes than the input
    or return `WouldBlock` after nonblocking mode is enabled; it is not
    `write_all`.
  - Add `PtyChild::poll(timeout_ms: i32) -> io::Result<PtyReadiness>` where
    readiness has exact boolean fields: `readable` for `POLLIN`, `hangup` for
    `POLLHUP`, `error` for `POLLERR`, and `invalid` for `POLLNVAL`. A poll
    timeout returns all fields false.
  - Add
    `PtyChild::read_available(&mut Vec<u8>, max_bytes: usize) -> io::Result<PtyRead>`
    that reads in a loop until `max_bytes` have been read, `WouldBlock`, EOF, or
    error. `PtyRead` has exact fields: `bytes_read: usize` and `eof: bool`.
  - Treat `read == 0` as EOF. Treat PTY-master `EIO` as EOF as well, matching
    common POSIX PTY behavior after the slave/child exits.
  - Add `PtyChild::resize(size)` forwarding to `Pty::set_size`.
  - Add `PtyChild::try_wait()` forwarding to the child process.
  - Keep these APIs internal to `os::pty`; no C ABI or App integration in this
    experiment.
- Tests in `roastty/src/os/pty.rs`
  - Spawn a shell that waits for input, write through the master, drain output,
    and assert the round trip. Disable shell echo first, for example
    `stty -echo; IFS= read line; printf 'out:%s' "$line"`, so the test is not
    dependent on PTY echo or CRLF translation.
  - Verify `read_available` returns promptly after `set_nonblocking()` when no
    bytes are available.
  - Verify `resize` changes the PTY's reported winsize after spawn.
  - Verify `try_wait` reports `None` for a running child and `Some(status)`
    after exit.
  - Verify poll reports readable output and eventually hangup/EOF for a
    short-lived child without blocking indefinitely.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/666-pty-child-io.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty os::pty`
- `git diff --check`

## Design Review

**Result:** Approved after amendments.

Codex first found four concrete API and test gaps: `read_available` needed an
explicit byte bound and exact result fields, `PtyReadiness` needed precise
timeout/readable/hangup/error/invalid mappings plus EOF/EIO semantics, `write`
needed to be documented as a single syscall with partial/`WouldBlock` behavior,
and the shell round-trip test needed deterministic line-discipline setup.

The design now uses `read_available(&mut Vec<u8>, max_bytes)` with
`PtyRead { bytes_read, eof }`, defines `PtyReadiness` fields and timeout
behavior exactly, treats PTY-master `EIO` as EOF, documents `write` as a
single-syscall operation, and disables shell echo in the round-trip test. Codex
re-reviewed the amended design and approved it for plan commit and
implementation with no remaining blockers.

## Result

**Result:** Pass.

`PtyChild` now exposes bounded synchronous master-side IO primitives:
`set_nonblocking`, single-syscall `write`, `poll`, bounded `read_available`,
`resize`, and `try_wait`. `PtyReadiness` reports readable, hangup, error, and
invalid readiness bits, while `PtyRead` reports the number of bytes read and
whether EOF was reached. `read_available` stops at the requested byte limit,
`WouldBlock`, EOF, or PTY-master `EIO`.

Focused tests cover a shell round trip with echo disabled, prompt return from an
empty nonblocking read, resize after spawn, running/exited `try_wait`, and
readable output plus EOF for a short-lived child.

Verification passed:

- `cargo fmt -p roastty`
- `cargo test -p roastty os::pty` — 13 passed, 0 failed

## Conclusion

Roastty now has tested PTY child IO primitives for the future termio loop. The
remaining PTY/termio gap is coordinating these primitives in a persistent
read/write loop with quit signaling, terminal processing, process wait handling,
and App/surface integration.

## Completion Review

**Result:** Approved after test fix.

Codex found no bounded-read, readiness mapping, EOF/EIO, write, resize, or
implementation correctness issues. It found one test-stability issue:
`pty_child_try_wait_reports_running_then_exited` originally used
`/bin/sh -c "sleep 0.1"`, which could complete before `try_wait()` observed the
running state on a slow runner.

The test now spawns `/bin/sleep 1` directly before asserting `try_wait()`
returns `None`. Codex re-reviewed the corrected diff and approved it for result
commit with no remaining findings.

Final verification later exposed a parallel-test flake while multiple PTY tests
spawned controlling-terminal children at once. The spawning PTY tests now use a
test-only static mutex to serialize those subprocess cases, and
`cargo test -p roastty os::pty` was rerun successfully.
