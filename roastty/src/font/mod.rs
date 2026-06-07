#![allow(dead_code)]
// The font subsystem is consumed by later font and renderer slices.

//! Font subsystem.
//!
//! Faithful macOS/CoreText slice of upstream `font/`: metrics, atlases, glyphs,
//! CoreText faces, discovery, shaping, collections, shared-grid rendering,
//! sprites, and OpenType table parsers.

pub(crate) mod atlas;
pub(crate) mod backend;
pub(crate) mod codepoint_map;
pub(crate) mod codepoint_resolver;
pub(crate) mod collection;
pub(crate) mod deferred_face;
pub(crate) mod discovery;
pub(crate) mod embedded;
pub(crate) mod emoji_presentation;
pub(crate) mod face;
pub(crate) mod glyph;
pub(crate) mod library;
pub(crate) mod metrics;
pub(crate) mod opentype;
pub(crate) mod run;
pub(crate) mod shape;
pub(crate) mod shaper_cache;
pub(crate) mod shared_grid;
pub(crate) mod shared_grid_set;
pub(crate) mod sprite;

/// The style (weight/slant) of a font face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Style {
    Regular = 0,
    Bold = 1,
    Italic = 2,
    BoldItalic = 3,
}

/// The presentation for an emoji.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Presentation {
    /// Text presentation (U+FE0E).
    Text = 0,
    /// Emoji presentation (U+FE0F).
    Emoji = 1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_discriminants() {
        assert_eq!(Style::Regular as u8, 0);
        assert_eq!(Style::Bold as u8, 1);
        assert_eq!(Style::Italic as u8, 2);
        assert_eq!(Style::BoldItalic as u8, 3);
    }

    #[test]
    fn presentation_discriminants() {
        assert_eq!(Presentation::Text as u8, 0);
        assert_eq!(Presentation::Emoji as u8, 1);
    }
}
