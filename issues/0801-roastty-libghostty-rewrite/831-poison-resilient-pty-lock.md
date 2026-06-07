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

# Experiment 831: Poison-resilient PTY_COMMAND_LOCK (kill the cascade amplifier)

## Description

Experiment 830 (Partial) proved the suite's flakiness has two independent
layers, and identified the **dominant** one: the global test mutex
`PTY_COMMAND_LOCK` (`os/pty.rs:211`) is acquired with `.lock().unwrap()` at
**192 sites** (160 in `lib.rs`, 20 in `termio.rs`, 12 in `os/pty.rs`). When any
test panics **while holding** the lock (e.g. a genuine round-trip assertion
failure), the mutex is **poisoned**, and every subsequent PTY test panics on
`.lock().unwrap()` with `PoisonError`. Exp 830's verification showed this turns
**1–2** genuine per-run failures into **13–77** red tests.

This experiment removes that amplifier. It does **not** fix the underlying
round-trip snapshot race (that is Exp 832) — but it makes a single flake cost
one failure instead of seventy-seven, so the real flakes become visible and the
suite output becomes meaningful.

### Why poison-resilience is correct (the Exp 830 design was wrong to reject it)

`PTY_COMMAND_LOCK` is a **pure serialization mutex over a `()`** — it guards
process-global PTY/child operations so PTY tests don't run concurrently; it
holds **no data** whose invariants a panic could break. Recovering from a
poisoned lock therefore loses nothing: each test builds fresh app/surface/child
resources. The result-review of Exp 830 confirmed that
`lock().unwrap_or_else(|e| e.into_inner())` **preserves the originating panic**
(that test still fails) and only spares the innocent cascade victims. So this is
not "masking failures" — it makes the failure report **accurate**.

Recovery also lets a successor test _run_ after a predecessor panicked mid-PTY
operation (before, it aborted on the poisoned lock). That cannot
cross-contaminate: each PTY test allocates its **own** pty + child via
`PtyCommand::spawn` (a fresh `openpty` + `Command::spawn` per call), so a
panicked test's leaked child/fd is not shared with the recovered successor.

## Changes

`roastty/src/os/pty.rs` — add a poison-recovering accessor next to the static
(`#[cfg(test)]`, `pub(crate)`):

```rust
#[cfg(test)]
pub(crate) fn pty_command_lock() -> std::sync::MutexGuard<'static, ()> {
    // A pure serialization mutex over `()`; a poisoned lock means a prior test
    // panicked while holding it, not that any guarded state is corrupt. Recover
    // so one test's panic cannot cascade into PoisonError across every other PTY
    // test (Issue 801, Exp 830/831).
    PTY_COMMAND_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}
```

Then convert **all 192** call sites by replacing the **sub-expression** (so each
binding name is preserved):

```
PTY_COMMAND_LOCK.lock().unwrap()   →   pty_command_lock()
```

across `roastty/src/os/pty.rs`, `roastty/src/termio.rs`, and
`roastty/src/lib.rs`. This is a mechanical exact-string replacement of that
expression; 191 sites bind `let _guard = …` and one binds `let _pty_guard = …`
(`lib.rs:28910`) — targeting the sub-expression covers all 192 without touching
the binding names. Update the `use` imports in `lib.rs` (`lib.rs:14503`) and
`termio.rs` (`termio.rs:427`) to import `pty_command_lock` instead of
`PTY_COMMAND_LOCK`; after the conversion `PTY_COMMAND_LOCK` is referenced only
inside the accessor in `os/pty.rs`, so its direct imports elsewhere are removed
to avoid unused-import warnings. (The `os/pty.rs` test module is
`mod tests { use super::* }`, so the module-level accessor is already in scope
there.)

No production code changes; `PTY_COMMAND_LOCK` and the accessor are both
`#[cfg(test)]`.

### Scope boundary

This experiment is **only** the cascade kill. The genuine round-trip snapshot
race (the ~110 `surface_snapshot_text_after_start(...).contains(...)` and ~10
`surface_snapshot_text` first-render sites) is **Exp 832**; the non-PTY
`config::tests::config_path_cli_*` env/path flake is **Exp 833**. Feature work
resumes only after all three land and the suite runs clean.

## Verification

The cascade is the thing being eliminated, so the verification measures the
**failure structure**, not (yet) a green suite:

- **Build/lint:** `cargo build -p roastty` — no warnings (incl. no unused-import
  warnings from the import change). `cargo fmt -p roastty -- --check` — clean.
- **Cascade gone:** `cargo test -p roastty` (bare, in-process) run **5×**; in
  every run **zero `PoisonError` panics**, and
  `total failures == genuine originator count` (expected 0–2 per run — the
  residual round-trip flakes that Exp 832 fixes), versus the 13–77 with cascade.
  An **originator** is a failing test whose panic message is **not**
  `PoisonError` (a real assertion failure); the Pass check is
  `grep -c PoisonError == 0` and `total failures == non-PoisonError panics`.
  Compare to the Exp 830 baseline (`logs/exp829/verify830-*.log`, 12–76
  `PoisonError` each).
- **Lock still serializes:** the PTY tests still pass when they win their race
  (no concurrency regression introduced) — confirmed by the runs above
  completing with only the known round-trip flakes, no new failures.
- No-ghostty grep on the three touched files — clean. `git diff --check` —
  clean.

**Pass** = zero `PoisonError` across 5 bare-`cargo test` runs (the cascade is
dead), with residual failures equal to the genuine round-trip originators only.
**Partial/Fail** = any `PoisonError` remains, or a new failure class appears.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Verified the lock is `Mutex<()>`, both it and the accessor are
`#[cfg(test)]`, exactly 192 `.lock().unwrap()` sites (lib 160 / termio 20 / pty
12), and the `verify830-*.log` PoisonError counts.

**Verdict:** CHANGES REQUIRED → fixed → approvable. Confirmed sound: the
poison-recovery semantics (only victims recover; the originating panic still
fails), the accessor's `MutexGuard<'static, ()>` type/lifetime and identical
lock scope, the 831/832/833 decomposition, and the non-green "Pass" criterion.

- **Required — one site binds a different name.** `lib.rs:28910` is
  `let _pty_guard = PTY_COMMAND_LOCK.lock().unwrap();`, so a full-line exact
  replacement would miss it. **Fixed:** the Changes section now specifies
  replacing the **sub-expression** `PTY_COMMAND_LOCK.lock().unwrap()` →
  `pty_command_lock()` (binding names preserved), explicitly covering the lone
  `_pty_guard` site.
- **Optional — process-global side effects.** **Adopted:** added a line noting
  each PTY test spawns its own pty+child, so a recovered successor cannot be
  contaminated by a panicked predecessor's leaked child/fd.
- **Nit — define "originator."** **Adopted:** the verification now defines an
  originator as a non-`PoisonError` panic and gives the exact grep check.

## Conclusion

_(to be written after the run)_
