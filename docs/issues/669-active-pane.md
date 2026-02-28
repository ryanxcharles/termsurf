# Issue 669: Active Pane Indicator (Take 2)

Revisit pane borders and desaturation now that the resize regression is fixed.

## Background

Issue 667 attempted to add configurable pane borders and unfocused-pane
desaturation. Three experiments all failed with the same symptom: panes didn't
resize when the window was resized or a new split was opened. Typing a key
triggered the resize.

Issue 668 discovered the real cause: Issue 666 had broken TUI resize by only
forwarding `Event::Key` from the crossterm reader thread, silently dropping
`Event::Resize`. The fix was trivial — forward all relevant event types.

This means **all three Issue 667 experiments were tested against a broken
baseline**. The conclusions blaming SwiftUI modifiers and non-empty
`updateOSView` may be wrong. We retry both borders and desaturation with the
resize fix in place.

## Strategy

Follow Ghostty's existing pattern: layout-dependent visual effects live in the
platform layer, not the renderer. The renderer doesn't know if it's in a split —
that state lives in `SurfaceWrapper.isSplit` (Swift) and GTK's `is-split`
property. The existing unfocused pane dimming uses this same pattern (SwiftUI
overlay on macOS, GTK Revealer on Linux).

Config fields go in Zig (Config.zig) — shared across all platforms. The visual
effect goes in Swift (SurfaceView.swift) — same ZStack overlay pattern as the
existing unfocused dimming.

Test features incrementally. Each experiment adds one thing.

## Experiment 1: Border via SwiftUI overlay

### Hypothesis

A `Rectangle().strokeBorder()` in the ZStack — the same pattern as the existing
unfocused opacity overlay — will render pane borders without breaking resize.

This was never tested in isolation in Issue 667. Experiment 2 combined it with
`.saturation()` on the representable and `.shadow()` on the ZStack. Experiment 3
used `updateOSView` instead. Neither tested a plain SwiftUI overlay alone.

The existing unfocused overlay
(`Rectangle().fill().allowsHitTesting(false) .opacity()`) proves that SwiftUI
overlays in the ZStack work. A stroke border overlay follows the same pattern.

### Config

Three new config options (same as Issue 667):

```
focused-split-border-color = 7dcfff
unfocused-split-border-color = 565f89
split-border-width = 2
```

All default to off (no border). Backward compatible.

### Changes

#### 1. Config.zig — 3 new fields after `split-divider-color`

```zig
@"focused-split-border-color": ?Color = null,
@"unfocused-split-border-color": ?Color = null,
@"split-border-width": f64 = 0,
```

Clamp in `finalize()`:

```zig
self.@"split-border-width" = @min(10.0, @max(0, self.@"split-border-width"));
```

#### 2. TermSurf.Config.swift — 3 new properties after `splitDividerColor`

```swift
var focusedSplitBorderColor: Color? { ... }
var unfocusedSplitBorderColor: Color? { ... }
var splitBorderWidth: Double { ... }
```

Same `termsurf_config_get` pattern as existing properties.

#### 3. SurfaceView.swift — border overlay in the ZStack

Add after the unfocused opacity overlay (line 231), before the grab handle (line
233). Same pattern as the existing overlay — a separate view in the ZStack, no
modifiers on the representable or the ZStack itself:

```swift
// Pane border (Issue 669).
if isSplit {
    let borderWidth = termsurf.config.splitBorderWidth
    if borderWidth > 0 {
        let borderColor = surfaceFocus
            ? termsurf.config.focusedSplitBorderColor
            : termsurf.config.unfocusedSplitBorderColor
        if let color = borderColor {
            Rectangle()
                .strokeBorder(color, lineWidth: borderWidth)
                .allowsHitTesting(false)
        }
    }
}
```

**No changes to SurfaceRepresentable.** `updateOSView` stays empty. No SwiftUI
visual modifiers on the representable or the ZStack.

### Result: PASS

Borders render correctly and resize works. All test criteria met:

1. Build compiles without errors.
2. Focused pane shows cyan border, unfocused shows dim border.
3. Switching focus swaps border colors immediately.
4. Window resize works correctly — no regression.
5. Opening a new split resizes existing panes correctly.
6. `split-border-width = 0` disables borders (backward compatible).

This confirms that the Issue 667 failures were caused by the Issue 666 resize
regression, not by SwiftUI overlays. The `Rectangle().strokeBorder()` overlay in
the ZStack — following the exact same pattern as the existing unfocused opacity
overlay — works without any side effects.

Key lesson: SwiftUI overlays in the ZStack are safe. The Issue 667 experiments
that used `.saturation()` on the representable or non-empty `updateOSView` were
red herrings — the resize was already broken before those changes.
