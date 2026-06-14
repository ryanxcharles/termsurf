# Experiment 10: Command Palette Default Format Parity

## Description

Experiment 9 left one default-config formatter gap: the pinned Ghostty fixture
and Roastty both emit 88 normalized `command-palette-entry` lines, but one
default entry differs by escaped text formatting.

The remaining normalized mismatch is:

```text
Ghostty: command-palette-entry = title:"{App}",description:"Put a little {App} in your terminal.",action:"text:\xf0\x9f\x91\xbb"
Roastty: command-palette-entry = title:"{App}",description:"Put a little {App} in your terminal.",action:"text:\\xf0\\x9f\\x91\\xbb"
```

The upstream source stores that command as `.{ .text = "👻" }` in
`vendor/ghostty/src/input/Command.zig`, and upstream action formatting prints it
as `text:\xf0\x9f\x91\xbb` in `vendor/ghostty/src/input/Binding.zig`. Roastty's
default command table currently stores the already-escaped action text and then
escapes the backslashes again during config formatting.

This experiment will close only this default `command-palette-entry` formatter
gap. It will not claim full command-palette parser parity, runtime command
palette UI parity, menu/action dispatch parity, or custom command-palette
config-file parity.

## Changes

- `roastty/src/config/mod.rs`
  - Store the default Ghostty command-palette text action in the same semantic
    form as upstream, so formatter escaping emits the pinned Ghostty text.
  - Update the focused command-palette config test to expect the semantic action
    value and the upstream escaped formatter output.
  - Tighten `config_default_format_oracle` so it compares all normalized default
    config lines exactly, including `command-palette-entry`.
  - Assert the default `command-palette-entry` ordered output matches Ghostty
    exactly, with 88 entries and 0 multiset mismatches.
- `issues/0805-roastty-ghostty-parity/default-config-oracle.md`
  - Record that the default-config oracle now covers every default config line
    after app-name normalization.
  - Update the current counts and remove the command-palette gap note if the
    oracle proves exact parity.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Update `CFG-213` so the comparable default surface is the full normalized
    default config output.
  - Update `CFG-215` from `Gap` to `Pass` only if the command-palette output
    matches the pinned Ghostty fixture exactly.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning about Ghostty text-action command-palette formatting if the
    experiment proves it.
  - Update the Experiment 10 status after the result is known.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle -- --nocapture`
  passes and proves:
  - Ghostty raw default output and Roastty raw default output have the same
    number of lines.
  - All normalized default config lines match exactly and in order.
  - Ghostty and Roastty each emit 88 `command-palette-entry` lines.
  - The ordered normalized `command-palette-entry` lines match exactly.
  - The normalized `command-palette-entry` multiset mismatch count is 0.
- `cargo test --manifest-path roastty/Cargo.toml command_palette_entry_config_parse_format_reset_and_diagnose -- --nocapture`
  passes and proves the focused default entry, parser, formatter, reset, clone,
  and diagnostic behavior covered by the existing command-palette test still
  works after the representation fix.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --write --prose-wrap always --print-width 80` has been run on the
  changed issue markdown files.
- `git diff --check` passes.
- Matrix updates do not mark command-palette UI dispatch, custom runtime
  behavior, or general config precedence as passing from this formatter-only
  evidence.

Suggested commands:

```bash
ROASTTY_DEFAULT_CONFIG_OUT=/Users/astrohacker/dev/termsurf/logs/issue805-exp10-roastty-default-config.txt \
  cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle -- --nocapture
cargo test --manifest-path roastty/Cargo.toml command_palette_entry_config_parse_format_reset_and_diagnose -- --nocapture
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/10-command-palette-default-format-parity.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/default-config-oracle.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context adversarial design review approved the plan with no findings.

Reviewer verdict:

```text
VERDICT: APPROVED

No Required findings.

No Optional findings or Nits.
```

## Result

**Result:** Pass

Roastty's default `command-palette-entry` output now matches the pinned Ghostty
macOS default output exactly after the existing app-name normalization. The
default-config oracle now compares every normalized default config line in
order, including `keybind` and `command-palette-entry`.

Key changes:

- `roastty/src/config/mod.rs`
  - Changed the built-in Ghostty command-palette text action from pre-escaped
    ASCII text to the semantic UTF-8 text payload.
  - Tightened `config_default_format_oracle` to compare all normalized default
    config lines exactly and in order.
  - Asserted ordered `command-palette-entry` equality, 88 entries on each side,
    and 0 command-palette multiset mismatches.
  - Updated the focused command-palette config test to expect the upstream
    formatter output for the default text action.
- Issue docs
  - Updated the default-config oracle, config matrix, README learning, and
    Experiment 10 status.

Current oracle counts:

- Ghostty raw default output: 635 lines.
- Roastty raw default output: 635 lines.
- Normalized ordered lines match after app-name normalization: true.
- Normalized multiset mismatches: 0.
- Ghostty `keybind` lines: 93.
- Roastty `keybind` lines: 93.
- `keybind` ordered match: true.
- `keybind` multiset mismatches: 0.
- Ghostty `command-palette-entry` lines: 88.
- Roastty `command-palette-entry` lines: 88.
- `command-palette-entry` ordered match: true.
- `command-palette-entry` multiset mismatches: 0.
- Total missing normalized lines: 0.
- Total extra normalized lines: 0.

Verification:

```bash
ROASTTY_DEFAULT_CONFIG_OUT=/Users/astrohacker/dev/termsurf/logs/issue805-exp10-roastty-default-config.txt \
  cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle -- --nocapture
cargo test --manifest-path roastty/Cargo.toml command_palette_entry_config_parse_format_reset_and_diagnose -- --nocapture
cargo fmt --manifest-path roastty/Cargo.toml --check
git diff --check
```

Evidence:

- `logs/issue805-exp10-roastty-default-config.txt`
- `logs/issue805-exp10-default-config-diff-summary.txt`

## Conclusion

Default config formatter parity is now exact for the full pinned Ghostty default
output after app-name normalization. This does not prove general command-palette
parser parity, command-palette UI dispatch, config-file precedence, reload
behavior, or runtime effects; those remain separate Issue 805 audit surfaces.

## Completion Review

Fresh-context adversarial completion review approved the result with no required
findings.

Reviewer verdict:

```text
VERDICT: APPROVED

No Required findings.
```

Accepted nit:

- Updated the stale default-config oracle regeneration example from the
  Experiment 9 Roastty output log path to the Experiment 10 log path.
