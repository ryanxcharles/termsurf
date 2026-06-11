+++
implementer = "codex"
review_design = "codex-adversarial"
+++

# Experiment 95: Phase F — TERM and enquiry-response config

## Description

Port the next pinned upstream config fields from
`vendor/ghostty/src/config/Config.zig` into `roastty/src/config/mod.rs`:

- `term: []const u8 = "xterm-ghostty"`
- `enquiry-response: []const u8 = ""`

These fields are adjacent immediately after `faint-opacity` upstream and before
the Linux-only `async-backend` field. This experiment is parser/formatter-only
for the new fields. Runtime `TERM` propagation into launched child processes,
ENQ (`0x05`) response behavior, app C ABI exposure, async backend selection, and
platform-specific IO runtime behavior remain later work.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config` fields in the current local config struct/default region:
    - `term: String`
    - `enquiry_response: String`
  - Initialize defaults to upstream values:
    - `term = "xterm-ghostty"`
    - `enquiry_response = ""`
  - Format the fields after `faint-opacity` and before the next existing
    formatter entry, preserving upstream order in formatter output and
    format-order tests. Do not reorder unrelated existing struct fields.
  - Route both fields through the existing string setter semantics used by local
    scalar string config:
    - a supplied value stores the string;
    - an empty value resets to the default;
    - a missing value reports `ValueRequired`;
    - embedded NUL reports `InvalidValue`.
  - Extend default-value, formatter output, string-route, diagnostics,
    format-order, and clone/equality tests.

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed` in the experiment index.
  - After implementation, add an operating note describing the parser-only
    status and runtime work left open.

## Verification

Before implementation:

- Codex-native adversarial design review approves the experiment.
- Plan commit exists before source edits begin.

After implementation:

- `cargo fmt`
- `cargo test -p roastty term_enquiry`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
- `cargo fmt --check`
- `git diff --check`

Pass criteria:

- `term` and `enquiry-response` are present in defaults, formatter output,
  `Config::set`, and format-order tests in the current local formatter region.
- String parsing uses local config semantics: supplied values store exactly,
  empty values reset to default, missing values diagnose as `ValueRequired`, and
  embedded NUL values diagnose as `InvalidValue`.
- Runtime child-process `TERM` propagation and ENQ response behavior are not
  claimed or changed by this experiment.

## Design Review

Codex-native adversarial reviewer `019eb594-e9a4-7543-92c2-085aa755116f`
returned **Approved** with no findings.
