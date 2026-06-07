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

Codex re-reviewed the amended design and approved it with no remaining blocking
findings. The follow-up review confirmed that the callback lifetime, `NSNull`
return ownership, and thread-safe class registration blockers were resolved and
that the revised verification covers the important subclass/callback paths.

## Result

**Result:** Pass

Roastty now backs `MetalIOSurfaceLayer` with a registered
`RoasttyIOSurfaceLayer` Objective-C subclass:

- `roastty/Cargo.toml` enables the `NSNull` Foundation feature needed for the
  no-op `CAAction`.
- `roastty/src/renderer/metal/iosurface_layer.rs` registers the subclass through
  `OnceLock<&'static AnyClass>` and allocates layer instances from that class.
- The subclass stores a heap-stable callback slot pointer in a `*mut c_void`
  ivar. `MetalIOSurfaceLayer::on_display` installs a Rust callback and `Drop`
  clears the ivar before callback storage is released.
- Callback replacement clears the layer ivar before dropping the old callback
  storage, so reentrant `display()` during an old callback capture's destructor
  cannot call through stale storage.
- Callback invocation uses a `Cell<bool>` reentrancy guard and stores the
  `FnMut` in a `RefCell`, so nested `display()` calls from inside the callback
  return before borrowing the callback again.
- The subclass `display` override invokes the installed Rust callback when one
  is present.
- The subclass `actionForKey:` override returns `NSNull` through
  `Retained::autorelease_return`, matching Cocoa's +0 return convention and
  disabling implicit CoreAnimation actions.
- The existing synchronous IOSurface contents assignment and size guard behavior
  remain intact.

Verification:

- Inspected `vendor/ghostty/src/renderer/metal/IOSurfaceLayer.zig`.
- Inspected `roastty/src/renderer/metal/iosurface_layer.rs`.
- Inspected local `objc2-quartz-core` generated `CALayer` and `CAAction`
  bindings.
- Inspected local `objc2-foundation` generated `NSNull` bindings.
- `cargo fmt -p roastty` — passed.
- `cargo test -p roastty metal::iosurface_layer -- --nocapture --test-threads=1`
  — passed, 10 tests.
- `cargo test -p roastty metal::target -- --nocapture --test-threads=1` —
  passed, 5 tests.

## Conclusion

Experiment 812 completes the custom IOSurfaceLayer subclass foundation:
display-callback dispatch and implicit-animation suppression now exist beside
the synchronous IOSurface presentation path from Experiment 811. The remaining
IOSurfaceLayer work is async/main-thread presentation, followed by integration
with full live frame orchestration.

## Completion Review

Codex reviewed the completed result and found one blocking callback-lifetime
issue: `on_display` replaced `display_callback` before clearing the Objective-C
ivar, so the old callback storage could be dropped while the layer still pointed
at it. If an old callback capture's destructor re-entered `display()` on a
retained layer, the subclass could read a stale ivar.

The implementation was fixed to clear the ivar before replacing the callback
storage, then install the new slot after it is allocated. A regression test now
captures an object whose `Drop` calls `display()` while callback replacement is
in progress; the test proves the old callback is not invoked during that
reentrant display and the new callback still works afterward.

Codex re-reviewed the replacement fix and found a second callback safety issue:
`display` could be called reentrantly from inside the installed callback,
creating a second mutable borrow of the same `FnMut`. The implementation now
stores the callback in a `RefCell` behind a `DisplayCallbackSlot` with a
`Cell<bool>` reentrancy guard. Nested `display()` calls return before borrowing
the callback, and a regression test proves a callback-triggered reentrant
`display()` is ignored while later non-reentrant displays still invoke the
callback.

Codex performed a final completion review after both callback safety fixes and
approved the result with no remaining blocking findings. The final review
confirmed the replacement ivar clearing, reentrant display guard, class
registration, retained-layer drop behavior, `NSNull` action ownership, and
IOSurface identity/size behavior.
