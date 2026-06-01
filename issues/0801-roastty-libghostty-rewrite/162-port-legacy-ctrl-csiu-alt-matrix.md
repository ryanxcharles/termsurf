# Experiment 162: Port Legacy Ctrl/CSI-u/Alt Matrix

## Description

Experiments 160 and 161 established the pure key encoder and completed its
lookup-table layer. The remaining high-value key-encoding gap is the non-table
legacy matrix: `ctrlSeq`, fixterm/CSI-u fallback, translated layout behavior,
and alt-escape prefix edge cases.

The upstream source material is:

- `vendor/ghostty/src/input/key_encode.zig`
  - `ctrlSeq(...)`;
  - `CsiUMods`;
  - legacy fixterm/CSI-u fallback;
  - `altCodepoint(...)`;
  - tests from `legacy: backspace with utf8 (dead key state)` through the
    `ctrlseq:` tests.

This experiment should port that behavior into `roastty/src/input/key_encode.rs`
on top of the value types and tables already created. It remains a pure internal
encoder experiment and must not wire live input or add any public key ABI.

## Changes

1. Port full upstream `ctrlSeq(...)` behavior.
   - Replace the current representative-only `ctrl_seq(...)` with the upstream
     mapping:
     - space, slash, digits `0`-`9`, question mark, at sign, backslash, right
       bracket, caret, underscore, `a`-`z`, and tilde;
     - intentionally exclude `i`, `m`, and left bracket from C0 output so they
       fall through to CSI-u/fixterm behavior;
     - keep alt allowed but ignored for the C0 decision, so Ctrl+Alt+C becomes
       ESC + ETX through the existing alt-prefix step;
     - ignore lock and side modifiers through binding-mod comparison.
   - Preserve upstream shift handling:
     - remove shift for non-uppercase ASCII except the fixterm `@` case;
     - use `unshifted_codepoint` to lowercase caps-lock letters;
     - reject Ctrl+Shift+letter from C0 output so it falls through to CSI-u.

2. Port the legacy fixterm/CSI-u character fallback.
   - Complete the fallback that emits `ESC[{codepoint};{mods}u` for Ctrl cases
     that should not become C0 bytes.
   - Use only the shift/alt/ctrl bits in the CSI-u modifier code, matching
     upstream `CsiUMods`.
   - Preserve the consumed-modifier behavior used by Experiment 160's
     modifyOtherKeys tests.
   - Include non-US layout behavior where the physical logical key maps to an
     ASCII key but UTF-8 text is non-ASCII, such as Cyrillic/Hungarian examples.

3. Port alt-prefix edge cases.
   - Complete `legacy_alt_prefix(...)` / `altCodepoint(...)` parity:
     - UTF-8 single-byte text uses that byte;
     - empty UTF-8 may use `unshifted_codepoint`;
     - multi-byte translated text is not alt-prefixed unless an unshifted ASCII
       codepoint is available and macOS option-as-alt permits it;
     - macOS `OptionAsAlt::False`, `True`, `Left`, and `Right` behavior remains
       source-of-truth for whether Option is treated as Alt.
   - Keep the existing public option value type; do not add config parsing.

4. Add focused parity tests.
   - Port or create equivalent tests for:
     - dead-key UTF-8 cases for backspace, enter, and escape;
     - DEL UTF-8 backspace with DECBKM reset and set;
     - Ctrl+Shift+minus / underscore;
     - Ctrl+Alt+C;
     - Alt+C, Alt+E with only `unshifted_codepoint`, macOS translated Option
       text, Shift+Alt+period, and non-ASCII Alt text without usable ASCII
       fallback;
     - exact full C0 mapping coverage for space, slash, digits `0`-`9`, question
       mark, at sign, backslash, right bracket, caret, underscore, `a`-`z`, and
       tilde;
     - explicit negative C0 coverage proving Ctrl+I, Ctrl+M, and Ctrl+left
       bracket fall through to CSI-u/fixterm instead of C0 output;
     - Ctrl+C with right-side Ctrl;
     - Ctrl+C with caps lock;
     - Ctrl+Shift+C rejected from C0 output and encoded through CSI-u;
     - Russian/Cyrillic Ctrl+C and Ctrl+Alt+C behavior;
     - Hungarian/non-ASCII Ctrl layout behavior producing CSI-u.
   - Keep existing Experiment 160 and 161 tests passing.

5. Keep scope boundaries hard.
   - Do not modify the Kitty table or PC-style legacy table except for tests
     that must prove this experiment did not regress them.
   - Do not add public `roastty_key_event_t`, `roastty_key_encoder_t`, or any
     other key C ABI.
   - Do not add live Swift/macOS input, terminal runtime dispatch, PTY writes,
     keybindings, keymaps, config parsing, renderer behavior, browser overlay
     behavior, or `Options::from_terminal`.
   - Do not add non-macOS platform branches.

6. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix every real finding and re-review until Codex finds no remaining
     blocking design issues.
   - Record the design-review outcome in this experiment file before committing
     the design.
   - After implementation and verification, get Codex review of the completed
     result before committing the result.
   - Do not proceed to the next experiment until the completed result review is
     approved or every real result finding has been fixed and re-reviewed.

## Verification

Run:

```bash
cargo fmt -- roastty/src/input/key_encode.rs
cargo test -p roastty key_encode
cargo test -p roastty key_event
cargo test -p roastty kitty_keyboard
cargo test -p roastty
! rg -n "ghostty|Ghostty|ghostty_" roastty/src/input roastty/src/lib.rs
```

Required evidence:

- `ctrl_seq(...)` has exact C0 table coverage for space, slash, digits `0`-`9`,
  question mark, at sign, backslash, right bracket, caret, underscore, `a`-`z`,
  and tilde.
- `ctrl_seq(...)` has explicit negative coverage for Ctrl+I, Ctrl+M, and
  Ctrl+left bracket falling through to CSI-u/fixterm.
- `ctrl_seq(...)` matches upstream for caps lock, side modifiers, Alt+Ctrl,
  non-ASCII layout, and rejected Ctrl+Shift letter cases.
- CSI-u/fixterm fallback matches upstream for `i`, `m`, left bracket, Ctrl+Shift
  letters, Ctrl+Shift+`@`, and non-ASCII layout cases.
- Backspace with DEL UTF-8 honors DECBKM reset/set behavior.
- Alt-prefix behavior matches upstream for text, unshifted-codepoint-only,
  translated macOS Option text, side-specific Option-as-Alt, and non-ASCII text
  without an ASCII fallback.
- Existing table-parity tests from Experiments 160 and 161 still pass.
- The encoder remains pure and internal.
- No public ABI, live input path, PTY write path, keybinding/keymap/config
  behavior, runtime dispatch, renderer behavior, browser overlay behavior, or
  terminal-handle `Options::from_terminal` behavior is added.
- Codex design review and result review both pass before moving to the next
  stage.

## Non-Negotiable Invariants

- Use Roastty names in implementation-facing comments, tests, and modules.
- Keep the encoder pure and internal.
- Do not add public key C ABI.
- Do not wire live input, runtime dispatch, or PTY writes.
- Do not add keybindings, keymaps, config parsing, modifier remap config, or
  `Options::from_terminal`.
- Do not add non-macOS platform behavior.
- Do not change the external `roastty_*` ABI.
- Run `cargo fmt` and accept its output.
- Pass Codex design and result reviews before moving to the next stage.

## Failure Criteria

This experiment fails if:

- any public `ghostty_*` or compatibility key ABI names are introduced;
- the encoder sends bytes to PTY/runtime/app code instead of returning them;
- Ctrl C0 output diverges from upstream for letters, digits, punctuation, side
  modifiers, caps lock, Alt+Ctrl, or non-ASCII layout cases;
- CSI-u/fixterm fallback diverges from upstream for the cases named in this
  experiment;
- macOS option-as-alt handling is replaced with generic Alt handling;
- the experiment expands into live input, public ABI, config/keybinding,
  runtime, renderer, browser overlay, or terminal-handle wiring;
- existing key encoder, key event, Kitty keyboard runtime, terminal, mouse,
  formatter, or ABI tests regress;
- the design or result proceeds without the required Codex review gate.

## Codex Design Review

Codex reviewed the initial design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-143832-102613-prompt.md`
- Result: `logs/codex-review/20260601-143832-102613-last-message.md`

Codex found two real design issues:

- full C0 `ctrlSeq` coverage was too loose for a finite correctness-sensitive
  mapping;
- DEL UTF-8 backspace behavior under DECBKM reset/set was cited by the upstream
  source range but not explicitly required.

Both findings were fixed by requiring exact C0 table coverage, explicit negative
C0 fallthrough cases, and explicit DEL UTF-8 backspace DECBKM tests.

Clean design re-review artifacts:

- Prompt: `logs/codex-review/20260601-144016-662129-prompt.md`
- Result: `logs/codex-review/20260601-144016-662129-last-message.md`

Codex found no remaining design blockers and approved the experiment for
implementation.
