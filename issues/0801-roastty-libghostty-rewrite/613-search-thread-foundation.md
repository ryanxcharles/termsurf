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

# Experiment 613: search Thread — part 4: the outer Thread foundation

## Description

The outer search `Thread` (upstream `terminal/search/Thread.zig`'s top-level
`Thread`) is the OS thread that drives the inner `Search` (Exps 606–608).
Upstream builds it on **libxev** (`xev.Loop` / `xev.Async` / `xev.Timer` +
completions); roastty has no libxev, so — per the chosen direction — it is
ported as a **std-concurrency adaptation**: `std::thread` for the OS thread, the
already-ported `BlockingQueue` (`datastruct/blocking_queue`) as the `Mailbox`, a
`Mutex` for the terminal lock, and `recv`/timeout-style waits replacing the xev
async/timer. The inner `Search`'s Rust-idiomatic adaptations (NonNull, FnMut
callbacks, raw-pointer Terminal access under the lock) already set this
precedent; the event-loop is the last construct to adapt.

This is **slice 1 of 3** — the foundation that the rest builds on:

1. **This experiment (613)** — the types (`Options`, `Message`, `Mailbox`), the
   `Thread` struct, `new` / `deinit`, the `change_needle` handler, and
   `drain_mailbox`. Single-threaded testable; **no OS thread spawned yet**, so
   no `unsafe Send` is needed here.
2. **Exp 614** — the `select` message handler (it needs new Screen
   scroll-to-pin + viewport-overlap surface, so it is its own slice); the
   `Message::Select` variant is added then.
3. **Exp 615** — `thread_main` (the event loop), `std::thread` spawn, the
   `unsafe Send` model for the search's raw `Terminal` pointers, the stop
   signal, and the refresh-timer cadence.

## Upstream behavior (`Thread.zig`, the parts in this slice)

```zig
pub const Options = struct { mutex: *Mutex, terminal: *Terminal, event_cb: ?EventCallback = null, event_userdata: ?*anyopaque = null };
pub const Mailbox = BlockingQueue(Message, 64);
pub const Message = union(enum) { change_needle: MessageData(u8, 255), select: ScreenSearch.Select };

mailbox: *Mailbox, search: ?Search = null, opts: Options, /* + xev fields (slice 3) */

pub fn init(alloc, opts) !Thread { /* create mailbox + xev loop/async/timer */ }
pub fn deinit(self) void { /* deinit xev + mailbox; if (search) |*s| s.deinit(); */ }

fn drainMailbox(self) !void {
    while (self.mailbox.pop()) |message| switch (message) {
        .change_needle => |v| { defer v.deinit(); try self.changeNeedle(v.slice()); },
        .select => |v| try self.select(v),
    };
}

fn changeNeedle(self, needle) !void {
    if (self.search) |*s| {
        if (std.ascii.eqlIgnoreCase(s.viewport.needle(), needle)) return;  // unchanged
        s.deinit(); self.search = null;
        if (self.opts.event_cb) |cb| { cb(.{ .total_matches = 0 }, ud); cb(.{ .selected_match = null }, ud); cb(.{ .viewport_matches = &.{} }, ud); }
    }
    if (needle.len == 0) return;                       // empty needle stops the search
    self.search = try .init(self.alloc, needle);
    self.opts.mutex.lock(); defer self.opts.mutex.unlock();
    self.search.?.feed(self.alloc, self.opts.terminal); // initial feed under the lock
}
```

## Rust mapping (`thread.rs`, extending the existing `Search` module)

The new types and `Thread` live alongside `Search` in `search/thread.rs` so they
can use `Search`'s `pub(in crate::terminal)` surface. `Options` carries raw
pointers to embedder-owned state (the lock and terminal), mirroring upstream's
`*Mutex` / `*Terminal`. The `EventCallback` fn-ptr + userdata becomes a boxed
`FnMut(Event) + Send` (the `+ Send` is required for slice 3's thread; harmless
now). `Allocator.Error` is infallible, so the `init`/`changeNeedle` error unions
drop.

```rust
use std::ptr::NonNull;
use std::sync::Mutex;
use super::super::message_data::MessageData;
use super::super::blocking_queue::BlockingQueue;

/// Messages the search thread accepts (upstream `Thread.Message`). `Select` is added in Exp 614.
pub(in crate::terminal) enum Message {
    /// Change the search term (start / restart / stop on empty).
    ChangeNeedle(MessageData<'static, u8, 255>),
}

/// The mailbox for sending the search thread messages (upstream `Mailbox = BlockingQueue(Message, 64)`).
/// Held behind an `Arc` so a producer's handle stays valid once the `Thread` moves into
/// `std::thread::spawn` (Exp 615), mirroring upstream's separately-allocated `*Mailbox`.
pub(in crate::terminal) type Mailbox = BlockingQueue<Message, 64>;

/// The event callback (upstream `EventCallback` fn-ptr + opaque userdata) as a boxed closure. `Send`
/// is required once the thread is spawned (Exp 615).
pub(in crate::terminal) type EventCallback = Box<dyn FnMut(Event<'_>) + Send>;

/// Embedder-supplied configuration (upstream `Thread.Options`). The lock and terminal are raw
/// pointers to embedder-owned state, accessed only while the lock is held.
pub(in crate::terminal) struct Options {
    /// Guards all access to `terminal`.
    pub(in crate::terminal) lock: NonNull<Mutex<()>>,
    /// The terminal to search.
    pub(in crate::terminal) terminal: NonNull<Terminal>,
    /// Optional event callback.
    pub(in crate::terminal) event_cb: Option<EventCallback>,
}

/// The search thread (upstream `Thread`). This slice lands its state + message handling; the OS
/// thread and event loop come in Exp 615.
pub(crate) struct Thread {
    mailbox: std::sync::Arc<Mailbox>,
    search: Option<Search>,
    opts: Options,
}

impl Thread {
    pub(in crate::terminal) fn new(opts: Options) -> Thread {
        Thread { mailbox: std::sync::Arc::new(Mailbox::new()), search: None, opts }
    }

    /// Tear down the active search (upstream `deinit`'s `if (search) |*s| s.deinit()`). The xev
    /// teardown is slice 3. The `deinit` runs under the terminal lock (it untracks pins, mutating
    /// terminal state).
    ///
    /// # Safety
    /// The terminal (and lock) the search points into must still be live (the `Search::deinit`
    /// contract).
    pub(in crate::terminal) unsafe fn deinit(&mut self) {
        if let Some(mut s) = self.search.take() {
            // SAFETY: `lock` is live and guards `terminal`.
            let _guard = unsafe { self.opts.lock.as_ref() }.lock().unwrap();
            // SAFETY: terminal live; lock held.
            unsafe { s.deinit() };
        }
    }

    /// An owned handle to the mailbox (so producers can post messages even after the `Thread` is
    /// spawned).
    pub(in crate::terminal) fn mailbox(&self) -> std::sync::Arc<Mailbox> {
        std::sync::Arc::clone(&self.mailbox)
    }

    /// Drain and dispatch all pending messages (upstream `drainMailbox`).
    ///
    /// # Safety
    /// As `change_needle`.
    pub(in crate::terminal) unsafe fn drain_mailbox(&mut self) {
        while let Some(message) = self.mailbox.pop() {
            match message {
                // SAFETY: caller's contract.
                Message::ChangeNeedle(v) => unsafe { self.change_needle(v.slice()) },
            }
        }
    }

    /// Change the search term (upstream `changeNeedle`): unchanged → no-op; otherwise stop the prior
    /// search (emitting reset events), and on a non-empty needle start a new search with an initial
    /// feed under the lock.
    ///
    /// # Safety
    /// `opts.terminal` and `opts.lock` must be live; the terminal must outlive any search.
    pub(in crate::terminal) unsafe fn change_needle(&mut self, needle: &[u8]) {
        // Unchanged needle → no-op (case-insensitive).
        if let Some(s) = self.search.as_ref() {
            if s.needle().eq_ignore_ascii_case(needle) {
                return;
            }
        }
        // Stop the prior search: deinit it UNDER the lock (it untracks pins, mutating terminal
        // state), then emit the reset events AFTER releasing the lock (so callbacks cannot reenter
        // while the terminal is locked).
        if let Some(mut old) = self.search.take() {
            {
                // SAFETY: `lock` is live and guards `terminal`.
                let _guard = unsafe { self.opts.lock.as_ref() }.lock().unwrap();
                // SAFETY: terminal live; lock held.
                unsafe { old.deinit() };
            }
            if let Some(cb) = self.opts.event_cb.as_mut() {
                cb(Event::TotalMatches(0));
                cb(Event::SelectedMatch(None));
                cb(Event::ViewportMatches(&[]));
            }
        }
        if needle.is_empty() {
            return; // empty needle stops the search
        }
        let mut s = Search::new(needle);
        {
            // Initial feed under the terminal lock.
            // SAFETY: `lock` is live and guards `terminal`.
            let _guard = unsafe { self.opts.lock.as_ref() }.lock().unwrap();
            // SAFETY: terminal live; the lock is held.
            unsafe { s.feed(self.opts.terminal) };
        }
        self.search = Some(s);
    }
}
```

### New `Search` accessor

`change_needle`'s unchanged-check needs the search's needle:

```rust
/// The needle this aggregator is searching for (upstream `s.viewport.needle()`).
pub(in crate::terminal) fn needle(&self) -> &[u8] {
    self.viewport.needle()
}
```

### Notes / deviations

- **std-concurrency adaptation** (the chosen direction): libxev → `std::thread`
  (slice 3) + `BlockingQueue` mailbox + `Mutex`. This slice introduces only the
  types + handlers; no thread is spawned, so no `unsafe Send` yet.
- `Options` holds `NonNull<Mutex<()>>` + `NonNull<Terminal>` (embedder-owned),
  mirroring upstream's `*Mutex` / `*Terminal`.
- `EventCallback` → `Box<dyn FnMut(Event<'_>) + Send>`; the `+ Send` is for
  slice 3 and is harmless now.
- `Message::Select` and the `select` handler are deferred to Exp 614 (they need
  new scroll-to-pin surface).
- `MessageData(u8, 255)` → `MessageData<'static, u8, 255>`; `v.deinit()` (Zig)
  is unnecessary (Rust `Drop`).

## Verification

- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — no regressions; new tests (a real `Terminal` + a
  `Mutex<()>`, an event-collecting `Arc<Mutex<Vec<…>>>` callback):
  - `change_needle_starts_a_search_and_feeds` — `change_needle(b"Fizz")` on a
    screen with the needle creates a search whose active screen searcher has
    matches (the initial feed ran under the lock).
  - `change_needle_unchanged_is_a_noop` — a second `change_needle` with the same
    needle (any case) leaves the search in place and emits no reset events.
  - `change_needle_empty_stops_the_search` — `change_needle(b"")` after a search
    drops it and emits `TotalMatches(0)` / `SelectedMatch(None)` /
    `ViewportMatches([])`.
  - `drain_mailbox_processes_change_needle` — push a `ChangeNeedle` message,
    `drain_mailbox`, and the search is created.
  - `deinit_releases_the_search` — after `change_needle`, `Thread::deinit`
    returns the terminal's tracked-pin count to baseline.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on touched source — clean.
- `git diff --check` — clean.

Pass = the `Thread` foundation accepts messages, `change_needle` starts / stops
/ no-ops the search with the correct reset events and an initial locked feed,
and `deinit` leaks no tracked pins — all single-threaded, with the OS thread and
`select` deferred to the next slices.

## Design Review

Codex reviewed the design and raised **two Required** findings, both adopted:

- **Required (adopted)**: the old search's `deinit` must run **under the
  terminal lock** — it untracks pins and mutates terminal-owned screen/page-list
  state, so doing it outside the lock would race a concurrent render thread once
  Exp 615 spawns the real thread. `change_needle` now: checks unchanged without
  locking; `take`s the prior search; locks; `deinit`s; unlocks; then emits the
  three reset events (after releasing the lock, so callbacks can't reenter while
  locked). `Thread::deinit` also locks around `Search::deinit`.
- **Required (adopted)**: the mailbox is held behind `Arc<Mailbox>` so a
  producer's handle stays valid once the `Thread` moves into
  `std::thread::spawn`; `mailbox()` returns a cloned `Arc`. This mirrors
  upstream's separately-allocated `*Mailbox` and avoids churn in Exp 615.
- **Optional (confirmed)**: deferring `Message::Select` to Exp 614 is a clean
  slice boundary; the raw `NonNull<Mutex<()>>` + `NonNull<Terminal>` `Options`
  is the right fit for this adaptation (an `Arc<Mutex<Terminal>>` rewrite would
  fight the existing raw-pointer search model).
- **Nit (adopted)**: documented that the reset callbacks are intentionally
  emitted after old-search teardown and after the lock is released.

Codex confirmed the rest is faithful: the case-insensitive unchanged no-op, the
empty-needle stop, the reset-event order, the initial feed under the lock,
`MessageData<'static, u8, 255>`, and `EventCallback + Send`.

Review artifacts:

- Prompt: `logs/codex-review/20260605-d613-prompt.md`
- Result: `logs/codex-review/20260605-d613-last-message.md`
