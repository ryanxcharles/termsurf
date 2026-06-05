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

# Experiment 560: small-buffer message data (MessageData)

## Description

This experiment ports upstream `datastruct/message_data.zig` — `MessageData`, a
small-buffer- optimization (SBO) union for thread messaging. A message's payload
is held inline (a fixed small array) when it fits, or as a borrowed "stable"
slice (e.g. `const` data passed through), or heap-allocated as a last resort. It
lets the IPC / thread-message layer avoid allocation for the common small case.
roastty homes its data structures under `terminal::`, so this lands at
`terminal::message_data`.

## Upstream behavior

`datastruct/message_data.zig` — `MessageData(Elem, small_size)`:

```zig
return union(enum) {
    pub const Small = struct {
        pub const Max = small_size;
        data: [Max]Elem = undefined,
        len: IntFittingRange(0, small_size) = 0,
    };
    pub const Alloc = struct { alloc: Allocator, data: []Elem };
    pub const Stable = []const Elem;

    small: Small,
    stable: Stable,
    alloc: Alloc,

    /// Fit into `small` if possible, else allocate. (Never produces `stable`.)
    pub fn init(alloc, data: []const Elem) !Self {
        if (data.len <= Small.Max) {
            var buf: Small.Array = undefined;
            @memcpy(buf[0..data.len], data);
            return .{ .small = .{ .data = buf, .len = @intCast(data.len) } };
        }
        const buf = try alloc.dupe(Elem, data);
        return .{ .alloc = .{ .alloc = alloc, .data = buf } };
    }

    pub fn deinit(self) void { switch (self) { .small, .stable => {}, .alloc => |v| v.alloc.free(v.data) } }

    pub fn slice(self: *const Self) []const Elem {
        return switch (self.*) {
            .small => |*v| v.data[0..v.len],
            .stable => |v| v,
            .alloc => |v| v.data,
        };
    }
};
```

- A union of `small` (an inline `[small_size]Elem` + a length), `stable` (a
  borrowed `[]const Elem`), or `alloc` (an owned, allocator-freed `[]Elem`).
- `init(data)`: if `data` fits in `small_size`, copy it inline; otherwise `dupe`
  (allocate). It never produces `stable` (the doc: "can't and will never detect
  stable pointers").
- `deinit`: frees only the `alloc` case. `slice()`: a `const` view of whichever
  variant holds the data.

The upstream tests: `init` of `"hello!"` (≤ 10) ⇒ `small`; `init` of a 700-byte
string (> 10) ⇒ `alloc`; `init` of a 500-byte string into a `small_size = 500`
data ⇒ `small`.

## Rust mapping (`roastty/src/terminal/message_data.rs`)

A const-generic enum; `Drop` (the owned `Vec` frees itself) replaces `deinit`;
the borrowed `stable` variant carries a lifetime:

```rust
//! A small-buffer-optimization message payload (port of upstream `datastruct/message_data`).

/// A message payload held inline (`Small`), borrowed (`Stable`), or heap-allocated (`Alloc`)
/// (upstream `datastruct.MessageData`). `Small` avoids allocation when the payload fits.
pub(crate) enum MessageData<'a, T: Copy + Default, const SMALL: usize> {
    /// The payload copied into a fixed inline array; only `data[..len]` is meaningful.
    Small { data: [T; SMALL], len: usize },
    /// A borrowed "stable" slice passed through directly (e.g. `const` data).
    Stable(&'a [T]),
    /// An owned, heap-allocated payload (freed on drop).
    Alloc(Vec<T>),
}

impl<'a, T: Copy + Default, const SMALL: usize> MessageData<'a, T, SMALL> {
    /// Build a message from `data`, fitting it inline when it fits, else allocating (upstream
    /// `init`). Never produces `Stable`.
    pub(crate) fn init(data: &[T]) -> MessageData<'static, T, SMALL> {
        if data.len() <= SMALL {
            let mut buf = [T::default(); SMALL];
            buf[..data.len()].copy_from_slice(data);
            MessageData::Small {
                data: buf,
                len: data.len(),
            }
        } else {
            MessageData::Alloc(data.to_vec())
        }
    }

    /// Wrap a borrowed "stable" slice (the `Stable` variant; `init` never produces this).
    pub(crate) fn stable(data: &'a [T]) -> Self {
        MessageData::Stable(data)
    }

    /// A read-only view of the payload (upstream `slice`).
    pub(crate) fn slice(&self) -> &[T] {
        match self {
            MessageData::Small { data, len } => &data[..*len],
            MessageData::Stable(s) => s,
            MessageData::Alloc(v) => v,
        }
    }
}
```

`init` performs the SBO: `≤ SMALL` ⇒ copy into the inline `[T; SMALL]` (a
`T: Copy + Default` buffer — the `Default` fills the unused tail that upstream
leaves `undefined`, and only `data[..len]` is read); otherwise `Vec`. `init`
returns `MessageData<'static, ...>` because the `Small` / `Alloc` it produces
borrow nothing. `Drop` frees the `Vec` (the `alloc` case); `Small` / `Stable`
need no cleanup — the faithful shape of `deinit`. `slice` is the
variant-agnostic view.

## Scope / faithfulness notes

- **Ported (bridged)**: `datastruct.MessageData` (the `Small` / `Stable` /
  `Alloc` union) → `terminal::message_data::MessageData`; `init` / `slice` /
  `deinit` → `init` / `slice` / `Drop`.
- **Faithful**: the SBO in `init` (inline when `data.len() <= SMALL`, else
  allocate; never `Stable`); the three variants; `slice`'s per-variant view;
  only the allocated case frees.
- **Faithful adaptation**: the Zig `union(enum)` → a Rust `enum`;
  `[Max]Elem = undefined` → a `[T; SMALL]` filled with `T::default()` (only
  `[..len]` read; adds a `Default` bound for the array init);
  `IntFittingRange(0, small_size)` length type → `usize` (negligible — the SBO
  is about avoiding allocation, not minimizing the length field); the `Stable`
  borrow → a lifetime `'a`; `Alloc` + `deinit` → `Vec` + `Drop`; `init`'s
  `Allocator.Error` → infallible (Rust `Vec` allocation aborts on failure).
- **Deferred**: nothing — `message_data.zig` is fully ported.
- No C ABI/header/ABI-inventory change (internal Rust). New
  `terminal::message_data` module.

## Changes

1. `roastty/src/terminal/message_data.rs` (new): `MessageData` (`init` /
   `stable` / `slice`).
2. `roastty/src/terminal/mod.rs`: add `#[allow(dead_code)] mod message_data;`.
3. Tests (in `message_data.rs`), porting the upstream cases over
   `MessageData<u8, …>`:
   - **init small**: `init(b"hello!")` into `SMALL = 10` is `Small` and
     `slice() == b"hello!"`.
   - **init alloc**: `init` of a 700-byte payload into `SMALL = 10` is `Alloc`
     and `slice()` equals the input.
   - **small fits at the boundary**: `init` of a 500-byte payload into
     `SMALL = 500` is `Small` and `slice()` equals the input.
   - **stable**: `MessageData::stable(b"const")` is `Stable` and
     `slice() == b"const"`.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty message_data
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/message_data.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `MessageData::init` fits a payload inline when `data.len() <= SMALL` (else
  allocates), `stable` wraps a borrowed slice, and `slice` views any variant —
  faithful to `datastruct/message_data.zig`;
- the tests pass (small / alloc / boundary / stable), and the existing tests
  still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the SBO threshold, the variant semantics, or `slice`
diverges from upstream, an unrelated item changes, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. Codex confirmed the design is faithful to upstream and current
usage: `init` keeps the exact SBO threshold (`<= SMALL` inline, otherwise
allocated) and never produces `Stable`; `stable` is a faithful explicit
constructor for the borrowed variant; `slice` matches each union arm; and `Vec`
/ `Drop` is the right Rust replacement for the allocator-owned `Alloc` plus
`deinit`. The `T: Copy + Default` adaptation is acceptable (upstream's unused
small-buffer tail is undefined, Rust must initialize it, and only `[..len]` is
exposed; current upstream uses are `u8`, so the bound does not constrain the
intended port), and `MessageData<'static, …>` from `init` is sound since the
produced variants borrow nothing.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d560-prompt.md` (design)
- Result: `logs/codex-review/20260604-d560-last-message.md` (design)

## Result

**Result:** Pass

`terminal::message_data::MessageData<'a, T, SMALL>` was added: a const-generic
SBO enum (`Small { data, len }` / `Stable(&[T])` / `Alloc(Vec<T>)`) with `init`
(inline when `data.len() <= SMALL`, else `Vec`; never `Stable`; returns
`MessageData<'static, …>`), the `stable` constructor for the borrowed variant,
and `slice` (the per-variant view). `Drop` frees the `Alloc` `Vec`. The module
is registered in `terminal/mod.rs`. Four tests: `init` small (`b"hello!"` ⇒
`Small`), `init` large (700 bytes ⇒ `Alloc`), boundary (`500` into `SMALL = 500`
⇒ `Small`), and `stable` (borrowed `Stable`) — each checking the variant and
`slice()`.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3108 passed, 0 failed (four new tests; no
  regressions, up from 3104).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + terminal/message_data.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **one Nit** (no
Required or Optional findings): the doc had `## Result` but no `## Conclusion` —
fixed by adding the conclusion below. Codex confirmed the implementation matches
upstream and the approved design: `init` uses the correct `<= SMALL` SBO
threshold and never creates `Stable`, `Alloc(Vec<T>)` owns and frees via `Drop`,
`Stable` is a borrowed slice, and `slice()` returns the correct per-variant
view; the four tests cover the upstream small/alloc/boundary behavior plus the
explicit `stable` constructor.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r560-prompt.md` (result)
- Result: `logs/codex-review/20260604-r560-last-message.md` (result)

## Conclusion

`terminal::message_data::MessageData` — a small-buffer-optimization payload
(inline / borrowed stable / heap-allocated) for thread messaging — is faithfully
ported from `datastruct/message_data.zig`. The Zig `union(enum)` became a
const-generic Rust `enum`, the `undefined` inline tail became a `Default`-filled
`[T; SMALL]` (only `[..len]` read), and `deinit` became `Drop`. This is
roastty's third `datastruct/` port (after `CacheTable` and the now-complete
`CircBuf`). Remaining `datastruct/` types are the pointer-heavy `lru` /
`intrusive_linked_list`, the libuv-specific `segmented_pool`, the channel-like
`blocking_queue`, and the large `split_tree`; the terminal **search subsystem**
(now that `CircBuf` is done) is the other natural target. The objc/bundle-id
helpers, the `home()` resolver, and config `loadDefaultFiles` remain deferred
pending roastty's naming decision; `background-image-opacity` stays
float-blocked.
