//! A CoreText-backed font face (`CTFont`).
//!
//! Faithful (macOS) port of the `CTFont` plumbing in upstream
//! `font/face/coretext.zig`. This slice provides face construction and raw
//! OpenType table access (`CTFontCopyTable`), the building block
//! `Face::get_metrics` will use to read `head`/`hhea`/`OS/2`/`post`. The full
//! metric assembly and glyph rasterization land in later experiments.

use std::ptr::NonNull;

use objc2_core_foundation::{CFRetained, CFString, CGSize};
use objc2_core_text::{CTFont, CTFontOrientation, CTFontTableOptions};

use crate::font::metrics::FaceMetrics;
use crate::font::opentype::{head::Head, hhea::Hhea, os2::Os2, post::Post};

/// A font face backed by a CoreText `CTFont`. `CFRetained` manages the
/// underlying CoreFoundation retain/release.
pub(crate) struct Face {
    font: CFRetained<CTFont>,
}

impl Face {
    /// Create a face for the named system font at the given point size. CoreText
    /// returns a fallback font if the exact name is unavailable, so this never
    /// fails.
    pub(crate) fn new(name: &str, size: f64) -> Face {
        let cf_name = CFString::from_str(name);
        // SAFETY: `cf_name` is a valid `CFString` that lives through the call,
        // and a null `matrix` pointer is documented as valid (no transform).
        let font = unsafe { CTFont::with_name(&cf_name, size, std::ptr::null()) };
        Face { font }
    }

    /// Copy the raw bytes of an OpenType table identified by its four-character
    /// tag (e.g. `b"head"`), or `None` if the font has no such table.
    pub(crate) fn copy_table(&self, tag: &[u8; 4]) -> Option<Vec<u8>> {
        // The table tag is a big-endian-packed four-character code.
        let table_tag = u32::from_be_bytes(*tag);
        // SAFETY: `self.font` is a live `CTFont`; the tag and (empty) options
        // are valid arguments to `CTFontCopyTable`.
        let data = unsafe { self.font.table(table_tag, CTFontTableOptions(0)) }?;
        Some(data.to_vec())
    }

    /// The point size the face was created at (pixels per em).
    pub(crate) fn size(&self) -> f64 {
        // SAFETY: `self.font` is a live `CTFont`.
        unsafe { self.font.size() }
    }

    /// The font's units per em (the head-table fallback).
    pub(crate) fn units_per_em(&self) -> u32 {
        // SAFETY: `self.font` is a live `CTFont`.
        unsafe { self.font.units_per_em() as u32 }
    }

    /// CoreText ascent in pixels (the hhea-absent fallback).
    pub(crate) fn ascent(&self) -> f64 {
        // SAFETY: `self.font` is a live `CTFont`.
        unsafe { self.font.ascent() }
    }

    /// CoreText descent in pixels, as a **positive** magnitude (CoreText's
    /// convention); the metric assembly negates it. The hhea-absent fallback.
    pub(crate) fn descent(&self) -> f64 {
        // SAFETY: `self.font` is a live `CTFont`.
        unsafe { self.font.descent() }
    }

    /// CoreText leading (line gap) in pixels (the hhea-absent fallback).
    pub(crate) fn leading(&self) -> f64 {
        // SAFETY: `self.font` is a live `CTFont`.
        unsafe { self.font.leading() }
    }

    /// CoreText cap height in pixels (the OS/2 `sCapHeight`-absent fallback).
    pub(crate) fn cap_height(&self) -> f64 {
        // SAFETY: `self.font` is a live `CTFont`.
        unsafe { self.font.cap_height() }
    }

    /// CoreText x-height in pixels (the OS/2 `sxHeight`-absent fallback).
    pub(crate) fn x_height(&self) -> f64 {
        // SAFETY: `self.font` is a live `CTFont`.
        unsafe { self.font.x_height() }
    }

    /// Map each input UTF-16 code unit to its glyph ID (`0` = no glyph).
    pub(crate) fn glyphs_for_characters(&self, chars: &[u16]) -> Vec<u16> {
        if chars.is_empty() {
            return Vec::new();
        }
        let mut glyphs = vec![0u16; chars.len()];
        let chars_ptr = NonNull::new(chars.as_ptr() as *mut u16).unwrap();
        let glyphs_ptr = NonNull::new(glyphs.as_mut_ptr()).unwrap();
        // SAFETY: `chars` and `glyphs` are non-empty slices of length `count`;
        // CoreText reads `characters` (const) and writes one glyph per char.
        unsafe {
            self.font
                .glyphs_for_characters(chars_ptr, glyphs_ptr, chars.len() as isize);
        }
        glyphs
    }

    /// The horizontal advance width of each glyph, in pixels.
    pub(crate) fn advances_for_glyphs(&self, glyphs: &[u16]) -> Vec<f64> {
        if glyphs.is_empty() {
            return Vec::new();
        }
        let mut advances = vec![CGSize::new(0.0, 0.0); glyphs.len()];
        let glyphs_ptr = NonNull::new(glyphs.as_ptr() as *mut u16).unwrap();
        // SAFETY: `glyphs` is a non-empty slice of length `count`; `advances` is
        // a buffer of the same length that CoreText fills.
        unsafe {
            self.font.advances_for_glyphs(
                CTFontOrientation::Horizontal,
                glyphs_ptr,
                advances.as_mut_ptr(),
                glyphs.len() as isize,
            );
        }
        advances.iter().map(|s| s.width).collect()
    }

    /// The overall bounding rectangle for the glyphs, as `(width, height)` in
    /// pixels.
    pub(crate) fn bounding_rect_for_glyphs(&self, glyphs: &[u16]) -> (f64, f64) {
        if glyphs.is_empty() {
            return (0.0, 0.0);
        }
        let glyphs_ptr = NonNull::new(glyphs.as_ptr() as *mut u16).unwrap();
        // SAFETY: `glyphs` is a non-empty slice of length `count`; a null
        // `bounding_rects` pointer requests only the overall rect (the return).
        let rect = unsafe {
            self.font.bounding_rects_for_glyphs(
                CTFontOrientation::Horizontal,
                glyphs_ptr,
                std::ptr::null_mut(),
                glyphs.len() as isize,
            )
        };
        (rect.size.width, rect.size.height)
    }

    /// Assemble the face's metrics from its OpenType tables and CoreText
    /// measurements. Faithful port of upstream `getMetrics`.
    pub(crate) fn get_metrics(&self) -> FaceMetrics {
        // Read the metric tables. `head` falls back to the byte-identical `bhed`
        // tag used by bitmap-only fonts.
        let head = self
            .copy_table(b"head")
            .or_else(|| self.copy_table(b"bhed"))
            .and_then(|b| Head::from_bytes(&b).ok());
        let post = self
            .copy_table(b"post")
            .and_then(|b| Post::from_bytes(&b).ok());
        let os2 = self
            .copy_table(b"OS/2")
            .and_then(|b| Os2::from_bytes(&b).ok());
        let hhea = self
            .copy_table(b"hhea")
            .and_then(|b| Hhea::from_bytes(&b).ok());

        let units_per_em = head
            .map(|h| h.units_per_em as f64)
            .unwrap_or_else(|| self.units_per_em() as f64);
        let px_per_em = self.size();
        let px_per_unit = px_per_em / units_per_em;

        // Vertical metrics fallback chain.
        let (ascent, descent, line_gap) = match hhea {
            // No hhea: use CoreText's pixel metrics directly (CoreText returns
            // descent as a positive magnitude, so negate it).
            None => (self.ascent(), -self.descent(), self.leading()),
            Some(hhea) => {
                let ha = hhea.ascender as f64;
                let hd = hhea.descender as f64;
                let hg = hhea.line_gap as f64;
                match os2 {
                    None => (ha * px_per_unit, hd * px_per_unit, hg * px_per_unit),
                    Some(os2) => {
                        let oa = os2.s_typo_ascender as f64;
                        let od = os2.s_typo_descender as f64;
                        let og = os2.s_typo_line_gap as f64;
                        if os2.fs_selection.use_typo_metrics() {
                            (oa * px_per_unit, od * px_per_unit, og * px_per_unit)
                        } else if hhea.ascender != 0 || hhea.descender != 0 {
                            (ha * px_per_unit, hd * px_per_unit, hg * px_per_unit)
                        } else if os2.s_typo_ascender != 0 || os2.s_typo_descender != 0 {
                            (oa * px_per_unit, od * px_per_unit, og * px_per_unit)
                        } else {
                            // usWinDescent is positive-down, so negate it.
                            (
                                os2.us_win_ascent as f64 * px_per_unit,
                                -(os2.us_win_descent as f64) * px_per_unit,
                                0.0,
                            )
                        }
                    }
                }
            }
        };

        // Underline from `post` (degenerate-zero thickness/position -> None).
        let (underline_position, underline_thickness) = match post {
            None => (None, None),
            Some(post) => {
                let broken = post.underline_thickness == 0;
                let pos = if broken && post.underline_position == 0 {
                    None
                } else {
                    Some(post.underline_position as f64 * px_per_unit)
                };
                let thick = if broken {
                    None
                } else {
                    Some(post.underline_thickness as f64 * px_per_unit)
                };
                (pos, thick)
            }
        };

        // Strikethrough from `OS/2` (same degenerate-zero logic).
        let (strikethrough_position, strikethrough_thickness) = match os2 {
            None => (None, None),
            Some(os2) => {
                let broken = os2.y_strikeout_size == 0;
                let pos = if broken && os2.y_strikeout_position == 0 {
                    None
                } else {
                    Some(os2.y_strikeout_position as f64 * px_per_unit)
                };
                let thick = if broken {
                    None
                } else {
                    Some(os2.y_strikeout_size as f64 * px_per_unit)
                };
                (pos, thick)
            }
        };

        // Cap/ex height: OS/2 values when present, else CoreText.
        let (cap_height, ex_height) = match os2 {
            None => (Some(self.cap_height()), Some(self.x_height())),
            Some(os2) => (
                Some(
                    os2.s_cap_height
                        .map(|v| v as f64 * px_per_unit)
                        .unwrap_or_else(|| self.cap_height()),
                ),
                Some(
                    os2.sx_height
                        .map(|v| v as f64 * px_per_unit)
                        .unwrap_or_else(|| self.x_height()),
                ),
            ),
        };

        // Cell width = widest printable-ASCII advance; ASCII height = the
        // overall bounding-box height of those glyphs.
        let ascii: Vec<u16> = (0x20u16..0x7F).collect();
        let ascii_glyphs = self.glyphs_for_characters(&ascii);
        let cell_width = self
            .advances_for_glyphs(&ascii_glyphs)
            .into_iter()
            .fold(0.0_f64, f64::max);
        let ascii_height = self.bounding_rect_for_glyphs(&ascii_glyphs).1;

        // Ideographic width: the advance of `水`, discarded if absent or if its
        // bounds are wider than its advance (a butchered patched-CJK font).
        let ic_width = {
            let glyph = self.glyphs_for_characters(&[0x6C34])[0];
            if glyph == 0 {
                None
            } else {
                let advance = self.advances_for_glyphs(&[glyph])[0];
                let bounds_w = self.bounding_rect_for_glyphs(&[glyph]).0;
                if bounds_w > advance {
                    None
                } else {
                    Some(advance)
                }
            }
        };

        FaceMetrics {
            px_per_em,
            cell_width,
            ascent,
            descent,
            line_gap,
            underline_position,
            underline_thickness,
            strikethrough_position,
            strikethrough_thickness,
            cap_height,
            ex_height,
            ascii_height: Some(ascii_height),
            ic_width,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::opentype::head::Head;

    #[test]
    fn face_copies_and_parses_head() {
        let face = Face::new("Menlo", 12.0);
        let bytes = face
            .copy_table(b"head")
            .expect("the font should have a head table");
        let head = Head::from_bytes(&bytes).expect("head table should parse");

        // `magic_number` is `0x5F0F3CF5` in every valid `head` table, regardless
        // of which font CoreText resolved — a version-independent check that the
        // FFI round-trip and parser are correct.
        assert_eq!(head.magic_number, 0x5F0F_3CF5);
        // units-per-em must be in the spec's valid range.
        assert!((16..=16384).contains(&head.units_per_em));
    }

    #[test]
    fn missing_table_is_none() {
        let face = Face::new("Menlo", 12.0);
        // No font has a `ZZZZ` table.
        assert!(face.copy_table(b"ZZZZ").is_none());
    }

    #[test]
    fn scalar_metrics_are_plausible() {
        let face = Face::new("Menlo", 12.0);
        assert_eq!(face.size(), 12.0);
        assert!((16..=16384).contains(&face.units_per_em()));
        assert!(face.ascent() > 0.0);
        assert!(face.descent() > 0.0); // CoreText returns descent positive
        assert!(face.leading() >= 0.0);
        assert!(face.cap_height() > 0.0);
        assert!(face.x_height() > 0.0);
        // Capitals are taller than the x-height.
        assert!(face.cap_height() > face.x_height());
    }

    #[test]
    fn glyph_measurement() {
        let face = Face::new("Menlo", 12.0);
        let glyphs = face.glyphs_for_characters(&[b'M' as u16, b'i' as u16]);
        assert_eq!(glyphs.len(), 2);
        assert!(glyphs.iter().all(|&g| g != 0)); // both chars have glyphs

        let advances = face.advances_for_glyphs(&glyphs);
        assert_eq!(advances.len(), 2);
        assert!(advances.iter().all(|&w| w > 0.0));
        // Menlo is monospaced, so 'M' and 'i' advance identically.
        assert_eq!(advances[0], advances[1]);

        let (w, h) = face.bounding_rect_for_glyphs(&glyphs);
        assert!(w > 0.0);
        assert!(h > 0.0);
    }

    #[test]
    fn empty_glyph_inputs() {
        let face = Face::new("Menlo", 12.0);
        assert!(face.glyphs_for_characters(&[]).is_empty());
        assert!(face.advances_for_glyphs(&[]).is_empty());
        assert_eq!(face.bounding_rect_for_glyphs(&[]), (0.0, 0.0));
    }

    #[test]
    fn get_metrics_is_sane() {
        let fm = Face::new("Menlo", 14.0).get_metrics();
        assert_eq!(fm.px_per_em, 14.0);
        assert!(fm.cell_width > 0.0);
        assert!(fm.ascent > 0.0);
        assert!(fm.descent < 0.0); // below the baseline
        assert!(fm.line_gap >= 0.0);
        assert!(fm.cap_height.unwrap() > 0.0);
        assert!(fm.ex_height.unwrap() > 0.0);
        assert!(fm.cap_height.unwrap() > fm.ex_height.unwrap());
        assert!(fm.ascii_height.unwrap() > 0.0);
    }

    #[test]
    fn get_metrics_feeds_calc() {
        use crate::font::metrics::Metrics;

        let m = Metrics::calc(Face::new("Menlo", 14.0).get_metrics());
        assert!(m.cell_width > 0);
        assert!(m.cell_height > 0);
        assert!(m.cell_baseline <= m.cell_height);
        assert!(m.underline_thickness >= 1);
    }
}
