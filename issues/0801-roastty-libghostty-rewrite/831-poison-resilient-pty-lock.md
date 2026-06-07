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

- **Build/lint:** the change is entirely `#[cfg(test)]`, so warning-cleanliness
  is established by the **test compile** (`cargo build -p roastty --tests`; the
  5 verify runs each compiled with 0 warnings, incl. no unused/missing-import
  warning from the import change), not the plain `cargo build`.
  `cargo fmt -p roastty -- --check` — clean.
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

## Result

**Result:** Pass

The accessor + a mechanical sub-expression conversion of **all 192**
`PTY_COMMAND_LOCK.lock().unwrap()` sites (160 `lib.rs` / 20 `termio.rs` / 12
`os/pty.rs`, including the lone `_pty_guard` at `lib.rs:28910`) landed, with the
two `use` imports switched to `pty_command_lock`. Build clean (no warnings), fmt
clean, no-ghostty clean, `git diff --check` clean.

**Verification — `cargo test -p roastty` (bare, in-process) ×5:**

| run | total failures | `PoisonError` | real originators | duration |
| --: | -------------: | ------------: | ---------------: | -------: |
|   1 |              2 |         **0** |                2 |    376 s |
|   2 |              2 |         **0** |                2 |    222 s |
|   3 |              2 |         **0** |                2 |    137 s |
|   4 |              3 |         **0** |                3 |    130 s |
|   5 |              2 |         **0** |                2 |     86 s |

**The cascade is dead:** zero `PoisonError` in every run (was 12–76 in Exp 830),
and `total failures == real originators` exactly. 13–77 red tests collapsed to
the **2–3 genuine flakes**. The lock still serializes correctly (4357–4358 PTY
tests pass each run; no concurrency regression).

The genuine originators, now visible:

- `surface_key_default_performable_action_falls_through_when_unperformed`
  (`lib.rs:16314`) and
  `surface_key_default_natural_text_editing_writes_legacy_bytes`
  (`lib.rs:16358`) — failed **5/5**; both are `surface_snapshot_text`
  first-render round-trip tests → **Exp 832**.
- `config::tests::config_path_cli_expands_relative_optional_absolute_home_and_missing`
  (`config/mod.rs:5642`) — failed **1/5**, a non-PTY env/path flake → **Exp
  833**.

(Measurement note: runs took 86–376 s with no hang, so the unbounded run
completed. Going forward, bare-`cargo test` verification runs use a
**no-progress watchdog** — kill + sample if the test log is silent > 90 s, hard
ceiling 600 s — so a future deadlock self-reports in ≤ 90 s instead of waiting
indefinitely.)

## Conclusion

Poison-resilience was the high-leverage fix the Exp 830 design wrongly rejected:
one mechanical change turned an unreadable 13–77-failure suite into a precise
2–3-failure one, with the real culprits named. The cascade amplifier is gone for
good.

The suite is **not yet clean** — that is the explicit, bounded remaining work:

- **Exp 832 (next):** the surface round-trip snapshot race — convert the ~110
  `surface_snapshot_text_after_start(...).contains(NEEDLE)` and ~10
  `surface_snapshot_text` first-render sites to wait for their output token (the
  Exp 830 five-test fix is the template). The two `surface_key` originators
  above are the immediate targets.
- **Exp 833:** the `config_path_cli` env/path flake (separate root cause).

Feature work (URI/regex, remaining `os/`) resumes only once 832 + 833 land and
`cargo test -p roastty` runs clean.

## Completion Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Independently verified the conversion and the evidence.

**Verdict:** APPROVED — no Required, Optional, or Nit findings. Confirmed: zero
remaining `PTY_COMMAND_LOCK.lock().unwrap()` sites; `PTY_COMMAND_LOCK`
referenced only in `os/pty.rs`; `pty_command_lock()` = 192 calls + 1 definition
(incl. the `_pty_guard` site); accessor correct and `#[cfg(test)]`; both imports
switched cleanly; `grep -c PoisonError` = 0 in all 5 logs with
`total == real originators`; the named originators match; the Pass verdict is
honest (cascade-kill goal met, residual flakes correctly deferred to 832/833);
and poison-recovery introduces no risk (mutex guards `()`; a real panic still
fails its own test). Two non-findings were noted and addressed: the build target
wording (corrected to the `--tests` compile) and pre-existing
`WindowTheme::Ghostty` literals in `lib.rs` that this diff does not introduce
(out of scope; flagged for a future cleanup).
