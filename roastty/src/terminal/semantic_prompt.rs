#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SemanticPrompt<'a> {
    pub(super) action: Action,
    options: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    FreshLine,
    FreshLineNewPrompt,
    NewCommand,
    PromptStart,
    EndPromptStartInput,
    EndPromptStartInputTerminateEol,
    EndInputStartOutput,
    EndCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Click {
    Line,
    Multiple,
    ConservativeVertical,
    SmartVertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PromptKind {
    Initial,
    Right,
    Continuation,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Redraw {
    True,
    False,
    Last,
}

impl<'a> SemanticPrompt<'a> {
    pub(super) const fn new(action: Action, options: &'a [u8]) -> Self {
        Self { action, options }
    }

    pub(super) const fn options(self) -> &'a [u8] {
        self.options
    }

    pub(super) fn aid(self) -> Option<&'a [u8]> {
        read_field(self.options, b"aid")
    }

    pub(super) fn click(self) -> Option<Click> {
        match read_field(self.options, b"cl")? {
            b"line" => Some(Click::Line),
            b"m" => Some(Click::Multiple),
            b"v" => Some(Click::ConservativeVertical),
            b"w" => Some(Click::SmartVertical),
            _ => None,
        }
    }

    pub(super) fn prompt_kind(self) -> Option<PromptKind> {
        match read_field(self.options, b"k")? {
            b"i" => Some(PromptKind::Initial),
            b"r" => Some(PromptKind::Right),
            b"c" => Some(PromptKind::Continuation),
            b"s" => Some(PromptKind::Secondary),
            _ => None,
        }
    }

    pub(super) fn err(self) -> Option<&'a [u8]> {
        read_field(self.options, b"err")
    }

    pub(super) fn redraw(self) -> Option<Redraw> {
        match read_field(self.options, b"redraw")? {
            b"0" => Some(Redraw::False),
            b"1" => Some(Redraw::True),
            b"last" => Some(Redraw::Last),
            _ => None,
        }
    }

    pub(super) fn special_key(self) -> Option<bool> {
        read_bool(read_field(self.options, b"special_key")?)
    }

    pub(super) fn click_events(self) -> Option<bool> {
        read_bool(read_field(self.options, b"click_events")?)
    }

    pub(super) fn cmdline(self) -> Option<&'a [u8]> {
        read_field(self.options, b"cmdline")
    }

    pub(super) fn cmdline_url(self) -> Option<&'a [u8]> {
        read_field(self.options, b"cmdline_url")
    }

    pub(super) fn exit_code(self) -> Option<i32> {
        let first = self
            .options
            .split(|byte| *byte == b';')
            .next()
            .unwrap_or_default();
        std::str::from_utf8(first).ok()?.parse().ok()
    }
}

fn read_field<'a>(options: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    for field in options.split(|byte| *byte == b';') {
        let Some(eq_idx) = field.iter().position(|byte| *byte == b'=') else {
            continue;
        };
        if &field[..eq_idx] == key {
            return Some(&field[eq_idx + 1..]);
        }
    }
    None
}

fn read_bool(bytes: &[u8]) -> Option<bool> {
    match bytes {
        b"0" => Some(false),
        b"1" => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(options: &[u8]) -> SemanticPrompt<'_> {
        SemanticPrompt::new(Action::FreshLineNewPrompt, options)
    }

    #[test]
    fn semantic_prompt_reads_raw_string_options() {
        let value = prompt(b"aid=a=b;err=;cmdline=\xff;cmdline_url=");

        assert_eq!(value.aid(), Some(b"a=b".as_slice()));
        assert_eq!(value.err(), Some(b"".as_slice()));
        assert_eq!(value.cmdline(), Some(b"\xff".as_slice()));
        assert_eq!(value.cmdline_url(), Some(b"".as_slice()));
        assert_eq!(prompt(b"bare;aid=x").aid(), Some(b"x".as_slice()));
    }

    #[test]
    fn semantic_prompt_reads_enums_case_sensitively() {
        assert_eq!(prompt(b"cl=line").click(), Some(Click::Line));
        assert_eq!(prompt(b"cl=m").click(), Some(Click::Multiple));
        assert_eq!(prompt(b"cl=v").click(), Some(Click::ConservativeVertical));
        assert_eq!(prompt(b"cl=w").click(), Some(Click::SmartVertical));
        assert_eq!(prompt(b"cl=Line").click(), None);

        assert_eq!(prompt(b"k=i").prompt_kind(), Some(PromptKind::Initial));
        assert_eq!(prompt(b"k=r").prompt_kind(), Some(PromptKind::Right));
        assert_eq!(prompt(b"k=c").prompt_kind(), Some(PromptKind::Continuation));
        assert_eq!(prompt(b"k=s").prompt_kind(), Some(PromptKind::Secondary));
        assert_eq!(prompt(b"k=initial").prompt_kind(), None);

        assert_eq!(prompt(b"redraw=0").redraw(), Some(Redraw::False));
        assert_eq!(prompt(b"redraw=1").redraw(), Some(Redraw::True));
        assert_eq!(prompt(b"redraw=last").redraw(), Some(Redraw::Last));
        assert_eq!(prompt(b"redraw=true").redraw(), None);
    }

    #[test]
    fn semantic_prompt_reads_bools_and_exit_code() {
        assert_eq!(prompt(b"special_key=0").special_key(), Some(false));
        assert_eq!(prompt(b"special_key=1").special_key(), Some(true));
        assert_eq!(prompt(b"special_key=true").special_key(), None);
        assert_eq!(prompt(b"click_events=0").click_events(), Some(false));
        assert_eq!(prompt(b"click_events=1").click_events(), Some(true));
        assert_eq!(prompt(b"click_events=11").click_events(), None);

        assert_eq!(prompt(b"0").exit_code(), Some(0));
        assert_eq!(prompt(b"-1").exit_code(), Some(-1));
        assert_eq!(prompt(b"2147483647").exit_code(), Some(i32::MAX));
        assert_eq!(prompt(b"-2147483648").exit_code(), Some(i32::MIN));
        assert_eq!(prompt(b"").exit_code(), None);
        assert_eq!(prompt(b"abc").exit_code(), None);
        assert_eq!(prompt(b"2147483648").exit_code(), None);
        assert_eq!(prompt(b"-2147483649").exit_code(), None);
    }

    #[test]
    fn semantic_prompt_first_matching_option_wins_even_when_malformed() {
        assert_eq!(prompt(b"cl=bad;cl=line").click(), None);
        assert_eq!(prompt(b"k=bad;k=i").prompt_kind(), None);
        assert_eq!(prompt(b"special_key=x;special_key=1").special_key(), None);
        assert_eq!(prompt(b"aid=;aid=x").aid(), Some(b"".as_slice()));
        assert_eq!(prompt(b"bad;cl=line").click(), Some(Click::Line));
    }
}
