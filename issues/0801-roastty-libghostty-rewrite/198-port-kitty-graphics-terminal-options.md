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

# Experiment 198: Port Kitty Graphics Terminal Options

## Description

Experiments 186-197 ported the Kitty graphics parser, direct image loading,
image/placement storage, display/delete execution, tracked placement ownership,
renderer-facing C handles, and placement geometry/render-info ABI. The remaining
Kitty graphics C ABI gap at the terminal boundary is configuration: upstream
exposes terminal options/data for image storage limits, allowed non-direct media
flags, and APC byte limits.

Roastty already has public terminal data selector numbers for:

- `ROASTTY_TERMINAL_DATA_KITTY_IMAGE_STORAGE_LIMIT = 26`
- `ROASTTY_TERMINAL_DATA_KITTY_IMAGE_MEDIUM_FILE = 27`
- `ROASTTY_TERMINAL_DATA_KITTY_IMAGE_MEDIUM_TEMP_FILE = 28`
- `ROASTTY_TERMINAL_DATA_KITTY_IMAGE_MEDIUM_SHARED_MEM = 29`

but `roastty_terminal_get(...)` still returns `ROASTTY_NO_VALUE` for them.
Roastty also still skips upstream terminal option selector numbers 15-20:

- `kitty_image_storage_limit = 15`
- `kitty_image_medium_file = 16`
- `kitty_image_medium_temp_file = 17`
- `kitty_image_medium_shared_mem = 18`
- `apc_max_bytes = 19`
- `apc_max_bytes_kitty = 20`

This experiment ports that coherent terminal configuration slice with Roastty
names. It does not implement file/temp-file/shared-memory image loading. The
media flags become stored and queryable now, and future non-direct-media
experiments will use them as their permission gates.

Use upstream source as the behavior reference:

- `vendor/ghostty/src/terminal/c/terminal.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_storage.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_image.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_command.zig`

## Changes

1. Add the missing public terminal option enum values in
   `roastty/include/roastty.h` and matching Rust constants in
   `roastty/src/lib.rs`.

   Add:

   ```c
   ROASTTY_TERMINAL_OPTION_KITTY_IMAGE_STORAGE_LIMIT = 15,
   ROASTTY_TERMINAL_OPTION_KITTY_IMAGE_MEDIUM_FILE = 16,
   ROASTTY_TERMINAL_OPTION_KITTY_IMAGE_MEDIUM_TEMP_FILE = 17,
   ROASTTY_TERMINAL_OPTION_KITTY_IMAGE_MEDIUM_SHARED_MEM = 18,
   ROASTTY_TERMINAL_OPTION_APC_MAX_BYTES = 19,
   ROASTTY_TERMINAL_OPTION_APC_MAX_BYTES_KITTY = 20,
   ```

   Preserve `ROASTTY_TERMINAL_OPTION_SELECTION = 21`.

2. Add terminal-level Kitty graphics config state in
   `roastty/src/terminal/terminal.rs`.

   The state should track:
   - current image storage limit (`usize`, default `DEFAULT_TOTAL_LIMIT` from
     `graphics_storage.rs`);
   - current allowed media flags (`LoadingImageLimits`, default
     `LoadingImageLimits::DIRECT`);
   - global APC max bytes override (`Option<usize>`);
   - Kitty-specific APC max bytes override (`Option<usize>`).

   Effective Kitty APC max bytes is:
   - Kitty-specific override, if set;
   - otherwise global APC override, if set;
   - otherwise `MAX_IMAGE_SIZE`.

   Store this at the terminal level, not only inside the active screen, so
   future screens created after configuration inherit the same Kitty graphics
   settings.

3. Apply Kitty graphics config to screens.

   Add narrow helpers on `TerminalScreens` and/or `Screen` so setting image
   storage limits and media flags applies to every initialized screen:
   - primary screen;
   - alternate screen if it already exists.

   When `TerminalScreens::ensure_alternate(...)` creates a new alternate screen,
   initialize the new screen's Kitty storage from the terminal-level config
   before it can process Kitty graphics commands.

   Setting the storage limit to zero disables Kitty graphics storage and clears
   stored images/placements on every initialized screen, preserving existing
   storage behavior. Setting it back to a nonzero value re-enables storage for
   future images.

   Reapply terminal-level Kitty graphics config after screen reset paths. This
   includes `Terminal::reset()`, RIS/full reset, and any screen reset path that
   rebuilds or reinitializes Kitty image storage. Configured storage limits and
   media flags are terminal options, not transient screen content. After reset:
   - `limit = 0` must remain disabled;
   - non-default nonzero limits must remain queryable;
   - non-default media flags must remain queryable;
   - images and placements may be cleared by reset, but the configured storage
     policy must survive.

4. Implement `roastty_terminal_set(...)` for the six new options.

   Behavior:
   - null terminal returns `ROASTTY_INVALID_VALUE`;
   - invalid option still returns `ROASTTY_INVALID_VALUE`;
   - storage limit option reads `const uint64_t*`; null means limit `0`,
     matching upstream's `?*const u64` semantics;
   - media options read `const bool*`; null means success/no mutation, matching
     upstream's `value orelse return .success` behavior;
   - APC max-byte options read `const size_t*`; null clears that override;
   - storage-limit application returns `ROASTTY_OUT_OF_MEMORY` if applying the
     new limit to screen storage fails.

   Do not use `u64` internally for storage sizes after reading the C value.
   Convert to `usize` with checked conversion. If the C value does not fit
   `usize`, return `ROASTTY_INVALID_VALUE`.

5. Implement `roastty_terminal_get(...)` for the four existing Kitty graphics
   data selectors.

   Behavior:
   - storage limit writes the current active screen's effective storage limit as
     `uint64_t`;
   - media flags write the current active screen's stored booleans;
   - null output remains `ROASTTY_INVALID_VALUE` through the existing validation
     path;
   - values must reflect the active screen, and active/alternate should match
     after options are applied through this ABI.

   If converting the active screen's storage limit from `usize` to `u64` ever
   fails, return `ROASTTY_INVALID_VALUE`.

6. Update Kitty APC parser max-byte behavior.

   Replace the test-only-only max-byte setter with production methods that can
   set/clear global and Kitty-specific limits. The existing test helper may
   remain as a convenience wrapper if it delegates to the production path.

   Verify precedence:
   - default uses `MAX_IMAGE_SIZE`;
   - global APC max affects Kitty APC parsing when Kitty-specific override is
     unset;
   - Kitty-specific max overrides the global value;
   - clearing the Kitty-specific override falls back to the global value;
   - clearing the global value falls back to `MAX_IMAGE_SIZE`.

7. Add Rust tests in `roastty/src/lib.rs` and
   `roastty/src/terminal/terminal.rs`.

   Cover:
   - terminal option numeric values 15-21;
   - default Kitty storage limit/media data getters;
   - setting/getting storage limit through C ABI;
   - limit `0` clears images/placements and suppresses future storage;
   - re-enabling storage allows future direct image storage;
   - media flags set/get through C ABI, including null no-op;
   - active/alternate screen propagation for storage limit and media flags;
   - future alternate screen inheritance after options were set before alternate
     creation;
   - configured storage limit and media flags survive `Terminal::reset()` and
     RIS/full reset, including `limit = 0` remaining disabled;
   - APC global and Kitty-specific max-byte precedence and clearing behavior;
   - over-limit Kitty APC is ignored using the configured effective limit;
   - oversized `uint64_t` storage limit returns `ROASTTY_INVALID_VALUE` on
     platforms where it does not fit `usize`;
   - invalid/null terminal and output validation.

8. Extend `roastty/tests/abi_harness.c`.

   Add C harness coverage for:
   - new option enum numeric values;
   - default Kitty storage limit/media data getters;
   - setting storage limit and reading it back;
   - setting media flags and reading them back;
   - null media option values are accepted as no-ops;
   - setting and clearing APC max byte options.

9. Preserve formatting and review rules.

   Run:

   ```bash
   cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs roastty/src/terminal/kitty/graphics_storage.rs roastty/src/terminal/kitty/graphics_image.rs
   prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/198-port-kitty-graphics-terminal-options.md
   ```

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs roastty/src/terminal/kitty/graphics_storage.rs roastty/src/terminal/kitty/graphics_image.rs
cargo test -p roastty kitty_graphics_terminal_options_c_abi
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty kitty_graphics_storage
cargo test -p roastty --test abi_harness
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when:

- the public terminal option enum includes selectors 15-20 with Roastty names;
- terminal set/get supports Kitty image storage limit and media flags;
- terminal get no longer returns `ROASTTY_NO_VALUE` for the four Kitty image
  data selectors;
- storage limit changes apply to primary, existing alternate, and future
  alternate screens;
- media flag changes apply to primary, existing alternate, and future alternate
  screens;
- storage limit and media flag terminal options survive reset/RIS even though
  images and placements are cleared;
- APC max-byte options affect Kitty APC parsing with the documented precedence;
- no non-direct image medium is loaded in this experiment;
- all existing Kitty graphics execution, storage, C ABI, and full Roastty tests
  still pass;
- Codex approves the experiment design before implementation and approves the
  result before the experiment is recorded.

## Non-Negotiable Invariants

- Do not implement file image loading.
- Do not implement temp-file image loading.
- Do not implement shared-memory image loading.
- Do not decode PNG.
- Do not render images.
- Do not add Metal or any platform renderer.
- Do not change Kitty direct image transmit/display/delete behavior except where
  storage limits or APC byte limits intentionally gate it.
- Do not expose any `ghostty_*` ABI names.
- Do not skip Codex design review or Codex result review.

## Result

**Result:** Pass

Implemented the missing Kitty graphics terminal option/data surface:

- added public terminal option selectors 15-20 in `roastty/include/roastty.h`
  and matching Rust constants;
- added terminal-level Kitty graphics configuration for storage limit, allowed
  non-direct media flags, global APC byte limit, and Kitty-specific APC byte
  limit;
- applied that configuration to the primary screen, existing alternate screen,
  future alternate screen creation, `Terminal::reset()`, and RIS/full reset;
- wired `roastty_terminal_set(...)` for storage/media/APC options with the
  upstream pointer semantics;
- wired `roastty_terminal_get(...)` so Kitty image storage/media data selectors
  no longer return `ROASTTY_NO_VALUE`;
- preserved the experiment non-goals: file, temp-file, shared-memory, PNG, and
  renderer support remain unimplemented.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs roastty/src/terminal/kitty/graphics_storage.rs roastty/src/terminal/kitty/graphics_image.rs
cargo test -p roastty kitty_graphics_terminal_options_c_abi
cargo test -p roastty terminal_stream_kitty_graphics
cargo test -p roastty kitty_graphics_storage
cargo test -p roastty --test abi_harness
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Codex reviewed the completed implementation and found no blocking issues. The
review specifically confirmed that the option selectors, `terminal_get`,
`terminal_set`, terminal-level config propagation, reset/RIS persistence, APC
precedence, Rust tests, and C ABI harness coverage satisfy this experiment.

## Conclusion

Roastty now exposes the Kitty graphics terminal configuration ABI slice. Direct
Kitty image storage remains the only loaded medium, but the storage limit,
non-direct media permission flags, and APC byte limits are now stored,
queryable, inherited across screens, and persistent across reset paths. Future
experiments can use the media flags as permission gates when they port
file-backed, temp-file, or shared-memory image loading.
