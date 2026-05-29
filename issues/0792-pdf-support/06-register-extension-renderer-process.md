# Experiment 6: Register Extension Renderer Processes

## Description

Experiment 5 proved TermSurf can serve the PDF component extension's
`index.html` through Chromium's `chrome-extension://` URL-loader path. The
viewer shell now loads as an extension URL, but its static dependencies still
hit policy barriers:

```text
Not allowed to load local resource: chrome://resources/css/text_defaults_md.css
Loading the script 'chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/main.js' violates the following Content Security Policy directive...
Loading the script 'chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/pdf_viewer_wrapper.js' violates the following Content Security Policy directive...
```

The next narrow layer is to make TermSurf's browser process recognize the PDF
viewer renderer as an extension renderer. Electron does this in
`ElectronBrowserClient::SiteInstanceGotProcessAndSite()` by looking up the
extension for the `SiteInstance` URL and inserting the extension id/process id
into `extensions::ProcessMap`. app_shell does the same in
`ShellContentBrowserClient::SiteInstanceGotProcessAndSite()`.

TermSurf currently does not override this hook. That means the PDF viewer frame
can navigate to an extension URL and receive `index.html`, but later extension
resource and policy checks may not know that the renderer process belongs to the
PDF extension.

This experiment adds only that process-map registration layer and measures what
changes. It does not wire PDF navigation, `PdfNavigationThrottle`,
`PdfViewerStreamManager`, guest-view, MimeHandlerView, `--pdf-renderer`,
`chrome://resources` serving, PDF viewer private APIs, or stream handoff.

This experiment must receive Claude design review before implementation. After
implementation and result recording, Claude must review the completed output
before any next experiment is designed.

## Changes

1. Create the Chromium implementation branch.

   Start from the accepted Experiment 5 branch:

   ```bash
   git -C chromium/src checkout 148.0.7778.97-issue-792-exp5
   git -C chromium/src checkout -b 148.0.7778.97-issue-792-exp6
   ```

   Add the branch to `chromium/README.md` only after the branch builds and the
   result is accepted.

2. Add `TsBrowserClient::SiteInstanceGotProcessAndSite()`.

   Implement the same narrow pattern Electron and app_shell use:
   - get the `BrowserContext` from the `SiteInstance`;
   - skip off-the-record contexts;
   - get `extensions::ExtensionRegistry` for the context;
   - resolve the extension with
     `registry->enabled_extensions().GetExtensionOrAppByURL(site_instance->GetSiteURL())`;
   - if no extension matches, return without side effects;
   - if the `SiteInstance` security principal is sandboxed, return without side
     effects;
   - insert:

     ```cpp
     extensions::ProcessMap::Get(browser_context)
         ->Insert(extension->id(),
                  site_instance->GetProcess()->GetDeprecatedID());
     ```

   Scope this to extension URLs discovered by Chromium's registry. Do not
   special-case the PDF extension id unless Chromium's lookup fails and the
   result is recorded as Partial.

   Skipping off-the-record contexts matches Experiment 3's deliberate scoping:
   the PDF extension is only enabled in the regular context.

   `ShouldUseProcessPerSite()` is the canonical Electron/app_shell companion to
   `SiteInstanceGotProcessAndSite()`. It is intentionally out of scope for this
   slice. If a future experiment surfaces multi-process-per-extension behavior
   breaking ProcessMap invariants, that becomes the next narrow slice.

3. Add minimal diagnostics.

   Use Chromium `LOG(INFO)` lines with this exact prefix:

   ```text
   [issue-792-exp6]
   ```

   Required low-volume line when an extension process is inserted:

   ```text
   [issue-792-exp6] process-map-insert extension_id=<id> process_id=<id> site_url=<url>
   ```

   If Chromium's registry lookup fails for the direct PDF extension URL, log:

   ```text
   [issue-792-exp6] process-map-miss site_url=<url>
   ```

   Keep misses low-volume. A miss for ordinary `http://` pages is expected and
   should not be logged. Log `process-map-miss` only when
   `site_instance->GetSiteURL().SchemeIs(extensions::kExtensionScheme)` is true
   and the registry returned no extension.

4. Do not widen the experiment.

   Forbidden in this experiment:
   - PDF navigation or MIME interception;
   - `PdfViewerStreamManager`;
   - guest-view or MimeHandlerView;
   - `--pdf-renderer`;
   - `chrome://resources` URL-loader work;
   - changing the PDF extension manifest;
   - restoring `web_accessible_resources`;
   - adding PDF viewer private APIs.

   If process-map insertion is not enough to unblock the viewer scripts, record
   exactly which policy/resource error remains and design the next experiment
   around that layer.

5. Build and archive only after verification.

   Build:

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp5 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

   If the branch builds and verification passes or produces a useful Partial, do
   the full bookkeeping after Claude after-review accepts the result:
   - commit the Chromium branch;
   - regenerate `chromium/patches/issue-792/`;
   - add the new branch row to `chromium/README.md`;
   - update Experiment 6's line in `issues/0792-pdf-support/README.md` from
     `Designed` to the final status.

## Verification

1. Confirm starting state.

   ```bash
   git status --short
   git -C chromium/src status --short
   git -C chromium/src branch --show-current
   ```

   Chromium should start clean on `148.0.7778.97-issue-792-exp5`.

2. Build the branch.

   ```bash
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   git -C chromium/src cl format --upstream=148.0.7778.97-issue-792-exp5 --full
   autoninja -C chromium/src/out/Default libtermsurf_chromium
   ```

3. Run the direct extension-resource smoke.

   Reuse the debug screenshot harness against:

   ```text
   chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/index.html
   ```

   Pass requires:
   - Experiment 5 still serves `index.html`:

     ```text
     [issue-792-exp5] bundle-resource-load path=index.html resource_id=21596 bytes=<n> mime=text/html ok=1
     ```

   - Experiment 6 inserts the PDF extension renderer process:

     ```text
     [issue-792-exp6] process-map-insert extension_id=mhjfbmdgcfjbbpaeojofohoefgiehjai process_id=<id> site_url=chrome-extension://mhjfbmdgcfjbbpaeojofohoefgiehjai/
     ```

   - no `FATAL`, `NOTREACHED`, renderer IPC crash, or hang occurs before the
     screenshot artifact is captured.

   Compare the post-insert console/resource errors with Experiment 5's direct
   extension artifact:

   ```text
   logs/issue-792-exp5-extension-20260529-094357/
   ```

   Record whether the `main.js` and `pdf_viewer_wrapper.js` CSP errors are gone,
   changed, or unchanged. If they remain unchanged, process-map insertion was
   necessary foundation but not sufficient for viewer script loading.

4. Run normal HTML regression smoke.

   Load:

   ```text
   http://localhost:9616/index.html
   ```

   Pass requires the page to render or lifecycle logs to reach `TitleChanged`
   and `LoadingState`, with no extension IPC crash.

5. Run the PDF unchanged smoke.

   Load:

   ```text
   http://localhost:9616/bitcoin.pdf
   ```

   The PDF is still expected to take the default content_shell download path
   because this experiment does not install PDF navigation or stream handling. A
   browser crash, renderer IPC crash, or hang is a failure.

6. Run Claude review after recording the result.

   Provide Claude with the experiment file, Chromium diff, build output summary,
   runtime logs, screenshot artifact paths, and the recorded result. Fix all
   real findings before proceeding.

## Pass Criteria

- Chromium branch `148.0.7778.97-issue-792-exp6` builds `libtermsurf_chromium`.
- Direct navigation to the PDF component extension still serves `index.html`
  from `ui::ResourceBundle`.
- `SiteInstanceGotProcessAndSite()` inserts the PDF extension id/process id into
  `extensions::ProcessMap`.
- The direct extension smoke does not crash or hang.
- Normal HTML browsing still works through the debug TermSurf path.
- Loading `bitcoin.pdf` does not crash; rendering is not required.
- Claude reviews the completed result and agrees it is good enough to proceed.

## Partial Criteria

Partial if:

- the branch builds and the hook fires, but
  `GetExtensionOrAppByURL(site_instance->GetSiteURL())` returns no extension for
  the direct PDF extension URL;
- process-map insertion succeeds, but the viewer scripts remain blocked by the
  same CSP/resource errors from Experiment 5;
- process-map insertion succeeds, but the next missing layer is clearly
  `chrome://resources` serving, manifest policy, `web_accessible_resources`, or
  viewer API binding.

## Failure Criteria

- The experiment changes PDF navigation, stream handling, guest-view,
  MimeHandlerView, or `--pdf-renderer`.
- The experiment changes the PDF extension manifest or restores
  `web_accessible_resources`.
- The experiment imports Chrome browser UI/resource stacks.
- The experiment changes TermSurf protocol, Wezboard, Roamium Rust, or webtui.
- The experiment regresses normal HTML browsing or reintroduces the extension
  renderer IPC crash.
- The experiment proceeds without Claude design review or ignores real Claude
  findings.
