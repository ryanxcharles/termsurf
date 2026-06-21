# libtermsurf_webkit

`libtermsurf_webkit` is the macOS WebKit C ABI for Surfari.

This directory currently contains the Issue 756 Experiment 5 scaffold: a
buildable dynamic library, public C header, and C smoke test that exercise the
initial WebKit browser-view lifecycle through `ts_*` functions compatible with
Roamium's Rust FFI shape.

## Build

Build WebKit first:

```bash
webkit/src/Tools/Scripts/build-webkit --debug
```

Then build the library and smoke test:

```bash
surfari/libtermsurf_webkit/build.sh
```

Outputs:

```text
surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib
surfari/libtermsurf_webkit/build/smoke-test
```

## Smoke Test

```bash
DYLD_FRAMEWORK_PATH="$(pwd)/webkit/src/WebKitBuild/Debug" \
  surfari/libtermsurf_webkit/build/smoke-test \
  "$(pwd)/surfari/libtermsurf_webkit/test-content/index.html" \
  "$(pwd)/surfari/libtermsurf_webkit/test-content/navigation.html"
```

The smoke test initializes the library, creates persistent and incognito browser
contexts, creates a WebKit-backed web contents, receives lifecycle callbacks,
navigates between deterministic local pages, resizes the view, destroys the
objects, and quits.

`DYLD_FRAMEWORK_PATH` is required because WebKit's debug framework has
source-built transitive framework dependencies such as `JavaScriptCore`.

## Current Limitations

Experiment 5 implements the first lifecycle slice only. Input forwarding,
DevTools, JavaScript dialogs, HTTP auth, renderer crash reporting, cursor
updates, and target URL updates are exported as Roamium-compatible symbols but
are not implemented yet.
