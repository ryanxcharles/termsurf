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
