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

# Experiment 811: IOSurfaceLayer Sync Presentation

## Description

Port the first useful slice of upstream `renderer/metal/IOSurfaceLayer.zig` into
Roastty: a retained CoreAnimation layer that can synchronously present an
IOSurface when the surface matches the layer's current pixel size.

Experiment 810 added the IOSurface-backed `MetalTarget` resource layer. The next
missing Metal primitive is the presentation layer that receives that target's
surface. Upstream `IOSurfaceLayer.zig` also contains a custom CALayer subclass,
display callback ivars, async main-thread dispatch, and animation suppression.
Those are important, but they are separable from the first resource/presentation
contract. This experiment adds the synchronous wrapper and size guard without
starting the live render loop or implementing the subclass/display-callback
path.

## Changes

- `roastty/Cargo.toml`
  - Add `objc2-quartz-core` with `CALayer` and `objc2-core-foundation` features
    so Roastty can create and inspect CoreAnimation layers.
  - Enable the `objc2` feature on `objc2-io-surface` so the IOSurface CF type is
    available to Objective-C APIs. The implementation will still use
    `IOSurfaceRef` because `MetalTarget` owns the surface through the existing
    CoreFoundation path.
- `roastty/src/renderer/metal/iosurface_layer.rs`
  - Add `MetalIOSurfaceLayer` owning a retained `CALayer`.
  - Create the layer with `CALayer::layer()`.
  - Set `contentsGravity` to `kCAGravityTopLeft`, matching upstream's resize
    behavior that avoids stretching stale frame contents.
  - Expose `layer() -> &CALayer` for later window/view integration.
  - Add a small unsafe bridge helper that views `&IOSurfaceRef` as `&AnyObject`
    for `CALayer::setContents`. The safety invariant is that IOSurface is
    toll-free bridged/CoreFoundation-backed and `objc2-io-surface` declares the
    CF type with Objective-C ref encoding when the `objc2` feature is enabled.
  - Add `set_surface_sync(&self, surface: &IOSurfaceRef)` that directly sets the
    layer `contents` to the IOSurface. This mirrors upstream `setSurfaceSync`
    and intentionally does not dispatch to the main thread.
  - Add `set_surface_if_size_matches(&self, surface: &IOSurfaceRef) -> bool`
    that computes the layer pixel size from `bounds * contentsScale`, assigns
    contents only when the IOSurface width/height match, and returns whether the
    assignment happened. This ports the discard logic from upstream's async
    callback in a testable synchronous form.
  - Add tests that create a layer, verify `contentsGravity`, set bounds and
    contents scale, present a matching `MetalTarget` surface, verify layer
    contents points at the same IOSurface object, verify a mismatched surface is
    rejected without replacing the previous contents, and verify scaled bounds
    math such as `1.5 × 2.0` bounds with `contentsScale = 2.0` accepting a
    `3 × 4` surface.
- `roastty/src/renderer/metal/mod.rs`
  - Add the `iosurface_layer` module.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the Metal checklist row to mention the
    synchronous IOSurfaceLayer wrapper while keeping the custom subclass,
    async/main-thread presentation, display callback, and full live frame
    orchestration open.

## Verification

- Inspect:
  - `vendor/ghostty/src/renderer/metal/IOSurfaceLayer.zig`
  - `roastty/src/renderer/metal/target.rs`
  - local `objc2-quartz-core` generated `CALayer` bindings
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty metal::iosurface_layer -- --nocapture --test-threads=1`
  - `cargo test -p roastty metal::target -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/811-iosurface-layer-sync.md`
- Run:
  - `git diff --check`

The experiment passes if Roastty has a retained CALayer wrapper that can
synchronously assign a matching IOSurface as layer contents and reject
mismatched surface sizes. It is Partial if layer creation lands but contents
assignment or size checks need follow-up. It fails if the synchronous layer
wrapper cannot be cleanly expressed with the current `objc2` bindings.

## Design Review

Codex reviewed the initial design and found one blocking gap before
implementation: `CALayer::setContents` takes `Option<&AnyObject>`, not
`&IOSurfaceRef`, so the plan needed to specify how the IOSurface is bridged into
the Objective-C object expected by CoreAnimation. The review also asked for
stronger verification: prove the assigned layer contents is the same IOSurface,
prove a rejected mismatched surface does not replace existing contents, and
cover scaled bounds math rather than only scale `1.0`.

The plan was updated to enable the `objc2` feature on `objc2-io-surface`, add an
explicit unsafe `IOSurfaceRef` to `AnyObject` bridge helper with its safety
invariant, and strengthen the tests with contents identity, unchanged contents
after mismatch, and a scaled `1.5 × 2.0` bounds / `contentsScale = 2.0` case.
