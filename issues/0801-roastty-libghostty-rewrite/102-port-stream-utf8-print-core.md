# Experiment 102: Port Stream UTF-8 Print Core

## Description

Start the terminal runtime input path by porting the first narrow slice of
upstream Ghostty's `terminal/stream.zig`: UTF-8 print decoding and action
dispatch.

Experiments 90-101 completed formatter-side terminal state serialization for the
active screen and terminal-level extras. The next major subsystem is runtime
mutation: bytes from a PTY must be decoded into stream actions, then applied to
terminal state. Upstream's `stream.zig` is large, so this experiment only builds
the foundational stream/handler shape and the ground-state UTF-8 print path. It
must not implement CSI, OSC, DCS, APC, ESC, terminal mutation, PTY IO, public
API, public ABI, renderer behavior, app behavior, or UI behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/stream.zig` for:
     - `Action.print`;
     - `Stream(H)` handler dispatch shape;
     - `nextSlice()` / incremental input behavior;
     - UTF-8 decoding behavior;
     - invalid UTF-8 replacement behavior.
   - Use `vendor/ghostty/src/terminal/UTF8Decoder.zig` for decoder semantics.
   - Do not modify `vendor/ghostty/`.

2. Add a private stream module.
   - Add `roastty/src/terminal/stream.rs`.
   - Export it only inside `terminal/mod.rs`.
   - Keep the module private to the crate/subsystem. Do not expose public API or
     ABI.
   - Add a private `Action` enum with at least:

     ```rust
     Action::Print { cp: char }
     ```

   - Use Rust `char` for decoded Unicode scalar values. Invalid UTF-8 must
     dispatch `U+FFFD` as a print action, matching upstream's replacement
     behavior.

3. Add a handler trait or equivalent private dispatch shape.
   - Preserve upstream's conceptual boundary: stream parsing emits actions; a
     handler receives those actions.
   - The first implementation may use a trait, callback, or small generic type,
     but it must keep parsing independent from terminal mutation.
   - Do not call into `Terminal` yet.
   - Do not add `Terminal::vt_stream()` yet.

4. Implement incremental UTF-8 print decoding.
   - `Stream::next_slice(&[u8])` processes byte slices.
   - Complete valid UTF-8 sequences dispatch one `Action::Print` per scalar.
   - ASCII printable bytes dispatch directly as print actions.
   - Split multi-byte UTF-8 sequences across calls are buffered until complete.
   - Invalid UTF-8 dispatches `U+FFFD` with upstream retry semantics:
     - if the rejecting byte is part of the invalid sequence, consume it;
     - if the rejecting byte is a new possible starter byte, emit `U+FFFD` for
       the invalid pending sequence but retry that same byte as the start of the
       next decode attempt.
   - At end-of-input, incomplete UTF-8 is not dispatched until another call
     either completes it or proves it invalid.
   - This experiment may use Rust's standard UTF-8 validation primitives rather
     than porting Ghostty's exact SIMD path. Do not add SIMD in this slice.

5. Explicitly defer escape and control handling.
   - C0 controls other than ESC and DEL (`0x7f`) are ignored in this slice.
   - Raw bytes in the `0x80..=0x9f` C1 range are handled by the UTF-8 decoder,
     because the stream is decoding UTF-8 bytes. A standalone C1 byte therefore
     dispatches `U+FFFD` as invalid UTF-8 instead of being treated as a terminal
     control action.
   - ESC (`0x1b`) starts a minimal unsupported-escape state and must not leak
     subsequent escape bytes as printable text.
   - For unsupported CSI-looking input such as `ESC [ C`, consume through the
     final byte and return to ground state without dispatching print actions.
   - For direct unsupported ESC final-byte input such as `ESC c`, consume the
     final byte and return to ground state without dispatching print actions.
   - This minimal state exists only to keep unsupported escape/control syntax
     from being misclassified as text. It is not a CSI/OSC/DCS/APC
     implementation.
   - Do not implement CSI, OSC, DCS, APC, parser state machine, modes, cursor
     movement, tab mutation, PWD mutation, keyboard mutation, screen writes, or
     terminal writes.
   - If the stream sees an unsupported escape/control byte, behavior must be
     documented in tests so later parser slices can replace it deliberately.

6. Add upstream-equivalent tests.
   - Add stream tests for:
     - ASCII text dispatches one print action per character;
     - Unicode scalar values dispatch correctly;
     - a multi-byte scalar split across `next_slice()` calls dispatches only
       after the final byte arrives;
     - invalid UTF-8 dispatches `U+FFFD`;
     - partial-invalid UTF-8 retries a rejecting starter byte instead of
       dropping it, matching upstream `UTF8Decoder.zig` behavior;
     - incomplete UTF-8 held at a slice boundary completes correctly on the next
       slice;
     - unsupported C0 controls and DEL do not masquerade as printable text;
     - raw C1-range bytes are handled by the UTF-8 decoder and do not become
       terminal control actions;
     - unsupported direct ESC final-byte sequences do not leak their final byte
       as printable text;
     - unsupported CSI-shaped escape sequences such as `ESC [ C` consume the
       whole unsupported sequence and do not leak `[` or `C` as printable text;
     - parser state remains usable after invalid UTF-8 and after ignored
       unsupported bytes.
   - Keep existing formatter, tabstops, modes, ScreenFormatter, PageList
     formatter, and PageList tests passing.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty stream
     cargo test -p roastty terminal_formatter
     cargo test -p roastty modes
     cargo test -p roastty tabstops
     cargo test -p roastty screen_formatter
     cargo test -p roastty styled_pin_map
     cargo test -p roastty pin_map
     cargo test -p roastty page_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - stream module visibility;
     - action/handler shape;
     - UTF-8 decoding and replacement behavior;
     - unsupported escape/control behavior;
     - why terminal mutation, CSI, OSC, PTY, public API, and ABI remain
       deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `roastty/src/terminal/stream.rs` exists and is private to the terminal
  subsystem;
- stream parsing emits print actions through a private handler boundary;
- ASCII and Unicode text dispatch as print actions;
- split UTF-8 sequences are buffered across `next_slice()` calls;
- invalid UTF-8 emits `U+FFFD` with upstream retry semantics for rejecting
  starter bytes;
- unsupported control and escape behavior is explicit and tested;
- no terminal mutation, CSI parser, OSC parser, DCS parser, APC parser, PTY IO,
  public API, public ABI, renderer behavior, app behavior, or UI behavior is
  added;
- `cargo fmt`, stream tests, formatter tests, tabstops tests, modes tests,
  PageList formatter tests, PageList tests, and full `cargo test -p roastty`
  pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- UTF-8 decoding cannot be represented without first porting Ghostty's parser
  state machine or UTF-8 decoder module, and that prerequisite is identified
  precisely.

The experiment fails if:

- stream parsing is coupled directly to `Terminal` mutation in this slice;
- invalid UTF-8 is silently dropped or panics;
- split valid UTF-8 emits replacement characters before enough bytes arrive;
- invalid UTF-8 consumes a rejecting starter byte that upstream would retry;
- unsupported ESC/control bytes or bytes inside unsupported escape sequences are
  treated as normal printable text;
- public API or ABI changes are added;
- formatter or existing terminal storage behavior regresses.

## Design Review

Codex reviewed this design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-003456-369584-prompt.md`
- Result: `logs/codex-review/20260601-003456-369584-last-message.md`

Codex found two real design gaps:

- invalid UTF-8 behavior had to specify Ghostty's retry-the-rejecting-byte
  semantics instead of only saying "resume at the next valid boundary";
- unsupported ESC/control behavior had to be concrete enough to prevent
  unsupported sequences such as `ESC [ C` from leaking `[` or `C` as printable
  text.

Both findings were applied.

Re-review artifacts:

- Prompt: `logs/codex-review/20260601-003712-140875-prompt.md`
- Result: `logs/codex-review/20260601-003712-140875-last-message.md`

Codex found no remaining blocking findings and approved implementation.

## Result

**Result:** Pass.

Implemented the first private stream slice in `roastty/src/terminal/stream.rs`
and wired it into the terminal subsystem with a private `mod stream`
declaration. The module is not part of the public Roastty API or ABI.

The stream uses a private `Action::Print { cp: char }` enum and a private
`Handler` trait with `vt(&mut self, action: Action)`. This preserves the
upstream boundary: byte parsing emits stream actions, and terminal mutation
remains deferred to a later experiment.

The implementation supports incremental UTF-8 print decoding through
`Stream::next_slice(&[u8])`:

- ASCII and complete Unicode scalar values dispatch one print action per scalar.
- Split multi-byte sequences are buffered across calls and dispatch only after
  the final byte arrives.
- Invalid UTF-8 dispatches `U+FFFD`.
- If a pending sequence rejects a byte that can start the next scalar, the
  stream emits `U+FFFD` for the pending sequence and retries the rejecting byte
  instead of dropping it. Tests cover both a retried multi-byte starter and a
  retried ASCII byte.
- If a pending sequence rejects an ESC, C0 control, or DEL byte, the stream
  first emits `U+FFFD` for the pending UTF-8 sequence and then handles the
  rejecting byte as terminal syntax. This preserves upstream's
  retry-the-rejecting-byte semantics without leaking unsupported terminal syntax
  as printable text.
- Incomplete UTF-8 at a slice boundary is held until a later call completes it
  or proves it invalid.

Unsupported terminal syntax is deliberately minimal and tested:

- C0 controls other than ESC and DEL are ignored.
- Raw `0x80..=0x9f` bytes are treated as UTF-8 input, not as terminal control
  actions; standalone invalid bytes dispatch `U+FFFD`.
- ESC starts a small unsupported-escape state.
- Direct unsupported ESC final-byte input such as `ESC c` is consumed without
  leaking the final byte as printable text.
- CSI-shaped unsupported input such as `ESC [ C` is consumed through the final
  byte and then returns to ground state.

This experiment did not add terminal mutation, CSI parsing, OSC parsing, DCS
parsing, APC parsing, PTY IO, public API, public ABI, renderer behavior, app
behavior, or UI behavior.

Verification run:

```text
cargo fmt
cargo test -p roastty stream
cargo test -p roastty terminal_formatter
cargo test -p roastty modes
cargo test -p roastty tabstops
cargo test -p roastty screen_formatter
cargo test -p roastty styled_pin_map
cargo test -p roastty pin_map
cargo test -p roastty page_string
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Results:

- `cargo fmt` passed.
- `cargo test -p roastty stream` passed 72 tests.
- `cargo test -p roastty terminal_formatter` passed 67 tests.
- `cargo test -p roastty modes` passed 20 tests.
- `cargo test -p roastty tabstops` passed 18 tests.
- `cargo test -p roastty screen_formatter` passed 55 tests.
- `cargo test -p roastty styled_pin_map` passed 9 tests.
- `cargo test -p roastty pin_map` passed 65 tests.
- `cargo test -p roastty page_string` passed 12 tests.
- `cargo test -p roastty terminal::page_list` passed 524 tests.
- Full `cargo test -p roastty` passed 973 unit tests, the ABI harness, and
  doc-tests.

Codex design review passed after the two design findings above were fixed. Codex
result review found one real upstream-fidelity bug: pending UTF-8 was silently
dropped if the rejecting byte was ESC, C0, or DEL.

Initial result-review artifacts:

- Prompt: `logs/codex-review/20260601-004839-680527-prompt.md`
- Result: `logs/codex-review/20260601-004839-680527-last-message.md`

The implementation now emits `U+FFFD` before retrying the rejecting byte through
terminal syntax, and tests cover pending UTF-8 followed by C0, DEL, direct ESC,
and CSI-shaped ESC input.

Clean result re-review artifacts:

- Prompt: `logs/codex-review/20260601-005142-208198-prompt.md`
- Result: `logs/codex-review/20260601-005142-208198-last-message.md`

Codex found no remaining correctness or upstream-fidelity findings and approved
the result as good enough to commit.

## Conclusion

Roastty now has the first runtime byte-stream boundary: a private, tested stream
parser that can decode printable UTF-8 input into print actions without touching
terminal state. The next stream/parser work can build on this by adding real
parser states and terminal mutation one narrow slice at a time, while keeping
the action/handler boundary intact.
