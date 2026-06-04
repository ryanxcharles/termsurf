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

# Experiment 525: the double-quoted string-literal decoder (parse_quoted_string)

# Description

Toward `Theme::parse_cli`, this experiment ports the next `parseAutoStruct`
building block: the double-quoted value decoder. `parseAutoStruct` decodes a
`"…"` field value as a Zig string literal via
`std.zig.string_literal.parseWrite`. This experiment ports `parseWrite` as
`config::string::parse_quoted_string`, building on the existing
`parse_escape_sequence` (Experiment 484). Ported ahead of its consumer
(`parseAutoStruct`), the same approach used for `CommaSplitter` (Experiment
524).

# Upstream behavior

`std.zig.string_literal.parseWrite` (Zig std, `zig/string_literal.zig`):

```zig
pub fn parseWrite(writer, bytes) !Result {
    assert(bytes.len >= 2 and bytes[0] == '"' and bytes[bytes.len - 1] == '"');
    var index: usize = 1;
    while (true) {
        const b = bytes[index];
        switch (b) {
            '\\' => {
                const escape_char_index = index + 1;
                const result = parseEscapeSequence(bytes, &index);  // advances index
                switch (result) {
                    .success => |cp| {
                        if (bytes[escape_char_index] == 'u') {
                            // UTF-8 encode the codepoint (fails ⇒ invalid_unicode_codepoint)
                            var buf: [4]u8; const len = utf8Encode(cp, &buf) catch return .{ .failure = … };
                            try writer.writeAll(buf[0..len]);
                        } else {
                            try writer.writeByte(@intCast(cp));        // single byte
                        }
                    },
                    .failure => |err| return .{ .failure = err },
                }
            },
            '\n' => return .{ .failure = .{ .invalid_character = index } },  // a literal newline
            '"' => return .success,                                          // closing quote
            else => { try writer.writeByte(b); index += 1; },                // a content byte
        }
    }
}
```

So, decoding the content between the surrounding quotes:

- `\` → `parseEscapeSequence` (advancing past the escape). A `\u{…}` escape's
  codepoint is **UTF-8-encoded** (a surrogate / out-of-range value ⇒ failure);
  any other escape (`\n \r \t \\ \' \"`, `\xNN`) writes the codepoint as a
  **single byte**.
- a **literal newline** byte (`\n`, `0x0A`) ⇒ failure.
- the closing `"` ⇒ success.
- any other byte is copied verbatim (so multibyte UTF-8 content is preserved).

The decoded result is the **bytes** written (not necessarily a Rust `String` —
the conversion to `Theme`'s `light` / `dark` strings happens in the `Theme`
experiment).

# Rust mapping (`roastty/src/config/string.rs`)

```rust
/// Decode a double-quoted string literal (upstream `std.zig.string_literal.parseWrite`):
/// `bytes` must start and end with `"`. Returns the decoded bytes, or `None` on a
/// failure (a malformed escape, a `\u{…}` codepoint that is not valid UTF-8, a
/// literal newline, or a missing surrounding quote). `\u{…}` escapes are UTF-8
/// encoded; every other escape writes a single byte.
pub(crate) fn parse_quoted_string(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < 2 || bytes[0] != b'"' || bytes[bytes.len() - 1] != b'"' {
        return None;
    }
    let mut out = Vec::new();
    let mut index = 1usize;
    loop {
        match *bytes.get(index)? {
            b'\\' => {
                let escape_char_index = index + 1;
                let cp = parse_escape_sequence(bytes, &mut index)?; // advances index
                if bytes.get(escape_char_index) == Some(&b'u') {
                    let ch = char::from_u32(cp)?; // invalid codepoint ⇒ failure
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                } else {
                    out.push(cp as u8); // upstream `@intCast(cp)`
                }
            }
            b'\n' => return None, // a literal newline
            b'"' => return Some(out),
            other => {
                out.push(other);
                index += 1;
            }
        }
    }
}
```

`parse_escape_sequence` already advances `index` past the escape and returns the
codepoint (or `None` on a malformed escape), exactly upstream's
`parseEscapeSequence`. The `\u` vs other distinction reuses `escape_char_index`
(the byte after `\`). `char::from_u32(cp)` standing in for `utf8Encode`'s
validity check (a surrogate / out-of-range value ⇒ `None`). Upstream's
panic-on-OOB indexing becomes the bounds-safe `bytes.get(index)?` (well-formed
input hits the closing `"` first; a malformed input fails).

# Scope / faithfulness notes

- **Ported (bridged)**: `std.zig.string_literal.parseWrite`, as
  `config::string::parse_quoted_string`.
- **Faithful**: content decoded between the surrounding quotes; `\u{…}`
  UTF-8-encoded (invalid ⇒ failure); other escapes single-byte; a literal
  newline ⇒ failure; the closing `"` ⇒ success; other bytes copied verbatim.
  Returns the decoded **bytes** (`Vec<u8>`), as upstream returns a byte buffer.
- **Faithful adaptation**: `writer.writeAll`/`writeByte` → pushing to a
  `Vec<u8>`; `utf8Encode` validity → `char::from_u32`; the `.failure` results
  (malformed escape, invalid unicode codepoint, literal newline) → `None`; OOB
  indexing → `bytes.get(index)?`.
- **Deferred**: `parseAutoStruct` (its consumer) and `Theme::parse_cli`; the
  `theme` `Config::set` arm; the `loadCli` / file loader.
  `background-image-opacity` stays float-blocked.
- No C ABI/header/ABI-inventory change (internal Rust).

# Changes

1. `roastty/src/config/string.rs`: add `parse_quoted_string`.
2. Tests (in `string.rs`): `"abc"` ⇒ `abc`; `"a,b"` ⇒ `a,b`; `"a\nb"` (the
   escape) ⇒ the bytes `a`, `0x0A`, `b`; `"a\x41b"` ⇒ `aAb`; `"\u{48}\u{49}"` ⇒
   `HI`; a multibyte/copied byte case; failures — a literal newline ⇒ `None`, a
   bad escape (`"\q"`) ⇒ `None`, a surrogate (`"\u{d800}"`) ⇒ `None`, a missing
   surrounding quote ⇒ `None`.
3. Format and test (`cargo fmt`, accept output).

# Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty quoted_string
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `parse_quoted_string` reproduces upstream `parseWrite`: escape decoding (`\u`
  UTF-8-encoded, others single-byte), literal-newline failure, closing-quote
  success, verbatim content bytes, and the failure cases;
- the tests pass (the decode cases + the failure cases), and the existing tests
  still pass;
- `parseAutoStruct` / `Theme::parse_cli` and the loader stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the decode diverges from upstream, an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the proposed function is a faithful adaptation of
`parseWrite` (`string_literal.zig:322`): `escape_char_index = index + 1` and
checking `bytes[escape_char_index] == 'u'` is exactly upstream's discriminator
(`:331`); `cp as u8` for non-`\u` escapes is faithful to `@intCast(codepoint)`
since those escapes are single-byte by construction (`:341`);
`char::from_u32(cp)` is the right stand-in for `utf8Encode` (Zig rejects
surrogate halves and too-large codepoints, `unicode.zig:44`/`:67`); the raw
`b'\n'` failure matches upstream's literal-newline failure and stays distinct
from the `\n` escape (`:348`); and returning `Vec<u8>` is the faithful shape
(upstream writes decoded bytes to a writer).

Review artifacts:

- Prompt: `logs/codex-review/20260604-184204-d525-prompt.md` (design)
- Result: `logs/codex-review/20260604-184204-d525-last-message.md` (design)

## Result

**Result:** Pass

`config::string::parse_quoted_string(bytes) -> Option<Vec<u8>>` was added — a
faithful port of `std.zig.string_literal.parseWrite`: requires the surrounding
quotes; decodes the content via the existing `parse_escape_sequence`
(UTF-8-encoding `\u{…}`, single byte for other escapes); fails on a literal
newline, a malformed escape, an invalid `\u` codepoint, or a missing quote;
copies other bytes verbatim. The new test
`parse_quoted_string_decodes_and_fails` covers the decode cases (plain, comma,
`\n`/`\xNN`/`\u{…}` escapes, multibyte) and the failure cases.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3013 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation matches `parseWrite` — surrounding-quote
requirement, escape dispatch through `parse_escape_sequence`, UTF-8 encoding for
`\u{…}`, byte output for non-Unicode escapes, raw-newline failure, and raw byte
copying otherwise; the surrogate failure via `char::from_u32` is faithful to
Zig's `utf8Encode` rejection; the tests cover the normal and edge cases; gates
are clean and the consumer work remains deferred. "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-184404-r525-prompt.md` (result)
- Result: `logs/codex-review/20260604-184404-r525-last-message.md` (result)

## Conclusion

`parse_quoted_string` (the `parseWrite` equivalent) is ported. Both
`parseAutoStruct` building blocks now exist — `CommaSplitter` (Experiment 524)
and `parse_quoted_string`. The next experiment ports **`parseAutoStruct`** (the
colon-keyed `key:value` comma-list parser that drives `CommaSplitter`, decodes
double-quoted values, and tracks required fields), then `Theme::parse_cli` + the
`theme` `Config::set` arm — the last parseable field. Then the `loadCli` /
config-file loader.
