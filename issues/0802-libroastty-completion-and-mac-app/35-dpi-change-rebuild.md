+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 35: Phase C — rebuild the renderer on a DPI (content-scale) change

## Description

`present_live` (`lib.rs:2439`) builds the Metal renderer **lazily and once**
(`if self.renderer.is_none()`), rasterizing glyphs at
`font_size_points * scale_factor_x` (physical pixels).
`roastty_surface_set_content_scale` (`lib.rs:~14329`) updates `scale_factor_x/y`
but **never drops the renderer**, so after a **DPI change** — moving the window
to a monitor with a different `backingScaleFactor`, or an OS scale change — the
renderer keeps rendering glyphs rasterized at the **old** scale: blurry (scale
up) or chunky (scale down) text until the surface is otherwise rebuilt. Upstream
rebuilds the font grid / renderer at the new DPI.

## Approach

In `roastty_surface_set_content_scale`, detect an actual scale change and drop
the renderer so `present_live` rebuilds it at the new scale on the next present:

```rust
pub extern "C" fn roastty_surface_set_content_scale(surface: RoasttySurface, x: f64, y: f64) {
    if let Some(surface) = surface_from_handle(surface) {
        let changed = surface.scale_factor_x != x || surface.scale_factor_y != y;
        surface.scale_factor_x = x;
        surface.scale_factor_y = y;
        if changed {
            // DPI changed (e.g. moved to a monitor with a different backing scale): drop the live
            // renderer so present_live rebuilds it at the new scale (Issue 802 / Exp 35) — else
            // glyphs stay rasterized at the old DPI.
            surface.renderer = None;
            surface.dirty = true;
        }
    }
}
```

`present_live`'s existing `if self.renderer.is_none()` branch then rebuilds via
`build_live_renderer` reading the new `scale_factor_x` (point size
`font_size * scale`), re-rasterizing glyphs at the new DPI. **Only
`libroastty`** (`lib.rs`, one FFI function). No app change. Only the **change**
case drops/dirties — an idempotent `set_content_scale` with the same scale is a
no-op (no spurious renderer churn).

## Verification

1. **Headless change-detection test** (`lib.rs`): on a new surface, reset
   `dirty = false`; call `roastty_surface_set_content_scale(surface, 2.0, 2.0)`
   (a change from the default `1.0`) → assert `scale_factor_x/y == 2.0` **and**
   `dirty == true` (a present — which rebuilds — is requested). Then reset
   `dirty = false` and call `set_content_scale(surface, 2.0, 2.0)` **again**
   (same scale) → assert `dirty == false` (no spurious rebuild). The renderer is
   `None` headlessly (no nsview), so the drop itself is a no-op in tests; the
   **change-detection + dirty trigger** (the mechanism that forces the rebuild)
   is what's asserted. `cargo test -p roastty` (full) green.
2. **No regression:** `set_content_scale` still updates the scale fields
   (existing scale-dependent tests at `lib.rs:27588`+ unaffected); no
   present-path or size-path change.
3. **Live confirmation** (screen unlocked — check `CGSSessionScreenIsLocked`):
   run the app, drag the window between a Retina and a non-Retina monitor (or
   change display scaling) → text re-rasterizes crisply at the new DPI (no
   lingering blur). If locked, record **Partial-pending-live** like Exp 29/30/33
   — the headless change-detection proves the trigger; the visual re-sharpen
   awaits the unlock.
4. Faithful to upstream's renderer/font-grid rebuild on a content-scale change.

**Pass** = a scale change drops the renderer (rebuilt at the new scale) + sets
dirty, the headless change-detection test passes, the suite is green, and the
live app re-sharpens text across a DPI change.

**Partial** = the headless change-detection + suite are green, but the live DPI
re-sharpen is screen-blocked (locked) — documented, pending the unlock re-probe.

**Fail** = dropping the renderer on scale change doesn't rebuild at the new
scale (documented).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** Confirmed against the code: the **gap is
real + the fix correct** — `present_live` builds the renderer only when `None`,
and `build_live_renderer` bakes in glyph rasterization (`font_size*scale`) +
layer bounds + attach at build-time scale, so without a rebuild text stays at
the old DPI; nothing else drops/rebuilds on a scale change today; dropping
`renderer` → next `present_live` fully re-rasterizes at the new scale. The
**drop is safe** — `attach_to_nsview` uses `setLayer:` (the view retains;
replace, not append), so `= None` (Drop) releases the renderer's ref while the
view holds the old layer until the rebuild's `setLayer:` swaps in a fresh one
(AppKit then releases the old) — no leak, no double-attach; both
`set_content_scale` and the present-driver tick run serialized on the main
thread (no race). **`dirty=true` is sufficient** (the present driver polls
`dirty` every 16ms → `present_live` → clears it). The **test is non-vacuous**
(default config `scale_factor=1.0`, so `set_content_scale(2.0,2.0)` is a genuine
change → `dirty`; the repeat at the same scale → not dirty; the drop itself is
untestable headlessly — stated honestly). Partial-pending- live is right (Exp
29/30/33 precedent). Optional/Nit (non-blocking): `f64 != f64` is sound for
`backingScaleFactor` (never NaN; `present_live`'s `max(1.0)` bounds it anyway) —
a NaN guard is optional defensiveness; the `present_live` doc comment's "driven
from set_content_scale" is a pre-existing inaccuracy, not a contract this
experiment must satisfy.

## Result

_(to be added after the run.)_

## Conclusion

_(to be added after the run.)_
