# Issue 716: Wezboard build warnings

## Goal

Eliminate all build warnings from `cargo build -p wezboard-gui`. Currently 193
warnings:

| Category                          | Count | Crate(s)                                                |
| --------------------------------- | ----- | ------------------------------------------------------- |
| `unexpected cfg: cargo-clippy`    | 188   | `window` (184), `wezboard-font` (3), `wezboard-gui` (1) |
| `unnecessary unsafe block`        | 2     | `wezboard-toast-notification`                           |
| `value assigned but never read`   | 1     | `wezboard-gui`                                          |
| `struct/fn never used` (our code) | 2     | `wezboard-gui` (termsurf scaffolding)                   |

A clean build with zero warnings. Not just suppressed — actually fixed.

## Background

These warnings are inherited from upstream WezTerm. The dominant issue (188 of
193 warnings) is the legacy `objc` 0.2 crate. Its `msg_send!`, `class!`, and
`sel!` macros emit `cfg(feature = "cargo-clippy")` checks, which modern Rust
flags as `unexpected cfg`. The proper fix is migrating to `objc2`, which the
codebase already partially uses (`objc2-core-graphics`, `objc2-foundation`,
`objc2-user-notifications` are workspace deps, and `window.rs` has one
`objc2_core_graphics` call).

## Analysis

### The `objc` 0.2 → `objc2` migration

**Scale:** ~182 `msg_send!`/`class!`/`sel!` call sites across 6 files.

**Files by size:**

| File                                     | Lines | `msg_send!`/`class!`/`sel!` | `ClassDecl`/`StrongPtr` |
| ---------------------------------------- | ----- | --------------------------- | ----------------------- |
| `window/src/os/macos/window.rs`          | 3440  | 124                         | 22                      |
| `window/src/os/macos/menu.rs`            | 409   | 25                          | 19                      |
| `window/src/os/macos/app.rs`             | 197   | 18                          | 5                       |
| `window/src/os/macos/connection.rs`      | 255   | 11                          | 0                       |
| `window/src/os/macos/mod.rs`             | 35    | 2                           | 0                       |
| `wezboard-font/src/locator/core_text.rs` | —     | 2                           | 0                       |
| `wezboard-gui/src/commands.rs`           | —     | 1 (`sel!` only)             | 0                       |

**What the migration involves:**

The `objc` 0.2 API is raw and untyped:

```rust
use objc::*;
use objc::runtime::{Object, Sel, Class, BOOL, YES, NO};
use objc::rc::StrongPtr;
use objc::declare::ClassDecl;

let cls = class!(NSWindow);
let obj: id = msg_send![cls, alloc];
let obj: id = msg_send![obj, initWithContentRect:rect
                              styleMask:style
                              backing:backing
                              defer:NO];
```

The `objc2` API is typed and safe:

```rust
use objc2_app_kit::{NSWindow, NSWindowStyleMask, NSBackingStoreType};
use objc2::rc::Retained;

let window = unsafe {
    NSWindow::initWithContentRect_styleMask_backing_defer(
        NSWindow::alloc(),
        rect,
        style,
        backing,
        false,
    )
};
```

Key differences:

- `msg_send![obj, method:arg]` → typed method calls on Rust wrapper types
- `class!(NSFoo)` → direct `NSFoo` type references
- `sel!(foo:)` → not needed (methods are Rust functions)
- `id` (raw pointer) → `Retained<NSFoo>` (typed, reference-counted)
- `StrongPtr` → `Retained<T>`
- `WeakPtr` → `objc2::rc::Weak<T>`
- `ClassDecl` → `objc2::declare::ClassBuilder`
- `BOOL`/`YES`/`NO` → Rust `bool`

**New workspace deps needed:**

- `objc2-app-kit` — NSWindow, NSView, NSEvent, NSMenu, NSAlert, etc.

### Other warnings (trivial)

1. **`unnecessary unsafe`** in `wezboard-toast-notification/src/macos.rs` lines
   97 and 101 — `UNUserNotificationCenter::currentNotificationCenter()` became
   safe in a newer `objc2-user-notifications` version. Remove the `unsafe`
   blocks.

2. **Dead assignment** in `wezboard-gui/src/termwindow/render/screen_line.rs`
   line 677 — `phys_cell_idx += info.pos.num_cells as usize` at end of loop
   where value is never read. Delete the line.

3. **Unused scaffolding** in `wezboard-gui/src/termsurf/state.rs` —
   `TermSurfState` and `state()` are intentional scaffolding for future
   experiments. Suppress with `#[allow(dead_code)]`.

## Ideas for experiments

1. **Quick fixes** — Remove 2 unnecessary `unsafe` blocks, 1 dead assignment,
   suppress scaffolding warnings. Knocks out all non-`objc` warnings in minutes.

## Experiments

### Experiment 1: Quick fixes

#### Goal

Fix the 5 non-`objc` warnings so only the `cargo-clippy` cfg noise remains.

#### Changes

1. **`wezboard-toast-notification/src/macos.rs`** — Remove 2 unnecessary
   `unsafe` blocks.

   Line 97:
   `LazyLock::new(|| unsafe { UNUserNotificationCenter::currentNotificationCenter() })`
   → `LazyLock::new(|| UNUserNotificationCenter::currentNotificationCenter())`

   Line 101: `INIT.call_once(|| unsafe {` → `INIT.call_once(|| {`

   Both `currentNotificationCenter()` and
   `requestAuthorizationWithOptions_completionHandler()` are safe in
   `objc2-user-notifications` 0.3.2.

2. **`wezboard-gui/src/termwindow/render/screen_line.rs`** — Delete the dead
   assignment at line 677: `phys_cell_idx += info.pos.num_cells as usize;`. The
   variable is incremented but never read again before the next loop iteration
   overwrites it.

3. **`wezboard-gui/src/termsurf/state.rs`** — Add `#[allow(dead_code)]` to
   `TermSurfState` and `state()`. These are intentional scaffolding for future
   protocol experiments.

#### Verification

`cargo build -p wezboard-gui` produces only `cargo-clippy` cfg warnings (188)
and their summary lines. No other warnings.

2. **Migrate `connection.rs` + `mod.rs`** — Smallest files (46 lines total, 13
   call sites, no `ClassDecl`). Good warmup to establish the migration pattern
   before tackling bigger files. Requires adding `objc2-app-kit` workspace dep.

3. **Migrate `app.rs`** — 197 lines, 18 call sites, 5 `ClassDecl`/`StrongPtr`.
   First file with `ClassDecl` — the custom Objective-C class registration
   pattern (`ClassDecl` → `ClassBuilder`).

4. **Migrate `menu.rs`** — 409 lines, 25 call sites, 19 `ClassDecl`/`StrongPtr`.
   Heavy use of `ClassDecl` for custom menu item delegate and `StrongPtr` for
   menu ownership.

5. **Migrate `window.rs`** — 3440 lines, 124 call sites, 22
   `ClassDecl`/`StrongPtr`/`WeakPtr`. The bulk of the work. Contains
   NSWindow/NSView subclass declarations, event handling, input method editor,
   drag-and-drop, window lifecycle. May need to be split into multiple
   experiments.

6. **Migrate remaining crates + remove `objc` 0.2** — 3 final call sites
   (`wezboard-font/core_text.rs` has 2, `wezboard-gui/commands.rs` has 1). Then
   remove the `objc` 0.2 dependency from the workspace entirely. End state: zero
   warnings.
