# Experiment 53: Diagnose Embedded PDF View Geometry

## Description

Experiment 52 ruled out the simplest synthetic mouse endpoint fixes. Positive x
deltas, an extra rightward drag, and hit-tested drag delivery all still copied
only `LEFT834 MID834` from embedded Surfari. Since standalone `WKWebView` copies
the exact same PDF fixture correctly, the next likely boundary is not the final
mouse endpoint alone; it is somewhere in embedded WebKit/PDF view geometry,
coordinate conversion, hit-testing, responder state, or native PDF selection
internals.

This experiment should compare embedded Surfari's PDF view hierarchy and
coordinate mappings against standalone `WKWebView` using the exact
separated-token fixture. The goal is to identify what differs before choosing a
product fix.

## Changes

- Add env-gated diagnostic tracing to
  `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`, enabled only by a flag
  such as `TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE=1`.
- Trace embedded `WKWebView` state around PDF selection:
  - `WKWebView` frame, bounds, backing scale, window status, and first responder
    chain;
  - recursive subview tree class names, frames, bounds, hidden/alpha state, and
    layer-backed status;
  - hit-test target class at the drag start, drag end, and mouse-up point;
  - coordinate conversions from web point to window point and from window point
    into each hit-test target;
  - visible scroll view/document view information if public APIs expose it;
  - any native PDF plugin/view classes visible through the AppKit hierarchy.
- Add or extend a standalone comparison harness, tentatively
  `scripts/test-issue-834-pdf-view-geometry-compare.sh`, that:
  - launches standalone `WKWebView` with the exact separated-token PDF fixture;
  - replays the same all-token/over-wide cell definitions used by the embedded
    Surfari probe;
  - records enough normalized geometry to prove that the standalone and embedded
    gestures are comparable: web/view coordinates, PDF page coordinates where
    observable, token-relative positions, viewport size, page rect, zoom/scale,
    and scroll offset;
  - records the same subview/hit-test/coordinate information for the standalone
    control;
  - proves standalone copy still copies `LEFT834 MID834 RIGHT834` in the same
    run;
  - runs the embedded Surfari all-token/over-wide cell with tracing enabled;
  - keeps the Experiment 50 oracle and fixture identity gates before comparing
    embedded behavior.
- Compare standalone and embedded traces:
  - view hierarchy classes and nesting;
  - PDF content view frame and bounds if present;
  - hit-test target classes for equivalent drag points;
  - converted coordinates relative to the deepest hit-test target;
  - normalized viewport/page/token geometry proving the compared gestures are
    equivalent;
  - key-window, main-window, first-responder, responder-chain, and copy-target
    evidence around selection and copy;
  - any clipping, scaling, padding, or scroll offset differences.
- Keep all tracing diagnostic-only. Do not change selection behavior in this
  experiment.
- Apply this outcome matrix:
  - **coordinate-conversion-gap:** embedded and standalone hit comparable PDF
    views, but equivalent drag points convert to materially different target
    coordinates or scales;
  - **hit-test-target-gap:** embedded drag points target a different view class
    or hierarchy layer than standalone;
  - **pdf-plugin-view-gap:** standalone exposes a native PDF/plugin view or
    document view that embedded Surfari lacks or wraps differently;
  - **responder-state-gap:** embedded and standalone geometry match, but
    first-responder/key-window/copy target state differs in a way relevant to
    PDF selection;
  - **geometry-equivalent:** embedded and standalone geometry/hit-testing look
    equivalent, pushing the next experiment toward deeper WebKit/PDFKit
    selection state rather than AppKit geometry;
  - **harness-insufficient:** traces are missing, responder/copy-target evidence
    is missing, or gesture equivalence cannot be proven.
- Map result status:
  - **Pass:** any non-`harness-insufficient` classification with complete
    embedded and standalone traces;
  - **Partial:** `harness-insufficient` with useful logs;
  - **Fail:** no embedded run, no standalone run, clipboard restoration failure,
    or behavior changes outside tracing.
- Do not implement a product fix in this experiment. If a gap is identified, the
  next experiment should target that specific gap.
- Update this experiment file with the result.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-pdf-view-geometry-compare.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
```

Run the geometry comparison:

```bash
rm -rf logs/issue-834-exp53-pdf-view-geometry-compare
scripts/test-issue-834-pdf-view-geometry-compare.sh
```

Pass criteria:

- standalone `WKWebView` copies all three tokens in the same run;
- embedded Surfari reproduces the missing-`RIGHT834` behavior in the same run;
- embedded and standalone traces include subview tree, hit-test target, and
  coordinate conversion evidence;
- embedded and standalone traces include key-window, main-window,
  first-responder, responder-chain, and copy-target evidence around selection
  and copy;
- embedded and standalone traces prove comparable gesture geometry for the same
  separated-token fixture, all-token/over-wide cells, viewport/page/token
  coordinates, zoom/scale, and scroll offset;
- oracle and fixture identity gates are open before embedded interpretation;
- one explicit outcome class is selected;
- no selection behavior is changed;
- completion review is recorded.

Partial criteria:

- one side's trace is incomplete but enough evidence remains to guide the next
  experiment;
- private WebKit/PDF views hide enough detail that classification must stay
  cautious.
- standalone and embedded traces are useful but cannot prove equivalent gesture
  geometry or responder/copy-target state.

Failure criteria:

- tracing changes behavior;
- clipboard state is not restored;
- standalone or embedded run cannot launch;
- the result claims a fix instead of a diagnosis.

## Design Review

Codex reviewed the design and agreed Experiment 53 is the correct next
diagnostic step after Experiments 51 and 52. The initial review required two
plan fixes before commit:

- require standalone and embedded runs to prove equivalent gesture geometry
  before classifying coordinate or geometry outcomes;
- require responder and copy-target evidence in the pass criteria because
  `responder-state-gap` depends on it.

Both findings were addressed. A follow-up Codex review approved the design for
the plan commit.

## Result

**Result:** Partial

Experiment 53 added diagnostic-only embedded Surfari PDF view geometry tracing
behind `TERMSURF_SURFARI_PDF_VIEW_GEOMETRY_TRACE=1` and added
`scripts/test-issue-834-pdf-view-geometry-compare.sh`.

The final harness run wrote:

`logs/issue-834-exp53-pdf-view-geometry-compare/pdf-view-geometry-compare-summary.json`

The summary reported:

- `overall_result = partial`
- `classification = harness-insufficient`
- `oracle_gate_open = true`
- `fixture_identity_match = true`
- `embedded_reproduced_missing_right = true`
- `traces_complete = true`
- `standalone_contains_all_tokens = false`
- `gesture_equivalence = false`
- `clipboard_restore_status = restored`

This means the experiment successfully reproduced the embedded Surfari failure
with the exact separated-token fixture: embedded Surfari copied only
`LEFT834 MID834`, not `RIGHT834`. It also produced complete embedded and
standalone view tree, hit-test, scroll, coordinate, responder, and copy-target
traces.

However, the standalone `WKWebView` control did not copy any tokens when driven
with the same normalized all-token/over-wide gesture ratios used by embedded
Surfari. The standalone oracle from Experiment 50 still proves the exact fixture
is selectable/copyable in standalone `WKWebView`, but this Experiment 53
comparison did not prove that the embedded and standalone gestures are
equivalent. The correct classification is therefore `harness-insufficient`, not
a product-level geometry finding.

Useful secondary observations from the trace:

- Embedded Surfari's `TSHostWindow` is not key or main during the selection/copy
  sequence.
- Embedded Surfari reports `target_nil=nil` and `target_webview=nil` for the
  copy action in the traced state, while standalone `WKWebView` reports
  `WKWebView` as the copy target.
- Both embedded and standalone traces hit the top-level `WKWebView`; the current
  AppKit subview tree does not expose a deeper public PDF document view suitable
  for direct comparison.

Verification run:

```bash
bash -n scripts/test-issue-834-pdf-view-geometry-compare.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
rm -rf logs/issue-834-exp53-pdf-view-geometry-compare
scripts/test-issue-834-pdf-view-geometry-compare.sh
```

`surfari/libtermsurf_webkit/build.sh` passed with the existing macOS SDK/WebKit
version warning. `cargo build -p surfari` passed.
`git -C webkit/src status --short` was clean.

## Conclusion

Experiment 53 improved the diagnostics but did not yet identify a root cause.
The next experiment should make the standalone control comparable before
classifying embedded geometry. A good next step is to derive or sweep standalone
`WKWebView` PDF selection coordinates until the same control copies all three
tokens, then compare that successful standalone gesture against the embedded
gesture and responder/copy-target state. The embedded trace's not-key/not-main
host window and nil copy targets are suspicious, but they should be isolated in
a focused experiment rather than accepted as the root cause from this partial
comparison.

## Completion Review

Codex reviewed the completed experiment and found no required issues. The review
agreed that `Partial` / `harness-insufficient` is justified because the oracle
and fixture gates were open, embedded Surfari reproduced `LEFT834 MID834`
without `RIGHT834`, and traces were complete, but standalone `WKWebView` did not
copy all tokens under the same normalized gesture. The review also agreed that
the result does not overclaim and that the next step should make the standalone
selection gesture comparable before classifying embedded geometry or responder
state.
