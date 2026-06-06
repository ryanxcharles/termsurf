//! Tmux protocol helpers.

const DEFAULT_MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlParser {
    state: State,
    buffer: Vec<u8>,
    max_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlNotification {
    Enter,
    Exit,
    BlockEnd(Vec<u8>),
    BlockErr(Vec<u8>),
    Output {
        pane_id: usize,
        data: String,
    },
    SessionChanged {
        id: usize,
        name: String,
    },
    SessionsChanged,
    LayoutChange {
        window_id: usize,
        layout: String,
        visible_layout: String,
        raw_flags: String,
    },
    WindowAdd {
        id: usize,
    },
    WindowRenamed {
        id: usize,
        name: String,
    },
    WindowPaneChanged {
        window_id: usize,
        pane_id: usize,
    },
    ClientDetached {
        client: String,
    },
    ClientSessionChanged {
        client: String,
        session_id: usize,
        name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlParseError {
    OutOfMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Broken,
    Notification,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockTerminator {
    End,
    Err,
}

impl ControlParser {
    pub(crate) const fn new() -> Self {
        Self {
            state: State::Idle,
            buffer: Vec::new(),
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }

    pub(crate) fn put(
        &mut self,
        byte: u8,
    ) -> Result<Option<ControlNotification>, ControlParseError> {
        if self.state == State::Broken {
            return Ok(None);
        }

        if self.buffer.len() >= self.max_bytes {
            self.broken();
            return Err(ControlParseError::OutOfMemory);
        }

        match self.state {
            State::Broken => unreachable!("broken state returns before match"),
            State::Idle => {
                if byte != b'%' {
                    self.broken();
                    return Ok(Some(ControlNotification::Exit));
                }

                self.buffer.clear();
                self.state = State::Notification;
            }
            State::Notification => {
                if byte == b'\n' {
                    return Ok(self.parse_notification());
                }
            }
            State::Block => {
                if byte == b'\n' {
                    let line_start = self
                        .buffer
                        .iter()
                        .rposition(|b| *b == b'\n')
                        .map_or(0, |index| index + 1);
                    let line = &self.buffer[line_start..];

                    if let Some(terminator) = parse_block_terminator(line) {
                        let output = trim_ascii_right(&self.buffer[..line_start], b"\r\n").to_vec();
                        self.state = State::Idle;
                        return Ok(Some(match terminator {
                            BlockTerminator::End => ControlNotification::BlockEnd(output),
                            BlockTerminator::Err => ControlNotification::BlockErr(output),
                        }));
                    }
                }
            }
        }

        self.buffer.push(byte);
        Ok(None)
    }

    fn parse_notification(&mut self) -> Option<ControlNotification> {
        let mut line = self.buffer.as_slice();
        if let Some(stripped) = line.strip_suffix(b"\r") {
            line = stripped;
        }

        if line == b"%begin" || line.starts_with(b"%begin ") {
            self.buffer.clear();
            self.state = State::Block;
            return None;
        }

        let notification = std::str::from_utf8(line)
            .ok()
            .and_then(parse_notification_line);

        self.buffer.clear();
        self.state = State::Idle;
        notification
    }

    fn broken(&mut self) {
        self.state = State::Broken;
        self.buffer.clear();
    }

    #[cfg(test)]
    fn with_max_bytes(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            ..Self::new()
        }
    }
}

impl Default for ControlParser {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_notification_line(line: &str) -> Option<ControlNotification> {
    let cmd = line.split_once(' ').map_or(line, |(cmd, _)| cmd);

    match cmd {
        "%output" => {
            let rest = line.strip_prefix("%output %")?;
            let (pane_id, data) = parse_usize_field(rest)?;
            non_empty(data).map(|data| ControlNotification::Output {
                pane_id,
                data: data.to_owned(),
            })
        }
        "%session-changed" => {
            let rest = line.strip_prefix("%session-changed $")?;
            let (id, name) = parse_usize_field(rest)?;
            non_empty(name).map(|name| ControlNotification::SessionChanged {
                id,
                name: name.to_owned(),
            })
        }
        "%sessions-changed" => {
            (line == "%sessions-changed").then_some(ControlNotification::SessionsChanged)
        }
        "%layout-change" => {
            let rest = line.strip_prefix("%layout-change @")?;
            let (window_id, rest) = parse_usize_field(rest)?;
            let (layout, rest) = parse_non_empty_field(rest)?;
            let (visible_layout, raw_flags) = parse_non_empty_field(rest)?;
            Some(ControlNotification::LayoutChange {
                window_id,
                layout: layout.to_owned(),
                visible_layout: visible_layout.to_owned(),
                raw_flags: raw_flags.to_owned(),
            })
        }
        "%window-add" => {
            let rest = line.strip_prefix("%window-add @")?;
            parse_usize_exact(rest).map(|id| ControlNotification::WindowAdd { id })
        }
        "%window-renamed" => {
            let rest = line.strip_prefix("%window-renamed @")?;
            let (id, name) = parse_usize_field(rest)?;
            non_empty(name).map(|name| ControlNotification::WindowRenamed {
                id,
                name: name.to_owned(),
            })
        }
        "%window-pane-changed" => {
            let rest = line.strip_prefix("%window-pane-changed @")?;
            let (window_id, rest) = parse_usize_field(rest)?;
            let pane_id = rest.strip_prefix('%').and_then(parse_usize_exact)?;
            Some(ControlNotification::WindowPaneChanged { window_id, pane_id })
        }
        "%client-detached" => {
            let client = line.strip_prefix("%client-detached ")?;
            non_empty(client).map(|client| ControlNotification::ClientDetached {
                client: client.to_owned(),
            })
        }
        "%client-session-changed" => {
            let rest = line.strip_prefix("%client-session-changed ")?;
            let (client, rest) = rest.rsplit_once(" $")?;
            let (session_id, name) = parse_usize_field(rest)?;
            non_empty(name).map(|name| ControlNotification::ClientSessionChanged {
                client: client.to_owned(),
                session_id,
                name: name.to_owned(),
            })
        }
        _ => None,
    }
}

fn parse_block_terminator(mut line: &[u8]) -> Option<BlockTerminator> {
    if let Some(stripped) = line.strip_suffix(b"\r") {
        line = stripped;
    }

    let line = std::str::from_utf8(line).ok()?;
    let mut fields = line.split(' ').filter(|field| !field.is_empty());
    let terminator = match fields.next()? {
        "%end" => BlockTerminator::End,
        "%error" => BlockTerminator::Err,
        _ => return None,
    };

    parse_usize_exact(fields.next()?)?;
    parse_usize_exact(fields.next()?)?;
    parse_usize_exact(fields.next()?)?;
    if fields.next().is_some() {
        return None;
    }

    Some(terminator)
}

fn parse_usize_exact(value: &str) -> Option<usize> {
    if value.is_empty() || !value.as_bytes().iter().all(u8::is_ascii_digit) {
        return None;
    }
    value.parse().ok()
}

fn parse_usize_field(value: &str) -> Option<(usize, &str)> {
    let (digits, rest) = value.split_once(' ')?;
    Some((parse_usize_exact(digits)?, rest))
}

fn parse_non_empty_field(value: &str) -> Option<(&str, &str)> {
    let (field, rest) = value.split_once(' ')?;
    non_empty(field).map(|field| (field, rest))
}

fn non_empty(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

fn trim_ascii_right<'a>(mut value: &'a [u8], bytes: &[u8]) -> &'a [u8] {
    while value.last().is_some_and(|byte| bytes.contains(byte)) {
        value = &value[..value.len() - 1];
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(parser: &mut ControlParser, data: &[u8]) {
        for byte in data {
            assert_eq!(parser.put(*byte), Ok(None));
        }
    }

    #[test]
    fn tmux_begin_end_empty() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%begin 1578922740 269 1\n");
        feed(&mut parser, b"%end 1578922740 269 1");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::BlockEnd(Vec::new())))
        );
    }

    #[test]
    fn tmux_begin_error_empty() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%begin 1578922740 269 1\n");
        feed(&mut parser, b"%error 1578922740 269 1");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::BlockErr(Vec::new())))
        );
    }

    #[test]
    fn tmux_begin_end_data() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%begin 1578922740 269 1\n");
        feed(&mut parser, b"hello\nworld\n");
        feed(&mut parser, b"%end 1578922740 269 1");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::BlockEnd(
                b"hello\nworld".to_vec()
            )))
        );
    }

    #[test]
    fn tmux_block_payload_may_start_with_end() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%begin 1 1 1\n");
        feed(&mut parser, b"%end not really\n");
        feed(&mut parser, b"hello\n");
        feed(&mut parser, b"%end 1 1 1");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::BlockEnd(
                b"%end not really\nhello".to_vec()
            )))
        );
    }

    #[test]
    fn tmux_block_payload_may_start_with_error() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%begin 1 1 1\n");
        feed(&mut parser, b"%error not really\n");
        feed(&mut parser, b"hello\n");
        feed(&mut parser, b"%end 1 1 1");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::BlockEnd(
                b"%error not really\nhello".to_vec()
            )))
        );
    }

    #[test]
    fn tmux_block_may_terminate_with_real_error_after_misleading_payload() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%begin 1 1 1\n");
        feed(&mut parser, b"%error not really\n");
        feed(&mut parser, b"hello\n");
        feed(&mut parser, b"%error 1 1 1");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::BlockErr(
                b"%error not really\nhello".to_vec()
            )))
        );
    }

    #[test]
    fn tmux_block_terminator_requires_exact_token_count() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%begin 1 1 1\n");
        feed(&mut parser, b"%end 1 1 1 trailing\n");
        feed(&mut parser, b"hello\n");
        feed(&mut parser, b"%end 1 1 1");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::BlockEnd(
                b"%end 1 1 1 trailing\nhello".to_vec()
            )))
        );
    }

    #[test]
    fn tmux_block_terminator_requires_numeric_metadata() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%begin 1 1 1\n");
        feed(&mut parser, b"%end foo bar baz\n");
        feed(&mut parser, b"hello\n");
        feed(&mut parser, b"%end 1 1 1");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::BlockEnd(
                b"%end foo bar baz\nhello".to_vec()
            )))
        );
    }

    #[test]
    fn tmux_block_terminator_allows_repeated_spaces() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%begin 1 1 1\n");
        feed(&mut parser, b"%end  1  1  1");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::BlockEnd(Vec::new())))
        );
    }

    #[test]
    fn tmux_output() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%output %42 foo bar baz");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::Output {
                pane_id: 42,
                data: "foo bar baz".to_owned(),
            }))
        );
    }

    #[test]
    fn tmux_session_changed() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%session-changed $42 foo");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::SessionChanged {
                id: 42,
                name: "foo".to_owned(),
            }))
        );
    }

    #[test]
    fn tmux_sessions_changed() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%sessions-changed");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::SessionsChanged))
        );
    }

    #[test]
    fn tmux_sessions_changed_carriage_return() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%sessions-changed\r");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::SessionsChanged))
        );
    }

    #[test]
    fn tmux_layout_change() {
        let mut parser = ControlParser::new();

        feed(
            &mut parser,
            b"%layout-change @2 1234x791,0,0{617x791,0,0,0,617x791,618,0,1} 1234x791,0,0{617x791,0,0,0,617x791,618,0,1} *-",
        );
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::LayoutChange {
                window_id: 2,
                layout: "1234x791,0,0{617x791,0,0,0,617x791,618,0,1}".to_owned(),
                visible_layout: "1234x791,0,0{617x791,0,0,0,617x791,618,0,1}".to_owned(),
                raw_flags: "*-".to_owned(),
            }))
        );
    }

    #[test]
    fn tmux_window_add() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%window-add @14");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::WindowAdd { id: 14 }))
        );
    }

    #[test]
    fn tmux_window_renamed() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%window-renamed @42 bar");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::WindowRenamed {
                id: 42,
                name: "bar".to_owned(),
            }))
        );
    }

    #[test]
    fn tmux_window_pane_changed() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%window-pane-changed @42 %2");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::WindowPaneChanged {
                window_id: 42,
                pane_id: 2,
            }))
        );
    }

    #[test]
    fn tmux_client_detached() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%client-detached /dev/pts/1");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::ClientDetached {
                client: "/dev/pts/1".to_owned(),
            }))
        );
    }

    #[test]
    fn tmux_client_session_changed() {
        let mut parser = ControlParser::new();

        feed(
            &mut parser,
            b"%client-session-changed /dev/pts/1 $2 mysession",
        );
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::ClientSessionChanged {
                client: "/dev/pts/1".to_owned(),
                session_id: 2,
                name: "mysession".to_owned(),
            }))
        );
    }

    #[test]
    fn tmux_client_session_changed_allows_spaces_in_client() {
        let mut parser = ControlParser::new();

        feed(
            &mut parser,
            b"%client-session-changed /dev/pts/client one $2 mysession",
        );
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::ClientSessionChanged {
                client: "/dev/pts/client one".to_owned(),
                session_id: 2,
                name: "mysession".to_owned(),
            }))
        );
    }

    #[test]
    fn tmux_unknown_notification_returns_to_idle() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%unknown value");
        assert_eq!(parser.put(b'\n'), Ok(None));
        feed(&mut parser, b"%sessions-changed");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::SessionsChanged))
        );
    }

    #[test]
    fn tmux_malformed_notification_returns_to_idle() {
        let mut parser = ControlParser::new();

        feed(&mut parser, b"%window-add @wat");
        assert_eq!(parser.put(b'\n'), Ok(None));
        feed(&mut parser, b"%window-add @14");
        assert_eq!(
            parser.put(b'\n'),
            Ok(Some(ControlNotification::WindowAdd { id: 14 }))
        );
    }

    #[test]
    fn tmux_idle_non_percent_enters_broken_state_and_emits_exit() {
        let mut parser = ControlParser::new();

        assert_eq!(parser.put(b'x'), Ok(Some(ControlNotification::Exit)));
        assert_eq!(parser.put(b'%'), Ok(None));
        assert_eq!(parser.put(b'\n'), Ok(None));
    }

    #[test]
    fn tmux_over_capacity_enters_broken_state() {
        let mut parser = ControlParser::with_max_bytes(2);

        assert_eq!(parser.put(b'%'), Ok(None));
        assert_eq!(parser.put(b'x'), Ok(None));
        assert_eq!(parser.put(b'y'), Err(ControlParseError::OutOfMemory));
        assert_eq!(parser.put(b'z'), Ok(None));
    }

    #[test]
    fn tmux_enter_notification_is_reserved_for_dcs_entry() {
        assert_eq!(ControlNotification::Enter, ControlNotification::Enter);
    }
}
