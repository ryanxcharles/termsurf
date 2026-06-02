//! Font metrics: recommended cell dimensions and decoration positions.
//!
//! Faithful port of the `Metrics` value type from upstream `font/Metrics.zig`.
//! The `FaceMetrics` input, the `Minimums` table, the `calc` derivation, and
//! constraint application are ported in later slices.

/// Recommended cell dimensions and decoration positions/thicknesses for a
/// monospace grid using a given font.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Metrics {
    /// Recommended cell width for a monospace grid using this font.
    pub cell_width: u32,
    /// Recommended cell height for a monospace grid using this font.
    pub cell_height: u32,

    /// Distance in pixels from the bottom of the cell to the text baseline.
    pub cell_baseline: u32,

    /// Distance in pixels from the top of the cell to the top of the underline.
    pub underline_position: u32,
    /// Thickness in pixels of the underline.
    pub underline_thickness: u32,

    /// Distance in pixels from the top of the cell to the top of the
    /// strikethrough.
    pub strikethrough_position: u32,
    /// Thickness in pixels of the strikethrough.
    pub strikethrough_thickness: u32,

    /// Distance in pixels from the top of the cell to the top of the overline.
    /// Can be negative to adjust the position above the top of the cell.
    pub overline_position: i32,
    /// Thickness in pixels of the overline.
    pub overline_thickness: u32,

    /// Thickness in pixels of box drawing characters.
    pub box_thickness: u32,

    /// The thickness in pixels of the cursor sprite. This is not determined by
    /// fonts but by user configuration; the deferred `calc`/config path applies
    /// the upstream default of `1`.
    pub cursor_thickness: u32,

    /// The height in pixels of the cursor sprite.
    pub cursor_height: u32,

    /// The constraint height for nerd fonts icons.
    pub icon_height: f64,

    /// The constraint height for nerd fonts icons limited to a single cell
    /// width.
    pub icon_height_single: f64,

    /// The unrounded face width, used in scaling calculations.
    pub face_width: f64,

    /// The unrounded face height, used in scaling calculations.
    pub face_height: f64,

    /// The offset from the bottom of the cell to the bottom of the face's
    /// bounding box, based on the rounded and potentially adjusted cell height.
    pub face_y: f64,
}

/// The raw metrics read from a font face — the input to `calc`, which derives a
/// `Metrics`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FaceMetrics {
    /// Pixels per em. Dividing the other values by this yields sizes in ems, to
    /// allow comparing metrics from faces of different sizes.
    pub px_per_em: f64,

    /// The minimum cell width that can contain any glyph in the ASCII range,
    /// measured over all printable ASCII glyphs.
    pub cell_width: f64,

    /// Typographic ascent: the maximum vertical position of the highest
    /// ascender, relative to the baseline (px, +Y up).
    pub ascent: f64,

    /// Typographic descent: the minimum vertical position of the lowest
    /// descender, relative to the baseline (px, +Y up). Typically negative.
    pub descent: f64,

    /// Typographic line gap ("leading"): additional space between lines beyond
    /// the ascent/descent (positive px).
    pub line_gap: f64,

    /// The TOP of the underline stroke, relative to the baseline (px, +Y up).
    pub underline_position: Option<f64>,

    /// The thickness of the underline stroke (px).
    pub underline_thickness: Option<f64>,

    /// The TOP of the strikethrough stroke, relative to the baseline (px, +Y up).
    pub strikethrough_position: Option<f64>,

    /// The thickness of the strikethrough stroke (px).
    pub strikethrough_thickness: Option<f64>,

    /// The height of capital letters, from a provided cap-height metric or the
    /// capital "H" glyph.
    pub cap_height: Option<f64>,

    /// The height of lowercase letters, from a provided ex-height metric or the
    /// lowercase "x" glyph.
    pub ex_height: Option<f64>,

    /// The measured bounding-box height of all printable ASCII characters
    /// (positive px); can differ from ascent − descent.
    pub ascii_height: Option<f64>,

    /// The width of "水" (CJK water ideograph, U+6C34) if present, used to
    /// normalize CJK font widths mixed with latin fonts.
    pub ic_width: Option<f64>,
}

impl FaceMetrics {
    /// The line height: `ascent - descent + line_gap`.
    pub(crate) fn line_height(&self) -> f64 {
        self.ascent - self.descent + self.line_gap
    }

    /// The effective cap height: the stored `cap_height` when present and
    /// positive, otherwise an estimate of `0.75 * ascent`.
    pub(crate) fn effective_cap_height(&self) -> f64 {
        if let Some(value) = self.cap_height {
            if value > 0.0 {
                return value;
            }
        }
        0.75 * self.ascent
    }

    /// The effective ex height: the stored `ex_height` when present and
    /// positive, otherwise an estimate of `0.75 * effective_cap_height()`.
    pub(crate) fn effective_ex_height(&self) -> f64 {
        if let Some(value) = self.ex_height {
            if value > 0.0 {
                return value;
            }
        }
        0.75 * self.effective_cap_height()
    }

    /// The effective ASCII height: the stored `ascii_height` when present and
    /// positive, otherwise an estimate of `1.5 * effective_cap_height()`.
    pub(crate) fn effective_ascii_height(&self) -> f64 {
        if let Some(value) = self.ascii_height {
            if value > 0.0 {
                return value;
            }
        }
        1.5 * self.effective_cap_height()
    }

    /// The effective ideograph width: the stored `ic_width` when present and
    /// positive, otherwise the minimum of the ASCII height and two cell widths.
    pub(crate) fn effective_ic_width(&self) -> f64 {
        if let Some(value) = self.ic_width {
            if value > 0.0 {
                return value;
            }
        }
        self.effective_ascii_height().min(2.0 * self.cell_width)
    }

    /// The effective underline thickness: the stored value when present and
    /// positive, otherwise an estimate of `0.15 * effective_ex_height()`.
    pub(crate) fn effective_underline_thickness(&self) -> f64 {
        if let Some(value) = self.underline_thickness {
            if value > 0.0 {
                return value;
            }
        }
        0.15 * self.effective_ex_height()
    }

    /// The effective strikethrough thickness: the stored value when present and
    /// positive, otherwise equal to the underline thickness.
    pub(crate) fn effective_strikethrough_thickness(&self) -> f64 {
        if let Some(value) = self.strikethrough_thickness {
            if value > 0.0 {
                return value;
            }
        }
        self.effective_underline_thickness()
    }

    /// The effective underline position. Positions are valid whether positive or
    /// negative, so a stored value is used as-is; otherwise it is placed one
    /// underline thickness below the baseline.
    pub(crate) fn effective_underline_position(&self) -> f64 {
        self.underline_position
            .unwrap_or_else(|| -self.effective_underline_thickness())
    }

    /// The effective strikethrough position. A stored value is used as-is;
    /// otherwise it is centered at half the ex height plus thickness.
    pub(crate) fn effective_strikethrough_position(&self) -> f64 {
        self.strikethrough_position.unwrap_or_else(|| {
            (self.effective_ex_height() + self.effective_strikethrough_thickness()) * 0.5
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Metrics {
        Metrics {
            cell_width: 8,
            cell_height: 16,
            cell_baseline: 3,
            underline_position: 13,
            underline_thickness: 1,
            strikethrough_position: 8,
            strikethrough_thickness: 1,
            overline_position: 0,
            overline_thickness: 1,
            box_thickness: 2,
            cursor_thickness: 1,
            cursor_height: 16,
            icon_height: 12.5,
            icon_height_single: 11.0,
            face_width: 7.75,
            face_height: 15.5,
            face_y: 1.25,
        }
    }

    #[test]
    fn metrics_holds_fields() {
        let m = sample();
        assert_eq!(m.cell_width, 8);
        assert_eq!(m.cell_height, 16);
        assert_eq!(m.cell_baseline, 3);
        assert_eq!(m.underline_position, 13);
        assert_eq!(m.underline_thickness, 1);
        assert_eq!(m.strikethrough_position, 8);
        assert_eq!(m.strikethrough_thickness, 1);
        assert_eq!(m.overline_position, 0);
        assert_eq!(m.overline_thickness, 1);
        assert_eq!(m.box_thickness, 2);
        assert_eq!(m.cursor_thickness, 1);
        assert_eq!(m.cursor_height, 16);
        assert_eq!(m.icon_height, 12.5);
        assert_eq!(m.icon_height_single, 11.0);
        assert_eq!(m.face_width, 7.75);
        assert_eq!(m.face_height, 15.5);
        assert_eq!(m.face_y, 1.25);
    }

    #[test]
    fn metrics_overline_position_is_signed() {
        let mut m = sample();
        m.overline_position = -2;
        assert_eq!(m.overline_position, -2);
    }

    #[test]
    fn metrics_face_fields_are_f64() {
        let mut m = sample();
        m.face_width = 7.3;
        m.icon_height = 0.5;
        assert_eq!(m.face_width, 7.3);
        assert_eq!(m.icon_height, 0.5);
    }

    fn face_sample() -> FaceMetrics {
        FaceMetrics {
            px_per_em: 16.0,
            cell_width: 8.0,
            ascent: 12.0,
            descent: -4.0,
            line_gap: 2.0,
            underline_position: Some(-1.0),
            underline_thickness: Some(1.0),
            strikethrough_position: None,
            strikethrough_thickness: None,
            cap_height: None,
            ex_height: None,
            ascii_height: None,
            ic_width: None,
        }
    }

    #[test]
    fn face_metrics_holds_fields() {
        let f = face_sample();
        assert_eq!(f.px_per_em, 16.0);
        assert_eq!(f.cell_width, 8.0);
        assert_eq!(f.ascent, 12.0);
        assert_eq!(f.descent, -4.0);
        assert_eq!(f.line_gap, 2.0);
        assert_eq!(f.underline_position, Some(-1.0));
        assert_eq!(f.strikethrough_position, None);
        assert_eq!(f.ic_width, None);
    }

    #[test]
    fn face_metrics_line_height() {
        let mut f = face_sample();
        f.ascent = 10.0;
        f.descent = -2.0;
        f.line_gap = 1.0;
        // 10 - (-2) + 1 = 13
        assert_eq!(f.line_height(), 13.0);
    }

    #[test]
    fn effective_cap_height_uses_value_when_positive() {
        let mut f = face_sample();
        f.cap_height = Some(9.0);
        assert_eq!(f.effective_cap_height(), 9.0);
    }

    #[test]
    fn effective_cap_height_estimates_when_absent_or_nonpositive() {
        let mut f = face_sample(); // ascent = 12 -> estimate 9.0
        f.cap_height = None;
        assert_eq!(f.effective_cap_height(), 9.0);
        f.cap_height = Some(0.0);
        assert_eq!(f.effective_cap_height(), 9.0);
        f.cap_height = Some(-1.0);
        assert_eq!(f.effective_cap_height(), 9.0);
    }

    #[test]
    fn effective_ex_height_uses_value_when_positive() {
        let mut f = face_sample();
        f.ex_height = Some(5.0);
        assert_eq!(f.effective_ex_height(), 5.0);
    }

    #[test]
    fn effective_ex_height_estimates_when_absent_or_nonpositive() {
        // ascent 12 -> cap estimate 9.0 -> ex estimate 0.75 * 9.0 = 6.75.
        let mut f = face_sample();
        f.ex_height = None;
        f.cap_height = None;
        assert_eq!(f.effective_ex_height(), 6.75);
        f.ex_height = Some(0.0);
        assert_eq!(f.effective_ex_height(), 6.75);
        f.ex_height = Some(-1.0);
        assert_eq!(f.effective_ex_height(), 6.75);
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn effective_ascii_height_value_and_estimate() {
        let mut f = face_sample();
        f.ascii_height = Some(20.0);
        assert!(approx(f.effective_ascii_height(), 20.0));
        // ascent 12 -> cap estimate 9 -> 1.5 * 9 = 13.5.
        for v in [None, Some(0.0), Some(-1.0)] {
            f.ascii_height = v;
            assert!(approx(f.effective_ascii_height(), 13.5));
        }
    }

    #[test]
    fn effective_ic_width_value_and_min() {
        let mut f = face_sample(); // cell_width 8, ascii estimate 13.5
        f.ic_width = Some(10.0);
        assert!(approx(f.effective_ic_width(), 10.0));
        // Non-positive falls through to min(ascii=13.5, 2*8=16) = 13.5.
        for v in [None, Some(0.0), Some(-1.0)] {
            f.ic_width = v;
            f.ascii_height = None;
            assert!(approx(f.effective_ic_width(), 13.5));
        }
        // When two cell widths is the smaller, it wins.
        let mut g = face_sample();
        g.ic_width = None;
        g.ascii_height = None;
        g.cell_width = 5.0; // 2 * 5 = 10 < 13.5
        assert!(approx(g.effective_ic_width(), 10.0));
    }

    #[test]
    fn effective_underline_thickness_value_and_estimate() {
        let mut f = face_sample();
        f.underline_thickness = Some(2.0);
        assert!(approx(f.effective_underline_thickness(), 2.0));
        // ex estimate 6.75 -> 0.15 * 6.75 = 1.0125.
        for v in [None, Some(0.0), Some(-1.0)] {
            f.underline_thickness = v;
            assert!(approx(f.effective_underline_thickness(), 1.0125));
        }
    }

    #[test]
    fn effective_strikethrough_thickness_value_and_fallback() {
        let mut f = face_sample();
        f.strikethrough_thickness = Some(3.0);
        assert!(approx(f.effective_strikethrough_thickness(), 3.0));
        // Fallback equals the underline thickness.
        f.strikethrough_thickness = None;
        f.underline_thickness = Some(2.0);
        assert!(approx(
            f.effective_strikethrough_thickness(),
            f.effective_underline_thickness()
        ));
    }

    #[test]
    fn effective_underline_position_honors_negative() {
        let mut f = face_sample();
        f.underline_position = Some(-2.0);
        // No `> 0` guard: a negative stored position is used as-is.
        assert!(approx(f.effective_underline_position(), -2.0));
        // Fallback: one underline thickness below the baseline.
        f.underline_position = None;
        f.underline_thickness = None;
        assert!(approx(
            f.effective_underline_position(),
            -f.effective_underline_thickness()
        ));
    }

    #[test]
    fn effective_strikethrough_position_honors_negative() {
        let mut f = face_sample();
        f.strikethrough_position = Some(-1.5);
        assert!(approx(f.effective_strikethrough_position(), -1.5));
        // Fallback: (ex + strikethrough_thickness) * 0.5.
        f.strikethrough_position = None;
        let expected = (f.effective_ex_height() + f.effective_strikethrough_thickness()) * 0.5;
        assert!(approx(f.effective_strikethrough_position(), expected));
    }
}
