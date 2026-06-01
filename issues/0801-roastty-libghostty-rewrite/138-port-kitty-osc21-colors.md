# Experiment 138: Port Kitty OSC 21 Colors

## Description

Experiments 135-137 added the shared color parser, ANSI palette OSC operations,
and dynamic foreground/background/cursor OSC operations. Ghostty also supports
Kitty's OSC 21 color protocol:

```text
OSC 21 ; key=value ; key=? ; key= ST
```

This protocol overlaps with the state Roastty now has:

- 256-color palette entries;
- dynamic foreground color;
- dynamic background color; and
- dynamic cursor color.

This experiment ports the OSC 21 parser and terminal execution for those
supported keys. It should follow Ghostty's parser in
`vendor/ghostty/src/terminal/osc/parsers/kitty_color.zig`, Ghostty's key model
in `vendor/ghostty/src/terminal/kitty/color.zig`, and Ghostty's termio query
response behavior in `vendor/ghostty/src/termio/stream_handler.zig`.

Ghostty's terminal-only handler ignores OSC 21 queries, but Roastty already
models query-capable terminal behavior through `pty_response` for OSC 4 and OSC
10/11/12. For Roastty, implement the query-producing behavior from Ghostty's
termio `kittyColorReport`.

## Changes

1. Add Kitty color protocol types.

   In `roastty/src/terminal/kitty.rs`, add a compact color protocol model:
   - `ColorSpecial`
     - `Foreground`
     - `Background`
     - `SelectionForeground`
     - `SelectionBackground`
     - `Cursor`
     - `CursorText`
     - `VisualBell`
     - `SecondTransparentBackground`
   - `ColorKind`
     - `Palette(u8)`
     - `Special(ColorSpecial)`
   - `ColorRequest`
     - `Query(ColorKind)`
     - `Set { key: ColorKind, rgb: color::Rgb }`
     - `Reset(ColorKind)`
   - fixed-capacity `ColorRequests`, using Ghostty's exact cap expression:
     `(u8::MAX as usize + special_count) * 2`.

   Note that this is Ghostty's actual expression, not the intuitive
   `(256 + special_count) * 2` count. With eight special keys, the cap is
   `263 * 2`, matching `vendor/ghostty/src/terminal/kitty/color.zig::Kind.max`.

   Keep these terminal-internal unless an existing public API requires exposing
   them.

2. Parse OSC 21 in `roastty/src/terminal/osc.rs`.

   Add `Command::KittyColor { requests, terminator }`.

   Parser behavior must match Ghostty:
   - body is split on semicolons;
   - each item is split on the first `=`;
   - empty keys are skipped;
   - unknown keys are skipped;
   - special keys are exact and lowercase, matching Ghostty's
     `std.meta.stringToEnum`;
   - numeric keys parse as palette indexes `0..=255`;
   - value is trimmed with literal edge spaces only;
   - empty value means reset;
   - `?` means query;
   - otherwise parse value through `Rgb::parse`;
   - invalid color values skip only that item and parsing continues;
   - if the request count reaches the fixed cap, the OSC command is invalid and
     no command is dispatched.

3. Execute OSC 21 in `roastty/src/terminal/terminal.rs`.

   Process requests in order:
   - palette set/reset mutates `TerminalColors.palette`;
   - foreground/background/cursor set/reset mutates the corresponding
     `DynamicRgb`;
   - unsupported special set/reset requests (`selection_foreground`,
     `selection_background`, `cursor_text`, `visual_bell`,
     `second_transparent_background`) are parsed but inert: they do not mutate
     state and do not write key-specific response fields;
   - palette query reports current palette color;
   - foreground/background query reports current dynamic color if present;
   - cursor query reports current cursor dynamic color if present, with no
     foreground fallback for OSC 21;
   - a supported special query with no value writes an empty value, for example
     `;cursor=`;
   - unsupported special queries produce Ghostty's termio prefix side effect:
     the response buffer gets `ESC ] 21` before the unsupported key is skipped,
     so an OSC 21 containing only an unsupported query emits bare `ESC ] 21`
     plus the incoming terminator.

   Query responses must match Ghostty's termio format:

   ```text
   ESC ] 21 ; key=rgb:rr/gg/bb ; key= terminator
   ```

   The response should be emitted whenever the response buffer is nonempty,
   including the bare-prefix unsupported-query side effect. Use the same BEL/ST
   terminator as the incoming OSC.

4. Keep scope limited.

   Do not add new color state for selection colors, cursor text, visual bell, or
   transparent backgrounds in this experiment. Do not add renderer, config, ABI,
   or surface-message behavior.

## Verification

Run formatting and tests:

```bash
cargo fmt
cargo test -p roastty kitty
cargo test -p roastty osc
cargo test -p roastty terminal_stream_osc
cargo test -p roastty
```

Add parser tests for:

- Ghostty's mixed example:
  `foreground=?;background=rgb:f0/f8/ff;cursor=aliceblue;cursor_text;visual_bell=;selection_foreground=#xxxyyzz;selection_background=?;selection_background=#aabbcc;2=?;3=rgbi:1.0/1.0/1.0`
  should produce the same nine valid requests Ghostty expects.
- OSC 21 with no key (`21;`) produces an empty request list command.
- empty keys are skipped.
- unknown keys are skipped.
- invalid colors skip only that item.
- uppercase special keys reject.
- palette key `255` is accepted and palette key `256` is rejected.
- value trimming uses literal spaces only, so `foreground= ? ` queries but
  `foreground=\t?\t` is an invalid color item and is skipped.
- cap overflow invalidates the command.

Add terminal execution tests for:

- palette set/reset/query;
- foreground/background/cursor set/reset/query;
- cursor query with cursor unset writes `;cursor=` instead of falling back to
  foreground;
- query responses use one `ESC ] 21` prefix and the incoming BEL/ST terminator;
- mixed request order reflects mutations before later queries;
- unsupported special set/reset requests do not mutate state and do not write
  response fields;
- unsupported special queries still produce Ghostty's bare `ESC ] 21` plus
  terminator prefix side effect;
- OSC 4 and OSC 10/11/12 behavior remains unchanged.

## Pass Criteria

- OSC 21 parses Ghostty-compatible Kitty color requests.
- OSC 21 palette operations mutate/query/reset the palette correctly.
- OSC 21 foreground/background/cursor operations mutate/query/reset dynamic
  colors correctly.
- OSC 21 query responses use Ghostty's `rgb:rr/gg/bb` format and preserve the
  incoming OSC terminator.
- Cursor query behavior differs correctly from OSC 12: OSC 21 cursor query has
  no foreground fallback.
- Unsupported special keys remain parsed-but-inert, with no new terminal state.
- Unsupported special queries preserve Ghostty's bare-prefix response side
  effect.
- Existing OSC 4 and OSC 10/11/12 tests keep passing.

## Failure Criteria

- The implementation adds renderer, config, ABI, or surface-message behavior.
- OSC 21 special key parsing becomes case-insensitive.
- OSC 21 response colors use 16-bit `rrrr/gggg/bbbb` format instead of Kitty's
  byte-width `rr/gg/bb` format.
- OSC 21 cursor queries fall back to foreground.
- Unsupported special keys mutate state.
- Unsupported special queries produce no response, diverging from Ghostty's
  termio prefix side effect.
- Invalid color values abort the whole OSC 21 parse instead of skipping only the
  invalid item.
- The parser dispatches a command after exceeding the Kitty color request cap.
- OSC 4 or OSC 10/11/12 behavior regresses.

## Design Review

Codex reviewed the initial design and found two real issues:

- Ghostty's OSC 21 cap is its exact `Kind.max * 2` expression, which evaluates
  to `(255 + special_count) * 2`, not the intuitive `(256 + special_count) * 2`.
- Ghostty's termio query path writes the `ESC ] 21` prefix before discovering
  that a special query is unsupported, so unsupported-only special queries still
  emit a bare prefix plus terminator.

The design now pins both behaviors for Ghostty parity and requires verification
coverage for them.

Codex re-reviewed the revised design and found one remaining wording
contradiction: unsupported special set/reset requests are inert, but unsupported
special queries intentionally emit Ghostty's bare-prefix response side effect.
The design now distinguishes those cases and Codex approved the experiment for
implementation with no remaining blocking findings.

## Result

**Result:** Pass

Roastty now parses and executes Kitty OSC 21 color requests for the state this
terminal currently owns: ANSI palette entries, dynamic foreground, dynamic
background, and dynamic cursor color.

The implementation added a `terminal::kitty` color request model with Ghostty's
exact fixed request cap expression, wired OSC 21 parsing through the OSC parser
and stream dispatcher, and executed requests in terminal order. Palette
set/reset/query mutates and reports `TerminalColors.palette`. Foreground,
background, and cursor set/reset/query mutates and reports the corresponding
dynamic color. Cursor queries intentionally do not fall back to foreground,
matching Kitty OSC 21 behavior rather than OSC 12 behavior.

Unsupported special keys are parsed but remain inert for set/reset operations,
so this experiment did not add new selection, cursor-text, visual-bell, or
transparent-background state. Unsupported special queries preserve Ghostty's
termio side effect: a query-only unsupported request emits a bare `ESC ] 21`
prefix plus the incoming terminator.

Verification passed:

```text
cargo fmt
completed successfully

cargo test -p roastty kitty
29 passed; 0 failed

cargo test -p roastty osc
58 passed; 0 failed

cargo test -p roastty terminal_stream_osc
22 passed; 0 failed

cargo test -p roastty
1518 unit tests passed; 0 failed
1 ABI harness test passed; 0 failed
```

## Result Review

Codex reviewed the completed implementation and found one real parity issue:
Rust's `u8` parser accepts `+1`, while Ghostty's unsigned integer parser rejects
that palette key. The parser now requires palette keys to contain only ASCII
digits before parsing, and the OSC 21 edge-case test covers `+1=red` being
skipped. After that fix, `cargo fmt` and the full Experiment 138 verification
suite passed again.

## Conclusion

Experiment 138 completes the Kitty OSC 21 color protocol slice for Roastty's
current color state. It preserves Ghostty parser details, Ghostty's unusual
request cap, byte-width Kitty response formatting, incoming BEL/ST terminator
selection, and the unsupported-query bare-prefix quirk while leaving unsupported
special color state out of scope.

The next experiment can continue with the next Ghostty terminal subsystem after
OSC color handling, using the same plan-review and result-review gate.
