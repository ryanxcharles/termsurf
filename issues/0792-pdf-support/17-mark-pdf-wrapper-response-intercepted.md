# Experiment 17: Mark the PDF Wrapper Response as Intercepted

## Description

Experiment 16 proved that TermSurf reaches the Chrome-style OOPIF PDF response
interceptor:

```text
PDF response arrives
  -> TermSurf creates the PDF viewer wrapper body
  -> TermSurf swaps the response body via delegate_->InterceptResponse(...)
  -> TermSurf registers an unclaimed PdfViewerStreamManager stream
  -> TermSurf resumes the deferred load
  -> content_shell still creates a download
```

The next gate is not `PdfViewerStreamManager` attachment. The wrapper navigation
does not reach `DidStartNavigation()` or `ReadyToCommitNavigation()` before
`ShellDownloadManagerDelegate::ChooseDownloadPath(...)` fires.

The relevant Chromium decision is in
`content/browser/loader/navigation_url_loader_impl.cc`: after response throttles
run, Chromium computes:

```text
known_mime_type = blink::IsSupportedMimeType(head->mime_type)
is_download = !head->intercepted_by_plugin && (must_download || !known_mime_type)
```

Chrome's own flow usually avoids this final download branch earlier:
`CheckPluginAndCallOnReceiveResponse(...)` asks `PluginService::HasPlugin(...)`
whether a plugin can handle `application/pdf`. Chrome succeeds there because its
PDF integration makes the PDF plugin visible for the external `application/pdf`
response as well as the internal `application/x-google-chrome-pdf` plugin MIME
type.

TermSurf currently does not get that canonical short-circuit. The PDF plugin
registration added in Experiment 15 covers the internal plugin MIME type, while
the intercepted navigation response still has the external `application/pdf`
MIME type. The navigation loader therefore sees an unsupported MIME type and
downloads it before the PDF viewer lifecycle can claim the stream.

There are two viable next fixes:

1. Extend TermSurf's internal PDF plugin registration so
   `PluginService::HasPlugin(..., "application/pdf")` succeeds, matching
   Chrome's earlier short-circuit more closely.
2. Mark the original wrapper response head as plugin-intercepted after TermSurf
   has actually swapped in the PDF viewer wrapper and registered the stream.

This experiment takes option 2. It is narrower: it suppresses the download only
for responses TermSurf's PDF stream interceptor has already taken ownership of,
rather than making every `application/pdf` response appear plugin-handled at the
plugin-service layer.

This experiment will make the smallest targeted change:

- log the download-classification inputs immediately before resume;
- set `response_head->intercepted_by_plugin = true` only after the TermSurf PDF
  interceptor has successfully:
  - created the wrapper body;
  - called `delegate_->InterceptResponse(...)`;
  - created/obtained `PdfViewerStreamManager`;
  - added the unclaimed stream container;
- rerun the PDF smoke test and confirm the download path no longer fires.

This is not a general "open all PDFs inline" policy. Attachments must still
download because the interceptor exits before the stream is created. Non-PDF
downloads and ordinary unsupported MIME types must still download.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 16 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp16
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp17
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Add local classification diagnostics in TermSurf's PDF response interceptor.

   In
   `content/libtermsurf_chromium/ts_plugin_response_interceptor_url_loader_throttle.cc`,
   log the wrapper response state immediately before `delegate_->Resume()`:

   ```text
   [issue-792-exp17] wrapper-classification-before frame_tree_node_id=<id> mime_type=<mime> intercepted_by_plugin=<0|1> request_destination=<n>
   [issue-792-exp17] wrapper-classification-after frame_tree_node_id=<id> mime_type=<mime> intercepted_by_plugin=<0|1> request_destination=<n>
   ```

   The `before` log should record the state before this experiment changes the
   field. The `after` log should record the state after setting it.

3. Mark the wrapper response as plugin-intercepted only after successful stream
   registration.

   In the same file, set:

   ```cpp
   response_head->intercepted_by_plugin = true;
   ```

   The assignment must occur after `AddPdfStreamContainer(...)` succeeds and
   before posting `ResumeLoad()`.

   Do not set this flag for:
   - non-PDF responses;
   - responses where `download_utils::MustDownload(...)` is true;
   - responses without the PDF component-extension mapping;
   - responses where `AddPdfStreamContainer(...)` fails.

   This keeps the tag tied to actual TermSurf stream ownership rather than to a
   broad MIME-type preference.

4. Add one narrow content-side confirmation log.

   In `content/browser/loader/navigation_url_loader_impl.cc`, log the final
   classification for PDF-like responses only:

   ```text
   [issue-792-exp17] navigation-download-classification url=<url> mime_type=<mime> intercepted_by_plugin=<0|1> must_download=<0|1> known_mime_type=<0|1> is_download=<0|1>
   ```

   Keep this diagnostic scoped to `application/pdf` or responses with
   `intercepted_by_plugin = true`, so normal browsing does not become noisy.

5. Preserve Experiment 16 claim-lifecycle logs.

   Do not remove the Experiment 16 logs yet. If this experiment works, the
   important proof is that the earlier `pvs-start`, `pvs-ready`, and `pvs-claim`
   logs begin firing after the wrapper response is no longer classified as a
   download.

6. Build and archive only after the result is accepted.

   Build with:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   If the experiment passes or produces a coherent partial branch, commit the
   Chromium branch and regenerate:

   ```bash
   rm -rf ../../chromium/patches/issue-792/
   git format-patch 148.0.7778.97..HEAD -o ../../chromium/patches/issue-792/
   ```

## Verification

1. Build `libtermsurf_chromium` with `autoninja`.

2. Run the fake-GUI PDF smoke test against the local bitcoin PDF fixture:

   ```bash
   LOG_DIR="logs/issue-792-exp17-pdf-$(date +%Y%m%d-%H%M%S)"
   scripts/test-issue-792-fake-gui.py \
     http://127.0.0.1:9787/bitcoin.pdf \
     --serve-bitcoin-pdf \
     --log-dir "$LOG_DIR" \
     --seconds 18
   ```

3. Inspect `roamium.stderr` in the run directory.

   Required before/after evidence:

   ```text
   [issue-792-exp17] wrapper-classification-before ... intercepted_by_plugin=0
   [issue-792-exp17] wrapper-classification-after ... intercepted_by_plugin=1
   [issue-792-exp17] navigation-download-classification ... intercepted_by_plugin=1 ... is_download=0
   ```

   There must be no `ShellDownloadManagerDelegate::ChooseDownloadPath(...)` line
   for the PDF navigation after the wrapper response is marked intercepted.

4. Confirm the stream lifecycle progresses past the Experiment 16 blocker.

   At minimum, the PDF log must contain one of:

   ```text
   [issue-792-exp16] pvs-start ...
   [issue-792-exp16] pvs-ready ...
   [issue-792-exp16] pvs-claim ...
   ```

   If the download path is gone but no stream lifecycle log appears, record a
   Partial result and identify the next lifecycle gate.

5. Classify the next rendering/lifecycle state from logs.

   The fake-GUI harness intentionally avoids Wezboard and does not produce a
   screenshot. Classify the result from the Chromium log and TermSurf protobuf
   message log as:
   - PDF stream claimed and plugin/viewer lifecycle continues;
   - PDF viewer shell without document content;
   - renderer crash/sad tab;
   - blank wrapper;
   - automation failure.

   If the download path is eliminated and the viewer advances to a new blocker,
   record Partial with the exact new blocker. A real-GUI screenshot pass can be
   added after the fake-GUI logs prove that the browser-side pipeline reaches a
   renderable state.

6. Run a normal HTML smoke test:

   ```bash
   LOG_DIR="logs/issue-792-exp17-html-$(date +%Y%m%d-%H%M%S)"
   scripts/test-issue-792-fake-gui.py \
     http://localhost:9616/index.html \
     --log-dir "$LOG_DIR" \
     --seconds 8
   ```

   The HTML run must not emit Experiment 17 PDF classification logs, must not
   trigger a download, and must render normally.

7. Run a non-PDF download smoke test.

   Extend the fake-GUI harness or use a one-off local HTTP server to serve a
   small fixture with an unsupported non-PDF MIME type. Confirm the response
   still downloads or otherwise follows the normal unsupported-MIME path. The
   new `intercepted_by_plugin` assignment must not appear for this request.

8. Run an attachment PDF smoke test if a fixture is available.

   Extend the fake-GUI harness or use a one-off local HTTP server to serve the
   same PDF with `Content-Disposition: attachment`. Confirm
   `download_utils::MustDownload(...)` prevents the TermSurf interceptor from
   setting `intercepted_by_plugin`, and the file still downloads.

## Pass Criteria

- `libtermsurf_chromium` builds.
- The PDF smoke test shows `intercepted_by_plugin` changing from `0` to `1` on
  the wrapper response.
- `navigation-download-classification` reports `is_download=0` for the
  intercepted PDF wrapper response.
- The PDF navigation does not reach
  `ShellDownloadManagerDelegate::ChooseDownloadPath(...)`.
- Either the PDF stream reaches a renderable viewer state, or the experiment
  advances to a later viewer/stream blocker with concrete logs.
- HTML navigation remains normal.
- Non-PDF downloads and attachment PDFs are not converted into inline viewer
  navigations.

## Partial Criteria

- The download path is eliminated but the PDF viewer hits a later blocker.
  Record the first missing or failing layer after `is_download=0`.
- `intercepted_by_plugin` reaches `1`, but `NavigationURLLoaderImpl` still
  reports `is_download=1`. Record the exact classification values.
- The PDF renders only for HTTP but not `file://`. Record this as Partial; the
  original local-file goal remains unsolved.

## Failure Criteria

- The experiment suppresses downloads by broad MIME-type policy rather than by
  successful TermSurf PDF stream interception.
- Attachment PDFs stop downloading.
- Non-PDF unsupported MIME types stop downloading.
- HTML navigation, DevTools, popups, or ordinary browser input regress.
- The implementation removes Experiment 16 lifecycle evidence before proving the
  next gate.
