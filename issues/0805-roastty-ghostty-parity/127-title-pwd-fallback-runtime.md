# Experiment 127: Title PWD Fallback Runtime

## Description

`RUNTIME-009B2B2B` still groups several terminal leftovers together. Experiment
126 proved configured/static surface titles and non-empty OSC title dispatch,
but intentionally left Ghostty's empty-title/PWD fallback semantics open.

Pinned Ghostty keeps a `seen_title` flag in `termio/stream_handler.zig`:

- non-empty OSC title sets terminal title, marks `seen_title = true`, and sends
  a surface `.set_title` message;
- empty OSC title resets `seen_title = false` and sends the terminal PWD as the
  surface title if a PWD exists, otherwise it sends a blank title;
- OSC 7/PWD updates become the title only while `seen_title` is false;
- empty OSC 7/PWD clears PWD and, when `seen_title` is false, blanks the title;
- once an explicit non-empty title is seen, later PWD changes must not replace
  it until an empty title resets the flag.

Roastty currently stores terminal title and PWD, and Experiment 126 added
callback-safe title propagation through `TermioPump`, but the terminal state
does not model the `seen_title` fallback rule. Roastty also currently stores OSC
7 payloads as its terminal PWD string; this experiment will prove the fallback
state machine against that stored PWD value. It will not claim full Ghostty OSC
7 URI parsing, local-host validation, or path normalization parity; those stay
in the remaining terminal gap.

This experiment will split a narrow title fallback row out of
`RUNTIME-009B2B2B`:

- `RUNTIME-009B2B2B1`: `Oracle complete` for the stored-PWD title fallback state
  machine and empty title app dispatch.
- `RUNTIME-009B2B2B2`: `Gap` for exact nonzero scrollback byte quota, OSC 7 URI
  parsing/hostname/path normalization, remaining shell-specific startup rewrite
  coverage, and other remaining terminal behavior effects.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Add title state equivalent to Ghostty's `seen_title` flag.
  - Add explicit pending title-update events that record each Ghostty-style
    surface title message in order, even when the effective title string is
    unchanged.
  - Make non-empty OSC title mark the title as explicitly seen.
  - Make empty OSC title clear the explicit-title flag and fall back to the
    current stored PWD, or blank if no PWD exists.
  - Make PWD updates drive the title only while no explicit title is active.
  - Make PWD clear blank the title only while no explicit title is active.
  - Expose a drain method for pending title updates so Termio can emit title
    events in order without installing callbacks on worker-owned terminals.
  - Reset the explicit-title flag on full terminal reset.
  - Add terminal-core tests for PWD-before-title fallback, explicit-title
    suppression of later PWD changes, empty-title reset back to PWD, and PWD
    clear blanking after fallback.
  - Add terminal-core tests for the Ghostty no-op dispatch edges: empty title
    with no PWD still queues a blank title event when the stored title is
    already blank, and empty title with a PWD still queues a PWD title event
    when the stored title already equals that PWD.
- `roastty/src/termio.rs`
  - Replace the current title-string diff pump signal with the terminal's
    explicit pending title events so empty/no-op title messages still emit
    through `TermioPump.titles`.
  - Add worker/PTY tests proving empty title and PWD/title fallback messages,
    including blank, same-string fallback, and multiple-message dispatches,
    still emit through `TermioPump.titles` without terminal callbacks.
- `roastty/src/lib.rs`
  - Update the surface pump title path so empty title resets dispatch
    `ROASTTY_ACTION_SET_TITLE` when no static `title` config is set.
  - Keep configured static titles suppressing every PTY title message, including
    empty-title resets.
  - Add focused surface tests for empty title dispatch and static-title
    suppression of fallback/empty title messages.
- `issues/0805-roastty-ghostty-parity/title_pwd_fallback_runtime_parity.py`
  - Add a static guard checking pinned Ghostty's `seen_title`, empty-title, PWD
    fallback, and PWD clear markers against Roastty's terminal, Termio, surface,
    and test markers.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-009B2B2B` into the complete stored-PWD title fallback row and
    the remaining terminal gap row.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the runtime inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate from the runtime inventory script so CFG-223 reflects the split.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add this experiment link and update Learnings after implementation.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml terminal_stream_title_pwd_fallback`
- `cargo test --manifest-path roastty/Cargo.toml termio_title_pwd_fallback`
- `cargo test --manifest-path roastty/Cargo.toml surface_title_pwd_fallback`
- `cargo test --manifest-path roastty/Cargo.toml worker_rejects_terminal_with_callbacks`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/title_pwd_fallback_runtime_parity.py`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
- A matrix assertion inside
  `issues/0805-roastty-ghostty-parity/title_pwd_fallback_runtime_parity.py`
  verifies:
  - `RUNTIME-009B2B2B1` is `Oracle complete`;
  - `RUNTIME-009B2B2B1` evidence names stored-PWD title fallback, empty-title
    reset, explicit-title suppression of PWD changes, blank and same-string
    empty-title dispatch, empty title app dispatch, and the static parity guard;
  - `RUNTIME-009B2B2B2` remains `Gap`;
  - `RUNTIME-009B2B2B2` still names exact nonzero scrollback byte quota, OSC 7
    URI parsing/hostname/path normalization, remaining shell-specific startup
    rewrite coverage, and other remaining terminal behavior effects;
  - CFG-223 remains `Gap` until all runtime/UI rows are closed.
- `prettier --check --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/127-title-pwd-fallback-runtime.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
- `cargo fmt --manifest-path roastty/Cargo.toml -- --check`
- `git diff --check`
- No generated `__pycache__` remains under the issue directory.

Fail criteria:

- The implementation installs terminal title callbacks on worker-owned terminals
  or weakens `TermioWorker::spawn` callback rejection.
- The split claims full OSC 7 URI parsing/hostname validation/path normalization
  parity.
- Empty title resets are still suppressed at the app surface when no static
  title is configured.
- Empty title resets only emit when the effective terminal title string changes;
  Ghostty dispatches these reset messages even when the resulting string is
  blank or equal to the prior title.
- Static configured titles fail to suppress empty-title or PWD-fallback title
  app actions.
- The generated inventory or matrix marks CFG-223 `Pass` while
  `RUNTIME-009B2B2B2` or any other runtime/UI row remains a gap.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Initial verdict: **Changes required**.

The reviewer found one required issue: the design did not account for Ghostty's
no-op title dispatch behavior. Pinned Ghostty sends `.set_title` for empty-title
resets even when the resulting title is blank or equal to the current title,
while Roastty's current pump signal is based on before/after title string
differences.

The design was updated to require explicit pending title-update events in
terminal state, drained through `TermioPump.titles`, and tests for blank
empty-title dispatch plus same-string PWD fallback dispatch.

Re-review verdict: **Approved**.

The reviewer confirmed the required finding was resolved and reported no new
required findings.

## Result

**Result:** Pass

Roastty now matches the pinned Ghostty title/PWD fallback state machine for the
stored terminal PWD slice covered by this experiment:

- PWD updates become the surface title until an explicit non-empty title is
  seen;
- explicit non-empty titles suppress later PWD title changes;
- empty OSC title resets explicit-title state and falls back to the stored PWD
  or blank;
- clearing PWD blanks the title while no explicit title is active;
- blank and same-string empty-title reset messages still queue explicit title
  events instead of relying on title string changes;
- multiple title messages in one parse/read cycle are preserved in order instead
  of being coalesced;
- `TermioPump.titles` drains the explicit pending title events without
  installing terminal callbacks;
- empty title resets dispatch `ROASTTY_ACTION_SET_TITLE` when no static title is
  configured;
- configured static titles suppress empty-title and PWD-fallback title app
  actions.

Verification passed:

- `cargo test --manifest-path roastty/Cargo.toml terminal_stream_title_pwd_fallback`
  — 3 passed.
- `cargo test --manifest-path roastty/Cargo.toml termio_title_pwd_fallback` — 3
  passed.
- `cargo test --manifest-path roastty/Cargo.toml surface_title_pwd_fallback` — 3
  passed.
- `cargo test --manifest-path roastty/Cargo.toml worker_rejects_terminal_with_callbacks`
  — 1 passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/title_pwd_fallback_runtime_parity.py`
  — passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/surface_title_runtime_parity.py`
  — passed after updating the prior guard for the explicit title-event pump
  path.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  — regenerated 36 runtime rows: 29 Oracle complete, 31 closed, 5 incomplete, 5
  gaps; CFG-223 remains Gap.
- `cargo fmt --manifest-path roastty/Cargo.toml -- --check` — passed.
- `git diff --check` — passed.

`RUNTIME-009B2B2B` is split into `RUNTIME-009B2B2B1` and `RUNTIME-009B2B2B2`.
`RUNTIME-009B2B2B1` is `Oracle complete` for the stored-PWD title fallback state
machine and empty title app dispatch. `RUNTIME-009B2B2B2` remains `Gap` for
exact nonzero scrollback byte quota, OSC 7 URI parsing/hostname/path
normalization, remaining shell-specific startup rewrite coverage, and other
remaining terminal behavior effects.

## Conclusion

Empty-title/PWD fallback is no longer part of the broad terminal leftovers gap
for Roastty's stored terminal PWD value. Full Ghostty OSC 7 URI parsing,
local-host validation, and path normalization are still unproven and remain in
the follow-up terminal gap.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

Initial verdict: **Changes required**.

The reviewer found one required issue: pending title updates were stored as a
single `Option<String>`, so multiple Ghostty-style title messages in one
parse/read cycle would be coalesced. That contradicted the design requirement to
record each title message.

The implementation was updated to store pending title updates as an ordered
`Vec<String>`, propagate them through `TermioPump.titles`, dispatch every title
event at the surface, and add terminal, Termio, and surface tests for multiple
title messages in one parse/read cycle.

Re-review verdict: **Approved**.

The reviewer confirmed the prior required finding was resolved and reported no
new required findings. It verified the ordered pending title update storage,
`TermioPump.titles` propagation, ordered surface dispatch, the three focused
multi-event regression tests, the static guard, prettier check, Rust format
check, and `git diff --check`.
