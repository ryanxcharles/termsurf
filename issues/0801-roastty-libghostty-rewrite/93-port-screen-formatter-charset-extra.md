# Experiment 93: Port Screen Formatter Charset Extra

## Description

Port the screen charset state needed by upstream Ghostty's
`ScreenFormatter.Extra.charsets` path, then wire the VT-only charset extra into
Roastty's `ScreenFormatter`.

Experiments 91 and 92 added private cursor/style/protection screen state and the
corresponding VT screen extras. The next screen extra that can be ported without
parser work is charset restore. Upstream stores G0-G3 charset designations plus
GL/GR active-slot invocations on `Screen`, and the formatter emits only
non-default designations/invocations so replay can restore charset state.

This experiment is formatter/state plumbing only. It must not add parser support
for ESC charset designation sequences, single-shift handling, character
translation during printing, or runtime charset mutation beyond test helpers.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/charsets.zig` for:
     - charset slots G0-G3;
     - active slots GL/GR;
     - charset values `utf8`, `ascii`, `british`, and `dec_special`;
     - charset translation tables.
   - Use `vendor/ghostty/src/terminal/Screen.zig` for `CharsetState`.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for
     `ScreenFormatter.Extra.charsets` output and ordering.
   - Do not modify `vendor/ghostty/`.

2. Add a private `charsets` module.
   - Add `roastty/src/terminal/charsets.rs`.
   - Wire it from `roastty/src/terminal/mod.rs` as a private module.
   - Port private value types:
     - `CharsetSlot` for G0, G1, G2, G3;
     - `CharsetGrSlot` for the upstream-valid GR slots G1, G2, and G3;
     - `ActiveSlot` for GL, GR if useful for clarity;
     - `Charset` for Utf8, Ascii, British, DecSpecial.
   - Port the static 256-entry translation tables for ASCII, British, and DEC
     special graphics, or a faithful helper that returns the same table values.
     These tables are private parity data only in this experiment. They must not
     be wired into parser mutation or print-time character translation yet.
   - Add tests proving:
     - non-UTF8 tables have 256 entries;
     - ASCII table maps byte values to themselves;
     - British maps `#` to `£`;
     - DEC special maps key documented bytes such as `` ` ``, `j`, `k`, `l`,
       `m`, `q`, `x`, and `~` to the upstream codepoints.
   - Keep the module private. Do not expose public API or ABI.

3. Add private screen charset state.
   - In `roastty/src/terminal/screen.rs`, add a private charset state matching
     the upstream shape needed by formatting:

     ```rust
     struct ScreenCharsetState {
         g0: Charset,
         g1: Charset,
         g2: Charset,
         g3: Charset,
         gl: CharsetSlot,
         gr: CharsetGrSlot,
     }
     ```

   - Defaults must match upstream: all designations `Utf8`, GL = G0, GR = G2.
   - Add narrow methods to read a slot's designation.
   - Add `#[cfg(test)] pub(super)` helpers to set G0-G3, GL, and GR for tests.
   - Do not add parser mutation, single-shift state, or print-time character
     translation in this experiment.

4. Extend `ScreenFormatterExtra`.
   - Add a private `charsets: bool` flag.
   - Extend `none()` and `is_empty()`.
   - Add a `charsets(bool)` builder.
   - Do not add placeholder fields for hyperlink or Kitty keyboard.

5. Emit charset extras only for VT output.
   - Plain and HTML output must ignore charset extras.
   - Preserve implemented upstream ordering:
     - style;
     - protection;
     - charsets;
     - cursor.
   - For G0-G3 designations, emit only non-default `Utf8` designations:
     - G0: `ESC ( final`
     - G1: `ESC ) final`
     - G2: `ESC * final`
     - G3: `ESC + final`
   - Final bytes:
     - Ascii: `B`
     - British: `A`
     - DecSpecial: `0`
     - Utf8: no output
   - Emit GL invocation only if GL is not G0:
     - G1: SO (`\x0e`)
     - G2: `ESC n`
     - G3: `ESC o`
   - Emit GR invocation only if GR is not G2:
     - G1: `ESC ~`
     - G3: `ESC |`
     - G0 is not representable as a GR state in Roastty. Use a private
       `CharsetGrSlot` type that can only represent G1, G2, and G3. This
       preserves upstream's `unreachable` assumption without introducing a
       normal-formatting panic or an error-returning formatter API.

6. Preserve pin-map semantics.
   - Charset extra bytes must be appended to the pin map exactly like the
     existing cursor/style/protection extra bytes.
   - The implementation must choose the extra pin from the actual post-content
     pin map: last content pin when available, otherwise screen top-left.
   - Pin maps must remain byte-indexed.

7. Keep TerminalFormatter delegation intact.
   - Do not add terminal extras.
   - Do not add TerminalFormatter forwarding for screen extras yet.
   - Existing TerminalFormatter default output and pin maps must remain
     unchanged even if active-screen charset state is non-default.

8. Add upstream-equivalent tests.
   - Add charset module tests for the value types and translation tables.
   - Add ScreenFormatter tests for:
     - default charset state emits no extra bytes;
     - non-default G0-G3 designations emit the exact upstream ESC sequences;
     - GL invocations emit SO/LS2/LS3 as applicable;
     - GR invocations emit LS1R/LS3R as applicable;
     - style, protection, charset, and cursor extras emit in upstream order for
       the implemented subset;
     - plain and HTML ignore charset extras;
     - `Content::None` with charset extras emits only charset bytes;
     - charset pin maps with content map extra bytes to the last content pin;
     - charset pin maps with `Content::None`, invalid selections, and valid
       empty selections map extra bytes to top-left.
   - Add or extend TerminalFormatter regression tests proving non-default
     charset state does not affect default TerminalFormatter text or pin maps.
   - Keep existing cursor/style/protection tests passing.

9. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty charsets
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
      - charset module/type names and visibility;
      - table parity coverage;
      - screen charset state defaults;
      - exact VT designation/invocation sequences emitted;
      - how invalid G0-as-GR state is avoided by `CharsetGrSlot`;
      - how plain/HTML ignore charset extras;
      - how pin-map entries for charset bytes are assigned;
      - why parser-driven charset behavior remains deferred;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private charset module with upstream-equivalent value types and
  table tests;
- `Screen` has private charset state with upstream defaults;
- G0 is not representable as a GR state in the normal Rust type;
- `ScreenFormatterExtra` supports a private charset flag;
- VT charset extras emit only requested non-default designations/invocations;
- implemented ordering is style -> protection -> charsets -> cursor;
- plain and HTML output ignore charset extras;
- default charset state emits no bytes;
- charset extra bytes are byte-indexed in pin maps and map to the last content
  pin or top-left pin when there is no content;
- TerminalFormatter default content and pin maps remain unchanged;
- no parser support, single-shift behavior, print-time charset translation,
  hyperlink state, Kitty keyboard state, terminal extras, public API, public
  ABI, app behavior, renderer behavior, PTY behavior, clipboard behavior, or UI
  behavior is added;
- `cargo fmt`, charset tests, targeted formatter tests, PageList formatter
  tests, PageList tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- charset formatter output requires a broader parser/printing charset port
  before the state can be represented honestly.

The experiment fails if:

- charset extras emit for plain or HTML output;
- default charset state emits bytes;
- non-default designations or invocations do not match upstream ESC sequences;
- charset bytes are emitted before content or after cursor;
- pin maps become character-indexed or shorter than output bytes;
- TerminalFormatter default delegation regresses;
- the implementation adds unrelated parser, terminal, app, renderer, PTY, public
  API, or ABI behavior.

## Design Review

Codex reviewed the design and agreed that charset restore is the right next
screen-extra slice after Experiment 92. It is VT-only formatter/state plumbing,
preserves the implemented ordering, and avoids parser, print-time translation,
terminal-extra, public API, and ABI scope.

Codex raised one design issue before approval: invalid G0-as-GR state was
under-specified. The design now requires a private `CharsetGrSlot` type that can
represent only G1, G2, and G3, with G2 as the default. This preserves upstream's
`unreachable` assumption without adding normal-formatting panics or an
error-returning formatter API.

Codex also asked that charset translation tables be described as private parity
data only. The design now explicitly forbids wiring those tables into parser
mutation or print-time character translation in this experiment.

Codex re-reviewed the updated design and found no remaining blockers. It noted
one non-blocking implementation preference: make GR emission return an optional
sequence for G1/G3 and `None` for default G2, rather than using `unreachable!()`
for any normal state.

## Result

**Result:** Pass

Roastty now has a private `roastty/src/terminal/charsets.rs` module with:

- `CharsetSlot` for G0, G1, G2, and G3;
- `CharsetGrSlot` for only the upstream-valid GR slots G1, G2, and G3;
- `Charset` for Utf8, Ascii, British, and DecSpecial.

`CharsetGrSlot` makes G0-as-GR unrepresentable in normal Rust code. GR emission
uses an optional sequence helper: G1 emits `\x1b~`, default G2 emits nothing,
and G3 emits `\x1b|`.

The charset translation tables were added only as private test parity helpers
under `#[cfg(test)]`. They are not wired into parser mutation or print-time
translation. Tests cover table length, ASCII identity mapping, British `#` ->
`£`, and key DEC special graphics mappings.

`Screen` now has private charset state with upstream defaults:

- all G0-G3 designations default to Utf8;
- GL defaults to G0;
- GR defaults to G2.

`ScreenFormatterExtra` now includes a private `charsets` flag. Charset extras
are emitted only for VT output and only when requested. Default charset state
emits no bytes. Non-default designations and invocations emit the upstream
restore sequences:

- G0: `ESC ( final`
- G1: `ESC ) final`
- G2: `ESC * final`
- G3: `ESC + final`
- Ascii final: `B`
- British final: `A`
- DEC special final: `0`
- GL G1: SO (`\x0e`)
- GL G2: `ESC n`
- GL G3: `ESC o`
- GR G1: `ESC ~`
- GR G3: `ESC |`

For the currently implemented screen-extra subset, ordering is:

```text
style -> protection -> charsets -> cursor
```

Pin maps remain byte-indexed. Charset extra bytes are assigned to the last
post-content pin when content emitted pins, or to the screen top-left pin when
content emitted no pins, including `Content::None`, invalid selections, and
valid selections trimmed to empty output.

`TerminalFormatter` still does not forward screen extras. Regression tests prove
that non-default charset state does not change default TerminalFormatter text or
pin maps.

Verification passed:

```text
cargo fmt
cargo test -p roastty charsets                 # 6 passed
cargo test -p roastty screen_formatter         # 36 passed
cargo test -p roastty terminal_formatter       # 15 passed
cargo test -p roastty styled_pin_map           # 9 passed
cargo test -p roastty pin_map                  # 41 passed
cargo test -p roastty page_string              # 12 passed
cargo test -p roastty terminal::page_list      # 524 passed
cargo test -p roastty                          # 870 unit + 1 ABI passed
```

Codex result review found no blockers. It confirmed invalid GR state is avoided
by the type shape, screen charset defaults match upstream, charset extras are
VT-only and correctly ordered, designation/invocation bytes match Ghostty's
formatter path, pin maps remain byte-indexed, and TerminalFormatter continues to
delegate without screen extras. Codex noted one non-blocking recording caveat
that the charset tables should be described as table parity test helpers, not
production formatter behavior; this result records them that way.

## Conclusion

Experiment 93 completes charset restore for `ScreenFormatter` without adding
parser or print-time charset behavior. Roastty can now restore active SGR style,
protection state, charset designations/invocations, and cursor position for the
ported VT screen-extra subset.
