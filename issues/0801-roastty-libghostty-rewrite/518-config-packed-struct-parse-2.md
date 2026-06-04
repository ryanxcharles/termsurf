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

# Experiment 518: the remaining packed-struct flag parsers (ShellIntegrationFeatures / NotifyOnCommandFinishAction parse_cli)

## Description

Completing the packed-struct parse work (Experiment 517 added
`parse_packed_flags` and applied it to `ScrollToBottom` / `FontShapingBreak`),
this experiment applies the same helper as `parse_cli` to the two remaining
packed structs: `ShellIntegrationFeatures` (`shell-integration-features`) and
`NotifyOnCommandFinishAction` (`notify-on-command-finish-action`). With these,
every packed-struct config type parses faithfully.

## Upstream behavior

Both are packed structs of bools, parsed by `cli.args.parsePackedStruct`
(`cli/args.zig:608`) — the behavior ported in Experiment 517: a standalone bool
sets every flag, otherwise a `[no-]flag` comma-list sets named flags (trimmed of
`" \t"`), with defaults for the rest and `error.InvalidValue` for an unknown
name.

The structs and their flag names + defaults (verified against
`config/Config.zig`):

- `ShellIntegrationFeatures` (`Config.zig:8672`): `cursor` (`true`), `sudo`
  (`false`), `title` (`true`), `ssh-env` (`false`), `ssh-terminfo` (`false`),
  `path` (`true`).
- `NotifyOnCommandFinishAction` (`Config.zig:10221`): `bell` (`true`), `notify`
  (`false`).

The flag keyword names are the upstream packed-struct field names — `ssh-env` /
`ssh-terminfo` are kebab-case (the Rust fields are `ssh_env` / `ssh_terminfo`).
These match the keyword strings the existing `entry_flags` formatters already
emit.

## Rust mapping (`roastty/src/config/mod.rs`)

Each struct gets `parse_cli(value) -> Result<Self, FlagsParseError>` using the
`parse_packed_flags` helper, with a single closure mapping each `FlagToken` to
its fields (mirroring Experiment 517):

```rust
impl ShellIntegrationFeatures {
    pub(crate) fn parse_cli(value: &str) -> Result<Self, FlagsParseError> {
        let mut result = ShellIntegrationFeatures::default();
        parse_packed_flags(value, |tok| match tok {
            FlagToken::All(b) => {
                result.cursor = b;
                result.sudo = b;
                result.title = b;
                result.ssh_env = b;
                result.ssh_terminfo = b;
                result.path = b;
                true
            }
            FlagToken::One("cursor", on) => { result.cursor = on; true }
            FlagToken::One("sudo", on) => { result.sudo = on; true }
            FlagToken::One("title", on) => { result.title = on; true }
            FlagToken::One("ssh-env", on) => { result.ssh_env = on; true }
            FlagToken::One("ssh-terminfo", on) => { result.ssh_terminfo = on; true }
            FlagToken::One("path", on) => { result.path = on; true }
            FlagToken::One(_, _) => false,
        })?;
        Ok(result)
    }
}

impl NotifyOnCommandFinishAction {
    pub(crate) fn parse_cli(value: &str) -> Result<Self, FlagsParseError> {
        let mut result = NotifyOnCommandFinishAction::default();
        parse_packed_flags(value, |tok| match tok {
            FlagToken::All(b) => { result.bell = b; result.notify = b; true }
            FlagToken::One("bell", on) => { result.bell = on; true }
            FlagToken::One("notify", on) => { result.notify = on; true }
            FlagToken::One(_, _) => false,
        })?;
        Ok(result)
    }
}
```

The `ssh-env` / `ssh-terminfo` keyword arms set the `ssh_env` / `ssh_terminfo`
Rust fields — the kebab keyword is the upstream field name, the snake field is
the Rust rename. Unmentioned flags keep their `Default` values.

## Scope / faithfulness notes

- **Ported (bridged)**: `parsePackedStruct` (via `parse_packed_flags`), applied
  to `ShellIntegrationFeatures` and `NotifyOnCommandFinishAction` `parse_cli`.
- **Faithful**: standalone-bool shortcut, `[no-]flag` comma-list with `" \t"`
  trimming, defaults for unmentioned fields, `InvalidValue` for an unknown flag
  — exactly upstream; the flag keywords are the upstream field names (kebab for
  the ssh flags), matching the `entry_flags` formatters.
- **Faithful adaptation**: the comptime `inline for (fields)` field match → the
  per-struct closure; `error.InvalidValue` → `FlagsParseError::InvalidValue`.
- **Deferred**: the bool / int / string magic parse paths (float stays blocked);
  the empty-string reset-to-default rule; the per-field `parseIntoField`
  dispatch and the `loadCli` / file loader.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `parse_cli` to `ShellIntegrationFeatures`
   and `NotifyOnCommandFinishAction`.
2. Tests (in `config/mod.rs`): standalone bool sets all flags; a `[no-]flag`
   comma-list sets named flags with defaults for the rest (including the
   `ssh-env` / `ssh-terminfo` kebab keywords → snake fields); an unknown flag is
   `Err(InvalidValue)`; a `format_entry` → `parse_cli` round-trip.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty packed_flags
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- both `parse_cli` match upstream `parsePackedStruct`: standalone bool sets all
  flags, `[no-]flag` comma-list sets named flags (incl. the kebab ssh keywords)
  with defaults for the rest, unknown flag → `InvalidValue`;
- the tests pass (standalone, comma-list, kebab keywords, unknown, round-trip),
  and the existing tests still pass;
- the remaining loader pieces stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the parse diverges from upstream (esp. a kebab
keyword mapped to the wrong field), an unrelated item changes, or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the upstream field names/defaults match exactly —
`ShellIntegrationFeatures` `cursor=true` / `sudo=false` / `title=true` /
`ssh-env=false` / `ssh-terminfo=false` / `path=true` (`Config.zig:8672`) and
`NotifyOnCommandFinishAction` `bell=true` / `notify=false` (`Config.zig:10221`);
the `ssh-env -> ssh_env` / `ssh-terminfo -> ssh_terminfo` field mapping is the
right adaptation as long as the parse keywords stay the upstream field names;
reusing `parse_packed_flags` preserves the upstream packed-struct semantics
(defaults first, raw bool sets all, comma-list with `" \t"` trim and `no-`,
unknown → `InvalidValue`, `args.zig:607`); and the proposed tests are adequate,
especially the kebab-keyword cases and the format/parse round-trip.

Review artifacts:

- Prompt: `logs/codex-review/20260604-175552-d518-prompt.md` (design)
- Result: `logs/codex-review/20260604-175552-d518-last-message.md` (design)

## Result

**Result:** Pass

`parse_cli` was added to `ShellIntegrationFeatures` and
`NotifyOnCommandFinishAction` via the `parse_packed_flags` helper. A standalone
bool sets every flag; otherwise each `[no-]flag` comma part sets a named flag
(the kebab `ssh-env` / `ssh-terminfo` keywords → the `ssh_env` / `ssh_terminfo`
Rust fields), with `Default` values for the rest, and an unknown flag returns
`FlagsParseError::InvalidValue`. The new test
`packed_flags_parse_cli_shell_notify` covers the standalone bool, the kebab
keywords, omitted-flag defaults, the snake-form `ssh_env` rejected as unknown,
the notify action, and a `format_entry` → `parse_cli` round-trip. Every
packed-struct config type now parses.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3004 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation matches the approved design and upstream
packed-struct semantics — starts from defaults, standalone bool sets all fields,
comma-list flags set only named fields, `no-` negates, unknown names error, and
the SSH fields accept only the upstream kebab names; the tests are adequate
(negative `ssh_env` case, default-preservation, notify action, `format_entry`
round-trip); the gates are clean and the remaining loader pieces are properly
deferred. "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-175752-r518-prompt.md` (result)
- Result: `logs/codex-review/20260604-175752-r518-last-message.md` (result)

## Conclusion

Every packed-struct config type now parses via `parse_packed_flags` (four
structs: `ScrollToBottom`, `FontShapingBreak`, `ShellIntegrationFeatures`,
`NotifyOnCommandFinishAction`). The remaining loader work is: the bool / int /
string "magic" parse paths (the `parseIntoField` type-magic for raw `bool` / int
/ string fields; float stays blocked); the empty-string reset-to-default rule;
and the per-field `parseIntoField` dispatch (`Config::set(key, value)`) + the
`loadCli` / file loader — the inverse of `Config::format_config`.
