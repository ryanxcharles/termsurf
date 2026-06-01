//! Mouse-related terminal state.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum MouseEventMode {
    #[default]
    None,
    X10,
    Normal,
    Button,
    Any,
}

impl MouseEventMode {
    pub(super) const fn sends_motion(self) -> bool {
        matches!(self, Self::Button | Self::Any)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum MouseFormat {
    #[default]
    X10,
    Utf8,
    Sgr,
    Urxvt,
    SgrPixels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseAction {
    Press,
    Release,
    Motion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MouseButton {
    Unknown,
    Left,
    Right,
    Middle,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Eleven,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MouseMods {
    pub(crate) shift: bool,
    pub(crate) alt: bool,
    pub(crate) ctrl: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MouseShape {
    Default,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    AllScroll,
    ColResize,
    RowResize,
    NResize,
    EResize,
    SResize,
    WResize,
    NeResize,
    NwResize,
    SeResize,
    SwResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ZoomIn,
    ZoomOut,
}

impl MouseShape {
    pub(super) fn parse(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"default" | b"left_ptr" => Some(Self::Default),
            b"context-menu" => Some(Self::ContextMenu),
            b"help" | b"question_arrow" => Some(Self::Help),
            b"pointer" | b"hand" => Some(Self::Pointer),
            b"progress" | b"left_ptr_watch" => Some(Self::Progress),
            b"wait" | b"watch" => Some(Self::Wait),
            b"cell" => Some(Self::Cell),
            b"crosshair" | b"cross" => Some(Self::Crosshair),
            b"text" | b"xterm" => Some(Self::Text),
            b"vertical-text" => Some(Self::VerticalText),
            b"alias" | b"dnd-link" => Some(Self::Alias),
            b"copy" | b"dnd-copy" => Some(Self::Copy),
            b"move" | b"dnd-move" => Some(Self::Move),
            b"no-drop" | b"dnd-no-drop" => Some(Self::NoDrop),
            b"not-allowed" | b"crossed_circle" => Some(Self::NotAllowed),
            b"grab" | b"hand1" => Some(Self::Grab),
            b"grabbing" => Some(Self::Grabbing),
            b"all-scroll" | b"fleur" => Some(Self::AllScroll),
            b"col-resize" => Some(Self::ColResize),
            b"row-resize" => Some(Self::RowResize),
            b"n-resize" | b"top_side" => Some(Self::NResize),
            b"e-resize" | b"right_side" => Some(Self::EResize),
            b"s-resize" | b"bottom_side" => Some(Self::SResize),
            b"w-resize" | b"left_side" => Some(Self::WResize),
            b"ne-resize" | b"top_right_corner" => Some(Self::NeResize),
            b"nw-resize" | b"top_left_corner" => Some(Self::NwResize),
            b"se-resize" | b"bottom_right_corner" => Some(Self::SeResize),
            b"sw-resize" | b"bottom_left_corner" => Some(Self::SwResize),
            b"ew-resize" => Some(Self::EwResize),
            b"ns-resize" => Some(Self::NsResize),
            b"nesw-resize" => Some(Self::NeswResize),
            b"nwse-resize" => Some(Self::NwseResize),
            b"zoom-in" => Some(Self::ZoomIn),
            b"zoom-out" => Some(Self::ZoomOut),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MouseShape;

    #[test]
    fn mouse_shape_parse_w3c_names() {
        let cases = [
            (b"default".as_slice(), MouseShape::Default),
            (b"context-menu".as_slice(), MouseShape::ContextMenu),
            (b"help".as_slice(), MouseShape::Help),
            (b"pointer".as_slice(), MouseShape::Pointer),
            (b"progress".as_slice(), MouseShape::Progress),
            (b"wait".as_slice(), MouseShape::Wait),
            (b"cell".as_slice(), MouseShape::Cell),
            (b"crosshair".as_slice(), MouseShape::Crosshair),
            (b"text".as_slice(), MouseShape::Text),
            (b"vertical-text".as_slice(), MouseShape::VerticalText),
            (b"alias".as_slice(), MouseShape::Alias),
            (b"copy".as_slice(), MouseShape::Copy),
            (b"move".as_slice(), MouseShape::Move),
            (b"no-drop".as_slice(), MouseShape::NoDrop),
            (b"not-allowed".as_slice(), MouseShape::NotAllowed),
            (b"grab".as_slice(), MouseShape::Grab),
            (b"grabbing".as_slice(), MouseShape::Grabbing),
            (b"all-scroll".as_slice(), MouseShape::AllScroll),
            (b"col-resize".as_slice(), MouseShape::ColResize),
            (b"row-resize".as_slice(), MouseShape::RowResize),
            (b"n-resize".as_slice(), MouseShape::NResize),
            (b"e-resize".as_slice(), MouseShape::EResize),
            (b"s-resize".as_slice(), MouseShape::SResize),
            (b"w-resize".as_slice(), MouseShape::WResize),
            (b"ne-resize".as_slice(), MouseShape::NeResize),
            (b"nw-resize".as_slice(), MouseShape::NwResize),
            (b"se-resize".as_slice(), MouseShape::SeResize),
            (b"sw-resize".as_slice(), MouseShape::SwResize),
            (b"ew-resize".as_slice(), MouseShape::EwResize),
            (b"ns-resize".as_slice(), MouseShape::NsResize),
            (b"nesw-resize".as_slice(), MouseShape::NeswResize),
            (b"nwse-resize".as_slice(), MouseShape::NwseResize),
            (b"zoom-in".as_slice(), MouseShape::ZoomIn),
            (b"zoom-out".as_slice(), MouseShape::ZoomOut),
        ];

        for (input, expected) in cases {
            assert_eq!(MouseShape::parse(input), Some(expected));
        }
    }

    #[test]
    fn mouse_shape_parse_xterm_and_foot_aliases() {
        let cases = [
            (b"left_ptr".as_slice(), MouseShape::Default),
            (b"question_arrow".as_slice(), MouseShape::Help),
            (b"hand".as_slice(), MouseShape::Pointer),
            (b"left_ptr_watch".as_slice(), MouseShape::Progress),
            (b"watch".as_slice(), MouseShape::Wait),
            (b"cross".as_slice(), MouseShape::Crosshair),
            (b"xterm".as_slice(), MouseShape::Text),
            (b"dnd-link".as_slice(), MouseShape::Alias),
            (b"dnd-copy".as_slice(), MouseShape::Copy),
            (b"dnd-move".as_slice(), MouseShape::Move),
            (b"dnd-no-drop".as_slice(), MouseShape::NoDrop),
            (b"crossed_circle".as_slice(), MouseShape::NotAllowed),
            (b"hand1".as_slice(), MouseShape::Grab),
            (b"right_side".as_slice(), MouseShape::EResize),
            (b"top_side".as_slice(), MouseShape::NResize),
            (b"top_right_corner".as_slice(), MouseShape::NeResize),
            (b"top_left_corner".as_slice(), MouseShape::NwResize),
            (b"bottom_side".as_slice(), MouseShape::SResize),
            (b"bottom_right_corner".as_slice(), MouseShape::SeResize),
            (b"bottom_left_corner".as_slice(), MouseShape::SwResize),
            (b"left_side".as_slice(), MouseShape::WResize),
            (b"fleur".as_slice(), MouseShape::AllScroll),
        ];

        for (input, expected) in cases {
            assert_eq!(MouseShape::parse(input), Some(expected));
        }
    }

    #[test]
    fn mouse_shape_parse_is_exact_and_case_sensitive() {
        assert_eq!(MouseShape::parse(b"Pointer"), None);
        assert_eq!(MouseShape::parse(b" pointer"), None);
        assert_eq!(MouseShape::parse(b"pointer "), None);
        assert_eq!(MouseShape::parse(b"not-a-shape"), None);
        assert_eq!(MouseShape::parse(b""), None);
    }
}
