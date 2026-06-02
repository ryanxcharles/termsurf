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

# Experiment 183: Port Terminal Formatter C ABI

## Description

Experiments 82-97 ported the formatter internals for plain, VT, HTML, screen
extras, terminal extras, palette output, modes, scrolling regions, tabstops,
keyboard state, and working-directory output. Those behaviors are available to
Rust tests, but Roastty does not yet expose upstream's
`terminal/c/formatter.zig` C ABI:

- `terminal_new`
- `format_buf`
- `format_alloc`
- `free`

This experiment adds the formatter handle and terminal formatter C API, using
Roastty names and the existing formatter implementation. It should not rewrite
formatter behavior. The implementation surface is the C boundary: option
structs, handle ownership, terminal binding, buffer formatting, allocated
formatting, and tests proving that C callers can use the API.

## Changes

1. In `roastty/include/roastty.h`, add `roastty_formatter_t` and the public C
   structs/enums needed by the formatter ABI:
   - `roastty_formatter_format_e` with `PLAIN = 0`, `VT = 1`, `HTML = 2`,
     matching the existing selection format values.
   - `roastty_formatter_screen_extra_s` with `size`, `cursor`, `style`,
     `hyperlink`, `protection`, `kitty_keyboard`, and `charsets`.
   - `roastty_formatter_terminal_extra_s` with `size`, `palette`, `modes`,
     `scrolling_region`, `tabstops`, `pwd`, `keyboard`, and nested screen
     extras.
   - `roastty_formatter_terminal_options_s` with `size`, `emit`, `unwrap`,
     `trim`, `extra`, and optional `const roastty_selection_s* selection`.

   Keep the field order aligned with upstream `terminal/c/formatter.zig`.
   Include layout assertions in Rust tests and C harness checks.

2. In `roastty/src/terminal/screen.rs` and `roastty/src/terminal/terminal.rs`,
   expose only the formatter types and builder methods needed by
   `roastty/src/lib.rs` as `pub(crate)`:
   - `ScreenFormatterExtra`
   - `TerminalFormatterOptions`
   - `TerminalFormatter`
   - `TerminalFormatterExtra`

   Do not change formatter behavior or existing tests. This is a visibility
   adjustment so the C bridge can construct the same formatter state that
   terminal-internal tests already use.

3. In `roastty/src/lib.rs`, add a formatter handle type and implementations for:
   - `roastty_formatter_terminal_new(roastty_formatter_t*, roastty_terminal_t, roastty_formatter_terminal_options_s)`
   - `roastty_formatter_format_buf(roastty_formatter_t, uint8_t*, size_t, size_t*)`
   - `roastty_formatter_format(roastty_formatter_t, roastty_string_s*)`
   - `roastty_formatter_free(roastty_formatter_t)`

   Naming note: upstream calls the allocated form `format_alloc`, but Roastty's
   existing allocation-returning APIs use `roastty_string_s` plus
   `roastty_string_free`. Use `roastty_formatter_format` for the public Roastty
   name unless Codex review identifies a stronger reason to expose
   `roastty_formatter_format_alloc`.

4. Define the validation contract before implementation:
   - `roastty_formatter_terminal_new` returns `ROASTTY_INVALID_VALUE` for a null
     output pointer, null terminal handle, invalid `emit` value, or any options
     struct whose `size` is smaller than the fields the implementation reads.
   - On every failure path in `roastty_formatter_terminal_new`, initialize
     `*out` to null before returning when `out` is non-null.
   - Use the existing sized-struct pattern from other Roastty C ABI helpers:
     read only fields covered by the caller-provided `size`, default omitted
     fields to zero/false, and reject undersized structs before reading required
     fields.
   - Apply the same sized-struct contract to nested
     `roastty_formatter_terminal_extra_s` and
     `roastty_formatter_screen_extra_s`.

5. Formatter handles must own the options needed to format later while borrowing
   the terminal by handle. A formatter handle is valid only while its terminal
   remains live. Calling a formatter after freeing its terminal is
   caller-invalid and is not required to return a clean error. This matches
   upstream's borrowed formatter lifetime model and the existing Roastty C
   convention that untracked borrowed handles are caller-invalid after their
   owner is freed.

   Implementation guidance:
   - Store the terminal handle, format, unwrap/trim flags, extra flags, and an
     optional copied `TerminalSelection`.
   - Do not store Rust references into the terminal inside the handle.
   - On each format call, resolve the terminal handle with
     `terminal_from_handle` and construct a fresh `TerminalFormatter` borrowing
     the terminal for that call only.
   - If the stored selection is present, convert it through the active screen's
     `selection_from_grid_refs` path at format time so invalid/stale grid refs
     return the same errors as selection formatting.

6. Buffer contracts:
   - `roastty_formatter_format_buf` returns `ROASTTY_INVALID_VALUE` for a null
     formatter or null `out_written`.
   - A null `out` pointer is allowed for measurement, regardless of `out_len`.
   - It writes the required byte length to `out_written` before returning
     `ROASTTY_OUT_OF_SPACE`.
   - It returns `ROASTTY_OUT_OF_SPACE` when `out == null` or `out_len` is too
     small for non-empty output, matching upstream's measuring behavior.
   - It succeeds for empty output with null output buffer and `out_len == 0`.
   - `roastty_formatter_format` initializes `out` to an empty string before
     validation, returns `ROASTTY_INVALID_VALUE` for null `out`, and returns an
     owned `roastty_string_s` freed by `roastty_string_free` on success.

7. Add focused Rust tests for:
   - formatter option struct layout and discriminants
   - terminal formatter creation/free/null validation
   - `terminal_new` failure initialization of `*out`
   - invalid `emit` rejection
   - sized-struct validation and defaulting for terminal options, terminal
     extras, and screen extras
   - plain formatting to caller buffer
   - `OUT_OF_SPACE` length reporting
   - null output buffer measurement, including `out == null && out_len > 0`
   - allocated string formatting through `roastty_formatter_format`
   - selection-restricted formatting
   - VT/HTML formatting smoke tests
   - extra flags reaching the existing formatter path, at least palette and one
     screen extra such as cursor or hyperlink

   Do not add a test that calls the formatter after terminal free. That call is
   caller-invalid for this borrowed-handle API and would only prove undefined
   behavior around a stale raw handle.

8. Update `roastty/tests/abi_harness.c` to compile and exercise the new
   formatter structs and functions from C.

## Verification

Run the focused and full verification set:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/screen.rs roastty/src/terminal/terminal.rs
cargo test -p roastty formatter_c_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when C callers can create a terminal formatter, format to
a caller buffer, format to an owned `roastty_string_s`, free the formatter, and
exercise plain/VT/HTML plus representative extra flags through the existing
Roastty formatter implementation.

## Non-Negotiable Invariants

- Do not expose any `ghostty_*` ABI names.
- Do not rewrite formatter internals in this experiment.
- Do not add a new formatter behavior that differs from the existing Rust
  formatter tests unless the experiment records an upstream mismatch.
- Do not make formatter handles own terminal state or clone terminal contents.
  Handles borrow a terminal by handle and format fresh snapshots on each call.
- Do not skip Codex result review. If the result review finds a real gap, fix it
  and re-review before recording the result.

## Result

**Result:** Pass

Implemented the terminal formatter C ABI with Roastty names:

- `roastty_formatter_t`
- `roastty_formatter_terminal_new`
- `roastty_formatter_format_buf`
- `roastty_formatter_format`
- `roastty_formatter_free`

The formatter handle stores the terminal handle, format mode, unwrap/trim flags,
extra flags, and an optional copied selection. It does not store Rust references
into terminal storage. Each format call resolves the terminal handle and formats
a fresh snapshot through the existing terminal formatter path.

The first Codex result review found one real blocker: nested sized structs were
not yet proven safe because a parent struct size could cover only a nested
`size` field while the nested struct claimed more fields than the parent
covered. The implementation was corrected to reject uncontained nested `extra`
and `screen` sizes, and tests were added for uncontained nested structs plus a
partial-but-contained screen cursor extra.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs
cargo test -p roastty formatter_c_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Codex re-reviewed the fixed implementation and found no remaining blockers.

## Conclusion

Roastty now exposes the formatter ABI needed by C callers for full-terminal
plain, VT, and HTML formatting, including caller-buffer formatting,
`roastty_string_s` allocation, explicit selection-restricted formatting, and
representative terminal/screen extras. The next experiment can continue with the
remaining public C ABI gaps rather than revisiting formatter internals.
