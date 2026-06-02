+++
[implementer]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 128: Port CSI Mode Set and Reset

## Description

Port Ghostty's basic SM/RM mode mutation commands:

- `CSI h` sets ANSI modes;
- `CSI l` resets ANSI modes;
- `CSI ? h` sets DEC modes;
- `CSI ? l` resets DEC modes.

Roastty already has the upstream-derived `modes::Mode` table, `ModeState`
storage, mode report encoding, and terminal formatter support from Experiments
98 and 101. Several execution paths already consult mode state through test-only
setup, including linefeed mode, origin mode, bracketed paste, and formatter
restoration. This experiment wires real stream input into that mode state.

This is a mode-state foundation experiment, not a full side-effect experiment
for every DEC private mode. Upstream Ghostty's `stream_terminal.zig::setMode`
always mutates `terminal.modes` first, then performs extra side effects for a
small set of modes. Roastty should follow that ordering, but only implement side
effects that the current terminal core can represent honestly:

- origin mode (`?6`) sets/resets the mode and moves the cursor to the current
  origin-home position, matching `setCursorPos(1, 1)`;
- resetting left/right margin mode (`?69l`) clears the horizontal margins to the
  full screen width;
- all dispatched modes mutate `ModeState` first, matching upstream's first
  execution step.

The following upstream side effects are deliberately deferred because their own
subsystems are not ported yet:

- insert-mode printing behavior, where printable characters insert blanks before
  writing instead of overwriting;
- linefeed-mode execution behavior, where LF adds carriage return behavior;
- wraparound-mode execution behavior, where disabled wraparound prevents
  automatic soft wrap;
- alternate-screen switching for `?47`, `?1047`, and `?1049`;
- save/restore cursor for `?1048`;
- DECCOLM / 80-column versus 132-column resize behavior for `?3`;
- mouse event mode/format derived flags for `?9`, `?1000`, `?1002`, `?1003`,
  `?1005`, `?1006`, `?1015`, and `?1016`;
- keypad application mode behavior for `?66`;
- renderer-visible behavior for cursor blinking/visibility, reverse colors, and
  synchronized output beyond stored mode state.

Those modes may still update `ModeState` so formatter output can preserve the
state, but the result must document that the runtime side effects remain
incomplete until the relevant subsystem experiment exists. A Pass for this
experiment means SM/RM parser and mode-state plumbing are correct; it does not
claim full runtime behavior for every affected mode. If this proves too
misleading during implementation or review, restrict runtime state mutation to
the currently representable modes and mark the experiment Partial with a
concrete follow-up.

Do not implement SGR, OSC, DCS, save/restore mode (`CSI ? ... s` / `r`), mode
request (`CSI ? ... $ p`), alternate-screen storage, mouse encoding, DECCOLM
resize, public ABI, or non-macOS behavior in this experiment.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/stream.zig` for `CSI h` / `CSI l` parsing.
   - Use `vendor/ghostty/src/terminal/stream_terminal.zig::setMode` for
     execution ordering and side-effect inventory.
   - Use `vendor/ghostty/src/terminal/modes.zig` and Roastty's existing
     `roastty/src/terminal/modes.rs` table for mode number mapping.
   - Use upstream tests around `stream: dec set mode (SM) and reset mode (RM)`,
     `stream: ansi set mode (SM) and reset mode (RM)`, and `modes` as the
     behavior checklist.
   - Do not modify `vendor/ghostty/`.

2. Expand CSI parameter capacity to match upstream.
   - Upstream `Parser.MAX_PARAMS` is `24`.
   - Change Roastty's `CsiState` parameter storage from the current two-slot
     array to a fixed `CSI_PARAM_CAPACITY` of `24`.
   - Preserve the current no-allocation parser shape.
   - Existing one-param and two-param commands must keep rejecting extra real
     parameters exactly as before.
   - Add boundary tests for exactly 24 parameters and for over-capacity input.
     The over-capacity case must not panic, must not leak the final byte as
     printable text, and must preserve whatever invalid/no-dispatch behavior the
     implementation explicitly chooses.
   - Existing unsupported/private/raw-C1/pending-invalid-UTF-8 tests must keep
     passing.
   - Colon-separated CSI params remain out of scope for this experiment. Roastty
     currently treats `:` as invalid globally, while upstream stores separator
     metadata. Do not broaden colon behavior here; that belongs in a later
     parser-separator parity experiment because it affects many commands.

3. Extend private stream actions.
   - Add `Action::SetMode { mode: modes::Mode }`.
   - Add `Action::ResetMode { mode: modes::Mode }`.
   - Import `super::modes` in `stream.rs`.
   - Keep these actions internal to the terminal module.
   - Do not add public API or ABI surface.

4. Extend CSI dispatch for final `h` and `l`.
   - Determine ANSI versus DEC mode from the private marker:
     - no private marker means ANSI;
     - `?` means DEC;
     - any other private/intermediate form is invalid.
   - For each parsed param, call `modes::mode_from_int(value, ansi)`.
   - Dispatch one action per known mode, in parameter order.
   - Unknown modes dispatch no action but do not invalidate or prevent later
     known modes in the same sequence.
   - Empty params use the parser's existing value `0`; if mode `0` is unknown,
     dispatch no action.
   - Multi-param forms such as `CSI 4 ; 20 h` and `CSI ? 1 ; 7 ; 2004 h` must
     dispatch multiple mode actions in order.
   - Preserve parser ground-state behavior on handler errors. If a later action
     fails, earlier actions in the same CSI sequence may already have reached
     the handler, matching ordered dispatch semantics.
   - Preserve pending invalid UTF-8 behavior: if an incomplete invalid UTF-8
     sequence is interrupted by a mode command, dispatch `U+FFFD` before the
     mode action.
   - Direct C1 CSI byte `0x9b` remains out of scope and follows the current
     UTF-8 replacement behavior.

5. Extend `CsiDispatch` to support ordered multi-action dispatch.
   - The existing `None`, `One`, and `Two` cases may remain.
   - Add a fixed-capacity ordered multi-action representation for mode commands,
     or another equivalently explicit representation that can invoke up to
     `CSI_PARAM_CAPACITY` actions without heap allocation.
   - `handle()` must invoke actions in order and stop on the first handler
     error.
   - Parser state must already be ground before `handle()` runs, as with the
     existing action dispatch paths.

6. Route terminal mode actions.
   - `TerminalStreamHandler` needs mutable access to `ModeState`.
   - Route `Action::SetMode` and `Action::ResetMode` through a helper such as
     `set_mode_basic(mode, enabled)`.
   - Always update `ModeState` first for dispatched modes, matching upstream.
   - Implement current-core side effects:
     - `Mode::Origin`: after changing the mode, move the cursor to `1,1` using
       origin-aware coordinates. With origin enabled this means the current
       scrolling-region top/left; with origin disabled this means screen
       top-left. Clear pending wrap as part of the cursor move.
     - `Mode::EnableLeftAndRightMargin` reset: clear `scrolling_region.left = 0`
       and `scrolling_region.right = size.cols.saturating_sub(1)`.
   - Do not fake alternate-screen switching, mouse flags, DECCOLM resize,
     save/restore cursor, renderer callbacks, or keypad behavior.

7. Add tests.
   - Stream parser tests:
     - `CSI 4 h` dispatches `SetMode(Insert)`;
     - `CSI 4 l` dispatches `ResetMode(Insert)`;
     - `CSI ? 6 h` dispatches `SetMode(Origin)`;
     - `CSI ? 6 l` dispatches `ResetMode(Origin)`;
     - multi-param ANSI commands dispatch known modes in order;
     - multi-param DEC commands dispatch known modes in order;
     - exactly 24 known mode params dispatch 24 ordered actions;
     - over-capacity mode params do not panic, do not leak final bytes, and
       follow the implementation's documented invalid/no-dispatch behavior;
     - unknown modes dispatch no action;
     - unknown modes mixed with known modes skip only the unknown entries;
     - invalid private/intermediate forms dispatch no action and do not leak the
       final byte as printable text;
     - colon-separated forms remain invalid under the current parser model;
     - split-feed mode commands dispatch correctly;
     - pending invalid UTF-8 emits `U+FFFD` before same-slice and split-feed
       mode commands;
     - direct C1 CSI byte `0x9b` followed by `h` or `l` remains out of scope and
       dispatches `U+FFFD` plus printable final byte;
     - handler errors from set/reset mode leave the parser in ground state;
     - multi-action dispatch stops after the first failing action.
   - Terminal tests:
     - `CSI 4 h` / `CSI 4 l` toggles `Mode::Insert`;
     - `CSI 20 h` / `CSI 20 l` toggles `Mode::Linefeed` state, while LF-as-CRLF
       runtime behavior remains explicitly deferred;
     - `CSI ? 7 h` / `CSI ? 7 l` toggles `Mode::Wraparound` state;
     - `CSI ? 2004 h` / `CSI ? 2004 l` toggles bracketed paste state and the
       terminal formatter mode extra emits the expected restore sequence;
     - multi-param mode commands toggle multiple modes in order;
     - unknown modes mixed with known modes skip only unknown modes;
     - setting and resetting origin mode moves the cursor to the correct
       origin-home position and clears pending wrap;
     - resetting left/right margin mode clears horizontal margins;
     - representative deferred modes such as `?1049`, `?1048`, `?3`, and a mouse
       mode toggle `ModeState` but do not fake screen switching, cursor restore,
       resize, or mouse encoding behavior;
     - unsupported/private/colon forms do not mutate terminal mode state;
     - mode commands do not modify cells or dirty rows unless a documented
       cursor movement side effect occurs.
   - Existing stream, cursor movement, positioning, tabstop, erase-display,
     erase-line, row mutation, scroll, formatter, PageList, and ABI tests must
     keep passing.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty stream
     cargo test -p roastty terminal::modes
     cargo test -p roastty terminal::terminal
     cargo test -p roastty terminal_formatter
     cargo test -p roastty screen_formatter
     cargo test -p roastty page_string
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

9. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - Commit the approved design before implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.
   - Commit the approved result separately from the design commit.

10. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - accepted and rejected SM/RM forms;
      - ANSI versus DEC mode behavior;
      - multi-param dispatch order and unknown-mode skipping behavior;
      - current treatment of colon params;
      - terminal behavior for insert, linefeed, wraparound, bracketed paste,
        origin, and left/right margin mode reset;
      - explicitly deferred side-effect modes;
      - confirmation that mode commands do not mutate cells or dirty rows;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `CSI h` / `CSI l` and `CSI ? h` / `CSI ? l` dispatch known ANSI/DEC modes
  correctly;
- multi-param mode commands dispatch known modes in order;
- unknown modes are skipped without preventing later known modes;
- invalid/private/intermediate/raw-C1/pending-invalid-UTF-8 behavior matches the
  current parser guarantees and the experiment's explicit scope;
- terminal mode state changes from real stream input;
- origin mode moves the cursor to the current origin-home position and clears
  pending wrap;
- resetting left/right margin mode clears horizontal margins;
- insert, linefeed, wraparound, alternate-screen, save/restore cursor, DECCOLM,
  mouse, and other deferred runtime effects are described as mode-state-only in
  this experiment unless specifically implemented;
- deferred side-effect modes are documented honestly and are not faked;
- existing formatter mode extras observe the updated state;
- existing raw print, linefeed, cursor, positioning, tabstop, erase-display,
  erase-line, row mutation, scroll, PageList, formatter, and ABI behavior
  remains unchanged;
- no unrelated SGR, OSC, DCS, save/restore mode, mode request, alternate-screen
  storage, mouse encoding, DECCOLM resize, public API, ABI, or non-macOS
  behavior is added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- stream dispatch works but terminal execution needs a narrower side-effect
  design before mode state can be mutated safely;
- expanding CSI parameter capacity exposes unrelated parser assumptions that
  need their own experiment;
- origin or left/right margin side effects require a broader cursor-positioning
  helper than expected.
- state-only handling for insert, linefeed, wraparound, or another runtime mode
  proves too misleading to land without the corresponding behavior.

The experiment fails if:

- it treats DEC private modes as ANSI modes or vice versa;
- it aborts a multi-param mode command just because one mode is unknown;
- it silently fakes alternate-screen, mouse, DECCOLM, save/restore cursor, or
  renderer behavior;
- it changes unrelated command parsing or execution semantics;
- it mutates cells or dirties rows for ordinary mode-state changes;
- it adds unrelated SGR, OSC, DCS, public API, ABI, or non-macOS behavior.

## Design Review

Codex reviewed the initial design and found four real issues:
`logs/codex-review/20260601-062551-558741-last-message.md`.

The design was updated to:

- explicitly treat insert, linefeed, and wraparound runtime behavior as deferred
  mode effects rather than “pure mode-state” behavior;
- remove the contradictory requirement that linefeed-mode runtime behavior be
  driven through real input in this experiment;
- require exact 24-param and over-capacity CSI parser boundary tests;
- require representative tests proving deferred modes such as alt-screen,
  save/restore cursor, DECCOLM, and mouse modes are not faked.

Codex re-reviewed the updated design and found no blocking findings:
`logs/codex-review/20260601-063004-451046-last-message.md`.

The design is approved for implementation.

## Result

**Result:** Pass

Roastty now accepts real SM/RM input for mode state:

- `CSI h` and `CSI l` dispatch ANSI mode set/reset actions;
- `CSI ? h` and `CSI ? l` dispatch DEC private mode set/reset actions;
- multi-param mode commands dispatch known modes in parameter order;
- unknown mode numbers are skipped without aborting later known mode params;
- empty mode params resolve to value `0`, which is unknown and dispatches no
  action;
- over-capacity CSI params are treated as invalid for dispatch, do not panic,
  and do not leak the final `h` / `l` byte as printable text;
- colon-separated CSI params remain invalid under the current parser model;
- unsupported private/intermediate forms do not dispatch;
- raw C1 `0x9b` remains out of scope and follows the existing UTF-8 replacement
  behavior.

The stream parser now uses an upstream-sized fixed CSI param capacity of 24, and
mode commands use a fixed-capacity ordered action list so up to 24 mode actions
can be delivered without heap allocation. Dispatch stops on the first handler
error, while the parser has already returned to ground state.

Terminal execution now routes `Action::SetMode` and `Action::ResetMode` through
real terminal input. Dispatched modes update `ModeState` first, matching
Ghostty's ordering. The current-core side effects implemented here are:

- `Mode::Origin` set/reset moves the cursor to origin-home, using the scrolling
  region top/left when origin mode is enabled and screen top-left when disabled;
- origin-home movement clears pending wrap through the normal cursor movement
  path;
- resetting `Mode::EnableLeftAndRightMargin` (`CSI ? 69 l`) clears horizontal
  margins to the full screen width while preserving the vertical scroll region.

The following modes intentionally remain state-only in this experiment:

- insert mode (`CSI 4 h/l`) does not yet change printable-character insertion
  behavior;
- linefeed mode (`CSI 20 h/l`) does not yet make LF behave like CRLF;
- wraparound mode (`CSI ? 7 h/l`) does not yet disable/enable runtime wrapping;
- alternate-screen modes (`?47`, `?1047`, `?1049`) do not switch screens;
- save/restore cursor (`?1048`) does not save or restore cursor state;
- DECCOLM (`?3`) does not resize the terminal;
- mouse modes and formats update mode state only and do not affect mouse
  encoding;
- keypad and renderer-visible modes update stored state only.

Tests verify that representative deferred modes toggle `ModeState` without
faking their side effects. Ordinary mode-state commands do not modify cells or
dirty rows. Formatter mode extras observe the updated mode state, including
bracketed paste.

Verification commands passed:

```bash
cargo fmt
cargo test -p roastty stream
cargo test -p roastty terminal::modes
cargo test -p roastty terminal::terminal
cargo test -p roastty terminal_formatter
cargo test -p roastty screen_formatter
cargo test -p roastty page_string
cargo test -p roastty
```

The final full `cargo test -p roastty` run passed: 1380 unit tests, 1 ABI
harness test, and 0 doc tests.

Codex design review passed after the design updates recorded above. Codex result
review passed with no blocking findings:
`logs/codex-review/20260601-063851-036562-last-message.md`.

## Conclusion

Experiment 128 successfully connects Roastty's existing mode table and
`ModeState` storage to real stream input. The parser now has the upstream CSI
parameter capacity needed for multi-param SM/RM commands, and terminal execution
implements the honest side effects currently supported by the core while
explicitly leaving heavier mode behavior for later subsystem experiments.
