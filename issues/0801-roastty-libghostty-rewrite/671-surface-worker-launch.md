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

# Experiment 671: Surface Worker Launch

## Description

Experiment 670 lets a frontend snapshot a surface when a termio worker is
already attached, but workers are still attached only by tests. The next slice
is to let the surface own enough copied configuration to launch a real termio
worker through a small C ABI entry point.

This experiment adds `roastty_surface_start(surface)`. It starts a background
`TermioWorker` for a surface using the surface's copied command, working
directory, initial input, and current/default PTY size. It still does not add
the full Ghostty draw/refresh lifecycle, renderer wakeup, shell integration
policy, environment-variable propagation, foreground process tracking, or
tty-name reporting.

## Changes

- `roastty/include/roastty.h`
  - Add `roastty_surface_start(roastty_surface_t) -> roastty_result_e`.
- `roastty/src/termio.rs`
  - Add
    `Termio::spawn_with_cwd(program, args, cwd: Option<PathBuf>, size: PtySize)`.
  - Keep `Termio::spawn` as the existing convenience wrapper with no cwd.
  - Add tests that `spawn_with_cwd` runs a child in the requested working
    directory.
  - Add a test that `spawn_with_cwd` reports an error for a missing working
    directory.
- `roastty/src/lib.rs`
  - Copy nullable C strings from `RoasttySurfaceConfig` into owned surface
    state: `working_directory`, `command`, and `initial_input`. Null means
    absent. Invalid UTF-8 is treated as absent for this experiment rather than
    failing `roastty_surface_new`, matching the current skeleton's permissive
    config behavior.
  - Add `Surface::pty_size()`:
    - use `surface.size.rows` when nonzero, otherwise default rows to `24`;
    - use `surface.size.columns` when nonzero, otherwise default columns to
      `80`;
    - clamp pixel dimensions to `u16::MAX` for `PtySize`;
    - use zero pixel dimensions when no pixel size is available yet.
  - Add `Surface::start_termio()`:
    - return `ROASTTY_INVALID_VALUE` for detached/null surfaces;
    - return `ROASTTY_SUCCESS` without replacing the worker when one is already
      attached, including an attached worker whose child has already exited;
    - if `command` is absent or empty, launch `/bin/sh` with no args;
    - if `command` is present, launch `/bin/sh -lc <command>`;
    - pass copied `working_directory` to `Termio::spawn_with_cwd` when present;
    - start a `TermioWorker` with a small bounded pump timeout;
    - queue copied `initial_input` after worker start when present;
    - store the worker on the surface and reset `process_exited`, `dirty`, and
      `last_termio_error`.
    - If spawning the termio, spawning the worker, or queuing initial input
      fails, return `ROASTTY_INVALID_VALUE`, drop any partially created worker,
      and leave the prior surface state unchanged. This preserves idempotence
      and avoids a half-started surface until a richer error ABI exists.
  - Implement `roastty_surface_start(surface)` by calling
    `Surface::start_termio`.
  - Continue deferring `env_vars`, `wait_after_command`, configured shell
    selection beyond `/bin/sh`, foreground process ID, tty-name, renderer
    wakeup, and terminal grid resize.
- Tests
  - `roastty/src/termio.rs`
    - Verify `spawn_with_cwd` changes the child working directory.
  - `roastty/src/lib.rs`
    - Create a surface with `command = "printf hello"`, call
      `roastty_surface_start`, tick until dirty, snapshot render state, and
      assert `hello` is visible.
    - Create a surface from scoped `CString` values for command,
      working-directory, and initial input; drop those strings before calling
      `roastty_surface_start`; then verify all three copied values still affect
      the child.
    - Create a surface with no command, start it, queue no input, and assert a
      worker is attached and start is idempotent.
    - Start a short-lived command, tick until process exit, call
      `roastty_surface_start` again, and assert it returns success without
      replacing the already attached exited worker.
    - Create a surface with command that reads one line plus `initial_input`,
      start it, tick/snapshot, and assert the input-driven output is visible.
    - Create a surface with `working_directory`, run `pwd`, and assert the
      output contains the configured directory.
    - Configure a missing working directory and assert `roastty_surface_start`
      returns `ROASTTY_INVALID_VALUE` and leaves the previous
      worker/dirty/process/error state unchanged.
    - Unit-test `Surface::pty_size()` for default size, partial zero row/column
      fallback, and pixel clamping.
    - Verify `roastty_surface_start(null)` returns `ROASTTY_INVALID_VALUE`.
    - Continue using `os::pty::PTY_COMMAND_LOCK` for subprocess tests.

## Design Review

**Result:** Approved after amendments.

Codex found four blockers: start failure semantics needed exact result and
rollback rules, copied config string ownership needed a drop-before-start test,
idempotence after child exit needed an explicit contract, and PTY sizing needed
exact partial-zero and pixel-clamp semantics with tests.

The design now returns `ROASTTY_INVALID_VALUE` and leaves prior surface state
unchanged for termio spawn, worker spawn, or initial-input queue failures;
requires scoped-CString ownership tests; defines an attached exited worker as
already started; and specifies independent row/column fallback plus pixel
clamping with focused tests.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/671-surface-worker-launch.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty termio`
- `cargo test -p roastty render_state`
- `git diff --check`

## Result

**Result:** Pass.

Roastty surfaces can now start a real background termio worker through the new
`roastty_surface_start(surface)` ABI. Surface construction copies the command,
working directory, and initial input strings from `RoasttySurfaceConfig` into
owned state. Start uses the current/default surface PTY size, launches `/bin/sh`
with no arguments when no command is configured, launches
`/bin/sh -lc <command>` when a command is configured, passes the copied working
directory when present, starts a `TermioWorker`, queues copied initial input,
and then stores the worker on the surface.

Start is idempotent while a worker is attached, including after the child has
exited. Spawn, worker, and initial-input failures return `ROASTTY_INVALID_VALUE`
and leave the prior surface state unchanged. PTY size fallback is now explicit:
zero rows default to 24, zero columns default to 80, and pixel dimensions clamp
to `u16::MAX`.

`Termio::spawn_with_cwd` now supports cwd-aware subprocess launch, while the
existing `Termio::spawn` remains the no-cwd convenience wrapper. This experiment
still defers environment-variable propagation, shell policy beyond `/bin/sh`,
foreground PID, tty-name, renderer wakeup, terminal grid resize, and full
draw/refresh integration.

Verification passed:

- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface` — 20 passed, 0 failed
- `cargo test -p roastty termio` — 16 passed, 0 failed
- `cargo test -p roastty render_state` — 21 passed, 0 failed
- `git diff --check`

## Conclusion

Surface workers are no longer test-only attachments. A frontend can create a
surface, start its configured command worker, tick the app, and snapshot the
surface render state. The remaining PTY/frontend work is to propagate
environment variables, report foreground PID and tty name, wire renderer
wakeups, resize terminal grids with surface size changes, and implement the
broader draw/refresh lifecycle.

## Completion Review

**Result:** Approved after fixes.

Codex found no ABI or implementation blockers in the worker launch path. It
confirmed that `roastty_surface_start` is exported in the header and source,
null/detached handling is correct, strings are copied into owned state,
command/cwd/initial-input semantics match the design, idempotence preserves an
attached worker, PTY fallback/clamping is covered, and failure rollback avoids
mutating surface state until start fully succeeds.

Codex found two result-commit issues: missing result-review provenance and fixed
`/tmp` paths in missing-cwd tests. The experiment frontmatter and README agent
tuple now record the result review, and the missing-cwd tests now use
process-specific temp paths that are removed before asserting spawn failure.
