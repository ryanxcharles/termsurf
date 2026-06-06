+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 669: Surface Termio Presentation

## Description

Experiment 668 added an internal background `TermioWorker`, but the app/surface
layer still cannot observe worker events. The next slice is to connect worker
events to internal surface presentation state without adding shell selection,
runtime configuration, renderer wakeup, or new public launch ABI.

This experiment adds an App-owned surface registry and gives each `Surface`
optional termio presentation state. `roastty_app_tick` walks the registered
surfaces and lets each surface drain its attached worker events. Pump events
update process-exit state and mark the surface as dirty when terminal output or
PTY writes occurred. Error events are recorded as process-exited for now,
because there is not yet a richer app mailbox/error ABI.

This is intentionally still not full frontend presentation. It does not create a
worker from `roastty_surface_new`, choose a shell, expose terminal snapshots to
Swift, schedule renderer frames, wake the runtime from the worker thread, or
resize the terminal grid. It creates the internal state path that those later
experiments can build on.

## Changes

- `roastty/src/lib.rs`
  - Extend `App` with a `surfaces: Vec<NonNull<Surface>>` registry.
  - Register every surface in `roastty_surface_new` after allocation.
  - Unregister a surface from its app in `roastty_surface_free` before dropping
    it.
  - Define teardown ownership explicitly:
    - the app registry is non-owning; app handles do not free surfaces;
    - `roastty_app_free` detaches all currently registered surfaces by setting
      their stored app handle to null, clearing their attached termio worker,
      and clearing the app registry before dropping the app;
    - `roastty_surface_free` skips app unregistration when its stored app handle
      is null, so freeing a surface after its app is safe;
    - `roastty_surface_app(surface)` returns null after app detachment.
  - Make `roastty_app_tick(app)` drain registered surface termio events.
  - Extend `Surface` with:
    - `termio_worker: Option<termio::TermioWorker>`;
    - `process_exited: bool`;
    - `dirty: bool`;
    - `last_termio_error: Option<String>`.
    - a test-only queued termio event source used to deterministically test
      event application without relying on hard-to-trigger PTY errors.
  - Add an internal `Surface::tick_termio` helper:
    - drains all currently queued worker events;
    - drains all test-queued events when built for tests;
    - on `TermioWorkerEvent::Pump`, sets `dirty` when bytes were read, bytes
      were written, pending writes remain, EOF occurred, or child exit occurred;
    - on `Pump` EOF or child exit, sets `process_exited = true`;
    - on `Error`, records the error string, sets `process_exited = true`, and
      marks the surface dirty.
  - Make `roastty_surface_process_exited(surface)` return the stored
    `process_exited` flag.
  - Keep all termio-worker attachment internal/test-only for this experiment.
    Public worker creation, shell configuration, and C ABI surface launch
    behavior remain deferred.
- Tests in `roastty/src/lib.rs`
  - Verify `roastty_surface_new` registers surfaces on the app and
    `roastty_surface_free` unregisters them.
  - Verify `roastty_app_free` detaches still-live surfaces, clears their stored
    app handle, clears attached termio workers, and allows later
    `roastty_surface_free` without dereferencing a freed app.
  - Attach a test-created `TermioWorker` to a surface, call `roastty_app_tick`,
    and assert terminal output marks the surface dirty.
  - Attach a short-lived worker, tick until EOF/child-exit is observed, and
    assert `roastty_surface_process_exited(surface)` returns `true`.
  - Push a test-only `TermioWorkerEvent::Error` into a surface's queued event
    source, call one `roastty_app_tick`, and assert the surface records
    `last_termio_error`, marks dirty, and reports process-exited.
  - Push multiple test-only events before one `roastty_app_tick`, including a
    dirty pump event and a final child-exit pump event, and assert a single tick
    drains and applies all events.
  - Continue using `os::pty::PTY_COMMAND_LOCK` for tests that spawn worker
    subprocesses.

## Design Review

**Result:** Approved after amendments.

Codex found three blockers: the non-owning app surface registry needed explicit
teardown rules for app-free-before-surface-free, the error-event branch was not
deterministically testable through the real worker API, and the plan did not
prove that one app tick drains all queued events rather than one event.

The design now makes `roastty_app_free` detach registered surfaces and null
their app handles before dropping the app, makes later surface free skip
unregistration when detached, adds a test-only queued event source for
deterministic error and multi-event tests, and requires a single-tick
all-events-drained test.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/669-surface-termio-presentation.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty termio`
- `cargo test -p roastty surface`
- `cargo test -p roastty os::pty`
- `git diff --check`
