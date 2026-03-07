# Issue 718: Finish `cocoa` and `objc` 0.2 removal from wezboard

## Goal

Complete the migration started in Issue 717. Remove all remaining `cocoa` and
`objc` 0.2 usage from the wezboard codebase, then delete both dependencies.

## Background

Issue 717 migrated 5 of 7 macOS files to pure objc2 and partially migrated
`window.rs` (class registration, callback signatures, geometry types). Two
categories of work remain:

1. `window.rs` still uses `cocoa` trait methods (~50 call sites) and `objc` 0.2
   types (`StrongPtr`, `WeakPtr`, `Object`, `BOOL`/`YES`/`NO`).
2. `wezboard-font/src/locator/core_text.rs` imports `cocoa::base::id`.

Once both are clean, the `cocoa` and `objc` crate dependencies can be deleted.

## Scope

### Files to migrate

| File                                        | Remaining `cocoa` usage                                          | Remaining `objc` 0.2 usage                       |
| ------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------ |
| `window/src/os/macos/window.rs`             | ~50 trait method calls (NSWindow, NSView, NSEvent, NSScreen...) | `StrongPtr`, `WeakPtr`, `Object`, `BOOL`/`YES`/`NO` |
| `wezboard-font/src/locator/core_text.rs`    | `cocoa::base::id`                                                | —                                                |

### Current `cocoa` imports in `window.rs`

```rust
use cocoa::appkit::{
    self, NSApplication, NSApplicationActivateIgnoringOtherApps,
    NSApplicationPresentationOptions, NSBackingStoreBuffered, NSEvent,
    NSEventModifierFlags, NSOpenGLContext, NSOpenGLPixelFormat, NSPasteboard,
    NSRunningApplication, NSScreen, NSView, NSViewHeightSizable,
    NSViewWidthSizable, NSWindow, NSWindowStyleMask,
};
use cocoa::base::*;
use cocoa::foundation::{
    NSArray, NSAutoreleasePool, NSFastEnumeration, NSPoint, NSRect, NSSize,
    NSString,
};
```

### Current `objc` 0.2 imports in `window.rs`

```rust
use objc::rc::{StrongPtr, WeakPtr};
use objc::runtime::{Object, BOOL, NO, YES};
```

### What gets removed when done

- `cocoa` dependency from `window/Cargo.toml` and `wezboard-font/Cargo.toml`
- `objc` dependency from `window/Cargo.toml`
- `cocoa` and `objc` from workspace `[workspace.dependencies]` in root
  `Cargo.toml`
- All `as id` casts and `id` type usage
- All `StrongPtr`/`WeakPtr` usage
- All callback bridge casts (`let this = &mut *(this as *mut Object)`)
- Remaining `cocoa::foundation` geometry types (`NSPoint`, `NSRect`, `NSSize`)
- `cocoa::base::*` wildcard import (`id`, `nil`, `BOOL`, `YES`, `NO`)

### What replaces it

| `cocoa` / `objc` 0.2                              | `objc2` equivalent                                      |
| ------------------------------------------------- | ------------------------------------------------------- |
| `cocoa::appkit::NSWindow` trait methods            | `objc2_app_kit::NSWindow` methods                       |
| `cocoa::appkit::NSView` trait methods              | `objc2_app_kit::NSView` methods                         |
| `cocoa::appkit::NSScreen` trait methods            | `objc2_app_kit::NSScreen` methods                       |
| `cocoa::appkit::NSEvent` trait methods             | `objc2_app_kit::NSEvent` methods                        |
| `cocoa::appkit::NSApplication` trait methods       | `objc2_app_kit::NSApplication` methods                  |
| `cocoa::appkit::NSOpenGLContext` trait methods     | `objc2_app_kit::NSOpenGLContext` methods or `msg_send!`  |
| `cocoa::appkit::NSPasteboard` trait methods        | `objc2_app_kit::NSPasteboard` methods                   |
| `cocoa::appkit::NSWindowStyleMask`                 | `objc2_app_kit::NSWindowStyleMask`                      |
| `cocoa::appkit::NSEventModifierFlags`              | `objc2_app_kit::NSEventModifierFlags`                    |
| `cocoa::appkit::NSApplicationPresentationOptions`  | `objc2_app_kit::NSApplicationPresentationOptions`       |
| `cocoa::appkit::NSBackingStoreBuffered`            | `objc2_app_kit::NSBackingStoreType::Buffered`           |
| `cocoa::foundation::NSPoint`                       | `objc2_core_foundation::CGPoint`                        |
| `cocoa::foundation::NSRect`                        | `objc2_core_foundation::CGRect`                         |
| `cocoa::foundation::NSSize`                        | `objc2_core_foundation::CGSize`                         |
| `cocoa::foundation::NSArray` trait                 | `objc2_foundation::NSArray` methods                     |
| `cocoa::foundation::NSAutoreleasePool` trait       | `objc2_foundation::NSAutoreleasePool`                   |
| `cocoa::foundation::NSString` trait                | `objc2_foundation::NSString` methods                    |
| `cocoa::base::id` (`*mut Object`)                  | `*mut AnyObject` or typed `Retained<NSFoo>`             |
| `cocoa::base::nil`                                 | `std::ptr::null_mut()` or `None`                        |
| `objc::rc::StrongPtr`                              | `objc2::rc::Retained<T>`                                |
| `objc::rc::WeakPtr`                                | `objc2::rc::Weak<T>`                                    |
| `objc::runtime::Object`                            | `objc2::runtime::AnyObject`                             |
| `objc::runtime::BOOL` / `YES` / `NO`              | `bool` / `true` / `false`                               |

## Ideas for experiments

1. **Replace `StrongPtr`/`WeakPtr` and `Object`** — Migrate `window.rs` struct
   fields and callback bodies from `objc` 0.2 memory management to `objc2`.
   `StrongPtr` → `Retained<AnyObject>`, `WeakPtr` → `Weak<AnyObject>`,
   `Object` → `AnyObject`. Remove the callback bridge pattern.

2. **Replace cocoa trait calls in batches** — Group by AppKit class:
   - NSWindow trait calls → `objc2_app_kit::NSWindow` methods
   - NSView trait calls → `objc2_app_kit::NSView` methods
   - NSEvent trait calls → `objc2_app_kit::NSEvent` methods
   - NSScreen trait calls → `objc2_app_kit::NSScreen` methods
   - NSApplication, NSOpenGLContext, NSPasteboard, etc.

3. **Replace cocoa constants and bitflags** — `NSWindowStyleMask`,
   `NSEventModifierFlags`, `NSApplicationPresentationOptions`,
   `NSBackingStoreBuffered`, `NSViewHeightSizable`/`NSViewWidthSizable`.

4. **Migrate `core_text.rs` + delete dependencies** — Replace
   `cocoa::base::id` with `*mut AnyObject`, then remove `cocoa` and `objc` from
   all `Cargo.toml` files.

## Experiments
