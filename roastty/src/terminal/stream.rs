//! Terminal byte stream decoding.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Print { cp: char },
    LineFeed,
    CarriageReturn,
    Backspace,
    HorizontalTab,
    TabSet,
    TabClearCurrent,
    TabClearAll,
    TabReset,
}

pub(super) trait Handler {
    type Error;

    fn vt(&mut self, action: Action) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EscapeState {
    Ground,
    Escape,
    Csi(CsiState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Stream {
    utf8: Utf8Decoder,
    escape: EscapeState,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Utf8Decoder {
    bytes: [u8; 4],
    len: usize,
    expected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecodeResult {
    cp: Option<char>,
    consumed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CsiState {
    tab: CsiWState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CsiWState {
    NoParams,
    QuestionNoParams,
    Param(u16),
    QuestionParam(u16),
    Invalid,
}

impl Stream {
    pub(super) const fn init() -> Self {
        Self {
            utf8: Utf8Decoder::new(),
            escape: EscapeState::Ground,
        }
    }

    pub(super) fn next_slice<H: Handler>(
        &mut self,
        input: &[u8],
        handler: &mut H,
    ) -> Result<(), H::Error> {
        for &byte in input {
            self.next(byte, handler)?;
        }
        Ok(())
    }

    fn next<H: Handler>(&mut self, byte: u8, handler: &mut H) -> Result<(), H::Error> {
        match self.escape {
            EscapeState::Ground => self.next_ground(byte, handler),
            EscapeState::Escape => self.next_escape(byte, handler),
            EscapeState::Csi(state) => self.next_csi(byte, state, handler),
        }
    }

    fn next_ground<H: Handler>(&mut self, byte: u8, handler: &mut H) -> Result<(), H::Error> {
        if self.utf8.is_pending() {
            let result = self.utf8.next(byte);
            if let Some(cp) = result.cp {
                handler.vt(Action::Print { cp })?;
            }
            if !result.consumed {
                self.next_ground(byte, handler)?;
            }
            return Ok(());
        }

        match byte {
            0x1b => {
                self.escape = EscapeState::Escape;
            }
            0x08 => handler.vt(Action::Backspace)?,
            b'\t' => handler.vt(Action::HorizontalTab)?,
            b'\n' | 0x0b | 0x0c => handler.vt(Action::LineFeed)?,
            b'\r' => handler.vt(Action::CarriageReturn)?,
            0x00..=0x07 | 0x0e..=0x1a | 0x1c..=0x1f | 0x7f => {}
            _ => self.next_utf8(byte, handler)?,
        }
        Ok(())
    }

    fn next_escape<H: Handler>(&mut self, byte: u8, handler: &mut H) -> Result<(), H::Error> {
        match byte {
            b'[' => {
                self.escape = EscapeState::Csi(CsiState::new());
                Ok(())
            }
            b'H' => {
                self.escape = EscapeState::Ground;
                handler.vt(Action::TabSet)
            }
            _ => {
                self.escape = EscapeState::Ground;
                Ok(())
            }
        }
    }

    fn next_csi<H: Handler>(
        &mut self,
        byte: u8,
        mut state: CsiState,
        handler: &mut H,
    ) -> Result<(), H::Error> {
        if (0x40..=0x7e).contains(&byte) {
            self.escape = EscapeState::Ground;
            if byte == b'W' {
                if let Some(action) = state.tab_action() {
                    return handler.vt(action);
                }
            }
            return Ok(());
        }

        state.push(byte);
        self.escape = EscapeState::Csi(state);
        Ok(())
    }

    fn next_utf8<H: Handler>(&mut self, byte: u8, handler: &mut H) -> Result<(), H::Error> {
        let result = self.utf8.next(byte);
        if let Some(cp) = result.cp {
            handler.vt(Action::Print { cp })?;
        }
        debug_assert!(result.consumed);
        Ok(())
    }
}

impl CsiState {
    const fn new() -> Self {
        Self {
            tab: CsiWState::NoParams,
        }
    }

    fn push(&mut self, byte: u8) {
        self.tab.push(byte);
    }

    fn tab_action(&self) -> Option<Action> {
        self.tab.action()
    }
}

impl CsiWState {
    fn push(&mut self, byte: u8) {
        *self = match (*self, byte) {
            (Self::NoParams, b'?') => Self::QuestionNoParams,
            (Self::NoParams, b'0'..=b'9') => Self::Param(u16::from(byte - b'0')),
            (Self::QuestionNoParams, b'0'..=b'9') => Self::QuestionParam(u16::from(byte - b'0')),
            (Self::Param(value), b'0'..=b'9') => value
                .checked_mul(10)
                .and_then(|value| value.checked_add(u16::from(byte - b'0')))
                .map_or(Self::Invalid, Self::Param),
            (Self::QuestionParam(value), b'0'..=b'9') => value
                .checked_mul(10)
                .and_then(|value| value.checked_add(u16::from(byte - b'0')))
                .map_or(Self::Invalid, Self::QuestionParam),
            _ => Self::Invalid,
        };
    }

    fn action(&self) -> Option<Action> {
        match self {
            Self::NoParams | Self::Param(0) => Some(Action::TabSet),
            Self::Param(2) => Some(Action::TabClearCurrent),
            Self::Param(5) => Some(Action::TabClearAll),
            Self::QuestionParam(5) => Some(Action::TabReset),
            Self::QuestionNoParams | Self::Param(_) | Self::QuestionParam(_) | Self::Invalid => {
                None
            }
        }
    }
}

impl Utf8Decoder {
    const fn new() -> Self {
        Self {
            bytes: [0; 4],
            len: 0,
            expected: 0,
        }
    }

    fn reset(&mut self) {
        self.bytes = [0; 4];
        self.len = 0;
        self.expected = 0;
    }

    fn is_pending(&self) -> bool {
        self.len > 0
    }

    fn next(&mut self, byte: u8) -> DecodeResult {
        if self.len == 0 {
            return self.start(byte);
        }

        if !self.accepts_continuation(byte) {
            self.reset();
            return DecodeResult {
                cp: Some(char::REPLACEMENT_CHARACTER),
                consumed: false,
            };
        }

        self.bytes[self.len] = byte;
        self.len += 1;

        if self.len < self.expected {
            return DecodeResult {
                cp: None,
                consumed: true,
            };
        }

        let cp = std::str::from_utf8(&self.bytes[..self.expected])
            .ok()
            .and_then(|value| value.chars().next())
            .unwrap_or(char::REPLACEMENT_CHARACTER);
        self.reset();
        DecodeResult {
            cp: Some(cp),
            consumed: true,
        }
    }

    fn start(&mut self, byte: u8) -> DecodeResult {
        match byte {
            0x00..=0x7f => DecodeResult {
                cp: Some(char::from(byte)),
                consumed: true,
            },
            0xc2..=0xdf => self.start_sequence(byte, 2),
            0xe0..=0xef => self.start_sequence(byte, 3),
            0xf0..=0xf4 => self.start_sequence(byte, 4),
            _ => DecodeResult {
                cp: Some(char::REPLACEMENT_CHARACTER),
                consumed: true,
            },
        }
    }

    fn start_sequence(&mut self, byte: u8, expected: usize) -> DecodeResult {
        self.bytes[0] = byte;
        self.len = 1;
        self.expected = expected;
        DecodeResult {
            cp: None,
            consumed: true,
        }
    }

    fn accepts_continuation(&self, byte: u8) -> bool {
        match (self.bytes[0], self.len) {
            (0xe0, 1) => (0xa0..=0xbf).contains(&byte),
            (0xed, 1) => (0x80..=0x9f).contains(&byte),
            (0xf0, 1) => (0x90..=0xbf).contains(&byte),
            (0xf4, 1) => (0x80..=0x8f).contains(&byte),
            _ => (0x80..=0xbf).contains(&byte),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct RecordingHandler {
        actions: Vec<Action>,
    }

    impl Handler for RecordingHandler {
        type Error = ();

        fn vt(&mut self, action: Action) -> Result<(), Self::Error> {
            self.actions.push(action);
            Ok(())
        }
    }

    fn print_chars(handler: &RecordingHandler) -> Vec<char> {
        handler
            .actions
            .iter()
            .filter_map(|action| match action {
                Action::Print { cp } => Some(*cp),
                Action::LineFeed
                | Action::CarriageReturn
                | Action::Backspace
                | Action::HorizontalTab
                | Action::TabSet
                | Action::TabClearCurrent
                | Action::TabClearAll
                | Action::TabReset => None,
            })
            .collect()
    }

    fn actions(handler: &RecordingHandler) -> &[Action] {
        &handler.actions
    }

    fn next_slice(stream: &mut Stream, handler: &mut RecordingHandler, input: &[u8]) {
        stream.next_slice(input, handler).unwrap();
    }

    #[test]
    fn stream_ascii_dispatches_one_print_action_per_character() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"Hello");

        assert_eq!(print_chars(&handler), vec!['H', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn stream_unicode_scalars_dispatch_correctly() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, "😄✤ÁA".as_bytes());

        assert_eq!(print_chars(&handler), vec!['😄', '✤', 'Á', 'A']);
    }

    #[test]
    fn stream_split_multibyte_scalar_dispatches_after_final_byte() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();
        let bytes = "😄".as_bytes();

        next_slice(&mut stream, &mut handler, &bytes[..2]);
        assert!(handler.actions.is_empty());

        next_slice(&mut stream, &mut handler, &bytes[2..]);
        assert_eq!(print_chars(&handler), vec!['😄']);
    }

    #[test]
    fn stream_invalid_utf8_dispatches_replacement_character() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, &[0xff]);

        assert_eq!(print_chars(&handler), vec![char::REPLACEMENT_CHARACTER]);
    }

    #[test]
    fn stream_partial_invalid_utf8_retries_rejecting_starter_byte() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(
            &mut stream,
            &mut handler,
            b"\xf0\x9f\xf0\x9f\x98\x84\xed\xa0\x80",
        );

        assert_eq!(
            print_chars(&handler),
            vec![
                char::REPLACEMENT_CHARACTER,
                '😄',
                char::REPLACEMENT_CHARACTER,
                char::REPLACEMENT_CHARACTER,
                char::REPLACEMENT_CHARACTER,
            ]
        );
    }

    #[test]
    fn stream_invalid_utf8_retries_rejecting_ascii_byte() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9fA");

        assert_eq!(
            print_chars(&handler),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_incomplete_utf8_held_at_slice_boundary_completes_on_next_slice() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();
        let bytes = "✤".as_bytes();

        next_slice(&mut stream, &mut handler, &bytes[..1]);
        next_slice(&mut stream, &mut handler, &bytes[1..2]);
        assert!(handler.actions.is_empty());

        next_slice(&mut stream, &mut handler, &bytes[2..]);
        assert_eq!(print_chars(&handler), vec!['✤']);
    }

    #[test]
    fn stream_lf_and_cr_dispatch_control_actions() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\nB\rC");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print { cp: 'A' },
                Action::LineFeed,
                Action::Print { cp: 'B' },
                Action::CarriageReturn,
                Action::Print { cp: 'C' },
            ]
        );
    }

    #[test]
    fn stream_backspace_dispatches_control_action() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x08B");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print { cp: 'A' },
                Action::Backspace,
                Action::Print { cp: 'B' },
            ]
        );
    }

    #[test]
    fn stream_horizontal_tab_dispatches_control_action() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\tB");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print { cp: 'A' },
                Action::HorizontalTab,
                Action::Print { cp: 'B' },
            ]
        );
    }

    #[test]
    fn stream_vt_and_ff_dispatch_linefeed_actions() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x0bB\x0cC");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print { cp: 'A' },
                Action::LineFeed,
                Action::Print { cp: 'B' },
                Action::LineFeed,
                Action::Print { cp: 'C' },
            ]
        );
    }

    #[test]
    fn stream_escape_h_dispatches_tab_set_action() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x1bHB");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print { cp: 'A' },
                Action::TabSet,
                Action::Print { cp: 'B' },
            ]
        );
    }

    #[test]
    fn stream_other_c0_controls_do_not_dispatch_print_actions() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x0eB");

        assert_eq!(print_chars(&handler), vec!['A', 'B']);
        assert_eq!(
            actions(&handler),
            &[Action::Print { cp: 'A' }, Action::Print { cp: 'B' }]
        );
    }

    #[test]
    fn stream_raw_c1_bytes_are_handled_by_utf8_decoder() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, &[0x80, b'A']);

        assert_eq!(
            print_chars(&handler),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_lf() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\nA");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print {
                    cp: char::REPLACEMENT_CHARACTER,
                },
                Action::LineFeed,
                Action::Print { cp: 'A' },
            ]
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_vt() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\x0bA");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print {
                    cp: char::REPLACEMENT_CHARACTER,
                },
                Action::LineFeed,
                Action::Print { cp: 'A' },
            ]
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_ff() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\x0cA");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print {
                    cp: char::REPLACEMENT_CHARACTER,
                },
                Action::LineFeed,
                Action::Print { cp: 'A' },
            ]
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_cr() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\rA");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print {
                    cp: char::REPLACEMENT_CHARACTER,
                },
                Action::CarriageReturn,
                Action::Print { cp: 'A' },
            ]
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_backspace() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\x08A");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print {
                    cp: char::REPLACEMENT_CHARACTER,
                },
                Action::Backspace,
                Action::Print { cp: 'A' },
            ]
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_horizontal_tab() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\tA");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print {
                    cp: char::REPLACEMENT_CHARACTER,
                },
                Action::HorizontalTab,
                Action::Print { cp: 'A' },
            ]
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_escape_h_tab_set() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\x1bHA");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print {
                    cp: char::REPLACEMENT_CHARACTER,
                },
                Action::TabSet,
                Action::Print { cp: 'A' },
            ]
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_split_escape_h_tab_set() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\x1b");
        assert_eq!(
            actions(&handler),
            &[Action::Print {
                cp: char::REPLACEMENT_CHARACTER,
            }]
        );

        next_slice(&mut stream, &mut handler, b"HA");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print {
                    cp: char::REPLACEMENT_CHARACTER,
                },
                Action::TabSet,
                Action::Print { cp: 'A' },
            ]
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_ignoring_del() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, &[0xf0, 0x9f, 0x7f, b'A']);

        assert_eq!(
            print_chars(&handler),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_direct_escape() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\x1bcA");

        assert_eq!(
            print_chars(&handler),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_csi_escape() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\x1b[CA");

        assert_eq!(
            print_chars(&handler),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_direct_unsupported_escape_final_does_not_leak_as_printable_text() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x1bcB");

        assert_eq!(print_chars(&handler), vec!['A', 'B']);
    }

    #[test]
    fn stream_split_escape_h_dispatches_tab_set_action() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x1b");
        assert_eq!(actions(&handler), &[Action::Print { cp: 'A' }]);
        next_slice(&mut stream, &mut handler, b"HB");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print { cp: 'A' },
                Action::TabSet,
                Action::Print { cp: 'B' },
            ]
        );
    }

    #[test]
    fn stream_unsupported_csi_sequence_does_not_leak_bytes_as_printable_text() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x1b[CB");

        assert_eq!(print_chars(&handler), vec!['A', 'B']);
    }

    #[test]
    fn stream_csi_w_dispatches_tab_set_action() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x1b[WB");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print { cp: 'A' },
                Action::TabSet,
                Action::Print { cp: 'B' },
            ]
        );
    }

    #[test]
    fn stream_csi_zero_w_dispatches_tab_set_action() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x1b[0WB");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print { cp: 'A' },
                Action::TabSet,
                Action::Print { cp: 'B' },
            ]
        );
    }

    #[test]
    fn stream_csi_tab_clear_and_reset_dispatch_actions() {
        for (input, expected) in [
            (b"A\x1b[2WB".as_slice(), Action::TabClearCurrent),
            (b"A\x1b[5WB".as_slice(), Action::TabClearAll),
            (b"A\x1b[?5WB".as_slice(), Action::TabReset),
        ] {
            let mut stream = Stream::init();
            let mut handler = RecordingHandler::default();

            next_slice(&mut stream, &mut handler, input);

            assert_eq!(
                actions(&handler),
                &[
                    Action::Print { cp: 'A' },
                    expected,
                    Action::Print { cp: 'B' },
                ]
            );
        }
    }

    #[test]
    fn stream_split_csi_w_dispatches_tab_set_action() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x1b[");
        assert_eq!(actions(&handler), &[Action::Print { cp: 'A' }]);
        next_slice(&mut stream, &mut handler, b"WB");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print { cp: 'A' },
                Action::TabSet,
                Action::Print { cp: 'B' },
            ]
        );
    }

    #[test]
    fn stream_split_csi_zero_w_dispatches_tab_set_action() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"A\x1b[0");
        assert_eq!(actions(&handler), &[Action::Print { cp: 'A' }]);
        next_slice(&mut stream, &mut handler, b"WB");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print { cp: 'A' },
                Action::TabSet,
                Action::Print { cp: 'B' },
            ]
        );
    }

    #[test]
    fn stream_split_csi_tab_clear_and_reset_dispatch_actions() {
        for (first, second, expected) in [
            (
                b"A\x1b[2".as_slice(),
                b"WB".as_slice(),
                Action::TabClearCurrent,
            ),
            (b"A\x1b[5".as_slice(), b"WB".as_slice(), Action::TabClearAll),
            (b"A\x1b[?".as_slice(), b"5WB".as_slice(), Action::TabReset),
        ] {
            let mut stream = Stream::init();
            let mut handler = RecordingHandler::default();

            next_slice(&mut stream, &mut handler, first);
            assert_eq!(actions(&handler), &[Action::Print { cp: 'A' }]);
            next_slice(&mut stream, &mut handler, second);

            assert_eq!(
                actions(&handler),
                &[
                    Action::Print { cp: 'A' },
                    expected,
                    Action::Print { cp: 'B' },
                ]
            );
        }
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_csi_w_tab_set() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\x1b[WA");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print {
                    cp: char::REPLACEMENT_CHARACTER,
                },
                Action::TabSet,
                Action::Print { cp: 'A' },
            ]
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_split_csi_w_tab_set() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xf0\x9f\x1b[");
        assert_eq!(
            actions(&handler),
            &[Action::Print {
                cp: char::REPLACEMENT_CHARACTER,
            }]
        );

        next_slice(&mut stream, &mut handler, b"WA");

        assert_eq!(
            actions(&handler),
            &[
                Action::Print {
                    cp: char::REPLACEMENT_CHARACTER,
                },
                Action::TabSet,
                Action::Print { cp: 'A' },
            ]
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_csi_tab_clear_and_reset() {
        for (input, expected) in [
            (b"\xf0\x9f\x1b[2WA".as_slice(), Action::TabClearCurrent),
            (b"\xf0\x9f\x1b[5WA".as_slice(), Action::TabClearAll),
            (b"\xf0\x9f\x1b[?5WA".as_slice(), Action::TabReset),
        ] {
            let mut stream = Stream::init();
            let mut handler = RecordingHandler::default();

            next_slice(&mut stream, &mut handler, input);

            assert_eq!(
                actions(&handler),
                &[
                    Action::Print {
                        cp: char::REPLACEMENT_CHARACTER,
                    },
                    expected,
                    Action::Print { cp: 'A' },
                ]
            );
        }
    }

    #[test]
    fn stream_unsupported_csi_w_variants_do_not_dispatch_tab_actions() {
        for input in [
            b"\x1b[>WA".as_slice(),
            b"\x1b[?WA".as_slice(),
            b"\x1b[99WA".as_slice(),
            b"\x1b[1WA".as_slice(),
            b"\x1b[0;0WA".as_slice(),
            b"\x1b[?2WA".as_slice(),
            b"\x1b[>5WA".as_slice(),
            b"\x1b[?1WA".as_slice(),
            b"\x1b[0;5WA".as_slice(),
        ] {
            let mut stream = Stream::init();
            let mut handler = RecordingHandler::default();

            next_slice(&mut stream, &mut handler, input);

            assert_eq!(actions(&handler), &[Action::Print { cp: 'A' }]);
        }
    }

    #[test]
    fn stream_overflowing_csi_w_parameter_does_not_dispatch_tab_set() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(
            &mut stream,
            &mut handler,
            b"\x1b[999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999WZ",
        );

        assert_eq!(actions(&handler), &[Action::Print { cp: 'Z' }]);
    }

    #[derive(Debug, Default)]
    struct ErrorOnActionHandler {
        fail: Option<Action>,
        actions: Vec<Action>,
    }

    impl ErrorOnActionHandler {
        fn new(fail: Action) -> Self {
            Self {
                fail: Some(fail),
                actions: Vec::new(),
            }
        }
    }

    impl Handler for ErrorOnActionHandler {
        type Error = ();

        fn vt(&mut self, action: Action) -> Result<(), Self::Error> {
            if self.fail == Some(action) {
                return Err(());
            }

            self.actions.push(action);
            Ok(())
        }
    }

    #[test]
    fn stream_escape_h_restores_ground_before_handler_error() {
        let mut stream = Stream::init();
        let mut handler = ErrorOnActionHandler::new(Action::TabSet);

        assert_eq!(stream.next_slice(b"\x1bH", &mut handler), Err(()));
        stream.next_slice(b"A", &mut handler).unwrap();

        assert_eq!(handler.actions, &[Action::Print { cp: 'A' }]);
    }

    #[test]
    fn stream_csi_w_restores_ground_before_handler_error() {
        let mut stream = Stream::init();
        let mut handler = ErrorOnActionHandler::new(Action::TabSet);

        assert_eq!(stream.next_slice(b"\x1b[W", &mut handler), Err(()));
        stream.next_slice(b"A", &mut handler).unwrap();

        assert_eq!(handler.actions, &[Action::Print { cp: 'A' }]);
    }

    #[test]
    fn stream_csi_tab_clear_and_reset_restore_ground_before_handler_error() {
        for (input, fail) in [
            (b"\x1b[2W".as_slice(), Action::TabClearCurrent),
            (b"\x1b[5W".as_slice(), Action::TabClearAll),
            (b"\x1b[?5W".as_slice(), Action::TabReset),
        ] {
            let mut stream = Stream::init();
            let mut handler = ErrorOnActionHandler::new(fail);

            assert_eq!(stream.next_slice(input, &mut handler), Err(()));
            stream.next_slice(b"A", &mut handler).unwrap();

            assert_eq!(handler.actions, &[Action::Print { cp: 'A' }]);
        }
    }

    #[test]
    fn stream_state_remains_usable_after_invalid_utf8_and_unsupported_escape() {
        let mut stream = Stream::init();
        let mut handler = RecordingHandler::default();

        next_slice(&mut stream, &mut handler, b"\xffA\x1b[CB");

        assert_eq!(
            print_chars(&handler),
            vec![char::REPLACEMENT_CHARACTER, 'A', 'B']
        );
    }
}
