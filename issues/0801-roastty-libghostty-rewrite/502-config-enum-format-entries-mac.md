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

# Experiment 502: the macOS enum-keyword config formatters (MacTitlebarStyle / MacTitlebarProxyIcon / MacWindowButtons / MacHidden)

## Description

Continuing the enum-keyword formatter pattern (Experiments 500–501), this
experiment ports `keyword()` + `format_entry` for the four macOS window config
enums: `MacTitlebarStyle`, `MacTitlebarProxyIcon`, `MacWindowButtons`, and
`MacHidden`. Each writes its variant's upstream tag name (the config keyword) as
a `name = keyword\n` entry — the generic enum `{t}` format. Grounded by the
`EntryFormatter` from Experiment 491.

## Upstream behavior

The generic `formatEntry` enum branch (`config/formatter.zig`) writes
`name = {tag-name}\n`. The four enums (upstream `enum`s) and their tag names
(verified against `config/Config.zig`):

- `MacTitlebarStyle` (`macos-titlebar-style`): `native`, `transparent`, `tabs`,
  `hidden`.
- `MacTitlebarProxyIcon` (`macos-titlebar-proxy-icon`): `visible`, `hidden`.
- `MacWindowButtons` (`macos-window-buttons`): `visible`, `hidden`.
- `MacHidden` (`macos-hidden`): `never`, `always`.

## Rust mapping (`roastty/src/config/mod.rs`)

Each enum gets a `keyword(self) -> &'static str` (the exact upstream tag) and a
`format_entry`:

```rust
impl MacTitlebarStyle {
    pub(crate) fn keyword(self) -> &'static str {
        match self {
            MacTitlebarStyle::Native => "native",
            MacTitlebarStyle::Transparent => "transparent",
            MacTitlebarStyle::Tabs => "tabs",
            MacTitlebarStyle::Hidden => "hidden",
        }
    }
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        formatter.entry_str(self.keyword());
    }
}

impl MacTitlebarProxyIcon {
    pub(crate) fn keyword(self) -> &'static str {
        match self {
            MacTitlebarProxyIcon::Visible => "visible",
            MacTitlebarProxyIcon::Hidden => "hidden",
        }
    }
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        formatter.entry_str(self.keyword());
    }
}

impl MacWindowButtons {
    pub(crate) fn keyword(self) -> &'static str {
        match self {
            MacWindowButtons::Visible => "visible",
            MacWindowButtons::Hidden => "hidden",
        }
    }
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        formatter.entry_str(self.keyword());
    }
}

impl MacHidden {
    pub(crate) fn keyword(self) -> &'static str {
        match self {
            MacHidden::Never => "never",
            MacHidden::Always => "always",
        }
    }
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        formatter.entry_str(self.keyword());
    }
}
```

Each `keyword` is the exact upstream tag name (verified), and `format_entry`
writes `name = keyword\n` (the generic `{t}` enum branch). All four enums are
`Copy`, so the methods take `self` by value.

## Scope / faithfulness notes

- **Ported (bridged)**: `keyword` + `format_entry` for `MacTitlebarStyle`,
  `MacTitlebarProxyIcon`, `MacWindowButtons`, and `MacHidden` (upstream's
  generic enum `{t}` format for these four).
- **Faithful**: each variant maps to its exact upstream tag name, written as
  `name = keyword\n` — exactly upstream's enum branch.
- **Faithful adaptation**: the comptime `{t}` (tag name) → an explicit
  `keyword(self)` match; `formatEntry` → `entry_str(self.keyword())`.
- **Deferred**: the remaining config enums' `keyword` / `format_entry` (ported
  in later slices), the other generic field-dispatch cases (float `{d}`,
  optional recurse), `QuickTerminalSize`, and the broader config
  parser/formatter.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `keyword` + `format_entry` for the four
   enums (each in its existing or a new `impl`).
2. Tests (in `config/mod.rs`): each variant of the four enums formats to
   `"a = {keyword}\n"` (e.g. `MacTitlebarStyle::Transparent` →
   `"a = transparent\n"`; `MacTitlebarProxyIcon::Visible` → `"a = visible\n"`;
   `MacWindowButtons::Hidden` → `"a = hidden\n"`; `MacHidden::Always` →
   `"a = always\n"`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty enum_format
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- each enum's `keyword` / `format_entry` writes `name = {exact upstream tag}\n`
  — faithful to upstream's enum branch;
- the tests pass (every variant of the four enums), and the existing tests still
  pass;
- the other config enums' formatters and the remaining generic field-dispatch
  stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a keyword differs from the upstream tag name, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the mappings are exact for all four macOS enum tag sets
— `native` / `transparent` / `tabs` / `hidden`, `visible` / `hidden`, and
`never` / `always` (`Config.zig:8988`/`:8994`/`:9002`/`:9008`); and that
`entry_str(self.keyword())` remains the faithful generic enum formatter shape,
with testing every variant adequate.

Review artifacts:

- Prompt: `logs/codex-review/20260604-160821-d502-prompt.md` (design)
- Result: `logs/codex-review/20260604-160821-d502-last-message.md` (design)

## Result

**Result:** Pass

`keyword` + `format_entry` were added for the four macOS window config enums
(`MacTitlebarStyle`, `MacTitlebarProxyIcon`, `MacWindowButtons`, `MacHidden`),
each `keyword` the exact upstream tag name and `format_entry` writing
`name = keyword\n`. The new test `enum_format_entries_mac` covers every variant.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 2988 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the keyword mappings match the upstream macOS enum tags exactly, and
`format_entry` preserves the generic enum output shape `name = tag\n`
(`Config.zig:8988`/`:8994`/ `:9002`/`:9008`); the test covers every variant;
gates are clean. "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-161119-r502-prompt.md` (result)
- Result: `logs/codex-review/20260604-161119-r502-last-message.md` (result)

## Conclusion

The four macOS window config enums now format their keywords (thirteen config
enums total across Experiments 500–502). The next slices can finish the
remaining config enums (`Fullscreen`, `NonNativeFullscreen`,
`OscColorReportFormat`, the bool-like `true`/`false`-keyword enums, the
background-image enums, `FontStyle`, `FontShapingBreak`), then the remaining
generic field-dispatch cases (float `{d}`, optional recurse), then the full
config loader, continuing toward the full config formatter and loader.
