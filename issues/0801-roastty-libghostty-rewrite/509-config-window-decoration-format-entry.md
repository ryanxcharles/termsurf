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

# Experiment 509: the WindowDecoration enum config formatter (window-decoration)

## Description

Continuing the config formatter port (Experiments 491–508), this experiment
ports `keyword()` + `format_entry` for `WindowDecoration` — the
`window-decoration` config value, a plain `enum` whose `format_entry` was
overlooked in the earlier plain-enum sweep (it parses bool aliases on the input
side, which is why it sat in a different group than Experiments 500–508). It
writes its variant's upstream tag name as a `name = keyword\n` entry — the
generic enum `{t}` format. Grounded by the `EntryFormatter` from Experiment 491.

This experiment also records a finding: the generic **float `{d}`**
field-dispatch case is float-formatting-blocked (see Conclusion), parallel to
`QuickTerminalSize`'s parseFloat block.

## Upstream behavior

Upstream `WindowDecoration` (`Config.zig:9782`) is:

```zig
pub const WindowDecoration = enum(c_int) {
    auto,
    client,
    server,
    none,
    // parseCLI (bool aliases true→auto / false→none) …
};
```

It has **no custom `formatEntry`**, so it formats through the generic enum
branch (`config/formatter.zig`, `@tagName`), which writes `name = {tag-name}\n`:

- `auto` → `name = auto\n`.
- `client` → `name = client\n`.
- `server` → `name = server\n`.
- `none` → `name = none\n`.

(The bool aliases `true` / `false` exist only on the parse side; formatting
always emits one of the four tag names.)

## Rust mapping (`roastty/src/config/mod.rs`)

`WindowDecoration` gets a `keyword(self) -> &'static str` (the exact upstream
tag) and a `format_entry`, added to its existing `impl` (alongside `parse_cli`):

```rust
impl WindowDecoration {
    pub(crate) fn keyword(self) -> &'static str {
        match self {
            WindowDecoration::Auto => "auto",
            WindowDecoration::Client => "client",
            WindowDecoration::Server => "server",
            WindowDecoration::None => "none",
        }
    }
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        formatter.entry_str(self.keyword());
    }
}
```

`keyword` is the exact upstream tag name (verified), and `format_entry` writes
`name = keyword\n` (the generic `{t}` enum branch). The enum is `Copy`, so the
methods take `self` by value.

## Scope / faithfulness notes

- **Ported (bridged)**: `keyword` + `format_entry` for `WindowDecoration`
  (upstream's generic enum `{t}` format).
- **Faithful**: each variant maps to its exact upstream tag name, written as
  `name = keyword\n`, exactly upstream's enum branch. The parse-side bool
  aliases do not affect formatting.
- **Faithful adaptation**: the comptime `{t}` (tag name) → an explicit
  `keyword(self)` match; `formatEntry` → `entry_str(self.keyword())`.
- **Deferred**: the generic float `{d}` field-dispatch case (float-formatting
  blocked — see Conclusion), the optional-recurse case, `QuickTerminalSize`, and
  the broader config parser/formatter.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `keyword` + `format_entry` to
   `WindowDecoration`'s existing `impl`.
2. Tests (in `config/mod.rs`): each variant formats to `"a = {keyword}\n"` (e.g.
   `WindowDecoration::Server` → `"a = server\n"`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty window_decoration_format_entry
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `WindowDecoration::keyword` / `format_entry` writes
  `name = {exact upstream tag}\n` for all four variants — faithful to upstream's
  enum branch;
- the tests pass (every variant), and the existing tests still pass;
- the generic float/optional field-dispatch cases stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a keyword differs from the upstream tag name, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed `WindowDecoration` has no custom `formatEntry` (its
body defines only `getGObjectType`, `parseCLI`, and tests), so the generic enum
branch applies (`Config.zig:9782`); the four tag mappings `auto` / `client` /
`server` / `none` are exact and the parse-side bool aliases do not affect
formatting; and `entry_str(self.keyword())` matches the generic enum branch
output `name = tag\n` (`formatter.zig:52`).

On the float formatter, Codex agreed deferral is correct: upstream config
formatting uses Zig `{d}` for floats (`formatter.zig:47`), and Rust `Display` is
not a faithful general substitute at the identified edges — float formatting
should stay blocked until either a compatible formatter is ported or it is
constrained with exact per-field evidence and tests.

Review artifacts:

- Prompt: `logs/codex-review/20260604-163730-d509-prompt.md` (design)
- Result: `logs/codex-review/20260604-163730-d509-last-message.md` (design)

## Result

**Result:** Pass

`keyword` + `format_entry` were added to `WindowDecoration`'s existing `impl`,
each `keyword` the exact upstream tag name (`auto` / `client` / `server` /
`none`) and `format_entry` writing `name = keyword\n` via the generic enum
branch. The new test `window_decoration_format_entry` covers every variant. With
this, every plain config enum (and `WindowDecoration`) has a `format_entry`.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 2995 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation matches upstream — `WindowDecoration` has no
custom formatter, so it uses the generic enum branch and writes the exact tag
name (`auto` / `client` / `server` / `none`) as `name = tag\n`
(`Config.zig:9782`, `formatter.zig:52`); the test covers every variant; gates
are clean. "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-163857-r509-prompt.md` (result)
- Result: `logs/codex-review/20260604-163857-r509-last-message.md` (result)

## Conclusion

`WindowDecoration` was the last plain config enum without a `format_entry`;
every plain config enum (and the `FontStyle` union, the `FontShapingBreak` /
`ScrollToBottom` packed structs) now formats faithfully. The generic float `{d}`
field-dispatch case is **float-formatting-blocked** — Codex confirmed Rust
`Display` is not a faithful substitute for Zig `{d}` (which emits full
positional decimal with no scientific notation, backed by a ~1700-line Ryū
formatter); it stays deferred alongside `QuickTerminalSize`'s parseFloat block
until a compatible float formatter is ported or it is constrained with exact
per-field evidence and tests. The remaining config-formatter work is the
optional-recurse field-dispatch case, then the full config loader (per-field
parser/formatter dispatch over the aggregate `Config`, `loadCli`, file I/O),
continuing toward the full config formatter and loader.
