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

# Experiment 667: Synchronous Termio Pump

## Description

Experiment 666 gave Roastty the PTY child IO primitives needed by termio:
nonblocking reads and writes, polling, resizing, and child exit checks. The next
step is to coordinate those primitives with the terminal core without adding the
full background read thread, mailbox, renderer wakeup, or App/surface ABI yet.

This experiment adds a deterministic synchronous termio pump that owns a
`Terminal` and a `PtyChild`. One call to the pump polls the PTY, drains bounded
child output into `Terminal::next_slice`, collects any terminal-generated PTY
responses, and flushes pending writes back to the child.

The pump keeps all outgoing bytes in a pending write buffer. Caller input and
terminal responses both append to that buffer. Flush attempts are best-effort
and nonblocking: successfully written bytes are removed, `WouldBlock` leaves the
remaining bytes queued for a later pump, and hard IO errors are returned to the
caller. This avoids dropping partial terminal responses from APIs such as device
attributes.

This is intentionally not Ghostty's full `termio` runtime yet. It is the
synchronous foundation that later experiments can wrap in a read thread, quit
signal, mailbox, renderer notifications, and App/surface presentation.

## Changes

- `roastty/src/termio.rs`
  - Add an internal `Termio` struct owning:
    - a `terminal::Terminal`;
    - an `os::pty::PtyChild`;
    - a reusable child-output buffer;
    - a pending PTY write buffer.
  - Add a spawn constructor that accepts an explicit full `PtySize` (`rows`,
    `cols`, `width_px`, and `height_px`), initializes the terminal from `cols`
    and `rows`, opens a PTY child with the exact same `PtySize`, and marks the
    master fd nonblocking. Pixel dimensions are caller supplied, not inferred or
    silently zeroed.
  - Add `queue_write(&[u8])` to append caller input to the pending write buffer.
  - Add
    `pump_once(timeout_ms, max_read_bytes) -> Result<TermioPump, TermioError>`
    with these semantics:
    - call `PtyChild::poll(timeout_ms)`;
    - treat `POLLNVAL` / invalid readiness as a hard error;
    - when readable, hangup, or error readiness is reported, call
      `read_available` with the supplied byte bound;
    - feed newly read bytes into `Terminal::next_slice`;
    - copy `Terminal::pty_response()` into the pending write buffer and then
      call `Terminal::clear_pty_response()`;
    - flush pending writes using repeated `PtyChild::write` calls until all
      pending bytes are written, a write would block, or an error occurs;
    - call `PtyChild::try_wait` and report whether the child has exited.
  - Add `TermioPump` with exact fields:
    - `readiness: PtyReadiness`;
    - `bytes_read: usize`;
    - `eof: bool`;
    - `bytes_written: usize`;
    - `pending_write_bytes: usize`;
    - `child_exited: bool`.
  - Add `TermioError` variants for IO errors, terminal initialization errors,
    terminal stream errors, and invalid PTY readiness. `WouldBlock` during a
    nonblocking flush is not an error; it leaves bytes queued and is reflected
    in `pending_write_bytes`.
  - Add `resize_pty(size)` forwarding to `PtyChild::resize`. This experiment
    updates the OS PTY size only; terminal grid resizing remains out of scope
    until the terminal resize path is ported.
  - Add accessors used by future App/surface code and tests:
    - immutable and mutable terminal access;
    - child id;
    - pending write byte count.
  - Keep the module internal to the `roastty` crate. No C ABI or app integration
    is added in this experiment.
- `roastty/src/lib.rs`
  - Add the internal `termio` module.
- Tests in `roastty/src/termio.rs`
  - Serialize tests that spawn PTY subprocesses with the same static mutex
    pattern used by Experiment 666's `os::pty` tests. These tests create
    controlling-terminal children and should not race each other under parallel
    `cargo test`.
  - Spawn `/bin/sh -c "printf hello"` and assert that `pump_once` delivers
    `hello` to the terminal screen.
  - Spawn a shell with echo disabled that waits for one input line, call
    `queue_write(b\"hello\\n\")`, pump until output arrives, and assert the
    terminal screen contains `out:hello`.
  - Spawn a shell command that sets the PTY to raw/no-echo/noncanonical mode,
    emits the primary device attributes request (`ESC [ c`), reads the exact
    terminal response bytes from stdin, and prints a marker only if those bytes
    match Roastty's expected response. Assert that:
    - the marker reaches the terminal screen;
    - `pending_write_bytes` returns to zero after the flush;
    - the response was not dropped or left queued.
  - Verify `resize_pty` changes the PTY winsize using `ioctl(TIOCGWINSZ)` on the
    child master fd.
  - Verify `pump_once` reports child exit for a short-lived child.

## Design Review

**Result:** Approved after amendments.

Codex found four concrete plan gaps: the spawn constructor did not specify how
`PtySize` pixel dimensions are chosen, the device-attributes response test could
block in canonical PTY mode, `TermioPump`/`TermioError` fields were not defined,
and the tests did not carry forward the PTY subprocess serialization fix from
Experiment 666.

The design now requires callers to pass a full `PtySize`, makes invalid PTY
readiness a hard error, defines the exact pump result and error semantics,
requires subprocess tests to share a static mutex, and makes the
device-attributes test use raw/no-echo/noncanonical mode while asserting the
response reaches the child and leaves no queued bytes behind.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/667-synchronous-termio-pump.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty termio`
- `git diff --check`

## Result

**Result:** Pass.

Roastty now has an internal synchronous `Termio` pump. It owns a `Terminal` and
`PtyChild`, spawns a child with caller-supplied `PtySize`, marks the PTY master
nonblocking, queues caller input, drains bounded child output into
`Terminal::next_slice`, collects terminal-generated PTY responses, and flushes
all pending writes back to the child without dropping bytes on partial
nonblocking writes.

The pump result reports PTY readiness, bytes read, EOF, bytes written, pending
write byte count, and child exit. Invalid PTY readiness is a hard `TermioError`,
while `WouldBlock` during write flushing keeps bytes queued for a later pump.
Resize currently forwards to the OS PTY only; terminal grid resizing remains for
a later terminal resize experiment.

Focused tests cover child output reaching the terminal screen, caller input
reaching a shell and returning output, primary device-attributes responses
flowing from child output through the terminal and back to the child, PTY
resize, child exit reporting, and the basic internal accessors. PTY subprocess
tests are serialized with a static mutex to avoid controlling-terminal races.

Verification passed:

- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty os::pty` — 13 passed, 0 failed
- `cargo test -p roastty termio` — 6 passed, 0 failed
- `git diff --check`

## Conclusion

The PTY work now has a tested synchronous coordination layer between child IO
and the terminal stream. The remaining termio work is to wrap this foundation in
a persistent background read/write loop with quit signaling, process-wait
handling, mailbox/event delivery, renderer wakeup, and App/surface integration.

## Completion Review

**Result:** Approved after fixes.

Codex found no pump correctness blockers: full `PtySize`, bounded reads, invalid
readiness handling, response collection, partial-write preservation, resize
forwarding, and child-exit reporting matched the approved design. It found two
result-commit issues: termio subprocess tests used a module-local mutex instead
of sharing Experiment 666's PTY subprocess lock, and the issue provenance still
showed no result review.

The PTY subprocess lock is now shared from `os::pty` so full parallel test runs
do not race `os::pty` and `termio` controlling-terminal children against each
other. The experiment frontmatter and README agent tuple now record the result
review. Codex re-reviewed the fixes and approved the result commit with no
remaining blockers.
