//! Scalar base64 decoder for Kitty Graphics / OSC 52 payloads (port of upstream
//! `simd/base64_scalar`).
//!
//! A copy of Zig stdlib's `Base64Decoder` with the invalid-padding checks commented out, because
//! Kitty Graphics requires a decoder that tolerates malformed padding. The "fast" path is
//! wide-integer arithmetic over precomputed lookup tables (no SIMD intrinsics).

/// Decode errors (upstream `Base64Decoder.Error`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Base64Error {
    InvalidCharacter,
    InvalidPadding,
    NoSpaceLeft,
}

const INVALID_CHAR: u8 = 0xff;
const INVALID_CHAR_TST: u32 = 0xff00_0000;
const STANDARD_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// A base64 decoder with precomputed lookup tables (upstream `Base64Decoder`).
pub(crate) struct Base64Decoder {
    /// `c` → its 0–63 value, or `INVALID_CHAR`.
    char_to_index: [u8; 256],
    /// For a char at position `i` of a 4-char group, the 6-bit value spread into bit positions so
    /// OR-ing the four entries yields three little-endian output bytes; invalid chars carry the
    /// `INVALID_CHAR_TST` sentinel.
    fast_char_to_index: [[u32; 256]; 4],
    pad_char: Option<u8>,
}

/// The standard-alphabet, no-pad lenient decoder (upstream `scalar_decoder`).
pub(crate) fn scalar_decoder() -> Base64Decoder {
    Base64Decoder::new(STANDARD_ALPHABET, None)
}

impl Base64Decoder {
    /// Build a decoder for `alphabet` with an optional `pad_char` (upstream `init`).
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

    /// The maximum decoded size for a `source_len`-byte input (upstream `calcSizeUpperBound`). The
    /// actual size may be less if the input is padded.
    pub(crate) fn calc_size_upper_bound(&self, source_len: usize) -> Result<usize, Base64Error> {
        let mut result = source_len / 4 * 3;
        let leftover = source_len % 4;
        if self.pad_char.is_some() {
            if leftover % 4 != 0 {
                return Err(Base64Error::InvalidPadding);
            }
        } else {
            if leftover % 4 == 1 {
                return Err(Base64Error::InvalidPadding);
            }
            result += leftover * 3 / 4;
        }
        Ok(result)
    }

    /// The exact decoded size for `source` (upstream `calcSizeForSlice`).
    pub(crate) fn calc_size_for_slice(&self, source: &[u8]) -> Result<usize, Base64Error> {
        let source_len = source.len();
        let mut result = self.calc_size_upper_bound(source_len)?;
        if let Some(pad) = self.pad_char {
            if source_len >= 1 && source[source_len - 1] == pad {
                result -= 1;
            }
            if source_len >= 2 && source[source_len - 2] == pad {
                result -= 1;
            }
        }
        Ok(result)
    }

    /// Decode `source` into `dest`, which must be `calc_size_for_slice(source)` bytes (upstream
    /// `decode`). The fast loops mirror upstream's `u128` / `u32` strides; the scalar leftover uses a
    /// `u32` accumulator masked to 12 bits (upstream's `u12` — the wider type avoids a pre-mask
    /// overflow while staying bit-for-bit equivalent).
    pub(crate) fn decode(&self, dest: &mut [u8], source: &[u8]) -> Result<(), Base64Error> {
        if self.pad_char.is_some() && source.len() % 4 != 0 {
            return Err(Base64Error::InvalidPadding);
        }

        let mut dest_idx = 0usize;
        let mut fast_src_idx = 0usize;
        let mut acc: u32 = 0;
        let mut acc_len: u8 = 0;
        let mut leftover_idx: Option<usize> = None;

        // Fast path: 16 chars → 12 bytes. The bounds (strict `<`) guarantee more input remains, so
        // the 4 extra zero bytes written by the 16-byte little-endian store are always overwritten.
        while fast_src_idx + 16 < source.len() && dest_idx + 15 < dest.len() {
            let mut bits: u128 = 0;
            for i in 0..4 {
                let base = fast_src_idx + i * 4;
                let mut new_bits: u128 = self.fast_char_to_index[0][source[base] as usize] as u128;
                new_bits |= self.fast_char_to_index[1][source[base + 1] as usize] as u128;
                new_bits |= self.fast_char_to_index[2][source[base + 2] as usize] as u128;
                new_bits |= self.fast_char_to_index[3][source[base + 3] as usize] as u128;
                if (new_bits & INVALID_CHAR_TST as u128) != 0 {
                    return Err(Base64Error::InvalidCharacter);
                }
                bits |= new_bits << (24 * i);
            }
            dest[dest_idx..dest_idx + 16].copy_from_slice(&bits.to_le_bytes());
            fast_src_idx += 16;
            dest_idx += 12;
        }

        // 4 chars → 3 bytes (one extra zero byte, likewise always overwritten).
        while fast_src_idx + 4 < source.len() && dest_idx + 3 < dest.len() {
            let mut bits: u32 = self.fast_char_to_index[0][source[fast_src_idx] as usize];
            bits |= self.fast_char_to_index[1][source[fast_src_idx + 1] as usize];
            bits |= self.fast_char_to_index[2][source[fast_src_idx + 2] as usize];
            bits |= self.fast_char_to_index[3][source[fast_src_idx + 3] as usize];
            if (bits & INVALID_CHAR_TST) != 0 {
                return Err(Base64Error::InvalidCharacter);
            }
            dest[dest_idx..dest_idx + 4].copy_from_slice(&bits.to_le_bytes());
            fast_src_idx += 4;
            dest_idx += 3;
        }

        // Scalar leftover.
        for (offset, &c) in source[fast_src_idx..].iter().enumerate() {
            let src_idx = fast_src_idx + offset;
            let d = self.char_to_index[c as usize];
            if d == INVALID_CHAR {
                if self.pad_char != Some(c) {
                    return Err(Base64Error::InvalidCharacter);
                }
                leftover_idx = Some(src_idx);
                break;
            }
            acc = ((acc << 6) + d as u32) & 0x0fff;
            acc_len += 6;
            if acc_len >= 8 {
                acc_len -= 8;
                dest[dest_idx] = (acc >> acc_len) as u8;
                dest_idx += 1;
            }
        }

        // Upstream comments these out so Kitty Graphics' malformed padding is tolerated:
        // if acc_len > 4 || (acc & ((1 << acc_len) - 1)) != 0 { return Err(InvalidPadding); }

        let Some(leftover_idx) = leftover_idx else {
            return Ok(());
        };
        let leftover = &source[leftover_idx..];
        if let Some(pad) = self.pad_char {
            let padding_len = acc_len / 2;
            let mut padding_chars = 0usize;
            for &c in leftover {
                if c != pad {
                    // Upstream's exact branch: only the raw `0xff` sentinel byte is an invalid
                    // character here; any other non-pad byte is invalid padding.
                    return Err(if c == INVALID_CHAR {
                        Base64Error::InvalidCharacter
                    } else {
                        Base64Error::InvalidPadding
                    });
                }
                padding_chars += 1;
            }
            if padding_chars != padding_len as usize {
                return Err(Base64Error::InvalidPadding);
            }
        }
        Ok(())
    }

    /// Convenience: allocate a right-sized `Vec` and decode into it.
    pub(crate) fn decode_alloc(&self, source: &[u8]) -> Result<Vec<u8>, Base64Error> {
        let mut out = vec![0u8; self.calc_size_for_slice(source)?];
        self.decode(&mut out, source)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reference standard, no-pad base64 encoder for round-trip tests.
    fn encode_std_nopad(data: &[u8]) -> Vec<u8> {
        let a = STANDARD_ALPHABET;
        let mut out = Vec::new();
        for chunk in data.chunks(3) {
            let b0 = chunk[0];
            let b1 = chunk.get(1).copied().unwrap_or(0);
            let b2 = chunk.get(2).copied().unwrap_or(0);
            out.push(a[(b0 >> 2) as usize]);
            out.push(a[(((b0 & 0x3) << 4) | (b1 >> 4)) as usize]);
            if chunk.len() > 1 {
                out.push(a[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize]);
            }
            if chunk.len() > 2 {
                out.push(a[(b2 & 0x3f) as usize]);
            }
        }
        out
    }

    #[test]
    fn decode_known_vectors() {
        let d = scalar_decoder();
        assert_eq!(d.decode_alloc(b"TWFu").unwrap(), b"Man");
        assert_eq!(d.decode_alloc(b"TWE").unwrap(), b"Ma");
        assert_eq!(d.decode_alloc(b"TQ").unwrap(), b"M");
        assert_eq!(d.decode_alloc(b"SGVsbG8").unwrap(), b"Hello");
    }

    #[test]
    fn decode_long_input_exercises_fast_path() {
        let d = scalar_decoder();
        // 21 data bytes → 28 base64 chars (no pad), so the 16-char fast loop, the 4-char loop, and
        // the scalar leftover all run.
        let data: Vec<u8> = (0u8..21).collect();
        let encoded = encode_std_nopad(&data);
        assert!(encoded.len() >= 24);
        assert_eq!(d.decode_alloc(&encoded).unwrap(), data);
    }

    #[test]
    fn decode_roundtrips_many_lengths() {
        let d = scalar_decoder();
        for len in 0..40usize {
            let data: Vec<u8> = (0..len).map(|i| (i * 7 + 1) as u8).collect();
            let encoded = encode_std_nopad(&data);
            assert_eq!(d.decode_alloc(&encoded).unwrap(), data, "len={len}");
        }
    }

    #[test]
    fn decode_invalid_char_errors() {
        let d = scalar_decoder();
        // `*` is not in the standard alphabet.
        assert_eq!(d.decode_alloc(b"TW*u"), Err(Base64Error::InvalidCharacter));
    }

    #[test]
    fn calc_size_for_slice_matches_decoded_len() {
        let d = scalar_decoder();
        for s in [&b"TWFu"[..], b"TWE", b"TQ", b"SGVsbG8"] {
            let size = d.calc_size_for_slice(s).unwrap();
            assert_eq!(size, d.decode_alloc(s).unwrap().len());
        }
    }

    #[test]
    fn decode_tolerates_trailing_padding_like_kitty() {
        let pad = Base64Decoder::new(STANDARD_ALPHABET, Some(b'='));
        assert_eq!(pad.decode_alloc(b"TWE=").unwrap(), b"Ma");
        // The no-pad decoder treats `=` as an invalid character.
        let nopad = scalar_decoder();
        assert_eq!(
            nopad.decode_alloc(b"TWE="),
            Err(Base64Error::InvalidCharacter)
        );
    }

    #[test]
    fn pad_tail_error_branch() {
        let pad = Base64Decoder::new(STANDARD_ALPHABET, Some(b'='));
        // A non-pad, non-0xff byte after the pad → InvalidPadding.
        assert_eq!(pad.decode_alloc(b"TQ=A"), Err(Base64Error::InvalidPadding));
        // The raw 0xff sentinel byte after the pad → InvalidCharacter (upstream's odd branch).
        assert_eq!(
            pad.decode_alloc(b"TQ=\xff"),
            Err(Base64Error::InvalidCharacter)
        );
    }
}
