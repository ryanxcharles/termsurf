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

# Experiment 529: the multi-line config loader (Config::load_str)

## Description

With the per-line parser (`parse_config_line`, Experiment 528) and `Config::set`
(43 of 44 fields), this experiment ports the multi-line **driver** —
`Config::load_str` — the heart of upstream's config-file `parse`. It iterates a
config source's lines, applies each `key = value` via `Config::set`, and
**collects per-line diagnostics** instead of aborting on a bad line (upstream's
`parse` records a diagnostic and continues). File IO (reading a path into this
driver) is the next experiment.

## Upstream behavior

Upstream `parse` (`cli/args.zig:55`) drives an iterator (here `LineIterator`),
calling `parseIntoField` per entry. On a field error it appends a **diagnostic**
(a location + message) and continues — it does **not** stop loading:

- the line counter is 1-indexed and increments for **every** line read,
  including blank and comment lines (`LineIterator` does `self.line += 1` before
  the blank/comment `continue`), so a diagnostic's line number counts all
  preceding lines.
- a blank line or `#` comment contributes no field (skipped by
  `parse_config_line`).
- a `key = value` line sets the field; a parse error becomes a diagnostic for
  that line, and the loader moves on to the next line.

So loading a config source is: for each line (1-indexed), `parse_config_line`;
if it yields `(key, value)`, `Config::set(key, value)`, recording a diagnostic
on error.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// A per-line config-load diagnostic (upstream's `parse` diagnostics): the
/// 1-indexed line, the offending key, and the `Config::set` error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigDiagnostic {
    pub line: usize,
    pub key: String,
    pub error: ConfigSetError,
}

impl Config {
    /// Load config from a source string (upstream's config-file `parse` driving
    /// `LineIterator`): apply each `key = value` line via `Config::set`, skipping
    /// blank lines and `#` comments, and collect a diagnostic per failing line
    /// (continuing rather than aborting). Lines are 1-indexed, counting blanks and
    /// comments.
    pub(crate) fn load_str(&mut self, text: &str) -> Vec<ConfigDiagnostic> {
        let mut diagnostics = Vec::new();
        for (i, line) in text.split('\n').enumerate() {
            let Some((key, value)) = parse_config_line(line) else {
                continue;
            };
            if let Err(error) = self.set(key, value) {
                diagnostics.push(ConfigDiagnostic {
                    line: i + 1,
                    key: key.to_string(),
                    error,
                });
            }
        }
        diagnostics
    }
}
```

`text.split('\n')` yields each line (a trailing `\r` of a CRLF line is trimmed
by `parse_config_line`); `enumerate()` + 1 gives the 1-indexed line number
counting all lines. A blank/comment line (`parse_config_line` ⇒ `None`) is
skipped. A failing `Config::set` records a `ConfigDiagnostic` and the loop
continues.

## Scope / faithfulness notes

- **Ported (bridged)**: the multi-line config-load driver, as
  `Config::load_str` + `ConfigDiagnostic`.
- **Faithful**: per-line iteration with 1-indexed line numbers counting
  blank/comment lines; blank/comment skip; `Config::set` per `key = value`;
  **continue past errors**, collecting a diagnostic per failing line — exactly
  upstream's `parse` (record + continue).
- **Faithful adaptation**: upstream's `Iterator` + `LineIterator` reader →
  iterating `text.split('\n')` (the IO/buffering is the file-IO experiment);
  upstream's diagnostic `Location` + message → a
  `ConfigDiagnostic { line, key, error }` (the precise message text is not
  reproduced — the line, key, and error kind are).
- **Deferred**: file IO (reading a config path into `load_str`); the
  `--key=value` CLI-arg form; the precise diagnostic message strings.
  `background-image-opacity` stays float-blocked (a
  `background-image-opacity = …` line yields an `UnknownField` diagnostic until
  a float formatter lands).
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `ConfigDiagnostic` and `Config::load_str`,
   using `loader::parse_config_line`.
2. Tests (in `config/mod.rs`): a multi-line config (several keys, blank lines, a
   `#` comment, a quoted value) loads all fields (verified via `format_config`)
   with no diagnostics; a bad line (an unknown key and an invalid value) records
   diagnostics with the correct 1-indexed line numbers (counting the
   blank/comment lines) while the other lines still apply (continue past
   errors); a `None`-value bare key line.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty config_load_str
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Config::load_str` applies each `key = value` line via `Config::set`, skips
  blank/comment lines, and collects a diagnostic per failing line (1-indexed,
  counting all lines) while continuing — faithful to upstream's `parse`;
- the tests pass (a clean multi-line load + a load with errors and correct line
  numbers), and the existing tests still pass;
- file IO and the precise diagnostic messages stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the driver diverges from upstream (esp. aborting on
an error instead of continuing, or mis-counting line numbers), an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the design matches upstream's file-driver behavior for
this slice: line numbers are 1-indexed and count blank/comment lines because
`LineIterator` increments `self.line` before skipping blanks/comments
(`args.zig:1449`); parse errors are diagnostic-recorded and the loop continues —
it does not stop on the first field error when diagnostics are available
(`args.zig:136`/`:173`); `split('\n')` is a reasonable Rust stand-in (CRLF
leaves `\r` for `parse_config_line` to trim, a trailing empty segment is skipped
as blank); capturing `{ line, key, error }` instead of upstream's formatted
diagnostic text is an acceptable narrowing (it preserves the actionable
location/key/error kind); and `background-image-opacity` producing
`UnknownField` is the correct documented in-progress behavior while the float
field remains blocked.

Review artifacts:

- Prompt: `logs/codex-review/20260604-190040-d529-prompt.md` (design)
- Result: `logs/codex-review/20260604-190040-d529-last-message.md` (design)
