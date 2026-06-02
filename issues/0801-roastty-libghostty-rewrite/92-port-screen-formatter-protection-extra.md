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

# Experiment 92: Port Screen Formatter Protection Extra

## Description

Port the next small `ScreenFormatter.Extra` slice from upstream Ghostty:
character protection state emission.

Experiment 91 added private screen cursor position/style state and VT-only
cursor/style extras. Upstream emits protection as another screen-level VT extra:
if `screen.cursor.protected` is true and the protection extra is requested,
`ScreenFormatter` appends DECSCA (`CSI 1 " q`) after style/hyperlink extras and
before keyboard/charset/cursor extras.

Roastty still does not have parser-driven protection handling, protected cell
write semantics, or erase semantics. This experiment should therefore add only
the minimal private cursor protection state needed by the formatter and verify
that the formatter emits the upstream VT restore sequence when that state is
set.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `ScreenFormatter.Extra.protection`;
     - DECSCA output: `\x1b[1\"q`;
     - screen-extra ordering.
   - Use `vendor/ghostty/src/terminal/Screen.zig` for the cursor protection
     state shape.
   - Do not modify `vendor/ghostty/`.

2. Add minimal private cursor protection state.
   - In `roastty/src/terminal/screen.rs`, extend private `ScreenCursor` with:

     ```rust
     protected: bool
     ```

   - Initialize it to `false`.
   - Add a `#[cfg(test)] pub(super)` helper to set the cursor protection flag
     for tests.
   - Do not add parser support for DECSCA, protected-cell storage behavior,
     protected erase behavior, or any runtime cursor movement.

3. Extend `ScreenFormatterExtra`.
   - Add a private `protection: bool` field to `ScreenFormatterExtra`.
   - Extend `none()` and `is_empty()`.
   - Add a `protection(bool)` builder.
   - Do not add placeholder fields for hyperlink, Kitty keyboard, or charsets.

4. Emit protection only for VT output.
   - Plain and HTML output must ignore protection extras.
   - For VT output, append protection after style and before cursor for the
     currently implemented subset.
   - If `extra.protection` is true and `screen.cursor.protected` is true, append
     `\x1b[1\"q`.
   - If `extra.protection` is true but the cursor is not protected, append
     nothing.
   - Do not emit the unprotected/reset form in this experiment. Upstream only
     emits DECSCA when the cursor protected flag is true.

5. Preserve pin-map semantics.
   - Protection extra bytes must be appended to the pin map exactly like
     Experiment 91's cursor/style extra bytes.
   - The implementation should continue choosing the extra pin from the actual
     post-content pin map: last content pin when available, otherwise the screen
     top-left pin.
   - Pin maps must remain byte-indexed.

6. Keep TerminalFormatter delegation intact.
   - Do not add terminal extras.
   - Do not add TerminalFormatter forwarding for screen extras yet.
   - Existing TerminalFormatter default output must remain unchanged even if the
     active screen cursor has `protected = true`.

7. Add upstream-equivalent tests.
   - Add ScreenFormatter tests for:
     - VT protection extra emits `\x1b[1\"q` after content when protected;
     - VT protection extra emits nothing when not protected;
     - style, protection, and cursor extras emit in upstream order for the
       implemented subset;
     - plain and HTML output ignore protection extras;
     - `Content::None` with protection emits only DECSCA when protected;
     - VT pin maps with protection map extra bytes to the last content pin;
     - VT pin maps with no content map protection bytes to top-left.
     - VT pin maps with an invalid or empty selection map protection bytes to
       top-left, proving the implementation uses the actual post-content pin map
       rather than guessing from the requested content mode.
   - Add or extend a TerminalFormatter regression test proving default
     TerminalFormatter output still ignores screen extras.
   - Add or extend a TerminalFormatter pin-map regression test proving a
     protected active-screen cursor does not change default TerminalFormatter
     text, pin-map length, or pin mapping when no screen extras are forwarded.
   - Keep existing ScreenFormatter cursor/style tests passing.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty screen_formatter
     cargo test -p roastty terminal_formatter
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
      - the protection state added and its visibility;
      - the exact VT sequence emitted;
      - how unprotected state behaves;
      - how plain/HTML ignore protection;
      - how pin-map entries for protection bytes are assigned;
      - why parser-driven protection behavior remains deferred;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `ScreenCursor` has private protection state initialized to false;
- `ScreenFormatterExtra` supports a private protection flag;
- VT protection extra emits `\x1b[1\"q` only when requested and the cursor is
  protected;
- unprotected cursor state emits no protection sequence;
- style/protection/cursor ordering matches the implemented upstream subset;
- plain and HTML output ignore protection extras;
- no-extra formatter output remains unchanged;
- protection extra bytes are byte-indexed in pin maps and map to the last
  content pin or top-left pin when there is no content;
- TerminalFormatter's default content delegation remains unchanged;
- no parser support, protected-cell semantics, erase semantics, hyperlink state,
  Kitty keyboard state, charset state, terminal extras, public API, public ABI,
  app behavior, renderer behavior, PTY behavior, clipboard behavior, or UI
  behavior is added;
- `cargo fmt`, targeted formatter tests, PageList formatter tests, PageList
  tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- protection formatting requires parser or protected-cell behavior before the
  formatter can honestly represent the state.

The experiment fails if:

- protection emits for plain or HTML output;
- protection emits when the cursor is not protected;
- protection emits before content or after cursor;
- pin maps become character-indexed or shorter than output bytes;
- TerminalFormatter default delegation regresses;
- the implementation adds unrelated parser, terminal, app, renderer, PTY, public
  API, or ABI behavior.

## Design Review

Codex reviewed the design and agreed that protection is the right next
`ScreenFormatter.Extra` slice after Experiment 91. It uses the existing private
cursor-state shape, adds one private boolean, preserves VT-only extra emission,
and does not require parser-driven DECSCA handling, protected-cell write/erase
semantics, terminal extras, public API, or ABI behavior.

Codex asked for two test clarifications before commit, both applied above:

- add an invalid or empty selection pin-map test for protection extras, proving
  the implementation chooses the extra pin from the actual post-content pin map;
- add a TerminalFormatter pin-map regression proving protected active-screen
  cursor state does not affect default TerminalFormatter text or mappings when
  screen extras are not forwarded.

Codex re-reviewed the updated design and found no remaining blockers. It noted
one non-blocking implementation suggestion: testing both an invalid selection
and a valid selection that emits empty content would make the fallback coverage
clearer, but the committed design already states the required fallback behavior.

## Result

**Result:** Pass

Roastty now has private protection state on `ScreenCursor`:

```rust
protected: bool
```

The flag defaults to `false` and can only be set in tests through a
`#[cfg(test)]` helper. No parser-driven DECSCA handling, protected-cell storage
semantics, protected erase behavior, public API, or C ABI surface was added.

`ScreenFormatterExtra` now includes a private `protection` flag with a builder.
For VT output, protection is emitted after active SGR style and before cursor
position for the currently implemented subset. If the protection extra is
requested and `screen.cursor.protected` is true, the formatter appends:

```text
\x1b[1"q
```

If the cursor is not protected, protection emits nothing. Plain and HTML output
ignore protection extras.

Pin maps remain byte-indexed. Protection bytes are assigned to the last
post-content pin when content emitted pins. If content emits no pins, including
`Content::None`, invalid selections, or a valid whitespace selection trimmed to
empty output, protection bytes map to the screen top-left pin.

`TerminalFormatter` still does not forward screen extras. Regression tests prove
that protected active-screen cursor state does not change default
TerminalFormatter text or pin maps.

Verification passed:

```text
cargo fmt
cargo test -p roastty screen_formatter        # 30 passed
cargo test -p roastty terminal_formatter      # 15 passed
cargo test -p roastty styled_pin_map          # 9 passed
cargo test -p roastty pin_map                 # 40 passed
cargo test -p roastty page_string             # 12 passed
cargo test -p roastty terminal::page_list     # 524 passed
cargo test -p roastty                         # 858 unit + 1 ABI passed
```

Codex result review found no blockers. It confirmed the implementation matches
the upstream Ghostty slice, preserves VT-only behavior, keeps the implemented
ordering as style -> protection -> cursor, emits DECSCA only for protected
cursor state, preserves byte-indexed pin maps, and avoids parser, terminal,
public API, and ABI scope creep. Codex noted one non-blocking naming detail: the
`Content::None` text assertion lives inside the pin-map fallback test rather
than a dedicated output-only test, but it directly asserts the required output.

## Conclusion

Experiment 92 completes the screen protection formatter extra. Roastty can now
restore active SGR style, protection state, and cursor position through
`ScreenFormatter` for the currently ported VT screen-extra subset. Parser-level
protection behavior remains a separate future port.
