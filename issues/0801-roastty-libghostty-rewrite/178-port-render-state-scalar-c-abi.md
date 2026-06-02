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

# Experiment 178: Port Render State Scalar C ABI

## Description

Begin the upstream render-state C ABI from:

- `vendor/ghostty/src/terminal/c/render.zig`;
- `vendor/ghostty/src/terminal/render.zig`.

Experiments 176 and 177 added the row/cell and style value ABI needed by render
row iteration. The full upstream render ABI is larger than one safe slice: it
includes render-state handles, dirty state, terminal dimensions, colors, cursor
metadata, row iterators, row-cell iterators, grapheme extraction, style output,
and row selection. This experiment ports the scalar render-state foundation
first:

- render-state handle lifecycle;
- update from a live terminal;
- dimensions, dirty, color, palette, cursor, and cursor-viewport getters;
- dirty setter;
- colors struct getter.

It deliberately does not add row iterators or row-cell iterators yet. Those are
the next coherent subsystem slice once the snapshot handle and scalar state are
stable.

Public names must use Roastty naming only:

- `roastty_render_state_t`;
- `roastty_render_state_*`;
- `roastty_render_state_colors_s`;
- `roastty_render_state_dirty_e`;
- `roastty_render_state_cursor_visual_style_e`.

Upstream names may appear only in this issue document as source citations.

## Changes

1. Re-read upstream and Roastty source:
   - `vendor/ghostty/src/terminal/c/render.zig`;
   - `vendor/ghostty/src/terminal/render.zig`;
   - `roastty/src/terminal/terminal.rs`;
   - `roastty/src/terminal/screen.rs`;
   - `roastty/src/terminal/color.rs`;
   - existing terminal color/cursor C ABI in `roastty/src/lib.rs`.

2. Add internal render-state storage.

   Add a crate-internal render-state representation that can be heap-owned by
   the C ABI. This experiment does not need the full row snapshot yet, but the
   shape should intentionally leave room for the next experiment to add row data
   without replacing the public handle.

   Minimum stored fields:
   - `cols: CellCountInt`;
   - `rows: CellCountInt`;
   - background RGB;
   - foreground RGB;
   - optional cursor RGB;
   - current palette;
   - dirty enum;
   - cursor visual style;
   - cursor visible;
   - cursor blinking;
   - cursor password input;
   - optional cursor viewport position and wide-tail flag.

   The update path should read from the current `Terminal` state. If Roastty
   lacks a direct internal accessor for one field, add a narrow internal
   accessor instead of guessing from the public C ABI.

3. Add public C ABI types in `roastty/include/roastty.h` and mirrored Rust
   `repr(C)`/constants in `roastty/src/lib.rs`.

   ```c
   typedef void* roastty_render_state_t;
   typedef void* roastty_render_state_row_iterator_t;

   typedef enum roastty_render_state_dirty_e {
     ROASTTY_RENDER_STATE_DIRTY_FALSE = 0,
     ROASTTY_RENDER_STATE_DIRTY_PARTIAL = 1,
     ROASTTY_RENDER_STATE_DIRTY_FULL = 2,
   } roastty_render_state_dirty_e;

   typedef enum roastty_render_state_cursor_visual_style_e {
     ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR = 0,
     ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK = 1,
     ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE = 2,
     ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK_HOLLOW = 3,
   } roastty_render_state_cursor_visual_style_e;
   ```

   `roastty_render_state_row_iterator_t` is declared as an opaque placeholder
   only because the upstream scalar `ROW_ITERATOR` selector's output type
   depends on it. This experiment must not implement row iterator allocation,
   row iteration, or row getters.

4. Add render-state data and option selector enums matching upstream numeric
   order with Roastty names.

   Data selectors:

   | Name                                                  | Value | Output type                                         |
   | ----------------------------------------------------- | ----- | --------------------------------------------------- |
   | `ROASTTY_RENDER_STATE_DATA_INVALID`                   | 0     | none; always invalid                                |
   | `ROASTTY_RENDER_STATE_DATA_COLS`                      | 1     | `uint16_t*`                                         |
   | `ROASTTY_RENDER_STATE_DATA_ROWS`                      | 2     | `uint16_t*`                                         |
   | `ROASTTY_RENDER_STATE_DATA_DIRTY`                     | 3     | `roastty_render_state_dirty_e*`                     |
   | `ROASTTY_RENDER_STATE_DATA_ROW_ITERATOR`              | 4     | deferred; returns `ROASTTY_NO_VALUE` in this slice  |
   | `ROASTTY_RENDER_STATE_DATA_COLOR_BACKGROUND`          | 5     | `roastty_rgb_s*`                                    |
   | `ROASTTY_RENDER_STATE_DATA_COLOR_FOREGROUND`          | 6     | `roastty_rgb_s*`                                    |
   | `ROASTTY_RENDER_STATE_DATA_COLOR_CURSOR`              | 7     | `roastty_rgb_s*`, `ROASTTY_NO_VALUE` when unset     |
   | `ROASTTY_RENDER_STATE_DATA_COLOR_CURSOR_HAS_VALUE`    | 8     | `bool*`                                             |
   | `ROASTTY_RENDER_STATE_DATA_COLOR_PALETTE`             | 9     | `roastty_palette_t*`                                |
   | `ROASTTY_RENDER_STATE_DATA_CURSOR_VISUAL_STYLE`       | 10    | `roastty_render_state_cursor_visual_style_e*`       |
   | `ROASTTY_RENDER_STATE_DATA_CURSOR_VISIBLE`            | 11    | `bool*`                                             |
   | `ROASTTY_RENDER_STATE_DATA_CURSOR_BLINKING`           | 12    | `bool*`                                             |
   | `ROASTTY_RENDER_STATE_DATA_CURSOR_PASSWORD_INPUT`     | 13    | `bool*`                                             |
   | `ROASTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_HAS_VALUE` | 14    | `bool*`                                             |
   | `ROASTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_X`         | 15    | `uint16_t*`, `ROASTTY_NO_VALUE` when absent         |
   | `ROASTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_Y`         | 16    | `uint16_t*`, `ROASTTY_NO_VALUE` when absent         |
   | `ROASTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_WIDE_TAIL` | 17    | `bool*`, `ROASTTY_NO_VALUE` when viewport is absent |

   Option selectors:

   | Name                                | Value | Input type                      |
   | ----------------------------------- | ----- | ------------------------------- |
   | `ROASTTY_RENDER_STATE_OPTION_DIRTY` | 0     | `roastty_render_state_dirty_e*` |

5. Add `roastty_render_state_colors_s`.

   ```c
   typedef struct roastty_render_state_colors_s {
     size_t size;
     roastty_rgb_s background;
     roastty_rgb_s foreground;
     roastty_rgb_s cursor;
     bool cursor_has_value;
     roastty_palette_t palette;
   } roastty_render_state_colors_s;
   ```

   Preserve the expected macOS C layout:
   - `sizeof(roastty_render_state_colors_s) == 792`;
   - `_Alignof(roastty_render_state_colors_s) == 8`;
   - `offsetof(roastty_render_state_colors_s, size) == 0`;
   - `offsetof(roastty_render_state_colors_s, background) == 8`;
   - `offsetof(roastty_render_state_colors_s, foreground) == 11`;
   - `offsetof(roastty_render_state_colors_s, cursor) == 14`;
   - `offsetof(roastty_render_state_colors_s, cursor_has_value) == 17`;
   - `offsetof(roastty_render_state_colors_s, palette) == 18`.

   Preserve upstream's size-field behavior:
   - `out->size` is an input capacity supplied by the caller;
   - `colors_get` must not rewrite `out->size`;
   - `colors_get(NULL, out)` returns `ROASTTY_INVALID_VALUE`;
   - `colors_get(state, NULL)` returns `ROASTTY_INVALID_VALUE`;
   - `out->size < sizeof(size_t)` returns `ROASTTY_INVALID_VALUE`;
   - fields are written only if they fit within `out->size`;
   - cursor RGB is written only when cursor color has a value and the field
     fits;
   - `cursor_has_value` is written when the field fits;
   - palette entries are copied only up to the number of complete entries that
     fit after the palette offset.

6. Add public functions:

   ```c
   ROASTTY_API roastty_result_e
   roastty_render_state_new(roastty_render_state_t*);

   ROASTTY_API void
   roastty_render_state_free(roastty_render_state_t);

   ROASTTY_API roastty_result_e
   roastty_render_state_update(roastty_render_state_t, roastty_terminal_t);

   ROASTTY_API roastty_result_e
   roastty_render_state_get(roastty_render_state_t,
                            roastty_render_state_data_e,
                            void*);

   ROASTTY_API roastty_result_e
   roastty_render_state_get_multi(roastty_render_state_t,
                                  size_t,
                                  const roastty_render_state_data_e*,
                                  void**,
                                  size_t*);

   ROASTTY_API roastty_result_e
   roastty_render_state_set(roastty_render_state_t,
                            roastty_render_state_option_e,
                            const void*);

   ROASTTY_API roastty_result_e
   roastty_render_state_colors_get(roastty_render_state_t,
                                   roastty_render_state_colors_s*);
   ```

   Behavior:
   - `new(NULL)` returns `ROASTTY_INVALID_VALUE`;
   - `new(out)` allocates a render state initialized to the upstream empty
     defaults;
   - `free(NULL)` is a no-op;
   - `update(NULL, terminal)` and `update(state, NULL)` return
     `ROASTTY_INVALID_VALUE`;
   - `update(state, terminal)` snapshots the scalar data listed above;
   - raw enum values are accepted as `int`/`c_int` and validated before
     matching;
   - invalid data/option selectors return `ROASTTY_INVALID_VALUE`;
   - null output pointers return `ROASTTY_INVALID_VALUE`;
   - `set(NULL, option, value)` returns `ROASTTY_INVALID_VALUE`;
   - `set(state, option, NULL)` returns `ROASTTY_INVALID_VALUE`;
   - `set(state, DIRTY, value)` validates the pointed-to dirty enum before
     assigning it;
   - invalid pointed-to dirty enum values return `ROASTTY_INVALID_VALUE` without
     mutating the existing dirty state;
   - `get_multi` null keys or values returns `ROASTTY_INVALID_VALUE`;
   - `get_multi` with `count == 0` succeeds and writes `0` to `out_written` if
     provided;
   - `get_multi` writes the number of completed entries on success or first
     failure;
   - `ROW_ITERATOR` returns `ROASTTY_NO_VALUE` until Experiment 179 ports row
     iterators.

7. Define pre-update and update semantics for scalar data.

   Immediately after `new` and before any `update`, getters expose the upstream
   empty render-state defaults:
   - `cols == 0`;
   - `rows == 0`;
   - `dirty == ROASTTY_RENDER_STATE_DIRTY_FALSE`;
   - background RGB is `(0, 0, 0)`;
   - foreground RGB is `(255, 255, 255)`;
   - cursor color is absent;
   - palette is the default palette;
   - cursor visual style is `ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK`;
   - cursor visible is `true`;
   - cursor blinking is `false`;
   - cursor password input is `false`;
   - cursor viewport is absent, so viewport x/y/wide-tail selectors return
     `ROASTTY_NO_VALUE`.

   On an untouched terminal, after `update`:
   - `cols` and `rows` match terminal dimensions;
   - background is the current background default;
   - foreground is the current foreground default;
   - cursor color is absent unless explicitly set;
   - palette is the current palette;
   - cursor visual style reflects the active screen's cursor visual style;
   - cursor visible reflects terminal cursor visibility mode;
   - cursor blinking reflects terminal cursor blinking mode;
   - cursor password input is `false` until the password-input detector exists;
   - cursor viewport is present when the active cursor is inside the viewport;
   - cursor viewport x/y match the cursor position;
   - cursor viewport wide-tail is `false` until wide-tail cursor detection is
     ported.

   If a field is intentionally stubbed in this slice (`password_input`,
   `wide_tail`), document it in the result and keep the corresponding future
   work in the conclusion.

8. Keep scope narrow:
   - Do not add row iterator allocation/free/next.
   - Do not add row getters, row setters, or row selection structs.
   - Do not add row-cell iterators, grapheme extraction, style lookup, cell
     foreground/background resolution, Kitty graphics, highlights, hyperlinks,
     or formatter objects.
   - Do not expose raw page pointers or node pointers.
   - Do not add Metal renderer, Swift integration, app runtime integration, or
     browser overlay behavior.
   - Do not add `ghostty_*` symbols or compatibility aliases.

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/mod.rs roastty/src/terminal/terminal.rs
cargo test -p roastty render_state_c_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Pass criteria:

- render-state handle lifecycle works and frees cleanly;
- pre-update getter defaults match the upstream empty render-state values;
- update snapshots scalar terminal state from a live terminal;
- data and option selector numeric values match upstream order with Roastty
  names;
- `roastty_render_state_colors_s` size, alignment, and offsets match the
  specified macOS C layout;
- dirty get/set round trips;
- dirty set rejects null state, null value, invalid option, and invalid
  pointed-to dirty enum values without mutation;
- color and palette getters match existing terminal color state;
- cursor visible/blinking/style/viewport scalar getters are populated from the
  terminal;
- cursor-absent and deferred-row-iterator fields return `ROASTTY_NO_VALUE` where
  specified;
- `get_multi` reports partial progress correctly;
- colors struct size-field compatibility is implemented and tested;
- C header, Rust ABI tests, C harness, full `roastty` tests, no-Ghostty check,
  and `git diff --check` all pass;
- Codex reviews and approves both the design and completed result.

Partial criteria:

- handle lifecycle, dimensions, dirty, and colors pass, but one cursor scalar
  lacks a clean internal accessor and is documented for a follow-up.

Failure criteria:

- the API exposes raw page/node pointers or row snapshot storage;
- raw C enum values are represented as Rust enums before validation;
- `ROW_ITERATOR` is silently implemented as a fake iterator instead of returning
  a clear deferred result;
- colors struct size handling reads or writes fields before validating the size
  field;
- the implementation expands into row iterators, row cells, formatter objects,
  renderer backend code, Swift integration, or browser overlay behavior;
- the API exposes `ghostty_*` symbols or compatibility aliases.

## Review

This design must be reviewed with the Codex review skill before implementation.
All real findings must be fixed, and the design must be re-reviewed until Codex
approves it.

After implementation and result recording, the completed result must also be
reviewed with Codex and approved before the result commit.

## Result

**Result:** Pass

Implemented the scalar render-state C ABI foundation:

- added opaque `roastty_render_state_t` and
  `roastty_render_state_row_iterator_t` handle types;
- added Roastty-named render-state dirty, cursor visual style, data selector,
  and option selector enums matching upstream numeric order;
- added `roastty_render_state_colors_s` with the specified macOS C layout;
- added `roastty_render_state_new`, `roastty_render_state_free`,
  `roastty_render_state_update`, `roastty_render_state_get`,
  `roastty_render_state_get_multi`, `roastty_render_state_set`, and
  `roastty_render_state_colors_get`;
- added private scalar render-state storage initialized to upstream empty
  defaults;
- added update from a live `roastty_terminal_t` for dimensions, colors, palette,
  cursor visual style, cursor visibility, cursor blinking, and cursor viewport
  position;
- added narrow internal terminal accessors for cursor visual style and cursor
  blinking;
- added Rust tests for layout, enum values, pre-update defaults, update
  snapshots, dirty set validation/no-mutation behavior, `get_multi`, deferred
  row iterator behavior, and colors size-capacity behavior;
- added C harness coverage for the same public ABI surface.

Two fields remain intentionally conservative in this scalar slice:

- `cursor_password_input` is always `false` until the password-input detector is
  ported;
- `cursor_viewport_wide_tail` is always `false` until wide-tail cursor detection
  is ported.

`ROASTTY_RENDER_STATE_DATA_ROW_ITERATOR` returns `ROASTTY_NO_VALUE` as designed.
No fake iterator, row snapshot storage, row-cell iterator, formatter object,
renderer backend, Swift integration, browser overlay behavior, raw page pointer,
or node pointer exposure was added.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/mod.rs roastty/src/terminal/color.rs roastty/src/terminal/cursor.rs roastty/src/terminal/terminal.rs
cargo test -p roastty render_state_c_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The full `roastty` test run passed with 1859 Rust tests, the C ABI harness, and
doc tests.

## Conclusion

Roastty now has the scalar render-state handle and getters needed before the
full row iterator API can be ported. The ABI can allocate/free render state,
snapshot scalar terminal state, expose colors and cursor state, and round-trip
dirty state without exposing page storage or inventing partial row iteration.

The next experiment should port render-state row iterators and row getters on
top of this handle. That experiment should add real row snapshot storage and
selection/dirty/raw-row access, while keeping row-cell iteration and grapheme
extraction as the following slice unless the design review shows they can be
verified together without weakening failure diagnosis.
