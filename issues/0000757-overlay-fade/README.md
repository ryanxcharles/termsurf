+++
status = "open"
opened = "2026-03-16"
+++

# Issue 757: Disable overlay fade animation on tab switch

## Goal

Browser overlays appear and disappear instantly when switching tabs. No fade
in/out animation.

## Background

When switching between tabs, the browser overlay fades in and out instead of
appearing instantly. This is distracting and makes tab switching feel sluggish.

We previously disabled CALayer animations for overlay repositioning (the
`setDisableActions: YES` calls in `CATransaction` blocks throughout `conn.rs`).
The same approach should work for the show/hide transition.

The fade is caused by `sync_overlay_visibility()` in `conn.rs`, which calls
`setHidden:` on the overlay's flipped layer. By default, CoreAnimation
implicitly animates property changes on CALayers — including `hidden`. The fix
is to wrap the `setHidden:` call in a `CATransaction` with animations disabled,
the same pattern we already use everywhere else.
