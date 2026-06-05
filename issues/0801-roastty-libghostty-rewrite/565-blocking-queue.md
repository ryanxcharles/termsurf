+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 565: the BlockingQueue (fixed-capacity SPSC ring)

## Description

This experiment ports upstream `datastruct/blocking_queue.zig` —
`BlockingQueue`, a fixed-capacity single-producer/single-consumer
message-passing queue for cross-thread communication. roastty's seventh
`datastruct/` port. It lands at `terminal::blocking_queue`. The synchronization
(a mutex guarding the ring plus a "not full" condition variable) maps directly
to Rust's `std::sync::{Mutex, Condvar}`.

## Upstream behavior

`datastruct/blocking_queue.zig` — `BlockingQueue(T, capacity)`. A fixed-size
ring of `capacity` slots with `write` / `read` / `len` cursors, a `mutex`, and a
`cond_not_full` condition variable (plus a `not_full_waiters` count). It is
deliberately SPSC and has **no blocking pop** (callers use an external notifier
— e.g. eventfd — for "not empty"); only the producer blocks, on "full".

- `Timeout`: `instant` (fail immediately), `forever` (wait until not full),
  `ns(u64)` (wait up to N nanoseconds).
- `push(value, timeout) -> Size`: lock; if `full()` (`len == capacity`), then
  per the timeout — `instant` returns `0`; `forever` waits on `cond_not_full`;
  `ns` does a timed wait, returning `0` on timeout. After waking, if **still**
  full, return `0`. Otherwise write `value` at `write`, advance `write`
  (wrapping), increment `len`, and return the new `len`. A return of `0` means
  the push failed.
- `pop() -> ?T`: lock; if `len == 0` return `null`; read at `read`, advance
  `read` (wrapping), decrement `len`; if `not_full_waiters > 0` signal
  `cond_not_full`; return the value.
- `drain() -> DrainIterator`: lock and hand back an iterator that `next()`s
  values without re-locking or signalling per item; on `deinit` it signals
  `cond_not_full` (if waiters) and unlocks. This avoids per-item lock/CV
  overhead when draining the whole queue each IO loop.
- `full()` (private, lock held): `len == capacity`.
- `create` / `destroy`: heap-allocate / free the queue.

The waiter accounting matters: `pop` / `drain.deinit` only signal when
`not_full_waiters > 0`, so a producer blocked in `push(forever)` is woken
exactly when space appears.

Upstream tests: (1) capacity 4 — `pop` on empty is `null`; four `instant` pushes
return `1, 2, 3, 4`; the fifth returns `0` (full); four pops return `1, 2, 3, 4`
then `null`; an empty `drain` yields nothing; a push then succeeds (returns
`1`). (2) capacity 1 — `instant` push returns `1`, the next `instant` returns
`0`; an `ns` (1000 ns) push also returns `0` (times out).

## Rust mapping (`roastty/src/terminal/blocking_queue.rs`)

The ring state lives in an `Inner<T, CAP>` guarded by a `Mutex`, with a sibling
`Condvar` for the not-full signal. `T` needs no bounds: slots are
`[Option<T>; CAP]` (`None` = the upstream `undefined`), built with
`std::array::from_fn`. Indices are `usize` (upstream's `Size = u32` is an
internal size-type optimization it explicitly defers; `usize` is behaviorally
identical at these capacities). `push` returns `usize` (the new `len`; `0` =
failed, exactly as upstream).

```rust
//! A fixed-capacity single-producer/single-consumer blocking queue (port of upstream
//! `datastruct/blocking_queue`).

use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

/// How long `push` waits when the queue is full (upstream `Timeout`).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Timeout {
    /// Fail immediately if full (upstream `.instant`).
    Instant,
    /// Wait until space is available (upstream `.forever`).
    Forever,
    /// Wait up to this many nanoseconds (upstream `.ns`).
    Ns(u64),
}

struct Inner<T, const CAP: usize> {
    data: [Option<T>; CAP],
    write: usize,
    read: usize,
    len: usize,
    not_full_waiters: usize,
}

/// A fixed-capacity SPSC blocking queue (upstream `BlockingQueue`). The producer blocks on a full
/// queue (per `Timeout`); there is no blocking pop (use an external "not empty" notifier).
pub(crate) struct BlockingQueue<T, const CAP: usize> {
    inner: Mutex<Inner<T, CAP>>,
    cond_not_full: Condvar,
}

impl<T, const CAP: usize> BlockingQueue<T, CAP> {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                data: std::array::from_fn(|_| None),
                write: 0,
                read: 0,
                len: 0,
                not_full_waiters: 0,
            }),
            cond_not_full: Condvar::new(),
        }
    }

    /// Push `value`, returning the new queue length (`0` = failed) (upstream `push`).
    pub(crate) fn push(&self, value: T, timeout: Timeout) -> usize {
        let mut inner = self.inner.lock().unwrap();

        if inner.len == CAP {
            match timeout {
                Timeout::Instant => return 0,
                Timeout::Forever => {
                    inner.not_full_waiters += 1;
                    inner = self.cond_not_full.wait(inner).unwrap();
                    inner.not_full_waiters -= 1;
                }
                Timeout::Ns(ns) => {
                    inner.not_full_waiters += 1;
                    let (guard, res) = self
                        .cond_not_full
                        .wait_timeout(inner, Duration::from_nanos(ns))
                        .unwrap();
                    inner = guard;
                    inner.not_full_waiters -= 1;
                    if res.timed_out() {
                        return 0;
                    }
                }
            }
            // Interrupted / spurious wake while still full: fail.
            if inner.len == CAP {
                return 0;
            }
        }

        let w = inner.write;
        inner.data[w] = Some(value);
        inner.write += 1;
        if inner.write >= CAP {
            inner.write -= CAP;
        }
        inner.len += 1;
        inner.len
    }

    /// Pop a value without blocking (upstream `pop`).
    pub(crate) fn pop(&self) -> Option<T> {
        let mut inner = self.inner.lock().unwrap();
        if inner.len == 0 {
            return None;
        }
        let value = Self::take_at_read(&mut inner);
        if inner.not_full_waiters > 0 {
            self.cond_not_full.notify_one();
        }
        Some(value)
    }

    /// Lock and return a draining iterator (upstream `drain`).
    pub(crate) fn drain(&self) -> Drain<'_, T, CAP> {
        Drain {
            guard: self.inner.lock().unwrap(),
            cond: &self.cond_not_full,
        }
    }

    /// Read the value at `read` and advance the cursor (lock held; caller guarantees `len > 0`,
    /// so the slot is always occupied — an empty slot here is an invariant violation).
    fn take_at_read(inner: &mut Inner<T, CAP>) -> T {
        let n = inner.read;
        inner.read += 1;
        if inner.read >= CAP {
            inner.read -= CAP;
        }
        inner.len -= 1;
        inner.data[n].take().expect("occupied slot")
    }
}

/// A draining iterator holding the queue lock (upstream `DrainIterator`).
pub(crate) struct Drain<'a, T, const CAP: usize> {
    guard: MutexGuard<'a, Inner<T, CAP>>,
    cond: &'a Condvar,
}

impl<T, const CAP: usize> Iterator for Drain<'_, T, CAP> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.guard.len == 0 {
            return None;
        }
        Some(BlockingQueue::<T, CAP>::take_at_read(&mut self.guard))
    }
}

impl<T, const CAP: usize> Drop for Drain<'_, T, CAP> {
    fn drop(&mut self) {
        // Signal a blocked producer (if any) on the way out (upstream `DrainIterator.deinit`).
        if self.guard.not_full_waiters > 0 {
            self.cond.notify_one();
        }
    }
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: `datastruct.BlockingQueue` →
  `terminal::blocking_queue::BlockingQueue` (`push`, `pop`, `drain`, the
  `Timeout` enum, and the `Drain` iterator).
- **Faithful**: the SPSC ring accounting (`write` / `read` / `len`, wrapping),
  the full-queue producer blocking with `instant` / `forever` / `ns` timeouts
  (including the post-wake "still full ⇒ fail" recheck), the `0`-means-failed
  `push` return, the non-blocking `pop`, and the drain-without-per-item-overhead
  (lock held until the iterator drops, signalling only then) are all reproduced.
  The `not_full_waiters` accounting (signal only when a producer waits) is kept
  exactly.
- **Faithful adaptation**: upstream's `std.Thread.Mutex` +
  `std.Thread.Condition` become `std::sync::{Mutex, Condvar}`; the ring state
  moves inside the `Mutex` (Rust guards data, not code) with a `take_at_read`
  helper shared by `pop` and `Drain::next`; slots are `[Option<T>; CAP]` (`None`
  = upstream `undefined`); `create` / `destroy` (manual heap alloc/free)
  disappear — Rust owns the queue (callers share it via `Arc` as needed); the
  `DrainIterator.deinit` becomes a `Drop` impl that signals then unlocks.
- **Divergence (documented)**: indices use `usize` rather than upstream's
  `Size = u32`, which upstream itself flags as a future size-type optimization;
  behavior is identical at these capacities. `push` returns `usize`.
- No C ABI/header/ABI-inventory change (internal Rust). Adds
  `terminal::blocking_queue`.

## Changes

1. `roastty/src/terminal/blocking_queue.rs` (new): `Timeout`, `Inner`,
   `BlockingQueue<T, CAP>`, `Drain` as above.
2. `roastty/src/terminal/mod.rs`: add `#[allow(dead_code)] mod blocking_queue;`
   (alphabetical).
3. Tests (in `blocking_queue.rs`), mirroring upstream plus the blocking path:
   - **basic push/pop** (capacity 4): empty `pop` is `None`; four `instant`
     pushes return `1, 2, 3, 4`; the fifth returns `0`; four pops return
     `1, 2, 3, 4` then `None`; an empty `drain` yields nothing; a later push
     returns `1`.
   - **timed push** (capacity 1): `instant` push returns `1`, the next `instant`
     returns `0`; an `ns(1000)` push returns `0` (times out).
   - **drain with values**: push three, `drain` yields them in order, and
     afterward the queue is empty and accepts pushes again.
   - **ring wrap-around**: interleave push/pop past `capacity` so `write`/`read`
     wrap, confirming FIFO order is preserved.
   - **forever blocks until space** (threaded): capacity 1, fill it; a spawned
     producer `push(_, Forever)` blocks; the main thread polls
     `not_full_waiters == 1` (with a deadline) to confirm the producer is
     actually registered as a waiter before `pop`ping — so the test
     deterministically exercises the condvar path — then the producer completes
     returning `1`.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty terminal::blocking_queue
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/blocking_queue.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `BlockingQueue` reproduces upstream's SPSC ring semantics — bounded FIFO with
  wrapping cursors, the `instant` / `forever` / `ns` producer-blocking timeouts
  (with the still-full recheck), the `0`-means-failed push return, non-blocking
  pop, and the lock-held `drain` that signals on drop — faithful to
  `datastruct/blocking_queue.zig`;
- the tests pass (basic / timed / drain-with-values / wrap-around / threaded
  forever-block), and the existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the ring accounting, the timeout/blocking behavior,
the waiter signalling, or the drain semantics diverge from upstream, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed the design and found **no Required findings**, with two Optionals
— both adopted:

- **Optional (adopted)**: `take_at_read` now returns `T` (via
  `.take().expect("occupied slot")`) rather than `Option<T>`, since it is only
  called after `len > 0`; `pop` / `Drain::next` wrap the result in `Some`,
  keeping `None` solely as the public empty-queue result.
- **Optional (adopted)**: the threaded `Forever` test polls
  `not_full_waiters == 1` (with a deadline) before popping, so it
  deterministically exercises the condvar/blocking path rather than racing past
  it.

Codex confirmed the `push` / `pop` / `drain` semantics, the `not_full_waiters`
accounting, the timeout handling (including the still-full recheck after a
spurious or timed wake), the ring wrapping, the `Mutex` / `Condvar` mapping, and
the `Drain` guard/drop shape are all faithful to upstream — including the
increment-before-wait / decrement-after-wait ordering matching Zig's `defer`
scope.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d565-prompt.md`
- Result: `logs/codex-review/20260604-d565-last-message.md`
