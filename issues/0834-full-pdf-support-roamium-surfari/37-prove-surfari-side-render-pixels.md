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

## Result

**Result:** Pass

Experiment 37 added an env-gated Surfari render-proof diagnostic:

- `surfari/libtermsurf_webkit` now exposes a diagnostic render-probe callback;
- when `TERMSURF_SURFARI_RENDER_PROOF_TRACE_FILE` is set, Surfari registers that
  callback;
- after `WKWebView` load completion, the WebKit wrapper calls
  `WKWebView.takeSnapshot`, counts deterministic colors, and reports the counts
  back to Rust;
- Rust logs the render proof with tab id, pane id, URL, method, status, image
  size, color counts, and error text.

The hook is not part of the TermSurf protobuf protocol and is inactive unless
the diagnostic environment variable is set.

The focused harness was added as
`scripts/test-issue-834-surfari-side-render-pixels.sh`. It reuses the Experiment
36 HTML/PDF real-app scenarios, exports
`TERMSURF_SURFARI_RENDER_PROOF_TRACE_FILE`, parses the Surfari-side render
proof, and compares it against Ghostboard-visible screenshot pixel proof.

Run:

```bash
rm -rf logs/issue-834-exp37-surfari-side-render-pixels \
  logs/issue-834-exp36-surfari-visual-compositing
scripts/test-issue-834-surfari-side-render-pixels.sh
```

Result summary:

- run id: `20260622-192036`;
- summary:
  `logs/issue-834-exp37-surfari-side-render-pixels/surfari-side-render-pixels-summary.json`;
- render proof trace:
  `logs/issue-834-exp37-surfari-side-render-pixels/surfari-render-proof-20260622-192036.log`;
- harness log:
  `logs/issue-834-exp37-surfari-side-render-pixels/harness-20260622-192036.log`;
- Experiment 36 summary:
  `logs/issue-834-exp36-surfari-visual-compositing/surfari-visual-compositing-summary.json`.

Surfari-side internal render proof passed:

- HTML control:
  - method: `WKWebView.takeSnapshot`;
  - status: `pass`;
  - size: `3712x2176`;
  - `cyan = 270400`;
  - `yellow = 270400`.
- PDF control:
  - method: `WKWebView.takeSnapshot`;
  - status: `pass`;
  - size: `3712x2176`;
  - `webkit_green = 1693120`.

Ghostboard-visible pixel proof still failed:

- HTML screenshot pixel status: `fail`;
- PDF screenshot pixel status: `fail`;
- no visible deterministic HTML or PDF color target reached the required
  threshold in window or full-screen screenshots.

The summary classified the result as `ghostboard-compositing-gap` with
`overall_result = "pass"`.

Verification:

```bash
./surfari/libtermsurf_webkit/build.sh
cargo fmt -p surfari
cargo build -p surfari
bash -n scripts/test-issue-834-surfari-side-render-pixels.sh
git diff --check
git -C webkit/src status --short
ps -ax -o pid=,comm= | rg 'TermSurf|surfari|server.py|termsurf-issue834-exp3[67]' || true
```

The wrapper and Surfari binary built successfully. Formatting, shell syntax, and
diff checks passed. `webkit/src` stayed clean. The process check produced no
matching rows. The summary records both scenario processes and the server as
`terminated`.

## Conclusion

The Surfari PDF blankness is not a WebKit/PDF rendering failure. Surfari's own
`WKWebView` renders both the HTML control and the PDF fixture with the expected
pixels. The failure is between Surfari's rendered `WKWebView` layer and
Ghostboard-visible composition.

The next experiment should diagnose the Ghostboard/CAContext hosting path:
whether `CAContext.remoteContext.layer = web_view.layer` is sufficient for a
hidden/transparent source window, whether the exported layer needs a wrapper
layer, whether Ghostboard's `CALayerHost` is attached or ordered incorrectly, or
whether the source window/layer visibility/lifetime is invalid for cross-process
display.

## Completion Review

An external Codex review checked the completed experiment result and harness.

Initial verdict: **Changes required**.

Finding:

- the completion review had not yet been recorded in this file.

Resolution:

- this completion-review section records the review.

The reviewer found no other required issues. It confirmed that the env-gated
runtime behavior is scoped correctly, Surfari only registers
`ts_set_on_render_probe` when `TERMSURF_SURFARI_RENDER_PROOF_TRACE_FILE` is set,
the Objective-C++ snapshot path produced real internal pixels for both controls,
Rust traces match the expected tab/pane/URL evidence, Ghostboard-visible pixels
still fail, and the `ghostboard-compositing-gap` classification is supported.

Follow-up verdict after fixes: **Approved**.

The reviewer found no remaining required changes before the result commit.
