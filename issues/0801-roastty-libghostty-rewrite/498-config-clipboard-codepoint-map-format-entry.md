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

# Experiment 498: the clipboard-codepoint-map formatter (RepeatableClipboardCodepointMap::format_entry)

## Description

Continuing the config **formatter** layer (Experiments 491–497), this experiment
ports `RepeatableClipboardCodepointMap.formatEntry` (upstream
`Config.RepeatableClipboardCodepointMap`) — the inverse of the Experiment 487
parser. It renders each map entry back to a `U+XXXX[-U+YYYY]=value` line
(uppercase 4-digit hex keys; the value is `U+XXXX` for a codepoint replacement
or the literal string), with an empty `name = \n` entry for an empty map.
Grounded by the `EntryFormatter` from Experiment 491.

## Upstream behavior

In `config/Config.zig`, `RepeatableClipboardCodepointMap.formatEntry`:

```zig
pub fn formatEntry(self: Self, formatter: anytype) !void {
    if (self.map.list.len == 0) {
        try formatter.formatEntry(void, {});
        return;
    }

    var buf: [1024]u8 = undefined;
    var value_buf: [32]u8 = undefined;
    const ranges = self.map.list.items(.range);
    const replacements = self.map.list.items(.replacement);
    for (ranges, replacements) |range, replacement| {
        const value_str = switch (replacement) {
            .codepoint => |cp| try std.fmt.bufPrint(&value_buf, "U+{X:0>4}", .{cp}),
            .string => |s| s,
        };

        if (range[0] == range[1]) {
            try formatter.formatEntry([]const u8, std.fmt.bufPrint(&buf, "U+{X:0>4}={s}", .{ range[0], value_str }) catch ...);
        } else {
            try formatter.formatEntry([]const u8, std.fmt.bufPrint(&buf, "U+{X:0>4}-U+{X:0>4}={s}", .{ range[0], range[1], value_str }) catch ...);
        }
    }
}
```

- An empty map writes a single void entry (`name = \n`).
- Otherwise it writes **one entry per map entry**. The replacement value is
  rendered as `U+{cp uppercase, zero-padded to 4}` for a codepoint, or the
  literal string for a string. The key is `U+{start}` for a single codepoint
  (`range[0] == range[1]`) or `U+{start}-U+{end}` for a range; the entry is
  `key=value`. The hex is **uppercase**, zero-padded to at least four digits
  (`{X:0>4}`).

Upstream's tests: `"U+2500=U+002D"` formats to `a = U+2500=U+002D\n`;
`"U+03A3=SUM"` formats to `a = U+03A3=SUM\n`.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
impl RepeatableClipboardCodepointMap {
    /// Format as config entries (upstream `RepeatableClipboardCodepointMap.formatEntry`):
    /// an empty map writes one empty entry; otherwise one `U+XXXX[-U+YYYY]=value`
    /// entry per mapping (uppercase 4-digit hex keys; the value is `U+XXXX` for a
    /// codepoint replacement, else the literal string).
    pub(crate) fn format_entry(&self, formatter: &mut EntryFormatter) {
        if self.map.is_empty() {
            formatter.entry_void();
            return;
        }
        for entry in &self.map {
            let value = match &entry.replacement {
                ClipboardReplacement::Codepoint(cp) => format!("U+{:04X}", cp),
                ClipboardReplacement::String(s) => s.clone(),
            };
            let [start, end] = entry.range;
            let key = if start == end {
                format!("U+{:04X}", start)
            } else {
                format!("U+{:04X}-U+{:04X}", start, end)
            };
            formatter.entry_str(&format!("{}={}", key, value));
        }
    }
}
```

`format_entry` mirrors upstream: the empty-map `entry_void`, then one entry per
mapping. The value is `format!("U+{:04X}", cp)` (uppercase, zero-padded to four
— the `{X:0>4}` equivalent) for a codepoint or the literal string; the key is
the single-codepoint or `start-end` range form; and the entry is `key=value`.
`format_entry` takes `&self` (it holds a non-`Copy` `Vec`).

## Scope / faithfulness notes

- **Ported (bridged)**: `RepeatableClipboardCodepointMap::format_entry`
  (upstream `RepeatableClipboardCodepointMap.formatEntry`).
- **Faithful**: the empty-map void entry; one `key=value` entry per mapping; the
  `U+{uppercase 4-digit hex}` codepoint value or the literal string value; the
  single-codepoint (`start == end`) vs `start-end` range key — exactly
  upstream's `formatEntry`.
- **Faithful adaptation**: `formatEntry(void, {})` → `entry_void`;
  `formatEntry([]const u8, bufPrint(…))` → `entry_str(&format!(…))`;
  `"U+{X:0>4}"` (uppercase, min-4-digit, zero-padded) → `"U+{:04X}"`; the
  fixed-buffer / `OutOfMemory` path has no Rust analog (a `String` format cannot
  fail).
- **Deferred**: the remaining types' `formatEntry` (ported in later slices), the
  generic field-dispatch `formatEntry`, and the broader config parser/formatter.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add
   `RepeatableClipboardCodepointMap::format_entry` (in the existing `impl`).
2. Tests (in `config/mod.rs`):
   - a round-trip via `parse_cli`: `"U+2500=U+002D"` formats to
     `"a = U+2500=U+002D\n"`; `"U+03A3=SUM"` formats to `"a = U+03A3=SUM\n"`;
     `"U+2500-U+2503=|"` formats to `"a = U+2500-U+2503=|\n"`.
   - the empty map → `"a = \n"`.
   - accumulation: two entries → two lines.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty clipboard_codepoint_map_format_entry
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `RepeatableClipboardCodepointMap::format_entry` writes one
  `U+XXXX[-U+YYYY]=value` entry per mapping (or one empty entry), with the
  uppercase 4-digit hex keys and the codepoint / string value — faithful to
  upstream's `formatEntry`;
- the tests pass (the round-trips; the empty map; the accumulation), and the
  existing tests still pass;
- the other types' `formatEntry` and the generic field-dispatch stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a formatted entry differs from upstream (wrong
key/value shape, lowercase hex, wrong padding, wrong single-vs-range branch), an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the formatter matches upstream: an empty map emits one
void entry, non-empty maps emit one line per mapping, codepoint replacements
format as uppercase zero-padded `U+{X:0>4}`, string replacements are literal,
and keys use either `U+START=…` or `U+START-U+END=…`
(`Config.zig:8308`/`:8322`/`:8327`); `U+{:04X}` is the right equivalent of
`U+{X:0>4}` (uppercase, minimum width 4, no truncation for larger codepoints);
and the tests cover the upstream codepoint / string cases, a range, empty
output, and multi-entry accumulation (`:8400`/`:8415`).

Review artifacts:

- Prompt: `logs/codex-review/20260604-154345-d498-prompt.md` (design)
- Result: `logs/codex-review/20260604-154345-d498-last-message.md` (design)
