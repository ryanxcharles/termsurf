# Experiment 35: Prove Basic Surfari PDF Rendering

## Description

Experiment 34 audited WebKit and Surfari PDF capabilities and concluded that the
first Surfari implementation step should be a real-app PDF load/render proof.
Before testing PDF input, links, forms, print, or WebKit-specific hooks, we need
objective evidence that a PDF can load through `web --browser surfari` and
render visibly inside the actual TermSurf/Ghostboard overlay.

This experiment should follow the Issue 756 real-app Surfari harness style. It
should not modify WebKit, Surfari product code, Ghostboard, WebTUI, Roamium,
protobuf, or Chromium unless the basic proof exposes a concrete integration bug
and a follow-up experiment is designed.

## Changes

- Add a focused harness, tentatively
  `scripts/test-issue-834-surfari-pdf-render.sh`.
- The harness should:
  - require the same repo-built artifacts as the Issue 756 Surfari real-app
    harnesses:
    - `ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf`;
    - `target/debug/web`;
    - `target/debug/surfari`;
    - `webkit/src/WebKitBuild/Debug/WebKit.framework`;
    - `surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib`;
  - create a temporary deterministic PDF fixture with high-contrast visual
    content and a known title/marker;
  - serve it over local HTTP with `Content-Type: application/pdf`;
  - launch the Debug TermSurf app with:
    - `TERMSURF_SURFARI_PATH` pointing to the repo-built Surfari binary;
    - `DYLD_FRAMEWORK_PATH` pointing to `webkit/src/WebKitBuild/Debug`;
    - Surfari/geometry/WebTUI trace files under
      `logs/issue-834-exp35-surfari-pdf-render/`;
  - run the repo-built `web --browser surfari <pdf-url>` as the initial command;
  - wait for Ghostboard `SetOverlay`, Surfari `ServerRegister`, `BrowserReady`,
    AppKit presentation, Surfari `create-tab`, loading/title callbacks, and a
    nonzero CAContext;
  - capture a screenshot of the target app window;
  - perform a deterministic pixel or crop analysis that proves the PDF fixture,
    not just a blank WebKit view, is visible in the overlay;
  - close the Surfari browser tab cleanly through the TermSurf socket and clean
    up temporary files/processes.
- The harness should write a compact machine-readable summary, tentatively
  `surfari-pdf-render-summary.json`, with:
  - `overall_result`;
  - `first_failing_hop`;
  - artifact paths;
  - fixture URL/path;
  - Surfari/WebTUI/Ghostboard trace evidence;
  - CAContext and presented overlay evidence;
  - screenshot path;
  - pixel-proof statistics;
  - cleanup result.
- Update this experiment file with the result.
- Do not add a broad Surfari PDF regression runner yet. That should wait until
  at least basic render and one or two input/navigation rows are proven.

## Verification

Run syntax/hygiene checks:

```bash
bash -n scripts/test-issue-834-surfari-pdf-render.sh
git diff --check
git -C webkit/src status --short
```

Run the focused probe:

```bash
rm -rf logs/issue-834-exp35-surfari-pdf-render
scripts/test-issue-834-surfari-pdf-render.sh
```

Inspect the summary:

```bash
python3 - <<'PY'
import json
from pathlib import Path

summary = json.loads(
    Path(
        "logs/issue-834-exp35-surfari-pdf-render/"
        "surfari-pdf-render-summary.json"
    ).read_text()
)
print(json.dumps({
    "overall_result": summary.get("overall_result"),
    "first_failing_hop": summary.get("first_failing_hop"),
    "ca_context": summary.get("ca_context"),
    "pixel_proof": summary.get("pixel_proof"),
    "cleanup": summary.get("cleanup"),
}, indent=2, sort_keys=True))
PY
```

Pass criteria:

- the harness exits `0`;
- the summary records `overall_result = "pass"` and
  `first_failing_hop = "no-failure-observed"`;
- Ghostboard launches repo-built Surfari through `TERMSURF_SURFARI_PATH`;
- Surfari runs with repo WebKit through `DYLD_FRAMEWORK_PATH`;
- Surfari registers as browser `surfari` and receives a `CreateTab` for the PDF
  URL;
- WebTUI renders Surfari ready state;
- Surfari emits a nonzero CAContext and Ghostboard presents a nonzero overlay;
- the screenshot/pixel proof shows deterministic PDF fixture content visible
  inside the overlay;
- title/URL/loading state is recorded if WebKit exposes it for the PDF;
- the browser tab closes cleanly and no native OS print/menu UI is opened;
- `webkit/src` remains clean;
- markdown is formatted with Prettier;
- design review and completion review are recorded.

Partial criteria:

- Surfari launches and receives the PDF URL, but rendering is blank or pixel
  proof is inconclusive;
- or the PDF renders but title/loading state is missing in a way that needs a
  follow-up classification.

Failure criteria:

- the harness uses installed Surfari/WebTUI/Ghostboard instead of repo-built
  artifacts;
- the proof is based only on process logs and does not show visible PDF pixels;
- the harness loads HTML instead of a real `application/pdf` response;
- WebKit, Surfari product code, Ghostboard, WebTUI, Roamium, protobuf, or
  Chromium source is changed;
- native print or context-menu UI is opened;
- cleanup leaves a running app/browser process.

## Design Review

An external Codex review checked the design.

Verdict: **Approved**.

The review found no findings. It confirmed that Experiment 35 is the correct
next step after the Surfari/WebKit audit, that the scope is narrow and safe, and
that the pass/failure criteria require repo-built artifacts, a real
`application/pdf` response, nonzero CAContext/overlay evidence, visible
deterministic PDF pixels, clean shutdown, no native print/menu UI, and no
WebKit/product source changes.

## Result

**Result:** Partial

The focused Surfari PDF harness was implemented as
`scripts/test-issue-834-surfari-pdf-render.sh` and run against repo-built
artifacts:

- Ghostboard debug app:
  `ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf`;
- WebTUI: `target/debug/web`;
- Surfari: `target/debug/surfari`;
- WebKit: `webkit/src/WebKitBuild/Debug`;
- libtermsurf_webkit:
  `surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib`.

The final run used WebKit's own PDF fixture,
`webkit/src/WebKitBuild/Debug/TestWebKitAPIResources.bundle/Contents/Resources/multiple-pages-colored.pdf`,
served over local HTTP as `application/pdf`.

Run:

```bash
rm -rf logs/issue-834-exp35-surfari-pdf-render
scripts/test-issue-834-surfari-pdf-render.sh
```

Result summary:

- run id: `20260622-184950`;
- summary:
  `logs/issue-834-exp35-surfari-pdf-render/surfari-pdf-render-summary.json`;
- app log: `logs/issue-834-exp35-surfari-pdf-render/app-20260622-184950.log`;
- Surfari trace:
  `logs/issue-834-exp35-surfari-pdf-render/surfari-trace-20260622-184950.log`;
- window screenshot:
  `logs/issue-834-exp35-surfari-pdf-render/screenshot-20260622-184950.png`;
- full-screen screenshot:
  `logs/issue-834-exp35-surfari-pdf-render/screenshot-full-20260622-184950.png`;
- pixel proof:
  `logs/issue-834-exp35-surfari-pdf-render/pixel-proof-20260622-184950.json`.

Passing hops:

- WebTUI discovered `TERMSURF_SOCKET`;
- `web --browser surfari` requested a Surfari PDF overlay;
- Ghostboard resolved Surfari through `TERMSURF_SURFARI_PATH`;
- Ghostboard spawned the repo-built Surfari binary with `--browser-name=surfari`
  and the default WebKit profile directory;
- Surfari registered as browser `surfari`;
- Ghostboard sent `CreateTab` with the PDF URL;
- Ghostboard emitted `BrowserReady`;
- WebTUI rendered the Surfari ready state;
- Surfari trace initialized and recorded the PDF URL;
- Surfari emitted nonzero `CAContext` values;
- Surfari reported loading `state=done`;
- Ghostboard/AppKit logged overlay presentation and nonzero presented pixels.

Failing hop:

- the visible PDF pixel proof failed. The WebKit test fixture should show a
  large green PDF page, but both capture modes showed only the terminal/WebTUI
  chrome:
  - window capture: `webkit_green = 0`;
  - full-screen capture: `webkit_green = 47`;
  - required minimum: `webkit_green >= 5000`.

Cleanup evidence:

- the failing path now calls process cleanup before writing the summary;
- the summary records `cleanup.ran = "true"`;
- the summary records `cleanup.app_status = "terminated"` for PID `34880`;
- the summary records `cleanup.server_status = "terminated"` for PID `34837`;
- a direct `ps -p 34880,34837 -o pid=,comm=` check after the run produced no
  process rows.

This is a partial result because the browser/profile/protocol/compositing hops
are all present, but the experiment did not prove visible Surfari PDF rendering.
The same screenshot behavior was observed in prior Surfari HTML smoke artifacts:
the screenshots captured terminal chrome and ready state but not WebKit page
pixels. Experiment 35 therefore uncovered a more basic Surfari visual-content
proof gap that must be diagnosed before the PDF feature matrix can continue.

## Conclusion

Surfari reaches the PDF URL and exports a CAContext, and Ghostboard believes it
has presented the overlay at nonzero size. However, neither the window capture
nor the full-screen capture shows the WebKit-rendered PDF content. The next
experiment should isolate whether this is:

- a Surfari/WebKit PDF rendering problem;
- a generic Surfari page-rendering/compositing problem;
- a `CAContext`/`CALayerHost` attachment or ordering problem in Ghostboard;
- a screenshot/capture limitation that also affects non-PDF Surfari content.

The next experiment should use a non-PDF HTML control and a PDF control in the
same harness, then compare Surfari's native WebKit snapshot or direct
`WKWebView` evidence against Ghostboard's visible overlay evidence.

## Completion Review

An external Codex review checked the completed experiment result and harness.

Initial verdict: **Changes required**.

Findings:

- the completion review had not yet been recorded in this file;
- the harness summary did not record cleanup result on the failing/Partial path,
  because the summary was written before the `EXIT` trap cleanup ran.

Resolution:

- this completion-review section records the review;
- `scripts/test-issue-834-surfari-pdf-render.sh` now performs process cleanup
  before writing the final summary on both pass and fail paths;
- the summary now records `cleanup.ran`, `cleanup.app_status`, and
  `cleanup.server_status`;
- the final run `20260622-184950` shows both app and server process cleanup as
  `terminated`.

The reviewer also confirmed that the Partial classification itself is supported:
the logs show repo-built Surfari launched, received the PDF URL, emitted a
nonzero CAContext, reached loading done, and failed only at the visible PDF
pixel proof.

Follow-up verdict after fixes: **Approved**.

The reviewer found no remaining findings and confirmed that the prior completion
review and cleanup-summary gaps were resolved.
