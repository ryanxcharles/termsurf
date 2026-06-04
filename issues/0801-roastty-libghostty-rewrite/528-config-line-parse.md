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

# Experiment 528: the config-file line parser (parse_config_line)

## Description

With `Config::set` routing 43 of 44 fields (Experiment 527), this experiment
begins the top-level **config-file loader** by porting its per-line extraction —
the heart of upstream `cli.args.LineIterator`. `parse_config_line` turns one
config-file line into a `(key, Option<value>)` pair (or skips a blank/comment
line), ready to drive `Config::set`. The IO/buffering and the multi-line
`Config::load_str` driver come in a following experiment.

## Upstream behavior

`LineIterator.next` (`cli/args.zig:1393`), per line (ignoring the reader/buffer
machinery):

```zig
// Trim whitespace (incl CR) around the line.
const trim = std.mem.trim(u8, entry, whitespace ++ "\r");   // " \t\r"
// Ignore blank lines and comments.
if (entry.len == 0 or entry[0] == '#') continue;
// …
if (mem.indexOf(u8, entry, "=")) |idx| {
    const key = std.mem.trim(u8, entry[0..idx], whitespace);          // " \t"
    var value = std.mem.trim(u8, entry[idx + 1 ..], whitespace);
    if (value.len >= 2 and value[0] == '"' and value[value.len - 1] == '"') {
        value = value[1 .. value.len - 1];   // strip surrounding quotes (not decode)
    }
    // → "--key=value"
} // else → "--<entry>"  (key with no value)
```

So, per line:

- trim `" \t\r"` (whitespace and CR).
- a blank line, or a line whose first byte is `#`, is **skipped**.
- if the line contains `=`: the key is `trim(before, " \t")`; the value is
  `trim(after, " \t")`, and if the value is wrapped in double quotes (`"…"`),
  the **surrounding quotes are stripped** (not decoded — the per-field parsers
  decode any inner escapes later). This yields `--key=value`.
- otherwise the whole trimmed line is the key with **no value** (yields
  `--<line>`).

The `--key=value` / `--key` form then feeds `parse`, which splits on `=` and
calls `parseIntoField` (the roastty analog is `Config::set(key, value)`). So a
line maps to `set(key, Some(value))` (or `set(key, None)` for a bare key).

(`MAX_LINE_SIZE = 4096` bounds a single line; that buffering belongs to the IO
driver, not this per-line extraction.)

## Rust mapping (`roastty/src/config/loader.rs`, new module)

A new `mod loader;` (in `config/mod.rs`) with:

```rust
/// Parse one config-file line into a `(key, value)` pair (upstream
/// `cli.args.LineIterator.next`'s per-line logic). Returns `None` for a blank line
/// or a `#` comment. A line with `=` yields `(key, Some(value))` with the key/value
/// `" \t"`-trimmed and the value's surrounding double quotes stripped; a line with no
/// `=` yields `(key, None)`.
pub(crate) fn parse_config_line(line: &str) -> Option<(&str, Option<&str>)> {
    let trimmed = line.trim_matches(|c: char| c == ' ' || c == '\t' || c == '\r');
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    match trimmed.find('=') {
        Some(idx) => {
            let key = trimmed[..idx].trim_matches(ws);
            let mut value = trimmed[idx + 1..].trim_matches(ws);
            if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
                value = &value[1..value.len() - 1];
            }
            Some((key, Some(value)))
        }
        None => Some((trimmed, None)),
    }
}
```

(`ws` is the `" \t"` predicate.) The returned slices borrow `line`. A
blank/comment line is `None` (skip); a `key = value` line is
`(key, Some(value))`; a bare `key` line is `(key, None)` — matching the
`--key=value` / `--key` forms upstream builds.

## Scope / faithfulness notes

- **Ported (bridged)**: the per-line key/value extraction of
  `cli.args.LineIterator.next`, as `config::loader::parse_config_line`.
- **Faithful**: the `" \t\r"` line trim; the blank-line / `#`-comment skip; the
  `=` split with `" \t"`-trimmed key and value; the surrounding-double-quote
  strip (not decode); the no-`=` bare-key form.
- **Faithful adaptation**: the iterator's `--key=value` arg construction →
  returning `(key, Option<value>)` directly (the roastty loader calls
  `Config::set(key, value)` rather than re-parsing a `--` arg); the
  IO/buffer/`MAX_LINE_SIZE` machinery is the next experiment's driver.
- **Deferred**: the multi-line `Config::load_str` driver (iterating lines,
  calling `Config::set`, collecting diagnostics), file IO, and the `--key=value`
  CLI-arg form; `background-image-opacity` stays float-blocked.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/loader.rs` (new): `parse_config_line`.
2. `roastty/src/config/mod.rs`: add `mod loader;`.
3. Tests (in `loader.rs`): `key = value` ⇒ `("key", Some("value"))`; whitespace
   trimmed (`  key  =  value  `); a quoted value (`key = "a b"`) ⇒ quotes
   stripped (`("key", Some("a b"))`); an empty value (`key =`) ⇒
   `("key", Some(""))`; a bare key (`flag`) ⇒ `("flag", None)`; a blank line and
   a `# comment` (and `  # x`) ⇒ `None`; a CRLF line (`key=value\r`) ⇒ the `\r`
   trimmed.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty parse_config_line
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `parse_config_line` reproduces upstream's per-line extraction (trim,
  blank/comment skip, `=` split with trimmed key/value, surrounding-quote strip,
  bare-key form);
- the tests pass (the value / quoted / empty / bare / comment / blank / CRLF
  cases), and the existing tests still pass;
- the multi-line driver and file IO stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the line parse diverges from upstream, an unrelated
item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the extraction matches upstream `LineIterator.next`:
the whole-line trim uses `whitespace ++ "\r"` (= `" \t\r"`) while the post-`=`
key/value trim uses only `" \t"` (`args.zig:1452`/`:1465`); blank/comment
detection happens after the whole-line trim, so leading-space comments are
skipped (`args.zig:1459`); quoted values have only their outer quotes stripped,
with no escape decoding in `LineIterator` (`args.zig:1469`); a line without `=`
stays a bare CLI-style key, so mapping it to `(key, None)` is faithful to the
upstream `--<entry>` construction (`args.zig:1491`); and `key =` producing
`Some("")` is correct (the later `Config::set` empty-reset handles the field
default). Codex's one note: this assumes `parse_config_line` receives a line
**without** the trailing `\n` delimiter, matching upstream's line reader (which
strips `\n` and trims the CR) — documented in the function (the multi-line
driver splits on `\n`).

Review artifacts:

- Prompt: `logs/codex-review/20260604-185650-d528-prompt.md` (design)
- Result: `logs/codex-review/20260604-185650-d528-last-message.md` (design)
