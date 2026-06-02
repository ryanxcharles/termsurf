#![allow(dead_code)]
// Cursor style selection is consumed by later renderer slices.

//! Renderer cursor style selection.
//!
//! Faithful port of upstream `renderer/cursor.zig`: the renderer-side cursor
//! `Style` superset, the mapping from a terminal cursor visual style, and the
//! `style()` priority function that decides which cursor style to draw — or
//! none — for the current render state.

use std::os::raw::c_int;

use crate::terminal::cursor::VisualStyle;
use crate::{
    RenderStateScalar, ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR,
    ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK,
    ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK_HOLLOW,
    ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE,
};

/// Available cursor styles the renderer must support. This is a superset of the
/// terminal cursor styles: it adds `BlockHollow` (drawn for an unfocused
/// window) and `Lock` (drawn at a password prompt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Style {
    // Typical cursor input styles.
    Block,
    BlockHollow,
    Bar,
    Underline,

    // Special cursor styles.
    Lock,
}

impl Style {
    /// Create a renderer cursor style from a terminal cursor visual style.
    pub(crate) fn from_terminal(style: VisualStyle) -> Style {
        match style {
            VisualStyle::Bar => Style::Bar,
            VisualStyle::Block => Style::Block,
            VisualStyle::BlockHollow => Style::BlockHollow,
            VisualStyle::Underline => Style::Underline,
        }
    }
}

/// Map the render state's stored `cursor_visual_style` integer back to a
/// terminal `VisualStyle`, using the `ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_*`
/// constants as the single source of truth for the encoding (the inverse of
/// `render_cursor_visual_style`). An unrecognized integer should never occur;
/// it falls back to the default `Block` cursor and is caught in debug builds.
fn visual_style_from_render_int(value: c_int) -> VisualStyle {
    match value {
        ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR => VisualStyle::Bar,
        ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK => VisualStyle::Block,
        ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE => VisualStyle::Underline,
        ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK_HOLLOW => VisualStyle::BlockHollow,
        other => {
            debug_assert!(false, "unknown cursor_visual_style integer: {other}");
            VisualStyle::Block
        }
    }
}

/// Options that influence cursor style independent of terminal state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct StyleOptions {
    pub preedit: bool,
    pub focused: bool,
    pub blink_visible: bool,
}

/// Returns the cursor style to use for the current render state, or `None` if a
/// cursor should not be rendered at all.
///
/// The order of the conditionals below is a priority system: it determines what
/// state overrides cursor visibility and style. It must match upstream
/// `renderer/cursor.zig` exactly.
pub(crate) fn style(state: &RenderStateScalar, opts: StyleOptions) -> Option<Style> {
    // The cursor must be visible in the viewport to be rendered.
    if state.cursor_viewport.is_none() {
        return None;
    }

    // If we are in preedit, then we always show the block cursor. We do this
    // even if the cursor is explicitly not visible because it shows an important
    // editing state to the user.
    if opts.preedit {
        return Some(Style::Block);
    }

    // If we're at a password input it's always a lock.
    if state.cursor_password_input {
        return Some(Style::Lock);
    }

    // If the cursor is explicitly not visible by terminal mode, we don't render.
    if !state.cursor_visible {
        return None;
    }

    // If we're not focused, our cursor is always visible so that we can show the
    // hollow box.
    if !opts.focused {
        return Some(Style::BlockHollow);
    }

    // If the cursor is blinking and our blink state is not visible, then we
    // don't show the cursor.
    if state.cursor_blinking && !opts.blink_visible {
        return None;
    }

    // Otherwise, we use whatever style the terminal wants.
    Some(Style::from_terminal(visual_style_from_render_int(
        state.cursor_visual_style,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{render_state_default, RenderStateCursorViewport};

    fn opts(preedit: bool, focused: bool, blink_visible: bool) -> StyleOptions {
        StyleOptions {
            preedit,
            focused,
            blink_visible,
        }
    }

    /// A render state whose cursor is present in the viewport (the common
    /// precondition for the cursor to render at all).
    fn present_state() -> RenderStateScalar {
        let mut state = render_state_default();
        state.cursor_viewport = Some(RenderStateCursorViewport {
            x: 0,
            y: 0,
            wide_tail: false,
        });
        state
    }

    #[test]
    fn from_terminal_maps_each_visual_style() {
        assert_eq!(Style::from_terminal(VisualStyle::Bar), Style::Bar);
        assert_eq!(Style::from_terminal(VisualStyle::Block), Style::Block);
        assert_eq!(
            Style::from_terminal(VisualStyle::BlockHollow),
            Style::BlockHollow
        );
        assert_eq!(
            Style::from_terminal(VisualStyle::Underline),
            Style::Underline
        );
    }

    #[test]
    fn render_int_round_trips_each_visual_style() {
        assert_eq!(
            visual_style_from_render_int(ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR),
            VisualStyle::Bar
        );
        assert_eq!(
            visual_style_from_render_int(ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK),
            VisualStyle::Block
        );
        assert_eq!(
            visual_style_from_render_int(ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE),
            VisualStyle::Underline
        );
        assert_eq!(
            visual_style_from_render_int(ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK_HOLLOW),
            VisualStyle::BlockHollow
        );
    }

    // Upstream "cursor: default uses configured style".
    #[test]
    fn cursor_default_uses_configured_style() {
        let mut state = present_state();
        state.cursor_visual_style = ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR;
        state.cursor_blinking = true;

        assert_eq!(style(&state, opts(false, true, true)), Some(Style::Bar));
        assert_eq!(
            style(&state, opts(false, false, true)),
            Some(Style::BlockHollow)
        );
        assert_eq!(
            style(&state, opts(false, false, false)),
            Some(Style::BlockHollow)
        );
        assert_eq!(style(&state, opts(false, true, false)), None);
    }

    // Upstream "cursor: blinking disabled".
    #[test]
    fn cursor_blinking_disabled() {
        let mut state = present_state();
        state.cursor_visual_style = ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR;
        state.cursor_blinking = false;

        assert_eq!(style(&state, opts(false, true, true)), Some(Style::Bar));
        assert_eq!(style(&state, opts(false, true, false)), Some(Style::Bar));
        assert_eq!(
            style(&state, opts(false, false, true)),
            Some(Style::BlockHollow)
        );
        assert_eq!(
            style(&state, opts(false, false, false)),
            Some(Style::BlockHollow)
        );
    }

    // Upstream "cursor: explicitly not visible".
    #[test]
    fn cursor_explicitly_not_visible() {
        let mut state = present_state();
        state.cursor_visual_style = ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR;
        state.cursor_visible = false;
        state.cursor_blinking = false;

        assert_eq!(style(&state, opts(false, true, true)), None);
        assert_eq!(style(&state, opts(false, true, false)), None);
        assert_eq!(style(&state, opts(false, false, true)), None);
        assert_eq!(style(&state, opts(false, false, false)), None);
    }

    // Upstream "cursor: always block with preedit" (in-viewport half).
    #[test]
    fn cursor_always_block_with_preedit() {
        let state = present_state();

        assert_eq!(style(&state, opts(true, false, false)), Some(Style::Block));
        assert_eq!(style(&state, opts(true, true, false)), Some(Style::Block));
        assert_eq!(style(&state, opts(true, true, true)), Some(Style::Block));
        assert_eq!(style(&state, opts(true, false, true)), Some(Style::Block));

        // Preedit holds even when the cursor would otherwise be hidden by mode.
        let mut hidden = present_state();
        hidden.cursor_visible = false;
        assert_eq!(style(&hidden, opts(true, true, true)), Some(Style::Block));
    }

    // Password input is a lock; logic-complete but not yet reachable from a real
    // terminal (`cursor_password_input` is hardcoded false in
    // `render_state_from_terminal`), so it is exercised via a constructed state.
    #[test]
    fn cursor_password_input_is_lock() {
        let mut state = present_state();
        state.cursor_password_input = true;

        // Lock takes priority over an explicitly-hidden cursor...
        state.cursor_visible = false;
        assert_eq!(style(&state, opts(false, true, true)), Some(Style::Lock));

        // ...but not over preedit, which is checked first.
        assert_eq!(style(&state, opts(true, true, true)), Some(Style::Block));
    }

    // No viewport => no cursor, ahead of every other condition (including
    // preedit). Not yet reachable from a real terminal (scroll-away viewport
    // nulling is unmodeled), so exercised via a constructed state.
    #[test]
    fn cursor_absent_viewport_is_none() {
        let mut state = render_state_default();
        state.cursor_viewport = None;
        state.cursor_password_input = true;
        state.cursor_visual_style = ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR;

        assert_eq!(style(&state, opts(false, true, true)), None);
        assert_eq!(style(&state, opts(false, false, false)), None);
        assert_eq!(style(&state, opts(true, true, true)), None);
        assert_eq!(style(&state, opts(true, false, false)), None);
    }
}
