//! Resolves a codepoint to the font face that should render it.
//!
//! Faithful port of the core resolution chain of upstream
//! `font/CodepointResolver.zig`. The resolver sits on top of a [`Collection`]
//! and adds style-disabled fallback, presentation defaults, and the
//! regular-style fallback chain, resolves sprite codepoints to the
//! procedurally-drawn sprite face when sprite drawing is enabled, and defaults a
//! presentation-less codepoint via the UCD `Emoji_Presentation` property.
//! Codepoint overrides and discovery-based fallback are deferred to later
//! experiments.

use crate::font::atlas::{Atlas, AtlasError};
use crate::font::collection::{Collection, EntryError, Index, PresentationMode, Special};
use crate::font::face::coretext::{RenderGlyphError, RenderOptions};
use crate::font::glyph::Glyph;
use crate::font::metrics::Metrics;
use crate::font::{Presentation, Style};

/// An all-zero [`Glyph`]: the upstream fallback when a resolved sprite index has
/// no draw function (defensive; should not occur for a properly-resolved index).
const BLANK_GLYPH: Glyph = Glyph {
    width: 0,
    height: 0,
    offset_x: 0,
    offset_y: 0,
    atlas_x: 0,
    atlas_y: 0,
};

/// An error rendering a resolved glyph through the [`CodepointResolver`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolverRenderError {
    /// The index is a sprite glyph, but sprite drawing is disabled (no grid
    /// metrics were set via [`CodepointResolver::set_sprite_metrics`]).
    SpriteUnavailable,
    /// The index couldn't be resolved to a face.
    Entry(EntryError),
    /// The face failed to render the glyph.
    Render(RenderGlyphError),
    /// The sprite render failed to reserve atlas space.
    Atlas(AtlasError),
}

impl From<EntryError> for ResolverRenderError {
    fn from(err: EntryError) -> Self {
        ResolverRenderError::Entry(err)
    }
}

impl From<AtlasError> for ResolverRenderError {
    fn from(err: AtlasError) -> Self {
        ResolverRenderError::Atlas(err)
    }
}

impl From<RenderGlyphError> for ResolverRenderError {
    fn from(err: RenderGlyphError) -> Self {
        ResolverRenderError::Render(err)
    }
}

/// Resolves codepoints to face indices over a [`Collection`].
pub(crate) struct CodepointResolver {
    /// The collection of faces this resolver searches.
    collection: Collection,
    /// Whether each style is enabled, indexed by `Style as usize`. A disabled
    /// non-regular style falls back to regular. Faithful port of upstream's
    /// `StyleStatus` (`EnumArray(Style, bool)`).
    styles: [bool; 4],
    /// The grid metrics for sprite rendering. `None` disables sprite drawing —
    /// the analog of upstream's optional `sprite: ?SpriteFace`.
    sprite_metrics: Option<Metrics>,
}

impl CodepointResolver {
    /// Create a resolver over `collection` with all styles enabled and sprite
    /// drawing disabled.
    pub(crate) fn new(collection: Collection) -> CodepointResolver {
        CodepointResolver {
            collection,
            styles: [true; 4],
            sprite_metrics: None,
        }
    }

    /// Enable sprite drawing with the given grid `metrics`, or disable it with
    /// `None`. Faithful analog of setting upstream's `sprite: ?SpriteFace`.
    pub(crate) fn set_sprite_metrics(&mut self, metrics: Option<Metrics>) {
        self.sprite_metrics = metrics;
    }

    /// The underlying collection.
    pub(crate) fn collection(&self) -> &Collection {
        &self.collection
    }

    /// The underlying collection, mutably (e.g. to add faces, complete styles).
    pub(crate) fn collection_mut(&mut self) -> &mut Collection {
        &mut self.collection
    }

    /// Enable or disable a style. A disabled non-regular style resolves as
    /// regular.
    pub(crate) fn set_style_enabled(&mut self, style: Style, enabled: bool) {
        self.styles[style as usize] = enabled;
    }

    /// Resolve `cp` (in `style`, with optional explicit presentation `p`) to a
    /// face [`Index`], or `None`. Faithful port of upstream `getIndex`'s core
    /// chain (the sprite check, the UCD presentation default, and the
    /// regular-style/last-resort fallbacks; codepoint overrides and discovery
    /// are deferred).
    pub(crate) fn get_index(
        &self,
        cp: u32,
        style: Style,
        p: Option<Presentation>,
    ) -> Option<Index> {
        // A disabled non-regular style falls back to regular.
        if style != Style::Regular && !self.styles[style as usize] {
            return self.get_index(cp, Style::Regular, p);
        }

        // (Codepoint overrides are deferred here.)

        // A sprite codepoint always resolves to the sprite face (when enabled).
        if let Some(m) = &self.sprite_metrics {
            if crate::font::sprite::draw::has_codepoint(cp, m) {
                return Some(Index::special(Special::Sprite));
            }
        }

        // Build the presentation mode. With an explicit presentation we use it;
        // otherwise we consult the Unicode Character Database (the
        // `Emoji_Presentation` property) for the default — emoji for codepoints
        // that render as emoji without a variation selector, text otherwise.
        let p_mode = match p {
            Some(v) => PresentationMode::Explicit(v),
            None => PresentationMode::Default(
                if crate::font::emoji_presentation::is_emoji_presentation(cp) {
                    Presentation::Emoji
                } else {
                    Presentation::Text
                },
            ),
        };

        // Exact match in the requested style.
        if let Some(idx) = self.collection.get_index(cp, style, p_mode) {
            return Some(idx);
        }

        // For a non-regular style, retry as regular before giving up.
        if style != Style::Regular {
            if let Some(idx) = self.get_index(cp, Style::Regular, p) {
                return Some(idx);
            }
        }

        // (Discovery-based fallback is deferred here.)

        // A regular request with `any` presentation has nothing more to try.
        // (Effectively unreachable: `p_mode` is always `Explicit` or `Default`.)
        if style == Style::Regular && p_mode == PresentationMode::Any {
            return None;
        }

        // Last resort: any regular face that has the codepoint in any
        // presentation.
        self.collection
            .get_index(cp, Style::Regular, PresentationMode::Any)
    }

    /// The presentation a glyph at `index` requires (which atlas to use).
    /// Faithful port of upstream `getPresentation`: a sprite index is text; a
    /// real face's glyph is emoji if it's a color glyph, else text.
    pub(crate) fn get_presentation(
        &self,
        index: Index,
        glyph: u16,
    ) -> Result<Presentation, EntryError> {
        // The only special kind is the sprite font, which is text presentation.
        if index.special_kind().is_some() {
            return Ok(Presentation::Text);
        }
        let face = self.collection.get_face(index)?;
        Ok(if face.is_color_glyph(glyph) {
            Presentation::Emoji
        } else {
            Presentation::Text
        })
    }

    /// Render the glyph `glyph_index` at `index` into `atlas`, returning its
    /// [`Glyph`]. For a sprite index the `glyph_index` is the codepoint (hence
    /// `u32`, which holds the high sprite ranges); it renders via the sprite
    /// font when enabled, else returns
    /// [`ResolverRenderError::SpriteUnavailable`]. Faithful port of upstream
    /// `renderGlyph`.
    pub(crate) fn render_glyph(
        &self,
        atlas: &mut Atlas,
        index: Index,
        glyph_index: u32,
        opts: &RenderOptions,
    ) -> Result<Glyph, ResolverRenderError> {
        if index.special_kind().is_some() {
            // The sprite glyph index is its codepoint.
            let m = self
                .sprite_metrics
                .as_ref()
                .ok_or(ResolverRenderError::SpriteUnavailable)?;
            return Ok(crate::font::sprite::render_codepoint(
                glyph_index,
                m,
                opts.cell_width,
                atlas,
            )?
            .unwrap_or(BLANK_GLYPH));
        }
        let face = self.collection.get_face(index)?;
        // CoreText glyph ids fit in `u16`.
        Ok(face.render_glyph(atlas, glyph_index as u16, opts)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::collection::SyntheticStyle;
    use crate::font::face::coretext::Face;

    const NO_SYNTHESIS: SyntheticStyle = SyntheticStyle {
        italic: false,
        bold: false,
        bold_italic: false,
    };

    fn menlo_resolver() -> CodepointResolver {
        let mut c = Collection::new();
        c.add(Face::new("Menlo", 32.0), Style::Regular, false)
            .unwrap();
        CodepointResolver::new(c)
    }

    #[test]
    fn get_index_default_presentation_emoji() {
        // U+2614 (umbrella with rain, Emoji_Presentation = Yes) exists in both a
        // monochrome text face (non-color) and the color emoji face. Both are
        // added as fallback faces, so presentation discriminates between them.
        // With no explicit presentation the UCD default is emoji, so it resolves
        // to the color face; with a forced text presentation it resolves to the
        // text face — proving `get_index` consults `is_emoji_presentation` for
        // the default (and that the exact-match step, not the last-resort `Any`,
        // makes the choice).
        let mut c = Collection::new();
        let text = c
            .add(Face::new("Menlo", 32.0), Style::Regular, true)
            .unwrap();
        let emoji = c
            .add(Face::new("Apple Color Emoji", 32.0), Style::Regular, true)
            .unwrap();
        let r = CodepointResolver::new(c);
        let umbrella = 0x2614;
        assert_eq!(
            r.get_index(umbrella, Style::Regular, None),
            Some(emoji),
            "the emoji default resolves the color umbrella"
        );
        assert_eq!(
            r.get_index(umbrella, Style::Regular, Some(Presentation::Text)),
            Some(text),
            "a forced text presentation resolves the monochrome umbrella"
        );
    }

    #[test]
    fn resolve_basic() {
        let r = menlo_resolver();
        let m = 'M' as u32;
        let at0 = Some(Index::new(Style::Regular, 0));
        assert_eq!(
            r.get_index(m, Style::Regular, Some(Presentation::Text)),
            at0
        );
        assert_eq!(r.get_index(m, Style::Regular, None), at0);
    }

    #[test]
    fn resolve_missing() {
        let r = menlo_resolver();
        // A Private-Use codepoint Menlo lacks; discovery is deferred -> None.
        assert_eq!(
            r.get_index(0xE000, Style::Regular, Some(Presentation::Text)),
            None
        );
    }

    #[test]
    fn resolve_emoji_via_regular_any() {
        let mut c = Collection::new();
        c.add(Face::new("Menlo", 32.0), Style::Regular, false)
            .unwrap();
        c.add(Face::new("Apple Color Emoji", 32.0), Style::Regular, false)
            .unwrap();
        let r = CodepointResolver::new(c);
        let emoji = 0x1F600u32;
        // Explicit Text misses (Menlo lacks it; the emoji glyph is color, not
        // text), but the final regular/any fallback finds the emoji at idx 1.
        assert_eq!(
            r.get_index(emoji, Style::Regular, Some(Presentation::Text)),
            Some(Index::new(Style::Regular, 1))
        );
    }

    #[test]
    fn resolve_style_disabled_falls_back() {
        let mut c = Collection::new();
        c.add(Face::new("Menlo", 32.0), Style::Regular, false)
            .unwrap();
        // Alias the missing styles (Bold -> Regular).
        c.complete_styles(NO_SYNTHESIS).expect("complete");
        let mut r = CodepointResolver::new(c);
        r.set_style_enabled(Style::Bold, false);
        // Bold is disabled -> recurse as regular -> {Regular, 0}.
        assert_eq!(
            r.get_index('M' as u32, Style::Bold, Some(Presentation::Text)),
            Some(Index::new(Style::Regular, 0))
        );
    }

    fn none_opts(face: &Face) -> RenderOptions {
        RenderOptions {
            grid_metrics: crate::font::metrics::Metrics::calc(face.get_metrics()),
            cell_width: None,
            constraint: crate::font::face::constraint::Constraint::default(),
            constraint_width: 1,
            thicken: false,
            thicken_strength: 255,
        }
    }

    #[test]
    fn get_presentation_text() {
        let r = menlo_resolver();
        let glyph = r
            .collection()
            .get_face(Index::new(Style::Regular, 0))
            .unwrap()
            .glyph_index('M' as u32)
            .unwrap();
        assert_eq!(
            r.get_presentation(Index::new(Style::Regular, 0), glyph),
            Ok(Presentation::Text)
        );
    }

    #[test]
    fn get_presentation_emoji() {
        let mut c = Collection::new();
        c.add(Face::new("Apple Color Emoji", 32.0), Style::Regular, false)
            .unwrap();
        let r = CodepointResolver::new(c);
        let glyph = r
            .collection()
            .get_face(Index::new(Style::Regular, 0))
            .unwrap()
            .glyph_index(0x1F600)
            .unwrap();
        assert_eq!(
            r.get_presentation(Index::new(Style::Regular, 0), glyph),
            Ok(Presentation::Emoji)
        );
    }

    #[test]
    fn get_presentation_sprite() {
        let r = menlo_resolver();
        // A sprite index is text presentation and never loads a face.
        assert_eq!(
            r.get_presentation(Index::special(crate::font::collection::Special::Sprite), 0),
            Ok(Presentation::Text)
        );
    }

    #[test]
    fn render_glyph_via_resolver() {
        use crate::font::atlas::Format;
        let r = menlo_resolver();
        let mut atlas = Atlas::new(512, Format::Grayscale);
        let idx = Index::new(Style::Regular, 0);
        let face_glyph = r.collection().get_face(idx).unwrap();
        let glyph = face_glyph.glyph_index('M' as u32).unwrap();
        let opts = none_opts(r.collection().get_face(idx).unwrap());
        let g = r
            .render_glyph(&mut atlas, idx, glyph as u32, &opts)
            .expect("render");
        assert!(g.width > 0 && g.height > 0);
    }

    #[test]
    fn render_glyph_sprite_unavailable() {
        use crate::font::atlas::Format;
        let r = menlo_resolver();
        let mut atlas = Atlas::new(512, Format::Grayscale);
        let opts = none_opts(
            r.collection()
                .get_face(Index::new(Style::Regular, 0))
                .unwrap(),
        );
        // Sprite drawing is disabled by default.
        assert_eq!(
            r.render_glyph(
                &mut atlas,
                Index::special(crate::font::collection::Special::Sprite),
                0x2500,
                &opts
            ),
            Err(ResolverRenderError::SpriteUnavailable)
        );
    }

    /// A resolver with sprite drawing enabled (grid metrics from the primary
    /// face).
    fn menlo_resolver_sprites() -> CodepointResolver {
        let mut r = menlo_resolver();
        r.collection_mut().update_metrics().expect("metrics");
        let m = r.collection().metrics().cloned();
        assert!(m.is_some(), "primary face yields grid metrics");
        r.set_sprite_metrics(m);
        r
    }

    #[test]
    fn get_index_sprite_enabled() {
        let r = menlo_resolver_sprites();
        // A box-drawing codepoint resolves to the sprite face.
        assert_eq!(
            r.get_index(0x2500, Style::Regular, None),
            Some(Index::special(Special::Sprite))
        );
    }

    #[test]
    fn get_index_sprite_disabled() {
        let r = menlo_resolver();
        // With sprites disabled, the box-drawing codepoint does not resolve to
        // the sprite face.
        assert_ne!(
            r.get_index(0x2500, Style::Regular, None),
            Some(Index::special(Special::Sprite))
        );
    }

    #[test]
    fn render_glyph_sprite_enabled() {
        use crate::font::atlas::Format;
        let r = menlo_resolver_sprites();
        let mut atlas = Atlas::new(512, Format::Grayscale);
        let opts = none_opts(
            r.collection()
                .get_face(Index::new(Style::Regular, 0))
                .unwrap(),
        );
        let g = r
            .render_glyph(&mut atlas, Index::special(Special::Sprite), 0x2500, &opts)
            .expect("sprite renders");
        assert!(g.width > 0 && g.height > 0, "box line glyph is non-empty");
    }

    #[test]
    fn render_glyph_sprite_high_codepoint() {
        use crate::font::atlas::Format;
        let r = menlo_resolver_sprites();
        let mut atlas = Atlas::new(512, Format::Grayscale);
        let opts = none_opts(
            r.collection()
                .get_face(Index::new(Style::Regular, 0))
                .unwrap(),
        );
        // 0x1FB00 (a sextant) is above u16: the u32 glyph index must not be
        // truncated. It resolves to and renders as a non-empty sprite.
        assert_eq!(
            r.get_index(0x1fb00, Style::Regular, None),
            Some(Index::special(Special::Sprite))
        );
        let g = r
            .render_glyph(&mut atlas, Index::special(Special::Sprite), 0x1fb00, &opts)
            .expect("high sprite renders");
        assert!(g.width > 0 && g.height > 0, "sextant glyph is non-empty");
    }
}
