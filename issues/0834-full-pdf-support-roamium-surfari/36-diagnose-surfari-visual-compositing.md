# Experiment 36: Diagnose Surfari Visual Compositing

## Description

Experiment 35 proved that Surfari can be launched by Ghostboard, receive a PDF
URL, report loading done, export a nonzero `CAContext`, and trigger Ghostboard's
AppKit overlay presentation path. However, both window and full-screen
screenshots showed only terminal/WebTUI chrome, not the WebKit-rendered PDF
content.

Before continuing the Surfari PDF matrix, we need to identify which layer is
actually failing. The next experiment should distinguish these possibilities:

- Surfari's `WKWebView` is not rendering page/PDF pixels at all;
- Surfari renders HTML but not PDF;
- Surfari renders inside its process, but the `CAContext`/`CALayerHost` exported
  to Ghostboard is blank, detached, ordered behind the terminal, or otherwise
  not visible;
- screenshot capture is blind to Surfari's hosted layer, even when a human would
  see it;
- the current Ghostboard presentation evidence logs geometry but does not prove
  composited content.

This experiment should be diagnostic only. It should not modify WebKit, Surfari,
Ghostboard, WebTUI, protobuf, Roamium, or Chromium product code unless the
diagnostic harness exposes a small, concrete instrumentation gap that cannot be
answered otherwise.

## Changes

- Add a focused diagnostic harness, tentatively
  `scripts/test-issue-834-surfari-visual-compositing.sh`.
- The harness should launch the repo-built Ghostboard, WebTUI, Surfari, WebKit,
  and `libtermsurf_webkit` artifacts, as Experiment 35 did.
- The harness should run at least two real app scenarios:
  - an HTML control page with large deterministic color blocks and a distinctive
    title;
  - the WebKit `multiple-pages-colored.pdf` PDF fixture used by Experiment 35.
- For each scenario, the harness should collect:
  - Ghostboard logs for `SetOverlay`, `BrowserReady`, `CaContext`, AppKit
    `presented`, and `presented_pixels`;
  - Surfari trace logs for `create-tab`, `url-changed`, `title-changed`,
    `loading-state-callback`, and `ca-context`;
  - WebTUI state trace for browser-ready state;
  - window screenshot and full-screen screenshot;
  - pixel classifications for the deterministic HTML colors and the PDF green
    fixture;
  - process cleanup status.
- Add native Surfari-side render evidence if feasible without product-code
  changes:
  - first search for existing `libtermsurf_webkit`/Surfari test hooks that can
    snapshot a `WKWebView` or report rendered content state;
  - if no existing hook exists, document that gap rather than guessing;
  - do not add a new product API in this experiment unless a follow-up design
    explicitly justifies it.
- Compare the HTML and PDF results in one machine-readable summary, tentatively
  `surfari-visual-compositing-summary.json`.
- Update this experiment file with the result.

## Verification

Run syntax/hygiene checks:

```bash
bash -n scripts/test-issue-834-surfari-visual-compositing.sh
git diff --check
git -C webkit/src status --short
```

Run the diagnostic harness:

```bash
rm -rf logs/issue-834-exp36-surfari-visual-compositing
scripts/test-issue-834-surfari-visual-compositing.sh
```

Inspect the summary:

```bash
python3 - <<'PY'
import json
from pathlib import Path

summary = json.loads(
    Path(
        "logs/issue-834-exp36-surfari-visual-compositing/"
        "surfari-visual-compositing-summary.json"
    ).read_text()
)
print(json.dumps({
    "overall_result": summary.get("overall_result"),
    "classification": summary.get("classification"),
    "html": summary.get("html", {}).get("pixel_proof"),
    "pdf": summary.get("pdf", {}).get("pixel_proof"),
    "native_snapshot_hook": summary.get("native_snapshot_hook"),
    "cleanup": summary.get("cleanup"),
}, indent=2, sort_keys=True))
PY
```

Pass criteria:

- the harness exits `0`;
- repo-built Ghostboard, WebTUI, Surfari, WebKit, and `libtermsurf_webkit`
  artifacts are used;
- the HTML control and PDF fixture are both loaded through
  `web --browser surfari`;
- each scenario records browser-ready, loading, nonzero CAContext, AppKit
  presentation, screenshots, pixel classifications, and cleanup;
- the summary classifies the failure layer into one of:
  - `capture-only-gap`: native/onscreen evidence proves content but screenshots
    cannot capture it;
  - `pdf-only-render-gap`: HTML is visible/proven but PDF is not;
  - `generic-surfari-render-gap`: neither HTML nor PDF content is visible or
    proven despite successful protocol hops;
  - `ghostboard-compositing-gap`: Surfari renders internally but Ghostboard does
    not visibly host it;
- no product source changes are made;
- no native print or context-menu UI is opened;
- `webkit/src` remains clean;
- markdown is formatted with Prettier;
- design review and completion review are recorded.

Partial criteria:

- the harness proves the HTML/PDF split but cannot collect native Surfari-side
  render evidence;
- or screenshot/capture behavior remains ambiguous, but the missing evidence is
  explicitly named and the next experiment is clear.
- or the summary classifies the result as `inconclusive`, with the missing
  evidence named explicitly.

Failure criteria:

- the harness tests only PDF and does not include an HTML control;
- the harness relies only on process logs and does not capture or classify
  visible pixels;
- installed artifacts are used instead of repo-built artifacts;
- product code is changed without a follow-up experiment design;
- cleanup leaves a running app/browser/server process.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Finding:

- the Pass criteria incorrectly allowed an `inconclusive` classification, which
  would let the experiment pass without actually distinguishing screenshot
  limitations, generic Surfari rendering, PDF-specific rendering, or Ghostboard
  compositing.

Resolution:

- `inconclusive` was removed from Pass criteria;
- `inconclusive` is now only a Partial result when the missing evidence is named
  explicitly and the next experiment remains clear.

Follow-up verdict after fixes: **Approved**.

The reviewer found no remaining required design changes before the plan commit.

## Result

**Result:** Pass

The diagnostic harness was implemented as
`scripts/test-issue-834-surfari-visual-compositing.sh` and run against
repo-built artifacts:

- Ghostboard debug app:
  `ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf`;
- WebTUI: `target/debug/web`;
- Surfari: `target/debug/surfari`;
- WebKit: `webkit/src/WebKitBuild/Debug`;
- libtermsurf_webkit:
  `surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib`.

Run:

```bash
rm -rf logs/issue-834-exp36-surfari-visual-compositing
scripts/test-issue-834-surfari-visual-compositing.sh
```

Result summary:

- run id: `20260622-190121`;
- summary:
  `logs/issue-834-exp36-surfari-visual-compositing/surfari-visual-compositing-summary.json`;
- harness log:
  `logs/issue-834-exp36-surfari-visual-compositing/harness-20260622-190121.log`;
- server log:
  `logs/issue-834-exp36-surfari-visual-compositing/server-20260622-190121.log`.

The harness tested two scenarios through `web --browser surfari`:

- HTML control: `http://127.0.0.1:57354/index.html`;
- PDF control: `http://127.0.0.1:57354/surfari-render.pdf`, backed by WebKit's
  `multiple-pages-colored.pdf` fixture.

Both scenarios passed the protocol/compositing setup checks:

- WebTUI discovered `TERMSURF_SOCKET`;
- `web --browser surfari` requested the Surfari overlay;
- Ghostboard resolved Surfari through `TERMSURF_SURFARI_PATH`;
- Ghostboard spawned the repo-built Surfari binary;
- Surfari registered as browser `surfari`;
- Ghostboard emitted `BrowserReady`;
- WebTUI rendered the Surfari ready state;
- Surfari recorded the URL, loading done, and nonzero CAContext;
- Ghostboard/AppKit logged overlay presentation and nonzero presented pixels;
- process cleanup terminated each scenario app and the HTTP server.

Both scenarios failed visible-pixel proof:

- HTML control:
  - window screenshot:
    `logs/issue-834-exp36-surfari-visual-compositing/screenshot-html-20260622-190121.png`;
  - full-screen screenshot:
    `logs/issue-834-exp36-surfari-visual-compositing/screenshot-full-html-20260622-190121.png`;
  - pixel proof:
    `logs/issue-834-exp36-surfari-visual-compositing/pixel-proof-html-20260622-190121.json`;
  - target counts: `magenta = 0`, `cyan = 0`, `yellow = 0` in the window
    capture; `magenta = 0`, `cyan = 33`, `yellow = 12` in the full-screen
    capture; required minimum was `5000` per target.
- PDF control:
  - window screenshot:
    `logs/issue-834-exp36-surfari-visual-compositing/screenshot-pdf-20260622-190121.png`;
  - full-screen screenshot:
    `logs/issue-834-exp36-surfari-visual-compositing/screenshot-full-pdf-20260622-190121.png`;
  - pixel proof:
    `logs/issue-834-exp36-surfari-visual-compositing/pixel-proof-pdf-20260622-190121.json`;
  - target counts: `webkit_green = 16` in the window capture and
    `webkit_green = 69` in the full-screen capture; required minimum was `5000`.

The summary classified the result as `generic-surfari-render-gap` with
`overall_result = "pass"`. The harness also checked for an existing Surfari-side
snapshot hook and recorded `native_snapshot_hook = "not-found"`.

Verification after the run:

```bash
bash -n scripts/test-issue-834-surfari-visual-compositing.sh
git diff --check
git -C webkit/src status --short
ps -ax -o pid=,comm= | rg 'TermSurf|surfari|server.py|termsurf-issue834-exp36' || true
```

The syntax and diff checks passed, `webkit/src` stayed clean, and the process
check produced no matching rows.

## Conclusion

The Surfari blank-content problem is not PDF-specific. A deterministic HTML page
and a deterministic PDF both reached Surfari loading callbacks, nonzero
CAContext export, and Ghostboard/AppKit presentation, but neither produced
visible page pixels in window or full-screen captures.

The next experiment should diagnose the generic Surfari visual gap. The most
likely next step is to add explicit Surfari-side render proof, such as a
temporary diagnostic `WKWebView` snapshot or layer-tree/content-state hook, then
compare that internal render evidence with Ghostboard's `CALayerHost`
presentation evidence. That follow-up should be designed separately before
changing Surfari/libtermsurf_webkit product code.

## Completion Review

An external Codex review checked the completed experiment result and harness.

Initial verdict: **Changes required**.

Findings:

- the completion review had not yet been recorded in this file;
- the verification snippet looked for `summary.get("surfari_snapshot")`, but the
  harness and summary use `native_snapshot_hook`.

Resolution:

- this completion-review section records the review;
- the verification snippet now inspects `native_snapshot_hook`, matching
  `scripts/test-issue-834-surfari-visual-compositing.sh` and the recorded
  summary.

The reviewer also confirmed that the core result is otherwise supported: both
HTML and PDF loaded through repo-built Surfari with nonzero CAContext and AppKit
presentation, both failed deterministic visible-pixel proof, and the conclusion
does not overclaim internal WebKit rendering given
`native_snapshot_hook = "not-found"`.

Follow-up verdict after fixes: **Approved**.

The reviewer found no remaining required changes before the result commit.
