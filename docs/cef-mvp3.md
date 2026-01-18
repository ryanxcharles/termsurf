# CEF MVP3: Precise Pane Matching

## Goal

The browser must always match the precise position and dimensions of its pane.

## Requirements

1. **Position**: The browser's top-left corner must be at the pane's top-left
   corner. Always.

2. **Size**: The browser's width and height must equal the pane's width and
   height. Always.

3. **Responsiveness**: When the pane changes (window resize, split, unsplit,
   drag divider), the browser must update to match near-instantaneously.

4. **No overflow**: The browser must never extend beyond its pane boundaries. It
   must never cover adjacent panes or UI elements.

5. **Temporary stretching is acceptable**: During resize transitions, the
   browser texture may be temporarily stretched or compressed. This is fine as
   long as it immediately corrects itself to match the new pane dimensions at
   1:1 scale.

## Non-Goals for MVP3

- Perfect frame-by-frame synchronization (minor lag is acceptable)
- Avoiding all visual artifacts during resize (temporary stretch is fine)
- Input handling improvements
- Navigation controls

## Success Criteria

The implementation is complete when:

- You can resize the window and the browser fills the pane exactly
- You can split the pane and the browser shrinks to match the new smaller pane
  exactly
- You can close a split and the browser grows to match the larger pane exactly
- You can drag pane dividers and the browser resizes to match exactly
- At no point does the browser overflow into adjacent panes or leave gaps
