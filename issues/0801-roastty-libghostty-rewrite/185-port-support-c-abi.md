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

# Experiment 185: Port Support C ABI

## Description

Experiment 184 completed the remaining small standalone terminal encoding C ABI.
The next coherent public surface is upstream's support C ABI:

- `vendor/ghostty/include/ghostty/vt/build_info.h`
- `vendor/ghostty/include/ghostty/vt/allocator.h`
- `vendor/ghostty/include/ghostty/vt/sys.h`
- `vendor/ghostty/src/terminal/c/build_info.zig`
- `vendor/ghostty/src/terminal/c/allocator.zig`
- `vendor/ghostty/src/terminal/c/sys.zig`

These APIs are not terminal behavior by themselves. They are library-support
plumbing: build-option queries, library-owned byte allocation/free helpers, and
process-global system callbacks for PNG decode and logging. Porting them now
removes another public C ABI gap and prepares the substrate needed by later
Kitty graphics experiments without implementing Kitty graphics in this slice.

Roastty already has a coarse `roastty_info()` skeleton from Issue 800. This
experiment must add the upstream-shaped query API rather than relying on that
legacy aggregate. Keep `roastty_info()` for now unless implementing this slice
proves it conflicts with the upstream-shaped API; removal or migration of the
old skeleton belongs in a separate cleanup experiment.

## Changes

1. Add build-info C ABI types and function.
   - In `roastty/include/roastty.h`, add:
     - `roastty_optimize_mode_e` with upstream discriminants: debug = 0, release
       safe = 1, release small = 2, release fast = 3.
     - `roastty_build_info_e` with upstream discriminants: invalid = 0, SIMD =
       1, Kitty graphics = 2, tmux control mode = 3, optimize = 4, version
       string = 5, version major = 6, version minor = 7, version patch = 8,
       version pre = 9, version build = 10.
     - `roastty_build_info(roastty_build_info_e, void*)`.
   - In `roastty/src/lib.rs`, implement `roastty_build_info`.
   - Return `ROASTTY_INVALID_VALUE` for invalid enum values, the invalid
     variant, or null `out`.
   - Return deterministic compile-time values:
     - SIMD: `false` unless a specific Roastty SIMD feature already exists;
     - Kitty graphics: `false` until the Kitty graphics subsystem is ported;
     - tmux control mode: `false` until that subsystem is ported;
     - optimize: map Rust build mode to the closest upstream mode
       (`debug_assertions` -> debug, otherwise release fast unless a stronger
       profile signal is available);
     - version string: borrowed static `roastty_string_s` pointing at `VERSION`;
     - version major/minor/patch: parsed from `VERSION` when possible, otherwise
       `0`;
     - version pre/build: borrowed empty `roastty_string_s` until version
       metadata parsing is added.
   - Build-info string outputs are borrowed process-static values, matching
     upstream's build-info string shape. They are valid for the lifetime of the
     process and must not be passed to `roastty_string_free`. Document this in
     the header comments and test that the C harness observes the pointer/length
     without freeing it.
   - Document in the result whether release-safe/release-small cannot be
     distinguished from Rust build metadata yet.

2. Add allocator C ABI.
   - In `roastty/include/roastty.h`, add:
     - `roastty_allocator_vtable_s`;
     - `roastty_allocator_s`;
     - `roastty_alloc(const roastty_allocator_s*, size_t)`;
     - `roastty_free(const roastty_allocator_s*, uint8_t*, size_t)`.
   - The vtable must preserve upstream field order and callback signatures:
     `alloc`, `resize`, `remap`, `free`.
   - Mirror these exact Rust-side ABI shapes with `#[repr(C)]` structs:
     - `alloc: Option<unsafe extern "C" fn(*mut c_void, usize, u8, usize) -> *mut u8>`;
     - `resize: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, u8, usize, usize) -> bool>`;
     - `remap: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, u8, usize, usize) -> *mut c_void>`;
     - `free: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, u8, usize)>`;
     - `roastty_allocator_s { void* ctx; const roastty_allocator_vtable_s* vtable; }`.
   - Implement default allocator behavior for `allocator == null` using Rust's
     global allocator and the caller-provided length. Do not add a global
     allocation tracking map in this experiment; upstream requires callers to
     pass the original allocation length to `free`, and the byte-buffer helper
     uses alignment `1`.
   - Implement custom allocator behavior for `allocator != null` by calling its
     vtable:
     - `roastty_alloc` calls `vtable->alloc(ctx, len, alignment, ret_addr)`;
     - `roastty_free` calls `vtable->free(ctx, ptr, len, alignment, ret_addr)`;
     - this experiment does not need to expose resize/remap helpers, but the
       struct must include those fields for ABI parity.
   - Use alignment `1` for the byte-buffer helper, matching the upstream
     `uint8_t` allocation helper.
   - Define zero-length allocation explicitly:
     - `roastty_alloc(..., 0)` returns null for both default and custom
       allocators and does not call a custom `alloc` callback;
     - `roastty_free(..., ptr, 0)` is a no-op, even when `ptr` is non-null.
   - `roastty_free(..., null, len)` is a no-op.
   - If a custom allocator is malformed, such as a null vtable or missing
     callback needed for the operation, fail safely: allocation returns null and
     free becomes a no-op.

3. Add system callback C ABI.
   - In `roastty/include/roastty.h`, add:
     - `roastty_sys_image_s`;
     - `roastty_sys_log_level_e`;
     - `roastty_sys_log_fn`;
     - `roastty_sys_decode_png_fn`;
     - `roastty_sys_option_e`;
     - `roastty_sys_set(roastty_sys_option_e, const void*)`;
     - `roastty_sys_log_stderr(...)`.
   - Preserve upstream enum discriminants:
     - log levels: error = 0, warning = 1, info = 2, debug = 3;
     - sys options: userdata = 0, decode PNG = 1, log = 2.
   - Store process-global sys state in Rust:
     - userdata pointer;
     - optional PNG decoder callback;
     - optional log callback.
   - Protect process-global sys state with synchronization (`Mutex`, `RwLock`,
     or equivalent). The public API is process-global and callbacks may be used
     from any thread, so unsynchronized `static mut` state is not acceptable.
   - Treat the `value` argument for callback options as the callback pointer
     value cast to `const void*`, matching upstream's
     `Input type: GhosttySysLogFn` / `GhosttySysDecodePngFn` contract and
     examples like `ghostty_sys_set(..., &ghostty_sys_log_stderr)`. Do not read
     `value` as a pointer to a separate function-pointer slot.
   - `roastty_sys_set` returns `ROASTTY_INVALID_VALUE` for invalid enum values
     and `ROASTTY_SUCCESS` for valid options, including null values that clear a
     callback.
   - Do not add Kitty graphics decoding or image storage in this experiment. The
     PNG callback is only stored and exposed through test-only/internal helpers
     if needed for verification.
   - Implement `roastty_sys_log_stderr` as the convenience formatter:
     - `"[error](scope): message\n"` when scope is non-empty;
     - `"[error]: message\n"` when scope is empty;
     - analogous text for warning/info/debug.
   - Define out-of-range log-level values for `roastty_sys_log_stderr` as
     `"unknown"` rather than panicking or causing undefined behavior.

4. Add Rust tests and C harness coverage.
   - Rust tests must cover:
     - build-info enum values and returned value shapes;
     - invalid/null `roastty_build_info` calls;
     - build-info string outputs are borrowed and are not freed;
     - default allocation/free;
     - zero-length allocation/free behavior;
     - custom allocator allocation/free callbacks with observed
       ctx/len/alignment;
     - malformed custom allocators fail safely;
     - null-free no-op;
     - sys userdata/decode/log set and clear;
     - sys callback values are stored from the `value` pointer itself, not by
       dereferencing a callback slot;
     - invalid sys options;
     - `roastty_sys_log_stderr` at least through a smoke call that does not
       crash.
   - `roastty/tests/abi_harness.c` must assert enum discriminants, struct layout
     where stable, and basic runtime behavior from C.

5. Preserve naming and scope.
   - Public names must be `roastty_*`, not compatibility `ghostty_*` symbols.
   - Do not add a generic allocator abstraction to unrelated Roastty internals
     unless one of these APIs requires it directly.
   - Do not route existing allocated-string helpers through the new allocator in
     this experiment unless required for correctness. Existing string ownership
     behavior can remain as-is.

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs
cargo test -p roastty build_info
cargo test -p roastty support_allocator
cargo test -p roastty sys_c_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when C callers can query build information, allocate/free
byte buffers through the default and custom allocator paths, and install/clear
system callbacks without adding Kitty graphics behavior.

## Non-Negotiable Invariants

- Do not expose any `ghostty_*` ABI names.
- Do not implement Kitty graphics image storage, decoding, rendering, placement,
  or terminal command handling.
- Do not add PTY, app runtime, Swift frontend, or browser overlay behavior.
- Do not remove `roastty_info()` unless this experiment proves it conflicts with
  the upstream-shaped support ABI.
- Do not silently change ownership rules for existing string-returning APIs.
- Do not skip Codex design review. If the design review finds a real issue, fix
  it and re-review before committing this experiment design.
- Do not skip Codex result review after implementation.

## Result

**Result:** Pass

The experiment added the support C ABI surface with Roastty naming:

- `roastty_build_info`;
- `roastty_alloc` and `roastty_free`;
- `roastty_sys_set`;
- `roastty_sys_log_stderr`;
- support enums and structs for build info, allocators, sys callbacks, and sys
  images.

The build-info API returns deterministic current values. SIMD, Kitty graphics,
and tmux control mode report `false` because those support paths are not ported
yet. Optimize mode maps debug builds to `ROASTTY_OPTIMIZE_DEBUG` and non-debug
builds to `ROASTTY_OPTIMIZE_RELEASE_FAST`; Rust build metadata does not
distinguish release-safe/release-small in this slice. Version strings are
borrowed process-static `roastty_string_s` values and are documented as not
owned by the caller.

The allocator helpers support the default Rust global allocator for
`allocator == null`, custom vtable-backed allocation/free for non-null
allocators, zero-length allocation returning null, null-free no-op behavior, and
safe handling of malformed custom allocators. The sys callback API stores
process-global userdata, PNG decode callback, and log callback under
synchronized state. Kitty graphics decoding/storage was deliberately not added.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs
cargo test -p roastty build_info
cargo test -p roastty support_allocator
cargo test -p roastty sys_c_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Codex result review initially found only low-level coverage gaps for
`ROASTTY_BUILD_INFO_TMUX_CONTROL_MODE`, `ROASTTY_BUILD_INFO_VERSION_BUILD`, and
the custom allocator missing-`free` path. Those tests were added in both Rust
and the C harness. A second Codex review found no blocking issues and explicitly
approved recording Experiment 185 as Pass.

## Conclusion

Roastty now has the upstream-shaped support ABI needed by later library
subsystems: build metadata queries, byte-buffer allocation helpers, and
process-global sys callback registration/logging. The support substrate is in
place without implementing Kitty graphics, PTY/app runtime behavior, or any
browser overlay behavior.
