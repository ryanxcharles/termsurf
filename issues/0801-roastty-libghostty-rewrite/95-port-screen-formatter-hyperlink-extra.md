# Experiment 95: Port Screen Formatter Hyperlink Extra

## Description

Port the cursor-owned hyperlink state needed by upstream Ghostty's
`ScreenFormatter.Extra.hyperlink` path, then wire the VT-only OSC 8 hyperlink
extra into Roastty's `ScreenFormatter`.

Experiments 91-94 ported the current `ScreenFormatter` extras for cursor, style,
protection, Kitty keyboard, and charsets. The remaining screen extra in this
group is hyperlink restore. Upstream stores the active OSC 8 hyperlink on the
cursor separately from already-written page hyperlink metadata so the formatter
can recreate active hyperlink state after formatted content.

This experiment is formatter/state plumbing only. It must not add OSC 8 parser
support, runtime hyperlink start/end mutation, page-cell hyperlink writes, HTML
anchor output, terminal input behavior, public API, or public ABI.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `ScreenFormatter.Extra.hyperlink`;
     - OSC 8 output for implicit and explicit IDs;
     - screen-extra ordering.
   - Use `vendor/ghostty/src/terminal/Screen.zig` for:
     - cursor active hyperlink fields;
     - active hyperlink lifetime rationale.
   - Use `vendor/ghostty/src/terminal/hyperlink.zig` for:
     - implicit versus explicit hyperlink IDs;
     - URI/id ownership model.
   - Do not modify `vendor/ghostty/`.

2. Add private cursor hyperlink state.
   - In `roastty/src/terminal/screen.rs`, add a private cursor hyperlink type
     associated with the cursor. The implementation may either remove `Copy`
     from `ScreenCursor` and store this directly on the cursor, or keep the
     owned hyperlink state outside the copied cursor shape if preserving
     `ScreenCursor: Copy` is clearer. The design intent is cursor-owned active
     hyperlink state, not a public or page-cell hyperlink API. Example shape:

     ```rust
     struct ScreenCursorHyperlink {
         id: ScreenCursorHyperlinkId,
         uri: String,
     }

     enum ScreenCursorHyperlinkId {
         Explicit(String),
         Implicit(u32),
     }
     ```

   - Initialize cursor hyperlink state to `None`.
   - If `Implicit(u32)` is used, the numeric value is private identity/parity
     data only. It must not appear in formatter output; upstream implicit
     hyperlink restore emits `OSC 8 ;;{uri} ST`.
   - Add `#[cfg(test)] pub(super)` helpers to set and clear cursor hyperlink
     state for formatter tests.
   - Keep the state private to the terminal module. Do not expose public API or
     ABI.
   - Use owned Rust strings for this formatter slice because the current Roastty
     formatter surface returns `String`. Arbitrary non-UTF-8 OSC 8 payload
     parity is deferred until a future byte-buffer formatter pass, if needed.

3. Extend `ScreenFormatterExtra`.
   - Add a private `hyperlink: bool` flag.
   - Extend `none()` and `is_empty()`.
   - Add a `hyperlink(bool)` builder.

4. Emit hyperlink extras only for VT output.
   - Plain and HTML output must ignore hyperlink extras.
   - Preserve upstream ordering for the implemented subset:
     - style;
     - hyperlink;
     - protection;
     - Kitty keyboard;
     - charsets;
     - cursor.
   - If `extra.hyperlink` is true and the active cursor hyperlink is present:
     - for an explicit ID, append:

       ```text
       \x1b]8;id={id};{uri}\x1b\
       ```

     - for an implicit ID, append:

       ```text
       \x1b]8;;{uri}\x1b\
       ```

   - If no active cursor hyperlink is present, emit nothing.
   - Do not escape URI or ID values for VT output. Upstream writes the OSC 8
     payload bytes directly.

5. Preserve pin-map semantics.
   - Hyperlink extra bytes must be appended to the pin map exactly like the
     existing screen extra bytes.
   - The implementation must choose the extra pin from the actual post-content
     pin map: last content pin when available, otherwise screen top-left.
   - Pin maps must remain byte-indexed.

6. Keep TerminalFormatter delegation intact.
   - Do not add terminal extras.
   - Do not add TerminalFormatter forwarding for screen extras yet.
   - Existing TerminalFormatter default output and pin maps must remain
     unchanged even if active-screen cursor hyperlink state is non-default.

7. Add upstream-equivalent tests.
   - Add ScreenFormatter tests for:
     - absent cursor hyperlink emits no extra bytes;
     - implicit cursor hyperlink emits exact OSC 8 open sequence;
     - implicit cursor hyperlink output does not include the private implicit
       numeric value;
     - explicit cursor hyperlink emits exact OSC 8 open sequence with `id={id}`;
     - URI and explicit ID values containing HTML-special characters are emitted
       raw for VT output, not HTML-escaped;
     - style, hyperlink, protection, Kitty keyboard, charset, and cursor extras
       emit in upstream order for the implemented subset;
     - plain and HTML ignore hyperlink extras;
     - `Content::None` with hyperlink extra emits only OSC 8 bytes when a cursor
       hyperlink is active;
     - hyperlink pin maps with content map extra bytes to the last content pin;
     - hyperlink pin maps with `Content::None`, invalid selections, and valid
       empty selections map extra bytes to top-left.
     - at least one hyperlink pin-map test uses a multibyte UTF-8 URI or
       explicit ID, asserts `text.len() == pin_map.len()`, and verifies every
       emitted OSC 8 byte maps to the expected last-content or top-left pin.
   - Add or extend TerminalFormatter regression tests proving non-default cursor
     hyperlink state does not affect default TerminalFormatter text or pin maps.
   - Keep existing cursor/style/protection/Kitty/charset tests passing.

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
      - cursor hyperlink type names and visibility;
      - implicit versus explicit output behavior;
      - exact OSC 8 sequences emitted;
      - how absent hyperlink state behaves;
      - how plain/HTML ignore hyperlink extras;
      - how pin-map entries for hyperlink bytes are assigned;
      - why parser/runtime/page-cell hyperlink behavior remains deferred;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `ScreenCursor` has private active hyperlink state initialized to absent;
- `ScreenFormatterExtra` supports a private hyperlink flag;
- VT hyperlink extras emit only when requested and an active cursor hyperlink is
  present;
- implicit hyperlink state emits the exact `\x1b]8;;{uri}\x1b\` sequence;
- implicit hyperlink state does not emit its private numeric identity value;
- explicit hyperlink state emits the exact `\x1b]8;id={id};{uri}\x1b\` sequence;
- implemented ordering is style -> hyperlink -> protection -> kitty keyboard ->
  charsets -> cursor;
- plain and HTML output ignore hyperlink extras;
- absent hyperlink state emits no bytes;
- hyperlink extra bytes are byte-indexed in pin maps and map to the last content
  pin or top-left pin when there is no content;
- TerminalFormatter default content and pin maps remain unchanged;
- no OSC 8 parser support, runtime start/end hyperlink mutation, page-cell
  hyperlink writes, HTML anchor output, terminal extras, public API, public ABI,
  app behavior, renderer behavior, PTY behavior, clipboard behavior, or UI
  behavior is added;
- `cargo fmt`, targeted formatter tests, PageList formatter tests, PageList
  tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- active cursor hyperlink state cannot be represented honestly without first
  porting runtime OSC 8 parser/start/end behavior.

The experiment fails if:

- hyperlink extras emit for plain or HTML output;
- absent cursor hyperlink state emits bytes;
- explicit and implicit OSC 8 output do not match upstream sequence shape;
- hyperlink bytes are emitted before style, after protection, or after
  charsets/cursor;
- pin maps become character-indexed or shorter than output bytes;
- TerminalFormatter default delegation regresses;
- the implementation adds unrelated parser, page-cell hyperlink mutation,
  terminal, app, renderer, PTY, public API, or ABI behavior.

## Design Review

Codex reviewed this design before implementation and agreed that hyperlink
restore is the correct final `ScreenFormatter` extra slice after Experiment 94.
It confirmed the upstream ordering, OSC 8 sequence shapes, TerminalFormatter
scope boundary, and parser/query/key-encoding/API/ABI exclusions.

Codex found three required design fixes, all applied above:

- the design now explicitly handles the owned hyperlink state versus
  `ScreenCursor: Copy` trait-shape conflict by requiring either removal of
  `Copy` from `ScreenCursor` or storage outside the copied cursor shape;
- the design now states that an implicit numeric ID is private identity/parity
  data only and must not appear in OSC 8 output;
- the design now requires multibyte UTF-8 hyperlink pin-map coverage so
  byte-indexed pin-map behavior is proven for non-ASCII OSC payload bytes.

Codex re-reviewed the revised design and found no remaining required changes. It
explicitly approved the design for implementation.

## Result

**Result:** Pass

Experiment 95 ported the private active hyperlink state needed by upstream
Ghostty's `ScreenFormatter.Extra.hyperlink` path.

The implementation added cursor-associated private state in
`roastty/src/terminal/screen.rs`:

- `ScreenCursorHyperlink`, containing an owned URI string and hyperlink ID;
- `ScreenCursorHyperlinkId`, with `Explicit(String)` and `Implicit(u32)`;
- test-only helpers to set and clear active cursor hyperlink state.

Because the state owns strings, `ScreenCursor` is no longer `Copy`. This follows
the design-review decision to keep active hyperlink state directly associated
with the cursor rather than inventing a second cursor-adjacent storage path.

`ScreenFormatterExtra` now has a private `hyperlink` flag. VT output emits
hyperlink state only when the extra is requested and active cursor hyperlink
state is present. Absent state emits no bytes.

Implicit hyperlinks emit exactly:

```text
\x1b]8;;{uri}\x1b\
```

The private implicit numeric identity is not emitted. Explicit hyperlinks emit
exactly:

```text
\x1b]8;id={id};{uri}\x1b\
```

URI and explicit ID payloads are emitted raw for VT output, matching upstream's
OSC 8 behavior. Plain and HTML output ignore hyperlink extras.

The implemented screen-extra ordering is now:

```text
style -> hyperlink -> protection -> kitty keyboard -> charsets -> cursor
```

Pin-map behavior reuses the existing byte-indexed extra mapping rule. Hyperlink
extra bytes map to the last content pin when formatted content exists, and to
the screen top-left pin when content is absent, invalid, or a valid empty
selection. Tests include a multibyte UTF-8 URI/id case to prove byte-indexed
pin-map length and mapping behavior for non-ASCII OSC payload bytes.

`TerminalFormatter` remains unchanged: it still delegates default active-screen
content without forwarding screen extras. Regression tests set non-default
cursor hyperlink state on the active screen and confirm default terminal text
and pin maps remain unchanged.

No OSC 8 parser support, runtime start/end hyperlink mutation, page-cell
hyperlink writes, HTML anchor output, terminal extras, public API, public ABI,
app behavior, renderer behavior, PTY behavior, clipboard behavior, or UI
behavior was added.

Verification passed:

```text
cargo fmt
cargo test -p roastty screen_formatter              # 54 passed
cargo test -p roastty terminal_formatter            # 15 passed
cargo test -p roastty styled_pin_map                # 9 passed
cargo test -p roastty pin_map                       # 50 passed
cargo test -p roastty page_string                   # 12 passed
cargo test -p roastty terminal::page_list           # 524 passed
cargo test -p roastty                               # 898 unit + 1 ABI passed
```

Codex result review approved the implementation with no required changes. It
confirmed the cursor-owned state, upstream extra ordering, explicit and implicit
OSC 8 sequence shapes, byte-indexed pin-map behavior, and unchanged
TerminalFormatter delegation.

## Conclusion

Roastty now has the full currently planned `ScreenFormatter` screen-extra subset
ported: cursor, style, hyperlink, protection, Kitty keyboard, charsets, and
their byte-indexed pin-map behavior.

The next experiment should move to the next formatter/state slice outside
`ScreenFormatter` extras. The likely candidate is the first `TerminalFormatter`
terminal-extra field, chosen by re-reading upstream `formatter.zig` and picking
the smallest coherent state surface that can be represented and tested without
parser/runtime scope creep.
