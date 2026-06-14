# Experiment 126: Surface Title Runtime Split

## Description

`RUNTIME-009B2B2` still combines several unrelated terminal leftovers: nonzero
scrollback byte-quota parity, configured/static title behavior, remaining
shell-specific startup rewrites, and other terminal behavior toggles. Pinned
Ghostty's surface-title behavior is separable:

- if `title` is configured, surface initialization sends a `.set_title` app
  action with that configured static title;
- if no static title is configured and the launch command is a direct command,
  surface initialization sends a `.set_title` app action with argv[0];
- shell commands do not get command-derived static titles;
- non-empty terminal title messages are ignored when a static config title is
  set;
- config updates with a configured `title` send a `.set_title` app action with
  the new configured title.

Roastty already has typed app action plumbing for `ROASTTY_ACTION_SET_TITLE`,
test action recording for title payloads, parsed `title` config storage, and
terminal title state. It does not yet have a callback-safe live PTY path for
terminal title changes: `TermioWorker::spawn` correctly rejects terminals with
effect callbacks installed, so this experiment must not install `title_changed`
callbacks on worker-owned terminals. Instead, it should carry a title-change
signal through `TermioPump`, analogous to the existing bell count path.

This experiment will split configured/static surface-title runtime behavior out
of `RUNTIME-009B2B2`. It will not claim exact nonzero scrollback byte-quota
parity, shell-specific startup rewrite parity beyond the already split
shell-integration slice, or the broader macOS titlebar/window walkthrough in
`RUNTIME-011`.

## Changes

- `roastty/src/termio.rs`
  - Extend `TermioPump` with a terminal-title change field that is populated
    after PTY output changes the terminal title.
  - Keep `TermioWorker::spawn` callback rejection intact; do not install
    terminal title callbacks for worker-owned terminals.
- `roastty/src/lib.rs`
  - Add surface startup/update logic that dispatches `ROASTTY_ACTION_SET_TITLE`
    for configured static titles and direct command argv[0] titles, matching the
    pinned Ghostty startup branches.
  - Dispatch non-empty PTY/OSC title changes through the new pump title field
    only when no static `title` is configured.
  - Add focused runtime tests for configured title startup, direct-command
    fallback, shell-command no-op, config-update title dispatch, non-empty OSC
    title dispatch without a static title, and non-empty OSC title suppression
    with a static title.
- `issues/0805-roastty-ghostty-parity/surface_title_runtime_parity.py`
  - Add a static guard that checks pinned Ghostty's configured title,
    direct-command title, static-title suppression, and config-update title
    markers against Roastty's runtime/test markers.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-009B2B2` into:
    - `RUNTIME-009B2B2A`: `Oracle complete` for configured/static surface-title
      startup/update and non-empty OSC title suppression/dispatch behavior.
    - `RUNTIME-009B2B2B`: `Gap` for exact nonzero scrollback byte quota,
      empty-title/PWD fallback semantics, remaining shell-specific startup
      rewrite coverage, and other remaining terminal behavior effects.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the runtime inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate from the runtime inventory script so CFG-223 reflects the split.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after implementation with any
    durable finding.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml surface_title_runtime`
- `cargo test --manifest-path roastty/Cargo.toml termio_title`
- `cargo test --manifest-path roastty/Cargo.toml worker_rejects_terminal_with_callbacks`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/surface_title_runtime_parity.py`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
- A matrix assertion inside
  `issues/0805-roastty-ghostty-parity/surface_title_runtime_parity.py` verifies:
  - `RUNTIME-009B2B2A` is `Oracle complete`;
  - `RUNTIME-009B2B2A` evidence and guard command name configured title, direct
    command title, non-empty OSC title dispatch, static title suppression, and
    the static parity guard;
  - `RUNTIME-009B2B2B` remains `Gap`;
  - `RUNTIME-009B2B2B` still names exact nonzero scrollback byte quota,
    empty-title/PWD fallback semantics, remaining shell-specific startup rewrite
    coverage, and other remaining terminal behavior effects;
  - CFG-223 remains `Gap` until all runtime/UI rows are closed.
- `prettier --check --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/126-surface-title-runtime-split.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
- `cargo fmt --manifest-path roastty/Cargo.toml -- --check`
- `git diff --check`
- No generated `__pycache__` remains under the issue directory.

Fail criteria:

- The implementation installs terminal title callbacks on worker-owned terminals
  or weakens `TermioWorker::spawn` callback rejection.
- The split claims exact nonzero `scrollback-limit` byte-quota parity or
  remaining shell-specific startup rewrite parity.
- Static configured titles do not suppress subsequent non-empty OSC title app
  actions.
- The generated inventory or matrix marks CFG-223 `Pass` while
  `RUNTIME-009B2B2B` or any other runtime/UI row remains a gap.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Initial verdict: **Changes required**.

The reviewer found one required issue: the design overclaimed OSC title
suppression/dispatch because pinned Ghostty also has empty-title reset and OSC 7
PWD fallback semantics. The design was narrowed to non-empty OSC title
dispatch/suppression, and `RUNTIME-009B2B2B` now explicitly keeps
empty-title/PWD fallback semantics as a remaining gap.

Re-review verdict: **Approved**.

The reviewer confirmed the required finding was resolved and reported no new
required findings.
