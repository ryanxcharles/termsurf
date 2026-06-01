# Experiment 164: Port OSC Parser C ABI

## Description

Experiments 158 and 163 exposed the pure mouse and key encoder layers through
the renamed Roastty C ABI. The next upstream `lib_vt` block after key/mouse is
the OSC parser C wrapper:

- `ghostty_osc_new`;
- `ghostty_osc_free`;
- `ghostty_osc_next`;
- `ghostty_osc_reset`;
- `ghostty_osc_end`;
- `ghostty_osc_command_type`;
- `ghostty_osc_command_data`.

Roastty already has the OSC parser and command model in
`roastty/src/terminal/osc.rs`, plus extensive parser and stream tests from
earlier experiments. This experiment should expose that existing parser through
`roastty_osc_*` names without adding terminal runtime dispatch, clipboard
writes, notifications, color mutation, hyperlink mutation, or any live
app/surface behavior.

The upstream source material is:

- `vendor/ghostty/src/terminal/c/osc.zig`;
- `vendor/ghostty/src/terminal/c/main.zig`;
- `vendor/ghostty/src/terminal/osc.zig`;
- existing Roastty parser code in `roastty/src/terminal/osc.rs`.

## Changes

1. Update the public header.
   - Add opaque handles:
     - `roastty_osc_parser_t`;
     - `roastty_osc_command_t`.
   - Add `roastty_osc_command_e` with upstream command-key values, renamed:
     - `ROASTTY_OSC_COMMAND_INVALID = 0`;
     - `ROASTTY_OSC_COMMAND_CHANGE_WINDOW_TITLE = 1`;
     - `ROASTTY_OSC_COMMAND_CHANGE_WINDOW_ICON = 2`;
     - `ROASTTY_OSC_COMMAND_SEMANTIC_PROMPT = 3`;
     - `ROASTTY_OSC_COMMAND_CLIPBOARD_CONTENTS = 4`;
     - `ROASTTY_OSC_COMMAND_REPORT_PWD = 5`;
     - `ROASTTY_OSC_COMMAND_MOUSE_SHAPE = 6`;
     - `ROASTTY_OSC_COMMAND_COLOR_OPERATION = 7`;
     - `ROASTTY_OSC_COMMAND_KITTY_COLOR_PROTOCOL = 8`;
     - `ROASTTY_OSC_COMMAND_SHOW_DESKTOP_NOTIFICATION = 9`;
     - `ROASTTY_OSC_COMMAND_HYPERLINK_START = 10`;
     - `ROASTTY_OSC_COMMAND_HYPERLINK_END = 11`;
     - `ROASTTY_OSC_COMMAND_CONEMU_SLEEP = 12`;
     - `ROASTTY_OSC_COMMAND_CONEMU_SHOW_MESSAGE_BOX = 13`;
     - `ROASTTY_OSC_COMMAND_CONEMU_CHANGE_TAB_TITLE = 14`;
     - `ROASTTY_OSC_COMMAND_CONEMU_PROGRESS_REPORT = 15`;
     - `ROASTTY_OSC_COMMAND_CONEMU_WAIT_INPUT = 16`;
     - `ROASTTY_OSC_COMMAND_CONEMU_GUIMACRO = 17`;
     - `ROASTTY_OSC_COMMAND_CONEMU_RUN_PROCESS = 18`;
     - `ROASTTY_OSC_COMMAND_CONEMU_OUTPUT_ENVIRONMENT_VARIABLE = 19`;
     - `ROASTTY_OSC_COMMAND_CONEMU_XTERM_EMULATION = 20`;
     - `ROASTTY_OSC_COMMAND_CONEMU_COMMENT = 21`;
     - `ROASTTY_OSC_COMMAND_KITTY_TEXT_SIZING = 22`;
     - `ROASTTY_OSC_COMMAND_KITTY_CLIPBOARD_PROTOCOL = 23`;
     - `ROASTTY_OSC_COMMAND_CONTEXT_SIGNAL = 24`.
   - The enum preserves upstream numeric parity, but not every value is
     currently produced by Roastty's parser. Values for commands Roastty does
     not currently parse, such as `CHANGE_WINDOW_ICON` and the ConEmu command
     family, are reserved for future parity work and must be documented as
     reserved/not currently returned.
   - Add `roastty_osc_command_data_e`:
     - `ROASTTY_OSC_COMMAND_DATA_INVALID = 0`;
     - `ROASTTY_OSC_COMMAND_DATA_CHANGE_WINDOW_TITLE_STR = 1`.
   - Add C functions:
     - `roastty_osc_new(roastty_osc_parser_t* out)`;
     - `roastty_osc_free(roastty_osc_parser_t parser)`;
     - `roastty_osc_reset(roastty_osc_parser_t parser)`;
     - `roastty_osc_next(roastty_osc_parser_t parser, uint8_t byte)`;
     - `roastty_osc_end(roastty_osc_parser_t parser, int terminator)`;
     - `roastty_osc_command_type(roastty_osc_command_t command)`;
     - `roastty_osc_command_data(roastty_osc_command_t command, int data, void* out)`.
   - Define exact command-data output type:
     - `ROASTTY_OSC_COMMAND_DATA_CHANGE_WINDOW_TITLE_STR` expects `out` to be
       `const char**`;
     - on success, the function writes a pointer to parser-owned NUL-terminated
       title memory into `*out`;
     - the returned pointer is valid until the owning parser receives another
       byte, is reset, ends another command, or is freed;
     - `out == NULL` returns `false`;
     - wrong command type or wrong data selector returns `false` and does not
       write output.
   - Use Roastty names only. Do not add `ghostty_*` compatibility aliases.

2. Add Rust ABI wrappers in `roastty/src/lib.rs`.
   - Import `terminal::osc`.
   - Add `OscParser` wrapper:
     - stores `osc::Parser`;
     - stores `last_command: Option<OwnedOscCommand>`.
   - Add an owned command representation rather than returning borrowed
     references into the parser buffer. The public `roastty_osc_command_t`
     points at `last_command` inside the parser wrapper.
   - Command handle lifetime:
     - valid only until the owning parser is reset, receives another byte, ends
       another OSC, or is freed;
     - `roastty_osc_next`, `roastty_osc_reset`, and `roastty_osc_end` clear or
       replace the previous command;
     - `roastty_osc_end` copies the parsed command into owned storage and then
       resets the parser input buffer so callers can feed the next OSC command
       immediately without a separate `reset` call;
     - there is no separate `roastty_osc_command_free`, matching upstream's C
       shape.
   - Convert all currently recognized `osc::Command` variants to owned command
     tags, even if `command_data` only exposes title data today. Preserve
     numeric slots for upstream commands Roastty does not currently parse, but
     do not claim those reserved values are returned yet.
   - For `WindowTitle`, store a NUL-terminated owned byte buffer for C string
     access. Reject or downgrade title commands containing interior NUL bytes so
     `ROASTTY_OSC_COMMAND_DATA_CHANGE_WINDOW_TITLE_STR` never exposes an
     ambiguous C string.

3. Validate raw ABI inputs.
   - `roastty_osc_new(NULL)` returns `ROASTTY_INVALID_VALUE`.
   - `roastty_osc_free(NULL)` is tolerated.
   - `roastty_osc_reset(NULL)` and `roastty_osc_next(NULL, ...)` are tolerated
     no-ops, matching the forgiving style used by existing skeleton functions.
   - `roastty_osc_end(NULL, ...)` returns `NULL`.
   - `roastty_osc_command_type(NULL)` returns `ROASTTY_OSC_COMMAND_INVALID`.
   - `roastty_osc_command_data(NULL, ..., ...)` returns `false`.
   - `roastty_osc_command_data(..., invalid_data, ...)` returns `false`.
   - `roastty_osc_command_data(..., valid_data, NULL)` returns `false`.
   - The `terminator` argument is a raw integer. Accept:
     - `0` as the upstream C-compatible default terminator, treated as ST;
     - `0x07` for BEL;
     - `'\\'` for ST after an ESC sequence. Upstream effectively treats non-BEL
       terminator bytes as ST. Roastty intentionally narrows the public ABI to
       these known values so invalid terminators fail predictably. Invalid
       terminator values return `NULL` rather than panicking.
   - Stale command handles after parser mutation/free are invalid by contract.
     The ABI does not attempt generation checks for old `roastty_osc_command_t`
     values. Tests should verify replacement/lifetime behavior without
     dereferencing stale handles after mutation.

4. Keep scope boundaries hard.
   - Do not dispatch OSC actions into a terminal.
   - Do not mutate app/surface/runtime state.
   - Do not write clipboard contents, issue notifications, change colors, change
     hyperlinks, change mouse shape, or update current working directory.
   - Do not add formatter, render-state, terminal, selection, Kitty graphics, or
     PTY APIs.
   - Do not expose `ghostty_*` names or compatibility aliases.
   - Do not add non-macOS platform behavior.

5. Add tests.
   - Add Rust unit tests in `roastty/src/lib.rs` for:
     - allocation/free/null behavior;
     - reset and next no-op behavior on null parser;
     - parsing `0;title` as `ROASTTY_OSC_COMMAND_CHANGE_WINDOW_TITLE`;
     - title command-data extraction as a NUL-terminated C string;
     - invalid data type and wrong data type returning `false`;
     - `0`, `0x07`, and `'\\'` terminators, including a terminator-sensitive
       owned command conversion such as OSC color query or Kitty clipboard/color
       protocol so BEL/ST is proven to survive conversion;
     - invalid terminator returning `NULL`;
     - `roastty_osc_end` resetting the parser input buffer by parsing two
       commands sequentially without an explicit reset;
     - command lifetime replacement after `next`, `reset`, and `end` without
       dereferencing stale command handles;
     - representative command-type mappings for report-pwd, hyperlink start/end,
       desktop notification, mouse shape, color operation, Kitty text sizing,
       Kitty clipboard protocol, and context signal.
     - reserved upstream enum values that Roastty does not currently produce are
       present in the public enum but not returned for unsupported OSC forms.
   - Extend `roastty/tests/abi_harness.c` to compile and exercise the new C
     declarations:
     - allocate parser;
     - feed `0;from-c`;
     - end the command;
     - assert command type;
     - extract title string through `const char* title = NULL` and `&title`;
     - parse two title commands sequentially without an explicit reset after the
       first `end`;
     - reset/free parser;
     - verify null and invalid-value behavior.
   - Existing key and mouse ABI harness coverage must keep passing.

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
cargo fmt -- roastty/src/lib.rs
cargo test -p roastty osc
cargo test -p roastty osc_parser_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
! rg -n "ghostty|Ghostty|ghostty_" roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c
```

Required evidence:

- `roastty/include/roastty.h` declares the OSC parser ABI with Roastty names
  only.
- Rust exports match the header declarations.
- Command type values match upstream command-key ordering.
- Reserved command type values that Roastty cannot currently produce are
  documented as reserved/not currently returned.
- The C ABI can allocate, feed, end, inspect, reset, and free an OSC parser.
- Command-data extraction safely exposes a NUL-terminated owned title string.
- Command-data output type is exact and tested as `const char**`.
- Current command handles never borrow invalid parser buffer memory. Stale
  command handles after parser mutation/free are documented as invalid by
  contract and are not dereferenced by tests.
- BEL/ST/default terminator behavior is tested with terminator-sensitive command
  conversion.
- Invalid handles, terminators, data selectors, and output pointers fail
  predictably without panicking or dereferencing null.
- Existing OSC parser/stream tests, key ABI tests, mouse ABI tests, terminal
  tests, and the C ABI harness still pass.
- No live terminal dispatch, runtime/app/surface mutation, clipboard writes,
  notification sends, color mutation, hyperlink mutation, renderer, PTY, or
  non-macOS platform behavior is added.
- Codex design review and result review both pass before moving to the next
  stage.

## Non-Negotiable Invariants

- Use Roastty names in public ABI, implementation-facing comments, tests, and
  modules.
- Do not add public `ghostty_*` compatibility names.
- Keep this as an OSC parser C ABI exposure only.
- Do not dispatch parsed OSC commands into terminal/runtime/app behavior.
- Do not add formatter, render-state, terminal, selection, Kitty graphics, PTY,
  or platform APIs.
- Do not add non-macOS platform behavior.
- Store C-visible command data in owned wrapper memory; do not expose borrowed
  parser buffer references with invalid lifetimes.
- Run `cargo fmt` and accept its output.
- Pass Codex design and result reviews before moving to the next stage.

## Failure Criteria

This experiment fails if:

- any public `ghostty_*` or compatibility OSC ABI names are introduced;
- command handles point at borrowed parser buffer memory that can be invalidated
  while still exposed to C;
- parsed OSC commands mutate terminal/runtime/app/surface state;
- invalid handles, terminators, data selectors, or output pointers can panic or
  dereference null;
- command type numeric values drift from upstream order;
- reserved upstream command enum values are claimed as currently returned even
  though the Roastty parser does not produce them;
- command-data title output is not NUL-terminated or has ambiguous interior-NUL
  behavior;
- `roastty_osc_command_data` does not define and test the exact pointed-to
  output type for each data selector;
- formatter, render-state, terminal, selection, Kitty graphics, PTY, browser, or
  non-macOS platform behavior is added;
- existing OSC, key, mouse, terminal, formatter, or ABI tests regress;
- the design or result proceeds without the required Codex review gate.

## Codex Design Review

**Result:** Approved after revision.

Codex's first review found real ABI design gaps: `command_data` needed an exact
`const char**` output convention, terminator behavior needed stronger
terminator-sensitive verification, reserved upstream command enum values needed
to be documented as not currently returned, `roastty_osc_end` needed explicit
parser-state behavior, and stale command handles needed to be documented as
invalid by contract rather than promised as safely detectable.

The design was updated to address those findings. Codex's second review found no
remaining blocking design issues and approved the experiment for implementation.
