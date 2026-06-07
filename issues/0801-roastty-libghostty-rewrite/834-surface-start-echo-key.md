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

# Experiment 834: Fix the surface_start snapshot KEY defeated by input echo

## Description

The Exp 833 residual —
`surface_start_uses_copied_config_after_source_strings_drop` (`lib.rs:29152`) —
is **not** contention and **not** a wrap. With the assert instrumented to dump
the snapshot and reproduced under threads=4, the captured state is:

```
SSNAP cwd="/Users/ryan/dev/termsurf/roastty"
      text="owned\n\n\n\n…"
```

The screen shows **only `owned`**; the `current_dir:` prefix is absent. Cause:
the child is `IFS= read line; printf '%s:%s' "$(pwd)" "$line"` with **no
`stty -echo`** and `initial_input = "owned\n"`, so the PTY **echoes the input
`owned`** to the screen _before_ the child reads it and prints `"$(pwd):owned"`.
Exp 832 converted this site to wait for the token `"owned"` — but `"owned"`
matches the **echo**, so `surface_snapshot_text_after_start_until(…, "owned")`
returns at the echoed input, **before** the child's real output renders. Then
`assert!(contains(current_dir))` fails while `assert!(contains("owned"))` passes
— exactly the observed asymmetry. It is timing-sensitive (the echo and the child
output normally render close together; threads=4 widened the gap, so it failed
2/3 there and 0/15 at default).

This is a **KEY-choice bug introduced in Exp 832**, not a new render fault. An
audit of all ten `_after_start_until` `let text =` sites confirms **only this
one** keys on echoed input; the other nine key on the child's _output_ (env-var
values like `surface-env`/`scoped-env`/`unset`/`second`, the `stty -echo` site
`out:hello`, or `printf`/no-`initial_input` sites), which never appears in an
echo (the only other `initial_input`-bearing site, `out:hello`, disables echo
with `stty -echo`).

## Changes

`roastty/src/lib.rs` (test code only), one site:

```
let text = surface_snapshot_text_after_start_until(app, surface, "owned");
    →
let needle = format!("{}:owned", current_dir.to_str().unwrap());
let text = surface_snapshot_text_after_start_until(app, surface, &needle);
```

Key on the **whole** child output token `"<current_dir>:owned"` rather than just
the prefix: the helper's `needle` is `&str` (a `&format!(…)` temporary is
valid), and this exact string appears **only** in the child's single
`printf '%s:%s'` write (one `write()` at child exit), never in the echoed
`owned`. Waiting for it guarantees **both** asserted halves (`current_dir` and
`owned`) are present at return, with no prefix-only torn-read window.
`"<current_dir>:owned"` (e.g. `/Users/ryan/dev/termsurf/roastty:owned`, 38
chars; `current_dir` alone is 32) is well under the 80-col `test_pty_size`, so
it does not wrap and the `contains(current_dir)` match stays contiguous. The two
existing asserts are unchanged. No production code change.

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked
task, no poll-watcher; each suite run its own `bounded-run.sh`):

- **Targeted, fast:** `surface_start_uses_copied_config…` passes in isolation
  after the change (it already did; this confirms no build/logic break).
- **Reproduce-the-fix at the failing setting:** the full suite at
  `--test-threads=4` (where this test failed 2/3 in Exp 833) run **3×** — every
  run **0 failures, 0 panics, 0 `PoisonError`**, `surface_start` `... ok` each
  time. (threads=4 is still the Exp 833 gate; this experiment fixes the one test
  the cap exposed, it does not yet change the gate.)
- **No default-parallelism regression:** one full-suite run at default
  parallelism still shows `surface_start` `... ok` (it was 15/15 before; the
  narrower KEY waits within the same single-write burst, so default must not
  regress).
- `cargo build -p roastty --tests` — no warnings.
  `cargo fmt -p roastty -- --check` — clean. No-ghostty grep on the changed line
  — clean. `git diff --check` — clean.

**Pass** = 3/3 clean full-suite runs at `--test-threads=4` with `surface_start`
passing every run. **Partial/Fail** = `surface_start` still fails, or a new
failure appears.

## Scope and what remains

This fixes the **one** echo-defeated KEY. It does **not** yet let us drop the
thread cap: the `surface_key`×2 and `surface_mouse` tests are a **different**
class — they pass at threads=4 and fail at _default_ parallelism (a
render-under- load issue, not a KEY bug), so making them robust at any
parallelism (the goal of dropping the cap) needs its own diagnosis-first
experiment (Exp 835: capture their default-parallelism snapshots, as was done
here). Only once 835 lands can the gate move off `--test-threads=4`.
`config_path_cli` (rare, non-PTY env/path) remains separately.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Independently verified the test body, the helper signature
(`needle: &str`), the `SSNAP` evidence, the 10-site audit, `cargo fmt --check`,
and the isolation run.

**Verdict:** APPROVED, no Required findings. Confirmed: the diagnosis is
evidence-backed (a screen showing `owned` alone can only be the echo, since the
child's single `printf` write renders `current_dir:owned` together); the fix is
sound; `$(pwd) == current_dir` on this system is proven empirically by the
sibling `surface_start_uses_working_directory` test which already keys a bare
`pwd` on `current_dir`; the scope boundary (surface*key/mouse fail at the
\_opposite* setting, so they are a genuinely different class for Exp 835) is
honest. Four Optional/Nit findings, all adopted:

- **Combined-token KEY.** Key on `format!("{}:owned", current_dir)` rather than
  the bare prefix, so both asserted halves are guaranteed present at return (no
  prefix-only torn read). Done.
- **Count fix.** Ten `_after_start_until` `let text =` sites, not nine;
  corrected (conclusion unchanged — only the `owned` site keys on echoed input).
- **Default-parallelism confirmation.** Added a default-parallelism run to the
  verification so the loop is closed.
- **Char-count nit.** `current_dir` is 32 chars / `current_dir:owned` 38;
  corrected (no-wrap conclusion unaffected).

## Result

**Result:** Pass

The KEY change landed (`let needle = format!("{}:owned", current_dir…)` →
`_until(&needle)`). Build clean (no warnings), fmt clean, no-ghostty clean,
`git diff --check` clean; isolation pass.

| run                | STATUS                | result               | `surface_start` |
| ------------------ | --------------------- | -------------------- | --------------- |
| threads=4 #1       | COMPLETED rc=0 241s   | 4360 passed / 0 fail | **ok**          |
| threads=4 #2       | COMPLETED rc=0 195s   | 4360 passed / 0 fail | **ok**          |
| threads=4 #3       | COMPLETED rc=0 176s   | 4360 passed / 0 fail | **ok**          |
| default (no-regr.) | COMPLETED rc=101 231s | 4357 / 3 fail        | **ok**          |

- **3/3 clean at threads=4** (`surface_start ... ok` each run; 0 panics, 0
  `PoisonError`) — versus 2/3 failing in Exp 833. The echo-defeated KEY is
  fixed: keying on `"<current_dir>:owned"` waits for the child's real output,
  never the echo.
- **No default-parallelism regression:** `surface_start ... ok` at default too.
  The 3 default failures (`config_path_cli`,
  `surface_key_default_natural_text_editing`,
  `surface_key_default_performable_action`) are the **separate** contention/echo
  class that threads=4 already masks and that this experiment does not touch.
- All runs `START=`/`END=`/`CMD=` stamped, none hit `HARD_TIMEOUT`/`IDLE_KILL`.

## Conclusion

The Exp 833 residual is gone: it was my Exp 832 KEY error (waiting for a token
that the PTY echoes before the child runs), not contention or a wrap — proven by
capturing the snapshot first, exactly as the result review insisted. **The
threads=4 gate is now green** (3/3 clean).

Remaining work to satisfy the user's goal of dropping the thread cap (tests
robust at _any_ parallelism):

- **Exp 835 (next):** `surface_key_default_natural_text_editing` and
  `surface_key_default_performable_action` — fail at **default** parallelism
  (pass at threads=4). Diagnose-first (capture their default-parallelism
  snapshots, as done here) before fixing; earlier glimpses suggest a lost
  key-echo render (`^E`) under load, but that must be **observed**, not assumed.
- **Exp 836 (later):** `config_path_cli` — rare non-PTY `$HOME`/cwd flake.

Once 835 (and 836) land, the suite is green at default parallelism and the gate
can drop `--test-threads=4`; then feature work (URI/regex, remaining `os/`)
resumes.

## Completion Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED — no Required/Optional/Nit findings.**
Independently confirmed: the diff is exactly the one-site KEY change (4-line
comment + `let needle` + `_until(&needle)`), test-code only, fmt/no-ghostty
clean; v1/v2/v3 each `4360 passed; 0 failed` with `surface_start ... ok`, 0
panics, 0 PoisonError, no timeout; v-default has `surface_start ... ok` with
exactly the 3 out-of-scope failures (config + surface_key×2); and the fix is
correct **by construction** (both asserted substrings are contained in the keyed
needle, which appears only in the child's single-write output, never the echo —
not 3/3 luck). Scope boundary and non-overclaim of default-green confirmed
honest.
