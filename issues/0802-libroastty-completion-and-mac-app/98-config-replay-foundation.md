# Experiment 98: Phase F — config replay foundation

## Description

Add the first replay foundation that upstream uses for theme loading and
conditional reload.

Upstream `Config.loadTheme()` cannot simply read a theme file into the current
config: theme values must be lower priority than the user's config. It therefore
loads the theme into a fresh `Config`, then replays the user's prior config
inputs on top. The same replay list is also the basis for
`changeConditionalState()`.

Roastty currently applies config lines directly and discards the input stream.
That makes faithful theme loading impossible without guessing which values came
from user config versus defaults. This experiment should add replay recording
for ordinary config entries while preserving current parser behavior. It should
not load themes yet.

This is intentionally a foundation slice, not the full upstream `Replay` port.
It should record the ordinary `key = value` / `--key=value` inputs needed for
theme overlay ordering. Later experiments can extend the replay model with
conditional entries, diagnostics, explicit path-expansion steps, `-e`, and the
theme loader itself.

## Changes

- `roastty/src/config/mod.rs`
  - Add a small internal replay entry type for ordinary config inputs:
    - config key
    - optional value
    - source (`File` or `Cli`)
  - Add replay storage to `Config`, preserving clone/equality behavior.
  - Record successful entries from:
    - `Config::load_str`
    - `Config::set_cli_args_from_base`
  - Do not record failed entries, comments, blanks, or direct programmatic
    `Config::set` calls.
  - Add an internal helper to replay the recorded ordinary entries onto a fresh
    config without recursively recording them.
  - Keep existing path-expansion behavior unchanged. This experiment should not
    claim complete replay support for path-expansion steps; that remains a later
    extension before full theme loading of relative path-bearing entries.
  - Add tests proving:
    - file and CLI successful entries are recorded in order
    - failed entries are not replay-recorded but diagnostics still behave as
      before
    - direct `Config::set` remains non-recording
    - replaying entries onto a fresh `Config` reconstructs the same values for
      representative scalar, enum, optional, and repeatable fields
    - replaying does not append duplicate replay entries

No theme loading, conditional reload, path-expansion replay step, diagnostic
replay, `-e` replay, or app runtime behavior should be implemented in this
experiment.

## Verification

Pass criteria:

1. `cargo test -p roastty config_replay`
2. `cargo test -p roastty config_set_cli_args_applies_and_collects_diagnostics`
3. `cargo test -p roastty config_load_str_applies_lines_and_collects_diagnostics`
4. `cargo test -p roastty`
5. `cargo fmt --check`
6. `git diff --check`

The full `cargo test -p roastty` run must pass. The existing ABI harness may
print its known enum-conversion warnings, but no new failures are acceptable.

## Design Review

Codex-native adversarial review ran in fresh context with subagent
`019eb5d1-a224-73f0-9116-09bd6593935b`.

Verdict: **APPROVED**

Findings: None.
