# Experiment 152: Port Charset Escape Controls

## Description

Port Ghostty's runtime charset designation and invocation behavior into Roastty.

Roastty already has most of the passive charset foundation:

- `terminal::charsets` stores UTF-8, ASCII, British, and DEC special graphics
  tables;
- `ScreenCharsetState` stores G0-G3, GL, and GR state;
- `ScreenFormatterExtra::charsets` can serialize charset designations and
  invocations;
- Experiment 151 proved RIS resets charset state.

What is missing is the active runtime path:

- parsing ESC charset designation sequences such as `ESC ( 0`;
- parsing locking and single-shift invocation sequences;
- applying the active GL charset while printing cells.

Upstream Ghostty source references:

- `vendor/ghostty/src/terminal/stream.zig`:
  - `configureCharset()` maps intermediates `(`, `)`, `*`, `+` to G0-G3;
  - ESC finals `B`, `A`, and `0` configure ASCII, British, and DEC special;
  - `ESC n`, `ESC o`, `ESC ~`, `ESC }`, `ESC |`, `ESC N`, and `ESC O` dispatch
    charset invocation actions.
- `vendor/ghostty/src/terminal/Terminal.zig`:
  - `configureCharset()` stores the charset in the selected slot;
  - `invokeCharset()` updates GL/GR or single-shift state;
  - `printCell()` maps printable characters through the active GL charset, with
    single-shift state consumed by exactly one printed character.

Ghostty currently has a `TODO` for GR/non-UTF-8 handling in `printCell()`. This
experiment should mirror that boundary: implement active GL and single-shift
print mapping, store GR invocation state for formatter round-tripping, but do
not invent GR print behavior that Ghostty has not implemented.

## Changes

1. Extend charset value support.
   - Move the existing charset mapping tables out from `#[cfg(test)]` so runtime
     printing can use them.
   - Keep table access internal to `terminal::charsets`; do not expose public
     ABI.
   - Keep UTF-8 and ASCII as identity mappings.
   - For codepoints outside the u8 charset-table range printed through a mapped
     charset, follow Ghostty's current behavior and write a space. Codepoints
     inside the u8 range use the active charset table.

2. Extend screen charset state.
   - Add single-shift state to `ScreenCharsetState`, matching Ghostty's
     `single_shift: ?charsets.Slots`.
   - Add screen helpers to:
     - configure G0-G3;
     - invoke a slot into GL;
     - invoke a slot into GR;
     - single-shift G2 or G3 for exactly one printed character.
   - Ensure save/restore cursor includes the full charset state, including
     single-shift state, because Experiment 150 already stores charset in saved
     cursor state.
   - Ensure RIS reset clears single-shift state via the default charset state.

3. Apply charset mapping during printing.
   - Before writing a printable cell, map the incoming character through the
     active GL slot, or through the pending single-shift slot if one is set.
   - Consume pending single-shift state whether the mapped character writes
     successfully or fails due to a managed-cell/codepoint error, matching
     Ghostty's "use the key exactly once" model.
   - Do not mark cells dirty merely for charset configuration or invocation;
     only actual printed cells should dirty rows.

4. Extend stream parser actions.
   - Add actions equivalent to `ConfigureCharset` and `InvokeCharset`.
   - Preserve exact parsing for already-supported ESC actions.
   - Change ESC intermediate handling so one intermediate byte can be used for
     charset designation instead of treating all intermediates as invalid.
   - Accept only one intermediate byte for designation. Multi-intermediate
     sequences such as `ESC ( ( B` must be ignored and must not leak the final
     byte as printable text.
   - Dispatch:
     - `ESC ( B`, `ESC ) B`, `ESC * B`, `ESC + B` as ASCII designations;
     - `ESC ( A`, `ESC ) A`, `ESC * A`, `ESC + A` as British designations;
     - `ESC ( 0`, `ESC ) 0`, `ESC * 0`, `ESC + 0` as DEC special designations.
   - Ignore unsupported designation finals and unsupported designation
     intermediates.
   - Dispatch invocation controls:
     - `SI` (`0x0f`) invokes G0 into GL;
     - `SO` (`0x0e`) invokes G1 into GL;
     - `ESC n` invokes G2 into GL;
     - `ESC o` invokes G3 into GL;
     - `ESC N` single-shifts G2 for one character;
     - `ESC O` single-shifts G3 for one character;
     - `ESC ~` invokes G1 into GR;
     - `ESC }` invokes G2 into GR;
     - `ESC |` invokes G3 into GR.

5. Wire terminal runtime handling.
   - On `ConfigureCharset`, call the screen charset configuration helper.
   - On GL/GR invocation, call the corresponding screen invocation helper.
   - On single shift, set pending single-shift state.
   - Do not write PTY responses for charset controls.
   - Do not change title, pwd, tabstops, modes, DCS/APC/OSC handling, or RIS
     behavior.

## Verification

1. Run formatting:

   ```bash
   cargo fmt
   ```

2. Run focused tests:

   ```bash
   cargo test -p roastty charset
   cargo test -p roastty invoke_charset
   ```

3. Run the full Roastty test suite:

   ```bash
   cargo test -p roastty
   ```

Required test coverage:

- Stream parser tests:
  - all four G slots dispatch for ASCII, British, and DEC special designation;
  - unsupported designation final dispatches nothing;
  - unsupported designation intermediate dispatches nothing;
  - multi-intermediate designation dispatches nothing and consumes the final
    byte;
  - split-feed `ESC (` followed by final byte works;
  - handler-error recovery restores parser ground state before returning the
    error;
  - `SI`, `SO`, `ESC n`, `ESC o`, `ESC N`, `ESC O`, `ESC ~`, `ESC }`, and
    `ESC |` dispatch invocation actions;
  - existing `ESC 7`, `ESC 8`, `ESC M`, `ESC c`, DCS, OSC, APC, and CSI behavior
    still dispatches as before.
- Runtime tests:
  - configuring G1-G3 alone does not affect printing while GL remains G0;
  - `ESC ( 0` maps DEC special characters through G0 while printing;
  - `ESC ( A` maps British `#` to `£`;
  - `ESC ( B` restores ASCII identity mapping;
  - non-ASCII codepoints remain unchanged under the default UTF-8 charset state;
  - non-ASCII codepoints remain unchanged after `ESC ( B` / ASCII designation;
  - codepoints outside the u8 table range printed through a mapped charset
    become spaces;
  - `SO`/`SI` switch between G1 and G0 for GL;
  - `ESC n` and `ESC o` lock G2/G3 into GL;
  - `ESC N` and `ESC O` affect exactly one printed character and then restore
    the prior GL behavior;
  - GR invocation state round-trips through the VT formatter charset extra, but
    does not affect printable Unicode input;
  - save/restore cursor round-trips charset designation, GL/GR invocation, and
    single-shift state;
  - RIS resets designations, GL/GR, and single-shift state;
  - charset controls do not dirty rows, while printed mapped cells do;
  - charset controls write no PTY responses.

## Non-Negotiable Invariants

- Do not add public ABI or app integration.
- Do not add Linux or other non-macOS platform paths.
- Do not invent GR print behavior beyond Ghostty's current TODO boundary.
- Do not regress formatter charset extras added before this experiment.
- Do not regress saved cursor charset restore from Experiment 150.
- Do not regress RIS charset reset from Experiment 151.
- Do not add `ghostty_*` names. Use Roastty names except when citing upstream
  Ghostty source paths or behavior.
- Run `cargo fmt` and accept its output.

## Failure Criteria

This experiment fails if:

- ESC charset designations are ignored or parsed as printable text.
- Unsupported designation forms leak their final byte as text.
- GL or single-shift invocation maps the wrong slot.
- Single shift persists for more than one printed character.
- Charset configuration or invocation dirties rows without printing.
- Charset controls write PTY responses.
- Save/restore cursor loses charset state that Ghostty preserves.
- RIS leaves charset designation, GL/GR, or single-shift state stale.
- The patch adds public ABI, renderer/app behavior, PTY behavior, browser
  overlay behavior, or non-macOS platform paths.

## Design Review

Initial Codex review found one real design gap: the draft required non-ASCII
codepoints printed through mapped charsets to become spaces, but did not require
tests proving default UTF-8 and ASCII-designated GL remain identity mappings for
non-ASCII input. The verification plan was updated to require both tests.

Follow-up Codex review approved the design with no findings.

## Result

**Result:** Pass

Roastty now parses and executes runtime charset designation and invocation
controls.

Implemented:

- moved ASCII, British, and DEC special graphics charset tables into runtime
  code while keeping them internal to `terminal::charsets`;
- added a `CharsetBank` value for GL/GR invocation actions;
- added `ScreenCharsetState::single_shift`;
- added screen helpers for G0-G3 configuration, GL/GR invocation, and
  single-shift invocation;
- mapped printed cells through the active GL charset or one-shot single-shift
  slot before writing the active cell;
- consumed single-shift state exactly once;
- preserved Ghostty's current GR boundary by storing GR invocation state for VT
  formatter round-tripping without applying GR to printable input;
- added stream actions for charset configuration and invocation;
- parsed `ESC (`, `ESC )`, `ESC *`, and `ESC +` designations for ASCII, British,
  and DEC special charsets;
- parsed `SI`, `SO`, `ESC n`, `ESC o`, `ESC N`, `ESC O`, `ESC ~`, `ESC }`, and
  `ESC |` charset invocation controls;
- wired terminal runtime handling without PTY responses, dirty rows, ABI
  changes, app integration, PTY changes, or browser overlay changes.

During implementation, the design text was corrected to match Ghostty's actual
u8-table behavior: UTF-8 and ASCII remain identity mappings, mapped charsets use
the table for u8-range codepoints, and only codepoints outside the u8 table
range become spaces.

The implementation also necessarily allowed completed non-ASCII Unicode scalar
values to print as single cells. This is still not full Unicode width parity;
wide-cell handling remains future terminal work.

Verification:

```bash
cargo fmt
cargo test -p roastty charset
cargo test -p roastty invoke_charset
cargo test -p roastty
```

Results:

- `cargo test -p roastty charset`: 28 passed
- `cargo test -p roastty invoke_charset`: 3 passed
- `cargo test -p roastty`: 1672 unit tests passed, 1 ABI harness test passed, 0
  doc tests

## Conclusion

Experiment 152 completes the active GL charset runtime path and the parser
controls needed to configure and invoke G0-G3 charsets. Roastty now has both the
formatter-side charset state from earlier experiments and the runtime behavior
that makes common DEC special graphics and British charset sequences visible in
printed cells.

The next experiment can move to another coherent terminal parser/runtime slice;
charset GR rendering remains intentionally deferred because upstream Ghostty
also leaves non-UTF-8 GR print behavior as a TODO.

## Result Review

Initial Codex result review found two real coverage gaps: save/restore coverage
did not explicitly prove GL/GR invocation state survived, and handler-error
recovery coverage did not include an ESC-based charset invocation action.

Both gaps were fixed:

- `stream_escape_invoke_charset_restores_ground_before_handler_error` covers
  parser recovery after an `ESC N` `InvokeCharset` handler error.
- `terminal_stream_charset_save_restore_preserves_gl_and_gr_invocation` proves
  GL affects printing after restore and GR round-trips through VT formatter
  extras after restore.

Follow-up Codex result review approved the implementation and result with no
findings.
