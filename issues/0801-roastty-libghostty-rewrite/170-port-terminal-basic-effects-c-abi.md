# Experiment 170: Port Terminal Basic Effects C ABI

## Description

Experiment 169 completed the terminal color set/get ABI. The next coherent
upstream terminal C ABI surface is the first half of terminal effects: callbacks
that let the embedder observe terminal output and answer simple terminal
queries.

This experiment ports only the basic effect callbacks that do not require new
structured query types:

- `userdata`;
- `write_pty`;
- `bell`;
- `enquiry`;
- `xtversion`;
- `title_changed`.

These callbacks are the foundation for later effects such as color-scheme
queries, device attributes, and size reports. Upstream Ghostty wires all effects
through the stream terminal handler. Roastty currently accumulates generated PTY
responses in an internal `pty_response` buffer and exposes that buffer through
`roastty_terminal_take_pty_response`. This experiment keeps that compatibility
path for tests and current embedders while adding callback delivery:

- if `write_pty` is set, response bytes are delivered to the callback;
- response bytes are still retained in the internal `pty_response` buffer;
- if `write_pty` is unset, existing buffered-response behavior is unchanged.

This dual path is intentional for this experiment. It lets the new C ABI match
upstream's callback shape without breaking existing Roastty tests and helper
APIs that already consume `roastty_terminal_take_pty_response`.

## Changes

### 1. Add public terminal callback types

In `roastty/include/roastty.h`, add Roastty-named typedefs:

```c
typedef void (*roastty_terminal_write_pty_cb)(roastty_terminal_t,
                                             void* userdata,
                                             const uint8_t* ptr,
                                             size_t len);

typedef void (*roastty_terminal_bell_cb)(roastty_terminal_t,
                                         void* userdata);

typedef roastty_string_s (*roastty_terminal_enquiry_cb)(roastty_terminal_t,
                                                       void* userdata);

typedef roastty_string_s (*roastty_terminal_xtversion_cb)(roastty_terminal_t,
                                                         void* userdata);

typedef void (*roastty_terminal_title_changed_cb)(roastty_terminal_t,
                                                 void* userdata);
```

The strings returned from `enquiry` and `xtversion` are borrowed callback
outputs. Roastty must copy or synchronously consume the bytes before the
callback returns. Roastty must not call `roastty_string_free` on
callback-returned strings.

### 2. Add implemented effect option constants

Extend `roastty_terminal_option_e` with upstream-compatible values:

```c
ROASTTY_TERMINAL_OPTION_USERDATA = 0,
ROASTTY_TERMINAL_OPTION_WRITE_PTY = 1,
ROASTTY_TERMINAL_OPTION_BELL = 2,
ROASTTY_TERMINAL_OPTION_ENQUIRY = 3,
ROASTTY_TERMINAL_OPTION_XTVERSION = 4,
ROASTTY_TERMINAL_OPTION_TITLE_CHANGED = 5,
```

Do not add `size_cb`, `color_scheme`, `device_attributes`, Kitty graphics, APC,
or selection options in this experiment.

### 3. Add terminal effects storage

Add a `TerminalEffects` storage object owned by the terminal handle wrapper in
`roastty/src/lib.rs`.

Required fields:

- `userdata: *mut c_void`;
- `write_pty: Option<extern "C" fn(...)>`;
- `bell: Option<extern "C" fn(...)>`;
- `enquiry: Option<extern "C" fn(...) -> RoasttyString>`;
- `xtversion: Option<extern "C" fn(...) -> RoasttyString>`;
- `title_changed: Option<extern "C" fn(...)>`.

Semantics:

- `USERDATA` stores the `value` pointer itself, not a pointer read from `value`;
- callback options store the callback pointer represented by `value`;
- passing `NULL` for a callback clears that callback;
- passing `NULL` for `USERDATA` clears userdata;
- null terminal returns `ROASTTY_INVALID_VALUE`;
- unsupported option constants remain `ROASTTY_INVALID_VALUE`.

Because Rust cannot portably validate an arbitrary non-null C function pointer,
the C ABI should treat a non-null callback value as caller-provided trusted FFI
input, matching the rest of the C callback surface.

### 4. Route generated PTY responses through effects

Change terminal response writing so generated PTY bytes go through a
terminal-handle-level effect path:

- existing response bytes continue to append to the internal `pty_response`
  buffer;
- if `write_pty` is set, call it synchronously with
  `(terminal_handle, userdata, ptr, len)`;
- if `write_pty` is unset, do nothing beyond the existing buffer append;
- preserve response order when one terminal action emits multiple chunks.

This can be implemented by adding a terminal wrapper method around
`InnerTerminal::next_slice` that copies/delivers newly-appended `pty_response`
bytes using an offset without removing them, or by passing an effects sink into
terminal stream handling that appends and calls the callback in the same write
path. Use the least invasive approach that keeps existing terminal unit tests
and the C ABI both correct.

The implementation must not use `roastty_terminal_take_pty_response` internally
to deliver callback bytes. Only the explicit public take API may clear the
buffer.

### 4.1 Callback reentrancy contract

The terminal handle is passed to callbacks for upstream ABI shape and userdata
identity, but callbacks are not reentrant entry points in this experiment.

During a callback, callers must not:

- call `roastty_terminal_free` on the same terminal;
- call `roastty_terminal_vt_write` on the same terminal;
- call `roastty_terminal_set` on the same terminal;
- call any API that mutates the same terminal.

Read-only queries that do not allocate or mutate terminal state may be supported
only when the implementation can do so without holding an active mutable borrow
across the callback. If this cannot be guaranteed cleanly, document callbacks as
non-reentrant and do not add read-during-callback tests.

Required implementation note: avoid creating a design that can accidentally
alias `&mut Terminal` while invoking user code. If callback delivery cannot be
implemented without such aliasing, stop and redesign the experiment instead of
using unsafe reentrant access.

### 5. Implement bell effect

Wire BEL (`0x07`) to the `bell` callback:

- with `bell` set, call it synchronously with `(terminal_handle, userdata)`;
- with `bell` unset, BEL remains silent;
- BEL must not mutate visible terminal content or dirty rows.

### 6. Implement enquiry effect

Wire ENQ (`0x05`) to the `enquiry` callback:

- with `enquiry` set, call it synchronously;
- if the returned `roastty_string_s` has a non-null pointer and nonzero length,
  write those bytes as PTY response bytes through the same response path as
  other terminal responses;
- empty returned strings produce no response;
- invalid callback-returned strings (`ptr == NULL && len > 0`) produce no
  response and must not crash;
- responses with `len >= 256` produce no response, matching upstream's fixed
  stack-buffer limit for ENQ;
- with `enquiry` unset, ENQ remains silent.

### 7. Implement xtversion effect

Wire XTVERSION (`CSI > q`) to the `xtversion` callback:

- with `xtversion` unset, keep Roastty's current default response:
  `ESC P >|libroastty ESC \`;
- with `xtversion` set and returning a nonempty valid string with `len <= 256`,
  respond with that string inside the existing XTVERSION DCS wrapper;
- with `xtversion` set but returning an empty string, `ptr == NULL`,
  `len > 256`, or `ptr == NULL && len > 0`, fall back to `libroastty`;
- public output must never say `ghostty` unless the callback explicitly returns
  that exact byte string.

### 8. Implement title-changed effect

Call `title_changed` when terminal title state changes through terminal stream
window-title handling:

- OSC title changes (`OSC 0`, `OSC 1`, `OSC 2`) trigger the callback after state
  is updated;
- `roastty_terminal_set(TITLE, ...)` does not trigger `title_changed`, matching
  upstream `terminal_set(.title)` behavior;
- setting the same title again may still trigger the callback, matching the
  "title was set" event shape rather than requiring value-diff tracking;
- with `title_changed` unset, title changes remain silent.

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/stream.rs
cargo test -p roastty terminal_basic_effects_abi
cargo test -p roastty terminal_metadata_setters_abi
cargo test -p roastty terminal_get_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty terminal_stream
cargo test -p roastty
! rg -n "ghostty|Ghostty|ghostty_" roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c
```

Required evidence:

- effect option discriminants `0..5` are stable;
- `USERDATA` passes through to every callback;
- callback options can be set, cleared with `NULL`, and set again;
- `write_pty` receives generated response bytes in order;
- `write_pty` delivery uses a copy/offset path and does not clear the internal
  `pty_response` buffer;
- existing `roastty_terminal_take_pty_response` behavior still works;
- BEL calls `bell` only when configured;
- ENQ calls `enquiry` only when configured and routes nonempty responses to PTY
  output;
- ENQ ignores empty, invalid, and `len >= 256` callback-returned strings;
- XTVERSION uses callback output when configured and `libroastty` otherwise;
- XTVERSION falls back to `libroastty` for empty, invalid, and `len > 256`
  callback-returned strings;
- title changes from OSC call `title_changed` after state is updated;
- direct `roastty_terminal_set(TITLE, ...)` does not call `title_changed`;
- C harness explicitly sets every callback from C, verifies userdata delivery,
  verifies `write_pty` byte order and callback bytes during the call, verifies
  ENQ/XTVERSION struct-return callbacks, verifies callback clearing, and
  verifies unsupported option rejection;
- no unrelated callback options or subsystem constants are exposed.

## Non-Negotiable Invariants

- Use Roastty names only for public ABI and implementation-facing text.
- Preserve upstream option discriminants for implemented options.
- Do not expose `size_cb`, `color_scheme`, `device_attributes`, Kitty graphics,
  APC, or selection options in this experiment.
- Do not remove `roastty_terminal_take_pty_response`.
- Do not make the internal buffered response path depend on a callback being
  installed.
- Do not clear the internal buffered response path while delivering callback
  bytes.
- Do not allow callback implementation to rely on reentrant mutable access to
  the same terminal.
- Do not retain callback-returned enquiry or xtversion string pointers after the
  callback returns.
- Do not call `roastty_string_free` on callback-returned strings.
- Do not implement renderer, PTY/process spawning, resize, selection, graphics,
  font, IME, Swift frontend, browser, or non-macOS behavior.

## Failure Criteria

This experiment fails if:

- any implemented option value differs from upstream;
- `USERDATA` is interpreted as a pointer-to-pointer instead of the pointer value
  itself;
- callback clear with `NULL` does not work;
- generated response bytes are lost from `roastty_terminal_take_pty_response`;
- generated response byte order changes;
- callback delivery clears or consumes `pty_response`;
- ENQ or XTVERSION retains borrowed callback memory;
- ENQ or XTVERSION accepts callback-returned strings outside the documented
  length/null-pointer rules;
- title callbacks fire for direct `roastty_terminal_set(TITLE, ...)`;
- title callbacks fire before terminal title state is updated;
- the C harness does not exercise every new public callback ABI path listed in
  Verification;
- unrelated terminal option constants are exposed;
- the design or result proceeds without the required Codex review gate.
