//! String-literal codepoint iteration (port of upstream `config/string.zig`).
//!
//! An allocation-free iterator over the codepoints of a string literal, parsing
//! Zig escape sequences (`\n`, `\t`, `\\`, `\xNN`, `\u{...}`, …) as it goes. The
//! byte-array `parse` variant in the upstream file is ported later.
#![allow(dead_code)]

/// A failure parsing a string literal (upstream `error.InvalidString`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InvalidString;

/// Iterate the codepoints of a string literal, parsing escape sequences (upstream
/// `config.string.codepointIterator`). Codepoints are `u32` (a `\u{...}` escape may
/// name a value, e.g. a surrogate, that is not a Rust `char`).
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
        // Not an escape: decode one UTF-8 codepoint. A bad leading byte fails before
        // advancing (upstream returns before its `defer self.i += cp_len`).
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

#[cfg(test)]
mod tests {
    use super::*;

    fn codepoints(s: &str) -> Result<Vec<u32>, InvalidString> {
        codepoint_iterator(s.as_bytes()).collect()
    }

    fn codepoints_bytes(bytes: &[u8]) -> Result<Vec<u32>, InvalidString> {
        codepoint_iterator(bytes).collect()
    }

    #[test]
    fn codepoint_iterator_plain_and_escapes() {
        // Upstream `codepointIterator` cases.
        assert_eq!(codepoints(""), Ok(vec![]));
        assert_eq!(codepoints("abc"), Ok(vec![0x61, 0x62, 0x63]));
        assert_eq!(codepoints("a│b"), Ok(vec![0x61, 0x2502, 0x62])); // multi-byte UTF-8
        assert_eq!(
            codepoints("a\\tb\\n\\\\"),
            Ok(vec![0x61, 0x09, 0x62, 0x0A, 0x5C])
        );
        assert_eq!(codepoints("\\u{2502}x"), Ok(vec![0x2502, 0x78]));

        // Single-char escapes.
        assert_eq!(
            codepoints("\\n\\r\\t\\'\\\""),
            Ok(vec![0x0A, 0x0D, 0x09, 0x27, 0x22])
        );
    }

    #[test]
    fn codepoint_iterator_hex_and_unicode_escapes() {
        // `\xNN` is exactly two hex digits.
        assert_eq!(codepoints("\\xFF"), Ok(vec![0xFF]));
        assert_eq!(codepoints("\\x00"), Ok(vec![0x00]));
        assert_eq!(codepoints("\\x"), Err(InvalidString)); // no digits
        assert_eq!(codepoints("\\xG"), Err(InvalidString)); // non-hex
        assert_eq!(codepoints("\\xA"), Err(InvalidString)); // one digit, then end

        // `\u{...}`.
        assert_eq!(codepoints("\\u{1F601}"), Ok(vec![0x1F601]));
        assert_eq!(codepoints("\\u"), Err(InvalidString)); // missing `{`
        assert_eq!(codepoints("\\u{"), Err(InvalidString)); // missing hex / `}`
        assert_eq!(codepoints("\\u{}"), Err(InvalidString)); // empty
        assert_eq!(codepoints("\\u{2502"), Err(InvalidString)); // no closing `}`
        assert_eq!(codepoints("\\u{110000}"), Err(InvalidString)); // > 0x10FFFF
    }

    #[test]
    fn codepoint_iterator_failures() {
        assert_eq!(codepoints("\\"), Err(InvalidString)); // lone backslash
        assert_eq!(codepoints("\\q"), Err(InvalidString)); // invalid escape char

        // Invalid UTF-8 over raw bytes: a valid 2-byte lead with a bad continuation.
        assert_eq!(codepoints_bytes(&[0xC2, 0x20]), Err(InvalidString));
    }
}
