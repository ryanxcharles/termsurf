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

# Experiment 514: more enum-keyword config parsers (from_keyword: the mac / background-image / shader-mouse enums)

## Description

Continuing the config loader (Experiment 513), this experiment adds
`from_keyword(value) -> Option<Self>` — the `std.meta.stringToEnum` parse — to
the next batch of plain enums: the macOS window enums, the background-image
enums, and the custom-shader / mouse-shift enums. These are the parse-side
inverse of the `keyword()` introduced in the formatter experiments (502 / 504 /
508).

## Upstream behavior

`parseIntoField` (`cli/args.zig:302`) parses an enum field with no custom
`parseCLI` via `std.meta.stringToEnum(Field, value)` — the variant whose tag
name equals `value`, else an error. The eight enums in this batch have no custom
upstream `parseCLI` (verified), so they parse purely by tag name. Their tags (=
their `keyword()` values, validated in Experiments 502 / 504 / 508):

- `MacTitlebarStyle` (`macos-titlebar-style`): `native`, `transparent`, `tabs`,
  `hidden`.
- `MacTitlebarProxyIcon` (`macos-titlebar-proxy-icon`): `visible`, `hidden`.
- `MacWindowButtons` (`macos-window-buttons`): `visible`, `hidden`.
- `MacHidden` (`macos-hidden`): `never`, `always`.
- `BackgroundImageFit` (`background-image-fit`): `contain`, `cover`, `stretch`,
  `none`.
- `BackgroundImagePosition` (`background-image-position`): `top-left`,
  `top-center`, `top-right`, `center-left`, `center-center`, `center-right`,
  `bottom-left`, `bottom-center`, `bottom-right`, `center`.
- `CustomShaderAnimation` (`custom-shader-animation`): `false`, `true`,
  `always`.
- `MouseShiftCapture` (`mouse-shift-capture`): `false`, `true`, `always`,
  `never`.

`stringToEnum` matches the exact tag — the bool-like `false` / `true` tags of
`CustomShaderAnimation` / `MouseShiftCapture` are matched only as the literal
strings (the enum-tag path, not the `bool` `parseBool` path).

## Rust mapping (`roastty/src/config/mod.rs`)

Each enum gets `from_keyword(value: &str) -> Option<Self>`, the inverse of its
`keyword()` — an exact `match` on the tag string, else `None` (mirroring
`std.meta.stringToEnum`'s `?Field`). For example:

```rust
impl MacHidden {
    pub(crate) fn from_keyword(value: &str) -> Option<Self> {
        match value {
            "never" => Some(MacHidden::Never),
            "always" => Some(MacHidden::Always),
            _ => None,
        }
    }
}
// … the same shape for the other seven enums (each arm = a keyword() value).
```

## Scope / faithfulness notes

- **Ported (bridged)**: the `stringToEnum` enum parse, as `from_keyword`, for
  the eight enums.
- **Faithful**: each maps the exact upstream tag name to its variant and returns
  `None` otherwise — exactly `std.meta.stringToEnum`. The bool-like `false` /
  `true` of `CustomShaderAnimation` / `MouseShiftCapture` match only as literal
  tag strings.
- **Faithful adaptation**: `std.meta.stringToEnum(Field, value)` → an explicit
  `match value { … }` returning `Option<Self>`.
- **Deferred**: `from_keyword` for the remaining enums (osc / confirm / link /
  subtitle / padding-color / fullscreen / shell / notify); the enums with custom
  upstream `parseCLI`; the empty-string reset rule; the bool / int / float /
  string magic paths; the per-field `parseIntoField` dispatch and the `loadCli`
  / file loader.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `from_keyword` to the eight enums (each in
   its existing `impl`).
2. Tests (in `config/mod.rs`): for each enum, every tag round-trips
   (`from_keyword(v.keyword()) == Some(v)`) and an unknown string is `None`.
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
findings**. It re-checked the upstream enum tags and the generic parse path:
these are plain enum fields, so `parseIntoField` uses exact
`std.meta.stringToEnum`, not `parseBool` (the bool parser only applies when the
field type is actually `bool`, `args.zig:416`/`:442`). The tags match upstream —
`CustomShaderAnimation` (`Config.zig:5244`), the macOS titlebar/window enums
(`:8988`/`:8994`/`:9002`/ `:9008`), `MouseShiftCapture` (`:9100`), and the
background-image position/fit tags (`:9611`/`:9625`). The round-trip tests plus
unknown rejection are adequate, and for `CustomShaderAnimation` /
`MouseShiftCapture` the `false` / `true` tags match only as literal strings (`1`
/ `t` / `0` / `f` do not).

Review artifacts:

- Prompt: `logs/codex-review/20260604-170444-d514-prompt.md` (design)
- Result: `logs/codex-review/20260604-170444-d514-last-message.md` (design)

## Result

**Result:** Pass

`from_keyword(value: &str) -> Option<Self>` was added to the eight enums
(`MacTitlebarStyle`, `MacTitlebarProxyIcon`, `MacWindowButtons`, `MacHidden`,
`BackgroundImageFit`, `BackgroundImagePosition`, `CustomShaderAnimation`,
`MouseShiftCapture`), each an exact tag match (the inverse of `keyword()`)
returning `None` otherwise — the `std.meta.stringToEnum` parse. The new test
`enum_from_keyword_round_trips_mac_bgimage_shader` round-trips every variant,
rejects an unknown string, and asserts the bool-like tags reject `"1"` / `"t"`.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3000 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation matches the approved design — exact
`stringToEnum`-style tag matching, inverse of the existing `keyword()` mappings,
with bool-like enum tags treated as literal strings only; the `"1"` / `"t"`
negative checks cover the parseBool non-alias behavior; the test and gates are
adequate (every variant round-trips, unknowns reject, build/tests clean, no
deferred-scope leaks). "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-170737-r514-prompt.md` (result)
- Result: `logs/codex-review/20260604-170737-r514-last-message.md` (result)

## Conclusion

Fifteen plain enums now have `from_keyword` (Experiments 513–514). The remaining
plain enums (osc / confirm / link / subtitle / padding-color / fullscreen /
shell / notify groups) come next, then the bool / int / float / string "magic"
parse paths, the empty-string reset-to-default rule, and the per-field
`parseIntoField` dispatch (`Config::set(key, value)`) + the `loadCli` / file
loader.
