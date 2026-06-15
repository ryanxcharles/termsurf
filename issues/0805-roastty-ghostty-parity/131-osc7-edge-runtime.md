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
