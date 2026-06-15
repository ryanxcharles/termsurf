# Experiment 131: OSC 7 Edge Runtime

## Description

`RUNTIME-009B2B2B3B2` still includes unproven exotic OSC 7 URI edge cases and
other remaining terminal behavior effects. Experiment 128 proved the common OSC
7 path: local `file` and `kitty-shell-cwd` URLs, hostname validation,
percent-decoding for `file`, raw paths for `kitty-shell-cwd`, invalid escapes,
missing hosts, remote hosts, empty URL reset, surface PWD dispatch, and title
fallback path dispatch.

Pinned Ghostty's `reportPwd` has a few remaining URI semantics that are not
called out by existing Roastty tests:

- normal `file` path extraction ignores query and fragment suffixes;
- `file` paths decode percent-encoded UTF-8 and encoded slash bytes as path
  bytes;
- `kitty-shell-cwd` uses raw path semantics, so percent escapes remain literal
  and query/fragment suffixes remain part of the raw path;
- a local URL with no slash path stores and dispatches an empty path, matching
  Ghostty's `uri.path.toRawMaybeAlloc` result after host validation.

Roastty's current normalizer appears to implement these behaviors, but the
runtime inventory should not close the edge-case clause without direct guards.
This experiment will add focused terminal-core, Termio pump, and live surface
dispatch tests for the remaining OSC 7 URI edge semantics, plus a static guard
that checks pinned Ghostty's corresponding parse/path markers and Roastty's test
markers.

This experiment will split the terminal row again:

- `RUNTIME-009B2B2B3B2A`: **Oracle complete** for exotic OSC 7 query/fragment,
  UTF-8 percent-decoding, encoded slash, raw kitty path, and empty-path edge
  behavior.
- `RUNTIME-009B2B2B3B2B`: **Gap** for other remaining terminal behavior effects.

This experiment will not claim every terminal behavior path in
`stream_handler.zig`, nor any GUI behavior.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Add focused `terminal_stream_osc7_pwd_edge_*` tests for:
    - `file://localhost/tmp/edge%20name?ignored#ignored` dispatching
      `/tmp/edge name`;
    - UTF-8 percent decoding such as `%E2%82%AC`;
    - encoded slash `%2F` decoding in `file` paths;
    - `kitty-shell-cwd://localhost/tmp/raw%2Fpath?ignored#ignored` preserving
      `%2F` and the raw query/fragment suffix;
    - local no-slash URL behavior dispatching an empty path and title fallback
      event.
- `roastty/src/termio.rs`
  - Add a Termio worker/pump guard proving an OSC 7 edge path travels through
    `TermioPump::pwd` without terminal callbacks.
- `roastty/src/lib.rs`
  - Add or extend a surface dispatch guard proving the normalized edge path is
    emitted through `ROASTTY_ACTION_PWD`.
- `issues/0805-roastty-ghostty-parity/osc7_edge_runtime_parity.py`
  - Add a static guard checking pinned Ghostty's `std.Uri` parse options,
    `raw_path` branch, `uri.path.toRawMaybeAlloc`, host validation, and
    Roastty's edge tests.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-009B2B2B3B2` into the new OSC 7 edge Oracle-complete row and
    a reduced remaining terminal behavior gap.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- Terminal-core tests prove `file` query/fragment trimming, UTF-8 percent
  decoding, encoded slash decoding for `file`, raw percent-preserving
  `kitty-shell-cwd` including query/fragment suffixes, and no-slash empty-path
  behavior.
- Termio and surface tests prove at least one edge path travels through the
  runtime pump and app action dispatch surfaces.
- `RUNTIME-009B2B2B3B2A` becomes Oracle complete.
- `RUNTIME-009B2B2B3B2B` remains `Gap` for other terminal behavior effects.
- `CFG-223` remains `Gap`.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml terminal_stream_osc7_pwd_edge
cargo test --manifest-path roastty/Cargo.toml termio_osc7_pwd_edge
cargo test --manifest-path roastty/Cargo.toml surface_osc7_pwd_edge
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/osc7_edge_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
cargo fmt --manifest-path roastty/Cargo.toml
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/131-osc7-edge-runtime.md
git diff --check
```

Fail criteria:

- Any edge case leaves PWD/title state unchanged when pinned Ghostty would
  accept and normalize it.
- `kitty-shell-cwd` percent escapes are decoded like `file`.
- `file` query or fragment suffixes become part of the stored path, or
  `kitty-shell-cwd` query or fragment suffixes are incorrectly trimmed.
- The inventory claims unrelated terminal behavior effects or CFG-223 complete.

## Design Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Initial verdict:** Changes required.

The reviewer found that the original design incorrectly expected
`kitty-shell-cwd` query and fragment suffixes to be excluded from the path.
Pinned Ghostty enables `raw_path` for `kitty-shell-cwd`, and its URI helper
documents and tests that raw path includes query and fragment text. The design
was updated so normal `file` URLs trim query/fragment suffixes while
`kitty-shell-cwd` preserves them in the raw path.

**Re-review verdict:** Approved.

The reviewer confirmed the corrected design matches pinned Ghostty's raw-path
behavior and reported no new findings.

## Result

**Result:** Pass

Roastty now has focused guards for the remaining OSC 7 URI edge semantics:

- `terminal_stream_osc7_pwd_edge_file_paths_trim_and_decode` proves `file` paths
  trim query/fragment suffixes and decode spaces, UTF-8, and encoded slash
  bytes.
- `terminal_stream_osc7_pwd_edge_kitty_raw_path_keeps_suffixes` proves
  `kitty-shell-cwd` keeps percent escapes plus raw query/fragment suffixes in
  the path, matching pinned Ghostty's raw-path parser mode.
- `terminal_stream_osc7_pwd_edge_no_slash_dispatches_empty_path` proves a local
  no-slash URL dispatches an empty PWD and title fallback event.
- `termio_osc7_pwd_edge_worker_emits_raw_kitty_pwd_pump` proves an edge PWD path
  travels through `TermioPump::pwd`.
- `surface_osc7_pwd_edge_dispatches_raw_kitty_path` proves an edge PWD path
  dispatches through `ROASTTY_ACTION_PWD`.
- `osc7_edge_runtime_parity.py` statically checks pinned Ghostty's `reportPwd`
  and raw-path URI markers plus Roastty's edge guards and CFG-223 row split.

`config_runtime_inventory.py` now splits the old remaining terminal row into
`RUNTIME-009B2B2B3B2A` as Oracle complete for OSC 7 edge behavior and
`RUNTIME-009B2B2B3B2B` as the reduced remaining terminal behavior gap. The
generated CFG-223 summary remains `Gap` with 40 runtime rows, 33 Oracle complete
rows, 35 closed rows, and 5 remaining runtime gaps.

Verification run:

```bash
cargo test --manifest-path roastty/Cargo.toml terminal_stream_osc7_pwd_edge
cargo test --manifest-path roastty/Cargo.toml termio_osc7_pwd_edge
cargo test --manifest-path roastty/Cargo.toml surface_osc7_pwd_edge
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/osc7_edge_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/surface_title_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/shell_startup_rewrite_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/osc7_pwd_normalization_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/scrollback_byte_limit_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/title_pwd_fallback_runtime_parity.py
```

## Conclusion

The OSC 7 edge slice is no longer part of the CFG-223 gap. Pinned Ghostty's
important distinction is that `file` URLs use normal decoded path semantics,
while `kitty-shell-cwd` uses raw-path semantics that preserve percent escapes
and query/fragment suffixes. Roastty now matches that behavior in terminal
state, worker pump output, and app action dispatch. The remaining CFG-223 work
should continue from `RUNTIME-009B2B2B3B2B`, which is limited to other terminal
behavior effects.

## Completion Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Verdict:** Approved.

The reviewer reported no Required findings. It independently ran the focused OSC
7 edge Rust tests, `osc7_edge_runtime_parity.py`, the updated parity scripts,
`cargo fmt --manifest-path roastty/Cargo.toml --check`, and `git diff --check`.
It also confirmed the runtime inventory split: 40 runtime rows, 33 Oracle
complete rows, 35 closed rows, 5 remaining gaps, new split IDs present, and the
old split ID absent.
