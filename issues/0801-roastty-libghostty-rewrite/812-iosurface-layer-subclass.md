+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "default"
reasoning = "medium"

[review.result]
agent = "codex"
model = "default"
reasoning = "medium"
+++

# Experiment 812: IOSurfaceLayer Subclass Callback

## Description

Port the custom CALayer subclass foundation from upstream
`renderer/metal/IOSurfaceLayer.zig` into Roastty. Experiment 811 proved the
synchronous IOSurface contents assignment path using a plain `CALayer`; the next
missing primitive is a layer type that can suppress implicit CoreAnimation
actions and expose a display callback hook for later asynchronous presentation.

This experiment intentionally stops before dispatching work to the main thread
or driving a live render loop. It creates the subclass/callback foundation,
keeps the existing synchronous IOSurface presentation behavior, and verifies the
subclass behavior directly through Objective-C messages.

## Changes

- `roastty/Cargo.toml`
  - Enable the `NSNull` feature on `objc2-foundation` so `actionForKey:` can
    return the standard null `CAAction` object used to disable implicit
    animations.
- `roastty/src/renderer/metal/iosurface_layer.rs`
  - Register a `RoasttyIOSurfaceLayer` Objective-C class through a
    `OnceLock<&'static AnyClass>`, subclassing `CALayer`, using
    `objc2::runtime::ClassBuilder`. Concurrent `new()` calls must all receive
    the same registered class and must not race `ClassBuilder::new`.
  - Allocate `MetalIOSurfaceLayer` instances from that subclass instead of
    `CALayer::layer()`, then cast the retained object to `Retained<CALayer>` for
    the public wrapper boundary.
  - Override `display` on the subclass. The method reads a Rust callback pointer
    ivar and calls it when present; it otherwise returns without falling back to
    CoreAnimation drawing.
  - Override `actionForKey:` on the subclass to return `NSNull::null()` as the
    `CAAction` object for all keys, disabling implicit animations for contents
    and bounds changes. Because `actionForKey:` follows normal Cocoa +0 return
    conventions, the raw method implementation will return a pointer produced by
    `Retained::autorelease_return` after casting the retained `NSNull` singleton
    to the expected Objective-C object/protocol type.
  - Add callback storage to `MetalIOSurfaceLayer` and an
    `on_display(&mut self, callback: impl FnMut() + 'static)` setter that keeps
    the callback alive for as long as the Rust wrapper lives.
  - Clear the subclass callback ivar in `Drop` before releasing the Rust
    callback storage. This prevents a retained Objective-C layer from calling a
    dangling Rust callback pointer if it outlives the `MetalIOSurfaceLayer`
    wrapper after being attached to a future view/layer tree.
  - Keep `layer()`, `set_bounds_pixels`, `expected_pixel_size`,
    `set_surface_sync`, and `set_surface_if_size_matches` behavior from
    Experiment 811 unchanged.
  - Add tests that verify new layers are instances of the custom subclass,
    `display()` invokes the registered Rust callback exactly once per call,
    replacing the callback stops calling the previous one, `actionForKey:`
    returns an object with `NSNull` identity for representative keys, and the
    subclass callback ivar is cleared before wrapper drop releases the callback
    by retaining the Objective-C layer past wrapper drop and calling `display()`
    with no callback effect. Also keep the existing IOSurface contents
    assignment path tests proving identity preservation and mismatched-size
    rejection.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the Metal checklist row to mention the
    IOSurfaceLayer subclass/display callback and animation suppression while
    keeping async main-thread presentation and full live frame orchestration
    open.

## Verification

- Inspect:
  - `vendor/ghostty/src/renderer/metal/IOSurfaceLayer.zig`
  - `roastty/src/renderer/metal/iosurface_layer.rs`
  - local `objc2-quartz-core` generated `CALayer` and `CAAction` bindings
  - local `objc2-foundation` generated `NSNull` bindings
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty metal::iosurface_layer -- --nocapture --test-threads=1`
  - `cargo test -p roastty metal::target -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/812-iosurface-layer-subclass.md`
- Run:
  - `git diff --check`

The experiment passes if Roastty's IOSurface layer wrapper is backed by a custom
CALayer subclass, can invoke a Rust display callback through the subclass
override, returns `NSNull` as the implicit animation action, and keeps the
existing IOSurface size/contents behavior passing. It is Partial if the subclass
lands but either callback invocation or action suppression needs follow-up. It
fails if the current `objc2` runtime bindings cannot safely express the custom
CALayer subclass.

## Design Review

Codex reviewed the initial design and found three blocking gaps before
implementation. First, the callback pointer ivar could dangle if the Objective-C
layer was retained by a future view/layer tree after the Rust
`MetalIOSurfaceLayer` wrapper dropped. Second, the `actionForKey:` plan did not
pin down Cocoa ownership semantics for returning the `NSNull` `CAAction` object
from a raw `ClassBuilder` method. Third, class registration needed an explicit
thread-safe singleton mechanism instead of merely saying "register once."

The plan was updated to clear the callback ivar in `Drop` before callback
storage is released, add a retained-layer-after-wrapper-drop test, return the
`NSNull` action object with `Retained::autorelease_return` to satisfy +0 method
ownership, and require `OnceLock<&'static AnyClass>` for race-free subclass
registration.
