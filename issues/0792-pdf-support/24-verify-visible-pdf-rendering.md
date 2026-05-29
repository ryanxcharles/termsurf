# Experiment 24: Verify Visible PDF Rendering

## Description

Experiment 23 completed the PDF wrapper and stream-info plumbing:

```text
[issue-792-exp23] static-response-check ... plugin_supports_mime=1 action=return
[issue-792-exp22] body-data-received ... encoded_size=536 ... has_template=1 has_iframe=1 has_shadowrootmode=1 ...
[issue-792-exp21] declarative-shadow-root ... success=1
[issue-792-exp21] frame-owner-inserted ... tag=IFRAME ... src=about:blank ...
[issue-792-exp15] pdf-extension-navigate handler_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html
[issue-792-exp18] real-mime-handler-get-stream-info has_stream=1 ... original_url=http://127.0.0.1:9787/bitcoin.pdf
```

The remaining question is visual, not plumbing: does the PDF viewer actually
render recognizable PDF content in a real TermSurf pane?

Experiment 24 runs the existing real-GUI screenshot harness against the vendored
Bitcoin PDF. It does not change Chromium, Wezboard, Roamium, webtui, protocol,
or PDF plumbing code unless the screenshot reveals a new failure that requires a
follow-up experiment.

This experiment must receive Claude design review before it runs. After the
result is recorded, Claude must review the completed output before any cleanup,
closure, or next experiment.

## Changes

No code changes are planned.

Use the existing screenshot harness:

```bash
TERMSURF_PDF_SETTLE_SECONDS=18 \
LOG_DIR="logs/issue-792-exp24-pdf-$(date +%Y%m%d-%H%M%S)" \
scripts/test-issue-776-pdf.sh http://localhost:9616/bitcoin.pdf
```

The script already:

- verifies `screencapture` permission with a permission-test PNG;
- launches debug `wezboard-gui`;
- launches debug `web`;
- passes the repo-built Roamium binary with `--browser`;
- serves the vendored PDF through `test-html` when needed;
- captures a screenshot artifact under the log directory;
- copies the Chromium log when available.

## Verification

1. Run the HTML screenshot sanity check first:

   ```bash
   TERMSURF_PDF_SETTLE_SECONDS=8 \
   LOG_DIR="logs/issue-792-exp24-html-sanity-$(date +%Y%m%d-%H%M%S)" \
   scripts/test-issue-776-pdf.sh https://example.com
   ```

   Inspect the screenshot artifact with `view_image`. It must show the intended
   debug Wezboard/web/Roamium run with visible HTML content. If it shows a black
   capture, the wrong app, an installed/stable Roamium, or missing Roamium logs,
   do not classify any PDF result from the same harness. Fix or replace the
   harness first.

2. Run the fake-GUI PDF smoke test as authoritative plumbing evidence:

   ```bash
   LOG_DIR="logs/issue-792-exp24-fakegui-$(date +%Y%m%d-%H%M%S)" \
   scripts/test-issue-792-fake-gui.py \
     http://127.0.0.1:9787/bitcoin.pdf \
     --serve-bitcoin-pdf \
     --log-dir "$LOG_DIR" \
     --seconds 18
   ```

   The fake-GUI logs should show the Experiment 23 success chain through:

   ```text
   real-mime-handler-get-stream-info has_stream=1
   ```

3. Run the real-GUI PDF screenshot harness:

   ```bash
   TERMSURF_PDF_SETTLE_SECONDS=18 \
   LOG_DIR="logs/issue-792-exp24-pdf-$(date +%Y%m%d-%H%M%S)" \
   scripts/test-issue-776-pdf.sh http://localhost:9616/bitcoin.pdf
   ```

4. Inspect the PDF screenshot artifact with `view_image`.

   Classify it as one of:
   - **Rendered PDF:** recognizable Bitcoin PDF content is visible in the pane,
     such as the paper title or PDF page body.
   - **Viewer shell only:** the PDF extension viewer frame is visible, but the
     PDF page itself is blank, loading forever, or showing a viewer error.
   - **Wrong app/run:** the screenshot does not show the intended debug
     Wezboard/web/Roamium run.
   - **Blank/black capture:** screenshot capture succeeded but does not show the
     app content.
   - **Automation failure:** no screenshot artifact was produced.

5. Cross-check the real-GUI logs.

   The primary pass signal is the screenshot, but a Pass also requires real-GUI
   logs showing the Experiment 23 success chain. If the real-GUI logs are
   missing or incomplete for an automation reason, the fake-GUI smoke test from
   step 2 may be used as backup plumbing evidence, but the result must say that
   explicitly.

   ```text
   real-mime-handler-get-stream-info has_stream=1
   ```

   If the screenshot is not a rendered PDF, use logs to classify the next layer:
   - if stream-info is missing, the regression is back in plumbing;
   - if stream-info is present but no PDF content appears, the next experiment
     should instrument PDF extension viewer JavaScript / PDFium plugin startup;
   - if the app or screenshot is wrong, the experiment result is an automation
     failure, not a PDF conclusion.

6. Record the result in this file.

   The result must include:
   - HTML sanity log directory and screenshot artifact path;
   - whether the HTML sanity run proves the harness captures the intended debug
     app;
   - fake-GUI smoke log directory and whether stream-info appears there;
   - exact log directory path;
   - screenshot artifact path;
   - visual classification;
   - whether the screenshot shows the intended debug run;
   - whether the Experiment 23 stream-info chain still appears;
   - pass/fail/partial status;
   - the next action.

## Pass Criteria

Experiment 24 passes only if:

- the HTML sanity screenshot proves the harness captures the intended debug app;
- the PDF screenshot shows recognizable Bitcoin PDF content in the TermSurf
  pane;
- real-GUI logs, or fake-GUI backup logs if the real-GUI log copy is incomplete
  for an automation reason, show
  `real-mime-handler-get-stream-info has_stream=1`;
- no logs contradict the run.

## Partial Criteria

Experiment 24 is partial if the plumbing logs remain healthy but the screenshot
shows only the viewer shell, blank content, a viewer error, or another
post-stream-info rendering failure. In that case, the next experiment should
instrument the PDF extension viewer and PDFium startup/rendering path.

## Failure Criteria

Experiment 24 fails if:

- the screenshot harness cannot produce a usable artifact;
- the screenshot captures the wrong app/run;
- the Experiment 23 stream-info chain regresses;
- the run uses an installed/stable Roamium instead of the repo-built debug
  Roamium.

## Result

**Result:** Fail

Experiment 24 did not produce valid visual evidence. This is an automation
failure, not a PDF-rendering conclusion.

Runs:

- HTML sanity, first attempt: `logs/issue-792-exp24-html-sanity-20260529-160850`
- HTML sanity, after harness log-path repair:
  `logs/issue-792-exp24-html-sanity-20260529-161036`
- Fake-GUI plumbing cross-check: `logs/issue-792-exp24-fakegui-20260529-161150`

The first HTML sanity run produced a black screenshot and no `run-web.log`.
`wezboard-gui.log` showed only the TermSurf listener and `pane_count=0`, so the
pane launcher had not produced usable evidence.

The harness was repaired to canonicalize `LOG_DIR` to an absolute path before it
generates `run-web.sh`. After that repair, the second HTML sanity run proved the
launcher and debug binaries were correct:

```text
using TERMSURF_SOCKET=/var/folders/.../termsurf/wezboard-79585.sock
exec /Users/ryan/dev/termsurf/webtui/target/debug/web --browser /Users/ryan/dev/termsurf/chromium/src/out/Default/roamium https://example.com
SetOverlay: pane_id=0 profile=default browser=/Users/ryan/dev/termsurf/chromium/src/out/Default/roamium url=https://example.com
CALayerHost created at (14.0,40.0,1106.0,1056.0): pane_id=0 contextId=2935611265 ...
```

That repaired run still produced an all-black screenshot:

- `logs/issue-792-exp24-html-sanity-20260529-161036/pdf-smoke.png`

The initial permission test image from the same run was also all black:

- `logs/issue-792-exp24-html-sanity-20260529-161036/screenshot-permission-test.png`

Visual classification: **Blank/black capture**. Experiment classification:
**Automation failure**. Because the HTML sanity screenshot failed, the PDF
screenshot was not run or classified.

The fake-GUI plumbing cross-check also did not reproduce the Experiment 23
stream-info chain in this run. It navigated to
`http://127.0.0.1:9787/bitcoin.pdf`, but the log reached a `chrome-error`
document and did not contain:

```text
real-mime-handler-get-stream-info has_stream=1
```

No HTTP fixture request was logged, so this fake-GUI run is not valid evidence
about the Experiment 23 plumbing either.

## Conclusion

Experiment 24 failed because the visual automation was not trustworthy:

1. the real-GUI screenshot path captures black images even for its permission
   test;
2. the first harness version hid the pane-launcher failure because relative log
   paths prevented `run-web.log` from being written;
3. after the log-path repair, the app and Roamium did launch correctly, but
   `screencapture` still captured black;
4. the fake-GUI backup run also failed to reproduce the known-good stream-info
   chain.

The next experiment should replace the visual evidence path. The likely shape is
to capture renderer output through the running debug Roamium DevTools endpoint,
while separately using the existing TermSurf logs to prove the debug real-GUI
path created the overlay and used the repo-built Roamium binary. Do not treat
the black screenshots as PDF-rendering evidence.
