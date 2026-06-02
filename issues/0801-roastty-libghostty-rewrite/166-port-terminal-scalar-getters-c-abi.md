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

# Experiment 166: Port Terminal Scalar Getters C ABI

## Description

Experiment 165 introduced `roastty_terminal_t` and `roastty_terminal_vt_write`,
plus narrow direct inspection helpers. The next upstream terminal C ABI boundary
is `terminal_get` / `terminal_get_multi`.

This experiment ports the first `roastty_terminal_get` slice for scalar terminal
metadata that Roastty can already answer without exposing borrowed strings,
render-state internals, selection state, formatter handles, Kitty graphics, or
color structs. The goal is to preserve upstream `TerminalData` numeric slots
while returning `ROASTTY_NO_VALUE` for deferred fields whose supporting ABI is
not ready yet.

String fields (`title`, `pwd`) are intentionally not included in this getter
slice even though the direct copied-string helpers exist. Upstream returns
borrowed `lib.String` for those fields; Roastty's public `roastty_string_s`
currently represents ABI-owned strings that callers free. Mixing borrowed and
owned strings under one struct would create a lifetime trap. Title/PWD should
continue using the direct copied-string helpers from Experiment 165 until a
separate experiment designs the full string/getter ownership model.

## Changes

### 1. Add `ROASTTY_NO_VALUE`

In `roastty/include/roastty.h` and `roastty/src/lib.rs`, add:

```c
ROASTTY_NO_VALUE = 4,
```

Roastty's existing result enum uses positive values, unlike upstream Ghostty's
negative `Result` values. Keep the existing Roastty convention and add
`ROASTTY_NO_VALUE` as the next positive value. Do not renumber existing result
codes.

### 2. Add terminal data enums with upstream slots

In `roastty/include/roastty.h`, add:

```c
typedef enum {
  ROASTTY_TERMINAL_DATA_INVALID = 0,
  ROASTTY_TERMINAL_DATA_COLS = 1,
  ROASTTY_TERMINAL_DATA_ROWS = 2,
  ROASTTY_TERMINAL_DATA_CURSOR_X = 3,
  ROASTTY_TERMINAL_DATA_CURSOR_Y = 4,
  ROASTTY_TERMINAL_DATA_CURSOR_PENDING_WRAP = 5,
  ROASTTY_TERMINAL_DATA_ACTIVE_SCREEN = 6,
  ROASTTY_TERMINAL_DATA_CURSOR_VISIBLE = 7,
  ROASTTY_TERMINAL_DATA_KITTY_KEYBOARD_FLAGS = 8,
  ROASTTY_TERMINAL_DATA_SCROLLBAR = 9,
  ROASTTY_TERMINAL_DATA_CURSOR_STYLE = 10,
  ROASTTY_TERMINAL_DATA_MOUSE_TRACKING = 11,
  ROASTTY_TERMINAL_DATA_TITLE = 12,
  ROASTTY_TERMINAL_DATA_PWD = 13,
  ROASTTY_TERMINAL_DATA_TOTAL_ROWS = 14,
  ROASTTY_TERMINAL_DATA_SCROLLBACK_ROWS = 15,
  ROASTTY_TERMINAL_DATA_WIDTH_PX = 16,
  ROASTTY_TERMINAL_DATA_HEIGHT_PX = 17,
  ROASTTY_TERMINAL_DATA_COLOR_FOREGROUND = 18,
  ROASTTY_TERMINAL_DATA_COLOR_BACKGROUND = 19,
  ROASTTY_TERMINAL_DATA_COLOR_CURSOR = 20,
  ROASTTY_TERMINAL_DATA_COLOR_PALETTE = 21,
  ROASTTY_TERMINAL_DATA_COLOR_FOREGROUND_DEFAULT = 22,
  ROASTTY_TERMINAL_DATA_COLOR_BACKGROUND_DEFAULT = 23,
  ROASTTY_TERMINAL_DATA_COLOR_CURSOR_DEFAULT = 24,
  ROASTTY_TERMINAL_DATA_COLOR_PALETTE_DEFAULT = 25,
  ROASTTY_TERMINAL_DATA_KITTY_IMAGE_STORAGE_LIMIT = 26,
  ROASTTY_TERMINAL_DATA_KITTY_IMAGE_MEDIUM_FILE = 27,
  ROASTTY_TERMINAL_DATA_KITTY_IMAGE_MEDIUM_TEMP_FILE = 28,
  ROASTTY_TERMINAL_DATA_KITTY_IMAGE_MEDIUM_SHARED_MEM = 29,
  ROASTTY_TERMINAL_DATA_KITTY_GRAPHICS = 30,
  ROASTTY_TERMINAL_DATA_SELECTION = 31,
  ROASTTY_TERMINAL_DATA_VIEWPORT_ACTIVE = 32,
} roastty_terminal_data_e;
```

Also add:

```c
typedef enum {
  ROASTTY_TERMINAL_SCREEN_PRIMARY = 0,
  ROASTTY_TERMINAL_SCREEN_ALTERNATE = 1,
} roastty_terminal_screen_e;
```

The numeric values must match upstream Ghostty's current `TerminalData` and
`TerminalScreen` slots. Values that this experiment does not implement should
remain declared so future experiments can fill them without renumbering.

### 3. Add `roastty_terminal_get`

Add:

```c
ROASTTY_API roastty_result_e roastty_terminal_get(roastty_terminal_t,
                                                  roastty_terminal_data_e,
                                                  void* out);
```

Rust implementation note: even though the C header uses
`roastty_terminal_data_e`, the exported Rust function must treat the selector as
a raw integer at the FFI boundary, validate it, and only then convert to an
internal enum. Out-of-range values such as `-1` and `33` return
`ROASTTY_INVALID_VALUE` without dereferencing `out`.

Implemented fields in this experiment:

- `COLS`: output type `uint16_t*`;
- `ROWS`: output type `uint16_t*`;
- `CURSOR_X`: output type `uint16_t*`;
- `CURSOR_Y`: output type `uint16_t*`;
- `CURSOR_PENDING_WRAP`: output type `bool*`;
- `ACTIVE_SCREEN`: output type `roastty_terminal_screen_e*`;
- `CURSOR_VISIBLE`: output type `bool*`;
- `KITTY_KEYBOARD_FLAGS`: output type `uint8_t*`;
- `MOUSE_TRACKING`: output type `bool*`;
- `TOTAL_ROWS`: output type `size_t*`;
- `SCROLLBACK_ROWS`: output type `size_t*`.

Semantics:

- null terminal returns `ROASTTY_INVALID_VALUE`;
- null `out` returns `ROASTTY_INVALID_VALUE`;
- invalid data selector returns `ROASTTY_INVALID_VALUE`;
- declared-but-deferred fields return `ROASTTY_NO_VALUE`;
- successful scalar writes return `ROASTTY_SUCCESS`;
- no field may allocate;
- no field may return borrowed pointers;
- no field may mutate terminal state.

Do not implement title/PWD here. Continue using `roastty_terminal_title` /
`roastty_terminal_pwd` for copied string reads until the full terminal getter
string ownership model is designed.

### 4. Add `roastty_terminal_get_multi`

Add:

```c
ROASTTY_API roastty_result_e roastty_terminal_get_multi(
    roastty_terminal_t,
    size_t count,
    const roastty_terminal_data_e* keys,
    void** values,
    size_t* out_written);
```

Semantics:

- null terminal, null keys, or null values returns `ROASTTY_INVALID_VALUE`;
- `count == 0` succeeds and writes `0` to `out_written` if present;
- every key is validated as a raw integer before dispatch;
- out-of-range key values such as `-1` and `33` return `ROASTTY_INVALID_VALUE`;
- if `values[i] == NULL`, return `ROASTTY_INVALID_VALUE`, stop at index `i`, and
  write the number of prior successful getters to `out_written` if present;
- call `roastty_terminal_get` in order;
- stop on the first non-success result;
- if `out_written` is non-null, write the number of successfully completed
  getters;
- if all succeed, write `count` and return `ROASTTY_SUCCESS`;
- if a deferred field returns `ROASTTY_NO_VALUE`, stop there and report the
  index of that field.

### 5. Add minimal terminal accessors

In `roastty/src/terminal/terminal.rs`, add crate-level read-only accessors as
needed for:

- columns;
- rows;
- cursor pending wrap;
- active screen key;
- cursor visible mode;
- current Kitty keyboard flags;
- mouse tracking;
- total rows;
- scrollback rows.

Do not expose `Screen`, `PageList`, color, selection, or render-state internals
outside the crate unless a narrow accessor is required.

### 6. Extend Rust tests

In `roastty/src/lib.rs`, add `terminal_get_abi` tests covering:

- `ROASTTY_SUCCESS`, existing result values, and `ROASTTY_NO_VALUE` numeric
  values;
- invalid terminal, invalid data selector, and null output pointer;
- raw invalid data values such as `-1` and `33`;
- implemented scalar fields on a fresh terminal;
- cursor coordinates and pending wrap after writes;
- active screen changes after alternate-screen mode sequences;
- cursor visibility after DEC mode 25 reset/set;
- Kitty keyboard flags after the existing Kitty keyboard protocol sequence;
- mouse tracking after mouse mode sequences;
- total rows and scrollback rows after enough linefeeds to create scrollback;
- declared-but-deferred fields return `ROASTTY_NO_VALUE`;
- `get_multi` success writes all requested fields and `out_written == count`;
- `get_multi` stops at the first invalid/deferred field and writes the partial
  count.
- `get_multi` stops at the first null `values[i]` slot and writes the partial
  count.

### 7. Extend the C ABI harness

In `roastty/tests/abi_harness.c`, extend the terminal scenario to verify:

- result enum values remain stable, including `ROASTTY_NO_VALUE == 4`;
- terminal data enum values match upstream slots;
- `roastty_terminal_get` returns expected cols/rows/cursor values;
- bool and enum outputs work through `void*`;
- a deferred field returns `ROASTTY_NO_VALUE`;
- raw invalid data values return `ROASTTY_INVALID_VALUE`;
- `roastty_terminal_get_multi` success, partial-failure, and null value-slot
  behavior.

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs
cargo test -p roastty terminal_get_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty terminal_stream
cargo test -p roastty
! rg -n "ghostty|Ghostty|ghostty_" roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c
```

Required evidence:

- result enum values remain stable and `ROASTTY_NO_VALUE == 4`.
- `roastty_terminal_data_e` numeric values match upstream `TerminalData` slots.
- raw out-of-range data selectors are rejected before internal conversion.
- `roastty_terminal_get` reads implemented scalar fields correctly.
- declared-but-deferred fields return `ROASTTY_NO_VALUE`, not bogus defaults.
- `roastty_terminal_get_multi` matches upstream stop-on-first-error behavior and
  reports `out_written` correctly.
- `roastty_terminal_get_multi` rejects null `values[i]` slots and reports the
  correct partial count.
- title/PWD remain copied-string direct helpers and are not exposed as borrowed
  `terminal_get` strings.
- no getter allocates, mutates terminal state, or returns borrowed pointer
  fields.
- existing terminal stream tests and the full `roastty` test suite still pass.
- no PTY spawn/read/write loop, renderer, font/text stack, selection API,
  clipboard IO, runtime callback dispatch, Swift/macOS frontend code, browser
  behavior, or non-macOS platform branch is added.
- Codex design and result reviews both pass before moving to the next stage.

## Non-Negotiable Invariants

- Use Roastty names in public ABI, implementation-facing comments, tests, and
  modules.
- Do not add public `ghostty_*` compatibility names.
- Preserve upstream terminal data numeric slots.

- Validate raw integer data selectors at the Rust FFI boundary before internal
  enum conversion.
- Keep this experiment to scalar `terminal_get` / `terminal_get_multi` fields.
- Do not expose borrowed strings through `terminal_get`.
- Do not implement title/PWD, color structs, scrollbar structs, cursor style,
  Kitty graphics, render state, selection, formatter handles, PTY, renderer,
  font, IME, Swift frontend, browser, or non-macOS platform behavior.
- Run `cargo fmt` and accept its output.
- Pass Codex design and result reviews before moving to the next stage.

## Failure Criteria

This experiment fails if:

- any public `ghostty_*` or compatibility ABI names are introduced;
- terminal data numeric values drift from upstream slots;
- raw out-of-range data selectors can reach internal enum conversion unchecked;
- title/PWD are exposed through `terminal_get` as borrowed strings;
- unsupported fields return success with placeholder data instead of
  `ROASTTY_NO_VALUE`;
- `get_multi` continues after a failing field or reports the wrong written
  count;
- `get_multi` does not reject null `values[i]` slots with the correct partial
  count;
- getters allocate, mutate terminal state, or expose borrowed internal pointers;
- PTY, renderer, font, selection, runtime callback, Swift frontend, browser, or
  non-macOS platform behavior is added;
- existing terminal, key, mouse, OSC, formatter, or C ABI tests regress;
- the design or result proceeds without the required Codex review gate.

## Codex Design Review

**Result:** Approved after revision.

Codex's first review found three real design gaps: raw integer selector
validation needed to be explicit at the Rust FFI boundary; `get_multi` needed
defined behavior for null `values[i]` slots; and the new `ROASTTY_NO_VALUE`
result code needed stability tests alongside the existing result values.

The design was updated to require raw selector validation for both
`roastty_terminal_get` and every `get_multi` key, stop-on-null-value-slot
semantics with correct partial `out_written`, and Rust/C harness assertions for
`ROASTTY_NO_VALUE == 4` without renumbering existing result codes.

Codex's second review found no remaining blocking design issues and approved the
experiment for implementation.

## Result

**Result:** Pass

Experiment 166 implemented the scalar terminal getter C ABI slice:

- added `ROASTTY_NO_VALUE = 4` without renumbering existing Roastty result
  values;
- added `roastty_terminal_data_e` slots `0..32` and `roastty_terminal_screen_e`
  slots matching upstream terminal data ordering;
- added `roastty_terminal_get`;
- added `roastty_terminal_get_multi`;
- added narrow read-only terminal accessors for columns, rows, cursor position,
  pending wrap, active screen, cursor visibility, Kitty keyboard flags, mouse
  tracking, total rows, and scrollback rows;
- kept title/PWD outside the generic getter path, preserving the copied-string
  direct helpers from Experiment 165;
- returned `ROASTTY_NO_VALUE` for declared-but-deferred fields instead of
  returning placeholders or borrowed pointers.

Codex result review initially found one non-blocking harness coverage gap: the C
ABI harness checked terminal data slots through `SCROLLBACK_ROWS == 15` and
`VIEWPORT_ACTIVE == 32`, but skipped explicit C-side assertions for slots
`16..31`. That gap was fixed by adding C assertions for the width/height, color,
Kitty image, Kitty graphics, and selection slots. The second Codex review found
no blocking issues and confirmed that the prior gap was fixed.

Verification run after the harness fix:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs roastty/src/terminal/page_list.rs
cargo test -p roastty terminal_get_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty terminal_stream
cargo test -p roastty
! rg -n "ghostty|Ghostty|ghostty_" roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c
```

Observed results:

- `terminal_get_abi`: 6 passed.
- C harness: 1 passed.
- `terminal_stream`: 381 passed.
- full `cargo test -p roastty`: 1793 unit tests passed, C harness passed,
  doc-tests passed.
- forbidden public-name grep passed.

## Conclusion

Roastty now has the first generic terminal metadata getter ABI, including
multi-get behavior and explicit deferred-field signaling. This gives future
frontend/app integration a stable scalar query path while leaving string,
selection, color, renderer, and Kitty graphics ownership models for later
experiments.
