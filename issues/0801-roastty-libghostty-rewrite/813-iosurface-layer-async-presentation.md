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

# Experiment 813: IOSurfaceLayer Async Presentation

## Description

Port the remaining presentation boundary from upstream
`renderer/metal/IOSurfaceLayer.zig`: `setSurface`, which retains the IOSurface,
runs immediately on the main thread, and otherwise dispatches a retained
presentation payload to the main queue before size-checking and assigning layer
contents.

Experiments 811 and 812 already added synchronous IOSurface assignment, size
discard behavior, the custom CALayer subclass, display callback, and implicit
animation suppression. This experiment adds the async/main-thread handoff
without starting the live renderer loop or frame scheduler.

## Changes

- `roastty/Cargo.toml`
  - Add `dispatch2` for Grand Central Dispatch main-queue scheduling.
  - Enable the `NSThread` feature on `objc2-foundation` so the wrapper can
    mirror upstream's `[NSThread isMainThread]` branch.
- `roastty/src/renderer/metal/iosurface_layer.rs`
  - Add
    `MetalIOSurfaceLayer::set_surface(&self, surface: &IOSurfaceRef) -> MetalSurfacePresentationMode`.
  - `set_surface` creates a retained presentation payload containing the layer
    and IOSurface. If `NSThread::isMainThread_class()` is true, it presents the
    payload immediately. Otherwise, it moves the payload into a narrow
    main-queue wrapper and schedules it with `DispatchQueue::main().exec_async`.
  - Add `MetalSurfacePresentationMode::{Immediate, Queued}` so callers and tests
    can observe which branch was taken without depending on a full render loop.
  - Factor the branch decision through a private test seam that accepts an
    explicit `is_main_thread` value and an enqueue function. Production
    `set_surface` passes `NSThread::isMainThread_class()` and an enqueue
    function that calls `DispatchQueue::main().exec_async`; tests can force the
    queued branch and inspect that exactly one retained main-queue payload was
    constructed without needing the macOS main queue to drain.
  - Add a private `SurfacePresentation` payload that owns `Retained<CALayer>`
    and `CFRetained<IOSurfaceRef>`, preserving upstream's
    retain-until-callback-completes rule. The payload's `present` method
    performs the existing `bounds * contentsScale` size check and assigns
    contents only when the IOSurface dimensions match.
  - Add a narrow `MainQueueSurfacePresentation` wrapper with an
    `unsafe impl Send` and a safety comment covering the full invariant: the
    wrapper is move-only, is not exposed after wrapping, is consumed by the
    queued closure, and dereferences/mutates CoreAnimation objects only inside a
    `run_on_main_thread` method that asserts `NSThread::isMainThread_class()`.
    If enqueueing panics before dispatch takes ownership, dropping the retained
    references may release them but does not dereference or mutate them; if a
    queued block never executes, the retained objects remain owned by the queued
    closure rather than being touched off-main.
  - Keep `set_surface_sync`, `set_surface_if_size_matches`, display callback,
    action suppression, and callback lifetime/reentrancy behavior from
    Experiments 811 and 812 unchanged.
  - Add tests for the deterministic payload behavior: a retained presentation
    assigns a matching IOSurface by identity, rejects a mismatched IOSurface
    without replacing previous contents, and keeps layer/surface objects alive
    through the payload. Add tests for the branch seam: forced-main presents
    immediately and does not call the enqueue function; forced-off-main calls
    the enqueue function exactly once with a `MainQueueSurfacePresentation`,
    leaves layer contents unchanged until the queued payload runs, and proves
    the queued payload holds the intended layer/surface identities. The unit
    tests do not claim to drain the real macOS main queue; they prove scheduling
    of the retained payload plus direct payload presentation behavior.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the Metal checklist row to mention
    async/main-thread IOSurfaceLayer presentation while keeping full live frame
    orchestration open.

## Verification

- Inspect:
  - `vendor/ghostty/src/renderer/metal/IOSurfaceLayer.zig`
  - `roastty/src/renderer/metal/iosurface_layer.rs`
  - local `dispatch2::DispatchQueue` bindings
  - local `objc2-foundation` generated `NSThread` bindings
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty metal::iosurface_layer -- --nocapture --test-threads=1`
  - `cargo test -p roastty metal::target -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/813-iosurface-layer-async-presentation.md`
- Run:
  - `git diff --check`

The experiment passes if Roastty has a `set_surface` method that retains the
layer and IOSurface, presents immediately on the main thread, schedules a
move-only retained payload to the main queue off the main thread, asserts main
thread before queued presentation mutates CoreAnimation state, and keeps the
existing size discard behavior passing through deterministic payload and
scheduling-seam tests. Unit tests prove retained payload scheduling and payload
presentation separately; they do not require the test harness to drain the real
macOS main queue. It is Partial if the retained payload lands but the dispatch
boundary or main-thread detection needs follow-up. It fails if the current
bindings cannot express the retained payload and main-queue handoff without
sound lifetime assumptions.

## Design Review

Codex reviewed the initial design and found two blockers before implementation.
First, the proposed branch-observation test did not prove the queued
presentation boundary: it either never exercised `Queued`, or it enqueued real
main-queue work that the test harness did not drain. Second, the
`unsafe impl Send` invariant was too narrow and needed to describe the move-only
wrapper, where CoreAnimation objects are dereferenced, how main-thread access is
asserted, and what happens if enqueueing panics or the queued block never runs.

The plan was updated to add a deterministic private enqueue seam. Tests will
force the off-main branch, assert exactly one retained
`MainQueueSurfacePresentation` is constructed, and verify no layer mutation
happens until the payload's presentation method runs. The plan also now requires
a main-thread assertion inside queued presentation and documents that drops
before enqueue completion release retained references without mutating them,
while a never-run queued closure holds the retained objects rather than touching
them off-main.
