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
    "surfari_snapshot": summary.get("surfari_snapshot"),
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
