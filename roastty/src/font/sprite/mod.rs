//! Sprite font subsystem.
//!
//! Faithful port of upstream `font/sprite/`, which procedurally draws
//! box-drawing, block, Powerline, Braille, and legacy-computing glyphs directly
//! into the atlas. The `Canvas` (a 2D rasterization surface), the `raster` 2D
//! pipeline, and the `draw` glyph tables render the glyphs; [`render_codepoint`]
//! is the sprite `Face.renderGlyph` entry point that writes one to the atlas.

pub(crate) mod canvas;
pub(crate) mod draw;
pub(crate) mod raster;

use crate::font::atlas::{Atlas, AtlasError};
use crate::font::glyph::Glyph;
use crate::font::metrics::Metrics;
use canvas::Canvas;

/// Render the sprite glyph for `cp` into `atlas`, returning its [`Glyph`], or
/// `None` if `cp` is not a drawable sprite codepoint. Faithful port of the
/// codepoint path of upstream sprite `Face.renderGlyph`: size a padded `Canvas`
/// (a quarter cell on each side), draw the codepoint, write the trimmed result
/// to the atlas, and build the `Glyph` from the atlas region and the trim
/// margins. (The wide-glyph `cell_width` factoring and the sprite-kind special
/// glyphs are deferred.)
pub(crate) fn render_codepoint(
    cp: u32,
    metrics: &Metrics,
    atlas: &mut Atlas,
) -> Result<Option<Glyph>, AtlasError> {
    let width = metrics.cell_width;
    let height = metrics.cell_height;
    let padding_x = width / 4;
    let padding_y = height / 4;

    let mut c = Canvas::new(width, height, padding_x, padding_y);
    if !draw::draw_codepoint(cp, metrics, &mut c) {
        return Ok(None);
    }

    let region = c.write_atlas(atlas)?;
    Ok(Some(Glyph {
        width: region.width,
        height: region.height,
        offset_x: c.clip_left() as i32 - padding_x as i32,
        offset_y: region.height.saturating_add(c.clip_bottom()) as i32 - padding_y as i32,
        atlas_x: region.x,
        atlas_y: region.y,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::atlas::Format;
    use crate::font::sprite::draw::draw_codepoint;

    fn fixture_metrics() -> Metrics {
        Metrics {
            cell_width: 9,
            cell_height: 18,
            cell_baseline: 4,
            underline_position: 15,
            underline_thickness: 1,
            strikethrough_position: 9,
            strikethrough_thickness: 1,
            overline_position: 0,
            overline_thickness: 1,
            box_thickness: 2,
            cursor_thickness: 1,
            cursor_height: 18,
            icon_height: 16.0,
            icon_height_single: 16.0,
            face_width: 9.0,
            face_height: 18.0,
            face_y: 0.0,
        }
    }

    #[test]
    fn render_codepoint_box_line() {
        let m = fixture_metrics();
        let mut atlas = Atlas::new(64, Format::Grayscale);
        let glyph = render_codepoint(0x2500, &m, &mut atlas)
            .unwrap()
            .expect("box line is a sprite");
        // The horizontal line trims to a thin, non-empty region.
        assert!(glyph.width > 0, "non-empty width");
        assert!(glyph.height > 0, "non-empty height");
        // The atlas got the glyph's pixels.
        assert!(
            atlas.data().iter().any(|&b| b != 0),
            "atlas has the glyph ink"
        );
    }

    #[test]
    fn render_codepoint_offsets() {
        let m = fixture_metrics();
        let width = m.cell_width;
        let height = m.cell_height;
        let padding_x = width / 4;
        let padding_y = height / 4;

        // Render directly to obtain the trim margins independently.
        let mut direct = Canvas::new(width, height, padding_x, padding_y);
        assert!(draw_codepoint(0x2500, &m, &mut direct));
        let mut atlas_a = Atlas::new(64, Format::Grayscale);
        let region = direct.write_atlas(&mut atlas_a).unwrap();
        let expect_offset_x = direct.clip_left() as i32 - padding_x as i32;
        let expect_offset_y =
            region.height.saturating_add(direct.clip_bottom()) as i32 - padding_y as i32;

        // render_codepoint computes the same bearings.
        let mut atlas_b = Atlas::new(64, Format::Grayscale);
        let glyph = render_codepoint(0x2500, &m, &mut atlas_b).unwrap().unwrap();
        assert_eq!(glyph.offset_x, expect_offset_x, "left bearing");
        assert_eq!(glyph.offset_y, expect_offset_y, "top bearing");
        assert_eq!(glyph.width, region.width);
        assert_eq!(glyph.height, region.height);
    }

    #[test]
    fn render_codepoint_blank() {
        // The blank Braille pattern is a covered glyph that draws no ink: it
        // renders to Some(_) (not None) without panicking on the blank canvas.
        let m = fixture_metrics();
        let mut atlas = Atlas::new(64, Format::Grayscale);
        let glyph = render_codepoint(0x2800, &m, &mut atlas)
            .unwrap()
            .expect("blank braille is still a covered sprite");
        // A fully-trimmed blank glyph has no atlas footprint.
        assert_eq!(glyph.width, 0);
        assert_eq!(glyph.height, 0);
    }

    #[test]
    fn render_codepoint_none() {
        let m = fixture_metrics();
        let mut atlas = Atlas::new(64, Format::Grayscale);
        // A non-sprite returns None.
        assert!(render_codepoint('M' as u32, &m, &mut atlas)
            .unwrap()
            .is_none());
        // The intervening None render reserved nothing: the next real glyph's
        // placement matches a fresh atlas.
        let after = render_codepoint(0x2500, &m, &mut atlas).unwrap().unwrap();
        let mut fresh = Atlas::new(64, Format::Grayscale);
        let baseline = render_codepoint(0x2500, &m, &mut fresh).unwrap().unwrap();
        assert_eq!(after.atlas_x, baseline.atlas_x, "no region wasted on None");
        assert_eq!(after.atlas_y, baseline.atlas_y);
    }
}
