//! Terminal byte stream decoding.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Print { cp: char },
}

pub(super) trait Handler {
    fn vt(&mut self, action: Action);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EscapeState {
    Ground,
    Escape,
    Csi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Stream<H> {
    handler: H,
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

impl<H> Stream<H> {
    pub(super) const fn init(handler: H) -> Self {
        Self {
            handler,
            utf8: Utf8Decoder::new(),
            escape: EscapeState::Ground,
        }
    }

    #[cfg(test)]
    fn handler(&self) -> &H {
        &self.handler
    }
}

impl<H: Handler> Stream<H> {
    pub(super) fn next_slice(&mut self, input: &[u8]) {
        for &byte in input {
            self.next(byte);
        }
    }

    fn next(&mut self, byte: u8) {
        match self.escape {
            EscapeState::Ground => self.next_ground(byte),
            EscapeState::Escape => self.next_escape(byte),
            EscapeState::Csi => self.next_csi(byte),
        }
    }

    fn next_ground(&mut self, byte: u8) {
        if self.utf8.is_pending() {
            let result = self.utf8.next(byte);
            if let Some(cp) = result.cp {
                self.handler.vt(Action::Print { cp });
            }
            if !result.consumed {
                self.next_ground(byte);
            }
            return;
        }

        match byte {
            0x1b => {
                self.escape = EscapeState::Escape;
            }
            0x00..=0x1a | 0x1c..=0x1f | 0x7f => {}
            _ => self.next_utf8(byte),
        }
    }

    fn next_escape(&mut self, byte: u8) {
        self.escape = if byte == b'[' {
            EscapeState::Csi
        } else {
            EscapeState::Ground
        };
    }

    fn next_csi(&mut self, byte: u8) {
        if (0x40..=0x7e).contains(&byte) {
            self.escape = EscapeState::Ground;
        }
    }

    fn next_utf8(&mut self, byte: u8) {
        let result = self.utf8.next(byte);
        if let Some(cp) = result.cp {
            self.handler.vt(Action::Print { cp });
        }
        debug_assert!(result.consumed);
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
        fn vt(&mut self, action: Action) {
            self.actions.push(action);
        }
    }

    fn print_chars(handler: &RecordingHandler) -> Vec<char> {
        handler
            .actions
            .iter()
            .map(|action| match action {
                Action::Print { cp } => *cp,
            })
            .collect()
    }

    #[test]
    fn stream_ascii_dispatches_one_print_action_per_character() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(b"Hello");

        assert_eq!(print_chars(stream.handler()), vec!['H', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn stream_unicode_scalars_dispatch_correctly() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice("😄✤ÁA".as_bytes());

        assert_eq!(print_chars(stream.handler()), vec!['😄', '✤', 'Á', 'A']);
    }

    #[test]
    fn stream_split_multibyte_scalar_dispatches_after_final_byte() {
        let mut stream = Stream::init(RecordingHandler::default());
        let bytes = "😄".as_bytes();

        stream.next_slice(&bytes[..2]);
        assert!(stream.handler().actions.is_empty());

        stream.next_slice(&bytes[2..]);
        assert_eq!(print_chars(stream.handler()), vec!['😄']);
    }

    #[test]
    fn stream_invalid_utf8_dispatches_replacement_character() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(&[0xff]);

        assert_eq!(
            print_chars(stream.handler()),
            vec![char::REPLACEMENT_CHARACTER]
        );
    }

    #[test]
    fn stream_partial_invalid_utf8_retries_rejecting_starter_byte() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(b"\xf0\x9f\xf0\x9f\x98\x84\xed\xa0\x80");

        assert_eq!(
            print_chars(stream.handler()),
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
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(b"\xf0\x9fA");

        assert_eq!(
            print_chars(stream.handler()),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_incomplete_utf8_held_at_slice_boundary_completes_on_next_slice() {
        let mut stream = Stream::init(RecordingHandler::default());
        let bytes = "✤".as_bytes();

        stream.next_slice(&bytes[..1]);
        stream.next_slice(&bytes[1..2]);
        assert!(stream.handler().actions.is_empty());

        stream.next_slice(&bytes[2..]);
        assert_eq!(print_chars(stream.handler()), vec!['✤']);
    }

    #[test]
    fn stream_c0_controls_do_not_dispatch_print_actions() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(b"A\t\nB");

        assert_eq!(print_chars(stream.handler()), vec!['A', 'B']);
    }

    #[test]
    fn stream_raw_c1_bytes_are_handled_by_utf8_decoder() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(&[0x80, b'A']);

        assert_eq!(
            print_chars(stream.handler()),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_ignoring_c0_control() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(b"\xf0\x9f\nA");

        assert_eq!(
            print_chars(stream.handler()),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_ignoring_del() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(&[0xf0, 0x9f, 0x7f, b'A']);

        assert_eq!(
            print_chars(stream.handler()),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_direct_escape() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(b"\xf0\x9f\x1bcA");

        assert_eq!(
            print_chars(stream.handler()),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_pending_utf8_replacement_dispatches_before_csi_escape() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(b"\xf0\x9f\x1b[CA");

        assert_eq!(
            print_chars(stream.handler()),
            vec![char::REPLACEMENT_CHARACTER, 'A']
        );
    }

    #[test]
    fn stream_direct_unsupported_escape_final_does_not_leak_as_printable_text() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(b"A\x1bcB");

        assert_eq!(print_chars(stream.handler()), vec!['A', 'B']);
    }

    #[test]
    fn stream_unsupported_csi_sequence_does_not_leak_bytes_as_printable_text() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(b"A\x1b[CB");

        assert_eq!(print_chars(stream.handler()), vec!['A', 'B']);
    }

    #[test]
    fn stream_state_remains_usable_after_invalid_utf8_and_unsupported_escape() {
        let mut stream = Stream::init(RecordingHandler::default());

        stream.next_slice(b"\xffA\x1b[CB");

        assert_eq!(
            print_chars(stream.handler()),
            vec![char::REPLACEMENT_CHARACTER, 'A', 'B']
        );
    }
}
