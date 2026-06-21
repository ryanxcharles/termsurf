# Experiment 5: Create initial libtermsurf_webkit ABI

## Description

Experiments 1-4 proved that WebKit builds locally, that WebKit content can be
hosted across a process boundary, that the hosted surface survives key lifecycle
events, and that WebKit source changes have a branch/patch workflow. The next
step is to turn the proof harness into the first production-shaped library
boundary: `libtermsurf_webkit`.

This experiment should create a buildable macOS `libtermsurf_webkit` C ABI
scaffold backed by Objective-C++/Cocoa. It should prove that a C caller can
initialize the library, create a browser context, create a WebKit-backed browser
view, receive the important callbacks, navigate, resize, destroy the view, and
shut down cleanly.

The scope is intentionally smaller than full Surfari. This experiment should not
create the Surfari Rust binary, modify Ghostboard, modify `termsurf.proto`,
implement every input path, or patch WebKit source. It should establish the
library shape and prove the first working browser-view lifecycle through the C
ABI.

## Changes

- Create a new tracked library directory, likely `surfari/libtermsurf_webkit/`,
  with:
  - a public C header declaring opaque `ts_browser_context_t` and
    `ts_web_contents_t` handles;
  - a public C ABI compatible with the current Roamium FFI names in
    `roamium/src/ffi.rs`;
  - an Objective-C++ implementation that owns Cocoa/WebKit objects behind opaque
    C handles;
  - a local build script for macOS development;
  - a smoke-test executable or harness that calls the C ABI directly.
- Link the library and smoke test against the locally source-built WebKit
  products under `webkit/src/WebKitBuild/Debug`, not accidentally against only
  `/System/Library/Frameworks/WebKit.framework`.
- Implement the initial working subset:
  - callback registration for `ts_set_on_initialized`, `ts_set_on_tab_ready`,
    `ts_set_on_ca_context_id`, `ts_set_on_url_changed`,
    `ts_set_on_loading_state`, and `ts_set_on_title_changed`;
  - exact export of `ts_content_main`, which initializes Cocoa on the main
    thread and fires the initialized callback;
  - `ts_post_task`;
  - `ts_quit`;
  - `ts_create_browser_context`;
  - `ts_create_incognito_browser_context`;
  - `ts_destroy_browser_context`;
  - `ts_create_web_contents`;
  - `ts_destroy_web_contents`;
  - `ts_load_url`;
  - `ts_set_view_size`.
- Export the remaining Roamium-compatible symbols as explicit unsupported stubs
  only if needed to make the ABI complete for a future Surfari link. Any
  unsupported stub must be documented in the experiment result and should not be
  claimed as implemented behavior.
- Reuse the proven compositor hook from the proof harness: create a `CAContext`,
  assign the `WKWebView` layer, and fire `ts_set_on_ca_context_id` with the
  exported context ID and current pixel size.
- Add deterministic local test content for the smoke test if the existing
  `surfari-proofs/hosting-context/test-content/` files are not suitable.
- Update `webkit/README.md` or a new Surfari README only if needed to document
  how to build the initial library.
- Do not modify `webkit/src` in this experiment. If the first ABI slice needs a
  WebKit source patch, record **Partial**, archive the reason, and design the
  next experiment around that patch.

## Verification

Start from a clean TermSurf repo root:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
find webkit/src/WebKitBuild/Debug -maxdepth 2 \
  \( -name 'WebKit.framework' -o -name 'JavaScriptCore.framework' \) -print
```

Then build the library and smoke test with the new documented command, expected
to be something like:

```bash
surfari/libtermsurf_webkit/build.sh
```

The build must produce a dynamic library named `libtermsurf_webkit.dylib` and a
smoke-test binary. Verify exported symbols:

```bash
nm -gU surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg ' _ts_'
```

Run the smoke test with logs in the repo `logs/` directory. The exact command
must be recorded in the result, but it should prove:

- the library initializes and fires `ts_set_on_initialized`;
- a persistent and an incognito browser context can be created and destroyed;
- a WebKit-backed web contents can be created through `ts_create_web_contents`;
- `ts_set_on_tab_ready` fires with a nonzero tab ID;
- `ts_set_on_ca_context_id` fires with a nonzero context ID and the expected
  size;
- `ts_set_on_loading_state` reports loading transitions;
- `ts_set_on_url_changed` reports the loaded URL;
- `ts_set_on_title_changed` reports the page title;
- `ts_load_url` can navigate from one deterministic local page to another;
- `ts_set_view_size` resizes the `WKWebView` and causes the exported context
  callback or smoke-test observation to reflect the new size;
- `ts_destroy_web_contents`, `ts_destroy_browser_context`, and `ts_quit` shut
  down cleanly;
- `webkit/src` remains clean and on `webkit-1452a439-issue-756`.

Use `otool` or an equivalent check to verify the library and smoke-test link
paths use the local WebKit build products under `webkit/src/WebKitBuild/Debug`:

```bash
otool -L surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg 'WebKit|JavaScriptCore'
otool -L surfari/libtermsurf_webkit/build/smoke-test | rg 'WebKit|JavaScriptCore|libtermsurf_webkit'
```

**Pass** = the library and smoke test build, the dynamic library exports the
expected `ts_*` symbols, the smoke test proves the initial WebKit browser-view
lifecycle through the C ABI, the binary links against the local source-built
WebKit products, unsupported stubs are explicitly documented, and `webkit/src`
remains unchanged.

**Partial** = the library builds but one or more required lifecycle callbacks,
source-built WebKit linkage checks, resize behavior, or shutdown behavior is
missing. The result must identify the exact missing behavior and the next
experiment needed.

**Fail** = the initial library cannot be built or cannot create a usable WebKit
view through the C ABI.

Before recording the result, capture:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

The TermSurf worktree must contain only the intended library, harness, docs, and
issue changes plus ignored `logs/` and build output.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

**Verdict:** Approved.

Required findings: none.

Optional findings accepted and fixed:

- The design originally allowed "`ts_content_main` or equivalent"; this was
  tightened to require the exact `ts_content_main` export for Roamium ABI
  compatibility.
- The local WebKit linkage check originally showed only the dylib; it now also
  checks the smoke-test binary.

## Result

**Result:** Pass

Experiment 5 created the first production-shaped `libtermsurf_webkit` C ABI
scaffold under `surfari/libtermsurf_webkit/`.

Implemented files:

- `surfari/libtermsurf_webkit/include/libtermsurf_webkit.h`
- `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`
- `surfari/libtermsurf_webkit/smoke-test/smoke_test.c`
- `surfari/libtermsurf_webkit/build.sh`
- `surfari/libtermsurf_webkit/README.md`
- `surfari/libtermsurf_webkit/test-content/index.html`
- `surfari/libtermsurf_webkit/test-content/navigation.html`
- `surfari/libtermsurf_webkit/.gitignore`

The library is backed by Objective-C++/Cocoa and uses the same Core Animation
export mechanism proven in Experiments 2 and 3:

- create a `WKWebView`;
- create a `CAContext`;
- assign the `WKWebView` layer to the context;
- fire `ts_set_on_ca_context_id` with the context ID and current size.

Implemented behavior:

- exact `ts_content_main` export;
- `ts_set_on_initialized`;
- `ts_post_task`;
- `ts_quit`;
- persistent and incognito browser context creation/destruction;
- `ts_create_web_contents`;
- `ts_destroy_web_contents`;
- `ts_load_url`;
- `ts_set_view_size`;
- callback registration and firing for tab ready, CA context ID, URL changes,
  loading state, and title changes.

Exported but unsupported stubs:

- `ts_create_devtools_web_contents`;
- mouse, scroll, and keyboard forwarding;
- focus, GUI-active, and color-scheme state;
- JavaScript dialog replies;
- HTTP auth replies;
- cursor, target URL, JavaScript dialog request, console message, HTTP auth
  request, and renderer crash callback registration.

These stubs exist so a future Surfari binary can link against the
Roamium-compatible ABI shape, but they are not claimed as implemented behavior
in this experiment.

Build command:

```bash
surfari/libtermsurf_webkit/build.sh
```

Output:

```text
ld: warning: building for macOS-26.0, but linking with dylib '/System/Library/Frameworks/WebKit.framework/Versions/A/WebKit' which was built for newer version 26.5
built surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib
built surfari/libtermsurf_webkit/build/smoke-test
```

The warning is emitted before `install_name_tool` rewrites the produced
`libtermsurf_webkit.dylib` dependency from WebKit's absolute framework install
name to an `@rpath` dependency. The produced dylib is what matters for runtime
verification.

Smoke-test command:

```bash
DYLD_FRAMEWORK_PATH="$PWD/webkit/src/WebKitBuild/Debug" \
surfari/libtermsurf_webkit/build/smoke-test \
  "$PWD/surfari/libtermsurf_webkit/test-content/index.html" \
  "$PWD/surfari/libtermsurf_webkit/test-content/navigation.html" \
  > logs/issue756-exp5-smoke.log 2>&1
rc=$?
echo "SMOKE_EXIT_STATUS=$rc" >> logs/issue756-exp5-smoke.log
```

`DYLD_FRAMEWORK_PATH` is required because the local debug `WebKit.framework`
depends on source-built transitive frameworks such as `JavaScriptCore`.

Smoke-test evidence from `logs/issue756-exp5-smoke.log`:

```text
CALLBACK initialized
CALLBACK tab_ready tab_id=1
CALLBACK ca_context_id context_id=2228940916 width=320 height=240
CALLBACK loading_state loading=1 url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/index.html
CALLBACK url_changed url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/index.html
CALLBACK url_changed url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/index.html
CALLBACK title_changed title=Surfari ABI First Page
CALLBACK loading_state loading=0 url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/index.html
CALLBACK ca_context_id context_id=2228940916 width=320 height=240
CALLBACK loading_state loading=1 url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html
CALLBACK url_changed url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html
CALLBACK url_changed url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html
CALLBACK title_changed title=Surfari ABI Navigation Page
CALLBACK loading_state loading=0 url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html
CALLBACK ca_context_id context_id=2228940916 width=320 height=240
CALLBACK ca_context_id context_id=2228940916 width=640 height=480
SMOKE_PASS initialized=1 tab_ready=1 ca_context=4 url=4 loading_started=2 loading_finished=2 title=2 navigations=2 resized=1
SMOKE_EXIT_STATUS=0
```

Exported symbol check:

```bash
nm -gU surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg ' _ts_' | sort
```

Output included every Roamium-compatible `ts_*` symbol declared in
`roamium/src/ffi.rs`, including:

```text
_ts_content_main
_ts_create_browser_context
_ts_create_devtools_web_contents
_ts_create_incognito_browser_context
_ts_create_web_contents
_ts_destroy_browser_context
_ts_destroy_web_contents
_ts_forward_key_event
_ts_forward_mouse_event
_ts_forward_mouse_move
_ts_forward_scroll_event
_ts_load_url
_ts_post_task
_ts_quit
_ts_reply_http_auth
_ts_reply_javascript_dialog
_ts_set_color_scheme
_ts_set_focus
_ts_set_gui_active
_ts_set_on_ca_context_id
_ts_set_on_console_message
_ts_set_on_cursor_changed
_ts_set_on_http_auth_request
_ts_set_on_initialized
_ts_set_on_javascript_dialog_request
_ts_set_on_loading_state
_ts_set_on_renderer_crashed
_ts_set_on_tab_ready
_ts_set_on_target_url_changed
_ts_set_on_title_changed
_ts_set_on_url_changed
_ts_set_view_size
```

Linkage check:

```bash
otool -L surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg 'WebKit|JavaScriptCore|libtermsurf'
otool -L surfari/libtermsurf_webkit/build/smoke-test | rg 'WebKit|JavaScriptCore|libtermsurf'
```

Output:

```text
surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib:
  @rpath/libtermsurf_webkit.dylib (compatibility version 0.0.0, current version 0.0.0)
  @rpath/WebKit.framework/Versions/A/WebKit (compatibility version 1.0.0, current version 625.1.21)
surfari/libtermsurf_webkit/build/smoke-test:
  @rpath/libtermsurf_webkit.dylib (compatibility version 0.0.0, current version 0.0.0)
```

Final checks:

```text
$ git diff --check
<no output>

$ git -C webkit/src status --short
<clean>

$ git -C webkit/src rev-parse HEAD
1452a43959523449099b2616793fd2c5b6a6487e

$ git -C webkit/src rev-parse --abbrev-ref HEAD
webkit-1452a439-issue-756

$ git -C webkit/src rev-parse --is-shallow-repository
true
```

## Conclusion

`libtermsurf_webkit` now exists as a buildable macOS C ABI scaffold backed by
Objective-C++/Cocoa and the local source-built WebKit products. The smoke test
proves the first browser-view lifecycle through the C ABI: initialize, create
contexts, create WebKit-backed web contents, export a CA context ID, navigate,
report URL/loading/title state, resize, destroy, and quit.

The next experiment should build the first Surfari Rust binary around this
library, reusing Roamium's socket/protobuf lifecycle where possible while
keeping unsupported browser features explicit.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Initial verdict:** Changes required.

Required finding:

- `surfari/libtermsurf_webkit/build.sh` should compute the repo root robustly
  before deriving `webkit/src/WebKitBuild/Debug`.

Fix:

- Updated the build script to use `git rev-parse --show-toplevel`, then derive
  `webkit_build="$repo_root/webkit/src/WebKitBuild/Debug"`.
- Reran the build and smoke test successfully.

**Re-review verdict:** Approved.

The reviewer confirmed the build script now targets the repo-root WebKit build
directory, `git diff --check` is clean, and the smoke log records
`SMOKE_PASS ... resized=1` plus `SMOKE_EXIT_STATUS=0`.
