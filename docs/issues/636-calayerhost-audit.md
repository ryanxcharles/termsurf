# Issue 636: CALayerHost Feature Audit (Resumed)

## Goal

Complete the CALayerHost feature audit that was paused in Issue 634. Issue 634
tested T1–T12, found a multi-pane regression, and deferred the remaining tests.
Issue 635 fixed the regression (per-tab persistent compositor). This issue
resumes the audit from where we left off.

## Background

[Issue 634](634-calayerhost-audit.md) systematically tested the browser overlay
pipeline after the CALayerHost migration (Issues 625–633). It passed 11 of 12
tests before discovering that the persistent compositor (Issue 633) broke
multi-tab isolation. [Issue 635](635-multi-pane-calayerhost.md) fixed this by
moving the compositor into per-tab `TabState`.

The audit now resumes with the Issue 635 fix in place.

## Prior results (from Issue 634)

| Test | Description             | Result |
| ---- | ----------------------- | ------ |
| T1   | Basic page load         | PASS   |
| T2   | Link navigation         | PASS   |
| T3   | Back/forward navigation | PASS   |
| T4   | Page reload             | —      |
| T5   | Resize (window)         | PASS   |
| T6   | Resize (pane split)     | PASS   |
| T7   | Multi-pane              | PASS   |
| T8   | Multi-profile           | —      |
| T9   | Mouse clicks            | PASS   |
| T10  | Mouse drag/selection    | PASS   |
| T11  | Mouse scroll            | PASS   |
| T12  | Cursor changes          | PASS   |
| T13  | Keyboard typing         | —      |
| T14  | Cmd+key bypass          | —      |
| T15  | Keyboard Tab            | —      |
| T16  | Focus lifecycle         | —      |
| T17  | Loading indicator       | —      |
| T18  | URL sync                | —      |
| T19  | Tab creation            | —      |
| T20  | Tab close               | —      |
| T21  | Retina rendering        | —      |
| T22  | Overlay positioning     | —      |

Known bug from T2: refocusing the TermSurf window eats the first click even
though the browser pane is already focused. Cosmetic — not blocking.

## Remaining tests

Build: release (`./build-release.sh`)

### T4: Page reload

Reload the current page (Cmd+R). Content re-renders without blank frame.

### T8: Multi-profile

Open browser panes with different profiles. Each gets its own Chromium Profile
Server process. Content renders independently in each.

### T13: Keyboard input (typing)

Click a text input or search field. Type characters. Text appears in the field.

### T14: Keyboard input (Cmd+key bypass)

Cmd+C, Cmd+V, Cmd+A, Cmd+X bypass the browser and work as expected (copy, paste,
select all, cut).

### T15: Keyboard input (Tab)

Press Tab to move between form fields. Focus advances correctly.

### T16: Focus lifecycle

Click between a terminal pane and a browser pane. Focus follows correctly.
Keyboard input routes to the focused pane.

### T17: Loading indicator

Navigate to a page. Progress indicator appears during load and disappears when
the page finishes loading.

### T18: URL sync

Navigate to a new page (by clicking a link or entering a URL). The URL bar in
the TUI updates to reflect the current page.

### T19: Tab creation

Open a new `web` tab. Browser pane appears and renders content.

### T20: Tab close

Close a browser tab. Overlay disappears. If it was the last tab for a profile,
the Chromium Profile Server exits.

### T21: Retina rendering

On a Retina display, content renders at physical pixel resolution (not blurry).
Text is sharp. Compare with native Chrome side-by-side if needed.

### T22: Overlay positioning (pixel-perfect)

Compare the browser overlay position with the TUI viewport border. Content
should align to the pixel — no gap, no overlap, no offset.
