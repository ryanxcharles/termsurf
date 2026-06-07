+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.result]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 837: Serialize the config tests' process-global env/cwd mutation

## Description

The last default-parallelism flake is the config `$HOME` class — two tests that
fail intermittently when run concurrently:

- `config::tests::config_path_cli_expands_relative_optional_absolute_home_and_missing`
  (`config/mod.rs:5572`) — failed with `left: "/Users/ryan/home-child.conf"`
  (the **real** `$HOME`) vs the expected temp base. Its `$HOME` was clobbered by
  another test mid-run.
- `config::tests::bell_audio_path_expands_from_file_cli_home_and_missing_bases`
  (`config/mod.rs:5675`) — failed with
  `left: …roastty-config-**path-cli**-base…` vs expected
  `…roastty-config-**bell-path**-base…`. It read the **other** test's `$HOME`.

Root cause: both set the process-global `HOME` via the test `EnvGuard`
(`config/mod.rs:7536`), which does `std::env::set_var("HOME", …)` on
construction and restores on `Drop`. `std::env` is process-global and shared
across all test threads, so two `EnvGuard`s racing (one's `set_var`/`Drop`
interleaving with the other's read) makes a test see the wrong `HOME`. The
sibling `CurrentDirGuard` (`config/mod.rs:7559`, `std::env::set_current_dir`) is
the same hazard for cwd.

There are exactly **two** `EnvGuard::set` sites (both `HOME`, lines 5580
and 5675) and **one** `CurrentDirGuard::set` (line 5554), each in a
**different** test — no test holds two guards — so a single serializing lock
cannot self-deadlock.

## Changes

`roastty/src/config/mod.rs` (test code only). Add a process-wide lock that both
guards hold for their whole lifetime, so env/cwd mutation is serialized across
test threads:

```rust
static PROCESS_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// Poison-resilient: the lock guards no data (it is a pure serialization mutex
// over `()`), so a test panicking while holding it must not cascade
// PoisonError into every other env/cwd test (mirrors pty_command_lock, Exp 831).
fn process_env_lock() -> std::sync::MutexGuard<'static, ()> {
    PROCESS_ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}
```

(`std::sync::Mutex` is fully qualified because the test module has no
`use std::sync::*`.) Then give `EnvGuard` and `CurrentDirGuard` each a
`_lock: std::sync::MutexGuard<'static, ()>` field, acquired **first** in their
`set` constructors (before the `set_var` / `set_current_dir`). The env/cwd
restore runs in each guard's **manual `impl Drop::drop` body**, which Rust
executes **before** dropping any field; the `_lock` field drops afterward — so
the lock is released **after** the restore, keeping the entire set→use→restore
window mutually exclusive with every other guard. (This holds because the
restore is in `drop()`, **not** a field drop, so field declaration order is
immaterial — no source comment should claim otherwise.)

No production code changes; the lock and both guards are test-only.

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked task
per run, no poll-watcher):

- **Targeted:** both config tests pass in isolation after the change.
- **Reproduce-the-fix at the failing setting:** the full suite at **default**
  parallelism run **5×** (each its own `bounded-run.sh`) — **every run
  `4360 passed; 0 failed`, 0 panics, 0 `PoisonError`**. What actually
  establishes the fix is the **structural mutual-exclusion** argument (no two
  guards can hold the process env/cwd at once); the 5 runs **corroborate** it —
  they do not independently prove it ((2/3)⁵ ≈ 0.13, so 5 greens is ~87% against
  a 1/3 flake). The prior baseline is on record (`logs/exp835`/`logs/exp836`:
  config flaked ~1/3 of runs).
- `cargo build -p roastty --tests` — no warnings.
  `cargo fmt -p roastty -- --check` — clean. No-ghostty grep on the changed
  lines — clean. `git diff --check` — clean.

**Pass** = 5/5 fully green full-suite runs at default parallelism (zero
failures, zero poison) — the suite is **default-green**. **Partial/Fail** = any
config (or other) failure remains.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Independently reproduced the failure evidence, traced the env-read
path (`config/mod.rs:1081` `var_os("HOME")`, exercised only by the two failing
tests), confirmed 3 guard sites in 3 distinct tests (no two-guard test → no
self-deadlock), and that all guard refs are under `#[cfg(test)]`.

**Verdict:** APPROVED, no Required findings. Confirmed: the assert mismatches
are a genuine concurrent `$HOME` interleave (one test read the _sibling's_ temp
HOME, the other read the _real_ HOME — only a process-global race can produce
the other test's PID+nanos path, not a restore/canonicalization bug);
serializing the three writers fully covers the observed flake (the
`WorkingDirectory` tilde tests use an explicit `finalize_with_home`, never the
process HOME); poison-resilience is warranted (the guards span the panicking
asserts). Three findings, adopted:

- **Optional — drop-order rationale was wrong.** The restore runs in the manual
  `impl Drop::drop` body, which Rust runs **before** any field drop, so `_lock`
  releases after the restore **regardless** of field declaration order.
  **Fixed:** the rationale now credits `drop()`-before-fields, and explicitly
  forbids a source comment claiming declaration order is load-bearing.
- **Optional — 5× is corroboration, not proof.** (2/3)⁵ ≈ 0.13. **Fixed:** the
  verification now states the structural mutual-exclusion argument carries the
  fix and the runs corroborate it.
- **Nit — `Mutex` not in scope.** **Fixed:** the static and field are
  fully-qualified `std::sync::Mutex` / `std::sync::MutexGuard`.

## Result

**Result:** Pass

The lock landed (`PROCESS_ENV_LOCK` + `process_env_lock()`, and a `_lock` field
on `EnvGuard` and `CurrentDirGuard`). Build clean (no warnings), fmt clean,
no-ghostty clean, `git diff --check` clean; both config tests pass in isolation.

| run (default) | STATUS              | result               |
| ------------- | ------------------- | -------------------- |
| #1            | COMPLETED rc=0 301s | 4360 passed / 0 fail |
| #2            | COMPLETED rc=0 295s | 4360 passed / 0 fail |
| #3            | COMPLETED rc=0 191s | 4360 passed / 0 fail |
| #4            | COMPLETED rc=0 156s | 4360 passed / 0 fail |
| #5            | COMPLETED rc=0 156s | 4360 passed / 0 fail |

**5/5 fully green at default parallelism** — 0 failures, 0 panics, 0
`PoisonError`, no `HARD_TIMEOUT`/`IDLE_KILL`. The structural mutual-exclusion
(one guard holds the process env/cwd at a time) eliminates the race; the 5 runs
corroborate it. (Serializing the _writers_ is sufficient: the only unguarded
process-env/cwd readers — a few `set_cli_args` callers reading `current_dir()` —
use no cwd-dependent relative path, so the env/cwd value never influences their
assertions.)

**The suite is now default-green, so the thread cap is dropped.** The README
"Test execution gate" routine gate reverts from
`cargo test -p roastty -- --test-threads=4` to plain **`cargo test -p roastty`**
(default parallelism) through `scripts/bounded-run.sh`; the machine-relative
`--test-threads=4` note is removed.

## Conclusion

This was the last flake. The full arc of the test-suite stabilization (Issue
801, Exps 829–837):

- **829** — deadlock-proof gate + the PTY-drop deadlock fix.
- **831** — poison-resilient `PTY_COMMAND_LOCK` (killed the 13–77-failure
  cascade).
- **832** — the surface-snapshot first-render race (~106 sites →
  wait-for-token).
- **833** — bounded parallelism (Partial: it shifted, not removed, the flake —
  the lesson that per-test fixes, not a global cap, were needed).
- **834** — `surface_start` echo-defeated KEY.
- **835** — `natural_text` racy-echo assertion → the deterministic byte.
- **836** — `performable_action` blocked child → `cat -v` deterministic output.
- **837** — the config `$HOME`/cwd env race → a serialization lock.

`cargo test -p roastty` now runs **green at default parallelism** (5/5), under
the 15-min bounded-run gate. The contention-driven flakes were _fixed_, not
masked, so the thread cap is no longer needed.

**Feature work resumes next** (Issue 801's actual remaining surface area:
URI/regex needing an oniguruma-class engine, the remaining `os/` slices, etc.),
now that the suite is a trustworthy signal again.

## Completion Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED — no Required findings.** Independently
verified: `_lock` is acquired first in both `set` constructors (before the
mutation), the restore lives in the manual `Drop::drop` body (runs before field
drops, so the lock outlives the restore), types fully-qualified, three guard
sites in three distinct tests (no self-deadlock), test-only; v1–v5 each
`4360 passed; 0 failed`, rc=0, 0 panics/poison, `CMD=cargo test -p roastty` (no
`--test-threads`), no timeout; the README gate is now plain
`cargo test -p roastty` with no prescriptive cap; run times fell (301→156 s, no
slowdown) and `PROCESS_ENV_LOCK` is independent of `PTY_COMMAND_LOCK`. One
Optional, adopted: noted that the unguarded `set_cli_args` cwd readers are
path-independent, so serializing the writers suffices.
