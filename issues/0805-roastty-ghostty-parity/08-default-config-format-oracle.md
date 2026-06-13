# Experiment 8: Default Config Format Oracle

## Description

Experiment 6 proved that Roastty has all 203 canonical Ghostty config option
names, and Experiment 7 proved the eight Ghostty compatibility map entries. The
canonical rows remain `Gap` because name presence and alias parsing do not prove
defaults, formatting, or formatter order.

This experiment should create a default-config oracle by comparing pinned
Ghostty's `+show-config --default --no-pager` output with Roastty's
`Config::default().format_config(...)` output. The goal is to turn default
format parity from a hard-coded Rust expectation into a reproducible A/B
artifact that future experiments can rerun.

The scope is default config formatting only:

- default values;
- config formatter output for default values;
- config formatter key order;
- documented intentional output differences such as app-name/resource renames.

The scope is not parser acceptance, diagnostics, config-file precedence, runtime
effects, GUI behavior, or changed/non-default config formatting.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused test or helper that emits the default Roastty `format_config`
    text to a deterministic string or fixture.
  - Replace or supplement the hand-maintained
    `config_format_config_emits_fields_in_upstream_order` expectations with an
    oracle that can be regenerated from the pinned Ghostty default output.
  - Keep any normalization explicit and minimal. Do not silently normalize
    semantic values.
- `issues/0805-roastty-ghostty-parity/default-config-oracle.md`
  - Record the Ghostty command used, the Roastty command/test used, the
    normalization rules, raw counts, diff summary, and any accepted intentional
    divergences.
  - Include enough detail for a future agent to regenerate the oracle without
    guessing.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Mark rows `Pass` only for default formatting behavior that the oracle
    proves.
  - Leave parser, diagnostic, precedence, and runtime-effect aspects as `Gap`
    unless separately proven.
  - If a single canonical option row cannot honestly represent partial default
    formatting parity, add a separate row for the default-format oracle rather
    than overclaiming full option parity.
- `issues/0805-roastty-ghostty-parity/divergences.md`
  - Record any intentional default-format divergence with source, reason, user
    impact, and acceptance status.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning if the experiment establishes a reusable Ghostty/Roastty
    default-config comparison recipe.

## Verification

Pass/fail criteria:

- The pinned Ghostty default config output is captured from a reproducible
  command using the resolved pinned executable path:

  ```bash
  GHOSTTY_BIN=/path/to/pinned/vendor/ghostty/Ghostty.app/Contents/MacOS/ghostty
  "$GHOSTTY_BIN" +show-config --default --no-pager
  ```

  `app-runtime=none` is not a valid way to obtain this executable on macOS; it
  is the library-only runtime and does not install `zig-out/bin/ghostty`. If a
  pinned-built Ghostty executable is not already present, use the smallest local
  build command that produces one from `vendor/ghostty`, and record the exact
  command.

- Roastty default formatter output is captured by a deterministic test/helper in
  `roastty/src/config/mod.rs`.
- The experiment records raw and normalized line counts for both outputs.
- The normalized diff is empty, or every remaining diff is recorded as either a
  `Gap` or an accepted intentional divergence.
- Matrix updates do not mark parser, diagnostics, precedence, or runtime effects
  as passing from formatter-only evidence.
- Rust formatting, markdown formatting, focused tests, and diff hygiene pass.

Suggested commands:

```bash
GHOSTTY_BIN=/path/to/pinned/vendor/ghostty/Ghostty.app/Contents/MacOS/ghostty
"$GHOSTTY_BIN" +show-config --default --no-pager \
  > logs/issue805-exp8-ghostty-default-config.txt
cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/08-default-config-format-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/default-config-oracle.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md \
  issues/0805-roastty-ghostty-parity/divergences.md
cargo fmt --manifest-path roastty/Cargo.toml --check
git diff --check
```

If building the Ghostty CLI is too slow for every PR, the result should still
leave a cheaper Tier 1 Roastty fixture test plus a documented Tier 4/manual A/B
refresh command for milestone checks.

## Design Review

Fresh-context adversarial design review approved the design with no required
findings.

Reviewer notes addressed before the plan commit:

- Fixed the suggested Ghostty-output redirect path to write under `logs/` from
  the repo root.
- Reworded the Ghostty command criterion to say the experiment must use the
  resolved pinned executable path, rather than calling the placeholder path
  "preferable".
