# Experiment 1: Inventory and Classify Missing Browser APIs

## Description

Build the authoritative inventory of TermSurf's missing browser APIs and
web-feature surfaces, then classify each item by whether it can be implemented
and verified automatically.

This is a research and triage experiment only. It must not change runtime code,
Chromium branches, protocol messages, or browser behavior. The output is a
decision table that defines the implementation scope for the rest of Issue 799.

The key question is not "does TermSurf eventually need this?" The key question
is "can this issue solve and verify this item without relying on ongoing manual
testing?" Items that fail that automation bar must be deferred with a clear
reason, even if they are important product gaps.

## Changes

1. Audit source documents.

   Read and extract candidate APIs/features from:
   - `TODO.md`, especially the Web features and Future issues sections;
   - `issues/0616-web-features/README.md`;
   - `issues/0655-substack-blank/README.md`;
   - `issues/0750-target-blank/README.md`;
   - `issues/0780-link-drag-freeze/README.md`;
   - `issues/0792-pdf-support/README.md`;
   - `issues/0794-pdf-viewer-interactions/README.md`;
   - `issues/0795-pdf-native-print/README.md`;
   - `issues/0796-pdf-implementation-audit/README.md`;
   - `issues/0797-pdf-core-workflow-coverage/README.md`;
   - `issues/0798-pdf-advanced-features/README.md`;
   - any other issue found by grep for missing browser APIs, Mojo binders,
     browser delegates, permissions, downloads, dialogs, file pickers, auth,
     crash recovery, console capture, or drag/drop.

2. Audit current implementation evidence.

   Use source searches to determine whether each candidate is already solved,
   partially solved, missing, or unknown. At minimum, inspect:
   - `termsurf.proto` and generated protocol usage for relevant message types;
   - `roamium/src/dispatch.rs`;
   - `roamium/src/ffi.rs`;
   - `webtui/src/`;
   - `wezboard/wezboard-gui/src/termsurf/`;
   - `wezboard/wezboard-gui/src/termwindow/`;
   - `chromium/src/content/libtermsurf_chromium/`;
   - `chromium/README.md` and the latest relevant Chromium patch archives.

   The experiment should use concrete greps, not memory. Suggested searches:

   ```bash
   rg -n "alert|confirm|prompt|download|upload|file picker|FileChooser|zoom|Basic Auth|Auth|permission|camera|microphone|console|crash|BadgeService|mojo|binder|drag|drop|clipboard|DevTools|target|blank" \
     TODO.md issues docs roamium webtui wezboard/wezboard-gui chromium/README.md

   rg -n "RegisterBrowserInterfaceBinders|RegisterAssociatedInterfaceBinders|AddInterface|mojom|BadgeService|Permission|Download|FileChooser|JavaScriptDialog|AuthRequired|RenderProcessGone|RenderFrameDeleted" \
     chromium/src/content/libtermsurf_chromium
   ```

   If Chromium source searches are too broad or slow, narrow them to
   `content/libtermsurf_chromium`, `content/shell`, `chrome/browser`,
   `components/`, and the branch patch archives relevant to TermSurf.

3. Compare TermSurf against reference embedders.

   The inventory must not only ask "what did TermSurf already mention?" It must
   also compare TermSurf's embedder surface against Chromium embedders that wire
   browser APIs more completely.

   Inspect reference code where practical:
   - `chromium/src/content/public/browser/content_browser_client.h`;
   - `chromium/src/content/public/renderer/content_renderer_client.h`;
   - `chromium/src/content/shell/`;
   - `chromium/src/headless/`;
   - relevant `chromium/src/chrome/browser/` binder, delegate, permission,
     download, file chooser, dialog, auth, and crash-handling code;
   - local Electron source, if present, especially `shell/browser/` and
     `shell/renderer/`.

   For each candidate where this comparison is meaningful, record whether
   Chrome, content shell, headless, or Electron provides a binder/delegate/API
   implementation that TermSurf lacks. The result can be brief, but it must be
   specific enough to justify the classification. For example:

   | Candidate     | Reference evidence              | TermSurf evidence         |
   | ------------- | ------------------------------- | ------------------------- |
   | BadgeService  | headless has `StubBadgeService` | fixed in Issue 655        |
   | File uploads  | Chrome wires file chooser       | no TermSurf path observed |
   | Notifications | Chrome has permission stack     | no TermSurf binder audit  |

4. Include broad Web Platform browser-service candidates.

   Issue 616 lists the known product gaps, but Issue 655 showed that renderer
   crashes can come from any missing browser-process API surface exposed through
   Mojo or delegates. The inventory must therefore include at least these broad
   families, even if many are deferred:
   - Notifications and Push API browser services;
   - Geolocation;
   - WebAuthn and Credential Management;
   - Payment Request;
   - Web Share;
   - File System Access and related storage/quota prompts;
   - Web Bluetooth, USB, HID, Serial, and MIDI;
   - screen capture and media-capture permissions;
   - Permissions API plumbing;
   - service-worker-adjacent browser services that require browser-process
     delegates or binders.

   If a family is not relevant to the current Chromium build, disabled by
   feature flags, unsupported by content shell, or intentionally outside
   TermSurf's product scope, record that instead of omitting it.

5. Create the inventory table inside this experiment file.

   Append a `## Result` section containing a table with one row per candidate.
   The table must include these columns:

   | Candidate | Source | Current status | Likely layer | Automation class | Proposed verification | In Issue 799 scope? | Deferred/follow-up |
   | --------- | ------ | -------------- | ------------ | ---------------- | --------------------- | ------------------- | ------------------ |

   Classification values:
   - `Automatable now`;
   - `Automatable after setup`;
   - `Deferred: not automatable enough`;
   - `Already solved`;
   - `Out of scope product decision`.

6. Define concrete verification plans for in-scope items.

   For every `Automatable now` or `Automatable after setup` item, record a
   proposed automated verification method. Acceptable methods include:
   - local deterministic HTML fixtures;
   - fake-GUI protocol harnesses;
   - DevTools Protocol probes;
   - screenshot classification;
   - log assertions for Chromium/Roamium/Wezboard;
   - contained download directories;
   - synthetic protocol messages;
   - controlled local servers;
   - test-only env vars or command-line switches, as long as production behavior
     remains unchanged.

   Do not mark an item in-scope if the proposed verification only says "manual
   test" or "visually inspect."

7. Treat native UI and platform permissions carefully.

   Items involving native dialogs, file pickers, permission prompts, print
   dialogs, camera/mic permissions, system Accessibility permissions, Screen
   Recording permissions, or OS-level drag/drop must be classified honestly.
   They can remain in scope only if the experiment identifies a contained or
   deterministic automation strategy. Otherwise they must be deferred.

8. Decide the next experiment.

   Append a `## Conclusion` section that states:
   - the in-scope implementation queue for Issue 799;
   - the deferred list and why each item is deferred;
   - any "automatable after setup" harness work that should happen before the
     first feature implementation;
   - the recommended Experiment 2.

9. Review the result with Codex before treating the triage as complete.

   After filling in `## Result` and `## Conclusion`, run `codex-review` against
   the completed experiment. Ask Codex to check for:
   - missing candidate APIs from the cited source documents;
   - incorrect "already solved" claims;
   - automation classifications that are too optimistic;
   - deferred items that actually have a practical automation path;
   - in-scope items whose proposed verification would not prove the feature.

   Fix all real findings before marking the experiment `Pass`, `Partial`, or
   `Fail`.

10. Update the issue README. When the result is recorded, update this
    experiment's line in `issues/0799-browser-api-automation-triage/README.md`
    from `Designed` to the final status.

## Verification

This experiment passes if:

- the candidate list is derived from issue history, `TODO.md`, and current code,
  not from memory alone;
- every candidate has exactly one automation classification;
- every in-scope candidate has a concrete automated verification proposal;
- every deferred candidate has a concrete deferral reason;
- already solved items cite the issue or source evidence that solved them;
- Codex reviews the completed triage and no blocking findings remain;
- no runtime code, protocol files, Chromium branches, patch archives, or test
  harness behavior changes are made.

This experiment is partial if:

- the inventory is useful but one or more major source areas could not be
  audited;
- Chromium source access or branch state prevents confident classification of
  Mojo binder coverage;
- Codex identifies unresolved gaps that require a second triage experiment.

This experiment fails if:

- it implements behavior instead of triaging;
- it marks manual-only work as automatable without a credible containment plan;
- it omits Issue 616, Issue 655, or `TODO.md` from the source inventory;
- it produces an implementation order without enough evidence to justify the
  automation boundary.

## Result

**Result:** Pass

Experiment 1 audited the required source documents and current implementation
surface without changing runtime code. The audit confirmed that Issue 799 should
not be treated as a generic "implement Chrome" project. The automatable core is
narrower:

1. prove and harden missing Mojo/browser-service binders so page JavaScript does
   not crash the renderer;
2. build automation harnesses for deterministic product features such as
   JavaScript dialogs, downloads, page zoom, HTTP auth, console capture, crash
   UX, and profile/session behavior;
3. defer native UI, hardware, OS permission, product-design, and broad Chrome
   feature stacks unless a later experiment creates a contained automation path.

### Audit Evidence

Source documents inspected:

- `TODO.md` records the remaining web-feature list and explicitly calls the
  missing Mojo binder class a "ticking time bomb."
- Issue 616 inventories the original 20 missing web features and says downloads,
  file uploads, JavaScript dialogs, HTTP auth, crash recovery, camera/mic
  permissions, console capture, DevTools, and dynamic titles need Chromium-side
  changes.
- Issue 655 proves the renderer-crash failure mode with
  `blink.mojom.BadgeService`, then records the systematic Mojo-interface audit
  as future work.
- Issue 750 records `target="_blank"` / `window.open()` as solved by routing new
  windows into the current tab.
- Issue 780 records native HTML drag-and-drop as intentionally suppressed to
  prevent freezes, while real cross-process drag/drop remains future work.
- Issues 792, 794, 796, 797, and 798 record that PDF-specific browser APIs,
  stream plumbing, resources, input, save/download, and title propagation are
  solved enough for the non-print PDF scope, while PDF native print and advanced
  PDF features remain separate issues.

Current implementation evidence:

- `proto/termsurf.proto` has tab lifecycle, navigation, input, focus/color
  state, overlay, DevTools, query, title, URL, loading, cursor, and target URL
  messages. It has no generic dialog, download, file chooser, auth, permission,
  crash report, notification, device, or browser-service API messages.
- `roamium/src/dispatch.rs` and `roamium/src/ffi.rs` route the current protocol
  to Chromium FFI for tabs, DevTools, navigation, input, focus, resize, query,
  and callback notifications. They do not expose general browser API requests.
- `webtui/src/main.rs` implements URL normalization, DevTools commands,
  clipboard-backed editor state, and existing browser navigation UX. It does not
  implement dialogs, downloads, uploads, auth prompts, permission prompts,
  bookmarks, or crash recovery UI.
- `wezboard/wezboard-gui/src/termsurf/` handles overlay lifecycle, routing,
  input, browser process management, and DevTools. It does not provide generic
  browser feature prompts or download/upload surfaces.
- `chromium/src/content/libtermsurf_chromium/ts_browser_client.cc` registers
  PDF-specific browser support and the Issue 655 `StubBadgeService`; it does not
  register broad Chrome browser services.
- `chromium/src/content/libtermsurf_chromium/extensions/` contains a PDF-scoped
  extension system and PDF-scoped `resourcesPrivate` / `pdfViewerPrivate`
  implementations. Those are intentionally not a general Chrome extension or
  browser API provider.
- `chromium/src/content/public/browser/content_browser_client.h` exposes
  embedder hooks for download directories, frame binders, service-worker
  binders, forced downloads, secure payment confirmation, Serial, HID,
  Bluetooth, USB, WebAuthn, and HTTP auth. TermSurf does not currently provide
  matching broad implementations.
- `chromium/src/headless/` provides useful reference patterns for headless
  download, notification, permission, DevTools, geolocation, Bluetooth, and
  BadgeService behavior; the BadgeService pattern is exactly what Issue 655
  copied.

Solved `TODO.md` entries that are terminal/input/UI infrastructure rather than
missing browser API surfaces are not implementation candidates for Issue 799:
loading progress, browser navigation keybindings, context menu removal, and the
Ctrl+Esc mode-switching use-after-free fix were already completed in earlier
issues. They remain excluded from the implementation queue unless a future
regression issue reopens them.

### Inventory

| Candidate                                        | Source                                                                                                | Current status                                                                               | Likely layer                                                                                                               | Automation class                                     | Proposed verification                                                                                                                                                             | In Issue 799 scope?                                                    | Deferred/follow-up                                                   |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | -------------------------------------------------------------------- |
| Missing Mojo/browser-service binder audit        | Issue 655, `TODO.md`, `ContentBrowserClient` hooks                                                    | Partially solved: `BadgeService` stub exists; broad audit not done                           | Chromium `TsBrowserClient`, browser/service-worker binder maps, reference comparison against Chrome/headless/content shell | Automatable after setup                              | Build local API caller pages plus Chromium log scanner for bad Mojo messages; compare TermSurf binder/delegate coverage against selected Chrome/headless/content-shell references | Yes                                                                    | Experiment 2 should build the audit harness and initial coverage map |
| Renderer crash UX                                | Issue 655, `TODO.md`, Issue 616                                                                       | Missing user-visible recovery; `TsTabObserver::RenderProcessGone()` sends loading error only | Chromium observer, Roamium notification, Wezboard/TUI display                                                              | Automatable after setup                              | Use a local crash fixture or Chromium-controlled renderer kill, assert crash message/progress clear/error UI via protocol logs and screenshot/terminal state                      | Yes                                                                    | Needs crash fixture and expected UX design before implementation     |
| JavaScript dialogs: `alert`, `confirm`, `prompt` | Issue 616, `TODO.md`                                                                                  | Missing TermSurf dialog prompt/reply surface                                                 | Chromium JavaScript dialog manager, protocol, Wezboard/TUI prompt UI                                                       | Automatable after setup                              | Local HTML page triggers each dialog; harness replies automatically; assert JS return values and no renderer hang                                                                 | Yes                                                                    | Needs protocol/design for contained prompt/reply path                |
| Downloads                                        | Issue 616, `TODO.md`; PDF save covered separately                                                     | Generic downloads missing; PDF viewer save works through PDF-specific path                   | Chromium download manager/delegate, protocol or contained download policy, TUI/GUI status                                  | Automatable now                                      | Local server serves deterministic attachment/blob; set per-run download dir; assert file hash, lifecycle event, and no native dialog                                              | Yes                                                                    | Start with non-native contained download path                        |
| File uploads: `<input type=file>`                | Issue 616, `TODO.md`; Issue 780 for drag/upload overlap                                               | Missing generic file chooser/upload UX                                                       | Chromium file chooser delegate, protocol/UI, possibly test-only chooser auto-response                                      | Automatable after setup                              | Local upload fixture; test-only auto-select fixture file or protocol-driven file selection; assert uploaded bytes/server receipt                                                  | Yes, only if the first implementation uses a contained automation path | Native NSOpenPanel-only solution is deferred                         |
| Page zoom                                        | Issue 616, `TODO.md`                                                                                  | Missing general page zoom; PDF zoom is PDF-specific                                          | Chromium WebContents zoom controller or command path; webtui/Wezboard key routing                                          | Automatable now                                      | Local page with measurable layout/text; trigger Cmd+=/-/0 or protocol command; assert zoom factor via DevTools or screenshot/layout metrics                                       | Yes                                                                    | None                                                                 |
| HTTP Basic Auth                                  | Issue 616, `TODO.md`                                                                                  | Missing prompt/credential response path                                                      | Chromium `CreateLoginDelegate`, protocol/TUI prompt, credential callback                                                   | Automatable after setup                              | Local HTTP server requiring Basic Auth; automation supplies credentials; assert protected page loads and failure path works                                                       | Yes                                                                    | Needs prompt/reply or test credential source design                  |
| Console capture                                  | Issue 616, `TODO.md`                                                                                  | Missing app-facing console stream; DevTools can observe separately                           | Chromium `WebContentsObserver::DidAddMessageToConsole` or DevTools Protocol bridge; protocol/TUI output                    | Automatable now                                      | Local page emits `console.log/warn/error`; assert messages in protocol/log/TUI capture with level/source/line                                                                     | Yes                                                                    | None                                                                 |
| Camera/microphone permissions                    | Issue 616, `TODO.md`, broad browser permissions                                                       | Generic page media permission UX missing; PDF extension has its own internal media allowance | Chromium permission/media access stack, OS media permissions, fake-device flags                                            | Deferred: not automatable enough for product feature | Fake-device tests can prove API plumbing, but real macOS camera/mic permission UX needs OS-level permissions and product decisions                                                | No                                                                     | Follow-up once permission UX and fake-device strategy are designed   |
| Screen capture / `getDisplayMedia`               | Broad candidate list, `ContentBrowserClient` media hooks                                              | Not implemented as a TermSurf product surface                                                | Chromium media/capture permission stack, OS Screen Recording permission                                                    | Deferred: not automatable enough                     | OS Screen Recording permission and native picker behavior are not reliably containable in this issue                                                                              | No                                                                     | Future permission/media issue                                        |
| Generic Permissions API plumbing                 | Issue 655 class, broad candidate list                                                                 | Unknown/incomplete outside PDF extension permissions                                         | Chromium permission controller/delegates                                                                                   | Automatable after setup                              | Local pages call `navigator.permissions.query()` for deterministic permission names; assert no bad Mojo/renderer kill and expected deny/prompt states                             | Yes for crash-safe/default-deny audit, not full UX                     | Full prompt UI belongs to feature-specific issues                    |
| Notifications and Push                           | Issue 655 class, broad candidate list; headless/chrome references                                     | `BadgeService` solved; generic notifications/push not audited                                | Chromium notification service, permission delegate, service worker binders                                                 | Automatable after setup                              | Local notification/push API pages under secure local origin; assert stable deny/no-crash behavior and no bad Mojo                                                                 | Yes for crash-safe/default-deny audit                                  | Real OS notifications/push service integration deferred              |
| Geolocation                                      | Issue 616 camera/mic-style permissions, broad candidate list; `ContentBrowserClient` geolocation hook | No TermSurf geolocation UX/API                                                               | Chromium geolocation permission and provider hooks                                                                         | Automatable after setup                              | Use fake geolocation provider or deny path; local page calls `getCurrentPosition`; assert deterministic denied/fake result and no crash                                           | Yes for deterministic fake/deny path                                   | Real user permission UX deferred                                     |
| WebAuthn / Credential Management                 | Broad candidate list; `ContentBrowserClient` WebAuthn hooks                                           | Not implemented/audited for TermSurf                                                         | Chromium WebAuthenticationDelegate, credential APIs                                                                        | Automatable after setup                              | DevTools virtual authenticator and local WebAuthn fixture; assert no renderer kill and expected success/denial                                                                    | Yes for crash-safe/virtual-authenticator path                          | Real authenticator UX/security decisions deferred                    |
| Payment Request / Secure Payment Confirmation    | Broad candidate list; `ContentBrowserClient` payment hooks                                            | Not implemented/audited                                                                      | Chromium payment service/delegate                                                                                          | Deferred: not automatable enough                     | Requires payment UI/product/security decisions; crash-safe API calls can be covered by the generic binder audit                                                                   | No feature work                                                        | Defer full payment support; include only in binder/no-crash audit    |
| Web Share                                        | Broad candidate list                                                                                  | Not implemented/audited                                                                      | Chromium share service / platform integration                                                                              | Deferred: not automatable enough                     | Native share sheet semantics are platform UI and product-scope heavy; no current TermSurf UX                                                                                      | No                                                                     | Future platform integration issue                                    |
| File System Access / storage/quota prompts       | Broad candidate list; `ContentBrowserClient` file-system access hook                                  | Not implemented/audited                                                                      | Chromium file-system access permission context, storage/quota delegates                                                    | Automatable after setup                              | Local fixture calls file-system access APIs with test-only auto-deny/contained temp dir; assert no crash and deterministic status                                                 | Yes for contained deny/temp-dir path                                   | Real native picker or persistent permission UX deferred              |
| Web Bluetooth / USB / HID / Serial / MIDI        | Broad candidate list; `ContentBrowserClient` device delegates; headless Bluetooth reference           | Not implemented/audited                                                                      | Chromium device delegates, chooser UI, OS hardware permissions                                                             | Deferred: not automatable enough                     | Hardware/native chooser behavior cannot be solved generally here; no-crash/default-deny can be part of binder audit                                                               | No feature work                                                        | Future hardware API issue if product requirement appears             |
| Service-worker-adjacent browser services         | Issue 655 class, broad candidate list; `ContentBrowserClient` service-worker binder hook              | Unknown/incomplete outside PDF/extension work                                                | Chromium service-worker binder maps and browser services                                                                   | Automatable after setup                              | Local service worker fixture calls selected APIs; scan for bad Mojo kills and expected default-deny behavior                                                                      | Yes for no-crash audit                                                 | Feature-specific service worker behavior deferred as needed          |
| Session isolation / incognito mode               | Issue 616, `TODO.md`; profile model in AGENTS.md                                                      | Named profiles exist; incognito/ephemeral profile UX missing                                 | Roamium profile path/browser context creation, webtui CLI/profile handling                                                 | Automatable now                                      | Launch two profiles/incognito temp profile against local storage/cookie fixture; assert persistence/isolation behavior across restarts                                            | Yes                                                                    | Needs product decision on CLI flag/name, but automatable             |
| Bookmarking                                      | Issue 616, `TODO.md`                                                                                  | Missing product feature                                                                      | webtui storage/UI, maybe hosted sync later                                                                                 | Out of scope product decision                        | Automation is possible, but scope is product design/storage/sync rather than browser API crash safety                                                                             | No                                                                     | Separate bookmarks/passwords product issue                           |
| Hosted passwords                                 | `TODO.md` 1.0/future list                                                                             | Missing product feature                                                                      | Password manager/storage/sync/security                                                                                     | Out of scope product decision                        | Requires security/product architecture beyond browser API triage                                                                                                                  | No                                                                     | Separate credentials/password manager issue                          |
| TermSurf JavaScript API: `window.termsurf`       | Issue 616, `TODO.md`                                                                                  | Missing                                                                                      | Chromium renderer bindings or injected script, protocol callback                                                           | Out of scope product decision                        | Technically automatable with local fixture, but API shape/security semantics are undefined                                                                                        | No                                                                     | Separate API design issue before implementation                      |
| Hide/show webviews / Ctrl-Z foregrounding        | Issue 616, `TODO.md`                                                                                  | Missing                                                                                      | Wezboard overlay lifecycle, webtui mode/process handling                                                                   | Out of scope product decision                        | More terminal/session lifecycle than browser API; automation possible but not part of this issue's API triage                                                                     | No                                                                     | Separate terminal lifecycle issue                                    |
| Multi-webview stacking per pane                  | Issue 616, `TODO.md`                                                                                  | Missing; current model is one overlay per pane                                               | Protocol/Wezboard overlay model/webtui UX                                                                                  | Out of scope product decision                        | Architecture/product change, not a missing browser service                                                                                                                        | No                                                                     | Separate overlay architecture issue                                  |
| Native/web drag-and-drop and file drops          | Issue 780, `TODO.md`                                                                                  | Native drag start suppressed to prevent freeze; full DnD missing                             | Chromium macOS drag path, Wezboard/Roamium cross-process protocol, OS drag events                                          | Deferred: not automatable enough                     | Requires native AppKit drag/drop bridge and likely manual/OS interaction; can be partially probed but not solved safely here                                                      | No                                                                     | Separate drag/drop design issue                                      |
| `target="_blank"` / `window.open()`              | Issue 750, `TODO.md`                                                                                  | Solved: current tab navigation instead of orphan window                                      | Chromium Content Shell `WebContentsDelegate` patch                                                                         | Already solved                                       | Existing issue tests; future regression can use local target blank fixture                                                                                                        | No                                                                     | None                                                                 |
| Clipboard copy/cut/paste                         | `TODO.md`, webtui/Wezboard code, PDF interactions                                                     | Solved for browser overlays and PDF selection/copy                                           | Wezboard input routing, Chromium edit commands, webtui clipboard                                                           | Already solved                                       | Existing protocol/input/PDF copy probes                                                                                                                                           | No                                                                     | None                                                                 |
| DevTools / Web Inspector                         | Issue 616, `TODO.md`, Issues 684/687/775                                                              | Solved enough for current product                                                            | Chromium DevTools tab creation, webtui command, Wezboard split routing                                                     | Already solved                                       | Existing DevTools query/open regression paths                                                                                                                                     | No                                                                     | None                                                                 |
| URL normalization                                | Issue 616, `TODO.md`, `webtui/src/main.rs`                                                            | Solved: omitted scheme becomes `https://`                                                    | webtui URL normalization                                                                                                   | Already solved                                       | Unit-level/local invocation or existing manual coverage                                                                                                                           | No                                                                     | None                                                                 |
| Dynamic tab titles                               | Issue 616, `TODO.md`, Issues 638/778, PDF title work                                                  | Solved for normal pages and PDFs                                                             | Chromium title observer, protocol, TUI display                                                                             | Already solved                                       | Existing title propagation probes and Issue 778 back/forward fix                                                                                                                  | No                                                                     | None                                                                 |
| PDF-specific browser APIs/interactions           | Issues 792, 794, 796-798                                                                              | Core non-print PDF scope solved; print and advanced features split out                       | PDF extension/resource/stream/Mojo/input plumbing                                                                          | Already solved                                       | Existing PDF automation suite; Issue 797/798 own remaining PDF checks                                                                                                             | No                                                                     | Issue 795, 797, 798 own PDF follow-ups                               |
| Native PDF print                                 | Issue 795                                                                                             | Open and intentionally out of Issue 799 scope                                                | PDF print/browser-side native print host                                                                                   | Out of scope product decision                        | Native print UI is a separate issue and not part of general browser API triage                                                                                                    | No                                                                     | Issue 795                                                            |
| Loading progress bar                             | `TODO.md`, Issue 616                                                                                  | Solved: indeterminate progress pulse integrated with Chromium loading lifecycle              | Chromium observer/protocol/TUI progress display                                                                            | Already solved                                       | Existing loading-state path sends `LoadingState`; future regressions can use local slow-resource fixture                                                                          | No                                                                     | None                                                                 |
| Browser navigation keybindings                   | `TODO.md`, Issue 616                                                                                  | Solved: back, forward, reload keybindings exist                                              | webtui/Chromium key command routing                                                                                        | Already solved                                       | Existing keybinding/navigation tests or local history fixture                                                                                                                     | No                                                                     | None                                                                 |
| Context menu removal                             | `TODO.md`, Issue 616                                                                                  | Solved: inherited Content Shell context menu removed because it did not fit TermSurf         | Chromium Content Shell / native menu behavior                                                                              | Already solved                                       | Future regression can right-click local page and assert no native Content Shell menu/focus steal                                                                                  | No                                                                     | None                                                                 |
| Ctrl+Esc mode switching                          | `TODO.md`, Issue 616                                                                                  | Solved: dangling pointer in focused pane fixed                                               | Wezboard/TUI mode state                                                                                                    | Already solved                                       | Existing mode-switch smoke; not a browser API surface                                                                                                                             | No                                                                     | None                                                                 |
| User-Agent spoofing                              | Issue 616 lower-priority row                                                                          | Not needed for Chromium: Chromium sends a real browser UA                                    | Chromium network/user-agent policy                                                                                         | Out of scope product decision                        | Could be tested with local UA echo page, but no current product requirement                                                                                                       | No                                                                     | Reopen only if sites serve wrong layouts                             |
| Header injection / Upgrade-Insecure-Requests     | Issue 616 lower-priority row                                                                          | Not needed: recorded as WKWebView-specific workaround; Chromium handles relevant behavior    | Chromium network stack                                                                                                     | Out of scope product decision                        | Local server header capture could verify if a product requirement appears                                                                                                         | No                                                                     | None                                                                 |
| Blob download workaround                         | Issue 616 lower-priority row                                                                          | Not needed as WKWebView bug workaround; generic blob downloads belong to Downloads row       | Chromium download stack                                                                                                    | Already solved                                       | Generic download automation should include a blob download fixture                                                                                                                | No                                                                     | Covered by Downloads row                                             |

### In-Scope Queue

The implementation queue for Issue 799 should be:

1. **Experiment 2: missing browser-service/Mojo binder audit harness.** Build
   the no-crash API fixture suite and reference comparison map. This is the
   direct successor to Issue 655 and should prevent more Substack-style renderer
   kills.
2. **JavaScript dialogs.** This is high-impact, deterministic, and can be
   verified with local pages once a prompt/reply path exists.
3. **Generic downloads.** This can be contained with per-run download
   directories and local fixtures.
4. **Page zoom.** This is small, deterministic, and likely requires no native
   UI.
5. **Console capture.** This is developer-facing and easy to verify with local
   pages.
6. **HTTP Basic Auth.** This is automatable with a local auth server after a
   credential-response design.
7. **Crash recovery UX.** This should follow the binder audit so crash causes
   are easier to identify.
8. **Permission/API default-deny hardening.** Include generic Permissions API,
   Notifications/Push, Geolocation, WebAuthn virtual-authenticator, File System
   Access, and service-worker-adjacent no-crash/default-deny checks only where
   automation can prove them.
9. **Session isolation/incognito.** Automatable, but lower priority than
   renderer-crash prevention and common browsing features.

### Deferred Items

Deferred from Issue 799:

- real native drag/drop and file drops: needs cross-process AppKit/OS drag
  design;
- camera/mic and screen capture product UX: OS permissions and native capture
  surfaces are not reliably containable here;
- Payment Request, Web Share, and hardware device APIs as full features:
  product/security/native UI scope is too broad, though no-crash probes can be
  part of the binder audit;
- bookmarks, hosted passwords, and `window.termsurf`: product/API design first;
- hide/show webviews and multi-webview stacking: terminal overlay architecture,
  not browser API plumbing;
- native PDF print and advanced PDF workflows: already tracked by Issues 795,
  797, and 798.

### Codex Completion Review

Codex completion review initially found one real gap: the result did not
explicitly account for Issue 616's lower-priority User-Agent/header/blob rows or
the solved infrastructure entries from `TODO.md`. The table was updated to
include those rows and an exclusion note. Follow-up Codex review reported no
findings and said Experiment 1 can be marked `Pass`.

## Conclusion

The triage succeeded. The most important missing browser API work is not a
single user-visible feature; it is the systematic missing binder/delegate audit
that Issue 655 warned about. TermSurf already fixed one instance
(`BadgeService`), but the current protocol and `libtermsurf_chromium` code show
there is no general browser API layer for dialogs, downloads, file chooser,
auth, permission prompts, crash UX, notifications, or device APIs.

Experiment 2 should build the automated browser-service/Mojo audit harness and
initial reference coverage map. That harness becomes the safety net for later
feature work: it should identify APIs that crash the renderer, APIs that fail
cleanly, and APIs that need scoped TermSurf-owned stubs or delegates.
