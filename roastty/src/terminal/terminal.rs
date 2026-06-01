//! Terminal state.

use super::color;
use super::page_list::{
    CodepointMapEntry, PageListAllocError, PageOutputFormat, PageStringWithPinMap,
};
use super::screen::{Screen, ScreenFormatter, ScreenFormatterContent, ScreenFormatterOptions};
use super::size::CellCountInt;

#[derive(Debug)]
pub(super) struct Terminal {
    screens: TerminalScreens,
}

#[derive(Debug)]
pub(super) struct TerminalScreens {
    active: Screen,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TerminalFormatterOptions<'a> {
    screen: ScreenFormatterOptions<'a>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TerminalFormatter<'a> {
    terminal: &'a Terminal,
    options: TerminalFormatterOptions<'a>,
    content: ScreenFormatterContent,
}

impl Terminal {
    pub(super) fn init(
        cols: CellCountInt,
        rows: CellCountInt,
        max_scrollback_rows: Option<usize>,
    ) -> Result<Self, PageListAllocError> {
        Ok(Self {
            screens: TerminalScreens {
                active: Screen::init(cols, rows, max_scrollback_rows)?,
            },
        })
    }
}

impl<'a> TerminalFormatterOptions<'a> {
    pub(super) const fn new(emit: PageOutputFormat) -> Self {
        Self {
            screen: ScreenFormatterOptions::new(emit),
        }
    }

    pub(super) const fn trim(mut self, trim: bool) -> Self {
        self.screen = self.screen.trim(trim);
        self
    }

    pub(super) const fn unwrap(mut self, unwrap: bool) -> Self {
        self.screen = self.screen.unwrap(unwrap);
        self
    }

    pub(super) const fn palette(mut self, palette: Option<&'a color::Palette>) -> Self {
        self.screen = self.screen.palette(palette);
        self
    }

    pub(super) const fn codepoint_map(
        mut self,
        codepoint_map: Option<&'a [CodepointMapEntry]>,
    ) -> Self {
        self.screen = self.screen.codepoint_map(codepoint_map);
        self
    }
}

impl<'a> TerminalFormatter<'a> {
    pub(super) fn init(terminal: &'a Terminal, options: TerminalFormatterOptions<'a>) -> Self {
        Self {
            terminal,
            options,
            content: ScreenFormatterContent::Selection(None),
        }
    }

    pub(super) const fn with_content(mut self, content: ScreenFormatterContent) -> Self {
        self.content = content;
        self
    }

    pub(super) fn format(self) -> String {
        ScreenFormatter::init(&self.terminal.screens.active, self.options.screen)
            .with_content(self.content)
            .format()
    }

    pub(super) fn format_with_pin_map(self) -> PageStringWithPinMap {
        ScreenFormatter::init(&self.terminal.screens.active, self.options.screen)
            .with_content(self.content)
            .format_with_pin_map()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::color;
    use crate::terminal::page_list::{CodepointReplacement, Pin};
    use crate::terminal::selection;
    use crate::terminal::style;

    fn terminal_with_lines(lines: &[&str]) -> Terminal {
        let rows = lines.len().max(1);
        let cols = lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(1)
            .max(1);
        let mut terminal = Terminal::init(cols.try_into().unwrap(), rows.try_into().unwrap(), None)
            .expect("test terminal must initialize");
        terminal.screens.active.set_text_lines_for_tests(lines);
        terminal
    }

    fn active_pin(terminal: &Terminal, x: CellCountInt, y: u32) -> Pin {
        terminal.screens.active.pin_for_tests(x, y)
    }

    fn active_selection(
        terminal: &Terminal,
        start: (CellCountInt, u32),
        end: (CellCountInt, u32),
    ) -> selection::Selection {
        selection::Selection::new(
            active_pin(terminal, start.0, start.1),
            active_pin(terminal, end.0, end.1),
            false,
        )
    }

    fn formatter<'a>(terminal: &'a Terminal, emit: PageOutputFormat) -> TerminalFormatter<'a> {
        TerminalFormatter::init(terminal, TerminalFormatterOptions::new(emit).unwrap(true))
    }

    fn screen_formatter<'a>(terminal: &'a Terminal, emit: PageOutputFormat) -> ScreenFormatter<'a> {
        ScreenFormatter::init(
            &terminal.screens.active,
            ScreenFormatterOptions::new(emit).unwrap(true),
        )
    }

    fn pins(terminal: &Terminal, points: &[(CellCountInt, u32)]) -> Vec<Pin> {
        points
            .iter()
            .map(|&(x, y)| active_pin(terminal, x, y))
            .collect()
    }

    #[test]
    fn terminal_formatter_plain_full_active_screen_single_line() {
        let terminal = terminal_with_lines(&["hello"]);

        assert_eq!(
            formatter(&terminal, PageOutputFormat::Plain).format(),
            "hello"
        );
    }

    #[test]
    fn terminal_formatter_plain_full_active_screen_multiline() {
        let terminal = terminal_with_lines(&["hello", "world"]);

        assert_eq!(
            formatter(&terminal, PageOutputFormat::Plain).format(),
            "hello\nworld"
        );
    }

    #[test]
    fn terminal_formatter_plain_selected_line() {
        let terminal = terminal_with_lines(&["line1", "line2", "line3"]);
        let selection = active_selection(&terminal, (0, 1), (4, 1));

        let actual = formatter(&terminal, PageOutputFormat::Plain)
            .with_content(ScreenFormatterContent::Selection(Some(selection)))
            .format();

        assert_eq!(actual, "line2");
    }

    #[test]
    fn terminal_formatter_no_content_emits_empty_output_and_pin_map() {
        let terminal = terminal_with_lines(&["hello"]);

        let formatter = formatter(&terminal, PageOutputFormat::Plain)
            .with_content(ScreenFormatterContent::None);

        assert_eq!(formatter.format(), "");
        assert_eq!(
            formatter.format_with_pin_map(),
            PageStringWithPinMap {
                text: String::new(),
                pin_map: Vec::new(),
            }
        );
    }

    #[test]
    fn terminal_formatter_vt_content_delegates_to_screen_formatter() {
        let terminal = terminal_with_lines(&["hello", "world"]);

        let terminal_output = formatter(&terminal, PageOutputFormat::Vt).format();
        let screen_output = screen_formatter(&terminal, PageOutputFormat::Vt).format();

        assert_eq!(terminal_output, screen_output);
        assert_eq!(terminal_output, "hello\r\nworld");
    }

    #[test]
    fn terminal_formatter_html_content_delegates_to_screen_formatter() {
        let terminal = terminal_with_lines(&["<hi"]);

        let terminal_output = formatter(&terminal, PageOutputFormat::Html).format();
        let screen_output = screen_formatter(&terminal, PageOutputFormat::Html).format();

        assert_eq!(terminal_output, screen_output);
        assert_eq!(
            terminal_output,
            "<div style=\"font-family: monospace; white-space: pre;\">&lt;hi</div>"
        );
    }

    #[test]
    fn terminal_formatter_codepoint_map_delegates_output_and_pin_map() {
        let terminal = terminal_with_lines(&["ao"]);
        let map = [CodepointMapEntry::new(
            'o' as u32,
            'o' as u32,
            CodepointReplacement::String("<é".to_string()),
        )
        .unwrap()];
        let options =
            TerminalFormatterOptions::new(PageOutputFormat::Html).codepoint_map(Some(&map));

        let terminal_output = TerminalFormatter::init(&terminal, options).format_with_pin_map();
        let screen_output = ScreenFormatter::init(
            &terminal.screens.active,
            ScreenFormatterOptions::new(PageOutputFormat::Html).codepoint_map(Some(&map)),
        )
        .format_with_pin_map();

        assert_eq!(terminal_output, screen_output);
        assert_eq!(
            terminal_output.text,
            "<div style=\"font-family: monospace; white-space: pre;\">a&lt;&#233;</div>"
        );
        assert_eq!(terminal_output.text.len(), terminal_output.pin_map.len());
    }

    #[test]
    fn terminal_formatter_trim_and_palette_delegate_to_screen_formatter() {
        let mut terminal = terminal_with_lines(&["X  "]);
        let styled = style::Style {
            fg_color: style::Color::Palette(1),
            ..style::Style::default()
        };
        terminal
            .screens
            .active
            .set_styled_cell_for_tests(0, 0, 'X', styled);
        let mut palette = color::DEFAULT_PALETTE;
        palette[1] = color::Rgb::new(1, 2, 3);
        let options = TerminalFormatterOptions::new(PageOutputFormat::Html)
            .trim(false)
            .palette(Some(&palette));

        let terminal_output = TerminalFormatter::init(&terminal, options).format();
        let screen_output = ScreenFormatter::init(
            &terminal.screens.active,
            ScreenFormatterOptions::new(PageOutputFormat::Html)
                .trim(false)
                .palette(Some(&palette)),
        )
        .format();

        assert_eq!(terminal_output, screen_output);
        assert!(terminal_output.contains("rgb(1, 2, 3)"));
        assert!(terminal_output.contains("</div>  </div>"));
    }

    #[test]
    fn terminal_formatter_plain_pin_map_single_line() {
        let terminal = terminal_with_lines(&["hello"]);

        let actual = formatter(&terminal, PageOutputFormat::Plain).format_with_pin_map();

        assert_eq!(actual.text, "hello");
        assert_eq!(
            actual.pin_map,
            pins(&terminal, &[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)])
        );
        assert_eq!(actual.text.len(), actual.pin_map.len());
    }

    #[test]
    fn terminal_formatter_plain_pin_map_multiline() {
        let terminal = terminal_with_lines(&["hello", "world"]);

        let actual = formatter(&terminal, PageOutputFormat::Plain).format_with_pin_map();

        assert_eq!(actual.text, "hello\nworld");
        assert_eq!(
            actual.pin_map,
            pins(
                &terminal,
                &[
                    (0, 0),
                    (1, 0),
                    (2, 0),
                    (3, 0),
                    (4, 0),
                    (4, 0),
                    (0, 1),
                    (1, 1),
                    (2, 1),
                    (3, 1),
                    (4, 1)
                ]
            )
        );
        assert_eq!(actual.text.len(), actual.pin_map.len());
    }

    #[test]
    fn terminal_formatter_selected_plain_pin_map() {
        let terminal = terminal_with_lines(&["line1", "line2", "line3"]);
        let selection = active_selection(&terminal, (0, 1), (4, 1));

        let actual = formatter(&terminal, PageOutputFormat::Plain)
            .with_content(ScreenFormatterContent::Selection(Some(selection)))
            .format_with_pin_map();

        assert_eq!(actual.text, "line2");
        assert_eq!(
            actual.pin_map,
            pins(&terminal, &[(0, 1), (1, 1), (2, 1), (3, 1), (4, 1)])
        );
        assert_eq!(actual.text.len(), actual.pin_map.len());
    }

    #[test]
    fn terminal_formatter_vt_and_html_pin_maps_delegate_to_screen_formatter() {
        let terminal = terminal_with_lines(&["<é"]);

        for emit in [PageOutputFormat::Vt, PageOutputFormat::Html] {
            let terminal_output = formatter(&terminal, emit).format_with_pin_map();
            let screen_output = screen_formatter(&terminal, emit).format_with_pin_map();

            assert_eq!(terminal_output, screen_output);
            assert_eq!(terminal_output.text.len(), terminal_output.pin_map.len());
        }
    }

    #[test]
    fn terminal_formatter_default_path_does_not_emit_screen_extras() {
        let mut terminal = terminal_with_lines(&["hi"]);
        terminal.screens.active.set_cursor_position_for_tests(4, 2);
        terminal
            .screens
            .active
            .set_cursor_style_for_tests(style::Style {
                flags: style::Flags {
                    bold: true,
                    ..style::Flags::default()
                },
                ..style::Style::default()
            });

        let terminal_output = formatter(&terminal, PageOutputFormat::Vt).format();
        let screen_output = screen_formatter(&terminal, PageOutputFormat::Vt).format();

        assert_eq!(terminal_output, screen_output);
        assert_eq!(terminal_output, "hi");
    }

    #[test]
    fn terminal_formatter_invalid_or_garbage_selection_returns_empty_output_and_map() {
        let terminal = terminal_with_lines(&["hello"]);
        let other = terminal_with_lines(&["other"]);
        let valid = active_pin(&terminal, 0, 0);
        let invalid = active_pin(&other, 0, 0);
        let mut garbage = valid;
        garbage.mark_garbage_for_tests();

        for selection in [
            selection::Selection::new(invalid, valid, false),
            selection::Selection::new(valid, invalid, false),
            selection::Selection::new(garbage, valid, false),
            selection::Selection::new(valid, garbage, false),
        ] {
            let actual = formatter(&terminal, PageOutputFormat::Plain)
                .with_content(ScreenFormatterContent::Selection(Some(selection)))
                .format_with_pin_map();
            assert_eq!(
                actual,
                PageStringWithPinMap {
                    text: String::new(),
                    pin_map: Vec::new(),
                }
            );
        }
    }
}
