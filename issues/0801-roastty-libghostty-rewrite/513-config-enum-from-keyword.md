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

# Experiment 513: the enum-keyword config parsers (from_keyword: CopyOnSelect / ClipboardAccess / RightClickAction / MiddleClickAction / WindowColorspace / AlphaBlending / GraphemeWidthMethod)

## Description

The config **formatter** layer is complete (Experiments 491–512). This
experiment begins the config **loader** (the inverse direction) with the
parse-side primitive most enum fields need:
`from_keyword(value) -> Option<Self>`, the inverse of the `keyword()` introduced
in the formatter experiments.

Upstream's per-field parse dispatch (`cli/args.zig` `parseIntoField`) parses an
enum field that has **no custom `parseCLI`** via
`std.meta.stringToEnum(Field, value)` — i.e. the value string is matched against
the enum's tag names. This experiment ports that `stringToEnum` parse for a
batch of seven such plain enums as `from_keyword`.

## Upstream behavior

`parseIntoField` (`cli/args.zig:302`), for an enum field with no `parseCLI`,
falls through to the type-magic path, which for an `enum` does
`std.meta.stringToEnum(Field, value)` — returning the variant whose **tag name**
equals `value`, or an error (`InvalidValue`) when none matches.

The seven enums in this batch have no custom upstream `parseCLI` (verified), so
they parse purely by tag name. Their tags (= their `keyword()` values, validated
in the formatter experiments):

- `CopyOnSelect` (`copy-on-select`): `false`, `true`, `clipboard`.
- `ClipboardAccess` (`clipboard-read` / `clipboard-write`): `allow`, `deny`,
  `ask`.
- `RightClickAction` (`right-click-action`): `ignore`, `paste`, `copy`,
  `copy-or-paste`, `context-menu`.
- `MiddleClickAction` (`middle-click-action`): `primary-paste`, `ignore`.
- `WindowColorspace` (`window-colorspace`): `srgb`, `display-p3`.
- `AlphaBlending` (`alpha-blending`): `native`, `linear`, `linear-corrected`.
- `GraphemeWidthMethod` (`grapheme-width-method`): `legacy`, `unicode`.

`stringToEnum` matches the exact tag — the bool-like `false` / `true` tags of
`CopyOnSelect` are matched only as the literal strings `"false"` / `"true"`
(this is the enum-tag path, not the `bool` `parseBool` path that also accepts
`1`/`t`/`0`/`f`).

## Rust mapping (`roastty/src/config/mod.rs`)

Each enum gets `from_keyword(value: &str) -> Option<Self>`, the inverse of its
`keyword()` — an exact match on the tag string, else `None`:

```rust
impl CopyOnSelect {
    pub(crate) fn from_keyword(value: &str) -> Option<Self> {
        match value {
            "false" => Some(CopyOnSelect::False),
            "true" => Some(CopyOnSelect::True),
            "clipboard" => Some(CopyOnSelect::Clipboard),
            _ => None,
        }
    }
}
// … the same shape for the other six enums (each arm = a keyword() value).
```

`from_keyword` returns `Some(variant)` for an exact tag match and `None`
otherwise, mirroring `std.meta.stringToEnum`'s `?Field`. The dispatch layer (a
later experiment) maps `None` to upstream's `error.InvalidValue`.

## Scope / faithfulness notes

- **Ported (bridged)**: the `stringToEnum` enum parse, as `from_keyword`, for
  the seven plain enums.
- **Faithful**: each maps the exact upstream tag name to its variant and returns
  `None` for anything else — exactly `std.meta.stringToEnum`. The bool-like
  `false` / `true` of `CopyOnSelect` match only as literal tag strings.
- **Faithful adaptation**: `std.meta.stringToEnum(Field, value)` → an explicit
  `match value { … }` returning `Option<Self>` (the `?Field` result).
- **Deferred**: `from_keyword` for the remaining enums; the enums with custom
  upstream `parseCLI` (e.g. `WindowDecoration`, already ported); the
  empty-string reset-to-default rule; the bool / int / float / string magic
  paths; the per-field `parseIntoField` dispatch and the `loadCli` / file
  loader.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `from_keyword` to the seven enums (each in
   its existing `impl`).
2. Tests (in `config/mod.rs`): for each enum, every tag round-trips
   (`from_keyword(v.keyword()) == Some(v)`) and an unknown string is `None`
   (e.g. `CopyOnSelect::from_keyword("nope") == None`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty from_keyword
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- each enum's `from_keyword` returns the variant for the exact tag and `None`
  otherwise — faithful to `std.meta.stringToEnum`;
- the tests pass (round-trip every tag + an unknown → `None`), and the existing
  tests still pass;
- the remaining loader pieces stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a tag mapping diverges from upstream, an unrelated
item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed these seven enum types have no custom `parseCLI`, so
upstream reaches the generic enum branch in `parseIntoField` —
`std.meta.stringToEnum(Field, value)` with exact tag-name matching
(`args.zig:341`/`:442`); the tag sets match the upstream definitions exactly,
including the kebab-case tags. For `CopyOnSelect`, `false` / `true` are enum
tags (not bool parsing) — the bool arm only applies when `Field` is actually
`bool`, so `1` / `t` / `0` / `f` are not accepted by
`CopyOnSelect::from_keyword`; round-tripping every tag plus an unknown input is
adequate.

Review artifacts:

- Prompt: `logs/codex-review/20260604-170002-d513-prompt.md` (design)
- Result: `logs/codex-review/20260604-170002-d513-last-message.md` (design)

## Result

**Result:** Pass

`from_keyword(value: &str) -> Option<Self>` was added to the seven plain enums
(`CopyOnSelect`, `ClipboardAccess`, `RightClickAction`, `MiddleClickAction`,
`WindowColorspace`, `AlphaBlending`, `GraphemeWidthMethod`), each an exact tag
match (the inverse of `keyword()`) returning `None` otherwise — the
`std.meta.stringToEnum` parse. The new test
`enum_from_keyword_round_trips_and_rejects_unknown` round-trips every variant,
rejects an unknown string, and asserts `CopyOnSelect::from_keyword` rejects
`"1"` / `"t"` (the bool-like tags match only as literal strings).

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 2999 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation matches upstream's generic enum parse path —
exact tag-name matching through `stringToEnum`, with no bool-alias parsing for
enum fields (`args.zig:341`/`:442`); the round-trip tests cover every variant,
unknown rejection, and the `CopyOnSelect` distinction that `"1"` / `"t"` are not
accepted; gates are clean. "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-170251-r513-prompt.md` (result)
- Result: `logs/codex-review/20260604-170251-r513-last-message.md` (result)

## Conclusion

The config **loader** now has its first parse-side primitive: the `stringToEnum`
enum parse (`from_keyword`) for seven plain enums. The next slices can add
`from_keyword` for the remaining plain enums (the mac / fullscreen /
background-image / font / shader groups), then the bool / int / float / string
"magic" parse paths, the empty-string reset-to-default rule, and finally the
per-field `parseIntoField` dispatch (`Config::set(key, value)`) and the
`loadCli` / file loader — the inverse of `Config::format_config`.
