# Experiment 143: Port OSC 3008 Context Signals

## Description

Experiments 141-142 finished notification and clipboard-facing OSC parser
surfaces. The next coherent OSC parser slice is OSC 3008 hierarchical context
signalling.

Ghostty parses this in:

- `vendor/ghostty/src/terminal/osc/parsers/context_signal.zig`

OSC 3008 lets programs announce hierarchical execution contexts such as shells,
containers, VMs, commands, sessions, and privilege changes. Ghostty parses these
signals into terminal commands with raw metadata fields that can be read lazily.

This experiment ports parser/stream recognition and terminal runtime no-op
handling only. Roastty does not yet have an app/surface context-stack boundary,
so the terminal must explicitly ignore context-signal actions for now. Do not
add public ABI, context stack state, UI highlighting, shell integration
behavior, or app/surface event delivery in this experiment.

## Changes

1. Add a context-signal terminal module.

   Add `roastty/src/terminal/context_signal.rs` and register it from
   `roastty/src/terminal/mod.rs`.

   Include:
   - `ContextSignal<'a> { action, id, metadata }`;
   - `Action::{Start, End}`;
   - `ContextType` values matching Ghostty:
     - `boot`
     - `container`
     - `vm`
     - `elevate`
     - `chpriv`
     - `subcontext`
     - `remote`
     - `shell`
     - `command`
     - `app`
     - `service`
     - `session`
   - `ExitStatus::{Success, Failure, Crash, Interrupt}`;
   - field readers matching Ghostty's `Field.read` behavior.

2. Implement lazy metadata field readers.

   Metadata is the raw byte slice after the context ID. Fields are
   semicolon-separated `key=value` pairs.

   Add field readers for:
   - start fields: `type`, `user`, `hostname`, `machineid`, `bootid`, `pid`,
     `pidfdid`, `comm`, `cwd`, `cmdline`, `vm`, `container`, `targetuser`,
     `targethost`, `sessionid`;
   - end fields: `exit`, `status`, `signal`.

   Match Ghostty's semantics:
   - unknown fields and fields without `=` are skipped;
   - the first matching key wins, even if its value is malformed;
   - matching is exact and case-sensitive;
   - string fields return `None` for empty values;
   - `type` and `exit` return `None` for unknown enum values;
   - numeric fields (`pid`, `pidfdid`, `status`) parse decimal `u64` only and
     return `None` on non-digits or overflow.

   The duplicate-field behavior is important: Ghostty does not continue looking
   for a later valid duplicate after it sees the requested key. For example,
   `pid=bad;pid=1`, `type=BAD;type=shell`, and `user=;user=root` all return
   `None` for their respective field readers.

   Keep metadata and string field values as raw bytes. Do not require UTF-8.

3. Parse OSC 3008.

   In `roastty/src/terminal/osc.rs`, add:

   ```rust
   Command::ContextSignal { value: context_signal::ContextSignal<'a> }
   ```

   Parse from the data after `3008;`:
   - `start=<id>[;<field>=<value>]*` -> `Action::Start`;
   - `end=<id>[;<field>=<value>]*` -> `Action::End`;
   - ID is bytes after `start=`/`end=` up to the next semicolon or end of data;
   - ID length must be 1-64 bytes;
   - every ID byte must be in the inclusive `0x20..=0x7e` range;
   - metadata is everything after the ID's separating semicolon, or empty when
     no metadata exists.

   Reject:
   - missing `3008;` separator;
   - empty data;
   - unknown action prefix;
   - empty ID;
   - over-64-byte ID;
   - ID containing bytes outside `0x20..=0x7e`.

   Implementation note: Roastty's generic OSC parser can split a command with no
   semicolon into an empty-rest path. The `3008` branch must explicitly require
   that the first semicolon was present, for example with
   `b"3008" if split.is_some()` or an equivalent guard. `OSC 3008 ST` and
   `OSC 3008; ST` are different invalid cases and both must reject.

4. Dispatch through the stream layer.

   Extend `OscAction` and the stream test harness so valid OSC 3008 actions
   reach the handler. Add tests for:
   - basic start;
   - basic end;
   - start with metadata;
   - end with metadata;
   - invalid forms consumed without dispatch or print leakage;
   - oversized OSC 3008 metadata consumed without dispatch or print leakage.

5. Ignore at terminal runtime.

   Add an explicit `TerminalStreamHandler` no-op arm for `ContextSignal`.

   Tests must prove context signals do not mutate display contents, title, PWD,
   hyperlink state, color state, cursor position, modes, dirty rows, or PTY
   response.

6. Keep scope limited.

   Do not implement:
   - app/surface context-stack delivery;
   - public C ABI for context signals;
   - UI highlighting based on context fields;
   - shell integration behavior;
   - semantic prompt handling.

   Semantic prompts already have row metadata infrastructure in Roastty and
   deserve a separate runtime-focused experiment.

## Verification

Run formatting and tests:

```bash
cargo fmt
cargo test -p roastty context_signal
cargo test -p roastty osc
cargo test -p roastty terminal_stream_osc
cargo test -p roastty
```

Add parser tests for valid OSC 3008:

- `3008;start=abc123` -> start action, ID `abc123`, empty metadata;
- `3008;end=abc123` -> end action, ID `abc123`, empty metadata;
- `3008;start=myctx;type=shell;user=root;hostname=myhost`;
- `3008;end=myctx;exit=failure;status=1;signal=SIGKILL`;
- raw non-UTF-8 metadata field values are preserved by string field readers.

Add field-reader tests:

- all `ContextType` values parse case-sensitively;
- all `ExitStatus` values parse case-sensitively;
- `pid`, `pidfdid`, and `status` parse decimal `u64`;
- numeric fields reject empty, non-digit, negative, and overflowing values;
- missing fields return `None`;
- unknown/malformed fields are skipped;
- duplicate matching fields stop at the first matching key, even when the first
  value is malformed:
  - `pid=bad;pid=1` -> `None`;
  - `type=BAD;type=shell` -> `None`;
  - `user=;user=root` -> `None`;
- empty string fields return `None`.

Add invalid parser tests:

- `3008` rejects;
- `3008;` rejects;
- `3008;bogus=abc123` rejects;
- `3008;start=` rejects;
- IDs longer than 64 bytes reject;
- IDs containing control bytes or bytes above `0x7e` reject.
- `3008;start=id;` followed by more than `osc::MAX_BUF` bytes rejects/consumes
  without dispatch or print leakage. OSC 3008 uses Ghostty's fixed capture path;
  it must not become a growable OSC family.

Add stream/runtime tests:

- stream dispatches valid start/end context signals;
- stream consumes invalid context signals without dispatch or print leakage;
- terminal runtime ignores context signals without mutating display contents,
  title, PWD, hyperlink state, color state, cursor position, modes, dirty rows,
  or PTY response.

## Pass Criteria

- OSC 3008 parser behavior matches Ghostty for start/end action parsing, ID
  validation, metadata splitting, and field readers.
- Context signal IDs and metadata are raw bytes; metadata fields do not require
  UTF-8.
- Invalid OSC 3008 forms do not dispatch actions or leak bytes into display
  output.
- Oversized OSC 3008 remains fixed-buffer rejected and does not become part of
  the growable OSC family.
- Terminal runtime explicitly ignores context signals without mutating unrelated
  state or emitting PTY replies.
- No app/surface/public-ABI/UI context behavior is added.
- Existing OSC behavior keeps passing.

## Failure Criteria

- OSC 3008 accepts unknown action prefixes, empty IDs, overlong IDs, or invalid
  ID bytes.
- Field readers become case-insensitive or accept malformed numeric values.
- Field readers skip past a malformed duplicate matching key to a later valid
  duplicate.
- Metadata values are required to be valid UTF-8.
- Invalid OSC 3008 forms dispatch actions or leak bytes into display output.
- Oversized OSC 3008 dispatches or leaks bytes instead of being fixed-buffer
  rejected.
- Terminal runtime mutates display, title, PWD, hyperlinks, color state, cursor
  position, dirty rows, modes, or PTY response for context signals.
- The experiment adds app/surface/public-ABI/UI context behavior.
- Semantic prompt behavior is mixed into this experiment.
- Existing OSC behavior regresses.

## Design Review

Codex reviewed the initial design and found three real issues:

- duplicate matching-field semantics were underspecified; Ghostty stops at the
  first matching key even when that value is malformed;
- fixed-buffer overflow coverage was missing, even though Ghostty captures OSC
  3008 with fixed storage rather than allocating storage;
- the Roastty parser implementation needed an explicit guard to distinguish
  missing `3008;` from `3008;` with empty data.

The design now pins first-matching-key behavior, adds duplicate malformed-value
tests, requires oversized OSC 3008 no-dispatch/no-leak coverage, and calls out
the implementation guard for the initial separator. Codex re-reviewed the
revised design and approved it for implementation with no blocking findings.
