# Experiment 137: Clipboard Device Attributes Runtime

## Description

`RUNTIME-009B2B2B3B2B2B` still groups other remaining terminal behavior effects.
One concrete unproven terminal config effect in that gap is the
`clipboard-write` influence on primary device attributes.

Pinned Ghostty stores `clipboard-write` in `termio.DerivedConfig`, updates it
through `StreamHandler.changeConfig`, and uses it when answering primary device
attributes (`CSI c` / `DECID`):

- if `clipboard-write != deny`, the response is `CSI ? 62 ; 22 ; 52 c`;
- if `clipboard-write = deny`, the response is `CSI ? 62 ; 22 c`.

Roastty already parses and uses `clipboard-write` for app-level clipboard write
policy, OSC 52, and kitty clipboard flows. However, the PTY-backed terminal
device-attributes path currently uses the default terminal response without
parsed `clipboard-write` state, so normal worker terminals do not expose the
same config-driven clipboard capability bit as pinned Ghostty.

This experiment will split the remaining terminal row:

- `RUNTIME-009B2B2B3B2B2B1`: **Oracle complete** for `clipboard-write` primary
  device-attributes runtime effects, including startup config and live config
  update wiring.
- `RUNTIME-009B2B2B3B2B2B2`: **Gap** for other remaining terminal behavior
  effects.

This experiment will not claim broader clipboard policy parity; that is already
owned by `RUNTIME-001`. It only closes the terminal capability advertisement
effect.

## Changes

- `roastty/src/terminal/device_attributes.rs`
  - Add a helper for primary attributes with clipboard access enabled or
    disabled, preserving the existing default encoding behavior where useful.
- `roastty/src/terminal/terminal.rs`
  - Add terminal-owned clipboard-write state.
  - Add a runtime setter for config updates.
  - Use the configured clipboard-write state in primary device attributes when
    no embedded device-attributes callback is installed.
  - Preserve the existing embedded callback path for direct terminal users.
  - Add focused terminal tests for allow/ask advertising `52`, deny omitting
    `52`, runtime updates, `DECID`, and callback compatibility.
- `roastty/src/termio.rs`
  - Add clipboard-write to `TermioSpawnOptions`.
  - Pass it into `TerminalInitOptions`.
  - Update existing PTY response tests and add a PTY-backed runtime test proving
    a child-visible primary device-attributes response follows configured
    clipboard-write.
- `roastty/src/lib.rs`
  - Thread parsed `Config.clipboard_write` into initial surface Termio spawn
    options.
  - Update existing live surfaces when app config changes so device-attributes
    responses use the latest parsed policy.
  - Add or extend focused app/surface config tests for startup and update
    propagation.
- `issues/0805-roastty-ghostty-parity/clipboard_device_attributes_runtime_parity.py`
  - Add a static guard checking pinned Ghostty markers: `clipboard_write`,
    `changeConfig`, `self.clipboard_write = config.clipboard_write`,
    `clipboard_write != .deny`, `"\x1B[?62;22;52c"`, and `"\x1B[?62;22c"`.
  - Check Roastty markers for parser coverage, terminal owned clipboard-write
    state, primary device-attributes formatting, Termio spawn wiring, app config
    startup/update wiring, focused runtime tests, and the inventory split.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-009B2B2B3B2B2B` into the clipboard device-attributes complete
    row and the reduced remaining-terminal gap row.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223 static guards that hard-code current runtime row counts or
  the remaining terminal gap row
  - Update expected counts after the split: 46 runtime rows, 39 Oracle complete
    rows, 41 closed rows, and 5 remaining runtime gaps.
  - Update references from the old remaining terminal gap row to
    `RUNTIME-009B2B2B3B2B2B2`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- Pinned Ghostty evidence shows `clipboard-write` is stored on the active stream
  handler, updated through `changeConfig`, and used to include or omit primary
  device-attributes feature `52`.
- Roastty terminal core includes feature `52` for `clipboard-write = allow` and
  `ask`, and omits it for `deny`.
- `DECID` follows the same primary response as `CSI c`.
- The existing embedded device-attributes callback path is preserved.
- PTY-backed `Termio` runtime proves a child-visible primary device-attributes
  response using parsed spawn options.
- Initial app/surface config and live config updates both propagate
  `clipboard-write` to the active terminal runtime.
- `RUNTIME-009B2B2B3B2B2B1` is Oracle complete and cites terminal, Termio,
  app/surface, and static guard evidence.
- `RUNTIME-009B2B2B3B2B2B2` remains `Gap` for other remaining terminal behavior
  effects.
- `CFG-223` remains `Gap`.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml terminal_stream_device_attributes_clipboard_write
cargo test --manifest-path roastty/Cargo.toml termio_device_attributes_clipboard_write
cargo test --manifest-path roastty/Cargo.toml surface_device_attributes_clipboard_write
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/clipboard_device_attributes_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
cargo fmt --manifest-path roastty/Cargo.toml
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/137-clipboard-device-attributes-runtime.md
git diff --check
```

Fail criteria:

- The device-attributes effect is only proven through parser/default tests.
- PTY-backed terminals still answer primary device attributes with a hard-coded
  response regardless of `clipboard-write`.
- Runtime config update changes stored config but not the active terminal
  response.
- The experiment promotes broader clipboard policy parity or unrelated terminal
  behavior from the remaining gap.
- CFG-223 is marked complete.

## Design Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Verdict:** Approved.

The reviewer found no required, optional, or nit findings.
