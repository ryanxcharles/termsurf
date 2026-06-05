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

# Experiment 556: the circular buffer core (CircBuf)

## Description

This experiment ports the **core** of upstream `datastruct/circ_buf.zig` —
`CircBuf`, a fixed-capacity circular (ring) buffer. It's the foundational
structure the terminal search subsystem builds on (the sliding window). The ring
stores `T` in a flat allocation with `head` (next write) / `tail` (oldest)
indices and a `full` flag. This experiment ports the core ring operations; the
iterator, auto-growing (`resize` / `ensureUnusedCapacity`), and the two-span
`getPtrSlice` accessor are deferred to keep the slice bounded. roastty homes its
data structures under `terminal::`, so this lands at `terminal::circ_buf`.

## Upstream behavior

`datastruct/circ_buf.zig` — `CircBuf(T, default)`:

- Fields: `storage: []T`, `head` (next write index), `tail` (oldest index),
  `full` (to disambiguate `head == tail` between empty and full).
- `init(size)`: allocate `size`, `@memset` to `default`, `head = tail = 0`,
  `full = (size == 0)`.
- `append(v)`: if `full` ⇒ `error.OutOfMemory`; else `storage[head] = v`,
  advance `head` (wrapping), `full = (head == tail)`.
- `appendAssumeCapacity(v)`: `assert(!full)` then the same.
- `clear()`: `head = tail = 0`, `full = false`.
- `empty()`: `!full and head == tail`. `capacity()`: `storage.len`.
- `len()`: `full` ⇒ `storage.len`; `head >= tail` ⇒ `head - tail`; else
  `storage.len - (tail - head)`.
- `deleteOldest(n)`: `assert(n <= storage.len)`; `n == 0` ⇒ return; reset the
  `n` oldest slots to `default`; `tail += min(len, n)` (wrapping);
  `full = false`.
- `first()` / `last()`: the oldest / newest value (or `null` if empty).

The upstream tests exercise append-to-full, length, wrap-around, `deleteOldest`,
and clear.

## Rust mapping (`roastty/src/terminal/circ_buf.rs`)

A `CircBuf<T>` (T `Copy`, with a stored `default` for fills / resets —
upstream's `default` is a comptime parameter). The ring math is ported verbatim
(wrapping with `%`); `first` / `last` are computed directly (the oldest is
`storage[tail]`, the newest is `storage[(head + len - 1) % len]`) rather than
via the deferred iterator:

```rust
//! A fixed-capacity circular buffer (port of the core of upstream `datastruct/circ_buf`).

/// Returned by `append` when the buffer is full (upstream returns `error.OutOfMemory`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Full;

/// A fixed-capacity ring buffer of `T` (upstream `datastruct.CircBuf`). `head` is the next
/// write index, `tail` the oldest; `full` disambiguates `head == tail`.
pub(crate) struct CircBuf<T: Copy> {
    storage: Vec<T>,
    head: usize,
    tail: usize,
    full: bool,
    default: T,
}

impl<T: Copy> CircBuf<T> {
    /// Allocate a ring of `size` elements, filled with `default` (upstream `init`).
    pub(crate) fn new(size: usize, default: T) -> Self {
        Self {
            storage: vec![default; size],
            head: 0,
            tail: 0,
            full: size == 0,
            default,
        }
    }

    /// Append a value; `Err(Full)` if the buffer is full (upstream `append`).
    pub(crate) fn append(&mut self, v: T) -> Result<(), Full> {
        if self.full {
            return Err(Full);
        }
        self.append_assume_capacity(v);
        Ok(())
    }

    /// Append a value, assuming there is capacity (upstream `appendAssumeCapacity`).
    pub(crate) fn append_assume_capacity(&mut self, v: T) {
        assert!(!self.full, "append to a full CircBuf");
        self.storage[self.head] = v;
        self.head += 1;
        if self.head >= self.storage.len() {
            self.head = 0;
        }
        self.full = self.head == self.tail;
    }

    /// Reset to empty (upstream `clear`).
    pub(crate) fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.full = false;
    }

    /// Whether the buffer holds no elements (upstream `empty`).
    pub(crate) fn is_empty(&self) -> bool {
        !self.full && self.head == self.tail
    }

    /// The total allocated capacity (upstream `capacity`).
    pub(crate) fn capacity(&self) -> usize {
        self.storage.len()
    }

    /// The number of used elements (upstream `len`).
    pub(crate) fn len(&self) -> usize {
        if self.full {
            return self.storage.len();
        }
        if self.head >= self.tail {
            self.head - self.tail
        } else {
            self.storage.len() - (self.tail - self.head)
        }
    }

    /// Delete the oldest `n` values, resetting their slots to `default` (upstream
    /// `deleteOldest`). Deletes everything if `n` exceeds the used length.
    pub(crate) fn delete_oldest(&mut self, n: usize) {
        assert!(n <= self.storage.len());
        if n == 0 {
            return;
        }
        let count = n.min(self.len());
        let cap = self.storage.len();
        for i in 0..count {
            let idx = (self.tail + i) % cap;
            self.storage[idx] = self.default;
        }
        self.tail = (self.tail + count) % cap;
        self.full = false;
    }

    /// The oldest value, or `None` if there are no elements (upstream `first`).
    pub(crate) fn first(&self) -> Option<&T> {
        // Guard on `len() == 0`, not `is_empty()`: a zero-capacity buffer has `full == true`
        // (so `is_empty()` is false) yet `len()` is 0, and upstream's iterator returns `null`.
        if self.len() == 0 {
            return None;
        }
        Some(&self.storage[self.tail])
    }

    /// The newest value, or `None` if there are no elements (upstream `last`).
    pub(crate) fn last(&self) -> Option<&T> {
        if self.len() == 0 {
            return None;
        }
        let cap = self.storage.len();
        Some(&self.storage[(self.head + cap - 1) % cap])
    }
}
```

`append_assume_capacity` / `len` / `delete_oldest` reproduce the upstream index
math verbatim. `delete_oldest` resets the `min(len, n)` actually-used oldest
slots to `default` and advances `tail` by the same — equivalent to upstream's
`getPtrSlice(0, n)` memset + `tail += min(len, n)` (the slots beyond `len` that
upstream also memsets are unused and already `default`, so the observable state
is identical). `first` / `last` compute the oldest / newest slot directly, the
faithful result of upstream's forward / reverse iterator's first element.

## Scope / faithfulness notes

- **Ported (bridged)**: the `CircBuf` core — `new` (`init`), `append`,
  `append_assume_capacity`, `clear`, `is_empty` (`empty`), `capacity`, `len`,
  `delete_oldest` (`deleteOldest`), `first`, `last`.
- **Faithful**: the `head` / `tail` / `full` ring representation; the wrapping
  append; the full-vs-empty disambiguation; the three-case `len`;
  `delete_oldest`'s reset-to-default + tail advance; `first` / `last` ends.
- **Faithful adaptation**: the comptime `CircBuf(T, default)` →
  `CircBuf<T: Copy>` with a stored runtime `default`; `[]T` + allocator →
  `Vec<T>`; `append`'s `error.OutOfMemory` (when full) → an `Err(Full)`; `first`
  / `last` computed directly (the deferred iterator's first element).
- **Deferred**: the `Iterator` (forward / reverse, `seekBy` / `reset`);
  auto-growing (`resize` / `ensureUnusedCapacity`); `getPtrSlice` (the two
  contiguous spans) and the `appendSliceAssumeCapacity` built on it — the next
  CircBuf slices, needed by the search subsystem.
- No C ABI/header/ABI-inventory change (internal Rust). New `terminal::circ_buf`
  module.

## Changes

1. `roastty/src/terminal/circ_buf.rs` (new): `Full`, `CircBuf` (the core methods
   above).
2. `roastty/src/terminal/mod.rs`: add `#[allow(dead_code)] mod circ_buf;`.
3. Tests (in `circ_buf.rs`):
   - **append to full**: a `CircBuf::new(3, 0u8)` is empty (`len 0`,
     `is_empty`); appending `1, 2, 3` fills it (`len 3`, `!is_empty`); a 4th
     `append` ⇒ `Err(Full)`; `first` ⇒ `1`, `last` ⇒ `3`.
   - **delete + wrap**: from `[1, 2, 3]`, `delete_oldest(1)` ⇒ `len 2`, `first`
     ⇒ `2`; then `append(4)` (head wraps) ⇒ `len 3`, `first` ⇒ `2`, `last` ⇒ `4`
     (verifying wrap-around); `delete_oldest(10)` deletes everything
     (`is_empty`, `first`/`last` ⇒ `None`).
   - **clear**: after `clear()`, `is_empty`, `len 0`; the buffer is reusable.
   - **capacity / zero-size**: `capacity()` is the allocated size;
     `CircBuf::new(0, 0u8)` is `full` (so `append` ⇒ `Err(Full)`) and `len 0`,
     and `first()` / `last()` are `None` (the `len() == 0` guard — `is_empty()`
     is false for a zero-capacity buffer).
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty circ_buf
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/circ_buf.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `CircBuf` appends with wrapping (erroring when full), reports `len` /
  `capacity` / `is_empty`, deletes the oldest `n` (resetting to default and
  advancing `tail`), `clear`s, and reports `first` / `last` — faithful to
  `datastruct/circ_buf.zig`'s core;
- the tests pass (append-to-full + delete/wrap + clear + capacity/zero-size),
  and the existing tests still pass;
- the iterator, resize/grow, and `getPtrSlice` stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the ring math, the `len` cases, or the
`delete_oldest` behavior diverges from upstream, an unrelated item changes, or
any public C API/ABI changes.

## Design Review

Codex's first design review raised **one Required** finding, now fixed; the
corrected design was **re-reviewed and approved with no findings**.

- **`first` / `last` zero-capacity guard (Required, fixed)**: the design guarded
  with `is_empty()`, but a zero-capacity buffer has `full == true` (so
  `is_empty()` is false) while `len()` is `0` — upstream's iterator-based
  `first` / `last` return `null` there. The Rust version would have indexed
  `storage[0]` / taken `% 0`. Fixed by guarding on `len() == 0` (which covers
  both the ordinary-empty and zero-capacity cases), and a zero-size `first()` /
  `last() == None` assertion was added.

On re-review Codex confirmed the `len() == 0` guard matches upstream's
iterator-first behavior and the rest of the scoped core is sound (append/full,
three-case `len`, `delete_oldest`, runtime default, `Err(Full)`, and the
deferred iterator / resize / two-span pieces).

Review artifacts:

- Prompt: `logs/codex-review/20260604-d556-prompt.md` (design),
  `logs/codex-review/20260604-d556b-prompt.md` (design re-review)
- Result: `logs/codex-review/20260604-d556-last-message.md` (design),
  `logs/codex-review/20260604-d556b-last-message.md` (design re-review)

## Result

**Result:** Pass

`terminal::circ_buf::CircBuf<T>` was added: the `head` / `tail` / `full` ring
with `new` (filled with a runtime `default`), `append` (`Err(Full)` when full) /
`append_assume_capacity` (wrapping), `clear`, `is_empty`, `capacity`, the
three-case `len`, `delete_oldest` (reset the oldest slots to `default` + advance
`tail`), and `first` / `last` (guarded on `len() == 0`). The module is
registered in `terminal/mod.rs`. Four tests: append-to-full (with the
`Err(Full)` and `first`/`last` ends), delete-and-wrap (the tail advances, a
subsequent append wraps the head, and an over-long delete empties it),
clear-and-reuse, and the zero-capacity edge (full, not appendable,
`first`/`last` ⇒ `None`).

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3089 passed, 0 failed (four new tests; no
  regressions, up from 3085).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + terminal/circ_buf.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **one Nit** (no
Required or Optional findings): the doc had `## Result` but no `## Conclusion` —
fixed by adding the conclusion below. Codex confirmed the implementation matches
the approved core — append wraps and sets `full`, `len` uses the three upstream
cases, `delete_oldest` resets/advances the oldest range, and `first` / `last`
correctly guard on `len() == 0` (including zero-capacity buffers) — and that the
tests cover the important edge cases for this scoped slice.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r556-prompt.md` (result)
- Result: `logs/codex-review/20260604-r556-last-message.md` (result)

## Conclusion

`terminal::circ_buf::CircBuf<T>` — the core of a fixed-capacity ring buffer — is
faithfully ported from `datastruct/circ_buf.zig`. This is the foundational
structure the terminal search subsystem (the sliding window) builds on, and
roastty's second `datastruct/` port (after `CacheTable`). The design review
caught a real zero-capacity edge: `first` / `last` must guard on `len() == 0`
rather than `is_empty()`, since a zero-capacity buffer is `full`. The `Iterator`
(forward / reverse), auto-growing (`resize` / `ensureUnusedCapacity`), and the
two-span `getPtrSlice` (and the `appendSliceAssumeCapacity` built on it) are the
next CircBuf slices — needed before the search subsystem itself can be ported.
The objc/bundle-id helpers, the `home()` resolver, and config `loadDefaultFiles`
remain deferred pending roastty's naming decision; `background-image-opacity`
stays float-blocked.
