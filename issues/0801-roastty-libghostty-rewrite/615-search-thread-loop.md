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

# Experiment 615: search Thread — part 6: thread_main + spawn (std-concurrency loop)

## Description

The final slice of the outer search `Thread`: `thread_main` (upstream's libxev
event loop) and the OS-thread lifecycle. Per the chosen direction this is a
**std-concurrency adaptation** — upstream's `xev.Loop` / `xev.Async` (wakeup,
stop) / `xev.Timer` (refresh) become a `std::thread`, a `Condvar`-based wakeup,
an `AtomicBool` stop flag, and a `recv`/`wait_timeout` refresh cadence. With
this, the entire search subsystem is complete (inner `Search` 606–608 + `Thread`
613–615).

This slice's crux is **`unsafe impl Send for Thread`**: the `Search` and
`Options` hold raw `NonNull<Terminal>` / `NonNull<Screen>` / `NonNull<Mutex>`
pointers into embedder-owned state, so moving the `Thread` into
`std::thread::spawn` requires an explicit `Send` with a documented contract.

## Upstream behavior (`Thread.zig` `threadMain` + callbacks)

```zig
const REFRESH_INTERVAL = 24; // ms, 40 FPS

fn threadMain_(self) !void {
    // name thread "search", lower QoS (macOS); start the wakeup/stop async waiters;
    // notify() once to drain the mailbox immediately; start the refresh timer.
    while (true) {
        if (self.loop.stopped()) { while (self.mailbox.pop()) |_| {} return; }   // drain + quit
        const s = self.search orelse { try self.loop.run(.once); continue; };    // idle → block
        if (event_cb) |cb| s.notify(alloc, cb, ud);                              // notify
        if (s.isComplete()) { try self.loop.run(.once); continue; }              // complete → block
        switch (s.tick()) {
            .complete, .progress => {},
            .blocked => { mutex.lock(); defer mutex.unlock(); s.feed(alloc, terminal); },
        }
        try self.loop.run(.no_wait);                                             // process msgs, return
    }
    // defer: emit `.quit` to the callback.
}

// async/timer callbacks:
//   wakeup → drainMailbox();   stop → loop.stop();
//   refresh (every 24ms) → if (search) { lock; s.feed(terminal); unlock; } rearm if active;
```

## Concurrency model (the design's crux)

| upstream (libxev)                 | roastty (std)                                                   |
| --------------------------------- | --------------------------------------------------------------- |
| `xev.Loop` + `loop.run(.once)`    | block on a `Condvar` with a `REFRESH_INTERVAL` timeout          |
| `xev.Loop` + `loop.run(.no_wait)` | drain the mailbox + loop without waiting (busy-progress)        |
| `xev.Async wakeup` → drainMailbox | a producer `push`es then signals the `Condvar`                  |
| `xev.Async stop` → loop.stop      | an `AtomicBool stop` + `Condvar` signal                         |
| `xev.Timer refresh` (24ms feed)   | track `last_refresh: Instant`; feed when `elapsed() >= REFRESH` |

```rust
const REFRESH_INTERVAL: Duration = Duration::from_millis(24); // 40 FPS

/// Shared wakeup / stop control between the producer (`ThreadHandle`) and the spawned thread. The
/// `pending` flag makes `wait` predicate-based so a posted message is handled immediately (no
/// lost-wakeup latency), not just on the next refresh tick.
struct Control {
    stop: AtomicBool,
    pending: Mutex<bool>,
    cv: Condvar,
}

impl Control {
    /// Mark work pending and wake the thread (a posted message or a stop).
    fn signal(&self) {
        *self.pending.lock().unwrap() = true;
        self.cv.notify_all();
    }
    fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
        self.signal();
    }
    /// Block up to `timeout` for pending work (or the refresh tick), then consume the flag.
    fn wait(&self, timeout: Duration) {
        let mut p = self.pending.lock().unwrap();
        if !*p {
            p = self.cv.wait_timeout_while(p, timeout, |pending| !*pending).unwrap().0;
        }
        *p = false;
    }
}
```

`Thread` gains `control: Arc<Control>`. The `unsafe impl Send for Thread`
carries the contract: the `terminal` / `lock` the raw pointers refer to must
outlive the thread (the embedder joins it before dropping them) and must not be
moved; ALL terminal access on both threads goes through `opts.lock`.

## Rust mapping (`thread.rs`)

```rust
// SAFETY: `Thread`'s raw pointers (`Options.terminal`/`lock`, the `Search`'s screen pointers) point
// at embedder-owned state that (per the embedder's contract) outlives the joined thread, is not
// moved, and is only accessed while `opts.lock` is held. The callback is already `Send`.
unsafe impl Send for Thread {}

/// A handle to a spawned search thread: post messages, request stop, and join.
pub(crate) struct ThreadHandle {
    join: Option<std::thread::JoinHandle<()>>,
    control: Arc<Control>,
    mailbox: Arc<Mailbox>,
}

impl ThreadHandle {
    /// Post a message and wake the thread (upstream: push to the mailbox + `wakeup.notify()`).
    pub(in crate::terminal) fn post(&self, message: Message) {
        self.mailbox.push(message, Timeout::Forever);
        self.control.signal();
    }
    /// Request the thread stop, then join it (upstream `stop.notify()` + the caller's join).
    pub(in crate::terminal) fn stop_and_join(&mut self) {
        self.control.request_stop();
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Thread {
    /// Spawn the OS thread running `thread_main`, returning a handle (upstream: caller spawns
    /// `threadMain`).
    ///
    /// # Safety
    /// As `Thread::deinit` / `change_needle`: the embedder's terminal + lock must outlive the
    /// thread (join via `ThreadHandle::stop_and_join` before dropping them) and not be moved.
    pub(in crate::terminal) unsafe fn spawn(self) -> ThreadHandle {
        let control = Arc::clone(&self.control);
        let mailbox = Arc::clone(&self.mailbox);
        let join = std::thread::spawn(move || {
            let mut thread = self;
            // SAFETY: the embedder's contract (as `spawn`).
            unsafe { thread.thread_main() };
            // Tear down the search (untrack pins) on the thread before `thread` drops — the
            // terminal + lock are still live per the contract (joined after this returns). Upstream's
            // lifecycle is "thread exits, then the caller deinits"; here the cleanup runs at the end
            // of the thread body since `ThreadHandle` no longer owns the `Thread`.
            // SAFETY: as `spawn`.
            unsafe { thread.deinit() };
        });
        ThreadHandle { join: Some(join), control, mailbox }
    }

    /// The thread body (upstream `threadMain_`): interleave message draining, search progress,
    /// periodic feed, and notifications until stopped.
    ///
    /// # Safety
    /// As `spawn`.
    unsafe fn thread_main(&mut self) {
        let mut last_refresh = Instant::now();
        loop {
            if self.control.stop.load(Ordering::Acquire) {
                while self.mailbox.pop().is_some() {} // drain + quit
                break;
            }

            // Process pending messages (upstream wakeup → drainMailbox).
            // SAFETY: embedder contract.
            unsafe { self.drain_mailbox() };

            // Periodic refresh tick (upstream refresh timer, 24ms). Reset `last_refresh` on EVERY
            // tick (even with no search) so the idle wait stays ~24ms rather than spinning; feed
            // only when a search is active.
            if last_refresh.elapsed() >= REFRESH_INTERVAL {
                if self.search.is_some() {
                    // SAFETY: embedder contract; feed under the lock.
                    unsafe { self.feed_under_lock() };
                }
                last_refresh = Instant::now();
            }

            // Notify any state changes.
            if let (Some(cb), Some(s)) = (self.opts.event_cb.as_mut(), self.search.as_mut()) {
                s.notify(cb);
            }

            // Tick the active search (compute the outcome before re-borrowing `self` to feed).
            let tick = match self.search.as_mut() {
                None => None,
                Some(s) if s.is_complete() => None,
                Some(s) => Some(s.tick()),
            };
            if matches!(tick, Some(Tick::Blocked)) {
                // SAFETY: embedder contract.
                unsafe { self.feed_under_lock() };
            }
            // Any tick outcome (Complete/Progress/Blocked) means active work — loop without waiting,
            // matching upstream's `run(.no_wait)`. `None` (idle/complete) → block.
            if tick.is_none() {
                // Idle/complete: block until a message/stop, or the next refresh tick.
                let until_refresh = REFRESH_INTERVAL.saturating_sub(last_refresh.elapsed());
                self.control.wait(until_refresh.max(Duration::from_millis(1)));
            }
        }

        // Emit the quit event.
        if let Some(cb) = self.opts.event_cb.as_mut() {
            cb(Event::Quit);
        }
    }

    /// Feed the active search under the terminal lock.
    /// # Safety: as `change_needle`.
    unsafe fn feed_under_lock(&mut self) {
        if let Some(s) = self.search.as_mut() {
            // SAFETY: lock live, guards terminal.
            let _guard = unsafe { self.opts.lock.as_ref() }.lock().unwrap();
            // SAFETY: terminal live; lock held.
            unsafe { s.feed(self.opts.terminal) };
        }
    }
}
```

`Thread::new` also initializes
`control: Arc::new(Control { stop: AtomicBool::new(false), waker: (Mutex::new(()), Condvar::new()) })`.

### Notes / deviations

- libxev → std (`std::thread`, `Condvar`, `AtomicBool`, `Instant`) — the chosen
  adaptation. `Instant`/`Duration` are fine in roastty source (the harness's
  `Date.now()` ban is only for workflow scripts).
- The macOS thread-name / QoS calls (`pthread_setname_np`, `setQosClass`) are a
  best-effort nicety; ported if a roastty `os::macos` wrapper exists, else
  omitted (a logged TODO) — they don't affect correctness.
- The refresh-pause optimization (`refresh_active` / `stopRefreshTimer`) is left
  as a TODO (upstream notes it too); the basic model refreshes while a search is
  active.
- **Callbacks run on the search thread.** The `EventCallback` body executes on
  the spawned thread; it must not touch terminal state except under the same
  `opts.lock`. This is documented on `Options::event_cb` / `spawn`.

## Verification

- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — no regressions; new tests (real `Terminal` + a
  `Box`-leaked `Mutex<()>` so it outlives the thread; an `Arc<(Mutex,Condvar)>`
  the callback signals on `Complete`):
  - `spawn_searches_and_emits_complete` — `spawn`; `post(ChangeNeedle("Fizz"))`;
    wait (bounded) for the callback to observe `Complete`; `stop_and_join`;
    assert a `Complete` (and a `TotalMatches`) event arrived and a final `Quit`
    after join.
  - `spawn_then_stop_emits_quit` — `spawn`, immediately `stop_and_join`, assert
    the `Quit` event fired and the thread joined.
  - `post_select_after_search` — `spawn`, `post(ChangeNeedle)`, wait for
    complete, `post(Select::Next)`, drain a bounded time, `stop_and_join`
    (smoke: no panic / no deadlock).
  - The single-threaded handler tests (613/614) remain.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on touched source — clean.
- `git diff --check` — clean.

Pass = a spawned search thread drains messages, makes search progress, feeds
under the lock periodically, emits the state-change + completion + quit events,
and stops/joins cleanly — completing the search subsystem.

## Design Review

Codex reviewed the design and raised **three Required** findings, all adopted:

- **Required (adopted)**: the spawned closure must run `thread.deinit()` after
  `thread_main()` returns — `Thread` has no `Drop`, and `ThreadHandle` can't
  deinit after join (it no longer owns the `Thread`), so without this the
  tracked pins leak. Added `unsafe { thread.deinit() }` at the end of the thread
  body; the contract is that the terminal + lock stay live until after join +
  this cleanup.
- **Required (adopted)**: the `thread_main` borrow conflict
  (`self.feed_under_lock()` called while `s = self.search.as_mut()` is borrowed)
  — restructured to compute the tick outcome into a `let tick: Option<Tick>`
  first, drop the `s` borrow, then `feed_under_lock()` if `Blocked`.
  `tick.is_none()` (idle/complete) → wait.
- **Required (adopted)**: the refresh cadence bug — `last_refresh` was updated
  only when a search was active, so an idle no-search thread spun at 1ms. Now
  `last_refresh` resets on every 24ms tick regardless of search (feeding only
  when a search is active), keeping the idle wait at ~24ms.
- **Optional (adopted)**: a predicate-based `Control` (`pending: Mutex<bool>` +
  `Condvar`, `wait_timeout_while`) so a posted message is handled immediately
  rather than waiting up to the refresh timeout — eliminating the (benign)
  lost-wakeup latency.
- **Optional (noted)**: `stop_and_join` could surface a thread panic; left as a
  best-effort `let _ = join()` for now.
- **Nit (adopted)**: documented that callbacks run on the search thread and must
  use `opts.lock` for any terminal access.

Codex confirmed `unsafe impl Send for Thread` is acceptable with the stated
contract (terminal + lock stable, outlive the joined thread, all terminal access
on every thread under `opts.lock`, callbacks respecting that boundary), and that
the std-concurrency loop is otherwise faithful to upstream's xev loop.

Review artifacts:

- Prompt: `logs/codex-review/20260605-d615-prompt.md`
- Result: `logs/codex-review/20260605-d615-last-message.md`
