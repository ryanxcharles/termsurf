use super::color::Rgb;
use super::size::Offset;
use super::style;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Row(u64);

impl Row {
    const CELLS_SHIFT: u32 = 0;
    const CELLS_MASK: u64 = u32::MAX as u64;
    const WRAP_SHIFT: u32 = 32;
    const WRAP_CONTINUATION_SHIFT: u32 = 33;
    const GRAPHEME_SHIFT: u32 = 34;
    const STYLED_SHIFT: u32 = 35;
    const HYPERLINK_SHIFT: u32 = 36;
    const SEMANTIC_PROMPT_SHIFT: u32 = 37;
    const SEMANTIC_PROMPT_MASK: u64 = 0b11;
    const KITTY_VIRTUAL_PLACEHOLDER_SHIFT: u32 = 39;
    const DIRTY_SHIFT: u32 = 40;

    pub(super) const fn cval(self) -> u64 {
        self.0
    }

    pub(super) const fn cells(self) -> Offset<Cell> {
        Offset::new(((self.0 >> Self::CELLS_SHIFT) & Self::CELLS_MASK) as u32)
    }

    pub(super) fn set_cells(&mut self, offset: Offset<Cell>) {
        self.set_bits(Self::CELLS_SHIFT, Self::CELLS_MASK, offset.offset() as u64);
    }

    pub(super) const fn wrap(self) -> bool {
        self.bit(Self::WRAP_SHIFT)
    }

    pub(super) fn set_wrap(&mut self, value: bool) {
        self.set_bit(Self::WRAP_SHIFT, value);
    }

    pub(super) const fn wrap_continuation(self) -> bool {
        self.bit(Self::WRAP_CONTINUATION_SHIFT)
    }

    pub(super) fn set_wrap_continuation(&mut self, value: bool) {
        self.set_bit(Self::WRAP_CONTINUATION_SHIFT, value);
    }

    pub(super) const fn grapheme(self) -> bool {
        self.bit(Self::GRAPHEME_SHIFT)
    }

    pub(super) fn set_grapheme(&mut self, value: bool) {
        self.set_bit(Self::GRAPHEME_SHIFT, value);
    }

    pub(super) const fn styled(self) -> bool {
        self.bit(Self::STYLED_SHIFT)
    }

    pub(super) fn set_styled(&mut self, value: bool) {
        self.set_bit(Self::STYLED_SHIFT, value);
    }

    pub(super) const fn hyperlink(self) -> bool {
        self.bit(Self::HYPERLINK_SHIFT)
    }

    pub(super) fn set_hyperlink(&mut self, value: bool) {
        self.set_bit(Self::HYPERLINK_SHIFT, value);
    }

    pub(super) const fn semantic_prompt(self) -> SemanticPrompt {
        SemanticPrompt::from_bits(
            ((self.0 >> Self::SEMANTIC_PROMPT_SHIFT) & Self::SEMANTIC_PROMPT_MASK) as u8,
        )
    }

    pub(super) fn set_semantic_prompt(&mut self, value: SemanticPrompt) {
        self.set_bits(
            Self::SEMANTIC_PROMPT_SHIFT,
            Self::SEMANTIC_PROMPT_MASK,
            value as u64,
        );
    }

    pub(super) const fn kitty_virtual_placeholder(self) -> bool {
        self.bit(Self::KITTY_VIRTUAL_PLACEHOLDER_SHIFT)
    }

    pub(super) fn set_kitty_virtual_placeholder(&mut self, value: bool) {
        self.set_bit(Self::KITTY_VIRTUAL_PLACEHOLDER_SHIFT, value);
    }

    pub(super) const fn dirty(self) -> bool {
        self.bit(Self::DIRTY_SHIFT)
    }

    pub(super) fn set_dirty(&mut self, value: bool) {
        self.set_bit(Self::DIRTY_SHIFT, value);
    }

    pub(super) const fn managed_memory(self) -> bool {
        self.styled() || self.hyperlink() || self.grapheme()
    }

    const fn bit(self, shift: u32) -> bool {
        ((self.0 >> shift) & 1) == 1
    }

    fn set_bit(&mut self, shift: u32, value: bool) {
        if value {
            self.0 |= 1_u64 << shift;
        } else {
            self.0 &= !(1_u64 << shift);
        }
    }

    fn set_bits(&mut self, shift: u32, mask: u64, value: u64) {
        assert_eq!(value & !mask, 0);
        self.0 = (self.0 & !(mask << shift)) | (value << shift);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SemanticPrompt {
    #[default]
    None = 0,
    Prompt = 1,
    PromptContinuation = 2,
}

impl SemanticPrompt {
    const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::None,
            1 => Self::Prompt,
            2 => Self::PromptContinuation,
            _ => panic!("invalid semantic prompt bits"),
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Cell(u64);

impl Cell {
    const CONTENT_TAG_SHIFT: u32 = 0;
    const CONTENT_TAG_MASK: u64 = 0b11;
    const CONTENT_SHIFT: u32 = 2;
    const CONTENT_MASK: u64 = 0x00ff_ffff;
    const CODEPOINT_MASK: u64 = 0x001f_ffff;
    const STYLE_ID_SHIFT: u32 = 26;
    const STYLE_ID_MASK: u64 = u16::MAX as u64;
    const WIDE_SHIFT: u32 = 42;
    const WIDE_MASK: u64 = 0b11;
    const PROTECTED_SHIFT: u32 = 44;
    const HYPERLINK_SHIFT: u32 = 45;
    const SEMANTIC_CONTENT_SHIFT: u32 = 46;
    const SEMANTIC_CONTENT_MASK: u64 = 0b11;

    pub(super) fn init(codepoint: u32) -> Self {
        assert!(codepoint <= 0x10ffff);

        let mut cell = Self::default();
        cell.set_content_tag(ContentTag::Codepoint);
        cell.set_content(codepoint as u64);
        cell
    }

    pub(super) fn bg_palette(index: u8) -> Self {
        let mut cell = Self::default();
        cell.set_content_tag(ContentTag::BgColorPalette);
        cell.set_content(index as u64);
        cell
    }

    pub(super) fn bg_rgb(rgb: Rgb) -> Self {
        let mut cell = Self::default();
        cell.set_content_tag(ContentTag::BgColorRgb);
        cell.set_content(rgb_to_bits(rgb));
        cell
    }

    pub(super) const fn cval(self) -> u64 {
        self.0
    }

    pub(super) const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub(super) const fn content_tag(self) -> ContentTag {
        ContentTag::from_bits(((self.0 >> Self::CONTENT_TAG_SHIFT) & Self::CONTENT_TAG_MASK) as u8)
    }

    pub(super) fn set_content_tag(&mut self, value: ContentTag) {
        self.set_bits(
            Self::CONTENT_TAG_SHIFT,
            Self::CONTENT_TAG_MASK,
            value as u64,
        );
    }

    pub(super) const fn codepoint(self) -> u32 {
        match self.content_tag() {
            ContentTag::Codepoint | ContentTag::CodepointGrapheme => {
                ((self.0 >> Self::CONTENT_SHIFT) & Self::CODEPOINT_MASK) as u32
            }
            ContentTag::BgColorPalette | ContentTag::BgColorRgb => 0,
        }
    }

    pub(super) const fn color_palette(self) -> u8 {
        ((self.0 >> Self::CONTENT_SHIFT) & 0xff) as u8
    }

    pub(super) const fn color_rgb(self) -> Rgb {
        bits_to_rgb((self.0 >> Self::CONTENT_SHIFT) & Self::CONTENT_MASK)
    }

    pub(super) const fn style_id(self) -> style::Id {
        ((self.0 >> Self::STYLE_ID_SHIFT) & Self::STYLE_ID_MASK) as style::Id
    }

    pub(super) fn set_style_id(&mut self, value: style::Id) {
        self.set_bits(Self::STYLE_ID_SHIFT, Self::STYLE_ID_MASK, value as u64);
    }

    pub(super) const fn wide(self) -> Wide {
        Wide::from_bits(((self.0 >> Self::WIDE_SHIFT) & Self::WIDE_MASK) as u8)
    }

    pub(super) fn set_wide(&mut self, value: Wide) {
        self.set_bits(Self::WIDE_SHIFT, Self::WIDE_MASK, value as u64);
    }

    pub(super) const fn protected(self) -> bool {
        self.bit(Self::PROTECTED_SHIFT)
    }

    pub(super) fn set_protected(&mut self, value: bool) {
        self.set_bit(Self::PROTECTED_SHIFT, value);
    }

    pub(super) const fn hyperlink(self) -> bool {
        self.bit(Self::HYPERLINK_SHIFT)
    }

    pub(super) fn set_hyperlink(&mut self, value: bool) {
        self.set_bit(Self::HYPERLINK_SHIFT, value);
    }

    pub(super) const fn semantic_content(self) -> SemanticContent {
        SemanticContent::from_bits(
            ((self.0 >> Self::SEMANTIC_CONTENT_SHIFT) & Self::SEMANTIC_CONTENT_MASK) as u8,
        )
    }

    pub(super) fn set_semantic_content(&mut self, value: SemanticContent) {
        self.set_bits(
            Self::SEMANTIC_CONTENT_SHIFT,
            Self::SEMANTIC_CONTENT_MASK,
            value as u64,
        );
    }

    pub(super) const fn has_text(self) -> bool {
        match self.content_tag() {
            ContentTag::Codepoint | ContentTag::CodepointGrapheme => self.codepoint() != 0,
            ContentTag::BgColorPalette | ContentTag::BgColorRgb => false,
        }
    }

    pub(super) const fn grid_width(self) -> u8 {
        match self.wide() {
            Wide::Narrow | Wide::SpacerHead | Wide::SpacerTail => 1,
            Wide::Wide => 2,
        }
    }

    pub(super) const fn has_styling(self) -> bool {
        self.style_id() != style::DEFAULT_ID
    }

    pub(super) const fn is_empty(self) -> bool {
        match self.content_tag() {
            ContentTag::Codepoint | ContentTag::CodepointGrapheme => {
                !self.has_text() && matches!(self.wide(), Wide::Narrow)
            }
            ContentTag::BgColorPalette | ContentTag::BgColorRgb => false,
        }
    }

    pub(super) const fn has_grapheme(self) -> bool {
        matches!(self.content_tag(), ContentTag::CodepointGrapheme)
    }

    pub(super) fn has_text_any(cells: &[Cell]) -> bool {
        cells.iter().any(|cell| cell.has_text())
    }

    fn set_content(&mut self, value: u64) {
        self.set_bits(Self::CONTENT_SHIFT, Self::CONTENT_MASK, value);
    }

    const fn bit(self, shift: u32) -> bool {
        ((self.0 >> shift) & 1) == 1
    }

    fn set_bit(&mut self, shift: u32, value: bool) {
        if value {
            self.0 |= 1_u64 << shift;
        } else {
            self.0 &= !(1_u64 << shift);
        }
    }

    fn set_bits(&mut self, shift: u32, mask: u64, value: u64) {
        assert_eq!(value & !mask, 0);
        self.0 = (self.0 & !(mask << shift)) | (value << shift);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ContentTag {
    #[default]
    Codepoint = 0,
    CodepointGrapheme = 1,
    BgColorPalette = 2,
    BgColorRgb = 3,
}

impl ContentTag {
    const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Codepoint,
            1 => Self::CodepointGrapheme,
            2 => Self::BgColorPalette,
            3 => Self::BgColorRgb,
            _ => panic!("invalid content tag bits"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Wide {
    #[default]
    Narrow = 0,
    Wide = 1,
    SpacerTail = 2,
    SpacerHead = 3,
}

impl Wide {
    const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Narrow,
            1 => Self::Wide,
            2 => Self::SpacerTail,
            3 => Self::SpacerHead,
            _ => panic!("invalid wide bits"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SemanticContent {
    #[default]
    Output = 0,
    Input = 1,
    Prompt = 2,
}

impl SemanticContent {
    const fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Output,
            1 => Self::Input,
            2 => Self::Prompt,
            _ => panic!("invalid semantic content bits"),
        }
    }
}

const fn rgb_to_bits(rgb: Rgb) -> u64 {
    rgb.r as u64 | ((rgb.g as u64) << 8) | ((rgb.b as u64) << 16)
}

const fn bits_to_rgb(bits: u64) -> Rgb {
    Rgb::new(bits as u8, (bits >> 8) as u8, (bits >> 16) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn row_layout() {
        assert_eq!(size_of::<Row>(), 8);
        assert_eq!(align_of::<Row>(), align_of::<u64>());
        assert_eq!(Row::default().cval(), 0);
    }

    #[test]
    fn row_raw_fields() {
        let mut row = Row::default();
        row.set_cells(Offset::new(0x1234_5678));
        assert_eq!(row.cells().offset(), 0x1234_5678);
        assert_eq!(row.cval(), 0x1234_5678);

        let bool_cases: [(fn(&mut Row, bool), u64); 7] = [
            (Row::set_wrap, 1_u64 << 32),
            (Row::set_wrap_continuation, 1_u64 << 33),
            (Row::set_grapheme, 1_u64 << 34),
            (Row::set_styled, 1_u64 << 35),
            (Row::set_hyperlink, 1_u64 << 36),
            (Row::set_kitty_virtual_placeholder, 1_u64 << 39),
            (Row::set_dirty, 1_u64 << 40),
        ];

        for (set, expected) in bool_cases {
            let mut row = Row::default();
            set(&mut row, true);
            assert_eq!(row.cval(), expected);
        }
    }

    #[test]
    fn row_semantic_prompt_raw_values() {
        let cases = [
            (SemanticPrompt::None, 0),
            (SemanticPrompt::Prompt, 1_u64 << 37),
            (SemanticPrompt::PromptContinuation, 2_u64 << 37),
        ];

        for (value, expected) in cases {
            let mut row = Row::default();
            row.set_semantic_prompt(value);
            assert_eq!(row.semantic_prompt(), value);
            assert_eq!(row.cval(), expected);
        }
    }

    #[test]
    fn row_managed_memory() {
        assert!(!Row::default().managed_memory());

        let mut row = Row::default();
        row.set_styled(true);
        assert!(row.managed_memory());

        let mut row = Row::default();
        row.set_hyperlink(true);
        assert!(row.managed_memory());

        let mut row = Row::default();
        row.set_grapheme(true);
        assert!(row.managed_memory());
    }

    #[test]
    fn cell_layout() {
        assert_eq!(size_of::<Cell>(), 8);
        assert_eq!(align_of::<Cell>(), align_of::<u64>());
    }

    #[test]
    fn cell_is_zero_by_default() {
        let cell = Cell::init(0);
        assert_eq!(cell.cval(), 0);
        assert!(cell.is_zero());
        assert_eq!(cell.semantic_content(), SemanticContent::Output);
    }

    #[test]
    fn cell_raw_content_fields() {
        assert_eq!(Cell::init('A' as u32).cval(), 0x41 << 2);
        assert_eq!(Cell::init('A' as u32).codepoint(), 'A' as u32);
        assert_eq!(Cell::bg_palette(7).cval(), 2 | (7 << 2));
        assert_eq!(Cell::bg_palette(7).color_palette(), 7);
        assert_eq!(
            Cell::bg_rgb(Rgb::new(1, 2, 3)).cval(),
            3 | ((1 | (2 << 8) | (3 << 16)) << 2)
        );
        assert_eq!(
            Cell::bg_rgb(Rgb::new(1, 2, 3)).color_rgb(),
            Rgb::new(1, 2, 3)
        );
    }

    #[test]
    fn cell_raw_style_and_flag_fields() {
        let mut cell = Cell::default();
        cell.set_style_id(1);
        assert_eq!(cell.style_id(), 1);
        assert_eq!(cell.cval(), 1_u64 << 26);

        let mut cell = Cell::default();
        cell.set_protected(true);
        assert!(cell.protected());
        assert_eq!(cell.cval(), 1_u64 << 44);

        let mut cell = Cell::default();
        cell.set_hyperlink(true);
        assert!(cell.hyperlink());
        assert_eq!(cell.cval(), 1_u64 << 45);
    }

    #[test]
    fn cell_raw_wide_values() {
        let cases = [
            (Wide::Narrow, 0),
            (Wide::Wide, 1_u64 << 42),
            (Wide::SpacerTail, 2_u64 << 42),
            (Wide::SpacerHead, 3_u64 << 42),
        ];

        for (value, expected) in cases {
            let mut cell = Cell::default();
            cell.set_wide(value);
            assert_eq!(cell.wide(), value);
            assert_eq!(cell.cval(), expected);
        }
    }

    #[test]
    fn cell_raw_content_tag_values() {
        let mut cell = Cell::default();
        cell.set_content_tag(ContentTag::Codepoint);
        assert_eq!(cell.content_tag(), ContentTag::Codepoint);
        assert_eq!(cell.cval(), 0);

        let mut cell = Cell::default();
        cell.set_content_tag(ContentTag::CodepointGrapheme);
        assert_eq!(cell.content_tag(), ContentTag::CodepointGrapheme);
        assert_eq!(cell.cval(), 1);

        assert_eq!(
            Cell::bg_palette(0).content_tag(),
            ContentTag::BgColorPalette
        );
        assert_eq!(Cell::bg_palette(0).cval(), 2);
        assert_eq!(
            Cell::bg_rgb(Rgb::new(0, 0, 0)).content_tag(),
            ContentTag::BgColorRgb
        );
        assert_eq!(Cell::bg_rgb(Rgb::new(0, 0, 0)).cval(), 3);
    }

    #[test]
    fn cell_raw_semantic_content_values() {
        let cases = [
            (SemanticContent::Output, 0),
            (SemanticContent::Input, 1_u64 << 46),
            (SemanticContent::Prompt, 2_u64 << 46),
        ];

        for (value, expected) in cases {
            let mut cell = Cell::default();
            cell.set_semantic_content(value);
            assert_eq!(cell.semantic_content(), value);
            assert_eq!(cell.cval(), expected);
        }
    }

    #[test]
    fn cell_helpers() {
        assert!(!Cell::init(0).has_text());
        assert!(Cell::init('x' as u32).has_text());
        assert_eq!(Cell::init('x' as u32).codepoint(), 'x' as u32);
        assert_eq!(Cell::bg_palette(1).codepoint(), 0);
        assert_eq!(Cell::bg_rgb(Rgb::new(1, 2, 3)).codepoint(), 0);

        let mut cell = Cell::init('x' as u32);
        assert_eq!(cell.grid_width(), 1);
        cell.set_wide(Wide::Wide);
        assert_eq!(cell.grid_width(), 2);
        cell.set_wide(Wide::SpacerTail);
        assert_eq!(cell.grid_width(), 1);
        cell.set_wide(Wide::SpacerHead);
        assert_eq!(cell.grid_width(), 1);

        assert!(!Cell::init(0).has_styling());
        let mut styled = Cell::init(0);
        styled.set_style_id(1);
        assert!(styled.has_styling());

        assert!(Cell::init(0).is_empty());
        assert!(!Cell::init('x' as u32).is_empty());
        let mut spacer = Cell::init(0);
        spacer.set_wide(Wide::SpacerTail);
        assert!(!spacer.is_empty());
        assert!(!Cell::bg_palette(1).is_empty());
    }

    #[test]
    fn cell_grapheme_and_has_text_any() {
        let mut grapheme = Cell::init('x' as u32);
        grapheme.set_content_tag(ContentTag::CodepointGrapheme);
        assert!(grapheme.has_grapheme());
        assert!(grapheme.has_text());

        assert!(!Cell::has_text_any(&[Cell::init(0), Cell::bg_palette(1)]));
        assert!(Cell::has_text_any(&[
            Cell::init(0),
            Cell::bg_palette(1),
            Cell::init('x' as u32),
        ]));
    }

    #[test]
    #[should_panic(expected = "assertion failed: codepoint <= 0x10ffff")]
    fn cell_rejects_invalid_codepoint() {
        let _ = Cell::init(0x11_0000);
    }
}
