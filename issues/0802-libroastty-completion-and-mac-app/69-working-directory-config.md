+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 69: Phase F — working-directory config

## Description

Experiment 68 added the `class` and `x11-instance-name` config surfaces. The
next upstream config field is:

- `working-directory`

Upstream declares `@"working-directory": ?WorkingDirectory = null` in
`vendor/ghostty/src/config/Config.zig`. Roastty already has a `WorkingDirectory`
value type with upstream keyword/path parsing, tilde expansion helper, `value`,
and formatting tests. This experiment wires that type into the aggregating
`Config` only: field, default, parser/reset behavior, formatting, diagnostics,
and focused tests.

Upstream finalize-time behavior is intentionally out of scope. In Ghostty,
`working-directory = null` becomes `inherit` for probable CLI launches and
`home` for desktop launches, with later home-directory lookup and `~/` expansion
during finalize. That cross-field and launcher-context behavior belongs in the
broader config `finalize()` workstream, not this parser/formatter slice.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config::working_directory: Option<WorkingDirectory> = None`.
  - Add `From<WorkingDirectoryParseError> for ConfigSetError`.
  - Route `working-directory` through defaults, `Config::set`, `format_config`,
    clone/equality, and diagnostics using the existing optional value helper.
  - Preserve local formatter order around the upstream sequence:
    - `title`
    - `class`
    - `x11-instance-name`
    - `working-directory`
  - Reuse the existing `WorkingDirectory` parser/formatter and tests rather than
    reimplementing its keyword/path behavior.

Out of scope:

- Config `finalize()` and probable-CLI vs desktop default selection.
- Home directory lookup and automatic `home` → path resolution.
- Applying `window-inherit-working-directory`, `tab-inherit-working-directory`,
  or `split-inherit-working-directory`.
- Runtime surface/app launch inheritance behavior.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/69-working-directory-config.md`
- Run targeted tests:
  - `cargo test -p roastty working_directory_config`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - the default is unset and formats as an empty optional line;
  - `home`, `inherit`, plain paths, quoted paths, and `~/` paths parse and
    format through `Config::set`;
  - empty values reset to unset;
  - missing values and whitespace-only values return `ValueRequired`;
  - `Config::load_str` records `ConfigDiagnostic` line/key/error entries for an
    invalid `working-directory` line while keeping valid neighboring lines;
  - formatter order places `working-directory` after `x11-instance-name`;
  - clone/equality preserves the value.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = `working-directory` is represented faithfully on `Config`,
round-trips through config loading/formatting, matches the existing upstream
keyword/path parser behavior, and has targeted and full tests passing.

**Partial** = the field lands but a parser, diagnostic, or formatter-order edge
needs a follow-up before finalize/runtime use.

**Fail** = `working-directory` cannot be represented faithfully on `Config`
without first porting the broader config `finalize()` path.

## Design Review

Codex adversarial reviewer `019eb3e9-37ad-78b3-8a9d-f06cec0141a2` returned
**Approved** with no findings.

The reviewer verified that the README links Exp69 as `Designed`, the experiment
has the required sections, the scope is narrow and config-only, the plan matches
upstream's `?WorkingDirectory = null`, optional empty-reset behavior, parser
behavior, formatter ordering, and deferred finalize/launcher inheritance, and
the verification plan includes the required formatting, targeted tests, full
`cargo test -p roastty`, `git diff --check`, and clean-status checks.

## Result

**Result:** Pass

Experiment 69 added the config-only `working-directory` surface to
`roastty/src/config/mod.rs`. `Config` now carries
`working_directory: Option<WorkingDirectory>` with the upstream default unset,
routes `working-directory` through `Config::set`, and emits it in
`format_config` after `x11-instance-name`.

The field reuses the existing `WorkingDirectory` parser and formatter. It
accepts `home`, `inherit`, plain paths, quoted paths, and `~/` paths. Empty
values reset to unset. Missing values and whitespace-only direct setter values
report `ValueRequired`; in config-file syntax, `working-directory =` is the
empty-reset path, while a missing-value line `working-directory` records a
`ValueRequired` diagnostic. `Config::load_str` keeps valid neighboring lines.

Finalize-time probable-CLI/home defaults, home-directory lookup, inheritance
booleans, and runtime surface/app launch inheritance remain out of scope.

Verification run:

- `cargo fmt`
- `cargo test -p roastty working_directory_config`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
- `cargo fmt --check`
- `git diff --check`
- `git status --short`

`cargo test -p roastty` passed with 4,504 unit tests, the C ABI harness, and doc
tests. The C ABI harness still emits existing enum-conversion warnings unrelated
to this config change.

## Conclusion

`working-directory` now has a faithful parser/formatter config surface with
defaults, reset behavior, diagnostics, formatter-order coverage, and
clone/equality coverage. The next adjacent upstream fields are the
inherit-working-directory booleans and other window/runtime options, while the
probable-CLI/home defaulting should remain part of the broader config
`finalize()` workstream.

## Completion Review

Codex-native adversarial reviewer `019eb3f0-493d-78e0-8ab9-4eebf1641b79`
returned **Approved** with no findings.

The reviewer checked the completed experiment with fresh context, including the
workflow contract, issue README, experiment file, implementation diff since the
plan commit, `roastty/src/config/mod.rs`, and upstream
`vendor/ghostty/src/config/Config.zig`. The reviewer independently verified
`cargo fmt --check`, `git diff --check`, both targeted test commands, and full
`cargo test -p roastty`, which passed with 4,504 unit tests, the C ABI harness,
and doc tests.
