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

# Experiment 189: Port Kitty Transmit and Query Execution

## Description

Experiment 188 added image storage. The next coherent Kitty graphics slice is
the non-placement execution path from:

- `vendor/ghostty/src/terminal/kitty/graphics_exec.zig`

This experiment ports query and transmit execution against `ImageStorage`. It
should connect the existing parser, direct image loader, and storage layer so
validated direct images can be queried, transmitted, chunked, assigned automatic
IDs, stored, and responded to with upstream-compatible quiet behavior.

This experiment must not implement display placements, deletion geometry,
terminal cursor movement, renderer integration, terminal APC dispatch, or public
C ABI. Those require terminal/screen integration and belong in later
experiments.

## Changes

1. Add an internal Kitty execution module.
   - Add `roastty/src/terminal/kitty/graphics_exec.rs`.
   - Add `pub(crate) mod graphics_exec;` from
     `roastty/src/terminal/kitty/mod.rs`.
   - Keep all types internal to Roastty. Do not add public C ABI.

2. Add an execution entry point over image storage.
   - Add an internal function such as:
     - `execute(storage: &mut ImageStorage, command: &Command) -> Option<Response<'static>>`
   - If storage is disabled, return `None`, matching upstream's "protocol
     disabled" behavior.
   - Support only:
     - `CommandControl::Query`;
     - `CommandControl::Transmit`.
   - Return an explicit error response for placement/display/delete/animation
     actions in this experiment, or document if a narrower helper is chosen
     instead. Do not silently treat display as successful.
   - Treat `CommandControl::TransmitAndDisplay` as unsupported in this
     experiment. It must return `ERROR: unimplemented action` and must not store
     the image first. Upstream `a=T` loads and then displays, but the display
     half requires placement state, grid pins, and cursor behavior that this
     experiment deliberately defers.

3. Port query behavior.
   - Query requires a non-zero image ID.
   - If the query has no image ID, return `None` and leave storage unchanged.
     `Response::encode()` intentionally suppresses responses that have neither
     `id` nor `image_number`, so there is no meaningful terminal-visible error
     packet for this case until a command supplies an addressable image.
   - Query attempts to initialize a `LoadingImage` with storage image limits,
     but does not store it.
   - Query encodes image-loading errors into response messages.
   - Query respects quiet response filtering through the shared execution entry
     point.

4. Port transmit behavior.
   - Reject commands that specify both image ID and image number.
   - Load direct images through `LoadingImage`.
   - Add completed images to `ImageStorage`.
   - Assign automatic image IDs from `storage.next_image_id` when image ID is
     zero, then increment with wrapping behavior.
   - Mark images as implicit when both image ID and image number are absent, so
     successful implicit-ID loads produce no response.
   - Preserve the resulting response ID after storage insertion.

5. Port chunk handling.
   - If storage has an in-progress `LoadingImage`, append the new command data
     to it.
   - If the continued command has `more_chunks = true`, keep the loading image
     in storage and produce no response.
   - If the continued command has `more_chunks = false`, complete the image,
     store it, clear `storage.loading`, and respond according to quiet rules.
   - If a first transmit command has `more_chunks = true`, store the loading
     image and produce no response.
   - Preserve upstream quiet inheritance:
     - subsequent chunks with `q=0` inherit the initial chunk's quiet mode;
     - subsequent chunks with `q=1` or `q=2` update the in-progress quiet mode.
   - If appending, completing, or storing a continued chunk fails, clear
     `storage.loading` before returning the error response. A failed continued
     load must not leave stale partial image data that the next command can
     accidentally continue.

6. Port response and error encoding.
   - Add upstream-compatible error messages for image-loading and storage
     errors:
     - `ENOMEM: out of memory`;
     - `EINVAL: invalid data`;
     - `EINVAL: decompression failed`;
     - `EINVAL: unsupported format`;
     - `EINVAL: unsupported medium`;
     - `EINVAL: dimensions required`;
     - `EINVAL: dimensions too large`;
     - `EINVAL: image ID required` for any internal helper path that can carry
       an addressable response. A no-ID query through `execute()` remains
       terminal-silent because `Response::encode()` cannot encode it.
     - `EINVAL: image ID and number are mutually exclusive`;
     - `ERROR: unimplemented action`.
   - Test the execution-layer mapping directly. Loader/storage unit tests are
     not enough because this experiment owns the conversion from
     `ImageLoadError` to Kitty response text.
   - Preserve quiet filtering:
     - `q=0`: return non-empty responses;
     - `q=1`: suppress successful `OK` responses and return failures;
     - `q=2`: suppress all responses.

7. Port focused execution tests.
   - Port or create equivalent Rust tests for:
     - `more_chunks` with `q=1`;
     - `more_chunks` with `q=0`;
     - a later chunk increasing quiet from `q=0` to `q=1`;
     - default format is RGBA after transmit;
     - no response when image ID and image number are both absent;
     - query success does not store an image;
     - query without image ID returns `None` and does not mutate storage because
       no encodable response can be addressed;
     - transmit stores an image by ID;
     - transmit rejects both ID and number;
     - implicit ID assignment increments `next_image_id`;
     - storage disabled suppresses all responses and mutations;
     - representative execution-layer error mappings for invalid data or
       dimensions, unsupported format, unsupported medium, decompression
       failure, and storage `OutOfMemory`;
     - final-chunk failure clears `storage.loading`;
     - display/delete/animation actions return the explicit unimplemented
       response and do not mutate storage;
     - `TransmitAndDisplay` returns the explicit unimplemented response and does
       not store the image.

## Verification

Run:

```bash
cargo fmt -- roastty/src/terminal/kitty/mod.rs roastty/src/terminal/kitty/graphics_command.rs roastty/src/terminal/kitty/graphics_image.rs roastty/src/terminal/kitty/graphics_storage.rs roastty/src/terminal/kitty/graphics_exec.rs
cargo test -p roastty kitty_graphics_exec
cargo test -p roastty kitty_graphics_storage
cargo test -p roastty kitty_graphics_image
cargo test -p roastty kitty_graphics_command
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when query/transmit execution stores and responds to
direct images under focused tests, chunked direct loads follow upstream quiet
behavior, and display/delete/placement/rendering/terminal/C ABI behavior remains
deferred.

## Non-Negotiable Invariants

- Do not expose any `ghostty_*` ABI names.
- Do not add public Kitty graphics C ABI in this experiment.
- Do not mutate terminal state or add terminal APC dispatch.
- Do not add placement tracking, grid pins, deletion geometry, cursor movement,
  renderer integration, or Unicode virtual placement.
- Do not implement file, temporary-file, or shared-memory reads.
- Do not implement PNG decoding.
- Do not silently treat display, delete, or animation commands as successful.
- Do not skip Codex design review. If the design review finds a real issue, fix
  it and re-review before committing this experiment design.
- Do not skip Codex result review after implementation.

## Result

**Result:** Pass

Implemented internal Kitty graphics query/transmit execution in
`roastty/src/terminal/kitty/graphics_exec.rs` and wired it from
`roastty/src/terminal/kitty/mod.rs`.

The implementation now:

- executes query commands without storing images;
- executes transmit commands against `ImageStorage`;
- stores validated direct images;
- assigns automatic image IDs with wrapping behavior;
- suppresses implicit-ID success responses;
- handles multi-chunk direct image loads;
- preserves upstream quiet inheritance across chunks;
- clears stale loading state on final-chunk failure;
- maps representative `ImageLoadError` values into Kitty response messages;
- suppresses all behavior when image storage is disabled;
- rejects `TransmitAndDisplay`, display, delete, and animation actions as
  explicitly unimplemented without mutating storage.

Codex result review found one real issue before approval: unaddressed
unimplemented animation/delete errors were being filtered out under default
`q=0` because they had no image ID or number. The fix changed quiet filtering so
empty `OK` responses are still suppressed, but empty non-OK responses are
returned internally. Focused tests now cover default-quiet unimplemented
animation and unaddressed delete behavior.

Verification passed:

```bash
cargo fmt -- roastty/src/terminal/kitty/mod.rs roastty/src/terminal/kitty/graphics_command.rs roastty/src/terminal/kitty/graphics_image.rs roastty/src/terminal/kitty/graphics_storage.rs roastty/src/terminal/kitty/graphics_exec.rs
cargo test -p roastty kitty_graphics_exec
cargo test -p roastty kitty_graphics_storage
cargo test -p roastty kitty_graphics_image
cargo test -p roastty kitty_graphics_command
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The full Roastty suite passed with 1,959 Rust tests plus the C harness.

## Conclusion

The parser, direct image loader, image storage, and non-placement execution path
now form a working internal Kitty graphics pipeline for query and transmit
commands. The next experiment should move into the first placement-oriented
slice: image display lookup and placement state, without yet adding renderer or
public C ABI integration.
