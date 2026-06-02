//! Font metrics: recommended cell dimensions and decoration positions.
//!
//! Faithful port of upstream `font/Metrics.zig`: the `Metrics` and `FaceMetrics`
//! value types and their accessors, the `calc` derivation and `clamp`, and the
//! `Modifier`/`Key`/`ModifierSet` metric-modifier types. `Metrics::apply` and
//! constraint application are ported in later slices.

use std::collections::HashMap;

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

/// Convert an integer-valued, non-negative `f64` metric to `u32`. The sources are
/// `round`/`ceil`/`max(1, …)` outputs, so the truncation is exact; the
/// `debug_assert!` catches an out-of-domain derivation in debug/test builds
/// instead of letting `as u32` silently saturate it to `0`.
fn f64_to_u32(value: f64) -> u32 {
    debug_assert!(
        value.is_finite() && value >= 0.0 && value <= u32::MAX as f64,
        "metric out of u32 domain: {value}"
    );
    value as u32
}

impl Metrics {
    /// Calculate metrics from values extracted from a font face. Pass values with
    /// as much precision as possible — do not round them before calling. Nullable
    /// inputs use the `FaceMetrics` effective-accessor estimates.
    pub(crate) fn calc(face: FaceMetrics) -> Metrics {
        // Unrounded advance width and line height, retained separately for
        // scaling calculations.
        let face_width = face.cell_width;
        let face_height = face.line_height();

        // Cell pixel dimensions. We round to keep within 0.5px of the true size.
        let cell_width = face_width.round();
        let cell_height = face_height.round();

        // Split the line gap evenly between the top and bottom of the cell.
        let half_line_gap = face.line_gap / 2.0;

        // NOTE: `cell_baseline` is relative to the BOTTOM of the cell.
        let face_baseline = half_line_gap - face.descent;
        // Center the face vertically in the pixel-rounded cell height.
        let cell_baseline = (face_baseline - (cell_height - face_height) / 2.0).round();

        // Offset from the cell bottom to the face's "true" bounding-box bottom.
        let face_y = cell_baseline - face_baseline;

        let top_to_baseline = cell_height - cell_baseline;

        let cap_height = face.effective_cap_height();
        let underline_thickness = face.effective_underline_thickness().ceil().max(1.0);
        let strikethrough_thickness = face.effective_strikethrough_thickness().ceil().max(1.0);
        let underline_position = (top_to_baseline - face.effective_underline_position()).round();
        let strikethrough_position =
            (top_to_baseline - face.effective_strikethrough_position()).round();

        // `icon_height` is kept separate from `face_height` so modifiers can apply
        // to the former without affecting the latter.
        let icon_height = face_height;
        let icon_height_single = (2.0 * cap_height + face_height) / 3.0;

        let mut result = Metrics {
            cell_width: f64_to_u32(cell_width),
            cell_height: f64_to_u32(cell_height),
            cell_baseline: f64_to_u32(cell_baseline),
            underline_position: f64_to_u32(underline_position),
            underline_thickness: f64_to_u32(underline_thickness),
            strikethrough_position: f64_to_u32(strikethrough_position),
            strikethrough_thickness: f64_to_u32(strikethrough_thickness),
            overline_position: 0,
            overline_thickness: f64_to_u32(underline_thickness),
            box_thickness: f64_to_u32(underline_thickness),
            // Not determined by fonts; the upstream `Metrics` struct default is 1.
            cursor_thickness: 1,
            cursor_height: f64_to_u32(cell_height),
            icon_height,
            icon_height_single,
            face_width,
            face_height,
            face_y,
        };

        // Ensure all metrics are within their allowable range.
        result.clamp();
        result
    }

    /// Ensure all metrics are within their allowable range (the `Minimums`).
    /// `cell_baseline`, the positions, and `face_y` have no minimum.
    fn clamp(&mut self) {
        self.cell_width = self.cell_width.max(1);
        self.cell_height = self.cell_height.max(1);
        self.underline_thickness = self.underline_thickness.max(1);
        self.strikethrough_thickness = self.strikethrough_thickness.max(1);
        self.overline_thickness = self.overline_thickness.max(1);
        self.box_thickness = self.box_thickness.max(1);
        self.cursor_thickness = self.cursor_thickness.max(1);
        self.cursor_height = self.cursor_height.max(1);
        self.icon_height = self.icon_height.max(1.0);
        self.icon_height_single = self.icon_height_single.max(1.0);
        self.face_height = self.face_height.max(1.0);
        self.face_width = self.face_width.max(1.0);
    }
}

/// An error parsing a [`Modifier`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModifierParseError {
    InvalidFormat,
}

/// A modifier to apply to a metrics value. The value is a delta, not a target:
/// a percent of `"20%"` means 20% larger (stored as the multiplier `1.2`), and
/// an absolute of `"20"` means 20 larger.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Modifier {
    Percent(f64),
    Absolute(i32),
}

impl Modifier {
    /// Parse a modifier value. A trailing `%` makes it a percent; otherwise the
    /// value is parsed as an integer absolute delta.
    pub(crate) fn parse(input: &str) -> Result<Modifier, ModifierParseError> {
        if input.is_empty() {
            return Err(ModifierParseError::InvalidFormat);
        }

        if let Some(prefix) = input.strip_suffix('%') {
            let percent: f64 = prefix
                .parse()
                .map_err(|_| ModifierParseError::InvalidFormat)?;
            let percent = percent / 100.0;
            // A percent of <= -1 (i.e. "-100%" or more negative) clamps the
            // multiplier to 0; otherwise the stored value is 1 + the fraction.
            if percent <= -1.0 {
                return Ok(Modifier::Percent(0.0));
            }
            return Ok(Modifier::Percent(1.0 + percent));
        }

        let absolute: i32 = input
            .parse()
            .map_err(|_| ModifierParseError::InvalidFormat)?;
        Ok(Modifier::Absolute(absolute))
    }

    /// Apply this modifier to an unsigned metric value.
    pub(crate) fn apply_u32(self, v: u32) -> u32 {
        match self {
            Modifier::Percent(p) => {
                let applied = (v as f64 * p.max(0.0)).round();
                applied.clamp(0.0, u32::MAX as f64) as u32
            }
            // Saturating add, then the unsigned clamp-below-0 and saturate-above.
            Modifier::Absolute(abs) => (v as i64)
                .saturating_add(abs as i64)
                .clamp(0, u32::MAX as i64) as u32,
        }
    }

    /// Apply this modifier to a signed metric value.
    pub(crate) fn apply_i32(self, v: i32) -> i32 {
        match self {
            Modifier::Percent(p) => {
                let applied = (v as f64 * p.max(0.0)).round();
                applied.clamp(i32::MIN as f64, i32::MAX as f64) as i32
            }
            // Upstream saturates a failed cast to `maxInt * sign`, so a negative
            // overflow becomes `-i32::MAX`, never `i32::MIN`.
            Modifier::Absolute(abs) => (v as i64)
                .saturating_add(abs as i64)
                .clamp(-(i32::MAX as i64), i32::MAX as i64)
                as i32,
        }
    }

    /// Apply this modifier to a floating-point metric value.
    pub(crate) fn apply_f64(self, v: f64) -> f64 {
        match self {
            Modifier::Percent(p) => v * p.max(0.0),
            Modifier::Absolute(abs) => v + abs as f64,
        }
    }
}

/// Identifies a modifiable metric — one per `Metrics` field. The discriminants
/// match the `Metrics` field order (upstream derives `Key` from the field
/// indices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum Key {
    CellWidth = 0,
    CellHeight = 1,
    CellBaseline = 2,
    UnderlinePosition = 3,
    UnderlineThickness = 4,
    StrikethroughPosition = 5,
    StrikethroughThickness = 6,
    OverlinePosition = 7,
    OverlineThickness = 8,
    BoxThickness = 9,
    CursorThickness = 10,
    CursorHeight = 11,
    IconHeight = 12,
    IconHeightSingle = 13,
    FaceWidth = 14,
    FaceHeight = 15,
    FaceY = 16,
}

/// A set of modifiers to apply to metrics, keyed by [`Key`]. Most metrics are
/// unmodified, so a map keeps it compact.
pub(crate) type ModifierSet = HashMap<Key, Modifier>;

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

    fn clean_face() -> FaceMetrics {
        FaceMetrics {
            px_per_em: 16.0,
            cell_width: 8.0,
            ascent: 12.0,
            descent: -4.0,
            line_gap: 0.0,
            underline_position: None,
            underline_thickness: None,
            strikethrough_position: None,
            strikethrough_thickness: None,
            cap_height: None,
            ex_height: None,
            ascii_height: None,
            ic_width: None,
        }
    }

    #[test]
    fn calc_derives_clean_metrics() {
        let m = Metrics::calc(clean_face());
        assert_eq!(m.cell_width, 8);
        assert_eq!(m.cell_height, 16);
        assert_eq!(m.cell_baseline, 4);
        assert_eq!(m.underline_position, 13);
        assert_eq!(m.underline_thickness, 2);
        assert_eq!(m.strikethrough_position, 8);
        assert_eq!(m.strikethrough_thickness, 2);
        assert_eq!(m.overline_position, 0);
        assert_eq!(m.overline_thickness, 2);
        assert_eq!(m.box_thickness, 2);
        assert_eq!(m.cursor_thickness, 1);
        assert_eq!(m.cursor_height, 16);
        assert!(approx(m.icon_height, 16.0));
        assert!(approx(m.icon_height_single, 34.0 / 3.0)); // (2*9 + 16) / 3
        assert!(approx(m.face_width, 8.0));
        assert!(approx(m.face_height, 16.0));
        assert!(approx(m.face_y, 0.0));
    }

    #[test]
    fn calc_clamps_minimums() {
        let face = FaceMetrics {
            px_per_em: 16.0,
            cell_width: 0.0,
            ascent: 0.0,
            descent: 0.0,
            line_gap: 0.0,
            underline_position: None,
            underline_thickness: None,
            strikethrough_position: None,
            strikethrough_thickness: None,
            cap_height: None,
            ex_height: None,
            ascii_height: None,
            ic_width: None,
        };
        let m = Metrics::calc(face);
        assert!(m.cell_width >= 1);
        assert!(m.cell_height >= 1);
        assert!(m.cursor_height >= 1);
        assert!(m.icon_height >= 1.0);
        assert!(m.icon_height_single >= 1.0);
        assert!(m.face_width >= 1.0);
        assert!(m.face_height >= 1.0);
    }

    #[test]
    fn calc_line_gap_splits_evenly() {
        let m0 = Metrics::calc(clean_face());
        let mut face = clean_face();
        face.line_gap = 4.0;
        let m4 = Metrics::calc(face);
        // The full 4px line gap grows the cell height; half (2px) is added above
        // the baseline.
        assert_eq!(m4.cell_height - m0.cell_height, 4);
        assert_eq!(m4.cell_baseline - m0.cell_baseline, 2);
    }

    #[test]
    fn clamp_raises_all_twelve_minimum_fields() {
        let mut m = Metrics {
            cell_width: 0,
            cell_height: 0,
            cell_baseline: 7,
            underline_position: 8,
            underline_thickness: 0,
            strikethrough_position: 9,
            strikethrough_thickness: 0,
            overline_position: -3,
            overline_thickness: 0,
            box_thickness: 0,
            cursor_thickness: 0,
            cursor_height: 0,
            icon_height: 0.0,
            icon_height_single: 0.0,
            face_width: 0.0,
            face_height: 0.0,
            face_y: 2.5,
        };
        m.clamp();
        // All twelve clamped fields raised to their minimum.
        assert_eq!(m.cell_width, 1);
        assert_eq!(m.cell_height, 1);
        assert_eq!(m.underline_thickness, 1);
        assert_eq!(m.strikethrough_thickness, 1);
        assert_eq!(m.overline_thickness, 1);
        assert_eq!(m.box_thickness, 1);
        assert_eq!(m.cursor_thickness, 1);
        assert_eq!(m.cursor_height, 1);
        assert_eq!(m.icon_height, 1.0);
        assert_eq!(m.icon_height_single, 1.0);
        assert_eq!(m.face_width, 1.0);
        assert_eq!(m.face_height, 1.0);
        // The five un-clamped fields are untouched.
        assert_eq!(m.cell_baseline, 7);
        assert_eq!(m.underline_position, 8);
        assert_eq!(m.strikethrough_position, 9);
        assert_eq!(m.overline_position, -3);
        assert_eq!(m.face_y, 2.5);
    }

    fn percent_of(m: Modifier) -> f64 {
        match m {
            Modifier::Percent(p) => p,
            other => panic!("expected Percent, got {other:?}"),
        }
    }

    #[test]
    fn modifier_parse_percent() {
        assert!(approx(percent_of(Modifier::parse("20%").unwrap()), 1.2));
        assert!(approx(percent_of(Modifier::parse("-20%").unwrap()), 0.8));
        assert!(approx(percent_of(Modifier::parse("0%").unwrap()), 1.0));
    }

    #[test]
    fn modifier_parse_percent_clamps() {
        assert!(approx(percent_of(Modifier::parse("-100%").unwrap()), 0.0));
        assert!(approx(percent_of(Modifier::parse("-150%").unwrap()), 0.0));
    }

    #[test]
    fn modifier_parse_absolute() {
        assert_eq!(Modifier::parse("5").unwrap(), Modifier::Absolute(5));
        assert_eq!(Modifier::parse("-3").unwrap(), Modifier::Absolute(-3));
        assert_eq!(Modifier::parse("+5").unwrap(), Modifier::Absolute(5));
    }

    #[test]
    fn modifier_parse_errors() {
        assert!(Modifier::parse("").is_err());
        assert!(Modifier::parse("abc").is_err());
        assert!(Modifier::parse("abc%").is_err());
        assert!(Modifier::parse("%").is_err());
    }

    #[test]
    fn apply_u32_percent() {
        assert_eq!(Modifier::Percent(1.2).apply_u32(10), 12);
        assert_eq!(Modifier::Percent(0.8).apply_u32(10), 8);
        assert_eq!(Modifier::Percent(-1.0).apply_u32(10), 0);
    }

    #[test]
    fn apply_u32_absolute() {
        assert_eq!(Modifier::Absolute(5).apply_u32(10), 15);
        assert_eq!(Modifier::Absolute(-3).apply_u32(10), 7);
        assert_eq!(Modifier::Absolute(-20).apply_u32(10), 0);
    }

    #[test]
    fn apply_u32_saturates() {
        assert_eq!(Modifier::Absolute(i32::MAX).apply_u32(u32::MAX), u32::MAX);
    }

    #[test]
    fn apply_i32_signed() {
        assert_eq!(Modifier::Absolute(-20).apply_i32(10), -10);
        assert_eq!(Modifier::Percent(1.5).apply_i32(-4), -6);
    }

    #[test]
    fn apply_i32_negative_overflow_saturates() {
        // Upstream `maxInt * sign` saturation yields -i32::MAX, not i32::MIN.
        assert_eq!(Modifier::Absolute(i32::MIN).apply_i32(i32::MIN), -i32::MAX);
    }

    #[test]
    fn apply_f64() {
        assert!(approx(Modifier::Percent(1.2).apply_f64(10.0), 12.0));
        assert_eq!(Modifier::Absolute(5).apply_f64(10.0), 15.0);
        assert_eq!(Modifier::Absolute(-3).apply_f64(2.5), -0.5);
        assert_eq!(Modifier::Percent(-1.0).apply_f64(10.0), 0.0);
    }

    const ALL_KEYS: [Key; 17] = [
        Key::CellWidth,
        Key::CellHeight,
        Key::CellBaseline,
        Key::UnderlinePosition,
        Key::UnderlineThickness,
        Key::StrikethroughPosition,
        Key::StrikethroughThickness,
        Key::OverlinePosition,
        Key::OverlineThickness,
        Key::BoxThickness,
        Key::CursorThickness,
        Key::CursorHeight,
        Key::IconHeight,
        Key::IconHeightSingle,
        Key::FaceWidth,
        Key::FaceHeight,
        Key::FaceY,
    ];

    #[test]
    fn key_discriminants() {
        for (i, key) in ALL_KEYS.iter().enumerate() {
            assert_eq!(*key as u8, i as u8);
        }
    }

    #[test]
    fn key_matches_metrics_field_count() {
        assert_eq!(ALL_KEYS.len(), 17);
        // An exhaustive, wildcard-free match: adding or removing a `Metrics`
        // field (hence a `Key` variant) forces this to be updated.
        for key in ALL_KEYS {
            match key {
                Key::CellWidth
                | Key::CellHeight
                | Key::CellBaseline
                | Key::UnderlinePosition
                | Key::UnderlineThickness
                | Key::StrikethroughPosition
                | Key::StrikethroughThickness
                | Key::OverlinePosition
                | Key::OverlineThickness
                | Key::BoxThickness
                | Key::CursorThickness
                | Key::CursorHeight
                | Key::IconHeight
                | Key::IconHeightSingle
                | Key::FaceWidth
                | Key::FaceHeight
                | Key::FaceY => {}
            }
        }
    }

    #[test]
    fn modifier_set_insert_get() {
        let mut set = ModifierSet::new();
        set.insert(Key::CellWidth, Modifier::Percent(1.2));
        set.insert(Key::OverlinePosition, Modifier::Absolute(-2));
        assert_eq!(
            set.get(&Key::OverlinePosition),
            Some(&Modifier::Absolute(-2))
        );
        match set.get(&Key::CellWidth) {
            Some(Modifier::Percent(p)) => assert!(approx(*p, 1.2)),
            other => panic!("unexpected {other:?}"),
        }
    }
}
