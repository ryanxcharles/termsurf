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

# Experiment 484: the config string codepoint iterator (config::string::CodepointIterator)

## Description

Several config value types parse a **string literal with Zig escape sequences**
(e.g. `selection-word-chars` reads `" \t;,"` and `"\u{2502};"`). The shared
helper behind that is upstream's `config/string.zig` — an allocation-free
`CodepointIterator` that walks a byte slice yielding codepoints, parsing `\n`,
`\t`, `\\`, `\xNN`, `\u{...}`, etc. as it goes. This experiment ports that
helper (the iterator and its escape-sequence parser) into a new
`roastty/src/config/string.rs` module, unblocking `SelectionWordChars` (a later
slice) and any other string-literal config value.

The escape parser is a faithful port of Zig's
`std.zig.string_literal.parseEscapeSequence` (the exact piece upstream delegates
to). The byte-array `parse(out, bytes)` variant in the same file stays deferred
(no consumer needs it yet; it shares the same escape logic).

## Upstream behavior

In `config/string.zig`:

```zig
pub fn codepointIterator(bytes: []const u8) CodepointIterator {
    return .{ .bytes = bytes, .i = 0 };
}

pub const CodepointIterator = struct {
    bytes: []const u8,
    i: usize,

    pub fn next(self: *CodepointIterator) error{InvalidString}!?u21 {
        if (self.i >= self.bytes.len) return null;
        switch (self.bytes[self.i]) {
            '\\' => return switch (std.zig.string_literal.parseEscapeSequence(self.bytes, &self.i)) {
                .failure => error.InvalidString,
                .success => |cp| cp,
            },
            else => |start| {
                const cp_len = std.unicode.utf8ByteSequenceLength(start) catch return error.InvalidString;
                defer self.i += cp_len;
                return std.unicode.utf8Decode(self.bytes[self.i..][0..cp_len]) catch return error.InvalidString;
            },
        }
    }
};
```

`next` yields one codepoint at a time: at a `\\`, it parses an escape sequence
(any failure → `error.InvalidString`); otherwise it decodes one UTF-8 codepoint
(advancing by the sequence length; a bad length or bad bytes →
`error.InvalidString`). End of input → `null`.

The escape parser, `std.zig.string_literal.parseEscapeSequence(slice, &offset)`
(the relevant subset, asserting `slice[offset] == '\\'`):

- If the `\\` is the last byte → failure.
- Advance past `\\` and the escape char, then switch on it:
  - `n` → `\n` (0x0A), `r` → `\r` (0x0D), `\\` → `\\` (0x5C), `t` → `\t` (0x09),
    `'` → `'` (0x27), `"` → `"` (0x22).
  - `x` → exactly two hex digits → a `u8` value (a missing or non-hex digit →
    failure).
  - `u` → `{`, then one-or-more hex digits, then `}` → a codepoint value (a
    missing `{`, an empty `{}`, a non-hex / non-`}` byte, a value `> 0x10FFFF`,
    or a missing `}` before end → failure).
  - anything else → failure (`invalid_escape_character`). `offset` is advanced
    past the consumed bytes on success.

Upstream's `codepointIterator` tests: `""` → no codepoints; `"abc"` → `a, b, c`;
`"a│b"` → `a, U+2502, b` (multi-byte UTF-8); `"a\tb\n\\"` (as the literal
`a\\tb\\n\\\\`) → `a, 0x09, b, 0x0A, 0x5C`; `"\u{2502}x"` → `U+2502, x`.

## Rust mapping (new `roastty/src/config/string.rs`)

```rust
//! String-literal codepoint iteration (port of ghostty `config/string.zig`).
//!
//! An allocation-free iterator over the codepoints of a string literal, parsing
//! Zig escape sequences (`\n`, `\t`, `\\`, `\xNN`, `\u{...}`, …) as it goes. The
//! byte-array `parse` variant in the upstream file is ported later.

/// A failure parsing a string literal (upstream `error.InvalidString`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InvalidString;

/// Iterate the codepoints of a string literal, parsing escape sequences
/// (upstream `config.string.codepointIterator`). Codepoints are `u32` (a `\u{...}`
/// escape may name a value, e.g. a surrogate, that is not a Rust `char`).
pub(crate) fn codepoint_iterator(bytes: &[u8]) -> CodepointIterator<'_> {
    CodepointIterator { bytes, i: 0 }
}

pub(crate) struct CodepointIterator<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl Iterator for CodepointIterator<'_> {
    type Item = Result<u32, InvalidString>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.bytes.len() {
            return None;
        }
        if self.bytes[self.i] == b'\\' {
            return Some(parse_escape_sequence(self.bytes, &mut self.i).ok_or(InvalidString));
        }
        // Not an escape: decode one UTF-8 codepoint. A bad leading byte fails
        // before advancing (upstream returns before its `defer self.i += cp_len`).
        let len = match utf8_sequence_length(self.bytes[self.i]) {
            Some(len) => len,
            None => return Some(Err(InvalidString)),
        };
        // Once the length is known, upstream installs `defer self.i += cp_len`, so
        // `i` advances by the sequence length even when the decode then fails.
        let start = self.i;
        self.i += len;
        let end = start + len;
        if end > self.bytes.len() {
            return Some(Err(InvalidString)); // truncated multi-byte sequence
        }
        match std::str::from_utf8(&self.bytes[start..end]) {
            Ok(s) => Some(Ok(s.chars().next().unwrap() as u32)),
            Err(_) => Some(Err(InvalidString)),
        }
    }
}

/// The UTF-8 sequence length of a leading byte (upstream
/// `std.unicode.utf8ByteSequenceLength`): 1 for ASCII, 2/3/4 for the multi-byte
/// leads, else `None` (a continuation or invalid start byte).
fn utf8_sequence_length(first: u8) -> Option<usize> {
    match first {
        0x00..=0x7F => Some(1),
        0xC0..=0xDF => Some(2),
        0xE0..=0xEF => Some(3),
        0xF0..=0xF7 => Some(4),
        _ => None,
    }
}

fn hex_digit(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a' + 10) as u32),
        b'A'..=b'F' => Some((c - b'A' + 10) as u32),
        _ => None,
    }
}

/// Parse a `\\`-escape at `bytes[*i]` (upstream
/// `std.zig.string_literal.parseEscapeSequence`). On success returns the codepoint
/// and advances `*i` past the consumed bytes; any failure is `None`. Assumes
/// `bytes[*i] == b'\\'`.
fn parse_escape_sequence(bytes: &[u8], i: &mut usize) -> Option<u32> {
    // A lone trailing backslash is invalid.
    if bytes.len() == *i + 1 {
        return None;
    }
    *i += 2;
    match bytes[*i - 1] {
        b'n' => Some(0x0A),
        b'r' => Some(0x0D),
        b'\\' => Some(0x5C),
        b't' => Some(0x09),
        b'\'' => Some(0x27),
        b'"' => Some(0x22),
        b'x' => {
            // Exactly two hex digits → a u8 value.
            let start = *i;
            let mut value: u32 = 0;
            let mut k = start;
            while k < start + 2 {
                if k == bytes.len() {
                    return None;
                }
                value = value * 16 + hex_digit(bytes[k])?;
                k += 1;
            }
            *i = k;
            Some(value)
        }
        b'u' => {
            // `{` hex+ `}` → a codepoint ≤ 0x10FFFF.
            let mut k = *i;
            if k >= bytes.len() || bytes[k] != b'{' {
                return None;
            }
            k += 1;
            if k >= bytes.len() || bytes[k] == b'}' {
                return None; // missing hex / empty `{}`
            }
            let mut value: u32 = 0;
            loop {
                if k >= bytes.len() {
                    return None; // no closing `}`
                }
                let c = bytes[k];
                if c == b'}' {
                    k += 1;
                    break;
                }
                value = value * 16 + hex_digit(c)?;
                if value > 0x10FFFF {
                    return None;
                }
                k += 1;
            }
            *i = k;
            Some(value)
        }
        _ => None, // invalid escape character
    }
}
```

`CodepointIterator` mirrors upstream's `next` (the `\\`-escape branch, the UTF-8
decode branch, and the end-of-input `None`); `parse_escape_sequence` mirrors
`parseEscapeSequence` (the single-char escapes, the two-digit `\x`, the
`\u{...}` codepoint, and the failure cases) — all failures collapse to
`InvalidString`, which is the only error `codepointIterator`'s caller observes.
Codepoints are `u32` (upstream `u21`) since a `\u{...}` may name a non-`char`
value.

## Scope / faithfulness notes

- **Ported (bridged)**: `config::string::codepoint_iterator` /
  `CodepointIterator` (upstream `config.string.codepointIterator` /
  `CodepointIterator`) and the escape parser `parse_escape_sequence` (upstream
  `std.zig.string_literal.parseEscapeSequence`), plus `InvalidString`,
  `utf8_sequence_length`, and `hex_digit`.
- **Faithful**: the `\\`-vs-UTF-8 dispatch; the end-of-input `None`; the
  single-char escapes (`n`/`r`/`\\`/`t`/`'`/`"`); the exactly-two-hex-digit `\x`
  `u8`; the `\u{...}` codepoint with the `{`, empty, non-hex, `> 0x10FFFF`, and
  missing-`}` failure cases; the invalid-escape-character failure; and the UTF-8
  sequence-length + decode (a bad length / bad bytes → `InvalidString`) —
  exactly upstream's `next` / `parseEscapeSequence`.
- **Faithful adaptation**: `[]const u8` → `&[u8]`; the detailed Zig `Error`
  union → a single `InvalidString` (the iterator collapses every escape failure
  to `error.InvalidString` anyway); `error{InvalidString}!?u21` → an
  `Iterator<Item = Result<u32, InvalidString>>` (codepoints as `u32`);
  `utf8Decode` → `std::str::from_utf8` of the leading sequence (same validation:
  overlong / surrogate / bad-continuation → error). After a valid leading byte,
  `i` advances by the sequence length even when the decode then fails, matching
  upstream's `defer self.i += cp_len` (folded in from the design review).
- **Deferred**: the byte-array `parse(out, bytes)` variant in the same file (no
  consumer yet; same escape logic), and the detailed escape `Error` reporting.
  (Consumed by later slices; this experiment lands the iterator.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/string.rs` (new): the module doc, `InvalidString`,
   `codepoint_iterator` / `CodepointIterator`, `parse_escape_sequence`,
   `utf8_sequence_length`, `hex_digit`, and the tests.
2. `roastty/src/config/mod.rs`: add `mod string;` (with `#[allow(dead_code)]` if
   needed, consistent with the crate's other not-yet-wired config pieces).
3. Tests (in `config/string.rs`): a helper collecting
   `codepoint_iterator(s.as_bytes())` into `Result<Vec<u32>, InvalidString>`,
   asserting:
   - upstream cases: `""` → `[]`; `"abc"` → `[a, b, c]`; `"a│b"` →
     `[a, 0x2502, b]`; `"a\\tb\\n\\\\"` → `[a, 0x09, b, 0x0A, 0x5C]`;
     `"\\u{2502}x"` → `[0x2502, x]`.
   - single-char escapes: `"\\n\\r\\t\\'\\\""` →
     `[0x0A, 0x0D, 0x09, 0x27, 0x22]`.
   - `\x`: `"\\xFF"` → `[0xFF]`; `"\\x"` / `"\\xG"` / `"\\xA"` (one digit at
     end) → `Err`.
   - `\u`: `"\\u{1F601}"` → `[0x1F601]`; `"\\u"` / `"\\u{"` / `"\\u{}"` /
     `"\\u{2502"` / `"\\u{110000}"` → `Err`.
   - other failures: `"\\"` (lone backslash) and `"\\q"` (invalid escape) →
     `Err`.
   - invalid UTF-8 over raw bytes (design-review Low): a valid 2-byte lead
     followed by a bad continuation (`&[0xC2, 0x20]`) → `Err`, locking the
     advance-on-decode-failure behavior.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty config::string
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `CodepointIterator` yields the correct codepoints for plain UTF-8 and the full
  escape grammar (`\n`/`\r`/`\t`/`\\`/`'`/`"`, `\xNN`, `\u{...}`), and returns
  `InvalidString` on every upstream failure case — faithful to upstream's `next`
  / `parseEscapeSequence`;
- the tests pass (the upstream cases; the escape and error cases), and the
  existing tests still pass;
- the byte-array `parse` variant and the detailed escape `Error` stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a codepoint or escape is parsed wrong (wrong escape
value, wrong `\x`/`\u` handling, a failure case accepted or a valid case
rejected), an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with no
Required findings (one **Recommended** and two **Low**, all folded in). It
verified against the vendored upstream and the Zig 0.16 `string_literal.zig`
source: `parseEscapeSequence`'s offset advancement on success and failure
matches Zig's `offset += 2` then local-loop behavior; `\x` is exactly two hex
digits; `\u{...}` handles the empty / missing / invalid / overflow cases;
invalid escape chars collapse to `InvalidString`; and `u32` is the right Rust
representation for Zig's `u21` result.

- **Recommended (folded in):** match upstream's iterator state after a UTF-8
  decode failure. Once `utf8ByteSequenceLength` succeeds, upstream installs
  `defer self.i += cp_len` _before_ `utf8Decode` (`string.zig:63`), so a
  malformed continuation / overlong / surrogate still advances `i` by the
  sequence length before returning `InvalidString`. The `from_utf8` path now
  advances `self.i` before handling the result.
- **Low (folded in):** add an invalid-UTF-8 state test (a valid 2-byte lead with
  a bad continuation) to lock that behavior; and remember the `mod string;`
  wiring in `config/mod.rs` (already in the Changes list).

Review artifacts:

- Prompt: `logs/codex-review/20260604-141313-d484-prompt.md` (design)
- Result: `logs/codex-review/20260604-141313-d484-last-message.md` (design)
