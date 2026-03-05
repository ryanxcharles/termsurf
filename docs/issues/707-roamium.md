# Issue 707: Roamium — Rust reimplementation of Plusium

## Goal

Rewrite Plusium in Rust. The new binary, Roamium, must be 100% compatible with
Plusium — same IPC protocol, same C API calls, same behavior. The GUI should not
be able to tell the difference.

## Background

### What Plusium is

Plusium (`content/plusium/plusium_main.cc`) is a ~500-line C++ binary that wraps
Chromium's Content API through `libtermsurf_content`, a C library. It does three
things:

1. **Connects to the GUI** via Unix domain socket (`--ipc-socket=` flag)
2. **Reads protobuf messages** (length-prefixed, LE u32 + payload) and
   dispatches them to the C API
3. **Sends protobuf responses** back when Chromium fires callbacks (tab ready,
   URL changed, etc.)

The C API (`libtermsurf_content.h`) exports ~20 functions with simple C types:
`int`, `const char*`, `void*`, `bool`, `uint32_t`. No C++ types cross the
boundary.

### Why Rust

- The TUI is already Rust. Roamium reuses the same toolchain, proto definitions,
  and socket framing patterns.
- `prost` (already a TUI dependency) handles protobuf. `std::os::unix::net`
  handles sockets. FFI to the C API is trivial.
- Rust's ownership model prevents the class of bugs that caused Issue 706 (void
  pointer corruption across async boundaries).

### What needs to be reimplemented

Every feature of `plusium_main.cc` (511 lines):

**Argument parsing** — Extract `--ipc-socket=` and `--user-data-dir=` from argv.
Derive profile name from the basename of the user data dir path.

**Tab registry** — A `Vec<TabEntry>` holding `handle` (void pointer from C API),
`tab_id`, `pane_id`, `inspected_tab_id`, and `last_url` for each tab. Lookup by
handle and by tab_id.

**Socket connection** — On initialized callback, connect to the GUI's Unix
socket, send `ServerRegister` with the profile name, spawn a reader thread.

**Socket reader loop** — Read from socket into a buffer, extract length-prefixed
protobuf messages, parse with prost, post each to the UI thread via
`ts_post_task`.

**Message dispatch** — Handle 12 incoming message types:

| Message             | Action                                             |
| ------------------- | -------------------------------------------------- |
| `CreateTab`         | Push entry, call `ts_create_web_contents`          |
| `CreateDevtoolsTab` | Push entry, call `ts_create_devtools_web_contents` |
| `Resize`            | `ts_set_view_size`                                 |
| `CloseTab`          | `ts_destroy_web_contents`, remove entry            |
| `Navigate`          | `ts_load_url`                                      |
| `MouseEvent`        | `ts_forward_mouse_event`                           |
| `MouseMove`         | `ts_forward_mouse_move`                            |
| `ScrollEvent`       | `ts_forward_scroll_event`                          |
| `KeyEvent`          | `ts_forward_key_event`                             |
| `FocusChanged`      | `ts_set_focus`                                     |
| `SetColorScheme`    | `ts_set_color_scheme`                              |
| `QueryTabsRequest`  | Count tabs, build reply, send                      |

**Callbacks** — 6 C callbacks registered before `ts_content_main`:

| Callback          | Sends           |
| ----------------- | --------------- |
| `OnTabReady`      | `TabReady`      |
| `OnCaContextId`   | `CaContext`     |
| `OnUrlChanged`    | `UrlChanged`    |
| `OnLoadingState`  | `LoadingState`  |
| `OnTitleChanged`  | `TitleChanged`  |
| `OnCursorChanged` | `CursorChanged` |

**String-to-int mappings** — Mouse type (`down`/`up` → 0/1), mouse button
(`left`/`right`/`middle` → 0/1/2), key type (`down`/`up`/`repeat` → 0/1/2).

**Shutdown** — When the last tab is closed, call `ts_quit()`.

### C API surface

The full API from `libtermsurf_content.h` (20 functions):

```c
// Lifecycle
int ts_content_main(int argc, const char** argv);
void ts_set_on_initialized(void (*cb)(void*), void*);
void ts_post_task(void (*task)(void*), void*);
void ts_quit(void);

// Profiles
ts_browser_context_t ts_create_browser_context(const char* path);
void ts_destroy_browser_context(ts_browser_context_t ctx);

// Tabs
ts_web_contents_t ts_create_web_contents(ctx, url, w, h, dark);
ts_web_contents_t ts_create_devtools_web_contents(ctx, tab_id, w, h, dark);
void ts_destroy_web_contents(ts_web_contents_t wc);

// Navigation
void ts_load_url(ts_web_contents_t wc, const char* url);

// Input
void ts_forward_mouse_event(wc, type, button, x, y, click_count, mods);
void ts_forward_mouse_move(wc, x, y, mods);
void ts_forward_scroll_event(wc, x, y, dx, dy, phase, momentum, precise, mods);
void ts_forward_key_event(wc, type, keycode, utf8, mods);

// State
void ts_set_focus(ts_web_contents_t wc, bool focused);
void ts_set_color_scheme(ts_web_contents_t wc, bool dark);
void ts_set_view_size(ts_web_contents_t wc, int w, int h);

// Callbacks (6 setters, each takes fn pointer + user_data)
void ts_set_on_tab_ready(...);
void ts_set_on_ca_context_id(...);
void ts_set_on_url_changed(...);
void ts_set_on_loading_state(...);
void ts_set_on_title_changed(...);
void ts_set_on_cursor_changed(...);
```

Handles (`ts_web_contents_t`, `ts_browser_context_t`) are `void*`. Roamium
stores them as `*mut c_void` and passes them back verbatim — never dereferences
them.

### Existing Rust patterns (from TUI)

The TUI (`tui/src/ipc.rs`) already has:

- **prost** for protobuf (v0.14, with `prost-build` for codegen)
- **`build.rs`** that compiles `../proto/termsurf.proto`
- **Length-prefixed framing**: 4-byte LE u32 + payload, same as Plusium
- **Reader thread**: `std::os::unix::net::UnixStream`, buffered reads, frame
  extraction
- **Message dispatch**: `match` on `msg.msg`

Roamium reuses the same proto file and framing code. The main difference is
direction: the TUI is a client that sends requests, while Roamium is a server
that receives commands and sends events.

### Build considerations

Plusium is built inside Chromium's GN build system because it links against
`libtermsurf_content` (a static library) and `content_shell_lib` (Chromium
internals). Roamium needs the same linkage.

Options:

1. **Build Roamium with Cargo, link Chromium dylibs.** Since
   `is_component_build = true`, `libtermsurf_content`'s symbols end up in shared
   libraries (`libcontent.dylib`, etc.). Roamium's `build.rs` would point
   `rustc` at `chromium/src/out/Default/` for `-L` and `-l` flags.
2. **Build Roamium from GN.** Add a GN target that invokes `cargo build` and
   links the result. More complex but integrates into the existing build.
3. **Build a small C shim.** A tiny `roamium_main.c` that calls
   `ts_content_main()` (which Chromium needs for process setup), but delegates
   all logic to a Rust library linked in. This sidesteps the question of how
   Rust calls `ts_content_main` — the C shim handles Chromium bootstrap, and the
   Rust code handles everything else.

The biggest question is `ts_content_main(argc, argv)`. This function enters
Chromium's message loop and never returns (until shutdown). Plusium calls it
from `main()`. Roamium needs to do the same, but from Rust's `main()`. This is
straightforward FFI — Rust calls the `extern "C"` function and blocks.

### Key files

- `content/plusium/plusium_main.cc` — The C++ original (511 lines)
- `content/libtermsurf_content/libtermsurf_content.h` — The C API (168 lines)
- `proto/termsurf.proto` — Protobuf message definitions
- `tui/src/ipc.rs` — TUI's socket + protobuf patterns (reference)
- `tui/Cargo.toml` — TUI's dependencies (prost, etc.)
- `tui/build.rs` — TUI's proto codegen
- `content/plusium/BUILD.gn` — Plusium's GN build config
- `content/libtermsurf_content/BUILD.gn` — Library's GN build config

## Ideas for experiments

1. **Make libtermsurf_content a shared library.** Change `static_library` to
   `shared_library` in its `BUILD.gn`. Update Plusium's `BUILD.gn` to depend
   only on the dylib (remove direct `content_shell_app`/`content_shell_lib`
   deps). Verify Plusium still works. This is a prerequisite for Roamium — it
   proves the C library is a proper abstraction boundary, not just a source
   grouping.

2. **Standalone Rust binary with FFI bindings.** Create `roamium/` at the repo
   root (sibling to `gui/` and `tui/`). Write FFI bindings to
   `libtermsurf_content.h` (hand-written, ~20 `extern "C"` declarations). Reuse
   prost + the same proto file. Implement the full message loop. Build with
   Cargo, link `libtermsurf_content.dylib`. Test by swapping `--browser plusium`
   for `--browser roamium`.

3. **Shared proto crate.** Extract the proto compilation into a shared crate
   (`termsurf-proto/`) that both Roamium and the TUI depend on, eliminating
   duplicate `build.rs` codegen.

## Experiments

### Experiment 1: Make libtermsurf_content a shared library

Plusium currently links 430+ Chromium dylibs directly, even though it only calls
20 `ts_*` functions from `libtermsurf_content`. The C library was designed as
the abstraction boundary, but GN builds it as a `static_library` — the `.o`
files get baked into the `plusium` binary, and all of Chromium's transitive
dependencies leak through.

Change `libtermsurf_content` to a `shared_library` so it becomes a proper
boundary. Plusium (and later Roamium) links one dylib instead of 430.

#### What to change

**`content/libtermsurf_content/BUILD.gn`** — Change `static_library` to
`shared_library`.

**`content/plusium/BUILD.gn`** — Remove `//content/shell:content_shell_app` and
`//content/shell:content_shell_lib` from Plusium's `deps`. These are
`libtermsurf_content`'s internal dependencies — Plusium should not need them.
Keep only `:termsurf_proto` and `//content/libtermsurf_content`.

#### Verification

1. `autoninja -C out/Default plusium` — builds clean.
2. `ls -la out/Default/libtermsurf_content.dylib` — the dylib exists.
3. `otool -L out/Default/plusium` — Plusium links `libtermsurf_content.dylib`,
   not 430 Chromium dylibs directly.
4. `nm -gU out/Default/libtermsurf_content.dylib | grep ts_` — all 20 `ts_*`
   symbols are exported.
5. `web google.com --browser plusium` — browse, navigate, DevTools — all
   working.

#### Result
