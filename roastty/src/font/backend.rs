//! Font backend selection.
//!
//! Roastty's current font slice targets upstream's CoreText backend: CoreText
//! handles discovery, rendering, and shaping, with no FreeType, Fontconfig,
//! HarfBuzz, Windows, or WASM canvas backend compiled in.

/// The active font backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Backend {
    /// CoreText for discovery, rendering, and shaping.
    CoreText,
}

impl Backend {
    /// The backend compiled into this Roastty build.
    pub(crate) const fn active() -> Backend {
        Backend::CoreText
    }

    /// Whether this backend uses CoreText.
    pub(crate) const fn has_coretext(self) -> bool {
        matches!(self, Backend::CoreText)
    }

    /// Whether this backend uses FreeType.
    pub(crate) const fn has_freetype(self) -> bool {
        match self {
            Backend::CoreText => false,
        }
    }

    /// Whether this backend uses Fontconfig.
    pub(crate) const fn has_fontconfig(self) -> bool {
        match self {
            Backend::CoreText => false,
        }
    }

    /// Whether this backend uses HarfBuzz.
    pub(crate) const fn has_harfbuzz(self) -> bool {
        match self {
            Backend::CoreText => false,
        }
    }

    /// Whether this backend uses the browser Canvas font system.
    pub(crate) const fn has_wasm_canvas(self) -> bool {
        match self {
            Backend::CoreText => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_backend_is_coretext() {
        assert_eq!(Backend::active(), Backend::CoreText);
    }

    #[test]
    fn coretext_capabilities_match_upstream_row() {
        let backend = Backend::CoreText;
        assert!(backend.has_coretext());
        assert!(!backend.has_freetype());
        assert!(!backend.has_fontconfig());
        assert!(!backend.has_harfbuzz());
        assert!(!backend.has_wasm_canvas());
    }
}
