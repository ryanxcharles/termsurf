# Experiment 156: Port Mouse Event Encoding

## Description

Port Ghostty's terminal mouse event encoder into Roastty as a pure, testable
terminal subsystem.

Roastty already has the terminal mode table entries that enable mouse tracking
and mouse formats:

- `?9` X10 tracking;
- `?1000` normal tracking;
- `?1002` button-motion tracking;
- `?1003` any-motion tracking;
- `?1005` UTF-8 mouse format;
- `?1006` SGR mouse format;
- `?1015` URXVT mouse format;
- `?1016` SGR-pixels mouse format.

Roastty also already has `roastty/src/terminal/mouse.rs` for OSC 22 cursor shape
parsing. What is missing is the encoder that takes a normalized mouse event plus
the terminal's current mouse tracking/format state and produces the VT bytes
sent to the PTY.

Upstream Ghostty source references:

- `vendor/ghostty/src/input/mouse.zig`
  - defines mouse action, button, momentum, pressure, and related app-input
    types.
- `vendor/ghostty/src/input/mouse_encode.zig`
  - defines `Options`, normalized `Event`, tracking-mode filtering, button-code
    mapping, coordinate conversion, last-cell deduplication, and encoders for
    X10, UTF-8, SGR, URXVT, and SGR-pixels.
- `vendor/ghostty/src/terminal/mouse.zig`
  - defines terminal mouse tracking modes and mouse output formats.
- `vendor/ghostty/src/terminal/c/mouse_encode.zig`
  - wraps the encoder for the C ABI, but this experiment must not port the C ABI
    wrapper yet.

This experiment should add the pure encoder and its tests only. It must not wire
macOS input, public ABI, Swift app integration, renderer event dispatch, PTY
process behavior, browser overlays, or TermSurf protocol behavior.

## Changes

1. Extend `roastty/src/terminal/mouse.rs` with mouse protocol value types.
   - Add `MouseEventMode` or equivalent for `none`, `x10`, `normal`, `button`,
     and `any`.
   - Add `MouseFormat` or equivalent for `x10`, `utf8`, `sgr`, `urxvt`, and
     `sgr_pixels`.
   - Add normalized `MouseAction` (`press`, `release`, `motion`) and
     `MouseButton` values matching upstream's terminal-supported button set:
     left, middle, right, four, five, six, seven, eight, nine, ten, eleven, plus
     unknown if useful for tests.
   - Add a small modifier struct for shift/alt/ctrl.
   - Keep existing `MouseShape` behavior intact.

2. Add a pure mouse encoder module.
   - Prefer `roastty/src/terminal/mouse_encode.rs` unless existing local style
     points to a better location.
   - Add it to `roastty/src/terminal/mod.rs`.
   - Define an `Options` type containing:
     - mouse event mode;
     - mouse format;
     - terminal geometry needed for coordinate conversion;
     - `any_button_pressed`;
     - optional last-cell state for motion deduplication.
   - Define an `Event` type containing action, optional button, modifiers, and a
     surface-space pixel position.
   - Define a compact geometry type instead of porting Ghostty's full renderer
     size module. It must still model the fields Ghostty's encoder uses via
     `vendor/ghostty/src/renderer/size.zig`: screen pixel width/height, cell
     pixel width/height, terminal-space origin/padding, clamped grid conversion,
     and unclamped rounded terminal-space pixel conversion for SGR-pixels. Do
     not implement SGR-pixels as raw surface pixels unless the geometry case
     proves that raw surface pixels equal terminal-space pixels.

3. Port Ghostty's filtering behavior.
   - `none` reports nothing.
   - `x10` reports only left/middle/right press events.
   - `normal` reports press and release, not motion.
   - `button` reports motion only when a button is associated with the event.
   - `any` reports all actions.
   - Outside-viewport non-release events are reported only when the tracking
     mode sends motion and `any_button_pressed` is true.
   - Release events are reported even outside the viewport.
   - Motion events are deduplicated by last reported cell for every format
     except SGR-pixels.

4. Port Ghostty's button-code behavior.
   - left/middle/right map to `0`, `1`, `2`.
   - four/five/six/seven map to `64..67`.
   - eight/nine map to `128..129`.
   - ten/eleven and unknown are unsupported and produce no output.
   - Legacy release formats encode button `3`; SGR and SGR-pixels release keep
     the actual button identity.
   - Add shift/alt/ctrl modifiers as `4`, `8`, and `16`, except X10 tracking
     mode ignores modifiers. This exception is based on `opts.event == .x10`,
     not on the output format alone.
   - Motion adds `32`.

5. Port Ghostty's output formats.
   - X10: `ESC [ M Cb Cx Cy`, with one-indexed cells encoded as byte values
     offset by 32. Reject coordinates above the X10 223-cell limit.
   - UTF-8: `ESC [ M Cb Cx Cy`, with Cx/Cy encoded as UTF-8 codepoints
     `cell + 33`.
   - SGR: `ESC [ < Cb ; x ; y M/m`, one-indexed cell coordinates.
   - URXVT: `ESC [ Cb ; x ; y M`, with `Cb` offset by 32.
   - SGR-pixels: `ESC [ < Cb ; x ; y M/m`, terminal-space pixel coordinates.

6. Keep scope boundaries hard.
   - Do not expose any new `roastty_*` C ABI functions.
   - Do not wire the encoder into a terminal app, Swift frontend, renderer,
     macOS event loop, or PTY process write path.
   - Do not add TermSurf protocol or browser overlay behavior.
   - Do not add Linux or other non-macOS platform paths.
   - Do not rename existing `MouseShape` behavior or disturb OSC 22.

## Verification

Run:

```bash
cargo fmt
cargo test -p roastty mouse
cargo test -p roastty modes
cargo test -p roastty
```

Required test coverage:

- Existing mouse-shape parser tests still pass.
- Tracking-mode filtering:
  - none reports nothing;
  - x10 reports only left/middle/right press;
  - normal reports press/release but not motion;
  - button mode requires a button;
  - any mode reports buttonless motion.
- Button-code mapping:
  - left/middle/right;
  - wheel buttons four through seven;
  - extended buttons eight and nine;
  - ten/eleven/unknown produce no output;
  - X10 tracking mode ignores modifiers;
  - non-X10 tracking mode adds shift/alt/ctrl even when the output format is
    X10;
  - motion adds `32`.
- Output format examples matching upstream tests:
  - X10 left press at `(0, 0)` emits `ESC [ M 32 33 33`;
  - X10 release emits nothing;
  - SGR right release keeps button identity and uses trailing `m`;
  - SGR buttonless motion emits button code `35`;
  - URXVT with all modifiers emits the expected `32 + button_code` prefix;
  - UTF-8 format encodes large coordinates as UTF-8 codepoints;
  - X10 rejects coordinates above its limit;
  - SGR-pixels uses terminal-space pixel coordinates;
  - viewport boundary positions clamp to the final visible cell;
  - outside-viewport motion without a pressed button is ignored;
  - outside-viewport motion with a pressed button is reported;
  - outside-viewport release is reported;
  - same-cell motion is deduplicated except for SGR-pixels.
- Regression checks:
  - mouse mode table tests still pass;
  - OSC 22 mouse shape runtime tests still pass;
  - no public ABI, app integration, renderer, PTY process, browser overlay,
    protocol, or platform-input behavior changes.

## Non-Negotiable Invariants

- Port the pure encoder only.
- Do not wire live macOS mouse events to the encoder in this experiment.
- Do not add C ABI wrappers in this experiment.
- Do not add renderer, Swift, app runtime, PTY process, browser overlay, or
  TermSurf protocol behavior.
- Do not add Linux or other non-macOS paths.
- Do not use `ghostty_*` names except when citing upstream Ghostty source paths
  or behavior.
- Run `cargo fmt` and accept its output.

## Failure Criteria

This experiment fails if:

- encoded mouse output diverges from the upstream `mouse_encode.zig` cases named
  in verification;
- tracking-mode filtering reports events Ghostty would suppress or suppresses
  events Ghostty would report;
- button code calculation diverges for releases, modifiers, motion, wheel, or
  extended buttons;
- coordinate conversion is off by one for cell formats or pixel formats;
- motion deduplication applies to SGR-pixels or fails for cell-based formats;
- existing mouse-shape or mouse-mode tests regress;
- the patch adds app/input wiring, public ABI, renderer behavior, PTY process
  behavior, browser overlay behavior, TermSurf protocol behavior, Kitty
  graphics, or non-macOS platform paths.

## Result

**Result:** Pass

Implemented Ghostty-style terminal mouse event encoding as a pure Roastty
terminal subsystem.

Code changes:

- `roastty/src/terminal/mouse.rs`
  - added internal protocol value types for mouse tracking mode, mouse output
    format, mouse action, mouse button, and shift/alt/ctrl modifiers;
  - kept existing OSC 22 `MouseShape` parsing unchanged.
- `roastty/src/terminal/mouse_encode.rs`
  - added a pure `encode(Event, Options) -> Option<Vec<u8>>` encoder;
  - ported Ghostty's tracking-mode filtering behavior;
  - ported button-code calculation, including legacy release behavior, modifier
    bits, motion bit, wheel buttons, extended buttons, and unsupported button
    suppression;
  - ported X10, UTF-8, SGR, URXVT, and SGR-pixels output formats;
  - added compact geometry conversion matching Ghostty's required renderer-size
    behavior for clamped grid cells and unclamped rounded terminal-space
    SGR-pixels coordinates;
  - added last-cell motion deduplication for cell-based formats while leaving
    SGR-pixels undeduplicated.
- `roastty/src/terminal/mod.rs`
  - registered the new internal `mouse_encode` module.

No C ABI, app input wiring, Swift frontend, renderer event dispatch, PTY process
write path, browser overlay, TermSurf protocol, Kitty graphics, or non-macOS
platform path was added.

Verification:

```bash
cargo fmt
cargo test -p roastty mouse
cargo test -p roastty modes
cargo test -p roastty
```

All commands passed. The final full suite reported 1735 unit tests passing, the
ABI harness passing, and 0 doc tests.

The new test coverage includes:

- existing mouse-shape parser behavior;
- all tracking-mode filtering cases named in the experiment;
- X10 tracking mode modifier suppression and non-X10 tracking mode modifier
  inclusion even with X10 output format;
- left/middle/right, wheel, extended, unsupported, release, modifier, and motion
  button-code behavior;
- X10, UTF-8, SGR, URXVT, and SGR-pixels output examples;
- cell coordinate clamping, padding-aware conversion, terminal-space pixel
  conversion, outside-viewport motion/release behavior, and motion
  deduplication.

## Conclusion

Roastty now has the pure terminal mouse encoder needed to turn normalized mouse
events into VT protocol bytes. This closes the encoder-only slice while keeping
all live input/app/ABI wiring deferred.

A later experiment should connect terminal mode state to encoder options and,
after the app/input layer exists, wire actual macOS mouse events through this
encoder to the PTY.
