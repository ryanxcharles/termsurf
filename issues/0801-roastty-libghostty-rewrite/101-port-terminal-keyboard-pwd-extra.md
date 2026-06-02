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

# Experiment 101: Port Terminal Keyboard and Pwd Formatter Extras

## Description

Port the remaining post-screen `TerminalFormatter.Extra` fields from upstream
Ghostty's terminal formatter: `keyboard` and `pwd`.

Experiment 100 completed the tabstops terminal formatter extra. Upstream Ghostty
then emits two more terminal-level VT extras after tabstops:

- keyboard mode state for `modify_other_keys_2`, emitted as `CSI > 4 ; 2 m`;
- present working directory state, emitted as `OSC 7`.

Roastty already has the screen-level Kitty keyboard extra, but it does not yet
have the terminal-level `modify_other_keys_2` flag or terminal PWD state. This
experiment adds only the private state and opt-in formatter serialization needed
to match upstream formatter behavior. It must not add a VT parser, OSC parser,
runtime terminal mutation, PTY integration, public API, public ABI, app
behavior, renderer behavior, clipboard behavior, or UI behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `TerminalFormatter.Extra.keyboard`;
     - `TerminalFormatter.Extra.pwd`;
     - post-screen ordering after `scrolling_region` and `tabstops`;
     - pin-map behavior for post-screen terminal extras.
   - Use `vendor/ghostty/src/terminal/Terminal.zig` for:
     - `flags.modify_other_keys_2`;
     - `pwd` storage;
     - `setPwd()` and `getPwd()` behavior.
   - Do not modify `vendor/ghostty/`.

2. Add private terminal state.
   - Add a private terminal flags struct with at least
     `modify_other_keys_2: bool`.
   - Initialize `modify_other_keys_2` to `false`.
   - Add private PWD storage to `Terminal`.
   - Store PWD in the same logical shape as upstream: empty means no PWD; a
     non-empty PWD has a terminator in storage and a getter that exposes the
     logical value without the terminator.
   - Add `#[cfg(test)] pub(super)` helpers to:
     - set `modify_other_keys_2`;
     - inspect `modify_other_keys_2`;
     - set PWD;
     - clear PWD;
     - inspect logical PWD.
   - Keep all state private. Do not expose public API or ABI.

3. Extend `TerminalFormatterExtra`.
   - Add `keyboard: bool`.
   - Add `pwd: bool`.
   - Extend `none()`.
   - Add `.keyboard(bool)` and `.pwd(bool)` builders.
   - Keep `TerminalFormatter::init()` defaulting to no extras.

4. Emit keyboard and PWD after tabstops.
   - Only VT output emits these extras.
   - Plain and HTML ignore these extras.
   - Preserve upstream post-screen ordering:
     `scrolling region -> tabstops -> keyboard -> pwd`.
   - When `keyboard` is enabled and `modify_other_keys_2` is true, emit:

     ```text
     \x1b[>4;2m
     ```

   - When `keyboard` is enabled and `modify_other_keys_2` is false, emit
     nothing.
   - When `pwd` is enabled and the stored PWD is non-empty, emit:

     ```text
     \x1b]7;{stored_pwd}\x1b\
     ```

   - When `pwd` is enabled and no PWD is stored, emit nothing.
   - Match upstream's exact PWD byte behavior for valid UTF-8 PWD values:
     - `setPwd()` stores the logical PWD text followed by a trailing NUL byte;
     - `getPwd()` exposes the logical PWD without the trailing NUL;
     - `TerminalFormatter` writes the stored `pwd.items` bytes directly, not the
       logical getter.
   - Therefore, Roastty formatter output must include the stored trailing NUL
     byte before the OSC string terminator when PWD is non-empty:

     ```text
     \x1b]7;file://host/home/user\0\x1b\
     ```

   - Roastty's formatter currently returns `String`, so this experiment's PWD
     storage accepts valid UTF-8 text plus the stored NUL terminator. Do not
     escape, sanitize, normalize, or URL-encode those stored UTF-8 bytes in this
     experiment. Upstream emits the stored bytes raw and terminates the OSC with
     ST (`ESC \`). Parser-side validation, sanitization, and any future
     arbitrary non-UTF-8 byte-output path are outside this formatter-only slice.

5. Preserve post-screen pin-map semantics.
   - Keyboard and PWD bytes are generated terminal-state bytes appended after
     screen formatter output and earlier terminal suffix extras.
   - Map appended keyboard and PWD bytes to the last existing pin when output
     already has content, screen extras, palette bytes, mode bytes,
     scrolling-region bytes, or tabstop bytes.
   - If the formatter emits only keyboard or PWD bytes, map them to active
     screen top-left.
   - Pin maps must remain byte-indexed.

6. Add upstream-equivalent tests.
   - Add TerminalFormatter tests for:
     - default output does not emit keyboard or PWD bytes even when stored state
       is non-default;
     - default pin maps remain unchanged when stored keyboard/PWD state is
       non-default but `TerminalFormatterExtra::none()` is used;
     - `keyboard` extra emits `CSI > 4 ; 2 m` only when `modify_other_keys_2` is
       true;
     - `keyboard` extra emits nothing when `modify_other_keys_2` is false;
     - `pwd` extra emits `OSC 7` only when a PWD is stored;
     - `pwd` exact output includes raw stored bytes, the stored trailing NUL
       byte, and ST termination;
     - `pwd` extra emits nothing for empty PWD;
     - keyboard and PWD emit after scrolling-region and tabstop bytes when all
       suffix extras are enabled;
     - palette, modes, content, screen extras, scrolling region, tabstops,
       keyboard, and PWD combine with ordering
       `palette -> modes -> content -> screen extras -> scrolling region -> tabstops -> keyboard -> pwd`;
     - plain and HTML ignore both extras;
     - `Content::None` can emit only keyboard/PWD bytes for VT;
     - pin maps are byte-indexed;
     - content plus prior suffix extras plus keyboard plus PWD in
       `format_with_pin_map()` has `text.len() == pin_map.len()` across the
       exact output bytes, including the PWD trailing NUL byte;
     - post-screen keyboard/PWD bytes map to the last existing pin when one
       exists;
     - post-screen keyboard/PWD bytes map to top-left when no prior bytes exist.
   - Keep existing tabstops, modes, TerminalFormatter, ScreenFormatter, PageList
     formatter, and PageList tests passing.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal_formatter
     cargo test -p roastty modes
     cargo test -p roastty tabstops
     cargo test -p roastty screen_formatter
     cargo test -p roastty styled_pin_map
     cargo test -p roastty pin_map
     cargo test -p roastty page_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - terminal flag and PWD state names and visibility;
     - default state;
     - exact `CSI > 4 ; 2 m` sequence behavior;
     - exact `OSC 7` sequence behavior, including raw byte emission, the stored
       trailing NUL byte, and ST termination;
     - plain/HTML no-op behavior;
     - ordering relative to palette, modes, content, screen extras, scrolling
       region, and tabstops;
     - pin-map behavior for post-screen generated bytes;
     - why parser/runtime mutation, PTY integration, public API, and ABI remain
       deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Terminal` owns private keyboard/PWD state matching upstream's logical
  formatter needs;
- `TerminalFormatterExtra` has opt-in `keyboard` and `pwd` flags;
- default TerminalFormatter output and pin maps remain unchanged;
- VT keyboard output emits `CSI > 4 ; 2 m` only when `modify_other_keys_2` is
  true;
- VT PWD output emits `OSC 7` only when PWD is non-empty;
- VT PWD output emits stored valid UTF-8 PWD bytes without escaping or
  normalization, including the stored trailing NUL byte before ST;
- keyboard and PWD bytes emit after scrolling-region and tabstop bytes;
- palette, modes, content, screen extras, scrolling region, tabstops, keyboard,
  and PWD can combine with ordering
  `palette -> modes -> content -> screen extras -> scrolling region -> tabstops -> keyboard -> pwd`;
- plain and HTML output ignore keyboard and PWD extras;
- generated keyboard/PWD bytes are byte-indexed in pin maps and map to the last
  existing pin, or top-left when there is no prior output;
- no VT parser/runtime mutation, OSC parser, public API, public ABI, PTY
  integration, app behavior, renderer behavior, clipboard behavior, or UI
  behavior is added;
- `cargo fmt`, targeted formatter tests, tabstops tests, modes tests, PageList
  formatter tests, PageList tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- keyboard/PWD formatter serialization cannot be represented honestly without
  first adding terminal parser/runtime state, and that prerequisite is
  identified precisely.

The experiment fails if:

- default TerminalFormatter output changes;
- keyboard or PWD bytes emit without explicit `TerminalFormatter::with_extra()`;
- HTML or plain output emits keyboard/PWD bytes;
- keyboard or PWD bytes emit before content, screen extras, scrolling-region
  bytes, or tabstop bytes;
- keyboard output emits when `modify_other_keys_2` is false;
- PWD output emits for empty state;
- PWD output serializes a different byte sequence than upstream, including
  missing or moving the stored trailing NUL byte;
- generated keyboard/PWD pin maps become character-indexed, shorter than output
  bytes, or map to top-left when prior content pins exist;
- runtime parser, public API, ABI, PTY, app, render, or UI behavior is added.

## Design Review

Codex reviewed this design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-002439-201214-prompt.md`
- Result: `logs/codex-review/20260601-002439-201214-last-message.md`

Codex found three real design gaps:

- PWD serialization had to specify the exact upstream trailing-NUL behavior
  before implementation;
- OSC 7 output had to specify raw stored valid-UTF-8 byte emission and ST
  termination, with no escaping or sanitization in this formatter-only slice;
- pin-map tests had to cover exact PWD bytes, including the trailing NUL byte.

All three findings were applied.

Re-review artifacts:

- Prompt: `logs/codex-review/20260601-002623-862778-prompt.md`
- Result: `logs/codex-review/20260601-002623-862778-last-message.md`

Codex found no remaining real findings and approved implementation.

## Result

**Result:** Pass

Implemented private terminal keyboard-mode and PWD state plus the opt-in
terminal formatter extras that serialize them.

Code changes:

- `Terminal` now owns private `flags: TerminalFlags` state.
- `TerminalFlags` currently contains `modify_other_keys_2: bool`, defaulting to
  `false`.
- `Terminal` now owns private `pwd: TerminalPwd` state.
- `TerminalPwd` stores upstream-shaped valid UTF-8 text:
  - empty storage means no PWD;
  - non-empty storage is logical PWD text followed by a trailing NUL byte;
  - the test getter exposes the logical PWD without the trailing NUL.
- Test-only helpers can set/inspect `modify_other_keys_2`, set/clear PWD, and
  inspect logical PWD.
- `TerminalFormatterExtra` now has opt-in `keyboard: bool` and `pwd: bool` flags
  with `.keyboard(bool)` and `.pwd(bool)` builders.

Formatter behavior:

- Default `TerminalFormatter::init()` still uses
  `TerminalFormatterExtra::none()` and emits no keyboard/PWD bytes without
  explicit opt-in.
- VT output emits keyboard mode bytes only when the `keyboard` extra is enabled
  and `modify_other_keys_2` is true:

  ```text
  \x1b[>4;2m
  ```

- VT output emits no keyboard bytes when `modify_other_keys_2` is false.
- VT output emits PWD bytes only when the `pwd` extra is enabled and stored PWD
  is non-empty.
- PWD output matches upstream's raw stored-byte behavior for valid UTF-8 PWD
  values, including the stored trailing NUL byte before ST:

  ```text
  \x1b]7;file://host/home/user\0\x1b\
  ```

- PWD bytes are not escaped, sanitized, normalized, or URL-encoded in this
  formatter-only slice. Arbitrary non-UTF-8 PWD output remains deferred because
  Roastty's formatter API currently returns `String`.
- Plain and HTML output ignore both extras.
- When combined with palette, modes, screen extras, scrolling region, tabstops,
  keyboard, and PWD, VT ordering is
  `palette -> modes -> content -> screen extras -> scrolling region -> tabstops -> keyboard -> pwd`.

Pin-map behavior:

- Generated keyboard/PWD bytes are byte-indexed.
- Keyboard/PWD bytes are appended after screen formatter output and all earlier
  post-screen terminal suffixes.
- Appended keyboard/PWD bytes map to the last existing pin when prior output
  exists.
- If the formatter emits only keyboard/PWD bytes, they map to active-screen
  top-left.
- Tests cover content plus scrolling region, tabstops, keyboard, and PWD,
  proving the combined post-screen suffix remains byte-indexed and maps to the
  final pre-suffix content pin, including the PWD trailing NUL byte.

Deferred by design:

- VT parser/runtime mutation for `CSI > 4 ; 2 m`.
- OSC parser/runtime mutation for `OSC 7`.
- PTY integration.
- Public API and public ABI.
- App behavior, renderer behavior, clipboard behavior, and UI behavior.

Verification run:

```text
cargo fmt
cargo test -p roastty terminal_formatter
cargo test -p roastty modes
cargo test -p roastty tabstops
cargo test -p roastty screen_formatter
cargo test -p roastty styled_pin_map
cargo test -p roastty pin_map
cargo test -p roastty page_string
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Results:

- `terminal_formatter`: 67 passed.
- `modes`: 20 passed.
- `tabstops`: 18 passed.
- `screen_formatter`: 55 passed.
- `styled_pin_map`: 9 passed.
- `pin_map`: 65 passed.
- `page_string`: 12 passed.
- `terminal::page_list`: 524 passed.
- full `cargo test -p roastty`: 957 unit tests passed, ABI harness passed, doc
  tests passed.

Codex reviewed the completed result before commit.

Initial result review artifacts:

- Prompt: `logs/codex-review/20260601-003022-727228-prompt.md`
- Result: `logs/codex-review/20260601-003022-727228-last-message.md`

Codex found one real issue: the first implementation stored PWD as `Vec<u8>` but
converted it through UTF-8 before appending to the formatter `String`, while the
experiment language claimed raw arbitrary-byte behavior. The fix was to make the
current formatter invariant explicit:

- `TerminalPwd` stores valid UTF-8 text plus the trailing NUL in a `String`;
- formatter output appends that stored string directly;
- the experiment records that arbitrary non-UTF-8 PWD output remains deferred
  because the formatter API currently returns `String`.

After the fix, `cargo fmt`, `cargo test -p roastty terminal_formatter`, and full
`cargo test -p roastty` passed again.

Result re-review artifacts:

- Prompt: `logs/codex-review/20260601-003220-306266-prompt.md`
- Result: `logs/codex-review/20260601-003220-306266-last-message.md`

Codex found no remaining real findings and approved commit.

## Conclusion

Roastty's terminal formatter now covers all upstream `TerminalFormatter.Extra`
fields currently in scope for formatter-only state: palette, modes, scrolling
region, tabstops, keyboard, PWD, and forwarded screen extras. The remaining work
is no longer formatter serialization for these extras; it is parser/runtime
state mutation and the broader terminal/app subsystems that feed this state in
normal operation.
