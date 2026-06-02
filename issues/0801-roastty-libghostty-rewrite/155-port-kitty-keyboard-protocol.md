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

# Experiment 155: Port Kitty Keyboard Protocol

## Description

Port Ghostty's Kitty keyboard protocol runtime commands for the active screen.

Roastty already has the lower-level state type in
`roastty/src/terminal/kitty.rs`:

- `KeyFlags`
- `KeySetMode`
- `KeyFlagStack`

Roastty also already has formatter support for emitting active Kitty keyboard
state through `ScreenFormatterExtra::kitty_keyboard(true)`. What is missing is
the parser/runtime path that lets terminal input mutate and query that stack.

Upstream Ghostty source references:

- `vendor/ghostty/src/terminal/stream.zig`
  - parses `CSI ? u` as `kitty_keyboard_query`;
  - parses `CSI > Ps u` as `kitty_keyboard_push`;
  - parses `CSI < Ps u` as `kitty_keyboard_pop`;
  - parses `CSI = Ps ; Pm u` as set/or/not Kitty keyboard flags.
- `vendor/ghostty/src/terminal/stream_terminal.zig`
  - applies push/pop/set/or/not to `terminal.screens.active.kitty_keyboard`;
  - responds to query with `CSI ? {flags} u`.
- `vendor/ghostty/src/terminal/kitty.zig`
  - defines the five-bit flag mapping.
- `vendor/ghostty/src/terminal/formatter.zig`
  - emits Kitty keyboard state via `CSI = {flags} u`.

This experiment should make the Kitty keyboard protocol functional at the
terminal stream layer, scoped to state/query behavior only. It should not add
key event encoding, platform input handling, public ABI, app integration, or
mouse/keyboard frontend behavior.

## Changes

1. Make Kitty keyboard state runtime-accessible.
   - Remove test-only gates from `KeyFlagStack::set`, `push`, and `pop` so the
     terminal runtime can use the existing implementation.
   - Keep `KeyFlags::from_int` private unless the stream parser needs a narrow
     helper such as `KeyFlags::from_protocol_int(u16) -> Option<KeyFlags>` that
     rejects values outside the five-bit Kitty flag range.

2. Extend stream actions.
   - Add actions for:
     - Kitty keyboard query;
     - Kitty keyboard push with flags;
     - Kitty keyboard pop with count;
     - Kitty keyboard set;
     - Kitty keyboard set-or;
     - Kitty keyboard set-not.
   - Keep these actions internal to `roastty/src/terminal/stream.rs`; do not add
     public ABI or app-visible API.

3. Parse Kitty keyboard CSI `u` forms.
   - Preserve existing `CSI u` restore-cursor behavior with no intermediates.
   - Parse `CSI ? u` as query.
   - Parse `CSI > Ps u` as push. Missing parameter defaults to `0`; one
     parameter must fit in the five-bit flag range; if the parameter count is
     not exactly one, match upstream Ghostty and default flags to `0`.
   - Parse `CSI < Ps u` as pop. Missing parameter defaults to `1`; if the
     parameter count is not exactly one, match upstream Ghostty and default the
     pop count to `1`.
   - Parse `CSI = Ps ; Pm u` as set/or/not. Missing `Ps` defaults to `0`;
     missing `Pm` defaults to `1`; `Pm=1` means set, `Pm=2` means OR, and `Pm=3`
     means NOT. Invalid flags or mode values are ignored. Extra parameters after
     `Pm` are ignored, matching upstream Ghostty.
   - Do not reinterpret unrelated CSI `u` forms as Kitty keyboard commands.

4. Apply runtime behavior on the active screen.
   - Query writes `\x1b[?{flags}u` to the PTY response buffer, where `{flags}`
     is the active screen's current `KeyFlags::int()`.
   - Push, pop, set, set-or, and set-not mutate only the active screen's
     `kitty_keyboard` stack.
   - Primary and alternate screen Kitty keyboard states remain isolated because
     the state belongs to `Screen`.
   - RIS remains a full reset and clears Kitty keyboard state through
     `Screen::reset()`.

5. Preserve existing behavior.
   - `CSI u` with no intermediates must still restore cursor.
   - Existing formatter Kitty keyboard extra behavior must remain unchanged.
   - Existing parser invalid-form behavior must remain non-mutating.

## Verification

Run:

```bash
cargo fmt
cargo test -p roastty kitty_keyboard
cargo test -p roastty save_cursor
cargo test -p roastty ris
cargo test -p roastty
```

Required test coverage:

- Stream parser tests:
  - `CSI ? u` dispatches query.
  - `CSI > u` defaults push flags to `0`.
  - `CSI > 3 u` dispatches push with flags `3`.
  - `CSI < u` defaults pop count to `1`.
  - `CSI < 2 u` dispatches pop count `2`.
  - `CSI = u` defaults to set flags `0`.
  - `CSI = 3 u`, `CSI = 3 ; 1 u`, `CSI = 3 ; 2 u`, and `CSI = 3 ; 3 u` dispatch
    set/set-or/set-not correctly.
  - Invalid flag values above the five-bit range, invalid set modes, and colon
    forms are ignored without dispatching an action.
  - Extra semicolon parameters follow upstream Ghostty's lenient behavior: query
    ignores parameters; push/pop default when parameter count is not exactly
    one; set uses the first two parameters and ignores later parameters.
  - `CSI u` remains restore-cursor.

- Runtime tests:
  - Query on default state writes `\x1b[?0u`.
  - Push changes the active flags and query reports the pushed value.
  - Pop restores the previous stack value; oversized pop resets to disabled.
  - Multiple pushes followed by `CSI < 2 u` pop two stack entries and restore
    the expected earlier value.
  - Set replaces, set-or ORs, and set-not clears bits from the current flags.
  - Primary and alternate screens maintain independent Kitty keyboard stacks.
  - RIS clears Kitty keyboard state on the active screen and on future alternate
    entries.

- Regression tests:
  - Existing formatter Kitty keyboard extra tests still pass.
  - Existing save/restore cursor tests still pass.
  - Existing RIS tests still pass.
  - No public ABI, app integration, PTY process, renderer, browser overlay,
    mouse input, or key event encoding behavior changes.

## Non-Negotiable Invariants

- Do not add key event encoding in this experiment.
- Do not add platform keyboard translation or macOS input integration.
- Do not add public ABI, app API, renderer behavior, PTY process behavior, or
  browser overlay behavior.
- Do not add mouse protocol behavior.
- Do not add Kitty graphics behavior.
- Do not add Linux or other non-macOS platform paths.
- Do not add `ghostty_*` names. Use Roastty names except when citing upstream
  Ghostty source paths or behavior.
- Run `cargo fmt` and accept its output.

## Failure Criteria

This experiment fails if:

- Kitty keyboard CSI forms parse but do not mutate/query the active screen;
- `CSI u` restore-cursor behavior regresses;
- invalid Kitty keyboard forms mutate state or write query responses;
- query responses use the wrong VT format;
- push/pop/set/or/not behavior diverges from `KeyFlagStack` semantics;
- primary and alternate screen Kitty keyboard state leaks across screens;
- RIS leaves stale Kitty keyboard state;
- the patch adds key event encoding, platform input handling, public ABI,
  renderer/app behavior, PTY process behavior, browser overlay behavior, mouse
  protocol behavior, Kitty graphics, or non-macOS platform paths.

## Result

**Result:** Pass

Implemented Kitty keyboard CSI `u` parsing and runtime state/query behavior for
the active screen.

Code changes:

- `roastty/src/terminal/kitty.rs`
  - added `KeyFlags::from_protocol_int(u16)` for five-bit protocol validation;
  - made `KeyFlagStack::set`, `push`, and `pop` available to runtime code.
- `roastty/src/terminal/screen.rs`
  - added screen-level helpers for querying, setting, pushing, and popping Kitty
    keyboard state;
  - kept the existing test helpers as wrappers around those runtime helpers.
- `roastty/src/terminal/stream.rs`
  - added internal actions for Kitty keyboard query, push, pop, and set/or/not;
  - parsed `CSI ? u`, `CSI > Ps u`, `CSI < Ps u`, and `CSI = Ps ; Pm u`;
  - preserved plain `CSI u` as restore-cursor;
  - rejected invalid flag values, invalid set modes, and colon forms without
    dispatching actions;
  - matched upstream Ghostty's lenient handling for extra semicolon parameters.
- `roastty/src/terminal/terminal.rs`
  - applied query/push/pop/set/or/not to `TerminalScreens::active()`;
  - wrote query responses as `\x1b[?{flags}u`;
  - left key event encoding, platform input, ABI, app integration, renderer,
    browser overlay, PTY process behavior, mouse behavior, and Kitty graphics
    untouched.

Verification:

```bash
cargo fmt
cargo test -p roastty kitty_keyboard
cargo test -p roastty save_cursor
cargo test -p roastty ris
cargo test -p roastty
```

All commands passed. The final full suite reported 1707 unit tests passing, the
ABI harness passing, and 0 doc tests.

During implementation, the targeted Kitty keyboard test filter caught one real
mistake: the first parser version rejected the required semicolon-separated
`CSI = Ps ; Pm u` forms, so set-or/set-not did not dispatch. That was fixed
before the required verification passed.

The mandatory Codex result review then found a second real parity issue: the
first completed parser was stricter than upstream Ghostty for extra semicolon
parameters. Vendored Ghostty dispatches `CSI ? u` without inspecting params,
defaults push/pop when the parameter count is not exactly one, and lets `=`
forms use the first two params while ignoring later params. The implementation
and tests were updated to match that behavior before this result was committed.

## Conclusion

Roastty now supports Ghostty's Kitty keyboard protocol state commands at the
terminal stream layer. Applications can query and mutate the active screen's
Kitty keyboard flag stack, primary and alternate screen state stays isolated,
RIS clears the state, invalid forms are inert, and the existing formatter extra
continues to emit the current state.

This experiment deliberately stops before frontend key event encoding. A later
experiment should port the actual key encoding/input side once the macOS input
translation layer is in scope.
