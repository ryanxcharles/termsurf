# Experiment 16: Trace PDF Stream Claim Lifecycle

## Description

Experiment 15 eliminated the wrong branch of the MIME-handler helper. For OOPIF
PDF, Chrome does **not** use
`MimeHandlerViewAttachHelper::OverrideBodyForInterceptedResponse(...)`. Chrome's
own `PluginResponseInterceptorURLLoaderThrottle` uses
`CreateTemplateMimeHandlerPage(...)` for OOPIF PDFs, then relies on
`pdf::PdfViewerStreamManager` to claim the stream during the wrapper navigation
lifecycle.

Experiment 14 already used `CreateTemplateMimeHandlerPage(...)` and proved
TermSurf can add a `StreamContainer`, but the stream was never claimed.
Experiment 15 proved that creating `MimeHandlerViewEmbedder` after the
intercepted response starts too late and does not cause a claim.

Experiment 16 returns to Chrome's OOPIF PDF branch and instruments the actual
claim lifecycle:

```text
PDF navigation starts
  -> response interceptor swaps in the PDF wrapper HTML
  -> response interceptor adds unclaimed StreamContainer
  -> PdfViewerStreamManager observes wrapper navigation ReadyToCommit
  -> manager finds unclaimed stream by frame_tree_node_id
  -> manager claims stream and sets internal id on renderer container manager
```

The goal is to answer one narrow question:

> Why does `PdfViewerStreamManager::ReadyToCommitNavigation()` not claim the
> unclaimed stream that Experiment 14 added?

Likely outcomes:

- download classification: content_shell enters the download path before the
  wrapper commit can happen;
- lifecycle ordering: `ReadyToCommitNavigation()` runs before
  `AddStreamContainer()`;
- observer lifetime: `PdfViewerStreamManager` is created after the relevant
  navigation event has already passed;
- key mismatch: the frame tree node id used by `AddStreamContainer()` differs
  from the id seen by `ReadyToCommitNavigation()`;
- deletion race: `RenderFrameDeleted()` removes the unclaimed stream before the
  claim;
- another lifecycle mismatch not yet visible from the Experiment 14/15 logs.

This is primarily a diagnostic experiment, but it may include the smallest
ordering fix if the logs prove it mechanically. For example, if the only problem
is that `PdfViewerStreamManager` is created too late, the experiment may create
the manager earlier in TermSurf's PDF navigation setup and re-run the same trace
to prove the claim fires. Do not add a speculative fix before the trace proves
which gate failed.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 15 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp15
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp16
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Revert the wrong OOPIF PDF body-path change.

   In
   `content/libtermsurf_chromium/ts_plugin_response_interceptor_url_loader_throttle.cc`,
   return the PDF response path to `CreateTemplateMimeHandlerPage(...)`,
   matching Chrome's OOPIF PDF branch.

   Remove the behavior change that calls
   `OverrideBodyForInterceptedResponse(...)` for OOPIF PDF. Keep useful
   Experiment 15 setup that is independent of the wrong branch:
   - internal PDF plugin registration in `TsContentClient::AddPlugins(...)`;
   - renderer `MimeHandlerViewContainerManager` associated-interface binding;
   - `IsPluginHandledExternally(...)` diagnostics, if they remain harmless;
   - issue-tagged logs in `PdfViewerStreamManager` if they are still relevant.

   The PDF path should again resume exactly as Chrome does for OOPIF PDF: add
   the stream container, then resume the deferred load. Keep the single-resume
   invariant.

   Required log:

   ```text
   [issue-792-exp16] oopif-template-body frame_tree_node_id=<id> internal_id=<id> original_url=<url>
   ```

3. Instrument `PdfViewerStreamManager::AddStreamContainer()`.

   In `chrome/browser/pdf/pdf_viewer_stream_manager.cc`, add issue-tagged logs
   recording:
   - frame tree node id;
   - internal id;
   - original URL;
   - handler URL;
   - stream count before/after;
   - whether an unclaimed stream for the same frame tree node already existed.

   Required log:

   ```text
   [issue-792-exp16] pvs-add frame_tree_node_id=<id> internal_id=<id> original_url=<url> handler_url=<url> count_before=<n> count_after=<n> replacing_unclaimed=<0|1>
   ```

4. Instrument `PdfViewerStreamManager::DidStartNavigation()` and
   `ReadyToCommitNavigation()`.

   In `DidStartNavigation()`, log every call, not only PDF content-frame cases:
   - frame tree node id;
   - navigation URL;
   - whether `navigation_handle->IsPdf()` is true;
   - whether this is the main frame;
   - whether an unclaimed stream exists for the frame tree node;
   - stream count.

   Required log:

   ```text
   [issue-792-exp16] pvs-start frame_tree_node_id=<id> url=<url> is_pdf=<0|1> is_in_main_frame=<0|1> has_unclaimed=<0|1> stream_count=<n>
   ```

   Log every call, not only successful claims:
   - frame tree node id;
   - navigation URL;
   - `HasCommitted()` if available at this point;
   - whether `navigation_handle->IsPdf()` is true;
   - whether `MaybeRegisterPdfSubresourceOverride()` returned true;
   - whether an unclaimed stream exists for the committing frame tree node;
   - stream count.

   Required logs:

   ```text
   [issue-792-exp16] pvs-ready frame_tree_node_id=<id> url=<url> is_pdf=<0|1> has_unclaimed=<0|1> stream_count=<n>
   [issue-792-exp16] pvs-ready-subresource-override frame_tree_node_id=<id> handled=<0|1>
   [issue-792-exp16] pvs-claim frame_tree_node_id=<id> claimed=<0|1> original_url=<url>
   [issue-792-exp16] pvs-no-claim frame_tree_node_id=<id> reason=<no-unclaimed|other>
   ```

   The `pvs-ready` log must fire before any conclusion is drawn. If it never
   fires, the next gate is manager lifetime or observer registration, not claim
   logic.

5. Instrument deletion and host-change paths that can remove streams.

   Add logs to:
   - `RenderFrameDeleted()`;
   - `FrameDeleted()`;
   - `RenderFrameHostChanged()`;
   - `DeleteUnclaimedStreamInfo()`;
   - `DeleteClaimedStreamInfo()`.

   Required logs:

   ```text
   [issue-792-exp16] pvs-render-frame-deleted frame_tree_node_id=<id> active=<0|1> has_unclaimed=<0|1> has_claimed=<0|1>
   [issue-792-exp16] pvs-frame-deleted frame_tree_node_id=<id> stream_count_before=<n> stream_count_after=<n>
   [issue-792-exp16] pvs-rfh-changed old_frame_tree_node_id=<id> new_frame_tree_node_id=<id> old_has_claimed=<0|1>
   [issue-792-exp16] pvs-delete-unclaimed frame_tree_node_id=<id>
   [issue-792-exp16] pvs-delete-claimed frame_tree_node_id=<id>
   ```

6. Instrument the response-interceptor ordering.

   In TermSurf's response interceptor, log the exact ordering around:
   - wrapper body creation;
   - `delegate_->InterceptResponse(...)`;
   - `PdfViewerStreamManager::Create(...)`;
   - `AddStreamContainer(...)`;
   - `delegate_->Resume()`.

   Required logs:

   ```text
   [issue-792-exp16] interceptor-template-created frame_tree_node_id=<id> internal_id=<id>
   [issue-792-exp16] interceptor-response-swapped frame_tree_node_id=<id>
   [issue-792-exp16] interceptor-manager-created frame_tree_node_id=<id>
   [issue-792-exp16] interceptor-stream-added frame_tree_node_id=<id> internal_id=<id>
   [issue-792-exp16] interceptor-resume frame_tree_node_id=<id>
   ```

7. Optional minimal fix only after the trace identifies the gate.

   First run the trace without any fix and classify the observed gate. Only then
   decide whether a minimal fix belongs in this experiment.

   If the first run shows pure lifecycle ordering — for example, `pvs-ready`
   fires before `interceptor-manager-created` or `pvs-add`, and there is no key
   mismatch, deletion race, or download-classification gate — try the smallest
   ordering fix:
   - create `PdfViewerStreamManager` earlier, before response interception, at
     the PDF navigation throttle/browser-client setup point;
   - keep `AddStreamContainer()` in the response interceptor where the
     `TransferrableURLLoader` exists;
   - rerun the same PDF smoke and compare the ordered logs.

   If the fix makes `pvs-claim claimed=1` fire, record a Pass or Partial
   depending on whether the next gate is `mimeHandlerPrivate.getStreamInfo()`.

   If the trace shows `ShellDownloadManagerDelegate::ChooseDownloadPath(...)`
   fires before `pvs-start`/`pvs-ready` can commit the wrapper, do not attempt a
   fix in this experiment. Record the exact timing and design Experiment 17
   around `TsBrowserClient`'s download decision: content_shell's default
   `application/pdf` download classification is the gate, and the likely fix is
   to suppress download when the PDF stream path has claimed ownership of the
   navigation.

   If the trace reveals a frame-tree-node mismatch, deletion race, observer
   registration failure, or any other gate, do not guess a fix in this
   experiment; record the exact mismatch and design Experiment 17 around it.

8. Scope guard.

   Forbidden in this experiment:
   - reintroducing `OverrideBodyForInterceptedResponse(...)` for OOPIF PDF;
   - calling `PdfViewerStreamManager::ClaimStreamInfoForTesting()` outside a
     test;
   - mutating stream manager internals to fake a claim;
   - creating synthetic PDF streams not backed by the intercepted response;
   - implementing `GuestViewManager`, `MimeHandlerViewGuest`, or
     `MimeHandlerStreamManager`;
   - replacing the diagnostic `MimeHandlerService.GetStreamInfo()` unless the
     stream is actually claimed and the logs prove that service is now the next
     gate;
   - changing `webtui`, `roamium`, `termsurf.proto`, or Wezboard.

9. Preserve existing foundation work.

   Do not remove or weaken:
   - PDF component extension registration;
   - PDF extension resource serving;
   - `chrome://resources` serving for the PDF viewer;
   - extension renderer activation;
   - PDF viewer private API provider wiring;
   - `PdfHelpBubbleHandlerFactory`;
   - `pdf::mojom::PdfHost`;
   - MIME-handler binders;
   - PDF extension frame binder population;
   - Experiment 14's stream-entry browser-client hooks;
   - Experiment 15's internal PDF plugin registration.

10. Build and archive only after an accepted result.

    Build:

    ```bash
    cd chromium/src
    export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
    autoninja -C out/Default libtermsurf_chromium
    ```

    If the branch reaches a coherent Pass or Partial, commit the Chromium
    branch, regenerate `chromium/patches/issue-792/`, update
    `chromium/README.md`, and commit those main-repo changes. If it fails in a
    way that leaves the branch incoherent, record the failure in this file and
    do not archive that branch.

## Verification

1. Build `libtermsurf_chromium` with `autoninja`.

2. Run the direct PDF navigation smoke through the fake GUI harness:

   ```bash
   LOG_DIR=logs/issue-792-exp16-pdf-$(date +%Y%m%d-%H%M%S)
   scripts/test-issue-792-fake-gui.py \
     http://127.0.0.1:9787/bitcoin.pdf \
     --serve-bitcoin-pdf \
     --log-dir "$LOG_DIR" \
     --seconds 12
   ```

3. Inspect the ordered lifecycle:

   ```bash
   rg '\\[issue-792-exp1[46]\\]|ShellDownloadManagerDelegate|mime-handler-get-stream-info|Stream has been aborted' "$LOG_DIR"
   ```

   The `pvs-add` log must fire for any PDF navigation; this confirms Experiment
   14's stream-add half is intact. If `pvs-start` and `pvs-ready` both never
   fire for the PDF navigation, record this as a Partial with the gate "manager
   observer never invoked."

4. Verify the single-resume invariant.

   For one PDF navigation, the Experiment 16 resume log must fire exactly once:

   ```bash
   COUNT=$(rg --no-filename -c 'interceptor-resume' "$LOG_DIR" | awk '{sum += $1} END {print sum + 0}')
   test "$COUNT" -eq 1
   ```

5. Run the normal HTML smoke:

   ```bash
   LOG_DIR=logs/issue-792-exp16-html-$(date +%Y%m%d-%H%M%S)
   scripts/test-issue-792-fake-gui.py \
     http://localhost:9616/index.html \
     --log-dir "$LOG_DIR" \
     --seconds 6
   ```

   Pass condition: ordinary URL/title/loading messages still appear, and no PDF
   stream claim/interceptor logs fire for HTML.

6. Classify the result.

   **Pass:** the experiment both identifies the missing claim gate and lands the
   smallest proven fix, causing `pvs-claim claimed=1` to fire without regressing
   HTML. This is only expected if the gate is a pure lifecycle-ordering problem.
   If the diagnosis reveals the gate is outside the safe minimal fix scope, such
   as content_shell's download classification, Partial is the correct result and
   the diagnostic value is the deliverable. If the next visible gate is
   `mimeHandlerPrivate.getStreamInfo() stream_info=null`, record that as the
   next experiment.

   **Partial:** the experiment does not make the stream claim, but the ordered
   logs identify the exact reason: manager lifetime, `ReadyToCommit` ordering,
   frame-tree-node mismatch, deletion race, or download classification before
   wrapper commit.

   **Fail:** the branch does not build, Experiment 14's response interception
   regresses, ordinary HTML regresses, resume fires zero or multiple times, the
   logs do not include enough lifecycle detail to identify the next gate, or the
   implementation fakes stream claim instead of proving the canonical path.

7. Record the result in this file and update the README experiment index.

8. Ask Claude to review the implementation, verification artifacts, and result
   language. Fix real issues before proceeding to Experiment 17.
