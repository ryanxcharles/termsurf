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
