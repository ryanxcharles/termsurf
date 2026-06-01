# Experiment 153: Port CSI Repeat Previous Character

## Description

Port Ghostty's repeat-previous-character control (`REP`, `CSI Ps b`) into
Roastty.

Upstream Ghostty source references:

- `vendor/ghostty/src/terminal/stream.zig`:
  - `Action` includes `print_repeat: usize`;
  - CSI final `b` with no intermediates dispatches `.print_repeat`;
  - no params means repeat once;
  - one param dispatches that value;
  - multiple params are rejected.
- `vendor/ghostty/src/terminal/Terminal.zig`:
  - terminal state stores `previous_char: ?u21`;
  - `print()` saves the unmapped printable codepoint in `previous_char`;
  - `printRepeat()` repeats `previous_char` through the normal `print()` path;
  - repeat count is clamped to at least one;
  - no previous character is a no-op;
  - RIS resets `previous_char` to `null`.

This is a good next slice because Experiment 151 deliberately left
previous-character state out of RIS, and Experiment 152 made the distinction
between unmapped input characters and mapped printed cells visible through
charset handling.

## Changes

1. Add stream action support.
   - Add `Action::PrintRepeat { count }`.
   - Parse `CSI b` as repeat count `1`.
   - Parse `CSI 0 b` as count `0` at the parser boundary; terminal runtime
     clamps it to one, matching Ghostty's `printRepeat()`.
   - Parse `CSI n b` as count `n`.
   - Reject multiple params, private CSI markers, CSI intermediates, colon
     params, mixed separators, and params the existing Roastty CSI parser marks
     invalid.
   - Keep raw C1 CSI bytes out of scope: they must not dispatch REP and must
     continue following Roastty's existing raw-C1 behavior.
   - Ensure rejected forms consume their final byte and do not leak printable
     text.
   - Restore parser ground state before calling the handler so handler errors do
     not strand the parser inside CSI.

2. Add terminal previous-character state.
   - Add `previous_char: Option<char>` to `Terminal`.
   - Pass it into `TerminalStreamHandler`.
   - On successful printable input entry, store the unmapped input character
     before charset mapping, matching Ghostty's `print()` / `printCell()` split.
   - Do not update `previous_char` for charset controls, CSI/OSC/DCS controls,
     line movement, erase/mutation controls, PTY query responses, or REP itself
     unless the repeated character goes through the normal `print()` path.

3. Implement REP runtime behavior.
   - If `previous_char` is `None`, do nothing and do not dirty rows.
   - If it is set, repeat it `max(count, 1)` times by calling the same print
     helper used for normal printable input.
   - Let the normal print path handle insert mode, wraparound mode, horizontal
     margins, pending wrap, charset mapping, style, hyperlinks, and managed-cell
     errors.
   - Preserve Ghostty's unmapped-character behavior: charset mapping happens at
     repeat time through the then-active GL/single-shift state. A character that
     originally printed as DEC special graphics may repeat differently after GL
     changes.

4. Reset behavior.
   - RIS / full reset must clear `previous_char`.
   - Save/restore cursor must not save or restore `previous_char`; Ghostty
     stores it as terminal-global state, not cursor state.

## Verification

Run:

```bash
cargo fmt
cargo test -p roastty print_repeat
cargo test -p roastty repeat_previous
cargo test -p roastty ris
cargo test -p roastty
```

Required test coverage:

- Stream parser tests:
  - `CSI b` dispatches `PrintRepeat { count: 1 }`;
  - `CSI 0 b` dispatches count `0`;
  - `CSI 3 b` dispatches count `3`;
  - multiple-param semicolon forms such as `CSI 1 ; 2 b`, `CSI ; b`, and
    `CSI 3 ; b` dispatch nothing and do not leak the final byte;
  - colon and mixed-separator forms such as `CSI 1 : 2 b` and `CSI 1 ; 2 : 3 b`
    dispatch nothing and do not leak the final byte;
  - private markers, CSI intermediates, and parser-invalid params dispatch
    nothing and do not leak the final byte;
  - raw C1 CSI bytes dispatch no REP action and preserve existing raw-C1
    behavior;
  - split-feed `CSI` REP works;
  - handler-error recovery restores parser ground state before returning the
    error;
  - existing CSI families continue dispatching as before.
- Runtime tests:
  - REP with no previous character is a no-op and does not dirty rows;
  - `A CSI b` produces `AA`;
  - `A CSI 0 b` produces `AA` because runtime clamps zero to one;
  - `A CSI 3 b` produces `AAAA`;
  - REP uses the normal print path and wraps at the right edge;
  - REP respects disabled wraparound mode by preserving the normal overwrite
    behavior at the right edge;
  - REP uses current style/hyperlink state by virtue of the normal print path;
  - REP stores/repeats the unmapped prior character and maps it through the
    current charset at repeat time;
  - single-shift charset state affects the repeated character exactly once if it
    is pending at repeat time;
  - REP itself updates `previous_char` through the normal print path, so a later
    REP can repeat the same character again;
  - save/restore cursor does not restore stale `previous_char`;
  - RIS clears `previous_char`.

## Non-Negotiable Invariants

- Do not add public ABI or app integration.
- Do not add Linux or other non-macOS platform paths.
- Do not add wide-cell, grapheme-cluster, alternate-screen, status-display, or
  Kitty graphics behavior in this experiment.
- Do not change existing charset mapping semantics from Experiment 152.
- Do not change existing CSI parameter semantics for unrelated finals.
- Do not add `ghostty_*` names. Use Roastty names except when citing upstream
  Ghostty source paths or behavior.
- Run `cargo fmt` and accept its output.

## Failure Criteria

This experiment fails if:

- `CSI b` is ignored or leaks `b` as printable text.
- Rejected REP forms leak their final byte as text.
- `CSI 0 b` becomes a no-op instead of repeating once at runtime.
- REP repeats the mapped cell contents instead of the previous unmapped
  character through the current print path.
- REP bypasses normal print behavior for style, hyperlink, insert mode,
  wraparound, pending wrap, margins, charset mapping, or errors.
- REP with no previous character dirties rows.
- Save/restore cursor incorrectly saves/restores `previous_char`.
- RIS leaves stale `previous_char`.
- The patch adds public ABI, renderer/app behavior, PTY behavior, browser
  overlay behavior, or non-macOS platform paths.

## Design Review

Initial Codex review found two real design issues:

- the draft mentioned overflowing params without grounding that behavior in the
  cited Ghostty source or Roastty parser boundary;
- the separator rejection coverage was too broad and needed concrete semicolon,
  colon, and mixed-separator cases.

The design was updated to scope invalid numeric behavior to params the existing
Roastty CSI parser marks invalid, and to require concrete negative cases for
`CSI 1 ; 2 b`, `CSI ; b`, `CSI 3 ; b`, `CSI 1 : 2 b`, and `CSI 1 ; 2 : 3 b`.

Follow-up Codex review approved the design with no findings.

During implementation planning, the raw-C1 requirement was tightened: raw C1 CSI
bytes remain out of scope and should preserve existing raw-C1 behavior while
dispatching no REP action. Codex reviewed and approved that correction with no
findings.
