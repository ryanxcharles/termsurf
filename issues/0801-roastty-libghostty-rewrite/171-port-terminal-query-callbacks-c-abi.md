# Experiment 171: Port Terminal Query Callback C ABI

## Description

Experiment 170 added the first host-effect callbacks (`write_pty`, `bell`,
`enquiry`, `xtversion`, and `title_changed`). The next coherent terminal ABI
slice is the remaining callback-backed terminal query surface that does not
require PTY process management, renderer state, Kitty graphics storage, APC
limits, selection C conversion, or terminal resize/reflow.

Port:

- `size_cb` option `6`;
- `color_scheme` option `7`;
- `device_attributes` option `8`;
- the public size-report value types and encoder needed by `size_cb`;
- CSI query handling that consumes those callbacks:
  - XTWINOPS size reports: `CSI 14 t`, `CSI 16 t`, `CSI 18 t`;
  - window-title report: `CSI 21 t`;
  - color scheme query: `CSI ? 996 n`;
  - device attributes: `CSI c`, `CSI > c`, `CSI = c`.

Do not implement `roastty_terminal_resize` in this experiment. Upstream resize
also mutates screen/page geometry, disables synchronized output, and may emit
in-band size reports when mode `2048` is active. Roastty does not yet have the
terminal resize/reflow layer ported, so resize belongs in a later coherent
resize experiment.

Upstream reference points:

- `vendor/ghostty/src/terminal/c/terminal.zig`:
  - options `size_cb = 6`, `color_scheme = 7`, `device_attributes = 8`;
  - callback trampolines;
  - `DeviceAttributes` C struct;
- `vendor/ghostty/src/terminal/stream_terminal.zig`:
  - `reportSize`;
  - `deviceStatus` color-scheme query;
  - `reportDeviceAttributes`;
- `vendor/ghostty/src/terminal/size_report.zig` and
  `vendor/ghostty/src/terminal/c/size_report.zig`;
- `vendor/ghostty/include/ghostty/vt/size_report.h`;
- `vendor/ghostty/include/ghostty/vt/device.h`.

## Changes

### 1. Add public size-report ABI types and encoder

In `roastty/include/roastty.h`, add Roastty-named equivalents of upstream's
size-report public ABI:

```c
typedef enum {
  ROASTTY_SIZE_REPORT_MODE_2048 = 0,
  ROASTTY_SIZE_REPORT_CSI_14_T = 1,
  ROASTTY_SIZE_REPORT_CSI_16_T = 2,
  ROASTTY_SIZE_REPORT_CSI_18_T = 3,
} roastty_size_report_style_e;

typedef struct {
  uint16_t rows;
  uint16_t columns;
  uint32_t cell_width;
  uint32_t cell_height;
} roastty_size_report_size_s;

ROASTTY_API roastty_result_e roastty_size_report_encode(
    roastty_size_report_style_e style,
    roastty_size_report_size_s size,
    char* buf,
    size_t buf_len,
    size_t* out_written);
```

Implement the encoder in Rust with upstream-compatible output:

- `MODE_2048`: `ESC [ 48 ; rows ; columns ; height_px ; width_px t`;
- `CSI_14_T`: `ESC [ 4 ; height_px ; width_px t`;
- `CSI_16_T`: `ESC [ 6 ; cell_height ; cell_width t`;
- `CSI_18_T`: `ESC [ 8 ; rows ; columns t`.

`height_px = rows * cell_height` and `width_px = columns * cell_width`, using
wide arithmetic and saturating or checked conversion so multiplication cannot
panic in release or debug builds.

Encoder behavior:

- null `out_written` returns `ROASTTY_INVALID_VALUE`;
- unknown style returns `ROASTTY_INVALID_VALUE` and writes `0` to `out_written`;
- if `buf` is null or `buf_len` is too small, return `ROASTTY_OUT_OF_SPACE` and
  write the required size to `out_written`;
- on success, copy exactly the encoded bytes, write the byte count to
  `out_written`, and do not append a NUL byte.

### 2. Add public device-attributes ABI types

In `roastty/include/roastty.h`, add Roastty-named equivalents of upstream's
device attributes structs:

```c
typedef struct {
  uint16_t conformance_level;
  uint16_t features[64];
  size_t num_features;
} roastty_device_attributes_primary_s;

typedef struct {
  uint16_t device_type;
  uint16_t firmware_version;
  uint16_t rom_cartridge;
} roastty_device_attributes_secondary_s;

typedef struct {
  uint32_t unit_id;
} roastty_device_attributes_tertiary_s;

typedef struct {
  roastty_device_attributes_primary_s primary;
  roastty_device_attributes_secondary_s secondary;
  roastty_device_attributes_tertiary_s tertiary;
} roastty_device_attributes_s;
```

The Rust implementation should convert these numeric C values into the existing
`terminal::device_attributes::Attributes` representation. Numeric values should
be trusted as C ABI input and encoded back as numeric VT response parameters;
unknown feature/device/conformance values must not panic. If the current
strongly typed internal enums cannot represent arbitrary numeric values, widen
the internal device-attributes representation for this experiment so callbacks
can round-trip numeric C values.

Clamp `num_features` to `64` when reading callback output.

This experiment deliberately preserves Roastty's current no-callback DA defaults
while adding the callback surface. Upstream Ghostty's C trampoline returns
zero-valued attributes when the callback is absent or returns `false`, and
upstream stream handling only reports DA through the effect callback. Roastty
already has nonempty default DA responses from earlier experiments, so this
experiment keeps those defaults as a staged compatibility choice rather than
claiming exact upstream false-callback behavior.

The required callback-`false` fallback bytes are therefore the same as the
current no-callback defaults:

- primary: `ESC [ ? 62 ; 22 c`;
- secondary: `ESC [ > 1 ; 0 ; 0 c`;
- tertiary: `ESC P ! | 00000000 ESC \`.

### 3. Add callback typedefs and option constants

In `roastty/include/roastty.h`, add:

```c
typedef bool (*roastty_terminal_size_cb)(
    roastty_terminal_t terminal,
    void* userdata,
    roastty_size_report_size_s* out_size);

typedef bool (*roastty_terminal_color_scheme_cb)(
    roastty_terminal_t terminal,
    void* userdata,
    roastty_color_scheme_e* out_scheme);

typedef bool (*roastty_terminal_device_attributes_cb)(
    roastty_terminal_t terminal,
    void* userdata,
    roastty_device_attributes_s* out_attrs);
```

Expose option constants:

```c
ROASTTY_TERMINAL_OPTION_SIZE_CB = 6,
ROASTTY_TERMINAL_OPTION_COLOR_SCHEME = 7,
ROASTTY_TERMINAL_OPTION_DEVICE_ATTRIBUTES = 8,
```

`roastty_terminal_set` behavior:

- `NULL` callback pointer clears the callback;
- non-null callback pointer stores the function pointer;
- `USERDATA` from option `0` is passed unchanged to all three callbacks;
- callbacks receive the same public `roastty_terminal_t` handle used by
  Experiment 170.

Callback safety contract:

- The callback handle is observational only while the callback is running.
- Callbacks must not free the terminal or mutate the same terminal through
  `roastty_terminal_set`, `roastty_terminal_stream`, or similar APIs
  reentrantly.
- The Rust implementation must not invoke user callback code while holding an
  aliased mutable borrow of the same `Terminal`. Take a copy of the callback
  function pointer/userdata/handle first, release the borrow needed for lookup,
  call user code, then re-enter terminal mutation only after the callback
  returns.
- This is the same non-reentrant callback contract established by Experiment
  170, restated here because these callbacks are also invoked from stream
  mutation paths.

### 4. Wire device attributes callback

Change terminal device attribute responses so:

- with no callback installed, current default responses remain unchanged:
  - primary: `ESC [ ? 62 ; 22 c`;
  - secondary: `ESC [ > 1 ; 0 ; 0 c`;
  - tertiary: `ESC P ! | 00000000 ESC \`;
- with a callback installed and returning `true`, encode the returned attributes
  for the requested DA level;
- with a callback installed and returning `false`, fall back to the default
  Roastty attributes listed above;
- `num_features > 64` is clamped to `64`;
- the callback is called once per DA query.

Do not add renderer, app, PTY, selection, or Kitty graphics behavior.

### 5. Wire color-scheme callback

Change `CSI ? 996 n` handling so:

- with no callback installed, no response is written, preserving current Roastty
  behavior;
- with a callback installed and returning `false`, no response is written;
- with a callback installed and returning `true`:
  - `ROASTTY_COLOR_SCHEME_DARK` writes `ESC [ ? 997 ; 1 n`;
  - `ROASTTY_COLOR_SCHEME_LIGHT` writes `ESC [ ? 997 ; 2 n`;
- invalid callback-written enum values produce no response and do not panic;
- the response goes through the existing PTY response path and therefore also
  triggers `write_pty` if installed.

Lock the public color-scheme discriminants to upstream's ABI values:

- `ROASTTY_COLOR_SCHEME_LIGHT = 0`;
- `ROASTTY_COLOR_SCHEME_DARK = 1`.

Do not make `roastty_app_set_color_scheme` or `roastty_surface_set_color_scheme`
implicitly feed this terminal callback in this experiment. Those are app/surface
integration behaviors, not terminal-core callback ABI.

### 6. Wire size reports

Add stream parsing and terminal handling for XTWINOPS size reports:

- `CSI 14 t` requests text area size in pixels;
- `CSI 16 t` requests cell size in pixels;
- `CSI 18 t` requests text area size in characters;
- `CSI 21 t` requests window title.

Behavior:

- `CSI 14 t`, `CSI 16 t`, and `CSI 18 t` call `size_cb` when installed;
- if `size_cb` is not installed or returns `false`, no response is written;
- if `size_cb` returns `true`, encode the requested report style using the new
  size-report encoder and write it through the PTY response path;
- invalid or zero size fields are encoded as provided unless doing so would
  panic or overflow; the encoder must remain total;
- `CSI 21 t` does not call `size_cb`; it reports the terminal title using
  upstream's OSC-title response shape: `ESC ] l {title} ESC \`;
- extra parameters, private prefixes, or intermediates for these CSI `t` forms
  remain ignored.

Do not implement `CSI 14 ; ... t` window operations, window manipulation, native
app resizing, or terminal resize/reflow.

### 7. Tests

Add Rust tests for:

- size-report encoder value layout and option values;
- size-report output for all four styles;
- out-of-space/null-buffer behavior;
- invalid style/null `out_written` behavior;
- callback setting/clearing for options `6..8`;
- userdata and terminal handle delivery for all three callbacks;
- DA default responses remain unchanged with no callback;
- DA callback custom primary/secondary/tertiary responses;
- DA callback `false` fallback behavior;
- DA `num_features > 64` clamps without panic;
- DA callback unknown numeric round-trip behavior:
  - unknown primary conformance level;
  - unknown primary feature codes;
  - unknown secondary device type;
  - all encoded without panic and without silently remapping to a known enum;
- color-scheme enum discriminants: light is `0`, dark is `1`;
- color-scheme query no-op without callback, false-return no-op, dark response,
  and light response;
- invalid callback-written color-scheme value no-op;
- size query no-op without callback, false-return no-op, and all three size
  query responses with callback;
- `CSI 21 t` reports title and does not call `size_cb`;
- ignored CSI `t` forms do not write responses and do not call `size_cb`,
  including representative private-prefix, intermediate, and extra-parameter
  variants;
- all callback-generated responses continue to use the same PTY response path as
  Experiment 170.

Extend `roastty/tests/abi_harness.c` to compile and execute the new public C
types and callbacks:

- assert option constants `6..8`;
- assert `roastty_size_report_size_s` and device-attributes struct fields are
  writable from C;
- call `roastty_size_report_encode` from C, including out-of-space behavior;
- set and clear `size_cb`, `color_scheme`, and `device_attributes`;
- verify userdata and terminal handle delivery;
- verify representative PTY response bytes from each callback family;
- verify `ROASTTY_COLOR_SCHEME_LIGHT == 0` and `ROASTTY_COLOR_SCHEME_DARK == 1`;
- verify DA callback unknown numeric values encode without panic;
- verify dark maps to `ESC [ ? 997 ; 1 n` and light maps to `ESC [ ? 997 ; 2 n`;
- verify callback clearing.

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/stream.rs
cargo test -p roastty terminal_query_callbacks_abi
cargo test -p roastty size_report
cargo test -p roastty terminal_basic_effects_abi
cargo test -p roastty terminal_color_set_get_abi
cargo test -p roastty terminal_get_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty terminal_stream
cargo test -p roastty
! rg -n "ghostty|Ghostty|ghostty_" roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c
```

Required evidence:

- public ABI constants and structs match the upstream value/layout intent under
  Roastty names;
- all query callbacks can be set, cleared, and called from both Rust and C;
- `USERDATA` and terminal handles are delivered unchanged;
- generated callback responses are retained in
  `roastty_terminal_take_pty_response`;
- generated callback responses also invoke `write_pty` when installed;
- default DA responses remain unchanged when no callback is installed;
- callback-`false` DA responses use the explicitly documented staged Roastty
  default fallback, not an accidental zero-value upstream fallback;
- unknown numeric DA callback values round-trip into VT response bytes without
  panic;
- public color-scheme enum values are locked to light `0`, dark `1`;
- no-response cases really write no bytes;
- size report encoding is deterministic and handles insufficient buffers;
- no `ghostty_*` public ABI names are introduced.

## Non-Negotiable Invariants

- Use Roastty names only for public ABI and implementation-facing text.
- Preserve upstream option discriminants for implemented options.
- Do not expose or implement Kitty graphics options `15..18`, APC options
  `19..20`, or selection option `21`.
- Do not implement terminal resize/reflow, `roastty_terminal_resize`, PTY
  process resizing, in-band resize reports, app/surface color-scheme plumbing,
  renderer callbacks, selection, Kitty graphics, APC, font, IME, Swift frontend,
  browser, or non-macOS behavior.
- Do not clear or bypass the existing buffered PTY response path.
- Do not make callback invocation rely on reentrant mutable access to the same
  terminal.
- Do not proceed to implementation until Codex approves this experiment design.

## Failure Criteria

This experiment fails if:

- option values `6`, `7`, or `8` differ from upstream;
- C ABI structs have unusable field order or missing fields;
- callback clear with `NULL` does not work;
- callback responses bypass `write_pty` or `take_pty_response`;
- default DA responses regress;
- no-response query cases write bytes;
- size report encoding appends a NUL byte or reports the wrong byte count;
- out-of-space handling does not report the required size;
- invalid callback values panic;
- `CSI 21 t` calls `size_cb`;
- terminal resize/reflow or app/surface integration is added in this experiment;
- public `ghostty_*` ABI names are introduced;
- the design or result proceeds without the required Codex review gate.

## Result

**Result:** Pass

Implemented the terminal query callback C ABI slice:

- added public option constants `6..8`;
- added public size-report, color-scheme, and device-attributes C ABI types;
- added `roastty_size_report_encode`;
- wired `size_cb`, `color_scheme`, and `device_attributes` callbacks through
  `roastty_terminal_set`;
- added terminal handling for `CSI 14 t`, `CSI 16 t`, `CSI 18 t`, `CSI 21 t`,
  `CSI ? 996 n`, and DA primary/secondary/tertiary queries;
- widened the internal DA response representation so unknown numeric callback
  values round-trip into VT response bytes without panicking;
- preserved current Roastty no-callback DA defaults and callback-`false`
  fallback behavior;
- extended Rust tests and the C harness to cover the new ABI.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/stream.rs roastty/src/terminal/device_attributes.rs roastty/src/terminal/size_report.rs roastty/src/terminal/mod.rs
cargo test -p roastty terminal_query_callbacks_abi
cargo test -p roastty size_report
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty terminal_basic_effects_abi
cargo test -p roastty terminal_color_set_get_abi
cargo test -p roastty terminal_get_abi
cargo test -p roastty terminal_stream
cargo test -p roastty
! rg -n "ghostty|Ghostty|ghostty_" roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c
git diff --check
```

Codex reviewed the completed implementation and approved it with no blocking
findings:

- review log: `logs/codex-review/20260601-172016-034602-last-message.md`
- conclusion: "Experiment 171 is approved; the result is ready to record as
  `Pass` and commit."

## Conclusion

Roastty now has the remaining pure terminal query callback ABI that can be
implemented before resize/reflow, app/surface integration, renderer callbacks,
selection, APC, or Kitty graphics. The next experiment should continue from the
terminal ABI inventory and choose the next coherent callback or terminal-control
slice without crossing into resize/reflow unless that whole subsystem is ready
to be designed.
