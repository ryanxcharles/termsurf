+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 463: grow the Config struct with the shell-integration group

## Description

Continuing the incremental growth of the aggregating `Config` struct
(Experiments 461–462), this experiment adds the **shell-integration config**
group: `shell_integration` and `shell_integration_features` — both
already-ported leaf types (`ShellIntegration`, `ShellIntegrationFeatures`). It
adds the two fields and their upstream `Config`-field defaults to `Config` and
its `Default`. The parser and the rest of upstream `Config` stay deferred.

## Upstream behavior

In `config/Config.zig`, the shell-integration group's field defaults:

```zig
@"shell-integration": ShellIntegration = .detect,
@"shell-integration-features": ShellIntegrationFeatures = .{},
```

`shell-integration` defaults to `.detect` (auto-detect the shell);
`shell-integration-features` defaults to `.{}` (the struct's own field defaults:
`cursor`, `title`, `path` are `true`; `sudo`, `ssh-env`, `ssh-terminfo` are
`false`).

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
pub(crate) struct Config {
    // ... clipboard group (461), mouse/click group (462) ...
    /// `shell-integration`.
    pub shell_integration: ShellIntegration,
    /// `shell-integration-features`.
    pub shell_integration_features: ShellIntegrationFeatures,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // ... clipboard, mouse/click ...
            shell_integration: ShellIntegration::Detect,
            shell_integration_features: ShellIntegrationFeatures::default(),
        }
    }
}
```

The defaults are upstream's Config-field defaults: `shell-integration` `Detect`,
and `shell-integration-features` the struct's own `Default` (the `.{}` literal —
`ShellIntegrationFeatures::default()`, ported in Experiment 452).

## Scope / faithfulness notes

- **Ported (bridged)**: the shell-integration field group of the aggregating
  `Config` struct (upstream `config.Config`) — the two fields and their
  `Default`.
- **Faithful**: the two fields use the already-ported types (`ShellIntegration`,
  `ShellIntegrationFeatures`); their `Default` values match upstream's
  Config-field defaults (`.detect`; `.{}` =
  `ShellIntegrationFeatures::default()`).
- **Faithful adaptation**: the `shell-integration-features` field default `.{}`
  (the struct's own field defaults) maps to
  `ShellIntegrationFeatures::default()` (which Experiment 452 implemented to
  match those field defaults). The struct continues to grow one coherent field
  group per experiment. The derive set (`Clone`/`PartialEq`) is unchanged.
- **Deferred**: the rest of upstream `Config`'s fields (added group by group in
  later slices), the parser, the `changeConfig` machinery, and the
  conditional-config system. (Consumed by later slices; this experiment grows
  the struct with the shell-integration group.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add the two fields `shell_integration: ShellIntegration` and
     `shell_integration_features: ShellIntegrationFeatures` to `Config`, and
     their defaults (`Detect`; `ShellIntegrationFeatures::default()`) to the
     `Default` impl.
2. Tests (in `config/mod.rs`):
   - extend the `Config::default()` assertion for the new fields:
     `shell_integration == ShellIntegration::Detect`,
     `shell_integration_features == ShellIntegrationFeatures::default()`; the
     existing group defaults still hold.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty config_default
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Config` gains the two shell-integration fields, and `Config::default()` sets
  their upstream defaults (`shell-integration` `Detect`;
  `shell-integration-features` `ShellIntegrationFeatures::default()`) while the
  earlier group defaults still hold — a faithful partial of upstream's `Config`;
- the tests pass (the new defaults; the existing defaults), and the existing
  tests still pass;
- the rest of upstream `Config` and the parser stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a default is wrong, a field uses the wrong type, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream:
`shell_integration = ShellIntegration::Detect` matches the `.detect` default
(`Config.zig:2813`);
`shell_integration_features = ShellIntegrationFeatures::default()` is the
correct mapping for upstream `.{}` because that struct's field defaults are
`cursor/title/path = true` and `sudo/ssh-env/ssh-terminfo = false`
(`Config.zig:2858` / `:8672`); adding these as a group is coherent (the leaf
enum and flag struct are already in place and the defaults are self-contained);
and the test plan is adequate (assert the two new defaults and keep checking the
existing group defaults so `Config::default()` stays stable as it grows).

Review artifacts:

- Prompt: `logs/codex-review/20260604-121149-d463-prompt.md` (design)
- Result: `logs/codex-review/20260604-121149-d463-last-message.md` (design)
