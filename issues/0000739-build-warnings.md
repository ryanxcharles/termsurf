# Issue 739: Build warnings in Roamium and Wezboard

## Goal

Clean build with zero warnings for both Roamium and Wezboard.

## Background

`./scripts/build.sh all --release` produces 5 warnings across two crates:

### Roamium (1 warning)

1. **`ts_destroy_browser_context` never used** (`roamium/src/ffi.rs:24`) — The
   FFI declaration exists but is never called because Roamium has no `Shutdown`
   handler. The `handle_message` dispatch (`dispatch.rs:223`) sends `Shutdown`
   to `_ => {}`. When the GUI sends `Shutdown`, Roamium should destroy all tabs,
   call `ts_destroy_browser_context`, and exit cleanly. Fix: add a `Shutdown`
   handler that calls it, which also eliminates the dead-code warning.

Research Content Shell's shutdown path
(`content/shell/browser/shell_browser_main_parts.cc` and
`shell_main_delegate.cc`) to understand the correct teardown sequence — which
objects to destroy, in what order, and whether
`BrowserContext::ShutdownStoragePartitions()` or similar cleanup is needed
before destroying the context.

### Wezboard (4 warnings)

2. **Unused import `state::SharedState`**
   (`wezboard-gui/src/termsurf/mod.rs:14`) — `pub use state::SharedState` is
   re-exported but never imported by any code outside the `termsurf` module.
   Internal submodules import `SharedState` directly from `super::state::`. The
   re-export can be removed.

3. **Unused variable `num_panes`**
   (`wezboard-gui/src/termwindow/render/pane.rs:35`) — The `paint_pane` method
   receives `num_panes` but only uses it in the `paint_pane_opengl` path (line
   589). The `paint_pane` method itself doesn't use it — it just forwards to
   `paint_pane_box_model` or `paint_pane_opengl`. Prefix with underscore.

4. **Field `process` never read** (`wezboard-gui/src/termsurf/state.rs:36`) —
   `Server.process` stores the `Child` handle from `Command::new().spawn()` but
   is never read back. It exists to keep the `Child` alive (dropping it doesn't
   kill the process, but it's good practice to hold it for future
   `wait()`/`kill()` calls). Needs `#[allow(dead_code)]` — the field is
   intentionally stored for future use.

5. **Method `first_ns_view` never used** (`wezboard-gui/src/frontend.rs:323`) —
   A helper that extracts the `NSView` pointer from the first window. Not
   currently called. It was likely written for overlay setup but superseded by
   the current `CALayerHost` approach. Can be removed since it's unused and easy
   to recreate if needed.

### Content Shell shutdown research

Content Shell's shutdown sequence: close all Shell windows (destroying their
WebContents) → `Shell::Shutdown()` auto-called on last tab destroy → message
loop exits → `PostMainMessageLoopRun` destroys the BrowserContext via
`.reset()`.

The TermSurf C library's `ts_destroy_browser_context()` is a no-op — the default
browser context is owned by `ShellBrowserMainParts` and destroyed automatically
in `PostMainMessageLoopRun`. The function was a stub for multi-context (multiple
profiles per process) support, which we've decided against — TermSurf uses one
profile per process across all engines.

Ghostboard never calls `ts_destroy_browser_context`. Neither does Roamium.
Removing it from `ffi.rs` has no effect on Ghostboard or the Chromium C library
(the symbol stays exported but harmlessly unused).

## Experiments

### Experiment 1: Remove unused FFI declaration

#### Description

Remove the `ts_destroy_browser_context` FFI declaration from Roamium. The
function is a no-op in the C library and represents a multi-context design we've
rejected. Removing the declaration eliminates the dead-code warning.

#### Changes

**`roamium/src/ffi.rs`**

Remove line 24 (`pub fn ts_destroy_browser_context(ctx: TsBrowserContext);`).

#### Verification

1. `./scripts/build.sh roamium --release`
2. Zero warnings from Roamium.

**Result:** Pass

`./scripts/build.sh roamium --release` compiles with zero warnings. The removed
declaration had no callers in Roamium or Ghostboard, and the C library symbol
remains exported but harmlessly unused.

#### Conclusion

Removed `ts_destroy_browser_context` from `roamium/src/ffi.rs`. This was a stub
for multi-context support (multiple profiles per process) that TermSurf decided
against — each engine process serves exactly one profile. The function was a
no-op in `libtermsurf_chromium` and was never called by any consumer. Roamium
now builds warning-free. Four Wezboard warnings remain for the next experiment.
