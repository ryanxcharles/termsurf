//! Terminal mouse protocol encoding.

use super::mouse::{MouseAction, MouseButton, MouseEventMode, MouseFormat, MouseMods};
use super::point;
use super::size::CellCountInt;

#[derive(Debug)]
pub(super) struct Options<'a> {
    pub(super) event: MouseEventMode,
    pub(super) format: MouseFormat,
    pub(super) geometry: Geometry,
    pub(super) any_button_pressed: bool,
    pub(super) last_cell: Option<&'a mut Option<point::Coordinate>>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Event {
    pub(super) action: MouseAction,
    pub(super) button: Option<MouseButton>,
    pub(super) mods: MouseMods,
    pub(super) pos: Position,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Position {
    pub(super) x: f32,
    pub(super) y: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Geometry {
    pub(super) screen: PixelSize,
    pub(super) cell: PixelSize,
    pub(super) padding: Padding,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PixelSize {
    pub(super) width: u32,
    pub(super) height: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Padding {
    pub(super) top: u32,
    pub(super) bottom: u32,
    pub(super) right: u32,
    pub(super) left: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PixelPoint {
    x: i32,
    y: i32,
}

impl Geometry {
    fn grid_size(self) -> point::Coordinate {
        let terminal_width = self
            .screen
            .width
            .saturating_sub(self.padding.left + self.padding.right);
        let terminal_height = self
            .screen
            .height
            .saturating_sub(self.padding.top + self.padding.bottom);
        let cell_width = self.cell.width.max(1);
        let cell_height = self.cell.height.max(1);
        let cols = (terminal_width / cell_width).max(1);
        let rows = (terminal_height / cell_height).max(1);

        point::Coordinate::new(
            cols.min(CellCountInt::MAX.into()) as CellCountInt,
            rows.min(u32::MAX),
        )
    }

    fn pos_out_of_viewport(self, pos: Position) -> bool {
        pos.x < 0.0
            || pos.y < 0.0
            || pos.x > self.screen.width as f32
            || pos.y > self.screen.height as f32
    }

    fn pos_to_cell(self, pos: Position) -> point::Coordinate {
        let term_x = pos.x - self.padding.left as f32;
        let term_y = pos.y - self.padding.top as f32;
        let cell_width = self.cell.width.max(1) as f32;
        let cell_height = self.cell.height.max(1) as f32;
        let grid = self.grid_size();

        let col = (term_x.max(0.0) / cell_width).trunc() as u32;
        let row = (term_y.max(0.0) / cell_height).trunc() as u32;

        point::Coordinate::new(
            col.min(u32::from(grid.x.saturating_sub(1))) as CellCountInt,
            row.min(grid.y.saturating_sub(1)),
        )
    }

    fn pos_to_pixels(self, pos: Position) -> PixelPoint {
        PixelPoint {
            x: (pos.x - self.padding.left as f32).round() as i32,
            y: (pos.y - self.padding.top as f32).round() as i32,
        }
    }
}

impl Default for Event {
    fn default() -> Self {
        Self {
            action: MouseAction::Press,
            button: None,
            mods: MouseMods::default(),
            pos: Position::default(),
        }
    }
}

impl<'a> Options<'a> {
    fn update_last_cell(&mut self, cell: point::Coordinate) {
        if let Some(last) = self.last_cell.as_mut() {
            **last = Some(cell);
        }
    }

    fn last_cell_matches(&mut self, cell: point::Coordinate) -> bool {
        self.last_cell
            .as_mut()
            .and_then(|last| **last)
            .is_some_and(|last| last == cell)
    }
}

pub(super) fn encode(event: Event, mut opts: Options<'_>) -> Option<Vec<u8>> {
    if !should_report(event, &opts) {
        return None;
    }

    if event.action != MouseAction::Release && opts.geometry.pos_out_of_viewport(event.pos) {
        if !opts.event.sends_motion() || !opts.any_button_pressed {
            return None;
        }
    }

    let cell = opts.geometry.pos_to_cell(event.pos);
    if event.action == MouseAction::Motion && opts.format != MouseFormat::SgrPixels {
        if opts.last_cell_matches(cell) {
            return None;
        }
    }
    opts.update_last_cell(cell);

    let button_code = button_code(event, &opts)?;
    let mut output = Vec::new();

    match opts.format {
        MouseFormat::X10 => {
            if cell.x > 222 || cell.y > 222 {
                return None;
            }

            output.extend_from_slice(b"\x1b[M");
            output.push(32 + button_code);
            output.push(32 + cell.x as u8 + 1);
            output.push(32 + cell.y as u8 + 1);
        }
        MouseFormat::Utf8 => {
            output.extend_from_slice(b"\x1b[M");
            output.push(32 + button_code);
            append_utf8_codepoint(&mut output, u32::from(cell.x) + 33);
            append_utf8_codepoint(&mut output, cell.y + 33);
        }
        MouseFormat::Sgr => {
            output.extend_from_slice(
                format!(
                    "\x1b[<{};{};{}{}",
                    button_code,
                    cell.x + 1,
                    cell.y + 1,
                    if event.action == MouseAction::Release {
                        'm'
                    } else {
                        'M'
                    }
                )
                .as_bytes(),
            );
        }
        MouseFormat::Urxvt => {
            output.extend_from_slice(
                format!("\x1b[{};{};{}M", 32 + button_code, cell.x + 1, cell.y + 1).as_bytes(),
            );
        }
        MouseFormat::SgrPixels => {
            let pixels = opts.geometry.pos_to_pixels(event.pos);
            output.extend_from_slice(
                format!(
                    "\x1b[<{};{};{}{}",
                    button_code,
                    pixels.x,
                    pixels.y,
                    if event.action == MouseAction::Release {
                        'm'
                    } else {
                        'M'
                    }
                )
                .as_bytes(),
            );
        }
    }

    Some(output)
}

fn should_report(event: Event, opts: &Options<'_>) -> bool {
    match opts.event {
        MouseEventMode::None => false,
        MouseEventMode::X10 => {
            event.action == MouseAction::Press
                && matches!(
                    event.button,
                    Some(MouseButton::Left | MouseButton::Middle | MouseButton::Right)
                )
        }
        MouseEventMode::Normal => event.action != MouseAction::Motion,
        MouseEventMode::Button => event.button.is_some(),
        MouseEventMode::Any => true,
    }
}

fn button_code(event: Event, opts: &Options<'_>) -> Option<u8> {
    let mut acc = match event.button {
        None => 3,
        Some(button)
            if event.action == MouseAction::Release
                && !matches!(opts.format, MouseFormat::Sgr | MouseFormat::SgrPixels) =>
        {
            let _ = button;
            3
        }
        Some(MouseButton::Left) => 0,
        Some(MouseButton::Middle) => 1,
        Some(MouseButton::Right) => 2,
        Some(MouseButton::Four) => 64,
        Some(MouseButton::Five) => 65,
        Some(MouseButton::Six) => 66,
        Some(MouseButton::Seven) => 67,
        Some(MouseButton::Eight) => 128,
        Some(MouseButton::Nine) => 129,
        Some(MouseButton::Unknown | MouseButton::Ten | MouseButton::Eleven) => return None,
    };

    if opts.event != MouseEventMode::X10 {
        if event.mods.shift {
            acc += 4;
        }
        if event.mods.alt {
            acc += 8;
        }
        if event.mods.ctrl {
            acc += 16;
        }
    }

    if event.action == MouseAction::Motion {
        acc += 32;
    }

    Some(acc)
}

fn append_utf8_codepoint(output: &mut Vec<u8>, value: u32) {
    let Some(ch) = char::from_u32(value) else {
        return;
    };
    let mut buf = [0; 4];
    output.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_geometry() -> Geometry {
        Geometry {
            screen: PixelSize {
                width: 1_000,
                height: 1_000,
            },
            cell: PixelSize {
                width: 1,
                height: 1,
            },
            padding: Padding::default(),
        }
    }

    fn encode_string(event: Event, opts: Options<'_>) -> Option<String> {
        encode(event, opts).map(|bytes| String::from_utf8(bytes).unwrap())
    }

    fn opts(event: MouseEventMode, format: MouseFormat) -> Options<'static> {
        Options {
            event,
            format,
            geometry: test_geometry(),
            any_button_pressed: false,
            last_cell: None,
        }
    }

    #[test]
    fn mouse_encode_should_report_none_mode_never_reports() {
        for action in [
            MouseAction::Press,
            MouseAction::Release,
            MouseAction::Motion,
        ] {
            assert_eq!(
                encode(
                    Event {
                        action,
                        button: Some(MouseButton::Left),
                        ..Event::default()
                    },
                    opts(MouseEventMode::None, MouseFormat::Sgr),
                ),
                None
            );
        }
    }

    #[test]
    fn mouse_encode_should_report_x10_reports_only_left_middle_right_press() {
        for button in [MouseButton::Left, MouseButton::Middle, MouseButton::Right] {
            assert!(encode(
                Event {
                    action: MouseAction::Press,
                    button: Some(button),
                    ..Event::default()
                },
                opts(MouseEventMode::X10, MouseFormat::X10),
            )
            .is_some());
        }

        for event in [
            Event {
                action: MouseAction::Release,
                button: Some(MouseButton::Left),
                ..Event::default()
            },
            Event {
                action: MouseAction::Motion,
                button: Some(MouseButton::Left),
                ..Event::default()
            },
            Event {
                action: MouseAction::Press,
                button: Some(MouseButton::Four),
                ..Event::default()
            },
            Event {
                action: MouseAction::Press,
                button: None,
                ..Event::default()
            },
        ] {
            assert_eq!(
                encode(event, opts(MouseEventMode::X10, MouseFormat::X10)),
                None
            );
        }
    }

    #[test]
    fn mouse_encode_should_report_normal_reports_press_release_not_motion() {
        assert!(encode(
            Event {
                action: MouseAction::Press,
                button: Some(MouseButton::Left),
                ..Event::default()
            },
            opts(MouseEventMode::Normal, MouseFormat::Sgr),
        )
        .is_some());
        assert!(encode(
            Event {
                action: MouseAction::Release,
                button: Some(MouseButton::Left),
                ..Event::default()
            },
            opts(MouseEventMode::Normal, MouseFormat::Sgr),
        )
        .is_some());
        assert_eq!(
            encode(
                Event {
                    action: MouseAction::Motion,
                    button: Some(MouseButton::Left),
                    ..Event::default()
                },
                opts(MouseEventMode::Normal, MouseFormat::Sgr),
            ),
            None
        );
    }

    #[test]
    fn mouse_encode_should_report_button_mode_requires_button() {
        for action in [
            MouseAction::Press,
            MouseAction::Release,
            MouseAction::Motion,
        ] {
            assert!(encode(
                Event {
                    action,
                    button: Some(MouseButton::Left),
                    ..Event::default()
                },
                opts(MouseEventMode::Button, MouseFormat::Sgr),
            )
            .is_some());
            assert_eq!(
                encode(
                    Event {
                        action,
                        button: None,
                        ..Event::default()
                    },
                    opts(MouseEventMode::Button, MouseFormat::Sgr),
                ),
                None
            );
        }
    }

    #[test]
    fn mouse_encode_should_report_any_mode_reports_buttonless_motion() {
        assert!(encode(
            Event {
                action: MouseAction::Motion,
                button: None,
                pos: Position { x: 1.0, y: 2.0 },
                ..Event::default()
            },
            opts(MouseEventMode::Any, MouseFormat::Sgr),
        )
        .is_some());
    }

    #[test]
    fn mouse_encode_x10_press_left() {
        assert_eq!(
            encode(
                Event {
                    action: MouseAction::Press,
                    button: Some(MouseButton::Left),
                    mods: MouseMods {
                        shift: true,
                        alt: true,
                        ctrl: true,
                    },
                    ..Event::default()
                },
                opts(MouseEventMode::X10, MouseFormat::X10),
            )
            .unwrap(),
            vec![0x1b, b'[', b'M', 32, 33, 33]
        );
    }

    #[test]
    fn mouse_encode_x10_tracking_ignores_modifiers_but_x10_format_does_not() {
        assert_eq!(
            encode(
                Event {
                    action: MouseAction::Press,
                    button: Some(MouseButton::Left),
                    mods: MouseMods {
                        shift: true,
                        alt: true,
                        ctrl: true,
                    },
                    pos: Position { x: 2.0, y: 3.0 },
                },
                opts(MouseEventMode::X10, MouseFormat::X10),
            )
            .unwrap(),
            vec![0x1b, b'[', b'M', 32, 35, 36]
        );

        assert_eq!(
            encode(
                Event {
                    action: MouseAction::Press,
                    button: Some(MouseButton::Left),
                    mods: MouseMods {
                        shift: true,
                        alt: true,
                        ctrl: true,
                    },
                    pos: Position { x: 2.0, y: 3.0 },
                },
                opts(MouseEventMode::Any, MouseFormat::X10),
            )
            .unwrap(),
            vec![0x1b, b'[', b'M', 60, 35, 36]
        );
    }

    #[test]
    fn mouse_encode_x10_ignores_release() {
        assert_eq!(
            encode(
                Event {
                    action: MouseAction::Release,
                    button: Some(MouseButton::Left),
                    ..Event::default()
                },
                opts(MouseEventMode::X10, MouseFormat::X10),
            ),
            None
        );
    }

    #[test]
    fn mouse_encode_normal_ignores_motion() {
        assert_eq!(
            encode(
                Event {
                    action: MouseAction::Motion,
                    button: Some(MouseButton::Left),
                    ..Event::default()
                },
                opts(MouseEventMode::Normal, MouseFormat::Sgr),
            ),
            None
        );
    }

    #[test]
    fn mouse_encode_button_mode_requires_button_for_motion() {
        assert_eq!(
            encode(
                Event {
                    action: MouseAction::Motion,
                    button: None,
                    ..Event::default()
                },
                opts(MouseEventMode::Button, MouseFormat::Sgr),
            ),
            None
        );
    }

    #[test]
    fn mouse_encode_sgr_release_keeps_button_identity() {
        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Release,
                    button: Some(MouseButton::Right),
                    pos: Position { x: 4.0, y: 5.0 },
                    ..Event::default()
                },
                opts(MouseEventMode::Any, MouseFormat::Sgr),
            )
            .unwrap(),
            "\x1b[<2;5;6m"
        );
    }

    #[test]
    fn mouse_encode_sgr_motion_with_no_button() {
        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Motion,
                    button: None,
                    pos: Position { x: 1.0, y: 2.0 },
                    ..Event::default()
                },
                opts(MouseEventMode::Any, MouseFormat::Sgr),
            )
            .unwrap(),
            "\x1b[<35;2;3M"
        );
    }

    #[test]
    fn mouse_encode_urxvt_with_modifiers() {
        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Press,
                    button: Some(MouseButton::Left),
                    mods: MouseMods {
                        shift: true,
                        alt: true,
                        ctrl: true,
                    },
                    pos: Position { x: 2.0, y: 3.0 },
                },
                opts(MouseEventMode::Any, MouseFormat::Urxvt),
            )
            .unwrap(),
            "\x1b[60;3;4M"
        );
    }

    #[test]
    fn mouse_encode_utf8_encodes_large_coordinates() {
        let out = encode(
            Event {
                action: MouseAction::Press,
                button: Some(MouseButton::Left),
                pos: Position { x: 300.0, y: 400.0 },
                ..Event::default()
            },
            opts(MouseEventMode::Any, MouseFormat::Utf8),
        )
        .unwrap();

        assert_eq!(&out[..4], &[0x1b, b'[', b'M', 32]);
        assert_eq!(std::str::from_utf8(&out[4..]).unwrap(), "ōƱ");
    }

    #[test]
    fn mouse_encode_x10_coordinate_limit() {
        assert_eq!(
            encode(
                Event {
                    action: MouseAction::Press,
                    button: Some(MouseButton::Left),
                    pos: Position { x: 223.0, y: 0.0 },
                    ..Event::default()
                },
                opts(MouseEventMode::Any, MouseFormat::X10),
            ),
            None
        );
    }

    #[test]
    fn mouse_encode_sgr_wheel_button_mappings() {
        for (button, code) in [
            (MouseButton::Four, 64),
            (MouseButton::Five, 65),
            (MouseButton::Six, 66),
            (MouseButton::Seven, 67),
        ] {
            assert_eq!(
                encode_string(
                    Event {
                        action: MouseAction::Press,
                        button: Some(button),
                        ..Event::default()
                    },
                    opts(MouseEventMode::Any, MouseFormat::Sgr),
                )
                .unwrap(),
                format!("\x1b[<{code};1;1M")
            );
        }
    }

    #[test]
    fn mouse_encode_sgr_extended_button_mappings() {
        for (button, code) in [(MouseButton::Eight, 128), (MouseButton::Nine, 129)] {
            assert_eq!(
                encode_string(
                    Event {
                        action: MouseAction::Press,
                        button: Some(button),
                        ..Event::default()
                    },
                    opts(MouseEventMode::Any, MouseFormat::Sgr),
                )
                .unwrap(),
                format!("\x1b[<{code};1;1M")
            );
        }
    }

    #[test]
    fn mouse_encode_urxvt_release_uses_legacy_button_three_encoding() {
        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Release,
                    button: Some(MouseButton::Right),
                    pos: Position { x: 2.0, y: 3.0 },
                    ..Event::default()
                },
                opts(MouseEventMode::Any, MouseFormat::Urxvt),
            )
            .unwrap(),
            "\x1b[35;3;4M"
        );
    }

    #[test]
    fn mouse_encode_unsupported_buttons_are_ignored() {
        for button in [MouseButton::Unknown, MouseButton::Ten, MouseButton::Eleven] {
            assert_eq!(
                encode(
                    Event {
                        action: MouseAction::Press,
                        button: Some(button),
                        pos: Position { x: 1.0, y: 1.0 },
                        ..Event::default()
                    },
                    opts(MouseEventMode::Any, MouseFormat::Sgr),
                ),
                None
            );
        }
    }

    #[test]
    fn mouse_encode_sgr_pixels_uses_terminal_space_cursor_coordinates() {
        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Press,
                    button: Some(MouseButton::Left),
                    pos: Position { x: 10.0, y: 20.0 },
                    ..Event::default()
                },
                opts(MouseEventMode::Any, MouseFormat::SgrPixels),
            )
            .unwrap(),
            "\x1b[<0;10;20M"
        );
    }

    #[test]
    fn mouse_encode_sgr_pixels_respects_padding_and_rounds_without_clamping() {
        let mut options = opts(MouseEventMode::Any, MouseFormat::SgrPixels);
        options.geometry.padding = Padding {
            left: 4,
            top: 7,
            ..Padding::default()
        };

        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Press,
                    button: Some(MouseButton::Left),
                    pos: Position { x: 10.4, y: 20.6 },
                    ..Event::default()
                },
                options,
            )
            .unwrap(),
            "\x1b[<0;6;14M"
        );
    }

    #[test]
    fn mouse_encode_sgr_pixels_release_keeps_button_identity() {
        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Release,
                    button: Some(MouseButton::Right),
                    pos: Position { x: 10.0, y: 20.0 },
                    ..Event::default()
                },
                opts(MouseEventMode::Any, MouseFormat::SgrPixels),
            )
            .unwrap(),
            "\x1b[<2;10;20m"
        );
    }

    #[test]
    fn mouse_encode_position_exactly_at_viewport_boundary_is_encoded_in_final_cell() {
        let options = Options {
            event: MouseEventMode::Any,
            format: MouseFormat::Sgr,
            geometry: Geometry {
                screen: PixelSize {
                    width: 10,
                    height: 10,
                },
                cell: PixelSize {
                    width: 2,
                    height: 2,
                },
                padding: Padding::default(),
            },
            any_button_pressed: false,
            last_cell: None,
        };

        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Press,
                    button: Some(MouseButton::Left),
                    pos: Position { x: 10.0, y: 10.0 },
                    ..Event::default()
                },
                options,
            )
            .unwrap(),
            "\x1b[<0;5;5M"
        );
    }

    #[test]
    fn mouse_encode_cell_formats_respect_padding_and_clamp_to_grid() {
        let options = Options {
            event: MouseEventMode::Any,
            format: MouseFormat::Sgr,
            geometry: Geometry {
                screen: PixelSize {
                    width: 50,
                    height: 50,
                },
                cell: PixelSize {
                    width: 10,
                    height: 10,
                },
                padding: Padding {
                    left: 5,
                    top: 10,
                    right: 5,
                    bottom: 0,
                },
            },
            any_button_pressed: false,
            last_cell: None,
        };

        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Press,
                    button: Some(MouseButton::Left),
                    pos: Position { x: 15.0, y: 20.0 },
                    ..Event::default()
                },
                options,
            )
            .unwrap(),
            "\x1b[<0;2;2M"
        );
    }

    #[test]
    fn mouse_encode_outside_viewport_motion_with_no_pressed_button_is_ignored() {
        assert_eq!(
            encode(
                Event {
                    action: MouseAction::Motion,
                    button: Some(MouseButton::Left),
                    pos: Position { x: -1.0, y: -1.0 },
                    ..Event::default()
                },
                opts(MouseEventMode::Any, MouseFormat::Sgr),
            ),
            None
        );
    }

    #[test]
    fn mouse_encode_outside_viewport_motion_with_pressed_button_is_reported() {
        let mut options = opts(MouseEventMode::Any, MouseFormat::Sgr);
        options.any_button_pressed = true;

        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Motion,
                    button: Some(MouseButton::Left),
                    pos: Position { x: -1.0, y: -1.0 },
                    ..Event::default()
                },
                options,
            )
            .unwrap(),
            "\x1b[<32;1;1M"
        );
    }

    #[test]
    fn mouse_encode_outside_viewport_release_is_reported() {
        assert_eq!(
            encode_string(
                Event {
                    action: MouseAction::Release,
                    button: Some(MouseButton::Right),
                    pos: Position { x: -1.0, y: -1.0 },
                    ..Event::default()
                },
                opts(MouseEventMode::Any, MouseFormat::Sgr),
            )
            .unwrap(),
            "\x1b[<2;1;1m"
        );
    }

    #[test]
    fn mouse_encode_motion_is_deduped_by_last_cell_except_sgr_pixels() {
        let mut last = None;
        let event = Event {
            action: MouseAction::Motion,
            button: Some(MouseButton::Left),
            pos: Position { x: 5.0, y: 6.0 },
            ..Event::default()
        };

        assert!(encode(
            event,
            Options {
                event: MouseEventMode::Any,
                format: MouseFormat::Sgr,
                geometry: test_geometry(),
                any_button_pressed: false,
                last_cell: Some(&mut last),
            },
        )
        .is_some());
        assert_eq!(
            encode(
                event,
                Options {
                    event: MouseEventMode::Any,
                    format: MouseFormat::Sgr,
                    geometry: test_geometry(),
                    any_button_pressed: false,
                    last_cell: Some(&mut last),
                },
            ),
            None
        );
        assert!(encode(
            event,
            Options {
                event: MouseEventMode::Any,
                format: MouseFormat::SgrPixels,
                geometry: test_geometry(),
                any_button_pressed: false,
                last_cell: Some(&mut last),
            },
        )
        .is_some());
    }
}
