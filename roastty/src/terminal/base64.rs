//! Public base64 decode entry point (port of upstream `simd/base64`).
//!
//! Upstream dispatches to a C++ SIMD decoder or the scalar `base64_scalar.scalar_decoder`; Roastty
//! uses Rust's `base64-simd` for the standard no-pad fast path and falls back to the scalar port for
//! the Kitty/OSC leniency cases that must preserve existing behavior.

use super::base64_scalar::scalar_decoder;
use base64_simd::AsOut;

/// Invalid base64 input (upstream `error{Base64Invalid}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Base64Invalid;

/// The maximum decoded length for `input` (upstream `maxLen` / `maxLenScalar`). Returns 0 on a
/// sizing error (upstream logs and returns 0).
pub(crate) fn max_len(input: &[u8]) -> usize {
    scalar_decoder()
        .calc_size_for_slice(scalar_input(input))
        .unwrap_or(0)
}

/// Decode `input` into `output` (which must be at least `max_len(input)` bytes), returning the
/// decoded prefix (upstream `decode` / `decodeScalar`).
pub(crate) fn decode<'o>(input: &[u8], output: &'o mut [u8]) -> Result<&'o [u8], Base64Invalid> {
    let stripped = scalar_input(input);
    let size = max_len(stripped);
    if size == 0 {
        return Ok(&[]);
    }
    assert!(output.len() >= size);
    if let Ok(decoded_len) = decode_simd(stripped, output) {
        return Ok(&output[..decoded_len]);
    }
    scalar_decoder()
        .decode(output, stripped)
        .map_err(|_| Base64Invalid)?;
    Ok(&output[..size])
}

fn decode_simd(input: &[u8], output: &mut [u8]) -> Result<usize, Base64Invalid> {
    base64_simd::STANDARD_NO_PAD
        .decode(input, output.as_out())
        .map(|decoded| decoded.len())
        .map_err(|_| Base64Invalid)
}

/// Trim trailing `=` padding so the no-pad scalar decoder accepts padded input and matches the SIMD
/// path's output (upstream `scalarInput`). Counts the padding safely, so empty / all-`=` input
/// yields an empty slice instead of upstream's index underflow.
fn scalar_input(input: &[u8]) -> &[u8] {
    let trailing = input.iter().rev().take_while(|&&b| b == b'=').count();
    &input[..input.len() - trailing]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_vec(input: &[u8]) -> Result<Vec<u8>, Base64Invalid> {
        let mut buf = vec![0u8; max_len(input)];
        decode(input, &mut buf).map(|s| s.to_vec())
    }

    fn scalar_decode_vec(input: &[u8]) -> Result<Vec<u8>, Base64Invalid> {
        let stripped = scalar_input(input);
        let size = max_len(stripped);
        let mut out = vec![0u8; size];
        scalar_decoder()
            .decode(&mut out, stripped)
            .map_err(|_| Base64Invalid)?;
        Ok(out)
    }

    #[test]
    fn max_len_of_padded_input() {
        assert_eq!(max_len(b"aGVsbG8gd29ybGQ="), 11);
    }

    #[test]
    fn decode_padded_input() {
        assert_eq!(decode_vec(b"aGVsbG8gd29ybGQ=").unwrap(), b"hello world");
    }

    #[test]
    fn decode_unpadded_input() {
        assert_eq!(decode_vec(b"TWFu").unwrap(), b"Man");
    }

    #[test]
    fn decode_strips_multiple_padding() {
        assert_eq!(decode_vec(b"TWE==").unwrap(), b"Ma");
    }

    #[test]
    fn decode_invalid_input_errors() {
        // `*` is not in the standard alphabet; sizing succeeds, decode fails.
        assert_eq!(decode_vec(b"TW*u"), Err(Base64Invalid));
    }

    #[test]
    fn decode_matches_scalar_on_representative_inputs() {
        for input in [
            b"aGVsbG8gd29ybGQ=".as_slice(),
            b"TWFu",
            b"",
            b"====",
            b"TWE==",
            b"VGhlIHF1aWNrIGJyb3duIGZveA",
        ] {
            assert_eq!(decode_vec(input), scalar_decode_vec(input));
        }
    }

    #[test]
    fn decode_long_payload_matches_scalar() {
        let data: Vec<u8> = (0..4096).map(|i| (i * 31 + 7) as u8).collect();
        let encoded = encode_std_nopad(&data);
        assert_eq!(
            decode_vec(&encoded).unwrap(),
            scalar_decode_vec(&encoded).unwrap()
        );
        assert_eq!(decode_vec(&encoded).unwrap(), data);
    }

    #[test]
    #[ignore = "release-mode perf probe"]
    fn simd_fast_path_perf_base64() {
        let data: Vec<u8> = (0..(1024 * 256)).map(|i| (i * 31 + 7) as u8).collect();
        let encoded = encode_std_nopad(&data);
        let iterations = 80;

        let scalar = time_iterations(iterations, || {
            assert_eq!(scalar_decode_vec(&encoded).unwrap(), data);
        });
        let fast = time_iterations(iterations, || {
            assert_eq!(decode_vec(&encoded).unwrap(), data);
        });
        assert_speedup("base64_decode", scalar, fast);
    }

    fn encode_std_nopad(data: &[u8]) -> Vec<u8> {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = Vec::new();
        for chunk in data.chunks(3) {
            let b0 = chunk[0];
            let b1 = chunk.get(1).copied().unwrap_or(0);
            let b2 = chunk.get(2).copied().unwrap_or(0);
            out.push(ALPHABET[(b0 >> 2) as usize]);
            out.push(ALPHABET[(((b0 & 0x3) << 4) | (b1 >> 4)) as usize]);
            if chunk.len() > 1 {
                out.push(ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize]);
            }
            if chunk.len() > 2 {
                out.push(ALPHABET[(b2 & 0x3f) as usize]);
            }
        }
        out
    }

    fn time_iterations(iterations: usize, mut f: impl FnMut()) -> std::time::Duration {
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            f();
        }
        start.elapsed()
    }

    fn assert_speedup(label: &str, scalar: std::time::Duration, fast: std::time::Duration) {
        let ratio = scalar.as_secs_f64() / fast.as_secs_f64();
        eprintln!("{label}: scalar={scalar:?} fast={fast:?} ratio={ratio:.2}x");
        assert!(
            ratio >= 1.05,
            "{label} fast path ratio {ratio:.2}x below 1.05x"
        );
    }

    #[test]
    fn max_len_of_empty_is_zero() {
        assert_eq!(max_len(b""), 0);
        assert_eq!(max_len(b"===="), 0);
    }

    #[test]
    fn decode_empty_is_empty() {
        let mut buf = [0u8; 4];
        assert_eq!(decode(b"", &mut buf).unwrap(), b"");
        assert_eq!(decode(b"====", &mut buf).unwrap(), b"");
    }
}
