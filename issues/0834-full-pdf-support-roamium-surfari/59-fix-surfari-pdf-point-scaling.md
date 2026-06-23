# Experiment 59: Fix Surfari PDF Point Scaling

## Description

Experiment 58 failed to obtain WebKit-internal PDF selection trace records, but
it exposed a more direct Surfari integration hypothesis: Surfari receives
browser geometry from Ghostboard in pixels, while its hidden `WKWebView` and
synthetic AppKit mouse events operate in points.

The current Surfari code path appears to apply pixel-space values directly as
AppKit point-space values:

- `ts_set_view_size` assigns `width` and `height` directly to an `NSWindow`
  frame and `WKWebView` frame;
- `ts_forward_mouse_event` and `ts_forward_mouse_move` pass integer `x`/`y`
  values through `eventLocationInWindow` into `NSEvent.locationInWindow`;
- the `screen_scale` argument to `ts_set_view_size` is currently ignored, and
  Ghostboard currently sends it as `0.0`.

On a 2x Retina display this can make the hidden WebKit view twice the intended
point size while TermSurf's visible overlay and automation gestures remain in
pixel space. That mismatch fits the observed symptom: a visually full-width PDF
drag selects only the left-side token (`LEFT834`) in embedded Surfari.

This experiment should prove or disprove that hypothesis and, if proven, fix
Surfari's point/pixel conversion for WebKit view sizing and input forwarding.

## Changes

- `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`
  - Add an effective-scale helper with this precedence:
    - use `screen_scale` from `ts_set_view_size` when it is greater than `0`;
    - otherwise use `contents->window.backingScaleFactor` when available;
    - otherwise use `contents->window.screen.backingScaleFactor`;
    - otherwise use `NSScreen.mainScreen.backingScaleFactor`;
    - finally fall back to `1.0`.
  - Store the latest effective scale on `WebContents`.
  - Apply point sizing at create time for both normal web contents and devtools
    contents, using the fallback scale because Ghostboard has not sent a resize
    yet.
  - Convert pixel dimensions to AppKit points before assigning the host window
    and `WKWebView` frame.
  - Convert pixel mouse coordinates to AppKit points before creating synthetic
    `NSEvent` instances.
  - Convert scroll event hit coordinates through the same point conversion path.
  - Keep exported CAContext pixel dimensions and snapshot-layer dimensions in
    pixel space so Ghostboard compositing remains unchanged.
  - Extend existing PDF copy/geometry traces to report:
    - raw pixel input;
    - effective scale;
    - converted point coordinates;
    - `WKWebView` point frame/bounds;
    - exported pixel dimensions.
- `scripts/test-issue-834-surfari-pdf-selection-copy.sh`
  - Reuse the existing separated-token PDF selection harness.
  - Add summary fields, if needed, that make the point/pixel proof explicit.

## Verification

The experiment passes only if all of the following are true:

- before the fix, trace evidence shows the mismatch:
  - `ts_set_view_size` receives pixel dimensions;
  - the hidden `WKWebView` is sized in those same numeric point dimensions;
  - Ghostboard supplied `screen_scale` is `0.0`;
  - Surfari's fallback display scale is greater than `1`;
  - mouse coordinates are injected without dividing by scale.
- after the fix, trace evidence shows:
  - the `WKWebView` point size is pixel size divided by scale;
  - synthetic mouse event point coordinates are pixel coordinates divided by
    scale;
  - scroll event hit coordinates are pixel coordinates divided by scale;
  - create-time host frames and first resize frames use the same point
    conversion;
  - exported CAContext pixel dimensions still match the requested browser pixel
    dimensions.
- the calibrated embedded Surfari separated-token PDF selection/copy harness
  copies all expected tokens:
  - `LEFT834`
  - `MID834`
  - `RIGHT834`
- the standalone oracle and calibration gates still match the cells being used.
- `scripts/test-issue-756-surfari-input-regression.sh` passes, proving ordinary
  Surfari page input still reaches the real app after the coordinate conversion
  change.

Required commands:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
bash -n scripts/test-issue-834-surfari-pdf-selection-copy.sh
scripts/test-issue-834-surfari-pdf-selection-copy.sh
scripts/test-issue-756-surfari-input-regression.sh
git diff --check
```

If the scaling fix makes the PDF selection/copy harness pass, the result should
record the exact geometry trace lines proving the point conversion and classify
the bug as a Surfari point/pixel conversion gap.

If the trace does not show a scale mismatch, or if the mismatch is fixed but PDF
copy still returns only `LEFT834`, the result should be recorded as partial or
fail and the next experiment should follow the new evidence rather than
continuing this hypothesis.

## Design Review

Codex reviewed the Experiment 59 design before implementation and agreed that
the point/pixel hypothesis is coherent and is the right next direction after
Experiments 56 through 58.

The review required four plan fixes before implementation:

- `screen_scale` cannot be the only scale source because Ghostboard currently
  sends `0.0`;
- scroll coordinates use the same coordinate path and must be handled or
  explicitly deferred;
- create-time normal and devtools WebKit view sizing must be covered, not only
  later resize messages;
- non-PDF Surfari regression verification must name a concrete command.

The design was updated to use a Surfari effective-scale fallback chain, include
scroll coordinate conversion, include create-time sizing, and require
`scripts/test-issue-756-surfari-input-regression.sh` as the concrete non-PDF
input regression guard. The plan is approved for implementation after the plan
commit.

## Result

**Result:** Fail

The point/pixel mismatch is real in the restored Surfari baseline, but the naive
point-scaling implementation did not fix embedded PDF selection/copy and was
reverted before recording this result.

The attempted implementation changed
`surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm` to store an effective
scale, size the hidden `WKWebView` in points rather than raw pixels, and report
converted point coordinates in the existing PDF copy/geometry traces. It also
tested two input variants:

- divide the forwarded mouse coordinates by the effective scale;
- keep mouse coordinates unchanged while sizing the hidden view in points.

Both variants failed the Surfari PDF selection/copy harness. The retained
harness logs for the first run, `20260623-042342`, and the second run,
`20260623-042512`, show the same default single-marker fixture and the same
failed proof classification:

```text
fixture_mode=single-marker
expected_tokens=TS834PDFCOPYQXJZ
WARN: Surfari PDF selection/copy was not fully proven
```

Those two attempted-fix runs did not retain separate summary JSON files because
the harness writes its current summary to a stable path. They are still enough
to reject the attempted scaling fix: neither variant made the default PDF copy
proof pass, so the product-code patch was removed before this result was
recorded.

After reverting the product-code changes and rebuilding
`libtermsurf_webkit.dylib`, the restored baseline was rerun as `20260623-042849`
with explicit PDF copy and geometry traces enabled:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
bash -n scripts/test-issue-834-surfari-pdf-selection-copy.sh
TERMSURF_SURFARI_PDF_COPY_TRACE=1 \
  TERMSURF_SURFARI_PDF_COPY_TRACE_FILE="$PWD/logs/issue-834-exp59-copy-restored-rebuilt.log" \
  TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE=1 \
  TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE_FILE="$PWD/logs/issue-834-exp59-geometry-restored-rebuilt.log" \
  scripts/test-issue-834-surfari-pdf-selection-copy.sh
git diff --check
```

The restored run returned to the known partial state:

```text
classification=surfari-pdf-selection-copy-partial
overall_result=partial
after_copy_sample=ISSUE834_EXP44_CLIPBOARD_SENTINEL_20260623-042849
fallback_select_all_after_sample=ISSUE834_EXP44_CLIPBOARD_SENTINEL_20260623-042849
```

The restored baseline trace proves the geometry mismatch remains:

```text
overlay_frame={{10, 19}, {928, 544}}
host_frame={{0, 0}, {928, 544}}
browser_pixel=1160x640
backing_scale=2.0
web_point={594, 234}

web_frame={{0, 0}, {1856, 1088}}
web_bounds={{0, 0}, {1856, 1088}}
backing_scale=2.000
render-proof width=3712 height=2176
```

That means the restored hidden `WKWebView` is still numerically sized in raw
pixel dimensions as AppKit points, and WebKit snapshots at 2x again. However,
the failed fix shows that simply dividing view size and mouse coordinates by the
display scale is not sufficient. The Ghostboard `web_point` values forwarded to
Surfari already behave like AppKit point-space inputs for the current hidden
view, and changing the hidden view size breaks the current selection path rather
than proving a fix.

One non-product-code harness fix remains from this experiment:
`scripts/test-issue-834-surfari-pdf-selection-copy.sh` now expands
`WEBKIT_RUN_ENV` using a `set -u` safe empty-array pattern, so the harness works
whether or not `TERMSURF_SURFARI_USE_LOCAL_WEBKIT_ENV=1` is set.

The planned non-PDF Surfari regression guard was also run after reverting the
product-code patch:

```bash
scripts/test-issue-756-surfari-input-regression.sh
```

Run `20260623-043403` passed the ordinary keyboard path and page typing checks,
then failed while waiting for Surfari to receive wheel input:

```text
PASS: Surfari received keyboard events
PASS: page received typed token
PASS: Surfari received click-zone mouse event
WARN: missing page observed click-zone click
FAIL: timed out waiting for Surfari received wheel input
```

Because the attempted point-scaling product-code patch had already been
reverted, this failure does not justify keeping any Experiment 59 implementation
code. It does show that the next experiment should avoid claiming broad Surfari
input regression coverage until the existing wheel/click regression behavior is
understood or the guard is split into narrower PDF-relevant checks.

## Conclusion

Experiment 59 disproved the direct scale-conversion fix as designed. The next
experiment should stop treating this as a simple point/pixel conversion bug and
instead isolate why the embedded PDF selection/copy action does not mutate the
pasteboard even when the drag, focus state, responder chain, and copy command
all reach Surfari.

The strongest next direction is to compare Surfari's embedded PDF action path
against a successful standalone `WKWebView` PDF selection/copy path at the
responder/action level: first responder, `targetForAction:copy:`, window key
state, PDF subview action target, pasteboard change count, and any PDFKit
selection state that can be observed without relying on patched WebKit
internals.

## Completion Review

Codex reviewed the completed Experiment 59 result and initially required three
fixes before commit:

- record or explicitly explain the missing
  `scripts/test-issue-756-surfari-input-regression.sh` verification;
- narrow the attempted-run claims because the retained logs no longer include
  separate summary JSON files for both failed variants;
- make clear that the retained failed-run evidence used the default
  single-marker fixture rather than the separated-token calibrated fixture.

The result was updated to record the non-PDF regression failure, narrow the
attempted-fix evidence to retained harness logs, and call out the single-marker
fixture limitation. Codex then re-reviewed the result and found no required
fixes. The completion review approved Experiment 59 for result commit.
