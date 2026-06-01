#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ContextSignal<'a> {
    pub(super) action: Action,
    pub(super) id: &'a [u8],
    pub(super) metadata: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContextType {
    Boot,
    Container,
    Vm,
    Elevate,
    ChangePrivilege,
    Subcontext,
    Remote,
    Shell,
    Command,
    App,
    Service,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExitStatus {
    Success,
    Failure,
    Crash,
    Interrupt,
}

impl ContextSignal<'_> {
    pub(super) fn context_type(&self) -> Option<ContextType> {
        match read_field(self.metadata, b"type")? {
            b"boot" => Some(ContextType::Boot),
            b"container" => Some(ContextType::Container),
            b"vm" => Some(ContextType::Vm),
            b"elevate" => Some(ContextType::Elevate),
            b"chpriv" => Some(ContextType::ChangePrivilege),
            b"subcontext" => Some(ContextType::Subcontext),
            b"remote" => Some(ContextType::Remote),
            b"shell" => Some(ContextType::Shell),
            b"command" => Some(ContextType::Command),
            b"app" => Some(ContextType::App),
            b"service" => Some(ContextType::Service),
            b"session" => Some(ContextType::Session),
            _ => None,
        }
    }

    pub(super) fn user(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"user")?)
    }

    pub(super) fn hostname(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"hostname")?)
    }

    pub(super) fn machineid(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"machineid")?)
    }

    pub(super) fn bootid(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"bootid")?)
    }

    pub(super) fn pid(&self) -> Option<u64> {
        parse_u64(read_field(self.metadata, b"pid")?)
    }

    pub(super) fn pidfdid(&self) -> Option<u64> {
        parse_u64(read_field(self.metadata, b"pidfdid")?)
    }

    pub(super) fn comm(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"comm")?)
    }

    pub(super) fn cwd(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"cwd")?)
    }

    pub(super) fn cmdline(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"cmdline")?)
    }

    pub(super) fn vm(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"vm")?)
    }

    pub(super) fn container(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"container")?)
    }

    pub(super) fn targetuser(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"targetuser")?)
    }

    pub(super) fn targethost(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"targethost")?)
    }

    pub(super) fn sessionid(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"sessionid")?)
    }

    pub(super) fn exit(&self) -> Option<ExitStatus> {
        match read_field(self.metadata, b"exit")? {
            b"success" => Some(ExitStatus::Success),
            b"failure" => Some(ExitStatus::Failure),
            b"crash" => Some(ExitStatus::Crash),
            b"interrupt" => Some(ExitStatus::Interrupt),
            _ => None,
        }
    }

    pub(super) fn status(&self) -> Option<u64> {
        parse_u64(read_field(self.metadata, b"status")?)
    }

    pub(super) fn signal(&self) -> Option<&[u8]> {
        non_empty(read_field(self.metadata, b"signal")?)
    }
}

fn read_field<'a>(metadata: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    for field in metadata.split(|byte| *byte == b';') {
        let Some(eq_idx) = field.iter().position(|byte| *byte == b'=') else {
            continue;
        };
        if &field[..eq_idx] == key {
            return Some(&field[eq_idx + 1..]);
        }
    }
    None
}

fn non_empty(bytes: &[u8]) -> Option<&[u8]> {
    (!bytes.is_empty()).then_some(bytes)
}

fn parse_u64(bytes: &[u8]) -> Option<u64> {
    if !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(metadata: &[u8]) -> ContextSignal<'_> {
        ContextSignal {
            action: Action::Start,
            id: b"id",
            metadata,
        }
    }

    #[test]
    fn context_signal_reads_string_fields_as_raw_nonempty_bytes() {
        let value = signal(
            b"user=root;hostname=host;machineid=machine;bootid=boot;comm=bash;cwd=/tmp;cmdline=\xff;vm=v;container=c;targetuser=tu;targethost=th;sessionid=s;signal=SIGKILL",
        );

        assert_eq!(value.user(), Some(b"root".as_slice()));
        assert_eq!(value.hostname(), Some(b"host".as_slice()));
        assert_eq!(value.machineid(), Some(b"machine".as_slice()));
        assert_eq!(value.bootid(), Some(b"boot".as_slice()));
        assert_eq!(value.comm(), Some(b"bash".as_slice()));
        assert_eq!(value.cwd(), Some(b"/tmp".as_slice()));
        assert_eq!(value.cmdline(), Some(b"\xff".as_slice()));
        assert_eq!(value.vm(), Some(b"v".as_slice()));
        assert_eq!(value.container(), Some(b"c".as_slice()));
        assert_eq!(value.targetuser(), Some(b"tu".as_slice()));
        assert_eq!(value.targethost(), Some(b"th".as_slice()));
        assert_eq!(value.sessionid(), Some(b"s".as_slice()));
        assert_eq!(value.signal(), Some(b"SIGKILL".as_slice()));
        assert_eq!(signal(b"user=").user(), None);
    }

    #[test]
    fn context_signal_reads_enums_case_sensitively() {
        for (raw, expected) in [
            (b"boot".as_slice(), ContextType::Boot),
            (b"container".as_slice(), ContextType::Container),
            (b"vm".as_slice(), ContextType::Vm),
            (b"elevate".as_slice(), ContextType::Elevate),
            (b"chpriv".as_slice(), ContextType::ChangePrivilege),
            (b"subcontext".as_slice(), ContextType::Subcontext),
            (b"remote".as_slice(), ContextType::Remote),
            (b"shell".as_slice(), ContextType::Shell),
            (b"command".as_slice(), ContextType::Command),
            (b"app".as_slice(), ContextType::App),
            (b"service".as_slice(), ContextType::Service),
            (b"session".as_slice(), ContextType::Session),
        ] {
            let mut metadata = b"type=".to_vec();
            metadata.extend_from_slice(raw);
            assert_eq!(signal(&metadata).context_type(), Some(expected));
        }
        assert_eq!(signal(b"type=Shell").context_type(), None);

        assert_eq!(signal(b"exit=success").exit(), Some(ExitStatus::Success));
        assert_eq!(signal(b"exit=failure").exit(), Some(ExitStatus::Failure));
        assert_eq!(signal(b"exit=crash").exit(), Some(ExitStatus::Crash));
        assert_eq!(
            signal(b"exit=interrupt").exit(),
            Some(ExitStatus::Interrupt)
        );
        assert_eq!(signal(b"exit=SUCCESS").exit(), None);
    }

    #[test]
    fn context_signal_reads_numeric_fields() {
        let value = signal(b"pid=1;pidfdid=2;status=3");
        assert_eq!(value.pid(), Some(1));
        assert_eq!(value.pidfdid(), Some(2));
        assert_eq!(value.status(), Some(3));

        assert_eq!(signal(b"pid=").pid(), None);
        assert_eq!(signal(b"pid=abc").pid(), None);
        assert_eq!(signal(b"pid=-1").pid(), None);
        assert_eq!(signal(b"pid=18446744073709551616").pid(), None);
    }

    #[test]
    fn context_signal_first_matching_field_wins_even_when_malformed() {
        assert_eq!(signal(b"pid=bad;pid=1").pid(), None);
        assert_eq!(signal(b"type=BAD;type=shell").context_type(), None);
        assert_eq!(signal(b"user=;user=root").user(), None);
        assert_eq!(signal(b"bad;pid=1").pid(), Some(1));
        assert_eq!(signal(b"unknown=value;pid=1").pid(), Some(1));
    }
}
