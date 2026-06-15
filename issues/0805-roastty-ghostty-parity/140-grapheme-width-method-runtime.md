# Experiment 140: Grapheme Width Method Runtime

## Description

Ghostty commit `2c62d182cec246764ff725096a70b9ef44996f7f` maps
`grapheme-width-method` into the terminal's default modes during `Termio`
initialization. `unicode` enables DEC mode 2027 (`grapheme_cluster`), while
`legacy` leaves it disabled. Ghostty stores those default modes as both the
current mode state and the reset/default state, so RIS/full reset restores the
configured DEC 2027 default rather than the static table default.

Roastty already parses, formats, and validates `grapheme-width-method`, but the
parsed value is not yet wired into the PTY-backed terminal runtime. This
experiment will close the startup runtime slice of the remaining terminal
CFG-223 gap by passing the parsed config through Roastty's surface, termio, and
terminal startup path.

This experiment is intentionally scoped to startup behavior. Pinned Ghostty sets
this default mode in `Termio.init`; it does not appear to update the active
terminal mode from `Termio.changeConfig`, so live reload behavior remains
outside this experiment unless source review proves otherwise during
implementation.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Add a terminal init option for default-mode overrides, or an equivalent
    `default_modes` initialization path, that can set both current and reset
    defaults for DEC 2027.
  - Add focused unit coverage proving the default remains legacy-compatible for
    direct terminal construction, while explicit init options can enable or
    disable the mode and RIS/full reset restores that configured default.
- `roastty/src/termio.rs`
  - Add a `TermioSpawnOptions` field for the initial grapheme-cluster mode.
  - Pass it into `TerminalInitOptions`.
  - Add PTY-backed tests proving the option reaches the terminal.
- `roastty/src/lib.rs`
  - Pass `config.grapheme_width_method.grapheme_cluster()` into termio startup
    options when a surface starts its PTY.
  - Add or extend surface-level tests proving parsed default/unicode config
    starts with grapheme clustering enabled and `legacy` starts with it
    disabled.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Split a completed `grapheme-width-method` startup row out of
    `RUNTIME-009B2B2B3B2B2B2B2`.
  - Leave remaining terminal runtime gaps in a reduced follow-up row.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Update the CFG-223 runtime coverage counts after the inventory split.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning that `grapheme-width-method` is a startup terminal default
    mode in the pinned Ghostty source unless future evidence shows live reload
    behavior.
- `issues/0805-roastty-ghostty-parity/grapheme_width_method_runtime_parity.py`
  - Add a static guard that checks the pinned Ghostty termio switch and the
    Roastty startup wiring, tests, and inventory status.

## Verification

Pass criteria:

- `cargo fmt --manifest-path roastty/Cargo.toml -- --check`
- `cargo test --manifest-path roastty/Cargo.toml grapheme_width_method_runtime`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/grapheme_width_method_runtime_parity.py`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
- `git diff --check`

The experiment passes only if the tests prove both config values map to the same
default terminal mode as Ghostty's `Termio.init`, that RIS/full reset restores
the configured DEC 2027 default for both `unicode` and `legacy`, the static
guard rejects loss of that wiring, and CFG-223 remains honest about any
remaining terminal runtime gap.

## Design Review

Fresh-context adversarial design review initially returned **Changes required**:
the first design could be satisfied by setting only the current
`Mode::GraphemeCluster` bit, missing Ghostty's default-mode semantics where
`Termio.init` passes `default_modes` into `Terminal.init` and terminal reset
restores those defaults. This design was updated to require current-and-default
mode initialization plus RIS/full-reset verification.

Re-review returned **Approved**. The reviewer confirmed the prior finding is
resolved because the design now requires current-and-default DEC 2027
initialization and reset verification for both `unicode` and `legacy`.

## Result

**Result:** Pass

Roastty now passes the parsed `grapheme-width-method` value through surface
startup, `TermioSpawnOptions`, and `TerminalInitOptions`. The terminal mode
initialization uses a default-mode path that updates both current and
reset/default DEC 2027 state, matching pinned Ghostty's `default_modes` behavior
instead of only flipping the live mode bit.

Verification completed:

- `cargo fmt --manifest-path roastty/Cargo.toml -- --check` — pass.
- `cargo test --manifest-path roastty/Cargo.toml grapheme_width_method_runtime`
  — pass: 3 tests passed, 0 failed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/grapheme_width_method_runtime_parity.py`
  — pass.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  — pass: `runtime_rows=49`, `oracle_complete=42`, `closed=44`, `incomplete=5`,
  `gap=5`, `cfg223=Gap`.
- `git diff --check` — pass.

CFG-223 remains a gap overall, but the completed runtime inventory count moved
from 41 to 42 oracle-complete rows. The remaining broad terminal behavior row
was renamed to `RUNTIME-009B2B2B3B2B2B2B3`.

## Conclusion

`grapheme-width-method` is now covered for startup terminal runtime parity.
Pinned Ghostty treats it as a terminal default mode rather than a live reload
setting, so Roastty's durable guard focuses on startup plus direct reset/RIS
restoration for both `unicode` and `legacy`. Other terminal behavior effects
remain in the reduced `RUNTIME-009B2B2B3B2B2B2B3` gap.

## Completion Review

Fresh-context adversarial completion review returned **Approved** with no
findings. The reviewer independently ran the focused Rust test, formatter check,
static parity guard, inventory generation against `/tmp` outputs, and
`git diff --check`, and confirmed the result commit had not yet been made.
