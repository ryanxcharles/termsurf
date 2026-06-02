# Experiment 200: Port Kitty Graphics Shared Memory

## Description

Experiment 199 ported Kitty graphics regular-file and temporary-file media.
Roastty now supports every non-direct image medium except shared memory.

This experiment ports `TransmissionMedium::SharedMemory`, following upstream's
macOS/POSIX `shm_open` + `fstat` + `mmap` path. This is a distinct subsystem
slice from file loading because it uses named POSIX shared-memory objects,
explicit unlink cleanup, unsafe memory mapping, and stat-size validation.

Use upstream source as the behavior reference:

- `vendor/ghostty/src/terminal/kitty/graphics_image.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_command.zig`

## Changes

1. Implement shared-memory loading in
   `roastty/src/terminal/kitty/graphics_image.rs`.

   Behavior:
   - reject the medium with `UnsupportedMedium` unless `limits.shared_memory` is
     enabled;
   - reject non-direct PNG with `UnsupportedMedium` before opening shared
     memory, because Roastty still has no PNG decode hook;
   - reject names containing an interior NUL byte;
   - convert the Kitty payload bytes into a NUL-terminated C string for
     `shm_open`;
   - call `libc::shm_open(name, O_RDONLY, 0)`;
   - return `InvalidData` if `shm_open` fails;
   - once `shm_open` succeeds, close the file descriptor on every path;
   - once `shm_open` succeeds, call `shm_unlink(name)` on every path, matching
     upstream's cleanup behavior;
   - call `fstat` and reject non-positive sizes;
   - compute expected image size from format/width/height for raw formats;
   - use checked arithmetic for width × height × bytes-per-pixel;
   - return `InvalidData` if expected-size arithmetic overflows;
   - reject the load when the shared-memory object is smaller than the expected
     size;
   - map the full stat size with read-only shared `mmap`;
   - copy only the requested image bytes into owned Roastty memory;
   - unmap before returning;
   - return `InvalidData` on open/stat/size/map/range errors and `OutOfMemory`
     only for owned-buffer allocation failure.

2. Make range handling explicit and safe.

   Upstream slices `map[start..end]` after computing:
   - `start = transmission.offset`;
   - `end = min(offset + size, expected_size)` when `size > 0`;
   - otherwise `end = expected_size`.

   Roastty must preserve the intended behavior without allowing panics:
   - reject if `offset > expected_size`;
   - use checked arithmetic for `offset + size`;
   - clamp `end` to `expected_size`;
   - reject if `end < start`;
   - copy `map[start..end]`.

3. Keep file/temp-file behavior unchanged.

   Do not change the regular-file or temporary-file code from Experiment 199
   except for small helper extraction if it clearly reduces duplication without
   weakening tests.

4. Keep PNG and rendering out of scope.

   Direct PNG still follows existing deferred behavior and fails at completion
   with `UnsupportedFormat`. Non-direct PNG, including shared-memory PNG, must
   still short-circuit with `UnsupportedMedium` before OS resource access.

5. Add tests.

   In `roastty/src/terminal/kitty/graphics_image.rs`, add tests for:
   - shared memory blocked by limits;
   - shared memory allowed by limits;
   - shared memory object is unlinked after successful load;
   - shared memory object is unlinked after `fstat`/size/range/mmap-era failure
     after `shm_open` succeeds;
   - missing shared memory object returns `InvalidData`;
   - interior NUL name returns `InvalidData`;
   - object smaller than expected image size returns `InvalidData`;
   - oversized dimensions that overflow expected-size arithmetic return
     `InvalidData` without panic and still close/unlink after `shm_open`
     succeeds;
   - offset/size read only the requested byte range;
   - offset beyond expected image size returns `InvalidData`;
   - non-direct PNG shared-memory input returns `UnsupportedMedium` and does not
     attempt to open or unlink the object.

   In `roastty/src/terminal/terminal.rs`, add terminal-stream coverage that goes
   through the parser and active-screen storage path:
   - shared-memory medium is rejected by default;
   - enabling `KittyImageMedium::SharedMemory` allows a shared-memory-backed
     image to be stored;
   - disabling the flag rejects shared-memory media again without affecting
     direct image loading.

   Tests may use `libc::shm_open`, `ftruncate`, `write`, `close`, `shm_unlink`,
   and unique names derived from process id plus timestamp. Every test-created
   object must be unlinked in cleanup even when the loader is expected to unlink
   it first.

6. Preserve formatting and review rules.

   Run:

   ```bash
   cargo fmt -- roastty/src/terminal/terminal.rs roastty/src/terminal/kitty/graphics_image.rs roastty/src/terminal/kitty/graphics_exec.rs
   prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/200-port-kitty-graphics-shared-memory.md
   ```

## Verification

Run:

```bash
cargo fmt -- roastty/src/terminal/terminal.rs roastty/src/terminal/kitty/graphics_image.rs roastty/src/terminal/kitty/graphics_exec.rs
cargo test -p roastty kitty_graphics_image
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when:

- shared-memory Kitty image loading works when `limits.shared_memory` is
  enabled;
- shared-memory media remains rejected when the flag is disabled;
- shared-memory objects are closed and unlinked after successful open;
- malformed names, missing objects, too-small objects, and invalid ranges fail
  without panics;
- expected-size overflow fails without panic and without leaking the shared
  memory object after `shm_open` succeeds;
- offset/size behavior matches the intended upstream byte-range behavior;
- file and temporary-file media from Experiment 199 still pass their tests;
- direct Kitty graphics behavior is unchanged;
- PNG decoding remains unsupported;
- Codex approves the experiment design before implementation and approves the
  result before the experiment is recorded.

## Non-Negotiable Invariants

- Do not decode PNG.
- Do not render images.
- Do not add Metal or any platform renderer.
- Do not weaken Experiment 198's terminal option persistence or reset behavior.
- Do not weaken Experiment 199's file/temp-file path validation or cleanup
  behavior.
- Do not leave test-created shared-memory objects behind.
- Do not allow shared-memory media unless the corresponding terminal option is
  enabled.
- Do not expose any `ghostty_*` ABI names.
- Do not skip Codex design review or Codex result review.
