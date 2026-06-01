# Experiment 94: Port Screen Formatter Kitty Keyboard Extra

## Description

Port the Kitty keyboard flag state needed by upstream Ghostty's
`ScreenFormatter.Extra.kitty_keyboard` path, then wire the VT-only Kitty
keyboard extra into Roastty's `ScreenFormatter`.

Experiment 93 completed charset restore for the current screen-extra subset. The
remaining `ScreenFormatter` extras are hyperlink and Kitty keyboard. Hyperlink
restore needs cursor-owned hyperlink URI/id state and memory handling; Kitty
keyboard is smaller and self-contained. Upstream stores a fixed-size stack of
Kitty keyboard flags on `Screen` and emits `CSI ={flags};1u` only when the
current flag set differs from disabled.

This experiment is formatter/state plumbing only. It must not add parser support
for CSI > u / CSI < u / CSI = u / CSI ? u, query replies, key encoding, terminal
input behavior, or runtime mutation beyond test helpers.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/kitty/key.zig` for:
     - `Flags`;
     - `FlagStack`;
     - `SetMode`;
     - push/pop/set behavior.
   - Use `vendor/ghostty/src/terminal/Screen.zig` for the screen field default.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for
     `ScreenFormatter.Extra.kitty_keyboard` output and ordering.
   - Do not modify `vendor/ghostty/`.

2. Add a private `kitty` module.
   - Add `roastty/src/terminal/kitty.rs`.
   - Wire it from `roastty/src/terminal/mod.rs` as a private module.
   - Port private value/state types:
     - `KeyFlags` with five booleans:
       - `disambiguate`;
       - `report_events`;
       - `report_alternates`;
       - `report_all`;
       - `report_associated`;
     - `KeyFlagStack` with eight fixed entries and a wrapping index;
     - `KeySetMode` for set/or/not.
   - Add an `int()` helper that matches upstream packed bit ordering:
     - disambiguate = bit 0;
     - report_events = bit 1;
     - report_alternates = bit 2;
     - report_all = bit 3;
     - report_associated = bit 4.
   - Add tests for:
     - flag bit ordering;
     - disabled/current defaults;
     - set/or/not behavior;
     - push/pop behavior;
     - `pop(8)` and a larger pop count reset to disabled because the stack
       length is eight and upstream resets when `n >= len`;
     - wrapping push evicts the oldest slot like upstream, using at least nine
       distinct pushed values so the test actually crosses the eight-entry wrap
       boundary.
   - Keep the module private. Do not expose public API or ABI.

3. Add private screen Kitty keyboard state.
   - In `roastty/src/terminal/screen.rs`, add:

     ```rust
     kitty_keyboard: kitty::KeyFlagStack
     ```

   - Initialize it to disabled/default.
   - Add `#[cfg(test)] pub(super)` helpers to set, push, and pop Kitty keyboard
     flags for formatter tests.
   - Do not add parser mutation, query replies, or key encoding behavior in this
     experiment.

4. Extend `ScreenFormatterExtra`.
   - Add a private `kitty_keyboard: bool` flag.
   - Extend `none()` and `is_empty()`.
   - Add a `kitty_keyboard(bool)` builder.
   - Do not add placeholder fields for hyperlink.

5. Emit Kitty keyboard extras only for VT output.
   - Plain and HTML output must ignore Kitty keyboard extras.
   - Preserve implemented upstream ordering:
     - style;
     - protection;
     - kitty keyboard;
     - charsets;
     - cursor.
   - If `extra.kitty_keyboard` is true and the current flags are not disabled,
     append:

     ```text
     \x1b[={flags};1u
     ```

   - If current flags are disabled, emit nothing.
   - Use decimal formatting for `{flags}`.

6. Preserve pin-map semantics.
   - Kitty keyboard extra bytes must be appended to the pin map exactly like the
     existing screen extra bytes.
   - The implementation must choose the extra pin from the actual post-content
     pin map: last content pin when available, otherwise screen top-left.
   - Pin maps must remain byte-indexed.

7. Keep TerminalFormatter delegation intact.
   - Do not add terminal extras.
   - Do not add TerminalFormatter forwarding for screen extras yet.
   - Existing TerminalFormatter default output and pin maps must remain
     unchanged even if active-screen Kitty keyboard state is non-default.

8. Add upstream-equivalent tests.
   - Add Kitty module tests for the value types and stack behavior.
   - Add ScreenFormatter tests for:
     - disabled Kitty keyboard state emits no extra bytes;
     - non-disabled flags emit the exact `CSI ={flags};1u` sequence;
     - multiple flag bits combine into the correct integer;
     - style, protection, Kitty keyboard, charset, and cursor extras emit in
       upstream order for the implemented subset;
     - plain and HTML ignore Kitty keyboard extras;
     - `Content::None` with Kitty keyboard extras emits only Kitty keyboard
       bytes when flags are non-disabled;
     - Kitty keyboard pin maps with content map extra bytes to the last content
       pin;
     - Kitty keyboard pin maps with `Content::None`, invalid selections, and
       valid empty selections map extra bytes to top-left.
   - Add or extend TerminalFormatter regression tests proving non-default Kitty
     keyboard state does not affect default TerminalFormatter text or pin maps.
   - Keep existing cursor/style/protection/charset tests passing.

9. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty kitty
     cargo test -p roastty screen_formatter
     cargo test -p roastty terminal_formatter
     cargo test -p roastty styled_pin_map
     cargo test -p roastty pin_map
     cargo test -p roastty page_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

10. Independent review.
    - Before implementation, get Codex review of this experiment design.
    - Fix all real design findings before implementation.
    - Record the design-review outcome in this experiment file before
      implementation.
    - After implementation and verification, get Codex review of the completed
      result.
    - Fix all real result findings before proceeding.

11. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - Kitty module/type names and visibility;
      - flag bit ordering and stack behavior coverage;
      - screen Kitty keyboard state defaults;
      - exact VT sequence emitted;
      - how disabled state behaves;
      - how plain/HTML ignore Kitty keyboard extras;
      - how pin-map entries for Kitty keyboard bytes are assigned;
      - why parser/query/key-encoding behavior remains deferred;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private Kitty keyboard module with upstream-equivalent flag and
  stack tests;
- `Screen` has private Kitty keyboard state initialized to disabled;
- `ScreenFormatterExtra` supports a private Kitty keyboard flag;
- VT Kitty keyboard extras emit only when requested and current flags are
  non-disabled;
- emitted VT uses the exact `\x1b[={flags};1u` sequence;
- implemented ordering is style -> protection -> kitty keyboard -> charsets ->
  cursor;
- plain and HTML output ignore Kitty keyboard extras;
- disabled state emits no bytes;
- Kitty keyboard extra bytes are byte-indexed in pin maps and map to the last
  content pin or top-left pin when there is no content;
- TerminalFormatter default content and pin maps remain unchanged;
- no parser support, query replies, key encoding behavior, hyperlink state,
  terminal extras, public API, public ABI, app behavior, renderer behavior, PTY
  behavior, clipboard behavior, or UI behavior is added;
- `cargo fmt`, Kitty module tests, targeted formatter tests, PageList formatter
  tests, PageList tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- Kitty keyboard formatter output requires parser/query/key-encoding behavior
  before the state can be represented honestly.

The experiment fails if:

- Kitty keyboard extras emit for plain or HTML output;
- disabled state emits bytes;
- non-disabled flags do not match upstream packed bit ordering;
- Kitty keyboard bytes are emitted before content, before protection, or after
  charsets/cursor;
- pin maps become character-indexed or shorter than output bytes;
- TerminalFormatter default delegation regresses;
- the implementation adds unrelated parser, terminal, app, renderer, PTY, public
  API, or ABI behavior.

## Design Review

Codex reviewed this design before implementation and found no blocking issues.
It agreed that Kitty keyboard restore is the right next `ScreenFormatter` slice
after charset restore because it is smaller and more self-contained than
hyperlink restore.

Codex specifically approved the scope boundaries: port the five flag bits, the
fixed eight-entry stack, wrapping index behavior, set/or/not mutation, push/pop
semantics, `n >= len` pop reset, screen default state, and VT-only
`\x1b[={flags};1u` output when requested and non-disabled. It also agreed with
the required formatter ordering for the currently implemented subset: style ->
protection -> kitty keyboard -> charsets -> cursor.

The only changes requested were test-precision notes, both applied above:

- the stack wrap/eviction test must push at least nine distinct values so it
  proves behavior past the eight-entry capacity;
- pop-reset tests must cover `pop(8)` specifically, not only an obviously large
  count, because upstream resets when `n >= len`.

With those updates, the design is approved for implementation.

## Result

**Result:** Pass

Experiment 94 ported the private Kitty keyboard formatter state needed by
upstream Ghostty's `ScreenFormatter.Extra.kitty_keyboard` path.

The implementation added a private `roastty/src/terminal/kitty.rs` module with:

- `KeyFlags`, containing the five upstream Kitty keyboard flags: `disambiguate`,
  `report_events`, `report_alternates`, `report_all`, and `report_associated`;
- `KeySetMode` for set/or/not mutation;
- `KeyFlagStack`, with eight fixed entries and wrapping push/pop behavior.

The flag bit ordering matches upstream: disambiguate is bit 0, report-events is
bit 1, report-alternates is bit 2, report-all is bit 3, and report-associated is
bit 4. Tests cover defaults, bit packing, set/or/not behavior, push/pop,
`pop(8)`, larger pop counts, and nine distinct pushes to prove wrap/eviction
past the eight-entry stack boundary.

`Screen` now owns private Kitty keyboard state initialized to disabled/default.
The only mutation path added in this experiment is test-only helper plumbing. No
parser handling for CSI `> u`, `< u`, `= u`, or `? u` was added. Query replies,
key encoding, terminal input behavior, public API, and public ABI all remain
deferred.

`ScreenFormatterExtra` now has a private `kitty_keyboard` flag. VT output emits
Kitty keyboard state only when the extra is requested and the current state is
non-disabled. The emitted sequence is exactly:

```text
\x1b[={flags};1u
```

Disabled state emits no bytes. Plain and HTML output ignore the Kitty keyboard
extra, matching the existing screen-extra format split.

The implemented screen-extra ordering is now:

```text
style -> protection -> kitty keyboard -> charsets -> cursor
```

This matches upstream ordering for the currently implemented subset, with
hyperlink restore still deferred.

Pin-map behavior reuses the existing byte-indexed extra mapping rule. Kitty
keyboard extra bytes map to the last content pin when formatted content exists,
and to the screen top-left pin when content is absent, invalid, or a valid empty
selection. Tests cover all of those cases.

`TerminalFormatter` remains unchanged: it still delegates default active-screen
content without forwarding screen extras. Regression tests set non-default Kitty
keyboard state on the active screen and confirm default terminal text and pin
maps remain unchanged.

Verification passed:

```text
cargo fmt
cargo test -p roastty kitty                         # 18 passed
cargo test -p roastty screen_formatter              # 44 passed
cargo test -p roastty terminal_formatter            # 15 passed
cargo test -p roastty styled_pin_map                # 9 passed
cargo test -p roastty pin_map                       # 45 passed
cargo test -p roastty page_string                   # 12 passed
cargo test -p roastty terminal::page_list           # 524 passed
cargo test -p roastty                               # 888 unit + 1 ABI passed
```

Codex result review approved the implementation with no required changes. It
confirmed that the implementation matches the design, preserves scope, keeps
TerminalFormatter delegation unchanged, avoids parser/query/key-encoding/API/ABI
creep, and can be recorded as a Pass.

## Conclusion

Roastty now has the Kitty keyboard state and VT formatter restore path needed by
the current screen-extra subset. This closes another formatter-state gap while
keeping runtime parser and input behavior out of scope.

The remaining known `ScreenFormatter` extra from this group is hyperlink
restore. That is larger than Kitty keyboard because it needs cursor-owned
hyperlink URI/id state and the corresponding memory/storage model, so it should
be designed as the next coherent formatter slice rather than folded into this
experiment.
