//! The `post` (PostScript) table.
//!
//! Faithful port of upstream `font/opentype/post.zig`. Like upstream, this
//! parses only the v1.0 32-byte header and does **not** parse the v2.0/v2.5
//! glyph-name arrays. Field names follow the spec (camelCase upstream → Rust
//! `snake_case`).
//!
//! Reference: <https://learn.microsoft.com/en-us/typography/opentype/spec/post>

use super::sfnt::{Fixed, OpenTypeError, Reader, Version16Dot16};

/// PostScript Table (v1.0 header, 32 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Post {
    /// Table version.
    pub version: Version16Dot16,
    /// Italic angle in counter-clockwise degrees from the vertical.
    pub italic_angle: Fixed,
    /// Suggested y-coordinate of the top of the underline (FWORD).
    pub underline_position: i16,
    /// Suggested underline thickness (FWORD).
    pub underline_thickness: i16,
    /// 0 if proportionally spaced, non-zero if monospaced.
    pub is_fixed_pitch: u32,
    /// Minimum memory usage when downloaded.
    pub min_mem_type42: u32,
    /// Maximum memory usage when downloaded.
    pub max_mem_type42: u32,
    /// Minimum memory usage when downloaded as a Type 1 font.
    pub min_mem_type1: u32,
    /// Maximum memory usage when downloaded as a Type 1 font.
    pub max_mem_type1: u32,
}

impl Post {
    /// Parse the v1.0 header from raw `post`-table bytes.
    pub(crate) fn from_bytes(data: &[u8]) -> Result<Post, OpenTypeError> {
        let mut r = Reader::new(data);
        Ok(Post {
            version: Version16Dot16::from_u32(r.read_u32()?),
            italic_angle: Fixed(r.read_i32()?),
            underline_position: r.read_i16()?,
            underline_thickness: r.read_i16()?,
            is_fixed_pitch: r.read_u32()?,
            min_mem_type42: r.read_u32()?,
            max_mem_type42: r.read_u32()?,
            min_mem_type1: r.read_u32()?,
            max_mem_type1: r.read_u32()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A hand-built 32-byte v1.0 `post` header, big-endian.
    #[rustfmt::skip]
    const POST_BYTES: [u8; 32] = [
        0x00, 0x02, 0x00, 0x00, // version = 2.0
        0x00, 0x00, 0x00, 0x00, // italic_angle = 0.0
        0xFF, 0x38,             // underline_position = -200
        0x00, 0x64,             // underline_thickness = 100
        0x00, 0x00, 0x00, 0x01, // is_fixed_pitch = 1
        0x00, 0x00, 0x00, 0x00, // min_mem_type42 = 0
        0x00, 0x00, 0x00, 0x00, // max_mem_type42 = 0
        0x00, 0x00, 0x00, 0x00, // min_mem_type1 = 0
        0x00, 0x00, 0x00, 0x00, // max_mem_type1 = 0
    ];

    #[test]
    fn parse_post() {
        let post = Post::from_bytes(&POST_BYTES).unwrap();
        assert_eq!(
            post,
            Post {
                version: Version16Dot16 { major: 2, minor: 0 },
                italic_angle: Fixed::from_f64(0.0),
                underline_position: -200,
                underline_thickness: 100,
                is_fixed_pitch: 1,
                min_mem_type42: 0,
                max_mem_type42: 0,
                min_mem_type1: 0,
                max_mem_type1: 0,
            }
        );
    }

    #[test]
    fn post_truncated() {
        assert_eq!(
            Post::from_bytes(&POST_BYTES[..31]),
            Err(OpenTypeError::EndOfStream)
        );
    }
}
