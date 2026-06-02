#![allow(dead_code)]
// Cell codepoint classification is consumed by later renderer slices.

//! Renderer cell codepoint classification.
//!
//! Faithful port of the pure codepoint-classification predicates in upstream
//! `renderer/cell.zig`. The `Contents` cell-render-data builder,
//! `constraintWidth`, and `isSymbol` depend on shader/font/terminal types and a
//! generated Unicode table, and are ported separately.

/// True only for U+2588 FULL BLOCK.
pub(crate) fn is_covering(cp: u32) -> bool {
    cp == 0x2588
}

/// Whether minimum-contrast adjustment should be disabled for a glyph. True for
/// graphics elements such as block elements and Powerline glyphs.
pub(crate) fn no_min_contrast(cp: u32) -> bool {
    is_graphics_element(cp)
}

/// True if the codepoint is used for terminal graphics: box drawing, block
/// elements, legacy computing, or Powerline glyphs.
fn is_graphics_element(cp: u32) -> bool {
    is_box_drawing(cp) || is_block_element(cp) || is_legacy_computing(cp) || is_powerline(cp)
}

/// True if the codepoint is a box drawing character.
fn is_box_drawing(cp: u32) -> bool {
    matches!(cp, 0x2500..=0x257F)
}

/// True if the codepoint is a block element.
fn is_block_element(cp: u32) -> bool {
    matches!(cp, 0x2580..=0x259F)
}

/// True if the codepoint is in a Symbols for Legacy Computing block, including
/// the Unicode 16.0 supplement.
fn is_legacy_computing(cp: u32) -> bool {
    matches!(cp, 0x1FB00..=0x1FBFF | 0x1CC00..=0x1CEBF)
}

/// True if the codepoint is part of the Powerline range.
fn is_powerline(cp: u32) -> bool {
    matches!(cp, 0xE0B0..=0xE0D7)
}

/// Some general spaces, kept to force the font to render as a fixed width.
fn is_space(cp: u32) -> bool {
    matches!(cp, 0x0020 | 0x2002)
}

/// True if the codepoint is "symbol-like". Faithful to upstream's generated
/// `is_symbol` table, whose membership is defined in `uucode_config.zig` as the
/// Private-Use general category plus eight named Unicode blocks. Unicode block
/// membership is range-based (including unassigned codepoints inside a block),
/// so this is byte-for-byte identical to the generated table.
pub(crate) fn is_symbol(cp: u32) -> bool {
    is_private_use(cp)
        || matches!(cp,
            0x2190..=0x21FF      // Arrows
            | 0x2700..=0x27BF    // Dingbats
            | 0x1F600..=0x1F64F  // Emoticons
            | 0x2600..=0x26FF    // Miscellaneous Symbols
            | 0x2460..=0x24FF    // Enclosed Alphanumerics
            | 0x1F100..=0x1F1FF  // Enclosed Alphanumeric Supplement
            | 0x1F300..=0x1F5FF  // Miscellaneous Symbols and Pictographs
            | 0x1F680..=0x1F6FF  // Transport and Map Symbols
        )
}

/// True for the Private-Use general category (`Co`). The supplementary planes
/// stop at `..FFFD`; the last two code points of each plane are noncharacters
/// (`Cn`), not Private-Use.
fn is_private_use(cp: u32) -> bool {
    matches!(cp, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_box_drawing_bounds() {
        assert!(!is_box_drawing(0x24FF));
        assert!(is_box_drawing(0x2500));
        assert!(is_box_drawing(0x257F));
        assert!(!is_box_drawing(0x2580));
    }

    #[test]
    fn is_block_element_bounds() {
        assert!(!is_block_element(0x257F));
        assert!(is_block_element(0x2580));
        assert!(is_block_element(0x259F));
        assert!(!is_block_element(0x25A0));
    }

    #[test]
    fn is_legacy_computing_bounds() {
        assert!(!is_legacy_computing(0x1FAFF));
        assert!(is_legacy_computing(0x1FB00));
        assert!(is_legacy_computing(0x1FBFF));
        assert!(!is_legacy_computing(0x1FC00));

        assert!(!is_legacy_computing(0x1CBFF));
        assert!(is_legacy_computing(0x1CC00));
        assert!(is_legacy_computing(0x1CEBF));
        assert!(!is_legacy_computing(0x1CEC0));
    }

    #[test]
    fn is_powerline_bounds() {
        assert!(!is_powerline(0xE0AF));
        assert!(is_powerline(0xE0B0));
        assert!(is_powerline(0xE0D7));
        assert!(!is_powerline(0xE0D8));
    }

    #[test]
    fn is_graphics_element_covers_each_block() {
        assert!(is_graphics_element(0x2500)); // box drawing
        assert!(is_graphics_element(0x2580)); // block element
        assert!(is_graphics_element(0x1FB00)); // legacy computing
        assert!(is_graphics_element(0x1CC00)); // legacy computing supplement
        assert!(is_graphics_element(0xE0B0)); // powerline
        assert!(!is_graphics_element('a' as u32));
    }

    #[test]
    fn is_covering_only_full_block() {
        assert!(is_covering(0x2588));
        // Both neighbors are still inside the block-element range, proving
        // `is_covering` is U+2588-only and not a range.
        assert!(!is_covering(0x2587));
        assert!(!is_covering(0x2589));
    }

    #[test]
    fn no_min_contrast_matches_graphics() {
        assert!(no_min_contrast(0x2500));
        assert!(!no_min_contrast('a' as u32));
    }

    #[test]
    fn is_space_fixed_width() {
        assert!(is_space(0x0020));
        assert!(is_space(0x2002));
        assert!(!is_space(0x2003));
        assert!(!is_space('a' as u32));
    }

    #[test]
    fn is_symbol_private_use() {
        // BMP Private Use Area.
        assert!(!is_symbol(0xDFFF));
        assert!(is_symbol(0xE000));
        assert!(is_symbol(0xF8FF));
        assert!(!is_symbol(0xF900));

        // Plane 15 Supplementary PUA-A, excluding the plane noncharacters.
        assert!(!is_symbol(0xEFFFF));
        assert!(is_symbol(0xF0000));
        assert!(is_symbol(0xFFFFD));
        assert!(!is_symbol(0xFFFFE));

        // Plane 16 Supplementary PUA-B, excluding the plane noncharacters.
        assert!(is_symbol(0x100000));
        assert!(is_symbol(0x10FFFD));
        assert!(!is_symbol(0x10FFFE));
    }

    #[test]
    fn is_symbol_blocks() {
        // Arrows 0x2190..=0x21FF.
        assert!(!is_symbol(0x218F));
        assert!(is_symbol(0x2190));
        assert!(is_symbol(0x21FF));
        assert!(!is_symbol(0x2200));

        // Dingbats 0x2700..=0x27BF.
        assert!(is_symbol(0x2700));
        assert!(is_symbol(0x27BF));
        assert!(!is_symbol(0x27C0));

        // Emoticons 0x1F600..=0x1F64F.
        assert!(is_symbol(0x1F600));
        assert!(is_symbol(0x1F64F));
        assert!(!is_symbol(0x1F650));

        // Miscellaneous Symbols 0x2600..=0x26FF.
        assert!(!is_symbol(0x25FF));
        assert!(is_symbol(0x2600));
        assert!(is_symbol(0x26FF));

        // Enclosed Alphanumerics 0x2460..=0x24FF.
        assert!(!is_symbol(0x245F));
        assert!(is_symbol(0x2460));
        assert!(is_symbol(0x24FF));
        assert!(!is_symbol(0x2500));

        // Enclosed Alphanumeric Supplement 0x1F100..=0x1F1FF.
        assert!(!is_symbol(0x1F0FF));
        assert!(is_symbol(0x1F100));
        assert!(is_symbol(0x1F1FF));

        // Miscellaneous Symbols and Pictographs 0x1F300..=0x1F5FF.
        assert!(!is_symbol(0x1F2FF));
        assert!(is_symbol(0x1F300));
        assert!(is_symbol(0x1F5FF));

        // Transport and Map Symbols 0x1F680..=0x1F6FF.
        assert!(!is_symbol(0x1F67F));
        assert!(is_symbol(0x1F680));
        assert!(is_symbol(0x1F6FF));
        assert!(!is_symbol(0x1F700));
    }

    #[test]
    fn is_symbol_excludes_general_symbols() {
        // Block-scoped definition: Unicode general symbol categories (e.g. `+`
        // is Sm, `$` is Sc) are not symbols here.
        assert!(!is_symbol('+' as u32));
        assert!(!is_symbol('$' as u32));
        assert!(!is_symbol('a' as u32));
    }
}
