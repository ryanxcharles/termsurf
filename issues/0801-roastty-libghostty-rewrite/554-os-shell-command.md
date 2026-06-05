+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 554: shell command construction (os::shell)

## Description

Continuing the `os` module (Experiments 541–553), this experiment opens
`os::shell` with the two shell-command-string helpers from upstream
`os/shell.zig`: `ShellCommandBuilder` (build a space-separated command from
arguments) and `shell_escape` (backslash-escape the characters a shell treats
specially). roastty uses these to construct shell-integration / launch command
strings safely. Both are pure, byte-level string operations with thorough
upstream tests; the Zig `Writer`-vtable machinery around the escaper is dropped
(Rust returns the escaped bytes directly).

## Upstream behavior

`os/shell.zig`:

```zig
/// Builder for constructing space-separated shell command strings.
pub const ShellCommandBuilder = struct {
    buffer: std.Io.Writer.Allocating,
    pub fn appendArg(self, arg: []const u8) !void {
        if (arg.len == 0) return;                       // empty args are skipped
        if (self.buffer.written().len > 0) try self.buffer.writer.writeByte(' ');
        try self.buffer.writer.writeAll(arg);
    }
    pub fn toOwnedSlice(self) ![:0]const u8 { … }       // NUL-terminated
};

/// Writer that escapes characters that shells treat specially (excludes linefeeds so they
/// can delineate lists of file paths).
pub const ShellEscapeWriter = struct {
    fn writeEscaped(self, s: []const u8, count) !void {
        for (s) |byte| {
            const buf = switch (byte) {
                '\\', '"', '\'', '$', '`', '*', '?', ' ', '|', '(', ')' => &.{ '\\', byte },
                else => &.{byte},
            };
            try self.child.writeAll(buf);
        }
    }
};
```

- `ShellCommandBuilder.appendArg`: an empty arg is ignored; otherwise a single
  space is written before the arg **if** the buffer is already non-empty, then
  the arg. `toOwnedSlice` yields the NUL-terminated command.
- `ShellEscapeWriter` (`writeEscaped`): each of ``\ " ' $ ` * ? <space> | ( )``
  is prefixed with a backslash; every other byte (notably linefeed) passes
  through unchanged.

The upstream tests: builder — `""` empty, `"bash"`, `"bash --posix -l"`, an
empty arg skipped (`"bash"`), and `toOwnedSlice` → `"bash --posix"`
(NUL-terminated); escape — `abc`→`abc`, `a c`→`a\ c`, `a?c`→`a\?c`,
`a\c`→`a\\c`, `a|c`→`a\|c`, `a"c`→`a\"c`, `a(1)`→`a\(1\)`.

## Rust mapping (`roastty/src/os/shell.rs`)

Byte-oriented (`&[u8]` / `Vec<u8>`) — these are shell/exec values; the builder
accumulates a `Vec<u8>`, and `shell_escape` returns the escaped bytes directly
(no `Writer` wrapper needed):

```rust
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
```

`append_arg` reproduces the empty-skip and "space only when the buffer is
non-empty" rule; `shell_escape` reproduces the exact escape set (the same
characters, linefeed excluded). The Zig `Writer.Allocating` /
`ShellEscapeWriter` vtable plumbing collapses to a `Vec<u8>` builder and a
`&[u8] -> Vec<u8>` function.

## Scope / faithfulness notes

- **Ported (bridged)**: `os.shell.ShellCommandBuilder` →
  `os::shell::ShellCommandBuilder` (`append_arg` / `as_bytes` / `into_bytes`);
  `os.shell.ShellEscapeWriter` (its `writeEscaped` logic) →
  `os::shell::shell_escape`.
- **Faithful**: the builder's empty-arg skip and single-space-when-non-empty
  separation; the escape set ``\ " ' $ ` * ? <space> | ( )`` with everything
  else (incl. linefeed) unescaped.
- **Faithful adaptation**: the Zig `Writer.Allocating` builder → a `Vec<u8>`;
  the `ShellEscapeWriter` (a `Writer` vtable) → a pure
  `shell_escape(&[u8]) -> Vec<u8>`; `[]const u8` → `&[u8]` / `Vec<u8>`
  (byte-faithful for shell/exec values); the `toOwnedSlice` `[:0]` NUL sentinel
  → dropped (the caller `CString`s when exec'ing).
- **Deferred**: nothing in `shell.zig` (both types are fully ported on the macOS
  arm).
- No C ABI/header/ABI-inventory change (internal Rust). New `os::shell` module.

## Changes

1. `roastty/src/os/shell.rs` (new): `ShellCommandBuilder`, `shell_escape`.
2. `roastty/src/os/mod.rs`: add `pub(crate) mod shell;`.
3. Tests (in `shell.rs`): port both upstream suites —
   - **builder**: empty ⇒ `b""`; `append_arg(b"bash")` ⇒ `b"bash"`; `bash` +
     `--posix` + `-l` ⇒ `b"bash --posix -l"`; `bash` + `b""` (empty skipped) ⇒
     `b"bash"`; `into_bytes` of `bash`
     - `--posix` ⇒ `b"bash --posix"`.
   - **escape**: `b"abc"`→`b"abc"`; `b"a c"`→`b"a\\ c"`; `b"a?c"`→`b"a\\?c"`;
     `b"a\\c"`→`b"a\\\\c"`; `b"a|c"`→`b"a\\|c"`; `b"a\"c"`→`b"a\\\"c"`;
     `b"a(1)"`→`b"a\\(1\\)"`; plus a linefeed passes through unescaped
     (`b"a\nc"`→`b"a\nc"`).
   - **full escape set** (Codex design review): a table test confirming each of
     the exact 11 characters (``\ " ' $ ` * ? <space> | ( )``) is
     backslash-prefixed when escaped alone (covering `'`, `$`, `` ` ``, `*`,
     which the upstream examples omit), and a representative non-special byte
     (e.g. `b'a'`) is not.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty os::shell
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/os/shell.rs roastty/src/os/mod.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `ShellCommandBuilder` joins arguments with single spaces (skipping empties)
  and `shell_escape` backslash-escapes exactly the upstream character set
  (linefeed excluded) — faithful to `os/shell.zig`;
- both ported test suites pass, and the existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the builder separation, the empty-arg skip, or the
escape set diverges from upstream, an unrelated item changes, or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it (no
Required findings), with **one Optional** suggestion, adopted:

- **(Optional, adopted)**: add a table test covering the full 11-character
  escape set, since the upstream examples exercise space / `?` / `\` / `|` / `"`
  / `(` / `)` but not `'`, `$`, `` ` ``, or `*`. A full-set test is added (see
  Tests).

Codex confirmed the builder semantics are faithful (empty args skipped, spaces
only between non-empty args), byte-oriented `&[u8]` / `Vec<u8>` is the right
Rust shape, dropping the `[:0]` NUL sentinel is fine (callers add it via
`CString`), and collapsing the `Writer` wrapper into a pure `shell_escape`
preserves the behavior.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d554-prompt.md` (design)
- Result: `logs/codex-review/20260604-d554-last-message.md` (design)

## Result

**Result:** Pass

`os::shell` was opened with `ShellCommandBuilder` (a `Vec<u8>` builder:
`append_arg` skips empty args and inserts a single space only between non-empty
args; `as_bytes` / `into_bytes`) and `shell_escape(&[u8]) -> Vec<u8>`
(backslash-prefixes exactly the 11 special bytes
``\ " ' $ ` * ? <space> | ( )``, everything else — incl. linefeed — passes
through). The module is registered in `os/mod.rs`. Eight tests: the builder
suite (empty, single, multiple, empty-arg skip, `into_bytes`), the upstream
escape examples, a linefeed-passthrough, and the full 11-character escape-set
table (covering `'`, `$`, `` ` ``, `*` that the examples omit).

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3081 passed, 0 failed (eight new tests; no
  regressions, up from 3073).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + os/shell.rs + os/mod.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **one Nit** (no
Required or Optional findings): the doc had `## Result` but no `## Conclusion` —
fixed by adding the conclusion below. Codex confirmed the implementation matches
upstream — the builder's empty-skip and single-space separation are faithful,
`shell_escape` covers the exact 11-byte escape set with linefeed and ordinary
bytes passing through, and the byte-oriented `Vec<u8>` adaptation is
appropriate; the added full-set table test closes the earlier coverage gap.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r554-prompt.md` (result)
- Result: `logs/codex-review/20260604-r554-last-message.md` (result)

## Conclusion

`os::shell` is opened with `ShellCommandBuilder` (space-separated command
construction, empty-arg skipping) and `shell_escape` (backslash-escaping the 11
shell-special characters, linefeed deliberately excluded), faithfully ported
from `os/shell.zig`. roastty will use these to build shell-integration / launch
command strings safely (wiring deferred). Both types are fully ported on the
macOS arm — `shell.zig` is complete. The OS-utility frontier still has a couple
of self-contained slices (`locale`, `i18n_locales`). The objc/bundle-id helpers,
the `home()` resolver, and config `loadDefaultFiles` remain deferred pending
roastty's naming decision; `background-image-opacity` stays float-blocked.
