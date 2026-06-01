# Experiment 174: Port Selection Gesture C ABI

## Description

Port upstream terminal selection gestures into Roastty with Roastty naming.
Experiment 173 exposed the selection and grid-reference ABI foundation; the next
coherent slice is the state machine that turns pointer press, drag, release,
autoscroll, and deep-press events into terminal selections.

This experiment ports both the core gesture engine and its C ABI wrapper because
they form one subsystem in upstream Ghostty:

- `vendor/ghostty/src/terminal/SelectionGesture.zig`
- `vendor/ghostty/src/terminal/c/selection_gesture.zig`

The public API must use `roastty_*` names only. Upstream names may appear in
this experiment document as citations, but not in new public symbols, comments,
or implementation names except where citing source provenance in tests.

Roastty currently has a simpler screen model than upstream Ghostty: primary and
alternate screens exist, but there is no standalone `ScreenSet` type. This
experiment should add only the narrow screen identity needed by selection
gestures:

- a per-screen generation counter that changes when a screen is reset,
  destroyed, or recreated;
- a per-screen owner id that identifies the currently allocated screen object
  that owns tracked pins;
- an active-screen epoch that changes on every active-screen switch, including a
  switch away and then back to the same key.

A gesture anchor is valid only when the stored screen key, stored screen
generation, stored owner id, and stored active-screen epoch all still match the
terminal. This is stricter than key-only validation and prevents a stale
alternate-screen anchor from becoming valid again after switching away and back.

Anchor validity and anchor cleanup are intentionally different. Validity uses
key + generation + owner id + epoch. Cleanup uses key + owner id only: if the
screen object that owns the tracked pin still exists, untrack the pin even if a
reset changed the generation and made the anchor invalid. If the screen object
was destroyed and recreated, the owner id changes and cleanup must not untrack a
pointer through the new screen.

## Changes

1. Add a new internal gesture module:
   - create `roastty/src/terminal/selection_gesture.rs`;
   - expose it from `roastty/src/terminal/mod.rs` as `pub(crate)` or
     `pub(super)` as narrowly as the C ABI needs;
   - port upstream behavior with Rust names:
     - `SelectionGesture`
     - `SelectionGestureBehavior`
     - `SelectionGestureAutoscroll`
     - `SelectionGestureGeometry`
     - `SelectionGesturePress`
     - `SelectionGestureRelease`
     - `SelectionGestureDrag`
     - `SelectionGestureAutoscrollTick`
     - `SelectionGestureDeepPress`
   - preserve default behavior mapping:
     - single click: cell
     - double click: word
     - triple click: line
   - preserve autoscroll states:
     - none
     - up
     - down
   - preserve behavior states:
     - cell
     - word
     - line
     - output

2. Add the terminal/screen helpers needed by the gesture engine without
   broadening page-list visibility unnecessarily:
   - track and untrack one gesture anchor pin on the active screen;
   - record the anchor's screen key, screen generation, screen owner id, and
     active-screen epoch;
   - validate a tracked anchor against the current active screen using key,
     generation, owner id, and epoch;
   - untrack an anchor when the stored screen still exists and its owner id
     matches, even if generation no longer matches;
   - skip untracking when the screen was destroyed/recreated and owner id no
     longer matches;
   - resolve viewport coordinates after autoscroll;
   - scroll the active viewport by one row for autoscroll ticks;
   - forward to the selection helpers added in Experiment 173.

   If `PageList::track_pin` / `untrack_pin` must become more visible, keep them
   no wider than `pub(super)` and route gesture usage through `Screen` or
   `Terminal` methods where practical.

   Add the smallest possible identity surface to `TerminalScreens`:
   - `primary_generation: u64`;
   - `alternate_generation: u64`;
   - `primary_owner_id: u64`;
   - `alternate_owner_id: u64`;
   - `next_screen_owner_id: u64`;
   - `active_epoch: u64`;
   - helper methods to read the active key, active generation, active owner id,
     active epoch, and the generation/owner id for an arbitrary screen key;
   - increment `active_epoch` on every successful `switch_to`;
   - increment the relevant screen generation when a screen is reset, destroyed,
     or recreated.
   - allocate a new owner id only when a screen object is created or recreated;
     do not change owner id for a reset that keeps the same screen object alive.

   Do not introduce a full upstream-style `ScreenSet` abstraction in this
   experiment.

3. Port core gesture behavior:
   - `press`:
     - starts or continues the click sequence;
     - tracks the anchor pin;
     - applies repeat interval and repeat-distance checks;
     - increments click count to a maximum of three;
     - returns the selection implied by the selected behavior, or no value for
       first-click cell behavior;
   - `drag`:
     - returns no value when no valid active press exists;
     - updates dragged state;
     - updates autoscroll state based on y position and surface height;
     - produces cell, word, line, or output selection according to the active
       behavior;
     - preserves upstream's 60% cell threshold for cell drag selection;
   - `autoscroll_tick`:
     - scrolls exactly one row in the current autoscroll direction;
     - resolves the supplied viewport coordinate after scrolling;
     - continues the drag at that point;
     - resets the gesture if the anchor is no longer valid;
   - `deep_press`:
     - selects the word under the active anchor;
     - marks the gesture dragged;
     - clears the click sequence and untracks the anchor;
   - `release`:
     - stops autoscroll;
     - marks dragged when release is outside the original anchor or no release
       pin is supplied;
     - does not clear click count/time, preserving double/triple-click behavior;
   - `reset` and `drop`/`free`:
     - clear all gesture state;
     - untrack any live anchor exactly once.

4. Add public C ABI types to `roastty/include/roastty.h` and matching Rust
   `#[repr(C)]` types in `roastty/src/lib.rs`:
   - opaque handles:
     - `roastty_selection_gesture_t`
     - `roastty_selection_gesture_event_t`
   - enums with upstream discriminants:
     - `ROASTTY_SELECTION_GESTURE_EVENT_PRESS = 0`
     - `ROASTTY_SELECTION_GESTURE_EVENT_RELEASE = 1`
     - `ROASTTY_SELECTION_GESTURE_EVENT_DRAG = 2`
     - `ROASTTY_SELECTION_GESTURE_EVENT_AUTOSCROLL_TICK = 3`
     - `ROASTTY_SELECTION_GESTURE_EVENT_DEEP_PRESS = 4`
     - `ROASTTY_SELECTION_GESTURE_DATA_CLICK_COUNT = 0`
     - `ROASTTY_SELECTION_GESTURE_DATA_DRAGGED = 1`
     - `ROASTTY_SELECTION_GESTURE_DATA_AUTOSCROLL = 2`
     - `ROASTTY_SELECTION_GESTURE_DATA_BEHAVIOR = 3`
     - `ROASTTY_SELECTION_GESTURE_DATA_ANCHOR = 4`
     - `ROASTTY_SELECTION_GESTURE_EVENT_OPTION_REF = 0`
     - `ROASTTY_SELECTION_GESTURE_EVENT_OPTION_POSITION = 1`
     - `ROASTTY_SELECTION_GESTURE_EVENT_OPTION_REPEAT_DISTANCE = 2`
     - `ROASTTY_SELECTION_GESTURE_EVENT_OPTION_TIME_NS = 3`
     - `ROASTTY_SELECTION_GESTURE_EVENT_OPTION_REPEAT_INTERVAL_NS = 4`
     - `ROASTTY_SELECTION_GESTURE_EVENT_OPTION_WORD_BOUNDARY_CODEPOINTS = 5`
     - `ROASTTY_SELECTION_GESTURE_EVENT_OPTION_BEHAVIORS = 6`
     - `ROASTTY_SELECTION_GESTURE_EVENT_OPTION_RECTANGLE = 7`
     - `ROASTTY_SELECTION_GESTURE_EVENT_OPTION_GEOMETRY = 8`
     - `ROASTTY_SELECTION_GESTURE_EVENT_OPTION_VIEWPORT = 9`
   - value structs:
     - `roastty_selection_gesture_behaviors_s`
     - `roastty_selection_gesture_geometry_s`
     - `roastty_surface_position_s`
     - `roastty_codepoints_s`

   Define every `event_set` payload explicitly:

   | Option                     | Payload type                                   |
   | -------------------------- | ---------------------------------------------- |
   | `REF`                      | `const roastty_grid_ref_s*`                    |
   | `POSITION`                 | `const roastty_surface_position_s*`            |
   | `REPEAT_DISTANCE`          | `const double*`                                |
   | `TIME_NS`                  | `const uint64_t*`                              |
   | `REPEAT_INTERVAL_NS`       | `const uint64_t*`                              |
   | `WORD_BOUNDARY_CODEPOINTS` | `const roastty_codepoints_s*`                  |
   | `BEHAVIORS`                | `const roastty_selection_gesture_behaviors_s*` |
   | `RECTANGLE`                | `const bool*`                                  |
   | `GEOMETRY`                 | `const roastty_selection_gesture_geometry_s*`  |
   | `VIEWPORT`                 | `const roastty_point_coordinate_s*`            |

   `roastty_surface_position_s` must match upstream `GhosttySurfacePosition`:
   two `double` / `f64` fields named `x` and `y`. Do not reuse
   `roastty_mouse_position_s`, because that type uses `float` and has different
   precision/layout semantics.

   `roastty_codepoints_s` must match upstream `GhosttyCodepoints`: a borrowed
   `const uint32_t* ptr` and `size_t len`. `event_set` must copy/validate the
   codepoints into event-owned storage; it must never retain the borrowed
   pointer after `event_set` returns.

   Incoming enum arguments must be received as raw `int`/`c_int` values and
   converted through checked helpers. Do not model incoming C enums as Rust
   enums before validation.

5. Add the public C functions:

   ```c
   ROASTTY_API roastty_result_e roastty_selection_gesture_new(
       roastty_selection_gesture_t*);
   ROASTTY_API void roastty_selection_gesture_free(
       roastty_selection_gesture_t,
       roastty_terminal_t);
   ROASTTY_API void roastty_selection_gesture_reset(
       roastty_selection_gesture_t,
       roastty_terminal_t);
   ROASTTY_API roastty_result_e roastty_selection_gesture_get(
       roastty_selection_gesture_t,
       roastty_terminal_t,
       int data,
       void* out);
   ROASTTY_API roastty_result_e roastty_selection_gesture_get_multi(
       roastty_selection_gesture_t,
       roastty_terminal_t,
       size_t count,
       const int* keys,
       void** values,
       size_t* out_written);
   ROASTTY_API roastty_result_e roastty_selection_gesture_event_new(
       roastty_selection_gesture_event_t*,
       int event_type);
   ROASTTY_API void roastty_selection_gesture_event_free(
       roastty_selection_gesture_event_t);
   ROASTTY_API roastty_result_e roastty_selection_gesture_event_set(
       roastty_selection_gesture_event_t,
       int option,
       const void* value);
   ROASTTY_API roastty_result_e roastty_selection_gesture_handle_event(
       roastty_selection_gesture_t,
       roastty_terminal_t,
       roastty_selection_gesture_event_t,
       roastty_selection_s* out_selection);
   ```

   Use Roastty's existing default allocator conventions for owned handles; do
   not introduce the upstream allocator parameter in this experiment unless an
   implementation blocker proves it is necessary.

   C lifetime contract:
   - a gesture with no live anchor may be freed with a null terminal;
   - a gesture with a live anchor must be reset or freed with the same live
     terminal before that terminal is freed;
   - passing a stale non-null `roastty_terminal_t` after `roastty_terminal_free`
     is caller misuse and is not safe or supported;
   - `reset` with a null terminal clears wrapper state but cannot untrack a live
     anchor, so callers must not use null reset as the normal cleanup path for a
     live gesture.

   Because `free` is a `void` function, invalid cleanup cannot be reported as a
   result. The implementation should avoid dereferencing a null terminal, but it
   must not promise safety for stale non-null terminal handles.

6. Preserve upstream C event validation semantics:
   - press requires `REF` before `handle_event`;
   - drag requires both `REF` and `GEOMETRY`;
   - autoscroll tick requires both `VIEWPORT` and `GEOMETRY`;
   - deep press requires no pin option but returns no value without a valid
     active anchor;
   - null event option values clear/reset only the options that upstream allows;
   - invalid option/event/data/behavior enum values return
     `ROASTTY_INVALID_VALUE`;
   - word-boundary codepoint arrays preserve Experiment 173 semantics:
     - null + len 0 means default or cleared default depending on option;
     - non-null + len 0 means explicitly empty;
     - len > 0 requires non-null;
     - invalid Unicode scalar values are rejected.

7. Keep application/surface integration out of this experiment:
   - do not add Swift/macOS event plumbing;
   - do not add clipboard/copy-on-select behavior;
   - do not add app/surface handles;
   - do not add renderer selection painting;
   - do not add platform pressure-event detection.

## Verification

1. Run `cargo fmt` after Rust edits and accept its output.

2. Add Rust unit tests for the core gesture engine:
   - lifecycle defaults and reset/free untracking;
   - press single/double/triple click behavior;
   - repeat interval, repeat distance, and max-three click count;
   - cell drag threshold behavior including same-cell null selection and
     rectangle selection;
   - word drag extends by word boundaries;
   - line drag extends by line boundaries;
   - output drag uses semantic output selection when available;
   - release with same ref, different ref, and no ref;
   - deep press selects the anchor word and clears the gesture;
   - autoscroll state changes on drag near top/bottom edges;
   - autoscroll tick scrolls one row and continues selection;
   - active-screen changes invalidate the anchor without leaking tracked pins;
   - primary reset invalidates the anchor but still untracks the old primary
     tracked pin on cleanup;
   - alternate destroy/recreate invalidates the anchor and does not untrack the
     stale pointer through the new alternate screen.

3. Add Rust ABI tests in `roastty/src/lib.rs` for:
   - enum discriminants;
   - public struct sizes, alignments, and offsets;
   - new/free/reset null handling;
   - get/get_multi success and first-failure written counts;
   - event_new rejects invalid event types;
   - event_set validates all invalid event/option combinations;
   - event_set copies codepoint arrays before caller mutation;
   - event_set rejects invalid codepoint arrays and invalid behavior enum
     values;
   - handle_event press/drag/autoscroll/deep-press success and required-field
     failures;
   - freeing a never-anchored gesture with null terminal does not crash;
   - freeing or resetting an anchored gesture with the owning live terminal
     untracks the anchor;
   - the implementation does not attempt to support stale non-null terminal
     handles.

4. Add C harness coverage in `roastty/tests/abi_harness.c` for:
   - header compile/link coverage for every new exported function;
   - C-side `sizeof`, `_Alignof`, and `offsetof` checks for public gesture
     structs;
   - lifecycle new/free/reset;
   - get/get_multi defaults;
   - press event over `"abc"` with double-click word behavior returning
     selection `0..2`;
   - drag event from x=1 to x=3 returning selection `1..3`;
   - deep press over `"abcde"` returning selection `0..4`;
   - invalid event option and missing required field paths.

5. Run:

   ```bash
   cargo test -p roastty selection_gesture
   cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
   cargo test -p roastty terminal_selection_c_abi
   cargo test -p roastty terminal_grid_ref
   cargo test -p roastty terminal_stream
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c roastty/src/terminal/selection_gesture.rs; then exit 1; else exit 0; fi
   git diff --check
   ```

   The no-Ghostty check must produce no matches in the edited Roastty
   source/header/harness files. References to upstream Ghostty remain allowed in
   issue documents and vendored paths.

## Failure Criteria

- Public symbols expose `ghostty_*` names or compatibility aliases.
- Incoming C enum values are represented as Rust enums before validation.
- A gesture anchor can outlive the active screen that owns its tracked pin.
- A gesture anchor becomes valid again after switching away from and back to the
  same screen key.
- Cleanup refuses to untrack a still-owned tracked pin merely because generation
  invalidated the anchor.
- Cleanup untracks a stale pointer through a destroyed/recreated screen object.
- Reset/free leaks or double-untracks tracked anchor pins.
- The design or implementation claims stale non-null terminal handles are safe.
- Press/drag/autoscroll/deep-press behavior diverges from upstream without a
  documented Roastty-specific reason.
- Event setters accept invalid option/event combinations.
- Event setters do not define or validate the concrete payload type for every
  event option.
- Codepoint arrays are borrowed after `event_set` returns instead of copied into
  event-owned storage.
- The experiment expands into app/surface event plumbing, clipboard behavior,
  renderer selection painting, or platform pressure detection.

## Review

This design must be reviewed with the Codex review skill before implementation.
All real findings must be fixed, and the design must be re-reviewed until Codex
approves it.

After implementation and result recording, the completed result must also be
reviewed with Codex and approved before the result commit.
