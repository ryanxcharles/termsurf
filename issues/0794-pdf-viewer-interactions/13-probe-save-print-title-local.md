# Experiment 13: Probe Save, Print, Title, and Local-File Parity

## Description

Experiment 12 fixed the remaining safe in-page toolbar controls: zoom, fit, and
rotate now work through real toolbar clicks, and Experiment 10 had already
proved page selector navigation. The remaining Issue 794 surfaces are the
browser-side PDF viewer actions and metadata paths that cannot be validated by
simple visual screenshots:

- save/download;
- print;
- PDF document title propagation;
- local `file://` and extensionless local PDF parity.

These paths are risky to test naively. A blind save click may open a native
panel or write outside the test directory. A blind print click may open a native
print dialog. This experiment therefore starts as a contained diagnostic and
only clicks controls when the harness proves the side effect is confined to a
per-run log directory or to a non-native test path.

The expected result may be Partial. If save/download or print require more
Electron-style browser infrastructure, this experiment should identify the exact
missing layer and design the next implementation experiment around that layer.
It must not silently grow into a broad save/print implementation.

This experiment must receive Codex design review before implementation. After
the result is recorded, Codex must review the completed output before the next
experiment is designed.

## Changes

1. Create a new Chromium branch only if Chromium product code or Chromium
   diagnostics are required.

   If implementation only changes scripts and issue documentation, no Chromium
   branch is needed. If Chromium is touched, fork the current good branch:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-794-exp12
   git checkout -b 148.0.7778.97-issue-794-exp13
   ```

   Add the branch to `chromium/README.md` with a description such as "Probe PDF
   save, print, title, and local-file parity."

2. Extend the toolbar harness with a side-effect-safe probe mode.

   Extend `scripts/probe-pdf-toolbar.mjs` and
   `scripts/test-issue-794-pdf-toolbar.py`, or add focused companion scripts, to
   run:

   ```bash
   LOG_DIR="logs/issue-794-exp13-save-print-title-local-$(date +%Y%m%d-%H%M%S)" \
     scripts/test-issue-794-pdf-toolbar.py \
     --probe save-print-title-local \
     --log-dir "$LOG_DIR" \
     --serve-bitcoin-pdf
   ```

   The probe must record:
   - PDF extension child target URL and title;
   - top-level target URL and title;
   - toolbar inventory for download/save and print controls;
   - whether `printingEnabled`, `pdfUseShowSaveFilePicker`,
     `pdfGetSaveDataInBlocks`, and related PDF load-time flags are present in
     the live viewer;
   - whether the viewer exposes `chrome.pdfViewerPrivate` and the functions
     involved in title/local-file/save behavior;
   - current `chrome.runtime.lastError` after each probed API call;
   - browser logs for `OnSaveURL()`, `SetPluginCanSave()`, and
     `UpdateContentRestrictions()`;
   - whether any native dialog is avoided by construction.

   Native dialog detection is best-effort only. The primary safety rule is:
   never click save/download or print unless the harness has already proven a
   concrete non-native containment path for that specific control. A missing
   native-dialog observation is not proof of containment.

3. Save/download containment.

   Before clicking the download/save control, the harness must install a
   controlled download directory under the log directory using the narrowest
   available DevTools mechanism, for example `Browser.setDownloadBehavior` or
   `Page.setDownloadBehavior` when available for this Chromium build.

   The save/download probe may click the toolbar control only if all of these
   are true:
   - a per-run download directory exists under the log directory;
   - DevTools confirms download behavior was set successfully, or another
     concrete containment/interception mechanism is active and recorded;
   - the PDF viewer action path does not rely on a native save panel according
     to the live viewer flags and observed API path.

   If `Browser.setDownloadBehavior` / `Page.setDownloadBehavior` is unavailable
   and no replacement containment mechanism is active, the harness must not
   click save/download. Record `download-not-contained`.

   The result must distinguish:
   - `download-file-created`: a PDF-like file appears in the controlled download
     directory;
   - `download-browser-callback-only`: the click reaches a browser callback such
     as `OnSaveURL()` or controlled download events, but no file is created;
   - `download-not-contained`: containment could not be proven, so the control
     was not clicked;
   - `download-native-dialog`: a native dialog or uncontrolled write was
     observed, which is a failure for this experiment;
   - `download-no-op`: the control was clicked, containment was active, and no
     file, browser callback, download event, or error was observed.

4. Print containment.

   Do not open a native print dialog. The print probe should first determine
   whether the viewer is configured to expose print at all:
   - inspect `printingEnabled` from PDF load-time data;
   - inspect toolbar print control visibility and disabled state;
   - inspect `UpdateContentRestrictions()` logs for print restrictions;
   - if practical, use a non-native print path such as headless/DevTools
     `Page.printToPDF` only as a control check, not as proof that the toolbar
     print button works.

   The toolbar print control may be clicked only if the harness can prove a
   non-native or intercepted print path is active. Otherwise record
   `print-not-contained` and do not click it.

   The result must distinguish:
   - `print-ready-disabled-by-flags`: print is hidden or disabled because
     TermSurf's PDF strings/data currently set `printingEnabled=false`;
   - `print-restricted-by-document`: the document restrictions disable print;
   - `print-contained-callback`: a contained click reaches a browser/plugin
     print callback without a native dialog;
   - `print-not-contained`: containment could not be proven, so the control was
     not clicked;
   - `print-native-dialog`: a native print dialog opened, which is a failure for
     this experiment;
   - `print-no-op`: containment was active, the control was clicked, and no
     callback, event, or error was observed.

5. Title propagation probe.

   Record the title state at each layer after loading:
   - the Bitcoin PDF fixture, which has a normal filename/title fallback;
   - an explicitly untitled PDF fixture created for this experiment, or a
     minimal fixture whose expected behavior is the URL/filename fallback.

   For each title fixture, record:
   - top-level DevTools target title;
   - PDF extension child target title;
   - `document.title` in the PDF extension frame;
   - any `chrome.pdfViewerPrivate.setPdfDocumentTitle(...)` availability and
     call behavior;
   - TermSurf title callbacks or `TitleChanged` protocol logs if available;
   - webtui-visible title if the existing automation exposes it.

   The probe must classify title behavior as one of:
   - `title-propagated`: the user-visible tab/title state matches the PDF title
     or expected URL fallback;
   - `title-extension-only`: the PDF extension target has the title but the
     top-level/TermSurf title does not;
   - `title-api-missing`: the viewer cannot call
     `pdfViewerPrivate.setPdfDocumentTitle`;
   - `title-unobserved`: automation cannot observe the user-visible title.

6. Local-file parity probe.

   Run the same render and interaction checks against:
   - `http://127.0.0.1:<port>/bitcoin.pdf`;
   - `file:///Users/ryan/dev/termsurf/test-html/public/bitcoin.pdf`;
   - an extensionless copy under the log directory, such as
     `$LOG_DIR/fixtures/bitcoin-extensionless`.

   For the extensionless fixture, the local server should serve it with
   `Content-Type: application/pdf` and the `file://` path should test the real
   local extensionless behavior. Do not treat HTTP MIME success as proof of
   local extensionless parity.

   Required checks for each URL:
   - visible PDF rendering;
   - wheel scroll changes viewer state or screenshot;
   - page selector navigation changes page;
   - toolbar zoom still works;
   - title classification from step 5.

   If a local path downloads, renders as text, or fails to enter the PDF viewer,
   record the exact first failing layer.

7. Keep implementation scope diagnostic.

   This experiment may add targeted logs or harness code. It should not
   implement a broad save/download manager, native file picker replacement,
   print preview stack, or new TermSurf protocol message.

   If the diagnostic proves a tiny, isolated fix is required only to make the
   probe observable, stop and update the experiment before making that product
   change. Do not hide implementation work inside a diagnostic experiment.

8. Build and format.

   If Chromium changes, build:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Then rebuild Roamium if needed:

   ```bash
   cd /Users/ryan/dev/termsurf
   ./scripts/build.sh roamium
   ```

   If Rust changes, run `cargo fmt` and accept its output. If Markdown changes,
   run Prettier. If Chromium C++ changes, run Chromium formatting on modified
   files if practical.

9. Codex review.

   Codex must review:
   - the experiment design before implementation;
   - the completed result, logs, and any diffs before Experiment 14 is designed.

## Verification

1. Save/download table:

   | Check                         | Result | Evidence |
   | ----------------------------- | ------ | -------- |
   | Save/download control found   |        |          |
   | Controlled download directory |        |          |
   | DevTools containment method   |        |          |
   | Control clicked               |        |          |
   | Browser callback/event        |        |          |
   | File created                  |        |          |
   | Classification                |        |          |

2. Print table:

   | Check                        | Result | Evidence |
   | ---------------------------- | ------ | -------- |
   | Print control found          |        |          |
   | `printingEnabled` value      |        |          |
   | Content restrictions         |        |          |
   | Non-native containment proof |        |          |
   | Control clicked              |        |          |
   | Callback/event               |        |          |
   | Classification               |        |          |

3. Title table:

   | Fixture                            | Layer                          | Observed title/state | Evidence |
   | ---------------------------------- | ------------------------------ | -------------------- | -------- |
   | Bitcoin PDF                        | Top-level DevTools target      |                      |          |
   | Bitcoin PDF                        | PDF extension child target     |                      |          |
   | Bitcoin PDF                        | PDF extension `document.title` |                      |          |
   | Bitcoin PDF                        | TermSurf title/protocol        |                      |          |
   | Bitcoin PDF                        | Classification                 |                      |          |
   | Untitled/fallback PDF              | Top-level DevTools target      |                      |          |
   | Untitled/fallback PDF              | PDF extension child target     |                      |          |
   | Untitled/fallback PDF              | PDF extension `document.title` |                      |          |
   | Untitled/fallback PDF              | TermSurf title/protocol        |                      |          |
   | Untitled/fallback PDF              | Classification                 |                      |          |
   | `setPdfDocumentTitle` availability | N/A                            |                      |          |

4. Local-file parity table:

   | URL kind                   | Render | Scroll | Page nav | Zoom | Title | First failing layer |
   | -------------------------- | ------ | ------ | -------- | ---- | ----- | ------------------- |
   | HTTP PDF fixture           |        |        |          |      |       |                     |
   | `file://` PDF fixture      |        |        |          |      |       |                     |
   | HTTP extensionless fixture |        |        |          |      |       |                     |
   | `file://` extensionless    |        |        |          |      |       |                     |

5. Regression checks:

   Re-run or reuse the existing automated probes for:
   - PDF wheel scroll;
   - PDF keyboard select/copy;
   - PDF drag selection;
   - PDF resize/reflow;
   - toolbar zoom/final controls from Experiment 12;
   - normal HTML click smoke.

6. Codex must review the completed output.

   Do not proceed to Experiment 14 until real issues from Codex's completion
   review are addressed.

## Pass Criteria

Experiment 13 passes if:

- save/download is either proven to create a file inside the controlled
  directory or proven to reach a specific browser-side callback/event without
  native UI;
- print is classified without opening a native print dialog;
- title propagation is classified at each observable layer;
- HTTP, local `file://`, and extensionless PDF behavior are tested and
  classified;
- prior passing PDF interactions remain working;
- no native save or print dialog is opened and no uncontrolled file write
  occurs.

## Partial Criteria

Experiment 13 is partial if:

- save/download or print cannot be safely clicked, but the experiment proves why
  and identifies the next implementation layer;
- local `file://` or extensionless local PDFs fail, but the first failing layer
  is identified;
- title propagation remains incomplete, but the missing layer is identified;
- the harness can classify the remaining surfaces but needs a follow-up product
  change to make them pass.

## Failure Criteria

Experiment 13 fails if:

- it opens a native save or print dialog during automation;
- it writes downloads outside the per-run log directory;
- it treats HTTP PDF success as proof of local `file://` or extensionless
  parity;
- it treats `Page.printToPDF` success as proof that the PDF viewer toolbar print
  button works;
- it claims save/download works without a file, browser callback, controlled
  download event, or explicit unsupported classification;
- it claims title propagation works from only the PDF extension child title;
- it regresses any prior passing PDF interaction without recording the
  regression and stopping;
- it uses installed/stable Roamium instead of repo-built debug Roamium;
- it omits Codex design or completion review.

## Result

**Result:** Partial

Experiment 13 added and ran a side-effect-safe diagnostic probe:

```bash
LOG_DIR="logs/issue-794-exp13-save-print-title-local-20260530-115054" \
  scripts/test-issue-794-pdf-toolbar.py \
  --probe save-print-title-local \
  --log-dir "$LOG_DIR" \
  --serve-bitcoin-pdf
```

The probe used repo-built debug Roamium, installed controlled DevTools download
behavior before clicking save/download, and did not click print because no
non-native/intercepted print containment path was proven.

Save/download result:

| Check                         | Result                  | Evidence                                                  |
| ----------------------------- | ----------------------- | --------------------------------------------------------- |
| Save/download control found   | Yes                     | summary records `controlFound=true`                       |
| Controlled download directory | Yes                     | `save-print-title-local/downloads/`                       |
| DevTools containment method   | Yes                     | `Browser.setDownloadBehavior` returned `ok=true`          |
| Control clicked               | Yes                     | toolbar `save` control activated through CDP mouse        |
| Browser callback/event        | Yes                     | `roamium.stderr` records `[issue-792-exp10] pdf-save-url` |
| File created                  | Yes                     | `downloads/bitcoin.pdf`, `184292` bytes                   |
| Classification                | `download-file-created` | `save-print-title-local-summary.json`                     |

Print result:

| Check                        | Result                                                            | Evidence                              |
| ---------------------------- | ----------------------------------------------------------------- | ------------------------------------- |
| Print control found          | Yes                                                               | toolbar inventory                     |
| `printingEnabled` value      | Not observable as true or false from the current live state probe | `print.flags=[]`                      |
| Content restrictions         | `restrictions=6`                                                  | `roamium.stderr`                      |
| Non-native containment proof | No                                                                | no intercepted print path             |
| Control clicked              | No                                                                | intentionally avoided                 |
| Callback/event               | No                                                                | no click was sent                     |
| Classification               | `print-not-contained`                                             | `save-print-title-local-summary.json` |

Title result:

| Fixture                            | Layer                          | Observed title/state             | Evidence               |
| ---------------------------------- | ------------------------------ | -------------------------------- | ---------------------- |
| Bitcoin PDF                        | Top-level DevTools target      | no matching propagated PDF title | summary title values   |
| Bitcoin PDF                        | PDF extension child target     | title exists inside extension    | `title-extension-only` |
| Bitcoin PDF                        | PDF extension `document.title` | extension-only title state       | summary title values   |
| Bitcoin PDF                        | TermSurf title/protocol        | not proven propagated            | summary classification |
| Bitcoin PDF                        | Classification                 | `title-extension-only`           | summary classification |
| Untitled/fallback PDF              | Top-level DevTools target      | no propagated fallback title     | local parity summaries |
| Untitled/fallback PDF              | PDF extension child target     | extension-only title state       | local parity summaries |
| Untitled/fallback PDF              | PDF extension `document.title` | extension-only title state       | local parity summaries |
| Untitled/fallback PDF              | TermSurf title/protocol        | not proven propagated            | local parity summaries |
| Untitled/fallback PDF              | Classification                 | `title-extension-only`           | local parity summaries |
| `setPdfDocumentTitle` availability | N/A                            | function exists in PDF extension | baseline API probe     |

Local-file parity result:

| URL kind                   | Render | Scroll                     | Page nav              | Zoom | Title                  | First failing layer                   |
| -------------------------- | ------ | -------------------------- | --------------------- | ---- | ---------------------- | ------------------------------------- |
| HTTP PDF fixture           | Yes    | Not observed by this probe | Yes                   | Yes  | `title-extension-only` | scroll classifier / title propagation |
| `file://` PDF fixture      | Yes    | Not observed by this probe | Yes                   | Yes  | `title-extension-only` | scroll classifier / title propagation |
| HTTP extensionless fixture | Yes    | Not observed by this probe | Yes                   | Yes  | `title-extension-only` | scroll classifier / title propagation |
| `file://` extensionless    | Yes    | Not observed by this probe | Yes                   | Yes  | `title-extension-only` | scroll classifier / title propagation |
| HTTP untitled fixture      | Yes    | Not observed by this probe | N/A, one-page fixture | Yes  | `title-extension-only` | title propagation                     |
| `file://` untitled fixture | Yes    | Not observed by this probe | N/A, one-page fixture | Yes  | `title-extension-only` | title propagation                     |

The local/extensionless rendering result is important: every tested URL entered
the PDF viewer and rendered. Page selector navigation and zoom worked for the
multi-page Bitcoin variants. The one-page untitled fixture cannot prove page
navigation because there is no later page.

The broad Experiment 13 local-parity probe did not observe wheel scroll changes,
even after switching the wheel dispatch to the top-level DevTools target and
comparing screenshots. This does not overturn the narrower prior evidence:
Experiment 4 and the Experiment 12 wheel regression both proved PDF wheel scroll
through the input router. It means Experiment 13's multi-navigation parity probe
is not a reliable scroll verifier yet; use the dedicated scroll harness for
scroll regressions.

Additional product finding:

- Roamium stderr repeatedly reports:
  `Uncaught (in promise) Error: Assertion failed: Could not find value for thumbnailPageAriaLabel`.
- Toolbar accessible labels still appear as `$i18n{...}` placeholders in the
  probe output.

That means Experiment 12's `resourcesPrivate.getStrings(PDF)` fix was narrow
enough to make zoom work, but it did not provide the complete PDF viewer string
surface expected by Chromium's PDF UI.

Build/checks:

- `node --check scripts/probe-pdf-save-print-title-local.mjs` passed.
- `python3 -m py_compile scripts/test-issue-794-pdf-toolbar.py` passed.

Codex completion review:

- Review log: `logs/codex-review/20260530-115418-817908-last-message.md`.
- Result: no blocking findings; Codex agreed the Partial result is correctly
  recorded and that completing PDF strings/load-time data is the right next
  target.
- Follow-up fixed before proceeding: the probe's aggregate `summary.status`
  could have false-passed a future run without title propagation or a contained
  print classification. The harness now requires `title-propagated`, a contained
  or explicitly disabled/restricted print classification, and a contained
  download file before it can report `pass`.

## Conclusion

Save/download is better than expected: with controlled DevTools download
behavior active, the real PDF toolbar save button creates a PDF file inside the
per-run downloads directory and reaches Chromium's PDF save callback. Local
`file://` PDFs and extensionless PDF URLs also render, and the existing page
navigation and zoom controls work for multi-page local/HTTP variants.

The remaining Issue 794 gaps are now narrower:

1. PDF UI strings/load-time data are incomplete. The viewer still throws on
   missing `thumbnailPageAriaLabel`, and controls expose `$i18n{...}`
   placeholders. The next experiment should replace the narrow Experiment 12
   string list with a complete-enough Chromium PDF viewer string/data provider.
2. Title propagation remains incomplete. The title is visible in the PDF
   extension layer but is not proven to reach the top-level TermSurf-visible tab
   title.
3. Toolbar print remains untested because the harness cannot prove a contained
   non-native print path. Do not click print until that containment exists.

Experiment 14 should target the incomplete PDF strings/load-time data first.
That is the most concrete product failure surfaced by this run and may also make
the print/title probes more trustworthy.
