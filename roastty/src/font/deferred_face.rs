//! Deferred CoreText font faces.
//!
//! Faithful macOS slice of upstream `font/DeferredFace.zig`: a deferred face
//! owns the lightweight CoreText descriptor and variation list needed to create a
//! full [`Face`] only when the caller needs one.

use std::ffi::c_void;

use objc2_core_foundation::{kCFNull, CFMutableDictionary, CFRetained, CFString, CFType};
use objc2_core_text::{kCTFontCharacterSetAttribute, CTFont, CTFontDescriptor};

use crate::font::discovery::Variation;
use crate::font::face::coretext::Face;
use crate::font::Presentation;

/// A CoreText-backed deferred font face.
pub(crate) struct DeferredFace {
    descriptor: CFRetained<CTFontDescriptor>,
    variations: Vec<Variation>,
}

impl DeferredFace {
    /// Create a deferred face from a discovered descriptor.
    ///
    /// The discovery character-set attribute is removed before storing the
    /// descriptor, matching upstream `DiscoverIterator.next`: the charset was a
    /// search filter, not a render constraint.
    pub(crate) fn from_descriptor(
        descriptor: CFRetained<CTFontDescriptor>,
        variations: Vec<Variation>,
    ) -> DeferredFace {
        let attrs = CFMutableDictionary::<CFString, CFType>::empty();
        // SAFETY: `kCTFontCharacterSetAttribute` is a static key; `kCFNull` is a
        // live singleton; the mutable dictionary retains both.
        unsafe {
            let null = kCFNull.expect("kCFNull is available");
            CFMutableDictionary::set_value(
                Some(attrs.as_opaque()),
                (kCTFontCharacterSetAttribute as *const CFString).cast::<c_void>(),
                (null as *const objc2_core_foundation::CFNull).cast::<c_void>(),
            );
        }
        // SAFETY: `descriptor` is live; `attrs` is a valid attributes dictionary.
        let descriptor = unsafe { descriptor.copy_with_attributes(attrs.as_opaque()) };

        DeferredFace {
            descriptor,
            variations,
        }
    }

    /// Load this deferred face into a renderable CoreText [`Face`].
    pub(crate) fn load(&self) -> Face {
        // Create the font at size 12; collection/load integration can resize it
        // later to match the active font grid.
        // SAFETY: `descriptor` is live; a null matrix is valid.
        let font =
            unsafe { CTFont::with_font_descriptor(&self.descriptor, 12.0, std::ptr::null()) };
        let mut face = Face::from_ct_font(font);
        face.set_variations(&self.variations);
        face
    }

    /// Whether this deferred face can render `cp` in the requested presentation.
    pub(crate) fn has_codepoint(&self, cp: u32, presentation: Option<Presentation>) -> bool {
        let face = self.load();
        let Some(glyph) = face.glyph_index(cp) else {
            return false;
        };
        match presentation {
            Some(Presentation::Text) => !face.is_color_glyph(glyph),
            Some(Presentation::Emoji) => face.is_color_glyph(glyph),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::discovery::Descriptor;

    fn deferred_family(family: &str) -> DeferredFace {
        let req = Descriptor {
            family: Some(family.into()),
            ..Default::default()
        };
        let desc = req
            .discover_descriptors()
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("{family} should have at least one descriptor"));
        DeferredFace::from_descriptor(desc, Vec::new())
    }

    #[test]
    fn deferred_face_load_renders_menlo() {
        let face = deferred_family("Menlo").load();
        assert!(face.glyph_index('M' as u32).is_some());
    }

    #[test]
    fn deferred_face_has_codepoint_text_and_missing() {
        let face = deferred_family("Menlo");
        assert!(face.has_codepoint('M' as u32, None));
        assert!(!face.has_codepoint(0xFDD0, None));
    }

    #[test]
    fn deferred_face_filters_text_presentation() {
        let face = deferred_family("Menlo");
        assert!(face.has_codepoint('M' as u32, Some(Presentation::Text)));
        assert!(!face.has_codepoint('M' as u32, Some(Presentation::Emoji)));
    }

    #[test]
    fn deferred_face_filters_emoji_presentation() {
        let face = deferred_family("Apple Color Emoji");
        assert!(face.has_codepoint(0x1F600, Some(Presentation::Emoji)));
        assert!(!face.has_codepoint(0x1F600, Some(Presentation::Text)));
    }

    #[test]
    fn deferred_face_load_applies_variations() {
        let req = Descriptor {
            family: Some("Menlo".into()),
            ..Default::default()
        };
        let desc = req
            .discover_descriptors()
            .into_iter()
            .next()
            .expect("Menlo should have at least one descriptor");
        let face = DeferredFace::from_descriptor(
            desc,
            vec![Variation {
                id: Variation::id_from_tag(b"wght"),
                value: 700.0,
            }],
        )
        .load();
        assert!(face.glyph_index('M' as u32).is_some());
    }
}
