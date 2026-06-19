+++
status = "closed"
opened = "2026-06-19"
closed = "2026-06-19"
+++

# Issue 823: Ghostboard Split Pane Border Configuration

## Goal

Add current `ghostboard/` support for the split-pane visual configuration
settings that existed in `ghostboard-legacy/`, so users can configure focused
and unfocused pane borders without losing terminal readability.

## Background

`ghostboard-legacy/` implemented configurable split-pane borders in Issue 669
and then fixed border/content overlap in Issue 672. The relevant historical
commits are:

- `882ada1b0` — Issue 669: Add split pane borders
- `d4d4756ab` — Issue 669: Add unfocused pane desaturation
- `ed6e7db63` — Issue 672: Inset content by border width
- `595857ca5` — Rename the archived tree to `ghostboard-legacy/`

The current `ghostboard/` tree was recreated from a fresh Ghostty fork and does
not necessarily contain these TermSurf-specific configuration keys or rendering
behavior. This issue should restore the behavior in the new Ghostboard codebase,
using the legacy implementation as reference evidence while adapting to the
current Ghostty `v1.3.1` structure.

## Legacy Behavior

Ghostboard Legacy added these Zig config settings:

```zig
@"focused-split-border-color": ?Color = null,
@"unfocused-split-border-color": ?Color = null,
@"split-border-width": f64 = 0,
```

It also added:

```zig
@"unfocused-split-saturation": f64 = 1.0,
```

`split-border-width` was clamped during config finalization:

```zig
self.@"split-border-width" = @min(10.0, @max(0, self.@"split-border-width"));
```

The macOS Swift layer exposed the Zig config through `termsurf_config_get` as:

- `focusedSplitBorderColor`
- `unfocusedSplitBorderColor`
- `splitBorderWidth`
- `unfocusedSplitSaturation`

The pane border rendered in `SurfaceView.swift` as a SwiftUI overlay:

```swift
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

Issue 672 made the overlay usable by shrinking and offsetting the terminal
surface by the border width, and by padding the progress bar by the same inset:

```swift
let borderInset = isSplit ? termsurf.config.splitBorderWidth : 0
let insetSize = CGSize(
    width: max(10, geo.size.width - borderInset * 2),
    height: max(10, geo.size.height - borderInset * 2)
)

SurfaceRepresentable(view: surfaceView, size: insetSize)
    .frame(width: insetSize.width, height: insetSize.height)
    .offset(x: borderInset, y: borderInset)
```

Example config:

```text
focused-split-border-color = 7dcfff
unfocused-split-border-color = 565f89
split-border-width = 2
```

Expected behavior:

- single-pane windows do not show split-pane borders;
- split panes show no border when `split-border-width = 0`;
- width alone is not enough: the relevant focused or unfocused color must be
  configured;
- the focused split pane uses `focused-split-border-color`;
- unfocused split panes use `unfocused-split-border-color`;
- borders do not intercept mouse input;
- borders do not obscure terminal text, browser overlays, progress bars, or
  other pane content;
- resizing windows, creating splits, closing splits, and focus changes keep
  borders aligned with the pane.

## Scope

In scope:

- Add config parsing and validation for the legacy split-pane border keys in
  current `ghostboard/`.
- Add the Swift config bridge for those settings.
- Render focused and unfocused split-pane borders on macOS.
- Preserve terminal content readability by insetting content or otherwise
  reserving border space.
- Decide whether `unfocused-split-saturation` should be restored in the same
  implementation path or handled as a separate experiment, based on current
  Ghostboard structure.
- Test the feature with split creation, focus switching, window resize, browser
  overlays, and disabled/default config values.

Out of scope unless required by the implementation:

- Wezboard split-border changes.
- New TermSurf protocol messages.
- Non-macOS Ghostboard platforms.
- Redesigning Ghostty's upstream split divider implementation.
- Adding new visual settings beyond the Ghostboard Legacy keys.

## Implementation Notes

Use `ghostboard-legacy/` as the reference implementation, especially:

- `ghostboard-legacy/src/config/Config.zig`
- `ghostboard-legacy/macos/Sources/TermSurf/TermSurf.Config.swift`
- `ghostboard-legacy/macos/Sources/TermSurf/Surface View/SurfaceView.swift`
- `issues/0669-active-pane/README.md`
- `issues/0672-border-padding/README.md`

The implementation should not blindly copy legacy paths. Current `ghostboard/`
is based on Ghostty `v1.3.1`, so the first experiment should audit the current
config and split-view structure before editing code.

## Acceptance Criteria

- Current `ghostboard/` accepts the legacy config keys:
  `focused-split-border-color`, `unfocused-split-border-color`, and
  `split-border-width`.
- `split-border-width` is clamped to a safe range equivalent to the legacy
  `0...10` behavior unless current Ghostty has a better established clamp
  pattern.
- The macOS UI can read focused and unfocused split-border colors and border
  width from the current config.
- In a split layout, focused and unfocused panes draw the configured borders.
- In a single-pane layout, no split-pane border is drawn.
- When `split-border-width = 0`, no split-pane border is drawn.
- If a focused or unfocused color is unset, that state does not draw a border.
- Borders do not intercept mouse input.
- Borders do not cover terminal text, browser overlays, progress bars, or other
  pane content.
- Focus changes update the visible border colors immediately.
- Window resize, split creation, split close, and browser overlay viewport
  geometry continue to work with borders enabled.
- The final result includes runtime evidence: screenshots or logs proving
  focused/unfocused color selection, disabled behavior, content inset, and at
  least one browser-overlay split scenario with borders enabled.

## Experiments

- [Experiment 1: Port split pane border config](01-port-split-pane-border-config.md)
  — **Pass**

## Conclusion

Ghostboard now restores the legacy split-pane border configuration behavior on
macOS. The implementation adds the legacy config keys, clamps border width and
unfocused saturation, bridges the settings into Swift, draws focused and
unfocused split borders with hit testing disabled, insets terminal/progress
content by the border width, and preserves browser overlay geometry in split
layouts.

Experiment 1 verified enabled, clamped, disabled, missing-color,
browser-overlay, hit-test, focus-switch, and adjacent no-border regression
cases. The completion review approved the result with no findings.
