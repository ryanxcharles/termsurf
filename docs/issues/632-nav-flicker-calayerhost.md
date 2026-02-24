# Issue 632: Navigation Flicker — CALayerHost Swap Artifact

## Goal

Eliminate the single-frame (~16ms) flicker that occurs on every page navigation.
The browser overlay should transition from old page to new page with no visible
blank frame.

## Background

### CALayerHost issue history

This is the eighth issue in the CALayerHost series:

- [Issue 625](625-calayerhost.md) — **CALayerHost migration.** Replaced
  `FrameSinkVideoCapturer` with `CALayerHost`. Chromium sends a `ca_context_id`
  once per tab; the GUI creates a `CALayerHost` sublayer and Window Server
  composites the remote content directly from GPU VRAM.

- [Issue 626](626-x-y-calayerhost.md) — **X/Y positioning.** Fixed ~10px Y and
  ~3px X offset by adding a positioning layer inside a geometry-flipped layer.

- [Issue 627](627-resize-calayerhost.md) — **Resize.** Fixed overlay resize by
  propagating resize events through XPC to the Chromium capturer.

- [Issue 628](628-navigation-calayerhost.md) — **Navigation (first attempt).**
  Eight experiments, all failed. Key finding: the new `ca_context_id` arrives
  quickly but the new host shows nothing for ~10 seconds.

- [Issue 629](629-understand-nav-calayerhost.md) — **Navigation (diagnosis).**
  Five research experiments. Produced the primary hypothesis and confirmed two
  latent bugs.

- [Issue 630](630-nav-calayerhost-6.md) — **Navigation (fix).** Seven
  coordinated fixes across GUI and Chromium resolved the permanent overlay
  disappearance. A brief flicker remained on every navigation.

- [Issue 631](631-continue-nav-calayerhost.md) — **Navigation flicker
  (investigation).** Five experiments attempting to eliminate the flicker. All
  failed, but produced critical understanding of the problem.

### What Issue 631 established

Issue 631 ran five experiments:

1. **Code smell audit** (Experiment 1) — identified 15 potential causes, 11
   confirmed. Primary suspect: unnecessary CALayerHost swap on same ID.

2. **Skip swap when ID unchanged** (Experiment 2) — the `ca_context_id` changes
   on every navigation (confirmed via Chromium server logs), so the skip never
   triggered.

3. **Delay old host removal** (Experiment 3) — the old CAContext's content is
   already destroyed by Chromium when navigation creates a new one. Keeping the
   old host around just keeps a pointer to a dead context.

4. **Research Chromium/Electron** (Experiment 4) — discovered Chromium's
   `DelegatedFrameHost` fallback surface mechanism and Electron's compositor
   recycling patch. Found that `CALayerTreeCoordinator` owns the `CAContext` and
   is recreated per compositor.

5. **Disable compositor recycling** (Experiment 5) — applied Electron's patch.
   Caused white screen on back navigation. The `ca_context_id` changes because
   of renderer/RenderViewHost swaps, not occlusion-triggered compositor
   recycling.

### The critical realization

The flicker was initially estimated at ~100ms. It is actually much shorter —
approximately one frame (~16ms). This changes the entire analysis.

If the gap were 100ms, it would mean the new CAContext genuinely has no content
for a significant period. But a single-frame gap is likely **the inherent cost
of the CALayerHost swap itself**, not a content gap. The new CAContext probably
already has content when we swap — Window Server just needs one vsync cycle to
composite the new host's layer tree after the CATransaction commits.

### Current swap mechanics

The swap in `Metal.zig`'s `setCALayerHostContextId()` is an atomic operation
inside a single `CATransaction`:

1. `CATransaction.begin()` + `setDisableActions:YES`
2. Create new `CALayerHost` with new `contextId`
3. `addSublayer:` new host to positioning layer
4. `removeFromSuperlayer` on old host
5. `CATransaction.commit()`

Window Server processes the entire transaction atomically at the next vsync: old
host removed + new host added = one frame where the new host hasn't been
composited yet. The result is a single blank frame.

### Why previous experiments missed this

- **Experiment 2** (skip swap): The ID changes every navigation, so the swap is
  unavoidable. But the swap itself is the problem — not the ID change.
- **Experiment 3** (delay removal): Kept the old host, but the old CAContext was
  dead. The right idea (keep something visible during the swap) but the wrong
  mechanism (the old host has no content to show).
- **Experiment 5** (prevent recycling): Addressed the wrong cause. The ID
  changes because of renderer swaps, not compositor recycling. But the flicker
  would exist even if the ID stayed the same — it's a swap artifact, not a
  content gap.

## Possible approaches

### Two-phase swap

Split the atomic swap into two CATransactions across two frames:

1. **Frame N**: Add the new CALayerHost on top of the old one. Both hosts are in
   the layer tree. Commit. Window Server composites the new host for the first
   time. The old host is still visible underneath (even if its content is dead —
   it doesn't matter because the new host is on top).
2. **Frame N+1**: Remove the old CALayerHost. Commit. By now, the new host has
   been composited for one full frame and is visible.

This requires a one-frame delay between add and remove. Implementable via
`dispatch_after_f` with a delay of one frame (~16ms), or by deferring the
removal to the next `drawFrame()` call.

### Pre-warm the CALayerHost

Create the new CALayerHost and add it to the layer tree as a hidden sublayer
(e.g., with `opacity: 0` or outside the visible bounds) before the actual swap.
Window Server starts compositing it immediately. When the `ca_context_id`
arrives, move the pre-warmed host into position and remove the old one. The
pre-warmed host has already been composited, so no blank frame.

Challenge: we don't know the new `ca_context_id` until the XPC message arrives.
We would need to create the host, add it to the tree, then set its `contextId` —
and hope that setting the property triggers Window Server to start compositing
before the next vsync.

### Crossfade via opacity

Instead of an instant swap, briefly overlap both hosts:

1. Add new host with `opacity: 0`
2. Animate old host opacity to 0 and new host opacity to 1 over 1-2 frames
3. Remove old host

Even with `setDisableActions:YES`, we could manually set opacity values across
two frames. This turns the blank frame into a crossfade.

### Accept and mask

If the single-frame blank is truly unavoidable with CALayerHost, mask it:

- Set the positioning layer's `backgroundColor` to white (or the page's
  background color). During the one-frame gap, the user sees white instead of
  the terminal background, which is far less jarring.
- Or set it to the previous page's dominant color, extracted before navigation.
