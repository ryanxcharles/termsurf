//! Shell command-string construction (port of upstream `os/shell`).

/// Builder for space-separated shell command strings (upstream `os.shell.ShellCommandBuilder`).
#[derive(Debug, Default)]
pub(crate) struct ShellCommandBuilder {
    buffer: Vec<u8>,
}

impl ShellCommandBuilder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Append an argument with automatic space separation; an empty argument is ignored
    /// (upstream `appendArg`).
    pub(crate) fn append_arg(&mut self, arg: &[u8]) {
        if arg.is_empty() {
            return;
        }
        if !self.buffer.is_empty() {
            self.buffer.push(b' ');
        }
        self.buffer.extend_from_slice(arg);
    }

    /// The built command bytes.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Consume the builder and return the built command bytes (upstream `toOwnedSlice`; the
    /// `[:0]` NUL sentinel is dropped — a Rust caller adds it via `CString` when exec'ing).
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }
}

/// Escape characters a shell treats specially in `input` (upstream `os.shell.ShellEscapeWriter`).
/// Backslash-escapes ``\ " ' $ ` * ? <space> | ( )``; every other byte — notably the linefeed,
/// so it can delineate lists of file paths — passes through unchanged.
pub(crate) fn shell_escape(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    for &byte in input {
        if matches!(
            byte,
            b'\\' | b'"' | b'\'' | b'$' | b'`' | b'*' | b'?' | b' ' | b'|' | b'(' | b')'
        ) {
            out.push(b'\\');
        }
        out.push(byte);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_empty() {
        let cmd = ShellCommandBuilder::new();
        assert_eq!(cmd.as_bytes(), b"");
    }

    #[test]
    fn builder_single_arg() {
        let mut cmd = ShellCommandBuilder::new();
        cmd.append_arg(b"bash");
        assert_eq!(cmd.as_bytes(), b"bash");
    }

    #[test]
    fn builder_multiple_args() {
        let mut cmd = ShellCommandBuilder::new();
        cmd.append_arg(b"bash");
        cmd.append_arg(b"--posix");
        cmd.append_arg(b"-l");
        assert_eq!(cmd.as_bytes(), b"bash --posix -l");
    }

    #[test]
    fn builder_skips_empty_arg() {
        let mut cmd = ShellCommandBuilder::new();
        cmd.append_arg(b"bash");
        cmd.append_arg(b"");
        assert_eq!(cmd.as_bytes(), b"bash");
    }

    #[test]
    fn builder_into_bytes() {
        let mut cmd = ShellCommandBuilder::new();
        cmd.append_arg(b"bash");
        cmd.append_arg(b"--posix");
        assert_eq!(cmd.into_bytes(), b"bash --posix".to_vec());
    }

    #[test]
    fn escape_upstream_examples() {
        assert_eq!(shell_escape(b"abc"), b"abc");
        assert_eq!(shell_escape(b"a c"), b"a\\ c");
        assert_eq!(shell_escape(b"a?c"), b"a\\?c");
        assert_eq!(shell_escape(b"a\\c"), b"a\\\\c");
        assert_eq!(shell_escape(b"a|c"), b"a\\|c");
        assert_eq!(shell_escape(b"a\"c"), b"a\\\"c");
        assert_eq!(shell_escape(b"a(1)"), b"a\\(1\\)");
    }

    #[test]
    fn escape_passes_linefeed_through() {
        // Linefeeds are deliberately not escaped (they delineate lists of paths).
        assert_eq!(shell_escape(b"a\nc"), b"a\nc");
    }

    #[test]
    fn escape_full_special_set() {
        // Each of the exact 11 special bytes is backslash-prefixed when escaped alone.
        for &c in &[
            b'\\', b'"', b'\'', b'$', b'`', b'*', b'?', b' ', b'|', b'(', b')',
        ] {
            assert_eq!(
                shell_escape(&[c]),
                vec![b'\\', c],
                "byte {c:#x} should escape"
            );
        }
        // A representative ordinary byte is not escaped.
        assert_eq!(shell_escape(b"a"), b"a");
    }
}
