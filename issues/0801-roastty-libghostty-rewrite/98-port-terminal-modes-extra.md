# Experiment 98: Port Terminal Modes and Formatter Extra

## Description

Port upstream Ghostty's terminal mode state model and use it to implement the
terminal formatter `modes` extra.

Experiment 97 established the terminal-level formatter pattern with palette
emission: terminal-owned private state, explicit `TerminalFormatterExtra` flags,
generated-byte pin maps anchored to the active screen top-left, and preserved
default formatter behavior. The next upstream terminal-level extra is modes.
Unlike palette, modes are not just formatter strings: upstream Ghostty has a
dedicated `terminal/modes.zig` model that owns the current, saved, and default
values for supported ANSI and DEC modes.

This experiment ports that mode model into Roastty and wires only the
formatter-facing part of it. It must not add VT parser support for CSI h/l,
DECRQM/DECRPM handling, public C ABI, input encoding behavior, render behavior,
mouse behavior, alternate-screen switching, or app behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/modes.zig` for:
     - supported mode entries;
     - ANSI vs DEC tagging;
     - defaults;
     - disabled-mode filtering;
     - `ModeState` current/saved/default behavior;
     - `Report` encoding.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `TerminalFormatter.Extra.modes`;
     - emitted CSI h/l sequence shape;
     - modes-before-screen-content ordering;
     - terminal-level pin-map behavior.
   - Do not modify `vendor/ghostty/`.

2. Add a private Roastty modes module.
   - Add `roastty/src/terminal/modes.rs` and include it from
     `roastty/src/terminal/mod.rs`.
   - Port the upstream supported mode list as Roastty data/types, preserving
     names, numeric values, ANSI/DEC tagging, disabled status, and default
     values.
   - Use Roastty naming and Rust idioms. Do not expose `ghostty` names.
   - Provide a private `Mode` enum and `ModeTag` value type.
   - Provide `mode_from_int(value, ansi)` that returns `None` for unknown or
     disabled modes.
   - Provide a private `ModeState` with:
     - current values;
     - saved values;
     - default values;
     - `set(mode, bool)`;
     - `get(mode)`;
     - `save(mode)`;
     - `restore(mode) -> bool`;
     - `reset()`.
     - `get_report(tag) -> Report`.
   - Provide a private `Report` type and `ReportState` enum equivalent to
     upstream `Report.State`, with VT encoding:

     ```text
     \x1b[{?}{value};{state}$y
     ```

     where DEC modes use `?` and ANSI modes do not.

3. Add private terminal mode state.
   - Add `modes: modes::ModeState` to `Terminal`.
   - Initialize it with upstream defaults.
   - Add `#[cfg(test)] pub(super)` helpers on `Terminal` to set/get/save/restore
     modes for formatter tests.
   - Keep this state private. Do not expose public API or ABI.

4. Extend `TerminalFormatterExtra`.
   - Add `modes: bool`.
   - Extend `none()`.
   - Add a `.modes(bool)` builder.
   - Keep `TerminalFormatter::init()` defaulting to no extras. This continues
     the intentional temporary divergence from upstream `.styles` defaults until
     all terminal extras are available.

5. Emit mode differences before screen content.
   - If `extra.modes` is true and output is VT, prepend CSI h/l sequences for
     every supported, non-disabled mode whose current value differs from its
     default.
   - Sequence shape:

     ```text
     \x1b[{prefix}{value}{suffix}
     ```

     where:
     - `prefix` is `?` for DEC modes and empty for ANSI modes;
     - `suffix` is `h` when the current value is true;
     - `suffix` is `l` when the current value is false.

   - Preserve upstream order by iterating the ported mode table in source order.
   - Emit modes after palette and before screen content when both extras are
     enabled.
   - Plain and HTML output ignore the modes extra.

6. Preserve pin-map semantics.
   - Mode bytes are generated terminal-state bytes, not content bytes.
   - Map all mode bytes to active screen top-left.
   - When both palette and modes are enabled, palette pin-map entries come
     first, then mode pin-map entries, then content entries.
   - `Content::None` can still emit mode bytes for VT.
   - Plain and HTML emit no mode bytes and therefore add no mode pin-map
     entries.

7. Add upstream-equivalent tests.
   - Add focused `modes` module tests for:
     - `mode_from_int` known ANSI mode, known DEC mode, and unknown mode;
     - disabled-mode filtering is preserved even though the current upstream
       table has no disabled entries. Do not invent a fake upstream mode for
       this. Either assert that no current ported entries are disabled, or test
       the filtering through a small internal helper if the implementation
       naturally exposes one;
     - default values for at least `send_receive_mode`, `wraparound`,
       `cursor_visible`, `mouse_alternate_scroll`, `ignore_keypad_with_numlock`,
       and `alt_esc_prefix`;
     - `set`, `get`, `save`, `restore`, and `reset`;
     - `ModeState::get_report()` for known DEC, known ANSI, unknown, current
       set, and current reset states;
     - `Report` encoding for DEC set/reset, ANSI set, and not-recognized.
   - Add TerminalFormatter tests for:
     - default formatting does not emit mode bytes;
     - VT modes output emits only modes that differ from defaults;
     - DEC true emits `CSI ? value h`;
     - DEC false from true default emits `CSI ? value l`;
     - ANSI true emits `CSI value h`;
     - ANSI false from true default emits `CSI value l`, using
       `send_receive_mode`;
     - a default-true ANSI mode emits nothing while it still matches its
       default;
     - output order follows the upstream mode table;
     - modes emit before content;
     - modes combine after palette and before content;
     - modes combine before forwarded screen extras, with ordering
       `palette -> modes -> content -> screen extras` when palette is also
       enabled;
     - plain and HTML ignore modes;
     - `Content::None` can emit only mode bytes for VT;
     - mode pin maps are byte-indexed and map generated bytes to top-left;
     - palette and modes together in `format_with_pin_map()` produce palette
       bytes mapped to top-left, then mode bytes mapped to top-left, then
       content bytes mapped to the selected content pins;
     - pin-map tests use content selected from row 1 so generated mode bytes
       cannot accidentally map to the first content pin.
   - Keep existing TerminalFormatter, ScreenFormatter, PageList formatter, and
     modes-adjacent tests passing.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty modes
     cargo test -p roastty terminal_formatter
     cargo test -p roastty screen_formatter
     cargo test -p roastty styled_pin_map
     cargo test -p roastty pin_map
     cargo test -p roastty page_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

9. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.

10. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - names and visibility of the ported mode types;
      - how upstream mode entries/defaults/disabled modes were represented;
      - terminal mode state ownership;
      - exact formatter CSI sequence shape;
      - plain/HTML no-op behavior;
      - ordering relative to palette, content, and forwarded screen extras;
      - pin-map behavior for mode bytes;
      - why VT parser/runtime mode mutation, input behavior, render behavior,
        public API, and ABI remain deferred;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private modes module with upstream-equivalent mode entries,
  tags, defaults, disabled filtering, state operations, and report encoding;
- `Terminal` owns private mode state initialized with upstream defaults;
- `TerminalFormatterExtra` has an opt-in modes flag;
- default TerminalFormatter output and pin maps remain unchanged;
- VT mode output emits only current-vs-default differences before screen
  content;
- DEC and ANSI mode sequence shapes match upstream;
- mode output preserves upstream mode-table order;
- plain and HTML output ignore the modes extra;
- generated mode bytes are byte-indexed in pin maps and map to active-screen
  top-left;
- palette and modes can combine with ordering `palette -> modes -> content`;
- palette, modes, content, and forwarded screen extras can combine with ordering
  `palette -> modes -> content -> screen extras`;
- no VT parser/runtime mode mutation, DECRQM/DECRPM runtime handling, public
  API, public ABI, input encoding behavior, render behavior, mouse behavior,
  alternate-screen switching, app behavior, renderer behavior, PTY behavior,
  clipboard behavior, or UI behavior is added;
- `cargo fmt`, focused modes tests, targeted formatter tests, PageList formatter
  tests, PageList tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- the upstream mode table cannot be represented cleanly without a broader
  parser/runtime mode port, but the exact missing prerequisite is identified.

The experiment fails if:

- default TerminalFormatter output changes;
- mode bytes emit without explicit `TerminalFormatter::with_extra()`;
- HTML or plain output emits mode bytes;
- mode bytes emit after content;
- unsupported, disabled, or default-valued modes are emitted;
- generated mode pin maps become character-indexed, shorter than output bytes,
  or map to content pins instead of top-left;
- runtime parser, public API, ABI, input, render, mouse, alternate-screen, or UI
  behavior is added.

## Design Review

Codex reviewed this design before implementation.

Review artifacts:

- Prompt: `logs/codex-review/20260531-235914-209063-prompt.md`
- Result: `logs/codex-review/20260531-235914-209063-last-message.md`

Codex approved the overall scope, with five required design fixes:

- do not invent a disabled upstream mode for tests, because the current upstream
  table has the disabled flag/filter but no disabled entries;
- port `ModeState::get_report()` instead of only porting `Report` encoding;
- add ANSI true-default reset coverage using `send_receive_mode`;
- add an explicit palette+modes `format_with_pin_map()` ordering test;
- add a modes plus forwarded screen-extras ordering test.

All five findings were applied before implementation.
