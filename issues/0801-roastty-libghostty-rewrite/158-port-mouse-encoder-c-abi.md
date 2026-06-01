# Experiment 158: Port Mouse Encoder C ABI

## Description

Expose the mouse event and mouse encoder functionality from Experiments 156 and
157 through Roastty's public C ABI using Roastty naming.

Ghostty exposes this layer through:

- `vendor/ghostty/src/terminal/c/mouse_event.zig`
  - `GhosttyMouseEvent` allocation/free;
  - action, button, modifier, and position setters/getters.
- `vendor/ghostty/src/terminal/c/mouse_encode.zig`
  - `GhosttyMouseEncoder` allocation/free;
  - encoder options for tracking mode, format, geometry, button state, and
    last-cell tracking;
  - `encode()` with required-size reporting on insufficient output space;
  - `reset()` for last-cell deduplication state;
  - `setopt_from_terminal()` to copy flags from a terminal handle.
- `vendor/ghostty/src/terminal/c/main.zig`
  - exports the mouse event and encoder functions as part of the public library
    surface.

Roastty now has the internal pieces needed for most of this ABI:

- `roastty/src/terminal/mouse.rs` has internal mouse value types;
- `roastty/src/terminal/mouse_encode.rs` has the pure encoder and last-cell
  behavior;
- `roastty/src/terminal/terminal.rs` has runtime mouse event/format caches.

This experiment should port the public mouse event and encoder ABI that does not
require a public terminal handle. The upstream `setopt_from_terminal()` behavior
must be documented as intentionally deferred because current Roastty surfaces do
not yet own or expose a real `terminal::Terminal` through the C ABI. Do not fake
that behavior from `ModeState`, do not add a dummy terminal handle, and do not
wire app/surface mouse input.

## Changes

1. Add public Roastty mouse ABI types in `roastty/include/roastty.h`.
   - Add opaque handles:
     - `roastty_mouse_event_t`;
     - `roastty_mouse_encoder_t`.
   - Replace the current anonymous single-value success enum with one public
     `roastty_result_e` definition:
     - `ROASTTY_SUCCESS = 0` must remain stable;
     - add explicit non-success values for out-of-memory, invalid value, and
       out-of-space.
     - do not create a second enum that duplicates the `ROASTTY_SUCCESS`
       enumerator.
   - Add C enums with Roastty names:
     - `roastty_mouse_action_e` for press/release/motion;
     - `roastty_mouse_button_e` for unknown, left, right, middle, four through
       eleven;
     - `roastty_mouse_tracking_mode_e` for none, X10, normal, button, any;
     - `roastty_mouse_format_e` for X10, UTF-8, SGR, URXVT, SGR-pixels;
     - `roastty_mouse_encoder_option_e` for event, format, size,
       any-button-pressed, and track-last-cell.
   - Add C structs:
     - `roastty_mouse_mods_s` with shift/alt/ctrl booleans;
     - `roastty_mouse_position_s` with `float x` / `float y`;
     - `roastty_mouse_encoder_size_s` with a leading `size_t size` field plus
       screen, cell, and padding dimensions.
   - Do not add any `ghostty_*` symbols.

2. Add Rust ABI handle storage in `roastty/src/lib.rs`.
   - Add `MouseEvent` wrapping `terminal::mouse_encode::Event`.
   - Add `MouseEncoder` wrapping `terminal::mouse_encode::Options`, a
     `track_last_cell` flag, and the optional last-cell state.
   - Keep ownership single and explicit:
     - `*_new` allocates with `Box`;
     - `*_free` accepts null and frees exactly once for valid handles;
     - setters/getters null-check handles at the ABI boundary.

3. Add mouse event functions.
   - `roastty_mouse_event_new(out)` returns a result code and writes a handle.
   - `roastty_mouse_event_free(event)` accepts null.
   - `roastty_mouse_event_set_action/get_action`.
   - `roastty_mouse_event_set_button`, `clear_button`, and `get_button`.
   - `roastty_mouse_event_set_mods/get_mods`.
   - `roastty_mouse_event_set_position/get_position`.
   - ABI safety requirement: exported Rust functions that receive C enum-like
     values must take raw integer types (`c_int` or the exact existing C scalar)
     at the FFI boundary, validate the integer, then convert to internal Rust
     enums. Do not expose Rust `repr(C)` enum parameters directly on exported
     functions where invalid C values could be passed before validation runs.
   - Invalid action or button values from C must return an invalid-value result
     or leave state unchanged according to the function's return convention.

4. Add mouse encoder functions.
   - `roastty_mouse_encoder_new(out)` returns a result code and writes a handle.
   - `roastty_mouse_encoder_free(encoder)` accepts null.
   - `roastty_mouse_encoder_setopt(encoder, option, value)` updates one option
     from a typed pointer.
   - `roastty_mouse_encoder_reset(encoder)` clears last-cell state.
   - `roastty_mouse_encoder_encode(encoder, event, out, out_len, out_written)`:
     - returns invalid-value for null encoder, event, or `out_written`;
     - supports `out == null && out_len == 0` as a required-size query;
     - returns out-of-space and writes the required byte count when the buffer
       is too small;
     - does not mutate last-cell dedupe state on out-of-space/required-size
       queries;
     - writes zero bytes with success when the pure encoder reports no output.
   - `roastty_mouse_encoder_setopt` must take the option as a raw integer, then
     validate it before dispatching to typed option handling.
   - Tracking mode and format option payloads are C enum-like values too. Read
     them as their C storage type, validate the integer value, then convert to
     internal Rust enums.

5. Port size and option behavior carefully.
   - Default encoder size should match upstream's safe default: 1x1 screen and
     1x1 cell with zero padding.
   - Reject or ignore size option values where the leading `size` field is
     smaller than the struct this experiment defines.
   - Reject zero cell width or height instead of panicking or dividing by zero.
   - Reset last-cell state when event mode, format, or size changes.
   - Changing `track_last_cell` to false clears last-cell state.

6. Explicitly defer `setopt_from_terminal()`.
   - Do not add `roastty_mouse_encoder_setopt_from_terminal` in this experiment
     unless a real public Roastty terminal/surface terminal handle already
     exists at implementation time.
   - Record in the result that the upstream function remains pending on the
     surface/terminal lifecycle work.
   - Do not synthesize this behavior from mode bits or a placeholder terminal.

7. Add ABI tests.
   - Extend `roastty/tests/abi_harness.rs` or add a focused integration test
     that compiles and links C code against `roastty/include/roastty.h`.
   - The C test should exercise:
     - event allocation/free/null-free;
     - event action/button/mods/position set/get;
     - encoder allocation/free/null-free;
     - setopt for event, format, size, any-button-pressed, and track-last-cell;
     - SGR left press encoding, including required-size query;
     - motion dedupe and reset;
     - required-size query not mutating dedupe state;
     - invalid/null handle behavior where observable through result codes.

8. Keep scope boundaries hard.
   - Do not wire live mouse input from macOS, Swift, app runtime, renderer, PTY,
     browser overlay, or TermSurf protocol paths.
   - Do not add terminal/surface ownership of `terminal::Terminal`.
   - Do not add `setopt_from_terminal()` without a real terminal handle.
   - Do not change the pure encoder behavior except to expose internal types
     safely where required by the ABI module.
   - Do not add non-macOS platform paths.

9. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix every real finding and re-review until Codex finds no remaining
     blocking design issues.
   - Record the design-review outcome in this experiment file before committing
     the design.
   - After implementation and verification, get Codex review of the completed
     result before committing the result.
   - Do not proceed to the next experiment until the completed result review is
     approved or every real result finding has been fixed and re-reviewed.

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/mouse.rs roastty/src/terminal/mouse_encode.rs
cargo test -p roastty mouse
cargo test -p roastty --test abi_harness
cargo test -p roastty
```

Required test coverage:

- Header/ABI compile checks:
  - `roastty/include/roastty.h` compiles as C through the ABI harness;
  - all new public names use `roastty` / `Roastty`, not `ghostty`;
  - enum and struct values used by the C test match Rust conversions.
  - the header has one public `roastty_result_e` definition and no duplicate
    `ROASTTY_SUCCESS` enumerator.
- Mouse event ABI:
  - new/free works;
  - freeing null is safe;
  - action set/get round trips press, release, and motion;
  - button set/get/clear round trips left/middle/right and at least one extended
    button;
  - mods set/get preserves shift/alt/ctrl;
  - position set/get preserves representative positive and negative float
    values;
  - invalid action and button values return invalid-value or leave state
    unchanged without undefined behavior.
- Mouse encoder ABI:
  - new/free works;
  - freeing null is safe;
  - setopt updates event, format, size, any-button-pressed, and track-last-cell;
  - invalid option, invalid tracking mode, invalid format, and invalid value
    pointers are handled without panic or undefined behavior;
  - zero cell width/height is rejected or ignored safely;
  - SGR left press at `(0, 0)` encodes `ESC [ < 0 ; 1 ; 1 M`;
  - `out == null && out_len == 0` reports the required size without mutating
    dedupe state;
  - too-small buffers return out-of-space, write required size, and do not
    mutate dedupe state;
  - motion dedupe suppresses same-cell motion when tracking is enabled and
    `reset()` clears the dedupe state.
- Regression checks:
  - pure mouse encoder tests still pass;
  - terminal mouse runtime state tests from Experiment 157 still pass;
  - full Roastty suite and ABI harness pass.
- Review gates:
  - Codex design review approves the experiment before implementation, or every
    real design finding is fixed and re-reviewed cleanly;
  - Codex result review approves the completed experiment before result commit,
    or every real result finding is fixed and re-reviewed cleanly.

## Non-Negotiable Invariants

- Use Roastty public names only; no `ghostty_*` compatibility ABI.
- Add public ABI only for mouse event/encoder behavior that can be backed by the
  existing internal implementation.
- Do not fake `setopt_from_terminal()`.
- Do not wire live input or PTY writes.
- Do not mutate pure encoder semantics while adding ABI wrappers.
- Keep C ABI pointer ownership explicit and null-safe.
- Run `cargo fmt` and accept its output.
- Pass Codex design and result reviews before moving to the next stage.

## Failure Criteria

This experiment fails if:

- public ABI uses `ghostty_*` names;
- the header and Rust ABI disagree on enum values, struct layout, or function
  signatures;
- invalid enum values or null pointers cause undefined behavior or panics across
  the ABI boundary;
- exported Rust functions accept C enum-like values as Rust enum parameters
  instead of raw validated integers;
- result codes are duplicated or split across incompatible public enums;
- out-of-space / required-size queries mutate last-cell dedupe state;
- buffer length reporting differs from the encoded byte count;
- zero cell dimensions can panic or divide by zero;
- `setopt_from_terminal()` is faked without a real terminal handle;
- live mouse input, Swift, renderer, PTY, browser overlay, TermSurf protocol, or
  non-macOS platform behavior is added;
- pure mouse encoder, terminal mouse runtime, or existing ABI harness tests
  regress;
- the design or result proceeds without the required Codex review gate.

## Codex Design Review

Codex reviewed the initial design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-134231-399347-prompt.md`
- Result: `logs/codex-review/20260601-134231-399347-last-message.md`

Codex found two real design issues:

- C enum-like values needed to be accepted as raw integers at the Rust FFI
  boundary before validation, instead of as Rust enum parameters;
- the result-code migration needed to explicitly replace the current anonymous
  success enum with one public `roastty_result_e`, rather than adding duplicate
  result definitions.

Both findings were fixed.

Clean design re-review artifacts:

- Prompt: `logs/codex-review/20260601-134514-169988-prompt.md`
- Result: `logs/codex-review/20260601-134514-169988-last-message.md`

Codex found no remaining blockers and approved the experiment for
implementation.

## Result

**Result:** Pass

Experiment 158 exposes the mouse event and mouse encoder through the public
Roastty C ABI.

Implemented:

- `roastty/include/roastty.h` now defines opaque `roastty_mouse_event_t` /
  `roastty_mouse_encoder_t` handles, one public `roastty_result_e`, C enum
  values for mouse actions, buttons, tracking modes, formats, and encoder
  options, plus ABI structs for modifiers, position, and encoder size.
- `roastty/src/lib.rs` now owns the event and encoder handles with explicit
  `Box` allocation/free, validates all C enum-like values as raw integers at the
  FFI boundary, rejects invalid/null inputs with result codes, and exposes
  required-size / out-of-space reporting for encoded mouse bytes.
- `roastty/src/terminal/{mouse.rs,mouse_encode.rs,point.rs,mod.rs}` exposes the
  already-ported internal mouse types to the ABI module without changing the
  pure encoder semantics.
- `roastty/tests/abi_harness.c` now exercises the new C header/API surface:
  event allocation/free/null-free, action/button/modifier/position setters and
  getters, encoder options, SGR left-press encoding, required-size queries,
  too-small buffer handling, motion dedupe, reset, and invalid value handling.

The upstream `setopt_from_terminal()` behavior remains intentionally deferred.
Roastty still does not expose a real public terminal/surface terminal handle
through the C ABI, so this experiment did not fake the behavior from mode bits
or a placeholder handle.

Verification:

- `cargo fmt -- roastty/src/lib.rs roastty/src/terminal/mod.rs roastty/src/terminal/point.rs roastty/src/terminal/mouse.rs roastty/src/terminal/mouse_encode.rs`
- `cargo test -p roastty mouse` — 43 passed
- `cargo test -p roastty --test abi_harness` — 1 passed
- `cargo test -p roastty` — 1744 unit tests passed, ABI harness passed, doc
  tests passed

## Codex Result Review

Codex reviewed the completed implementation and recorded result before commit.

Initial result-review artifacts:

- Prompt: `logs/codex-review/20260601-135347-412672-prompt.md`
- Result: `logs/codex-review/20260601-135347-412672-last-message.md`

Codex found one real result issue:

- `roastty_mouse_encoder_setopt(..., ROASTTY_MOUSE_ENCODER_OPTION_SIZE, ...)`
  read the full `roastty_mouse_encoder_size_s` before validating the leading
  `size` field, so a caller with an actually smaller versioned struct could be
  read past before rejection.

The issue was fixed by reading only the leading `size_t` first, rejecting
undersized payloads before reading the full struct, and adding a C ABI harness
case that passes a genuinely smaller one-field struct.

Clean result re-review artifacts:

- Prompt: `logs/codex-review/20260601-135540-444027-prompt.md`
- Result: `logs/codex-review/20260601-135540-444027-last-message.md`

Codex found no remaining blockers and approved the result for commit.

## Conclusion

The public C ABI now covers the mouse event and standalone mouse encoder layer
that can be backed by Roastty's current internal implementation. The remaining
upstream mouse C ABI gap is `setopt_from_terminal()`, which should wait until a
future experiment introduces a real public terminal/surface terminal handle.
