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

# Experiment 835: Fix surface_key natural_text — assert the byte, not the racy echo

## Description

`surface_key_default_natural_text_editing_writes_legacy_bytes` (`lib.rs:16345`)
fails at **default** parallelism (passes at threads=4). The `_until` panic dump
from the Exp 834 default run shows the real cause:

```
condition not met after 30s, latest snapshot: "           05   …newlines"
```

The screen shows **`05`** — od's hex of the legacy byte the key wrote — but the
test waits for and asserts **`"^E"`**, which never arrives under load. The child
is `stty -echo -icanon min 1 time 0; dd bs=1 count=1 | od -An -tx1 -v`: it sends
Super+ArrowRight (legacy byte `0x05`), `dd` reads the one byte, `od` prints
`05`.

`"^E"` is **not** child output — it is the terminal's **echo** of the `0x05`
byte, rendered only in the brief window **before `stty -echo` takes effect**.
The child disables echo precisely so the hex is the only output; the `"^E"` echo
is a **race**. In isolation/threads=4 the key usually wins the race (echo
renders, test passes); under default load it loses it (no echo → only `05` → the
wait for `"^E"` times out). So the test asserts a racy artifact while the
deterministic legacy byte (`05`) is right there.

This is the **only** `surface_snapshot_text_until` site asserting a pre-`stty`
echo where the deterministic od output already renders. (The sibling
`surface_key_default_performable_action…` also asserts a racy echo, but its od
output does **not** render even with a matched byte count — a separate, deeper
problem deferred to **Exp 836**.)

## Changes

`roastty/src/lib.rs` (test code only), one test, two lines:

```
let text = surface_snapshot_text_until(app, surface, "^E");
assert!(text.contains("^E"), "{text:?}");
    →
let text = surface_snapshot_text_until(app, surface, "05");
assert!(text.contains("05"), "{text:?}");
```

Wait for and assert the **deterministic** od hex `05` (the legacy byte `0x05`
that reached the child) instead of the racy pre-`stty` echo `"^E"`. Robustness
does not rest on the single captured run: `od` writes a complete ` 05\n` line in
one flush (no torn partial), and the data path that produces it (surface → pty →
`dd` → `od`) is **independent of echo timing**, so `05` renders whenever the
byte round-trips — which it must for the test to mean anything. The captured
default-load snapshot (`05` present while `^E` is absent) is confirmation of
that, not the sole basis. The child command and the key event are unchanged —
the test still verifies "Super+ArrowRight writes legacy byte 0x05", now via the
byte itself rather than its incidental echo. No production code change.

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked task
per run, no poll-watcher):

- **Targeted, fast:** the test passes in isolation after the change.
- **Reproduce-the-fix at the failing setting:** the full suite at **default**
  parallelism (where this test failed in Exp 834) run **3×** —
  `surface_key_default_natural_text_editing_writes_legacy_bytes` is `... ok`
  every run. (The other default failures — `performable_action` and
  `config_path_cli` — are expected to remain; this experiment fixes only
  natural_text. Pass is judged on natural_text, not a green suite.)
- `cargo build -p roastty --tests` — no warnings.
  `cargo fmt -p roastty -- --check` — clean. No-ghostty grep on the changed
  lines — clean. `git diff --check` — clean.

**Pass** = `natural_text_editing` passes 3/3 at default parallelism (it was
failing there). **Partial/Fail** = it still fails, or a new failure appears.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Verified the snapshot evidence, ran the isolation slice (current
test still passes in isolation — echo wins the race there, matching the
premise), and checked `cargo fmt --check`.

**Verdict:** APPROVED, no Required findings. Confirmed: the diagnosis is sound
(`od -tx1` emits hex `05` and can never emit caret notation, so `^E` is
necessarily the pre-`stty` echo of byte 0x05); the fix is semantically faithful
and arguably stronger (`stty -echo` proves the author did not intend to test the
echo; `05` is the byte's full round-trip); the `performable_action` deferral to
Exp 836 is a genuinely different command-design bug (`min 8`/`count 8` vs 6
bytes → od never flushes). One Optional, adopted: the robustness claim now cites
the atomic ` 05\n` flush + echo-timing-independent data path, not just the
single captured run. (Nit: `contains("05")` is a bare substring but safe on the
otherwise-blank screen — left as-is.)

## Result

**Result:** Pass

The two-line change landed (`_until`/assert needle `"^E"` → `"05"`). Build clean
(no warnings), fmt clean, no-ghostty clean, `git diff --check` clean; isolation
pass.

| run (default) | STATUS                | `natural_text` | other failures (out of scope)                        |
| ------------- | --------------------- | -------------- | ---------------------------------------------------- |
| #1            | COMPLETED rc=101 185s | **ok**         | performable_action                                   |
| #2            | COMPLETED rc=101 180s | **ok**         | performable_action                                   |
| #3            | COMPLETED rc=101 110s | **ok**         | performable_action, config_path_cli, bell_audio_path |

- **`natural_text_editing` is `... ok` 3/3 at default parallelism** — the
  setting where it failed in Exp 834. Keying on the deterministic od byte `05`
  instead of the racy pre-`stty` echo `"^E"` makes it robust. All runs
  `START`/`END`/`CMD` stamped, none hit `HARD_TIMEOUT`/`IDLE_KILL`.
- The other default failures are the expected, separately-scoped ones: the
  `performable_action` sibling (Exp 836), and the config `$HOME`/cwd flakes —
  which I now see are **two** tests of the same class:
  `config_path_cli_expands_…` and
  `bell_audio_path_expands_from_file_cli_home_…`.

## Conclusion

Another racy-echo assertion replaced with the deterministic byte it was really
verifying. natural_text is green at default and threads=4 (and isolation).

Remaining before the gate can drop `--test-threads=4` (default-green):

- **Exp 836 (next):** `surface_key_default_performable_action…` — the harder
  sibling: its `dd`/`od` output never renders (its `stty min 8`/`count 8` waits
  for 8 bytes while Shift+ArrowLeft writes only 6), so it relies entirely on the
  racy echo. Needs the command redesigned so a deterministic output renders, or
  the assertion re-grounded. Diagnose first (the od path did not flush even at
  `min 6`/`count 6` in isolation — understand why before fixing).
- **Exp 837 (later):** the config `$HOME`/cwd flake **class** —
  `config_path_cli` **and** `bell_audio_path` (non-PTY, env/cwd dependent), fix
  together.

Once 836 + 837 land, the suite is green at default parallelism and the gate
drops the thread cap; then feature work resumes.

## Completion Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Confirmed the diff is exactly the two-line `"^E"`→`"05"` change
(test-only, no production), `natural_text ... ok` 3/3 at default
(`CMD=cargo test -p roastty`, no timeouts), and that run-3's extra
`config::tests` failures (`config_path_cli`, `bell_audio_path`) are env/cwd
tests untouched by this diff (cannot be caused by it). **Verdict: CHANGES
REQUIRED → fixed.**

- **Required — stale README index status.** The 835 index line still read
  `Designed`. **Fixed:** flipped to `Pass`.
- **Nit — table elapsed inconsistency.** **Fixed:** all rows now cite the
  `STATUS=` wall-clock (185/180/110 s).
- **Nit — "any parallelism" overgeneralized.** **Fixed:** softened to "default
  and threads=4 (and isolation)".
