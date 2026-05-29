# Experiment 19: Trace Wrapper-to-Extension Startup

## Description

Experiment 17 fixed the content_shell download gate. Experiment 18 then proved
that the next blocker is not the null `MimeHandlerService` stub:

```text
navigation-download-classification ... is_download=0
pvs-claim ... claimed=1
mime-handler-container-manager-bound ...
```

But the PDF viewer never calls either stream-info API:

```text
mime-handler-service-request             absent
real-mime-handler-get-stream-info        absent
pdf-viewer-private-get-stream-info       absent
```

That means the pipeline is stopping earlier than stream-info. The current
failure is between the committed wrapper HTML and the PDF extension viewer
startup.

Chromium's OOPIF PDF wrapper is generated from
`components/pdf/resources/pdf_embedder.html`. It contains a declarative shadow
root with an `iframe name="$internal_id" src="about:blank" ...>` placeholder.
`PdfViewerStreamManager::DidFinishNavigation()` expects that child `about:blank`
navigation to commit under the claimed embedder host. When it sees that child,
it records the child frame tree node and navigates it to the PDF extension URL:

```text
wrapper PDF response commits
  -> stream is claimed by the wrapper's main RenderFrameHost
  -> wrapper creates child about:blank frame
  -> PdfViewerStreamManager sees the child about:blank navigation
  -> NavigateToPdfExtensionUrl(...)
  -> chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html loads
  -> viewer asks for stream info
```

The Experiment 18 logs show the first two steps only. There is no
`pdf-extension-about-blank`, no `pdf-extension-navigate`, and no stream-info
request. Experiment 19 adds narrow diagnostics at this transition so the next
experiment can fix the correct layer.

The likely outcomes for OOPIF PDF startup failure are:

- the wrapper HTML never creates the child frame;
- the child iframe commits, but `DidFinishNavigation()` does not see it under
  the claimed embedder because the parent frame or stream lookup key does not
  match;
- the child iframe commits, but at a URL other than `about:blank`;
- the `NavigateToPdfExtensionUrl(...)` call path is blocked after the
  `about:blank` child commits;
- the PDF extension URL navigation starts, but the extension viewer does not
  progress to stream-info requests.

The renderer-side plugin path and legacy guest-view attach path are not expected
to drive OOPIF PDF startup. They are included as absence-evidence diagnostics
only. If `IsPluginHandledExternally(...)` or `MimeHandlerViewEmbedder` fires for
this top-level PDF case, that means TermSurf unexpectedly fell out of the OOPIF
PDF path and into the legacy embed/plugin path.

This experiment is diagnostic-only. It must not add another speculative PDF
startup mechanism. The result should identify exactly which handoff is missing
and what the next fix experiment should change.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 18 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp18
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp19
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Add a wrapper payload fingerprint log.

   In
   `content/libtermsurf_chromium/ts_plugin_response_interceptor_url_loader_throttle.cc`,
   after `CreateTemplateMimeHandlerPage(...)`, log only structural facts about
   the generated wrapper:

   ```text
   [issue-792-exp19] wrapper-payload frame_tree_node_id=<id> internal_id=<id> bytes=<n> has_template=<0|1> has_iframe=<0|1> has_embed=<0|1> has_about_blank=<0|1> has_internal_id=<0|1> has_pdf_extension_url=<0|1>
   ```

   Do not dump the full HTML body. The goal is to prove which wrapper template
   was served and whether it contains the expected child-frame placeholder.
   Check for the actual generated `internal_id` string, not only the literal
   placeholder text, so the log distinguishes a substituted wrapper from an
   unsubstituted template.

   Local source audit shows `IDR_PDF_EMBEDDER_HTML` is declared in
   `components/resources/pdf_resources.grdp`, aggregated into
   `components_resources.pak`. TermSurf's current resource loader loads
   `pdf_resources.pak`, `common_resources.pak`, and
   `extensions_renderer_resources.pak`, but not `components_resources.pak`. This
   experiment must verify the wrapper payload before chasing deeper frame
   lifecycle issues. If the payload is empty or does not contain the expected
   iframe, the result should stop there and the next experiment should load the
   missing components resource pak. Do not add that fix in Experiment 19.

3. Instrument `PdfViewerStreamManager::DidFinishNavigation()`.

   In `chrome/browser/pdf/pdf_viewer_stream_manager.cc`, add issue-tagged logs
   that explain every branch around the extension startup path.

   Required logs:

   ```text
   [issue-792-exp19] pvs-finish frame_tree_node_id=<id> url=<url> has_committed=<0|1> is_error_page=<0|1> is_pdf=<0|1> has_parent=<0|1> parent_frame_tree_node_id=<id-or-none> stream_count=<n>
   [issue-792-exp19] pvs-finish-no-parent frame_tree_node_id=<id> url=<url>
   [issue-792-exp19] pvs-finish-no-claimed-stream frame_tree_node_id=<id> parent_frame_tree_node_id=<id> url=<url>
   [issue-792-exp19] pvs-finish-extension-already-started frame_tree_node_id=<id> url=<url> matches_extension_url=<0|1> matches_tracked_frame=<0|1> has_committed=<0|1> is_error_page=<0|1>
   [issue-792-exp19] pvs-finish-not-about-blank frame_tree_node_id=<id> parent_frame_tree_node_id=<id> url=<url>
   [issue-792-exp19] pvs-finish-about-blank frame_tree_node_id=<id> parent_frame_tree_node_id=<id> url=<url>
   ```

   Preserve the existing Experiment 15 `pdf-extension-about-blank` and
   `pdf-extension-navigate` logs. These new logs explain why those existing
   success logs do or do not appear.

   Use the literal string `none` for `parent_frame_tree_node_id` when a
   navigation has no parent.

4. Instrument the renderer-side plugin decision.

   In `content/libtermsurf_chromium/ts_content_renderer_client.cc`, extend the
   Experiment 15 logs around:
   - `IsPluginHandledExternally(...)`;
   - `OverrideCreatePlugin(...)`.

   Required logs:

   ```text
   [issue-792-exp19] renderer-plugin-external document_url=<url> original_url=<url> mime_type=<mime> has_internal_id=<0|1> handled=<0|1>
   [issue-792-exp19] renderer-override-create-plugin document_url=<url> mime_type=<mime> url=<url> delegated_to_extensions=<0|1>
   ```

   These logs answer whether Blink asks TermSurf's renderer client to treat the
   wrapper placeholder as a MIME-handler view or tries to instantiate a normal
   plugin path instead.

   This is diagnostic-only. Do not modify return values, predicate logic, or
   delegation behavior in `IsPluginHandledExternally(...)` or
   `OverrideCreatePlugin(...)`. Only add logs next to the existing Experiment 15
   behavior.

5. Instrument the guest-view attach observer without changing behavior.

   In
   `extensions/browser/guest_view/mime_handler_view/mime_handler_view_embedder.cc`,
   add issue-tagged diagnostics next to the existing Experiment 15 logs:

   ```text
   [issue-792-exp19] mhv-render-frame-created frame_tree_node_id=<id> parent_matches=<0|1> url_matches=<0|1> owner_type=<n> owner_matches=<0|1> name_matches=<0|1> ready=<0|1>
   [issue-792-exp19] mhv-destroy reason=<new-navigation|sandboxed|frame-deleted|not-ready|attach-complete> frame_tree_node_id=<id>
   [issue-792-exp19] mhv-create-guest-start frame_tree_node_id=<id> stream_id=<id>
   ```

   Do not modify guest creation or attachment. This is only to determine whether
   the guest-view path is observing the wrapper child frame and whether it
   destroys itself before viewer startup.

6. Preserve prior logs.

   Keep the Experiment 16/17/18 logs. The result needs the complete sequence
   from interception through claim through whichever startup gate fails.

7. Build and archive only after the result is accepted.

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
   LOG_DIR="logs/issue-792-exp19-pdf-$(date +%Y%m%d-%H%M%S)"
   scripts/test-issue-792-fake-gui.py \
     http://127.0.0.1:9787/bitcoin.pdf \
     --serve-bitcoin-pdf \
     --log-dir "$LOG_DIR" \
     --seconds 18
   ```

3. Inspect `roamium.stderr` in the run directory.

   The first required chain is:

   ```text
   [issue-792-exp17] navigation-download-classification ... is_download=0
   [issue-792-exp16] pvs-claim ... claimed=1
   [issue-792-exp19] wrapper-payload ... has_iframe=1 ... has_about_blank=1
   ```

   Then classify the result by the first missing transition:
   - If there is no `pvs-finish` for any child of the claimed frame, the wrapper
     is not creating the expected child frame or the child frame is not
     navigating.
   - If `wrapper-payload` shows `bytes=0` or `has_iframe=0`, stop at that
     finding. The likely next experiment is loading `components_resources.pak`
     so `IDR_PDF_EMBEDDER_HTML` can be served.
   - If a child `pvs-finish` appears with `has_parent=1` but
     `pvs-finish-no-claimed-stream`, the manager is looking up the wrong parent
     host or the stream was claimed under the wrong frame key.
   - If `pvs-finish-not-about-blank` appears for the child, the wrapper
     placeholder URL does not match the canonical OOPIF PDF expectation.
   - If `pvs-finish-about-blank` appears but `pdf-extension-navigate` does not,
     the `NavigateToPdfExtensionUrl(...)` call path is blocked inside
     `PdfViewerStreamManager`.
   - If `pdf-extension-navigate` appears but no stream-info API is requested,
     the next blocker is extension viewer startup after navigation.
   - If `renderer-plugin-external` never appears, Blink is not asking the
     renderer client to externalize the wrapper placeholder. For the OOPIF PDF
     path this is expected absence-evidence, not a failure by itself.
   - If `renderer-plugin-external ... handled=0` appears for the wrapper
     placeholder, the renderer-side MIME-handler predicate is wrong.
   - If `mhv-render-frame-created` appears and then `mhv-destroy ... not-ready`
     appears, the guest-view attach path is observing the child but racing
     readiness before viewer startup.

4. Run the normal HTML smoke test to verify the diagnostics do not interfere
   with non-PDF navigation:

   ```bash
   LOG_DIR="logs/issue-792-exp19-html-$(date +%Y%m%d-%H%M%S)"
   scripts/test-issue-792-fake-gui.py \
     http://localhost:9616/index.html \
     --log-dir "$LOG_DIR" \
     --seconds 8
   ```

   The run should not emit `wrapper-payload`, `pvs-claim`, or PDF-specific
   Experiment 19 transition logs.

5. Record the result in this file.

   The result must include:
   - the exact PDF and HTML log directories;
   - the first missing transition in the PDF startup chain;
   - whether the wrapper payload contains the expected iframe placeholder;
   - whether the wrapper payload proves `IDR_PDF_EMBEDDER_HTML` is available
     from the loaded resource paks;
   - whether the child `about:blank` frame committed;
   - whether `IsPluginHandledExternally(...)` was called and what it returned;
   - whether the guest-view attach observer saw and destroyed the placeholder;
   - the concrete next experiment implied by the evidence.

## Result

**Result:** Pass

Chromium branch: `148.0.7778.97-issue-792-exp19`

Build:

```bash
cd chromium/src
export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium
```

The build passed in 5 steps.

PDF smoke log:

```text
logs/issue-792-exp19-pdf-20260529-145032
```

The PDF run preserved the Experiment 17/18 success chain through download
suppression and stream claim:

```text
[issue-792-exp17] navigation-download-classification ... is_download=0
[issue-792-exp16] pvs-claim frame_tree_node_id=1 claimed=1 original_url=http://127.0.0.1:9787/bitcoin.pdf
```

Experiment 19 then found the first missing transition:

```text
[issue-792-exp19] wrapper-payload frame_tree_node_id=1 internal_id=C6E28BE60AAB7E159B240EF68348DF05 bytes=0 has_template=0 has_iframe=0 has_embed=0 has_about_blank=0 has_internal_id=0 has_pdf_extension_url=0
```

The wrapper payload is empty. That means Chromium never serves
`components/pdf/resources/pdf_embedder.html` into the intercepted PDF response,
so the wrapper cannot create the expected child `about:blank` iframe. With no
iframe, `PdfViewerStreamManager::DidFinishNavigation()` has no child navigation
to observe and cannot call `NavigateToPdfExtensionUrl(...)`.

The later lifecycle logs are consistent with that diagnosis:

```text
[issue-792-exp19] pvs-finish frame_tree_node_id=1 url=http://127.0.0.1:9787/bitcoin.pdf has_committed=1 is_error_page=0 is_pdf=0 has_parent=0 parent_frame_tree_node_id=none stream_count=1
[issue-792-exp19] pvs-finish-no-parent frame_tree_node_id=1 url=http://127.0.0.1:9787/bitcoin.pdf
```

Only the top-level wrapper navigation finishes. No child `about:blank` frame
appears, no `pdf-extension-about-blank` log appears, no `pdf-extension-navigate`
log appears, and no stream-info API is requested.

HTML smoke log:

```text
logs/issue-792-exp19-html-20260529-145106
```

The HTML smoke emitted no `wrapper-payload`, `pvs-claim`,
`plugin-response-intercept`, `navigation-download-classification`, or other
Experiment 19 PDF transition logs.

## Conclusion

Experiment 19 proved the current blocker precisely: the OOPIF PDF wrapper body
is empty because `IDR_PDF_EMBEDDER_HTML` is not available from the resource paks
TermSurf loads.

The source audit in the design explains why:

- `IDR_PDF_EMBEDDER_HTML` comes from `components/resources/pdf_resources.grdp`;
- that resource is aggregated into `components_resources.pak`;
- TermSurf currently loads `pdf_resources.pak`, `common_resources.pak`, and
  `extensions_renderer_resources.pak`;
- TermSurf does not load `components_resources.pak`.

The next experiment should load `components_resources.pak` in TermSurf's PDF
resource-bundle setup, verify
`wrapper-payload bytes>0 has_template=1 has_iframe=1 has_about_blank=1 has_internal_id=1 has_pdf_extension_url=1`,
and then re-check whether the child `about:blank` frame advances to
`pdf-extension-navigate`.
