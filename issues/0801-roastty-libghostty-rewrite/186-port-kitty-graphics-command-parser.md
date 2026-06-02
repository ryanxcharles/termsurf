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

# Experiment 186: Port Kitty Graphics Command Parser

## Description

Experiment 185 added the support ABI substrate that Kitty graphics eventually
needs: allocator helpers and process-global sys callbacks. The next coherent
Kitty graphics slice is the command parser and response encoder from:

- `vendor/ghostty/src/terminal/kitty/graphics_command.zig`

This is the right first Kitty graphics implementation layer because every later
image-loading, storage, deletion, placement, and C ABI experiment consumes
parsed Kitty graphics commands. The command parser is deterministic,
self-contained, and has a dense upstream test suite. Port it before adding image
storage or terminal mutation so later failures can be localized to
execution/storage rather than command decoding.

This experiment must not implement the Kitty graphics terminal protocol, image
loading, file/shared-memory handling, PNG decoding, zlib decompression, render
geometry, or public Kitty graphics C ABI. It only ports command parsing, decoded
payload ownership, command model types, delete/display/transmission parsing, and
response byte encoding.

## Changes

1. Refactor the existing Kitty module without changing behavior.
   - Move current `roastty/src/terminal/kitty.rs` to
     `roastty/src/terminal/kitty/mod.rs`.
   - Preserve the existing public/private Rust API used by Kitty keyboard, Kitty
     color, and formatter tests.
   - Add `roastty/src/terminal/kitty/graphics_command.rs`.
   - Update `roastty/src/terminal/mod.rs` only as needed for the module move.

2. Port the command data model.
   - Add internal Rust equivalents of upstream:
     - `Parser`;
     - `Command`;
     - `CommandControl`;
     - `CommandAction`;
     - `Quiet`;
     - `Transmission`;
     - `TransmissionFormat`;
     - `TransmissionMedium`;
     - `TransmissionCompression`;
     - `Display`;
     - `CursorMovement`;
     - `Delete`;
     - `AnimationFrameLoading`;
     - `AnimationFrameComposition`;
     - `AnimationControl`;
     - `AnimationAction`;
     - `CompositionMode`;
     - `Response`.
   - Preserve upstream defaults and integer mappings.
   - Preserve signed parsing for `z`, `H`, and `V` by storing/decoding the
     signed `i32` values exactly, not by clamping or treating them as unsigned.
   - Preserve the rule that `m` / `more_chunks` is respected only for direct
     transmission medium and ignored for file, temporary-file, and shared-memory
     media.

3. Port parser behavior.
   - The parser starts immediately after the `G` in the APC sequence.
   - Parse single-character keys and numeric/single-byte values.
   - Ignore unknown or overlong keys/values the same way upstream does.
   - Return an error for invalid final states, invalid enum values, invalid
     integer overflow, invalid base64 payload, and malformed delete ranges.
   - Enforce a configurable maximum payload byte count before base64 decode.
   - Implement decoded payload ownership with Rust-owned `Vec<u8>` or
     equivalent. Empty payloads must not allocate unnecessarily.
   - Implement a local base64 decoder or add a dependency only if the design
     stays minimal. The decoder must support upstream's unpadded test payloads
     such as `QUFBQQ`.

4. Port response encoding.
   - `Response::encode` must write nothing when both image ID and image number
     are zero.
   - Preserve upstream field order:
     - `i`;
     - `I`;
     - `p`;
     - `;`;
     - message;
     - string terminator `ESC \`.
   - Preserve `ok()` and `empty()` semantics.

5. Port upstream tests.
   - Port every upstream test from
     `vendor/ghostty/src/terminal/kitty/graphics_command.zig`, including:
     - transmission command;
     - transmission `m` ignored for non-direct media;
     - transmission `m` respected for direct media;
     - query command;
     - display command;
     - delete command;
     - no control data;
     - unknown/overlong key/value handling;
     - invalid base64 payload returning the parser's invalid-data error;
     - `max_bytes` enforcement before decode, without allocating past the
       configured limit;
     - large negative signed values;
     - u32/i32 overflow errors;
     - all i32 values;
     - response encoding cases;
     - delete range cases.
   - If any upstream test cannot be ported exactly because Zig-specific memory
     ownership differs, add an equivalent Rust test and document the difference
     in the result.

## Verification

Run:

```bash
cargo fmt -- roastty/src/terminal/mod.rs roastty/src/terminal/kitty/mod.rs roastty/src/terminal/kitty/graphics_command.rs
cargo test -p roastty kitty_graphics_command
cargo test -p roastty kitty_keyboard
cargo test -p roastty kitty_color
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when Kitty graphics command parsing and response encoding
match upstream behavior under the ported tests, and existing Kitty
keyboard/color behavior is unchanged after the module refactor.

## Non-Negotiable Invariants

- Do not expose any `ghostty_*` ABI names.
- Do not add public Kitty graphics C ABI in this experiment.
- Do not mutate terminal state from parsed graphics commands.
- Do not add image storage, image loading, file/shared-memory reads, PNG
  decoding, zlib decompression, placement rendering, or renderer integration.
- Do not hook parsed graphics commands into terminal APC execution except
  through parser-focused tests.
- Do not change existing Kitty keyboard/color behavior except for the mechanical
  module move.
- Do not skip Codex design review. If the design review finds a real issue, fix
  it and re-review before committing this experiment design.
- Do not skip Codex result review after implementation.

## Result

**Result:** Pass

The experiment ported the Kitty graphics command parser and response encoder
into `roastty/src/terminal/kitty/graphics_command.rs`. The existing Kitty module
was moved from `roastty/src/terminal/kitty.rs` to
`roastty/src/terminal/kitty/mod.rs`, with existing Kitty keyboard and color APIs
preserved.

The Rust port includes the parser state machine, command/control model,
transmission/display/delete/animation parsing, response encoding, configurable
payload byte limit enforcement before base64 decode, and Rust-owned decoded
payload storage. The ported tests cover the upstream `graphics_command.zig`
cases plus additional invalid-base64 padding checks and explicit `max_bytes`
coverage.

Verification passed:

```bash
cargo fmt -- roastty/src/terminal/mod.rs roastty/src/terminal/kitty/mod.rs roastty/src/terminal/kitty/graphics_command.rs
cargo test -p roastty kitty_graphics_command
cargo test -p roastty kitty_keyboard
cargo test -p roastty kitty_color
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The full `roastty` suite passed with 1909 Rust tests plus the C harness.

Codex result review found one real implementation issue: the initial local
base64 decoder accepted malformed padding such as `QU=F`. The decoder was
tightened to reject padding before the third quartet slot and reject any
non-padding byte after padding starts. Regression tests for `QU=F`, `Q=UF`, and
`QU==AA` were added. A follow-up Codex review found no remaining code issues.

## Conclusion

Roastty now has a self-contained Kitty graphics command decoder and response
encoder, matching the upstream parser behavior needed by later Kitty graphics
execution and image-storage experiments. No terminal APC wiring, image loading,
storage, decoding, rendering, or public C ABI was added in this experiment, so
the next experiment can choose the next coherent Kitty graphics layer without
debugging command parsing at the same time.
