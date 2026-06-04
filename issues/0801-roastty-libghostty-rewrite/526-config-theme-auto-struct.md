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

# Experiment 526: parseAutoStruct for Theme (Theme::parse_auto_struct)

## Description

With both building blocks in place — `CommaSplitter` (Experiment 524) and
`parse_quoted_string` (Experiment 525) — this experiment ports upstream
`cli.args.parseAutoStruct` for `Theme`, its only consumer in roastty's scope. It
parses a comma-list of `key:value` pairs into `Theme`'s two required string
fields (`light`, `dark`). This is the light/dark-pair branch that
`Theme::parse_cli` (the next experiment) calls.

## Upstream behavior

`parseAutoStruct(T, alloc, v, default_)` (`cli/args.zig:525`), for `Theme`
(fields `light` / `dark`, both `[]const u8`, no defaults, `default_ = null`):

```zig
var result: T = undefined;
var fields_set = …;                       // a bitset of which fields were set
var iter = CommaSplitter.init(v);
loop: while (try iter.next()) |entry| {
    const idx = mem.indexOf(u8, entry, ":") orelse return error.InvalidValue;
    const key = mem.trim(u8, entry[0..idx], whitespace);     // whitespace = " \t"
    const value = value: {
        const value = mem.trim(u8, entry[idx + 1 ..], whitespace);
        if (value.len >= 2 and value[0] == '"' and value[value.len - 1] == '"') {
            const parsed = try string_literal.parseWrite(&buf.writer, value);  // quoted decode
            if (parsed == .failure) return error.InvalidValue;
            break :value buf.written();
        }
        break :value value;
    };
    inline for (info.fields, …) |field, i| {
        if (mem.eql(u8, field.name, key)) { try parseIntoField(T, …, key, value); fields_set.set(i); continue :loop; }
    }
    return error.InvalidValue;             // no field matched the key
}
inline for (info.fields, …) |field, i| {  // required-field check
    if (!fields_set.isSet(i)) {
        // default_ ⇒ that default; else struct default; else error.InvalidValue
    }
}
return result;
```

So: split by comma (`CommaSplitter`); each part must have a `:` (else
`InvalidValue`); the key and value are whitespace-trimmed (`" \t"`); a `"…"`
value is decoded via `parseWrite` (a failure ⇒ `InvalidValue`), else used
verbatim; the key must match a field (`light` / `dark`), else `InvalidValue`;
setting a field again overwrites; and every field with no default must be set,
else `InvalidValue`. `Theme`'s `light` / `dark` have no defaults, so **both are
required**. A `CommaSplitter` error (`UnclosedQuote` / …) propagates.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// An error parsing a `Theme` (upstream `parseAutoStruct` / `Theme.parseCLI`
/// `error.InvalidValue`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeParseError {
    Invalid,
}

impl Theme {
    /// Parse a `light:…,dark:…` pair (upstream `cli.args.parseAutoStruct` for
    /// `Theme`): a comma-list of `key:value` pairs into the required `light` /
    /// `dark` fields. A missing `:`, an unknown key, a missing required field, a
    /// quoted-value decode failure, or a `CommaSplitter` error is `Invalid`.
    pub(crate) fn parse_auto_struct(input: &str) -> Result<Theme, ThemeParseError> {
        let ws = |c: char| c == ' ' || c == '\t';
        let mut light: Option<String> = None;
        let mut dark: Option<String> = None;

        let mut splitter = CommaSplitter::new(input);
        while let Some(entry) = splitter.next().map_err(|_| ThemeParseError::Invalid)? {
            let idx = entry.find(':').ok_or(ThemeParseError::Invalid)?;
            let key = entry[..idx].trim_matches(ws);
            let raw = entry[idx + 1..].trim_matches(ws);
            let value = if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
                let bytes = parse_quoted_string(raw.as_bytes()).ok_or(ThemeParseError::Invalid)?;
                String::from_utf8(bytes).map_err(|_| ThemeParseError::Invalid)?
            } else {
                raw.to_string()
            };
            match key {
                "light" => light = Some(value),
                "dark" => dark = Some(value),
                _ => return Err(ThemeParseError::Invalid),
            }
        }

        Ok(Theme {
            light: light.ok_or(ThemeParseError::Invalid)?,
            dark: dark.ok_or(ThemeParseError::Invalid)?,
        })
    }
}
```

`CommaSplitter` does the quote-aware splitting; `parse_quoted_string` decodes a
quoted value. Setting `light` / `dark` again overwrites (upstream's later-wins +
`fields_set`); the `Option`s double as the required-field check.

## Scope / faithfulness notes

- **Ported (bridged)**: `parseAutoStruct` specialized to `Theme`, as
  `Theme::parse_auto_struct`.
- **Faithful**: comma splitting (via `CommaSplitter`), the `:` requirement, the
  `" \t"` trim of key and value, the `"…"` quoted-value decode (via
  `parse_quoted_string`), the `light` / `dark` field match (unknown key ⇒
  `Invalid`), later-wins overwrite, and both fields required (`Invalid` if
  missing) — exactly upstream's `parseAutoStruct` for `Theme`.
- **Documented narrowings**:
  - Upstream surfaces the specific `CommaSplitter` error kinds (`UnclosedQuote`,
    …); roastty collapses them — and every malformed-pair case — to a single
    `Invalid` (the same accept/reject outcome; the precise Zig error name is not
    surfaced).
  - `Theme`'s fields are Rust `String` (upstream `[]const u8` bytes), so a
    decoded value that is not valid UTF-8 (only reachable via a `\xNN` escape in
    a quoted value) is `Invalid` rather than stored as raw bytes. Unreachable
    for real theme names.
- **Deferred**: `Theme::parse_cli` (the comma/`=`/`:` detection + the
  single-name branch, next experiment) and the `theme` `Config::set` arm; the
  `loadCli` / file loader. `background-image-opacity` stays float-blocked.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `ThemeParseError` and
   `Theme::parse_auto_struct`, using `config::comma_splitter::CommaSplitter` and
   `config::string::parse_quoted_string` (add the `use`s).
2. Tests (in `config/mod.rs`): `light:day,dark:night` ⇒ `Theme { day, night }`;
   whitespace trimmed (`light : day , dark : night`); a quoted value with a
   comma (`light:"a,b",dark:c`) ⇒ `light = "a,b"`; later-wins
   (`light:a,light:b,dark:c`) ⇒ `light = b`; missing `:` ⇒ `Invalid`; unknown
   key (`bright:x`) ⇒ `Invalid`; a missing field (`light:day`) ⇒ `Invalid`;
   round-trip with `format_entry` for a pair.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty theme_parse_auto_struct
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Theme::parse_auto_struct` reproduces upstream `parseAutoStruct` for `Theme`:
  comma split, `:` requirement, `" \t"` trim, quoted decode, `light` / `dark`
  match, later-wins, both required;
- the tests pass (the pair / whitespace / quoted / later-wins cases + the
  `Invalid` cases), and the existing tests still pass;
- `Theme::parse_cli` and the loader stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the parse diverges from upstream, an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the Theme-specialized `parseAutoStruct` matches
upstream — `CommaSplitter` drives entries, `:` is required, key/value are
trimmed with `" \t"`, quoted values are decoded through `parseWrite`, unknown
keys are invalid, repeated keys overwrite, and the final required-field check
makes both `light` and `dark` mandatory (`args.zig:525`/`:547`/`:586`);
`Theme.parseCLI` delegates the pair form to `parseAutoStruct(Theme, …, null)`
(`Config.zig:9852`/`:9871`). The two narrowings are acceptable for this Rust
slice (collapsing splitter/malformed errors to `Invalid` preserves accept/reject
behavior; rejecting non-UTF-8 decoded values is reasonable since `Theme` stores
`String`). The quoted detection matches upstream's
`len >= 2 && first == '"' && last == '"'` (`args.zig:560`), and `light:`
becoming an empty string is faithful (the empty-reset branch falls through when
the string field has no default, then the string type-magic copies the empty
value).

Review artifacts:

- Prompt: `logs/codex-review/20260604-184646-d526-prompt.md` (design)
- Result: `logs/codex-review/20260604-184646-d526-last-message.md` (design)

## Result

**Result:** Pass

`ThemeParseError` and `Theme::parse_auto_struct` were added — a port of
`cli.args.parseAutoStruct` for `Theme`, driving `CommaSplitter` +
`parse_quoted_string`: each comma-split entry needs a `:`; the key/value are
`" \t"`-trimmed; a `"…"` value is decoded; the key matches `light` / `dark`
(later-wins, unknown ⇒ `Invalid`); both fields are required; and malformed
splitter/pair cases collapse to `Invalid`. The new test
`theme_parse_auto_struct` covers the pair, whitespace, quoted-comma, later-wins,
empty-value, and the missing-colon / unknown-key / missing-field failures, plus
a `parse → format_entry` round-trip.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3014 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation matches the approved Theme-specific
`parseAutoStruct` — comma-aware splitting, required colon, `" \t"` trimming,
quoted-value decoding, `light` / `dark` matching with later-wins overwrite, both
fields required, and malformed splitter/pair cases collapsed to `Invalid`; the
tests cover the main behavior and edge cases (quoted comma, whitespace,
duplicate-key overwrite, empty value, missing colon, unknown key, missing field,
format round-trip); gates are clean and the remaining Theme/loader work stays
deferred. "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-184932-r526-prompt.md` (result)
- Result: `logs/codex-review/20260604-184932-r526-last-message.md` (result)

## Conclusion

`Theme::parse_auto_struct` ports the light/dark-pair branch. The next experiment
wraps it as **`Theme::parse_cli`** — upstream `Theme.parseCLI`'s `None`/empty ⇒
`ValueRequired`, the comma/`=`/`:` detection routing to `parse_auto_struct`, and
the single-name branch (`light = dark = trimmed`) — and adds the `theme`
`Config::set` arm (the last parseable field, taking `Config::set` to 43 of 44).
Then the `loadCli` / config-file loader drives `Config::set` over `key = value`
lines.
