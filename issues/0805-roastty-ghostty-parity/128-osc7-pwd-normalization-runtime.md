# Experiment 128: OSC 7 PWD Normalization Runtime

## Description

Experiment 127 proved the stored-PWD title fallback state machine, but it
intentionally left the incoming OSC 7 payload semantics open. Pinned Ghostty
does not store the raw OSC 7 URL as the working directory: `reportPwd` accepts
only `file` and `kitty-shell-cwd` URLs, requires a local hostname, normalizes
the URL path, stores the path as terminal PWD, reports that path to the surface,
and uses that path for title fallback when no explicit title has been seen.

Roastty currently accepts the raw OSC 7 URL as PWD. This experiment will split a
narrow runtime row out of `RUNTIME-009B2B2B2` for the deterministic OSC 7 PWD
normalization behavior, without claiming the remaining broad terminal gaps.

The new split will be:

- `RUNTIME-009B2B2B2A`: OSC 7 local `file` and `kitty-shell-cwd` PWD URI
  validation, hostname checks, path normalization, surface PWD reporting, and
  title fallback path dispatch.
- `RUNTIME-009B2B2B2B`: remaining terminal gaps, including exact nonzero
  scrollback byte quota, remaining shell-specific startup rewrite coverage,
  unproven exotic OSC 7 URI edge cases, and other remaining terminal behavior
  effects.

The terminal OSC parser can continue to return the raw OSC 7 payload. The
normalization belongs in the terminal/report-PWD handling path, matching
Ghostty's separation between OSC parsing and `StreamHandler.reportPwd`.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Replace raw non-empty `report_pwd` storage with a Ghostty-style
    normalization helper.
  - Accept `file://<local-host>/<path>` and
    `kitty-shell-cwd://<local-host>/<path>`.
  - Reject unsupported schemes, missing hostnames, non-local hosts, and invalid
    encoded `file` paths without mutating terminal PWD or title state.
  - Preserve Ghostty's empty-path behavior for otherwise valid local URIs such
    as `file://localhost`.
  - Percent-decode `file` URL path bytes while leaving `kitty-shell-cwd` paths
    raw.
  - Keep the existing empty-PWD clear behavior from Experiment 127.
  - Add terminal tests for accepted local paths, percent decoding,
    `kitty-shell-cwd` raw paths, invalid/remote rejection, PWD events, and title
    fallback.
- `roastty/src/termio.rs`
  - Add explicit PWD pump propagation, such as pending terminal PWD updates plus
    a `TermioPump` PWD field, so OSC 7 PWD changes can reach live surfaces
    independently from title updates.
  - Add or update PTY pump tests proving normalized OSC 7 paths reach
    `TermioPump` PWD events and `TermioPump.titles` without terminal callbacks.
- `roastty/src/lib.rs`
  - Add or update surface tests proving normalized OSC 7 PWD changes dispatch
    the path, not the raw URL, to live surfaces and title fallback.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-009B2B2B2` into the new Oracle-complete OSC 7 row and a
    reduced remaining gap row.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate the runtime inventory.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate the CFG-223 summary; it must remain `Gap`.
- `issues/0805-roastty-ghostty-parity/osc7_pwd_normalization_runtime_parity.py`
  - Add a static guard checking the pinned Ghostty markers and Roastty
    implementation/test markers for the OSC 7 normalization path.

## Verification

Pass criteria:

- Local OSC 7 `file://localhost/...` payloads store and dispatch only the
  normalized path.
- `file` paths percent-decode `%xx` sequences and reject invalid encodings.
- Local `kitty-shell-cwd://localhost/...` payloads store and dispatch the raw
  path bytes after the host.
- Unsupported schemes, missing hostnames, invalid `file` encodings, and
  non-local hostnames leave the previous PWD/title state unchanged. Empty
  normalized paths from otherwise valid local URIs are accepted, matching
  Ghostty.
- Title fallback from OSC 7 uses the normalized path, and Experiment 127's title
  event ordering remains intact.
- The runtime inventory contains an Oracle-complete OSC 7 row and keeps the
  remaining terminal row plus CFG-223 as `Gap`.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml terminal_stream_osc7_pwd_normalization
cargo test --manifest-path roastty/Cargo.toml termio_osc7_pwd_normalization
cargo test --manifest-path roastty/Cargo.toml surface_osc7_pwd_normalization
cargo test --manifest-path roastty/Cargo.toml terminal_stream_title_pwd_fallback
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/osc7_pwd_normalization_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/128-osc7-pwd-normalization-runtime.md
git diff --check
```

Fail criteria:

- Roastty still stores or dispatches the raw OSC 7 URL for accepted local `file`
  or `kitty-shell-cwd` payloads.
- Rejected OSC 7 payloads mutate terminal PWD, surface PWD events, or title
  fallback state.
- The experiment marks exact nonzero scrollback quota, remaining shell rewrites,
  unproven exotic OSC 7 URI cases, or CFG-223 as complete.

## Design Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Initial verdict:** Changes required.

Required findings:

- The plan incorrectly rejected missing or empty paths, while pinned Ghostty
  accepts an otherwise valid local URI such as `file://localhost` and stores the
  empty normalized path.
- The plan expected normalized PWD values to reach `TermioPump.pwd`, but Roastty
  had no explicit PWD pump field or dispatch path and the design did not add
  one.

Fixes:

- Removed missing-path rejection from the plan and made Ghostty's empty-path
  behavior an explicit pass criterion.
- Added explicit PWD pump propagation to the implementation plan, including
  pending terminal PWD updates, a `TermioPump` PWD field/event, and surface
  dispatch tests.

**Re-review verdict:** Approved. No Required findings remain.

## Result

**Result:** Pass.

Implemented the OSC 7 PWD normalization runtime slice:

- Roastty now validates non-empty OSC 7 PWD reports through a Ghostty-style
  scheme/local-host/path normalizer instead of storing the raw URL.
- Accepted local `file` URLs store and dispatch a percent-decoded path.
- Accepted local `kitty-shell-cwd` URLs store and dispatch the raw path after
  the host.
- Unsupported schemes, missing hostnames, non-local hostnames, invalid `file`
  encodings, and invalid UTF-8 decoded paths leave the previous PWD/title state
  unchanged.
- Valid local empty-path reports such as `file://localhost` are accepted and
  clear the logical PWD, matching Ghostty's normalized empty-path behavior.
- Terminal PWD changes are queued as explicit pending PWD events, drained into
  `TermioPump.pwd`, emitted by the worker, and dispatched to live surfaces as
  `ROASTTY_ACTION_PWD`.
- Title fallback continues to use pending title events, now with normalized PWD
  paths instead of raw URLs.

The runtime inventory split is now:

- `RUNTIME-009B2B2B2`: **Oracle complete** for common local OSC 7 PWD URI
  validation, hostname checks, path normalization, surface PWD dispatch, and
  title fallback path dispatch.
- `RUNTIME-009B2B2B3`: **Gap** for exact nonzero scrollback byte quota,
  remaining shell-specific startup rewrite coverage, unproven exotic OSC 7 URI
  edge cases, and other remaining terminal behavior effects.

Verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml terminal_stream_osc7_pwd_normalization
cargo test --manifest-path roastty/Cargo.toml termio_osc7_pwd_normalization
cargo test --manifest-path roastty/Cargo.toml surface_osc7_pwd_normalization
cargo test --manifest-path roastty/Cargo.toml terminal_stream_title_pwd_fallback
cargo test --manifest-path roastty/Cargo.toml termio_title_pwd_fallback
cargo test --manifest-path roastty/Cargo.toml surface_title_pwd_fallback
cargo test --manifest-path roastty/Cargo.toml terminal_stream_osc
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/osc7_pwd_normalization_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
cargo fmt --manifest-path roastty/Cargo.toml --check
git diff --check
```

The regenerated runtime inventory reports:

```text
runtime_rows=37
oracle_complete=30
closed=32
audit_covered=0
incomplete=5
gap=5
cfg223=Gap
```

## Conclusion

Roastty no longer treats accepted OSC 7 PWD reports as raw URLs. The common
local `file` and `kitty-shell-cwd` paths now match Ghostty's state, surface PWD
dispatch, and title fallback behavior with durable Tier 2 guards. CFG-223
correctly remains `Gap` because exact nonzero scrollback byte quotas, remaining
shell rewrite coverage, exotic OSC 7 URI edge cases, and other terminal effects
still need separate experiments.

## Completion Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Initial verdict:** Changes required.

Required finding:

- Broader `terminal_stream_osc` tests still seeded OSC 7 PWD with non-local
  `file://host/...` fixtures and expected raw URL values. The focused experiment
  filters passed, but the broader OSC suite failed after local-host validation
  was added.

Fix:

- Updated the OSC stream tests that seed PWD via OSC 7 to use
  `file://localhost/...` and assert normalized paths such as `/home` and
  `/split`.
- Left direct `set_pwd_for_tests("file://host/...")` formatter tests unchanged,
  because those tests bypass OSC 7 parsing and exercise stored terminal state
  serialization.
- Added `cargo test --manifest-path roastty/Cargo.toml terminal_stream_osc` to
  the verification evidence for this result.

**Re-review verdict:** Approved. The reviewer verified `terminal_stream_osc`,
`terminal_stream_osc7_pwd_normalization`, and `git diff --check` passed, and
reported no remaining Required findings.
