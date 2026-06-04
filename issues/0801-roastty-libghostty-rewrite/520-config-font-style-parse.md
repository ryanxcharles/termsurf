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

# Experiment 520: the FontStyle config parser (FontStyle::parse_cli)

## Description

Continuing the config loader, this experiment ports `parse_cli` for `FontStyle`
— the `font-style*` config value. `FontStyle` is the union type whose
`format_entry` was ported in Experiment 506; this is its parse side. It is a
prerequisite for the aggregate `Config::set` dispatch (four `font-style*`
fields), mirroring how `Theme::format_entry` was a prerequisite for
`Config::format_config`.

(The sibling `Theme::parse_cli` is more involved — its light/dark-pair branch
needs upstream `parseAutoStruct` — and stays deferred to its own experiment.)

## Upstream behavior

Upstream `FontStyle.parseCLI` (`Config.zig:8444`):

```zig
pub fn parseCLI(self: *Self, alloc: Allocator, input: ?[]const u8) !void {
    const value = input orelse return error.ValueRequired;
    if (std.mem.eql(u8, value, "default")) { self.* = .{ .default = {} }; return; }
    if (std.mem.eql(u8, value, "false")) { self.* = .{ .false = {} }; return; }
    const nameZ = try alloc.dupeZ(u8, value);
    self.* = .{ .name = nameZ };
}
```

So `FontStyle`:

- a missing value → `error.ValueRequired`;
- `"default"` → the `default` variant;
- `"false"` → the `false` variant;
- any other value → the `name` variant holding an owned copy of the value (this
  includes the empty string `""` — the set-but-empty reset is a _separate_
  earlier branch of `parseIntoField`, not part of `parseCLI`).

The only error is `ValueRequired` (the `dupeZ` `OutOfMemory` has no Rust
analog).

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// An error parsing a `FontStyle` (upstream `error.ValueRequired`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FontStyleParseError {
    ValueRequired,
}

impl FontStyle {
    /// Parse the `font-style*` value (upstream `FontStyle.parseCLI`): a missing
    /// value is `ValueRequired`; `default` / `false` select those variants; any
    /// other value is a named style (an owned copy).
    pub(crate) fn parse_cli(value: Option<&str>) -> Result<FontStyle, FontStyleParseError> {
        let value = value.ok_or(FontStyleParseError::ValueRequired)?;
        Ok(match value {
            "default" => FontStyle::Default,
            "false" => FontStyle::False,
            other => FontStyle::Name(other.to_string()),
        })
    }
}
```

`None` maps to `ValueRequired` (upstream's `input orelse return error`); the
literal `"default"` / `"false"` select the void variants; any other string —
including `""` — becomes `Name(copy)`, matching upstream's `dupeZ`.

## Scope / faithfulness notes

- **Ported (bridged)**: `FontStyle::parse_cli` (upstream `FontStyle.parseCLI`).
- **Faithful**: missing value → `ValueRequired`; `default` / `false` → the void
  variants; any other value → `Name` of an owned copy (incl. `""`). Round-trips
  with `format_entry` for the `default` / `false` / named cases.
- **Faithful adaptation**: `input orelse return error.ValueRequired` →
  `value.ok_or(…)?`; `dupeZ(value)` → `other.to_string()`; `error.ValueRequired`
  → `FontStyleParseError::ValueRequired`.
- **Deferred**: `Theme::parse_cli` (needs `parseAutoStruct`); the int type-magic
  path (float blocked); the set-but-empty reset rule; the per-field
  `parseIntoField` dispatch (`Config::set`) and the `loadCli` / file loader.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `FontStyleParseError` and
   `FontStyle::parse_cli`.
2. Tests (in `config/mod.rs`): `parse_cli(None) == Err(ValueRequired)`;
   `Some("default") == Ok(Default)`; `Some("false") == Ok(False)`;
   `Some("bold") == Ok(Name("bold"))`; `Some("") == Ok(Name(""))`; a `parse_cli`
   → `format_entry` round-trip for the three formatted cases.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty font_style_parse
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `FontStyle::parse_cli` matches upstream `parseCLI`: `None` → `ValueRequired`,
  `default` / `false` → the void variants, any other value → `Name(copy)`;
- the tests pass (missing, default, false, named, empty, round-trip), and the
  existing tests still pass;
- `Theme::parse_cli` and the remaining loader pieces stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the parse diverges from upstream, an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the parser matches upstream exactly — `None` →
`ValueRequired`, `"default"` → the default arm, `"false"` → the false arm, and
every other byte string copied into the `name` arm (`Config.zig:8444`); upstream
does not special-case empty input inside `FontStyle.parseCLI`, so
`Some("") -> Name("")` is faithful for the method in isolation (the empty-string
reset belongs to the earlier outer dispatch branch). A single
`FontStyleParseError::ValueRequired` is fine — the only other upstream failure
is the `dupeZ` allocator failure, consistently omitted in the Rust-owned
`String` adaptations.

Review artifacts:

- Prompt: `logs/codex-review/20260604-180731-d520-prompt.md` (design)
- Result: `logs/codex-review/20260604-180731-d520-last-message.md` (design)

## Result

**Result:** Pass

`FontStyleParseError` and `FontStyle::parse_cli` were added: `None` →
`ValueRequired`, `"default"` / `"false"` select the void variants, and any other
value (including `""`) becomes `Name(copy)` — faithful to upstream
`FontStyle.parseCLI`. The new test `font_style_parse_cli` covers the missing
value, the two literals, a named style, the empty-string case, and a `parse_cli`
→ `format_entry` round-trip for the three formatted cases.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3006 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation matches upstream `FontStyle.parseCLI` — `None` is
`ValueRequired`, `"default"` / `"false"` select the void variants, and
everything else (incl. `""`) becomes a copied `Name`; the test coverage is
adequate (explicit empty-string case, parse/format round-trip); the gates are
clean and the deferred dispatch/reset work stayed out of scope. "Approved with
no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-180941-r520-prompt.md` (result)
- Result: `logs/codex-review/20260604-180941-r520-last-message.md` (result)

## Conclusion

`FontStyle::parse_cli` is ported — four of the aggregate `Config::set` fields'
parsers (`font-style*`) are now ready. The remaining leaf parser for the
aggregate is `Theme::parse_cli` (its light/dark-pair branch needs upstream
`parseAutoStruct` — its own experiment). After that, the aggregate `Config::set`
dispatch (with the set-but-empty reset rule) can wire `from_keyword`, the
packed-struct / leaf `parse_cli`, and the bool magic — the inverse of
`Config::format_config`. The int type-magic (float blocked) and the `loadCli` /
file loader follow.
