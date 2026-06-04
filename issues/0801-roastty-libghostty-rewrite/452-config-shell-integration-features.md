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

# Experiment 452: the shell-integration-features config type (ShellIntegrationFeatures)

## Description

This experiment ports the `shell-integration-features` config type —
`ShellIntegrationFeatures`, a six-flag struct toggling individual shell
integration features. It is the companion to Experiment 451's `ShellIntegration`
(which selects which shell's integration to inject); this type toggles which
features that integration provides. Its intrinsic field defaults are mixed
(`cursor`, `title`, `path` default `true`; `sudo`, `ssh_env`, `ssh_terminfo`
default `false`), so the `Default` impl is hand-written, like the earlier flag
structs (Experiments 437/448/450). The termio consumer reads the flags directly;
the integration injection that consumes them stays deferred.

## Upstream behavior

In `config/Config.zig`, the type and its `Config` field (default `.{}`):

```zig
@"shell-integration-features": ShellIntegrationFeatures = .{},

/// Shell integration features
pub const ShellIntegrationFeatures = packed struct {
    cursor: bool = true,
    sudo: bool = false,
    title: bool = true,
    @"ssh-env": bool = false,
    @"ssh-terminfo": bool = false,
    path: bool = true,
};
```

`ShellIntegrationFeatures` has six independent flags toggling features of the
injected shell integration: `cursor` (shell cursor reporting, default `true`),
`sudo` (sudo wrapping, default `false`), `title` (window-title updates, default
`true`), `ssh-env` (SSH environment propagation, default `false`),
`ssh-terminfo` (SSH terminfo install, default `false`), and `path` (PATH
adjustments, default `true`). The `Config` field default `.{}` adopts the
struct's own field defaults.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// The `shell-integration-features` config (upstream `ShellIntegrationFeatures`):
/// which features the injected shell integration provides. Defaults: `cursor`,
/// `title`, `path` are `true`; `sudo`, `ssh_env`, `ssh_terminfo` are `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShellIntegrationFeatures {
    /// Shell cursor reporting.
    pub cursor: bool,
    /// `sudo` wrapping.
    pub sudo: bool,
    /// Window-title updates.
    pub title: bool,
    /// SSH environment propagation (upstream `ssh-env`).
    pub ssh_env: bool,
    /// SSH terminfo install (upstream `ssh-terminfo`).
    pub ssh_terminfo: bool,
    /// PATH adjustments.
    pub path: bool,
}

impl Default for ShellIntegrationFeatures {
    /// Upstream's field defaults: `cursor`, `title`, `path` are `true`; `sudo`,
    /// `ssh_env`, `ssh_terminfo` are `false`.
    fn default() -> Self {
        Self {
            cursor: true,
            sudo: false,
            title: true,
            ssh_env: false,
            ssh_terminfo: false,
            path: true,
        }
    }
}
```

The hand-written `Default` matches upstream's field defaults; a derived
`Default` would make every flag `false`. The hyphenated upstream tags `ssh-env`
/ `ssh-terminfo` map to `ssh_env` / `ssh_terminfo`. The flags are independent
`bool`s; the consumer reads them directly (no method).

## Scope / faithfulness notes

- **Ported (bridged)**: the `ShellIntegrationFeatures` config type
  (`config/Config.zig`), with its intrinsic field defaults.
- **Faithful**: the struct has the six upstream flags (`cursor`, `sudo`,
  `title`, `ssh-env`, `ssh-terminfo`, `path`); the `Default` is
  `cursor = title = path = true`, `sudo = ssh_env = ssh_terminfo = false`
  (upstream's field defaults).
- **Faithful adaptation**: upstream is a `packed struct` (bit-packed storage);
  in Rust it is a plain value struct (no ABI involved — internal config), so a
  derived layout is fine. The `Default` is hand-written because Rust's derived
  `Default` for `bool` is `false`, not upstream's mixed defaults. The hyphenated
  `ssh-env` / `ssh-terminfo` tags map to `ssh_env` / `ssh_terminfo`. No method
  is extracted — the consumer is plain field access.
- **Deferred**: the string parsing, the `formatEntry`, the `Config` struct that
  holds the key, and the termio shell-integration injection that reads the
  flags. (Consumed by a later slice; this experiment lands the value type and
  its defaults.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add
     `pub(crate) struct ShellIntegrationFeatures { pub cursor: bool, pub sudo: bool, pub title: bool, pub ssh_env: bool, pub ssh_terminfo: bool, pub path: bool }`
     (derive `Debug, Clone, Copy, PartialEq, Eq`) and a hand-written
     `impl Default` (`cursor`/`title`/`path` `true`;
     `sudo`/`ssh_env`/`ssh_terminfo` `false`).
2. Tests (in `config/mod.rs`):
   - `ShellIntegrationFeatures::default()` has `cursor`, `title`, `path` `true`
     and `sudo`, `ssh_env`, `ssh_terminfo` `false`; a value with all flags
     flipped differs from the default and round-trips `Copy`/`Eq`; the flags are
     independent (a value differing only in `sudo` is `!=` the default).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty shell_integration_features
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `ShellIntegrationFeatures` has the six upstream flags and the `Default`
  matches upstream's mixed field defaults — faithful to upstream's type;
- the tests pass (the default; the independent flags; `Copy`/`Eq`), and the
  existing tests still pass;
- the parsing, the `Config` struct, and the termio injection stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a flag is missing/extra or misnamed, the `Default`
is wrong (a flag with the wrong default), an unrelated item changes, or any
public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream: the six fields match
exactly (`cursor`, `sudo`, `title`, `ssh-env`, `ssh-terminfo`, `path`,
`Config.zig:8672`); `ssh_env` / `ssh_terminfo` are the right Rust identifiers
for the hyphenated Zig field names; the hand-written default is exact
(`cursor`/`title`/`path = true`, `sudo`/`ssh-env`/`ssh-terminfo = false`, and
the `Config` field default `.{}` uses those field defaults, `Config.zig:2858`);
not extracting a method is the right call (upstream passes the flag struct
through to shell-integration setup and reads fields directly there,
`shell_integration.zig:188`); and the tests (the non-derived mixed default,
value semantics, flag independence) are adequate.

Review artifacts:

- Prompt: `logs/codex-review/20260604-112519-d452-prompt.md` (design)
- Result: `logs/codex-review/20260604-112519-d452-last-message.md` (design)

## Result

**Result:** Pass

The shell-integration-features config type is now live.

- `roastty/src/config/mod.rs`:
  `pub(crate) struct ShellIntegrationFeatures { pub cursor, sudo, title, ssh_env, ssh_terminfo, path: bool }`
  (upstream `ShellIntegrationFeatures`; the hyphenated `ssh-env` /
  `ssh-terminfo` map to `ssh_env` / `ssh_terminfo`) with a hand-written
  `impl Default` (`cursor`/`title`/`path` `true`;
  `sudo`/`ssh_env`/`ssh_terminfo` `false`) — upstream's mixed field defaults (a
  derived `Default` would make every flag `false`). No method — the termio
  injection reads the flags directly.

Test (in `config/mod.rs`): `shell_integration_features_default_mixed_flags` —
`default()` has `cursor`/`title`/`path` `true` and
`sudo`/`ssh_env`/`ssh_terminfo` `false`; an all-flipped value differs from the
default; the flags are independent (`{ sudo: true, ..default() } != default()`);
`Copy`/`Eq`.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2940 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

The config layer now carries `ShellIntegrationFeatures` — the companion to
Experiment 451's `ShellIntegration`, completing the shell-integration config
pair (which shell, and which features). It is the fourth flag struct (after
`FontShapingBreak`, `ScrollToBottom`, `NotifyOnCommandFinishAction`) with
hand-written intrinsic field defaults, and the first with mixed (`true` and
`false`) per-field defaults. The string parsing, the `Config` struct, and the
termio shell-integration injection that reads the flags stay deferred. The
config-type family — now thirteen enums/flag-structs with consumers plus three
color value types — remains a clean, gated way to advance the rewrite while the
larger coupled subsystems stay deferred.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed `ShellIntegrationFeatures` faithfully ports all
six upstream flags with the correct Rust names for `ssh-env` / `ssh-terminfo`;
the hand-written `Default` preserves the upstream mixed defaults
(`cursor`/`title`/`path = true`, the other three `false`); no helper method is
needed (field consumption belongs in the later termio injection port); and the
test covers the mixed default, value semantics, and flag independence. No public
C ABI/header impact; nothing needed to change before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-112709-r452-prompt.md` (result)
- Result: `logs/codex-review/20260604-112709-r452-last-message.md` (result)
