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

# Experiment 611: base64 scalar decoder

## Description

Another self-contained, dependency-free port taken while the search subsystem's
last piece (the outer libxev `Thread`) stays blocked: upstream
`simd/base64_scalar.zig`. It is the **scalar base64 decoder** the terminal uses
for Kitty Graphics image payloads and OSC 52 clipboard data — a copy of Zig
stdlib's `Base64Decoder` with the invalid-padding checks commented out (Kitty
requires a lenient decoder). roastty currently re-implements base64 ad-hoc at
each use site (`kitty/graphics_command.rs`, `os/file.rs`) but has no canonical
base64 module; this lands one, faithful to upstream, joining roastty's other
flattened `simd/`/`datastruct/` ports in `terminal/`.

It is fully self-contained: `std` + an `assert` only, no SIMD intrinsics (the
"fast" path is wide-integer arithmetic over precomputed lookup tables), no
external crates.

## Upstream behavior (`base64_scalar.zig`)

`scalar_decoder` = `Base64Decoder.init(standard_alphabet, null)` (standard
alphabet, **no** pad char). `Base64Decoder`:

- `char_to_index: [256]u8` — `c` → its 0–63 value, or `invalid_char` (0xff).
- `fast_char_to_index: [4][256]u32` — for char at the i-th position of a 4-char
  group, the 6-bit value `ci` is spread into bit positions so OR-ing the four
  produces three little-endian output bytes:
  - `[0][c] = ci << 2`
  - `[1][c] = (ci >> 4) | ((ci & 0x0f) << 12)`
  - `[2][c] = ((ci & 0x3) << 22) | ((ci & 0x3c) << 6)`
  - `[3][c] = ci << 16`
  - invalid chars carry the `invalid_char_tst = 0xff000000` sentinel bit.
- `calcSizeUpperBound(len)` / `calcSizeForSlice(src)` — decoded length (and the
  no-pad / pad `InvalidPadding` rules).
- `decode(dest, src)`:
  1. with a pad char, `src.len % 4 != 0` → `InvalidPadding`;
  2. fast loop in 16-char / 12-byte strides (four `fast_char_to_index` ORs per
     4-char sub-group into a `u128`, sentinel-check, little-endian write);
  3. then 4-char / 3-byte strides into a `u32`;
  4. scalar leftover via a `u12` accumulator (`acc<<6 + d`, emit a byte each
     time `acc_len >= 8`), stopping at the first pad char;
  5. the two invalid-padding checks are **commented out** (lenient); padding
     bytes after the data are validated (count must equal `acc_len / 2`).

## Rust mapping (`roastty/src/terminal/base64_scalar.rs`, new file)

The `dest: []u8` output (sized by `calc_size_*`) is kept as a `&mut [u8]`
(matching upstream's API); a `decode_alloc` convenience allocates a right-sized
`Vec<u8>`. The error union becomes a `Base64Error` enum (`InvalidCharacter` /
`InvalidPadding` / `NoSpaceLeft`). The tables are built at construction
(upstream's `init` loop) rather than `comptime`, exposed via `scalar_decoder()`.

```rust
//! Scalar base64 decoder for Kitty Graphics / OSC 52 payloads (port of upstream
//! `simd/base64_scalar`). A copy of Zig stdlib's `Base64Decoder` with the invalid-padding checks
//! commented out, because Kitty Graphics requires a decoder that tolerates malformed padding.

/// Decode errors (upstream `Base64Decoder.Error`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Base64Error {
    InvalidCharacter,
    InvalidPadding,
    NoSpaceLeft,
}

const INVALID_CHAR: u8 = 0xff;
const INVALID_CHAR_TST: u32 = 0xff000000;
const STANDARD_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(crate) struct Base64Decoder {
    char_to_index: [u8; 256],
    fast_char_to_index: [[u32; 256]; 4],
    pad_char: Option<u8>,
}

/// The standard-alphabet, no-pad lenient decoder (upstream `scalar_decoder`).
pub(crate) fn scalar_decoder() -> Base64Decoder {
    Base64Decoder::new(STANDARD_ALPHABET, None)
}

impl Base64Decoder {
    pub(crate) fn new(alphabet: &[u8; 64], pad_char: Option<u8>) -> Base64Decoder {
        let mut d = Base64Decoder {
            char_to_index: [INVALID_CHAR; 256],
            fast_char_to_index: [[INVALID_CHAR_TST; 256]; 4],
            pad_char,
        };
        // Upstream `init` asserts alphabet uniqueness and that no alphabet char equals the pad char.
        let mut seen = [false; 256];
        for (i, &c) in alphabet.iter().enumerate() {
            assert!(!seen[c as usize], "duplicate base64 alphabet char");
            assert!(pad_char != Some(c), "pad char overlaps alphabet");
            let ci = i as u32;
            d.fast_char_to_index[0][c as usize] = ci << 2;
            d.fast_char_to_index[1][c as usize] = (ci >> 4) | ((ci & 0x0f) << 12);
            d.fast_char_to_index[2][c as usize] = ((ci & 0x3) << 22) | ((ci & 0x3c) << 6);
            d.fast_char_to_index[3][c as usize] = ci << 16;
            d.char_to_index[c as usize] = i as u8;
            seen[c as usize] = true;
        }
        d
    }

    pub(crate) fn calc_size_upper_bound(&self, source_len: usize) -> Result<usize, Base64Error> { /* … */ }
    pub(crate) fn calc_size_for_slice(&self, source: &[u8]) -> Result<usize, Base64Error> { /* … */ }

    /// Decode `source` into `dest` (which must be `calc_size_for_slice(source)` long). Upstream
    /// `decode`. The fast loops mirror upstream's `u128` / `u32` strides; the leftover uses a `u16`
    /// accumulator (upstream's `u12`; `u16` carries the same low bits).
    pub(crate) fn decode(&self, dest: &mut [u8], source: &[u8]) -> Result<(), Base64Error> { /* … */ }

    /// Convenience: allocate a right-sized `Vec` and decode into it.
    pub(crate) fn decode_alloc(&self, source: &[u8]) -> Result<Vec<u8>, Base64Error> {
        let mut out = vec![0u8; self.calc_size_for_slice(source)?];
        self.decode(&mut out, source)?;
        Ok(out)
    }
}
```

The fast loops translate directly: the `inline for (0..4)` becomes a
`for i in 0..4`; `std.mem.writeInt(u128/u32, …, .little)` becomes
`dest[..].copy_from_slice(&bits.to_le_bytes())`; the bounds
`fast_src_idx + 16 < source.len && dest_idx + 15 < dest.len` and the 4-stride
equivalent are preserved exactly. The scalar leftover uses a `u16` accumulator
masked to 12 bits
(`acc = ((acc << 6) + d) & 0x0fff; acc_len += 6; while acc_len >= 8 { acc_len -= 8; dest[dest_idx] = (acc >> acc_len) as u8; … }`),
where the `& 0x0fff` keeps the `u16` exactly equivalent to upstream's `u12`. The
first non-alphabet, non-pad char → `InvalidCharacter`; the first pad char
records `leftover_idx` and breaks; and the commented-out invalid-padding checks
are kept as comments (verbatim intent).

The trailing pad-tail loop is ported **exactly**, including upstream's odd
branch: for each leftover byte `c`, if `c != pad_char` then
`return if c == INVALID_CHAR /* 0xff */ { InvalidCharacter } else { InvalidPadding }`
— it compares the **raw source byte** to the `0xff` sentinel, not whether it is
a valid alphabet char. Otherwise `padding_chars += 1`, and the final
`padding_chars == acc_len / 2` check is ported.

Registered in `terminal/mod.rs` as `#[allow(dead_code)] mod base64_scalar;`
(alphabetically after `array_list_collection`, before `bitmap_allocator`).

### Notes / deviations

- `comptime`-built tables → runtime `new` (the loop is identical);
  `scalar_decoder` is a function, not a `const`, since the tables aren't
  trivially const in Rust.
- `dest: []u8` + `calc_size_*` mirror upstream; `decode_alloc` is an additive
  convenience (Rust callers usually want an owned `Vec`).
- The leftover accumulator widens `u12`→`u16` (Rust has no `u12`); only the low
  `acc_len` bits are read, so behavior is identical.
- The lenient invalid-padding checks stay commented out, matching upstream's
  Kitty-compatibility deviation from Zig stdlib.

## Verification

- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — no regressions; new tests:
  - `decode_known_vectors` — `TWFu`→`Man`, `TWE=`?(no — no-pad decoder)…; use
    the no-pad standard decoder: `TWFu`→`Man`, `TWFueQ`→`Many`,
    `SGVsbG8gV29ybGQ` →`Hello World`.
  - `decode_long_input_exercises_fast_path` — a **24-char** input (so the
    16-char fast loop AND the subsequent 4-char loop both run — the latter needs
    `fast_src_idx + 4 < len`, i.e. > 20) decodes correctly; compare against a
    reference scalar decode.
  - `decode_invalid_char_errors` — a non-alphabet byte → `InvalidCharacter`.
  - `calc_size_for_slice_matches_decoded_len` — the computed size equals the
    actual decoded length across several inputs.
  - `decode_tolerates_trailing_padding_like_kitty` — with a pad-char decoder,
    `TWE=` decodes to `Ma` (and the no-pad decoder treats `=` as invalid).
  - `pad_tail_error_branch` — with a pad-char decoder, `b"TQ=A"` →
    `InvalidPadding` and `b"TQ=\xff"` → `InvalidCharacter` (the raw-`0xff`
    branch).
  - round-trip: encode bytes with a small local standard-base64 encoder, decode
    with `scalar_decoder`, expect the original.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on the new file / `terminal/mod.rs` — clean.
- `git diff --check` — clean.

Pass = the decoder reproduces upstream's output across the fast (16/4-char) and
scalar paths, computes correct sizes, errors on invalid characters, and
tolerates Kitty-style padding leniency.

## Design Review

Codex reviewed the design and raised **two Required** findings, both adopted:

- **Required (adopted)**: `Base64Decoder::new` must preserve upstream `init`'s
  alphabet asserts — `assert!(!seen[c])` (uniqueness) and
  `assert!(pad_char != Some(c))` (no pad/alphabet overlap) — via a
  `seen: [bool; 256]` tracker, since `new` is `pub(crate)` and not only a
  fixed-standard constructor.
- **Required (adopted)**: the trailing pad-tail loop must mirror upstream's odd
  branch exactly — a non-pad byte returns `InvalidCharacter` only when the **raw
  byte equals `0xff`** (the `invalid_char` sentinel), else `InvalidPadding`; it
  does not test alphabet validity there. Tests added for `b"TQ=A"` →
  `InvalidPadding` and `b"TQ=\xff"` → `InvalidCharacter`.
- **Optional (adopted)**: the fast-path test uses a **24-char** input (the
  4-char loop needs `fast_src_idx + 4 < len`, so 20 chars wouldn't reach it).
- **Optional (adopted)**: the `u16` scalar accumulator is masked `& 0x0fff`
  after each update, making the `u12` equivalence exact and auditable.

Codex confirmed the rest is faithful: the lookup-table formulas, sentinel
values, fast-loop bounds and little-endian writes, no-pad sizing rules, the
lenient commented-out padding checks, and the additive `decode_alloc` helper.

Review artifacts:

- Prompt: `logs/codex-review/20260605-d611-prompt.md`
- Result: `logs/codex-review/20260605-d611-last-message.md`
