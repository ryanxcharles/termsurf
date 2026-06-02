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

# Experiment 139: Port OSC 22 Mouse Shape

## Description

Experiment 138 completed the Kitty OSC 21 color slice. The next coherent OSC
runtime slice is OSC 22 mouse shape:

```text
OSC 22 ; <shape> ST
```

Ghostty parses OSC 22 in
`vendor/ghostty/src/terminal/osc/parsers/mouse_shape.zig`, converts the string
to `terminal.mouse.Shape` in `vendor/ghostty/src/terminal/stream.zig`, and then
updates `Terminal.mouse_shape` through either `termio/stream_handler.zig` or
`terminal/stream_terminal.zig`.

Roastty already has mouse mode state, but it does not yet model the terminal's
current mouse cursor shape. This experiment ports the terminal-owned part of OSC
22: parse the requested shape, map Ghostty's W3C and xterm/foot aliases to a
Rust `MouseShape`, and update terminal state. Surface/app delivery of
`set_mouse_shape` is intentionally deferred until Roastty has the app/surface
message boundary.

## Changes

1. Add a terminal mouse-shape model.

   Add a terminal-internal mouse module or equivalent local type with
   `MouseShape` variants matching Ghostty's
   `vendor/ghostty/src/terminal/mouse.zig::Shape`:
   - `Default`
   - `ContextMenu`
   - `Help`
   - `Pointer`
   - `Progress`
   - `Wait`
   - `Cell`
   - `Crosshair`
   - `Text`
   - `VerticalText`
   - `Alias`
   - `Copy`
   - `Move`
   - `NoDrop`
   - `NotAllowed`
   - `Grab`
   - `Grabbing`
   - `AllScroll`
   - `ColResize`
   - `RowResize`
   - `NResize`
   - `EResize`
   - `SResize`
   - `WResize`
   - `NeResize`
   - `NwResize`
   - `SeResize`
   - `SwResize`
   - `EwResize`
   - `NsResize`
   - `NeswResize`
   - `NwseResize`
   - `ZoomIn`
   - `ZoomOut`

   Add `MouseShape::parse(bytes: &[u8]) -> Option<MouseShape>` with Ghostty's
   exact string aliases:
   - W3C names such as `default`, `context-menu`, `pointer`, `no-drop`,
     `not-allowed`, `col-resize`, `zoom-in`, etc.;
   - xterm/foot aliases such as `left_ptr`, `question_arrow`, `hand`,
     `left_ptr_watch`, `watch`, `cross`, `xterm`, `dnd-link`, `dnd-copy`,
     `dnd-move`, `dnd-no-drop`, `crossed_circle`, `hand1`, `right_side`,
     `top_side`, `top_right_corner`, `top_left_corner`, `bottom_side`,
     `bottom_right_corner`, `bottom_left_corner`, `left_side`, and `fleur`.

   Matching is exact and case-sensitive. Unknown names return `None`.

2. Parse OSC 22 in `roastty/src/terminal/osc.rs`.

   Add `Command::MouseShape { value: &'a [u8] }` or a parsed
   `Command::MouseShape { shape: MouseShape }`.

   Prefer parsing to `MouseShape` before dispatch, matching Ghostty's effective
   stream behavior: an unknown shape is ignored and produces no terminal state
   mutation.

   Preserve existing OSC parser rules:
   - valid OSC 22 payloads are bounded by `MAX_BUF`;
   - payload bytes are not normalized;
   - unknown payloads are consumed but do not dispatch a mutating action;
   - OSC 1 icon parsing remains ignored unless this experiment deliberately adds
     a parse-only action for it.

3. Dispatch OSC 22 through the stream layer.

   Add the new OSC action to the stream test harness so `Stream` can prove that
   a valid OSC 22 sequence reaches the handler and an unknown shape does not.

4. Execute OSC 22 in `roastty/src/terminal/terminal.rs`.

   Add `mouse_shape: MouseShape` terminal state, defaulting to Ghostty's
   `Terminal.mouse_shape = .text`.

   On a valid OSC 22 shape, update the terminal field. Repeatedly setting the
   same shape is allowed to be a no-op. Do not add surface/app messages, public
   ABI, renderer behavior, pointer-hover behavior, or key-to-shape behavior in
   this experiment.

5. Keep scope limited.

   Do not implement mouse event encoding, surface cursor changes, app runtime
   messages, C ABI accessors, or macOS cursor integration. Those are separate
   app/surface/input experiments.

## Verification

Run formatting and tests:

```bash
cargo fmt
cargo test -p roastty mouse_shape
cargo test -p roastty osc
cargo test -p roastty terminal_stream_osc
cargo test -p roastty
```

Add tests for:

- every W3C shape string maps to the expected `MouseShape`;
- every xterm/foot alias maps to the expected `MouseShape`;
- parsing is exact and case-sensitive;
- unknown shape strings return `None`;
- OSC 22 valid payload dispatches a mouse-shape action through `Stream`;
- OSC 22 unknown payload is consumed without dispatching an action;
- terminal default mouse shape is `Text`;
- terminal OSC 22 updates `mouse_shape`;
- repeated OSC 22 with the same shape preserves state;
- invalid/unknown OSC 22 does not mutate state;
- existing OSC title, PWD, hyperlink, color, and Kitty color tests still pass.

## Pass Criteria

- Roastty has a terminal-owned `MouseShape` model matching Ghostty's shape list.
- W3C and xterm/foot alias parsing matches Ghostty.
- OSC 22 valid shapes update terminal mouse shape state.
- OSC 22 unknown shapes are ignored without mutating terminal state.
- Default terminal mouse shape is `Text`.
- No app/surface/renderer/public-ABI behavior is added.
- Existing OSC behavior keeps passing.

## Failure Criteria

- The parser accepts unknown or differently-cased shape names.
- OSC 22 mutates state for an unknown shape.
- Default terminal mouse shape differs from Ghostty's `.text`.
- The experiment adds surface cursor messages, public ABI, renderer behavior, or
  macOS cursor integration.
- Existing OSC title, PWD, hyperlink, color, or Kitty color behavior regresses.

## Design Review

Codex reviewed the design and found no blocking issues. The review confirmed
that the experiment follows Ghostty's OSC 22 flow at the current Roastty
boundary: exact shape parsing, unknown-shape ignore behavior, terminal-owned
state only, and deferred surface/app cursor delivery. The design was approved
for implementation.

## Result

**Result:** Pass

Roastty now models Ghostty's terminal mouse shape list and parses OSC 22 mouse
shape requests into terminal-owned state.

The implementation added `terminal::mouse::MouseShape` with Ghostty's W3C shape
names and xterm/foot aliases. Parsing is exact and case-sensitive. Unknown shape
names are consumed without dispatching a mutating action, matching Ghostty's
stream behavior.

OSC 22 is wired through the OSC parser, stream dispatcher, and terminal runtime.
`Terminal` now defaults `mouse_shape` to `Text`, matching Ghostty's
`Terminal.mouse_shape = .text`, and valid OSC 22 sequences update that field.
The experiment did not add public ABI, renderer behavior, app/surface cursor
messages, or macOS cursor integration.

Verification passed:

```text
cargo fmt
completed successfully

cargo test -p roastty mouse_shape
6 passed; 0 failed

cargo test -p roastty osc
62 passed; 0 failed

cargo test -p roastty terminal_stream_osc
24 passed; 0 failed

cargo test -p roastty
1525 unit tests passed; 0 failed
1 ABI harness test passed; 0 failed
```

## Result Review

Codex reviewed the completed implementation and result record and found no real
issues. The review confirmed exact and case-sensitive shape parsing, full W3C
and xterm/foot alias coverage, unknown-shape ignore behavior, terminal-owned
state defaulting to `Text`, and no app/surface/renderer/public-ABI drift. The
only commit hygiene note was to include the new `roastty/src/terminal/mouse.rs`
file in the result commit.

## Conclusion

Experiment 139 completes the terminal-owned OSC 22 mouse-shape slice. Roastty
can now preserve the same terminal state Ghostty would set from shell-provided
cursor-shape control sequences, while leaving delivery to the future app/surface
message boundary out of scope.

The next experiment can continue with the next self-contained Ghostty terminal
protocol slice.
