# Issue 667: Active Pane Indicator

Replace simple opacity dimming with a more readable visual indicator for the
active pane in split layouts.

## Problem

Ghostty's `unfocused-split-opacity` dims inactive panes by blending them toward
a fill color. This makes unfocused panes harder to read — contrast drops
uniformly across all text. The user needs to know which pane is active, but
dimming is a blunt tool that sacrifices readability to achieve it.

The goal: a clear visual signal for the active pane that preserves full
readability in all panes.

## Options to Research

### 1. Active pane border/glow

Add a colored border or subtle glow to the focused pane only. Don't modify
unfocused panes at all. TermSurf already uses colored borders for mode
indication (cyan for Browse, purple for Edit, yellow for Command). This extends
that visual language to pane focus.

**Research questions:**

- Where does Ghostty render split borders on macOS (Metal renderer? Swift layer?
  CALayer?)
- Can we change the border color/thickness based on focus state?
- Can we add a glow effect (e.g., shadow or bloom) around the active pane?

### 2. Desaturation

Shift unfocused panes toward grayscale. Luminance stays identical, so contrast
ratios are perfectly preserved. The active pane pops because it's the only one
with full color.

**Research questions:**

- Where does the current opacity dimming happen on macOS?
- Can we apply a saturation shader in the Metal compositing pass?
- Does Core Image or CALayer provide built-in desaturation filters?
- Performance cost of per-frame desaturation on unfocused panes?

### 3. Color temperature shift

Tint unfocused panes cooler (toward blue) or warmer (toward orange). A cool
shift reads as "receded" / "inactive" perceptually. Can combine with light
desaturation.

**Research questions:**

- Same renderer questions as desaturation — where to apply the filter
- Can this be done with a color matrix transform on the CALayer?
- How subtle can we make it while still being noticeable?

### 4. Soft light blend

A gentle tonal overlay that shifts the mood without crushing blacks or blowing
highlights. Think of it as a color filter rather than a dimmer.

**Research questions:**

- What blend modes does Metal / Core Animation support natively?
- Can CALayer compositingFilter achieve this without a custom shader?
- Performance implications vs. the current opacity approach?

### 5. Photoshop-style blend modes (multiply, screen, overlay)

Apply blend modes between a color overlay and the pane content.

**Research questions:**

- Which blend modes preserve readability? (Multiply and burn darken — same
  problem as dimming. Screen and dodge wash out.)
- Does Core Animation support these as compositingFilter values?
- Would this require a custom Metal fragment shader?

## Plan

Research all five options. For each, determine:

1. Where in the codebase the change would live
2. How much code is required
3. Whether it needs custom shaders or can use system APIs
4. Performance cost
5. Readability preservation

If one option stands out as clearly superior, implement it as Experiment 1.

## Experiment 1: Research

### How pane dimming works today on macOS

The current unfocused pane dimming is a **SwiftUI overlay**, not a Metal shader
or CALayer filter. In `SurfaceView.swift` (lines 219–231):

```swift
if (isSplit && !surfaceFocus) {
    let overlayOpacity = termsurf.config.unfocusedSplitOpacity;
    if (overlayOpacity > 0) {
        Rectangle()
            .fill(termsurf.config.unfocusedSplitFill)
            .allowsHitTesting(false)
            .opacity(overlayOpacity)
    }
}
```

A semi-transparent `Rectangle()` is composited on top of the Metal surface in a
ZStack. `allowsHitTesting(false)` lets clicks pass through. The opacity is
inverted from the config value: `unfocused-split-opacity = 0.85` becomes
`1 - 0.85 = 0.15` SwiftUI opacity. Config bridge lives in
`TermSurf.Config.swift` (lines 431–454).

The Metal renderer (`Metal.zig`) outputs full-brightness content to an
IOSurface. It has no awareness of focus state or dimming — that's entirely
SwiftUI's responsibility.

### How split dividers work today

Split dividers are SwiftUI views in `SplitView.Divider.swift`. Each divider is a
1pt visible `Rectangle()` with a 6pt invisible hit region for resize dragging.
The divider color is configurable via `split-divider-color` in the Zig config
(`Config.zig` line 1061), with a smart default that darkens the background color
by 8% (light themes) or 40% (dark themes). The color does **not** change based
on focus state.

Split tree layout lives in `TerminalSplitTreeView.swift`, which recursively
builds the SwiftUI hierarchy with dividers between panes.

### Option 1: Active pane border/glow

**Where it lives:** SwiftUI layer or CALayer on the surface view.

**Approach:** Use `CALayer.borderColor`/`borderWidth` for a solid border and
`CALayer.shadowColor`/`shadowRadius`/`shadowOffset` for a glow effect:

```swift
layer.borderColor = NSColor.cyan.cgColor
layer.borderWidth = 2.0
layer.shadowColor = NSColor.cyan.cgColor
layer.shadowOffset = .zero       // radiate evenly (glow, not drop shadow)
layer.shadowRadius = 15.0        // glow spread
layer.shadowOpacity = 0.8
layer.shadowPath = CGPath(rect: layer.bounds, transform: nil)  // critical perf
```

Setting `shadowOffset = .zero` makes the shadow radiate in all directions like a
glow instead of casting in one direction. `shadowPath` must be set for
performance — without it, Core Animation computes the shadow from the layer's
alpha channel every frame, which is slow. With `shadowPath`, it's GPU-composited
and essentially free.

**Difficulty:** Very low. A few lines in SurfaceView. No shaders, no new render
passes.

**Performance:** Free. Same cost as a simple opacity change when `shadowPath` is
set.

**Readability:** Perfect — unfocused panes are untouched.

**Risk:** Zero. Works on all macOS versions. Battle-tested API.

### Option 2: Desaturation

**Where it lives:** Three possible locations, each with different tradeoffs.

**Approach A — CALayer.filters with CIColorControls:**

```swift
let filter = CIFilter(name: "CIColorControls")!
filter.setValue(0.3, forKey: kCIInputSaturationKey)  // 0 = gray, 1 = normal
filter.name = "desaturate"
view.layer?.filters = [filter]
```

GPU-accelerated through Core Image. CIColorControls provides `inputSaturation`,
`inputBrightness`, and `inputContrast`. Very cheap shader (per-pixel
multiply/lerp).

**Problem:** `CALayer.filters` is macOS-only and has had reliability regressions
starting with macOS 11 (Big Sur). Some developers report CIFilters silently
failing when applied as layer filters. `backgroundFilters` changed clipping
behavior in macOS Sonoma. Not a showstopper, but a maintenance risk.

**Approach B — Custom Metal post-processing shader:**

The renderer already has `full_screen_vertex` in `shaders.metal` (line 191) and
pipeline descriptor infrastructure in `shaders.zig`. A desaturation shader is
trivial:

```metal
fragment float4 post_process_fragment(
    FullScreenVertexOut in [[stage_in]],
    texture2d<float> tex [[texture(0)]],
    constant float& saturation [[buffer(0)]]
) {
    float4 color = tex.read(uint2(in.position.xy));
    float luma = dot(color.rgb, float3(0.299, 0.587, 0.114));
    color.rgb = mix(float3(luma), color.rgb, saturation);
    return color;
}
```

Requires an intermediate render target (render to offscreen texture, then blit
through post-process shader to the drawable). The existing `RenderPass` /
`Target` / `Texture` infrastructure supports this. Sub-0.1ms on Apple Silicon.

**Problem:** Plumbing the intermediate texture is the hard part — it means
rendering to an offscreen texture instead of directly to the drawable, then
blitting. Estimated 1–2 days.

**Approach C — Replace the SwiftUI overlay:**

Instead of a semi-transparent `Rectangle()`, apply a `saturation` modifier to
the surface view:

```swift
surfaceView
    .saturation(isSplit && !surfaceFocus ? 0.3 : 1.0)
```

SwiftUI's `.saturation()` modifier desaturates the view's content. This is the
simplest approach — one line of SwiftUI replaces the existing overlay code.
Under the hood, SwiftUI likely applies a CIFilter or Metal shader, but we don't
manage it directly.

**Difficulty:** Approach C is trivial (one line). Approach A is low (a few
lines). Approach B is moderate (1–2 days for render-to-texture plumbing).

**Performance:** All three are cheap. Approach B is the cheapest (part of the
existing Metal command buffer). Approach C depends on SwiftUI's implementation.

**Readability:** Excellent — luminance is preserved, contrast ratios unchanged.

**Risk:** Approach A has macOS version risk. Approach B has zero risk but more
code. Approach C is untested — need to verify SwiftUI's saturation modifier
works correctly over a Metal-backed IOSurface.

### Option 3: Color temperature shift

**Where it lives:** Same three locations as desaturation.

**Approach:** Add a warm/cool tint by shifting the red/blue balance.
`CIColorMatrix` can do this as a CALayer filter, or it's 2 lines in a Metal
shader:

```metal
color.r += temperature * 0.1;
color.b -= temperature * 0.1;
```

Can combine with partial desaturation for a stronger signal (cool + muted =
clearly "inactive").

**Difficulty:** Same as desaturation — the shader math is trivially different.

**Performance:** Same as desaturation.

**Readability:** Good if subtle. A strong temperature shift can make some colors
harder to distinguish (e.g., warm tint makes red and orange look identical).

**Risk:** Same as desaturation per approach.

### Option 4: Soft light blend

**Where it lives:** CALayer.compositingFilter on an overlay layer.

**Approach:** `CALayer.compositingFilter` controls how a layer composites with
layers behind it. Setting it to `CISoftLightBlendMode` on a colored overlay
would produce a gentle tonal shift instead of the current opacity blend.

```swift
let overlay = CALayer()
overlay.backgroundColor = NSColor(red: 0.1, green: 0.1, blue: 0.2).cgColor
overlay.compositingFilter = CIFilter(name: "CISoftLightBlendMode")
```

The full list of available blend modes matches Photoshop: multiply, screen,
overlay, soft light, hard light, color dodge, color burn, darken, lighten,
difference, exclusion, hue, saturation, luminosity, and more.

**Problem:** `compositingFilter` controls how a layer blends with what's behind
it — it doesn't transform the layer's own content. So you still need a separate
overlay layer, just with a different blend mode. The effect is subtle and hard
to control precisely.

**Difficulty:** Low (swap the blend mode on the existing overlay approach).

**Performance:** Free — same compositing cost as the current opacity overlay.

**Readability:** Depends on the blend color and mode. Soft light preserves
midtones well but the effect is very subtle — may not be noticeable enough.

**Risk:** Low, but `compositingFilter` is macOS-only and less commonly used than
simple opacity.

### Option 5: Photoshop-style blend modes

**Where it lives:** Same as option 4 — CALayer.compositingFilter on an overlay.

**Available modes:** All standard Photoshop blend modes are available via
CIFilter names (`CIMultiplyBlendMode`, `CIScreenBlendMode`,
`CIOverlayBlendMode`, `CIColorBurnBlendMode`, `CIColorDodgeBlendMode`,
`CIDarkenBlendMode`, `CILightenBlendMode`, `CIDifferenceBlendMode`,
`CIExclusionBlendMode`, `CIHardLightBlendMode`, `CISoftLightBlendMode`,
`CIHueBlendMode`, `CISaturationBlendMode`, `CILuminosityBlendMode`, etc.).

**Problem:** Most modes destroy readability. Multiply and burn darken (same
problem as dimming). Screen and dodge wash out highlights. Overlay increases
contrast harshly. The only modes that preserve readability are soft light
(subtle), hue (shifts colors), and saturation (desaturates) — but those are
better achieved through dedicated approaches (options 2–4).

**Difficulty:** Same as option 4.

**Readability:** Poor for most modes. The modes that preserve readability are
covered by other options.

**Risk:** Same as option 4.

### Summary

| Option                    | Difficulty | Performance | Readability | Risk      | Notes                       |
| ------------------------- | ---------- | ----------- | ----------- | --------- | --------------------------- |
| 1. Border/glow            | Very low   | Free        | Perfect     | Zero      | No unfocused pane changes   |
| 2. Desaturation (SwiftUI) | Trivial    | Low         | Excellent   | Low       | One-line `.saturation()`    |
| 2. Desaturation (CALayer) | Low        | Low         | Excellent   | Medium    | macOS 11+ regressions       |
| 2. Desaturation (Metal)   | Moderate   | Lowest      | Excellent   | Zero      | Needs render-to-texture     |
| 3. Temperature shift      | Same as 2  | Same as 2   | Good        | Same as 2 | Can muddy colors            |
| 4. Soft light             | Low        | Free        | OK          | Low       | Effect may be too subtle    |
| 5. Blend modes            | Low        | Free        | Poor        | Low       | Most modes hurt readability |

**Option 1 (border/glow) stands out.** It's the simplest, lowest risk, zero
performance cost, and preserves perfect readability in all panes. It also
extends TermSurf's existing visual language of colored borders for mode
indication. The glow effect via `CALayer.shadow*` is a well-tested macOS API.

Option 2 via SwiftUI's `.saturation()` modifier is a compelling second choice —
one line of code, preserves contrast — but needs testing to verify it works over
Metal-backed IOSurface content.

Options 3–5 don't offer enough advantage over 1 and 2 to justify their
tradeoffs.

## Experiment 2: Border + desaturation

### Hypothesis

Adding configurable pane borders (for both focused and unfocused panes) and
configurable desaturation (for unfocused panes) will provide clear active pane
indication without sacrificing readability. Both features are independent and
can be used alone or together.

### Config design

New config options follow Ghostty's existing naming convention
(`unfocused-split-*`, `split-*`). All are optional with sensible defaults.

#### Border options

```
# Border color for the focused pane. Default: none (no border).
focused-split-border-color = 7dcfff

# Border color for unfocused panes. Default: none (no border).
unfocused-split-border-color = 565f89

# Border width in points. Applies to both focused and unfocused.
# 0 disables borders entirely. Default: 0 (off — backward compatible).
split-border-width = 2

# Border style: solid, dashed, dotted, double. Default: solid.
# "dashed" and "dotted" use CAShapeLayer with stroke patterns.
# "double" draws two 1pt lines separated by 1pt gap.
split-border-style = solid
```

#### Glow options (focused pane only)

```
# Glow color for the focused pane. Default: none (no glow).
# Only visible when split-border-width > 0.
focused-split-glow-color = 7dcfff

# Glow radius in points. Controls how far the glow spreads.
# 0 disables glow. Default: 0.
focused-split-glow-radius = 10

# Glow opacity (0.0 to 1.0). Default: 0.6.
focused-split-glow-opacity = 0.6
```

#### Desaturation option

```
# Saturation level for unfocused panes (0.0 = grayscale, 1.0 = full color).
# Default: 1.0 (no desaturation — backward compatible).
unfocused-split-saturation = 0.4
```

#### Example configs

Minimal — just a border on the active pane:

```
focused-split-border-color = 7dcfff
split-border-width = 2
```

Border + glow for a prominent active indicator:

```
focused-split-border-color = 7dcfff
unfocused-split-border-color = 565f89
split-border-width = 2
focused-split-glow-color = 7dcfff
focused-split-glow-radius = 12
```

Desaturation only — mute unfocused panes without borders:

```
unfocused-split-saturation = 0.3
```

All features combined:

```
focused-split-border-color = 7dcfff
unfocused-split-border-color = 3b4261
split-border-width = 2
split-border-style = solid
focused-split-glow-color = 7dcfff
focused-split-glow-radius = 10
focused-split-glow-opacity = 0.6
unfocused-split-saturation = 0.4
```

### Design notes

- **All defaults are backward compatible.** Border width defaults to 0 (off),
  saturation defaults to 1.0 (full color). Users who don't set these options see
  no change.
- **`unfocused-split-opacity` and `unfocused-split-fill` remain.** The existing
  dimming system is not removed. Users can use dimming, desaturation, borders,
  or any combination.
- **Border is on the pane, not the divider.** `split-divider-color` controls the
  1pt line between panes. `split-border-width` adds a border around each
  individual pane. These are separate — the divider sits between two pane
  borders.
- **Border style trades simplicity for flexibility.** `solid` uses plain
  `CALayer.borderWidth`/`borderColor` (zero code complexity). `dashed`,
  `dotted`, and `double` require `CAShapeLayer` with custom stroke patterns.
  Start with `solid` only. Add other styles later if needed.

### Changes

1. **Add config options** in `gui/src/config/Config.zig` — 8 new fields with
   defaults matching the backward-compatible values above.

2. **Bridge to Swift** in `TermSurf.Config.swift` — read each new config value
   via `termsurf_config_get`, same pattern as existing `unfocusedSplitOpacity`.

3. **Apply borders** in `SurfaceView.swift` — use CALayer border/shadow
   properties on the surface view's backing layer, toggled by focus state.

4. **Apply desaturation** in `SurfaceView.swift` — replace or augment the
   existing `Rectangle()` overlay with SwiftUI's `.saturation()` modifier on the
   surface view. If `.saturation()` doesn't work over Metal-backed IOSurface,
   fall back to `CALayer.filters` with `CIColorControls`.

5. **Register config in c_get.zig** — add the new keys to the config getter so
   the Swift bridge can read them.

### Test

1. `cd gui && zig build` — compiles without errors.
2. Open two split panes. Focused pane shows colored border + glow. Unfocused
   pane shows dim border, no glow, desaturated content.
3. Switch focus between panes — borders and saturation update immediately.
4. Set `split-border-width = 0` — no borders visible (backward compatible).
5. Set `unfocused-split-saturation = 1.0` — no desaturation (backward
   compatible).
6. Set `split-border-style = solid` — solid border renders correctly.
7. Verify existing `unfocused-split-opacity` still works alongside new options.
8. Verify `split-divider-color` (the inter-pane line) is unaffected.
