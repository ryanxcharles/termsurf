# Experiment 96: Port Terminal Formatter Screen Extra Forwarding

## Description

Add the private `TerminalFormatter` extra plumbing needed to forward screen
extras to the active-screen `ScreenFormatter`.

Experiments 91-95 completed the current `ScreenFormatter` screen-extra subset:
cursor, style, hyperlink, protection, Kitty keyboard, and charsets. Upstream
Ghostty's `TerminalFormatter.Extra` contains a nested
`screen: ScreenFormatter.Extra` field, and `TerminalFormatter.format()` passes
that field to the active-screen formatter.

Roastty currently has no `TerminalFormatterExtra` at all. Its
`TerminalFormatter` always delegates content only, so the completed screen
extras are reachable only through direct `ScreenFormatter` tests. This
experiment adds the forwarding bridge without adding terminal-level extras such
as palette, modes, scrolling region, tabstops, pwd, or keyboard.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `TerminalFormatter.Extra`;
     - `TerminalFormatter.Extra.none`;
     - the `screen: ScreenFormatter.Extra` field;
     - the `screen_formatter.extra = self.extra.screen` forwarding line.
   - Do not modify `vendor/ghostty/`.

2. Add private terminal formatter extras.
   - In `roastty/src/terminal/terminal.rs`, add:

     ```rust
     pub(super) struct TerminalFormatterExtra {
         screen: ScreenFormatterExtra,
     }
     ```

   - Use `pub(super)` visibility, matching `TerminalFormatter` and
     `ScreenFormatterExtra`, so `TerminalFormatter::with_extra(...)` can be used
     at the terminal-module boundary without exposing public API or ABI.
   - Add `TerminalFormatterExtra::none()` returning no screen extras.
   - Add a `screen(ScreenFormatterExtra)` builder.
   - Add an `extra: TerminalFormatterExtra` field to `TerminalFormatter`.
   - Initialize `TerminalFormatter` with `TerminalFormatterExtra::none()` so
     existing default behavior remains unchanged.
   - Add `TerminalFormatter::with_extra(...)`.
   - Keep the type private to the terminal module. Do not expose public API or
     ABI.

3. Forward screen extras to `ScreenFormatter`.
   - In `TerminalFormatter::format()`, pass `self.extra.screen` to the
     `ScreenFormatter` created for the active screen.
   - In `TerminalFormatter::format_with_pin_map()`, pass the same screen extra.
   - Preserve current content, trim, unwrap, palette, codepoint-map, and
     selection delegation behavior.

4. Preserve scope.
   - Do not implement terminal-level extras yet:
     - palette;
     - modes;
     - scrolling region;
     - tabstops;
     - pwd;
     - keyboard.
   - Do not change the default terminal formatter output.
   - Do not add parser/runtime behavior, public API, public ABI, app behavior,
     renderer behavior, PTY behavior, clipboard behavior, or UI behavior.

5. Add upstream-equivalent tests.
   - Add TerminalFormatter tests proving default output and pin maps still do
     not emit screen extras when `with_extra()` is not used.
   - Add TerminalFormatter tests proving explicit forwarded screen extras emit
     the same output as direct `ScreenFormatter` for:
     - VT content with style, hyperlink, protection, Kitty keyboard, charsets,
       and cursor extras;
     - `Content::None` with the same extras;
     - byte-indexed pin maps with content;
     - byte-indexed pin maps with `Content::None`.
   - Include a multibyte hyperlink URI/id in at least one forwarded pin-map test
     to preserve the Experiment 95 byte-indexing guard through the terminal
     formatter.
   - Add plain and HTML tests proving forwarded screen extras are ignored for
     non-VT output, matching `ScreenFormatter`.
   - Keep existing TerminalFormatter and ScreenFormatter tests passing.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal_formatter
     cargo test -p roastty screen_formatter
     cargo test -p roastty styled_pin_map
     cargo test -p roastty pin_map
     cargo test -p roastty page_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

7. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - terminal extra type names and visibility;
     - default behavior;
     - forwarding behavior for `format()` and `format_with_pin_map()`;
     - how VT, plain, and HTML behave;
     - how pin-map entries for forwarded screen extras are assigned;
     - why terminal-level extras remain deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `TerminalFormatter` has private extra plumbing with a nested
  `ScreenFormatterExtra`;
- default TerminalFormatter output and pin maps remain unchanged;
- explicitly forwarded screen extras reach `ScreenFormatter::format()`;
- explicitly forwarded screen extras reach
  `ScreenFormatter::format_with_pin_map()`;
- VT output matches direct active-screen formatting for the implemented screen
  extra subset;
- plain and HTML output ignore forwarded screen extras;
- forwarded extra pin maps remain byte-indexed and match direct
  `ScreenFormatter` behavior;
- terminal-level palette, modes, scrolling region, tabstops, pwd, and keyboard
  extras are not implemented;
- no parser/runtime behavior, public API, public ABI, app behavior, renderer
  behavior, PTY behavior, clipboard behavior, or UI behavior is added;
- `cargo fmt`, targeted formatter tests, PageList formatter tests, PageList
  tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- forwarding screen extras through `TerminalFormatter` requires adding a broader
  terminal-extra model first.

The experiment fails if:

- default TerminalFormatter output changes;
- screen extras are emitted without explicit `TerminalFormatter::with_extra()`;
- forwarded extras emit for plain or HTML in a way that differs from
  `ScreenFormatter`;
- forwarded pin maps become character-indexed or shorter than output bytes;
- terminal-level extras are implemented in this experiment;
- the implementation adds unrelated parser, terminal runtime, app, renderer,
  PTY, public API, or ABI behavior.

## Design Review

Codex reviewed this design before implementation and agreed with the experiment
scope: explicit opt-in forwarding only, default `TerminalFormatter` behavior
preserved, no terminal-level palette/modes/scrolling-region/tabstop/pwd/keyboard
extras, and verification covering VT forwarding, non-VT ignoring,
`Content::None`, pin maps, and multibyte hyperlink byte indexing.

Codex found one required design fix, applied above:

- `TerminalFormatterExtra` must be `pub(super)`, matching `TerminalFormatter`
  and `ScreenFormatterExtra`, so `TerminalFormatter::with_extra(...)` does not
  expose a more-private type and remains usable at the terminal-module boundary.

With that update, the design is approved for implementation.
