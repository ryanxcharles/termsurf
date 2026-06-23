# Experiment 37: Prove Surfari-Side Render Pixels

## Description

Experiment 36 classified the current visible-content failure as a generic
Surfari render/compositing gap: both a deterministic HTML page and a
deterministic PDF reached Surfari loading callbacks, nonzero CAContext export,
and Ghostboard/AppKit overlay presentation, but neither produced visible page
pixels in window or full-screen screenshots.

The next unknown is whether Surfari's `WKWebView` is internally rendering
content at all. Without that evidence, we cannot tell whether the failure is in
WebKit rendering, the exported `CAContext`, the Ghostboard `CALayerHost`
attachment, z-order/opacity, or screenshot capture.

This experiment should add the smallest possible temporary diagnostic path that
captures or classifies rendered `WKWebView` pixels inside
Surfari/libtermsurf_webkit. The goal is evidence, not a permanent public API.

## Changes

- Add a narrow diagnostic hook in `surfari/libtermsurf_webkit` that can classify
  the current `WKWebView` contents after page load.
- Prefer WebKit/AppKit-native mechanisms:
  - `-[WKWebView takeSnapshotWithConfiguration:completionHandler:]`, if usable
    in this embedding context;
  - otherwise `cacheDisplayInRect:toBitmapImageRep:` or another AppKit snapshot
    path that works for `WKWebView`;
  - if neither can capture WebKit contents, record the exact failure/error.
- Expose the hook only for diagnostics, for example behind an environment
  variable such as `TERMSURF_SURFARI_RENDER_PROOF_TRACE_FILE`.
- The hook should write machine-readable evidence for each tab/load:
  - tab id and pane id;
  - URL;
  - snapshot/capture method attempted;
  - whether the capture method was validated for this process/context;
  - image size;
  - deterministic color counts for the HTML control and WebKit green PDF
    fixture;
  - status: `pass`, `blank`, `capture-failed`, or `unsupported`.
- The hook must not classify a blank snapshot as a WebKit render failure unless
  the capture method can distinguish real blank content from capture failure. If
  that distinction is unavailable, the result must be Partial with the missing
  evidence named.
- Extend the Surfari Rust dispatcher only as much as necessary to pass pane/tab
  context through to the diagnostic output. Do not alter the TermSurf protobuf
  protocol.
- Add a focused harness, tentatively
  `scripts/test-issue-834-surfari-side-render-pixels.sh`, that reuses the
  Experiment 36 HTML and PDF controls and compares:
  - Surfari-side render proof;
  - Ghostboard AppKit presentation proof;
  - window/full-screen pixel proof.
- Update this experiment file with the result.

## Verification

Run formatting and hygiene checks:

```bash
cargo fmt
./surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
bash -n scripts/test-issue-834-surfari-side-render-pixels.sh
git diff --check
git -C webkit/src status --short
```

Run the focused diagnostic harness:

```bash
rm -rf logs/issue-834-exp37-surfari-side-render-pixels
scripts/test-issue-834-surfari-side-render-pixels.sh
```

Inspect the summary:

```bash
python3 - <<'PY'
import json
from pathlib import Path

summary = json.loads(
    Path(
        "logs/issue-834-exp37-surfari-side-render-pixels/"
        "surfari-side-render-pixels-summary.json"
    ).read_text()
)
print(json.dumps({
    "overall_result": summary.get("overall_result"),
    "classification": summary.get("classification"),
    "html_internal": summary.get("html", {}).get("internal_render"),
    "pdf_internal": summary.get("pdf", {}).get("internal_render"),
    "ghostboard_visible": summary.get("ghostboard_visible"),
    "cleanup": summary.get("cleanup"),
}, indent=2, sort_keys=True))
PY
```

Pass criteria:

- repo-built Surfari/libtermsurf_webkit/Ghostboard/WebTUI/WebKit artifacts are
  used;
- HTML and PDF controls both load through `web --browser surfari`;
- the Surfari-side diagnostic records render-pixel evidence for both controls;
- the summary classifies the failure layer into one of:
  - `ghostboard-compositing-gap`: Surfari-side pixels are present, but
    Ghostboard screenshots are blank;
  - `webkit-pdf-render-gap`: HTML Surfari-side pixels are present, PDF
    Surfari-side pixels are blank or failed;
  - `generic-webkit-render-gap`: neither HTML nor PDF Surfari-side pixels are
    present despite loading callbacks, and the capture method is validated well
    enough to distinguish true blank content from capture failure.
- Ghostboard AppKit presentation and nonzero CAContext evidence are still
  recorded;
- cleanup leaves no running app/browser/server process;
- `webkit/src` remains clean;
- design review and completion review are recorded.

Partial criteria:

- a diagnostic hook is implemented and runs, but one scenario lacks enough
  internal evidence to classify the layer;
- or the hook proves HTML but cannot test PDF due a WebKit/PDF snapshot
  limitation that is explicitly recorded;
- or the result is `capture-api-unsupported` and identifies the next viable
  proof method.
- or internal captures are blank, but the hook cannot distinguish real blank
  content from a capture method that cannot see `WKWebView` contents.

Failure criteria:

- the experiment modifies the TermSurf protobuf protocol;
- the hook becomes a broad product API instead of narrow diagnostic
  instrumentation;
- the harness does not compare internal Surfari evidence against Ghostboard
  visible-pixel evidence;
- installed artifacts are used instead of repo-built artifacts;
- cleanup leaves running processes.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Findings:

- `capture-api-unsupported` was allowed as both Pass and Partial;
- `generic-webkit-render-gap` could overclaim if the native capture API silently
  returned blank instead of actually proving that WebKit failed to render.

Resolution:

- `capture-api-unsupported` was removed from Pass criteria and remains Partial
  when it identifies the next viable proof method;
- `generic-webkit-render-gap` now requires a capture method validated well
  enough to distinguish true blank content from capture failure;
- blank internal captures with an unvalidated capture method are now explicitly
  Partial.

Follow-up verdict after fixes: **Approved**.

The reviewer found no remaining required design changes before the plan commit.
