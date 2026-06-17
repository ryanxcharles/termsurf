# Experiment 10: Batch C Product Hardening Audit

## Description

Classify Batch C from Experiment 4: issues `0600`-`0679`. This batch covers the
late TermSurf/Ghostboard product-hardening era before the direct-browser and
socket migration work: initial Ghostboard/XPC setup, early browser overlay
demos, multi-pane and multi-profile behavior, mouse and keyboard input, URL and
search input, app/icon/rename/docs cleanup, XDG behavior, web features, alpha
readiness, URL sync, input latency, Chromium embedding, CALayerHost rendering,
navigation, persistent compositing, active pane tracking, editable URL bar,
profile server work, TUI mode/keybinding behavior, context menus, escape-key
behavior, resize/focus fixes, scripts, website, hello messages, URL
normalization, and licensing.

This experiment should read every Batch C issue and map each durable lesson to
current Ghostboard risk using the schema defined in Experiment 4. The output is
a classification table, not fixes.

This is an audit/documentation experiment only. It must not change application
code, generated code, historical issue files, closed issue files, scripts, test
harnesses, screenshots, website assets, or build configuration.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/10-batch-c-product-hardening.md`
  - record this experiment design, design review, Batch C classification result,
    completion review, and conclusion;
  - classify every issue in Batch C using the Experiment 4 historical audit row
    schema.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 10 to the `## Experiments` index with status `Designed`, then
    update status after the result.

No application code, generated protobuf code, historical issue files, closed
issue files, scripts, test harnesses, screenshots, website assets, or build
configuration should be edited.

## Verification

Design-gate pass criteria:

- The issue README links this experiment as `Designed`.
- A fresh-context adversarial design review approves the plan.
- The plan commit exists before implementation begins.

Implementation pass criteria:

- The result audits every Batch C issue exactly once:
  - `0600-termsurf-ghost`
  - `0601-zig-xpc`
  - `0602-pink-texture`
  - `0603-box-demo`
  - `0604-two-panes`
  - `0605-two-profiles`
  - `0606-mouse-input`
  - `0607-keyboard-input`
  - `0608-search-input`
  - `0609-keyboard-input-2`
  - `0610-app-icon`
  - `0611-rename`
  - `0612-icon`
  - `0613-rename-directories`
  - `0614-docs-review`
  - `0615-xdg`
  - `0616-web-features`
  - `0617-alpha`
  - `0618-url-sync`
  - `0619-input-latency`
  - `0620-zig-content-shell`
  - `0621-single-process`
  - `0622-javascript-is-slow`
  - `0623-viz-display-serialization`
  - `0624-chromium-ipc`
  - `0625-calayerhost`
  - `0626-x-y-calayerhost`
  - `0627-resize-calayerhost`
  - `0628-navigation-calayerhost`
  - `0629-understand-nav-calayerhost`
  - `0630-nav-calayerhost-6`
  - `0631-continue-nav-calayerhost`
  - `0632-nav-flicker-calayerhost`
  - `0633-persistent-compositor`
  - `0634-calayerhost-audit`
  - `0635-multi-pane-calayerhost`
  - `0636-calayerhost-audit`
  - `0637-editable-url-bar`
  - `0638-page-title`
  - `0639-open-in-same-tab`
  - `0640-project-cleanup`
  - `0641-chromium-patches`
  - `0642-zig-profile-server`
  - `0643-zig-profile-server-2`
  - `0644-simplified-cpp`
  - `0645-audit-xdg`
  - `0646-normal-insert`
  - `0647-tui-restructure`
  - `0648-devtools-research`
  - `0649-control-mode`
  - `0650-installation`
  - `0651-bundle-identifier`
  - `0652-termsurf-cli`
  - `0653-xpc-gateway`
  - `0654-cmd-h`
  - `0655-substack-blank`
  - `0656-rename-script`
  - `0657-url-edit-color`
  - `0658-edtui-improvements`
  - `0659-command-mode`
  - `0660-lazyvim-tokyonight-colors`
  - `0661-title-spacing`
  - `0662-context-menu`
  - `0663-js-context-menu`
  - `0664-clap`
  - `0665-esc`
  - `0666-devils-esc`
  - `0667-active-pane`
  - `0668-fix-resize`
  - `0669-active-pane`
  - `0670-click-to-focus`
  - `0671-app-icon`
  - `0672-border-padding`
  - `0673-consolidate-scripts`
  - `0674-homepage`
  - `0675-hello-message`
  - `0676-url-normalization`
  - `0677-website-deps`
  - `0678-website-lint-format`
  - `0679-license`
- The result uses the Experiment 4 row schema for every classification: source
  issue, batch, subsystem, durable lesson, current Ghostboard relevance,
  evidence paths, likelihood, risk or impact, recommended follow-up, and
  historical classification note.
- The result classifies each row as `Highly likely`, `Maybe`, or `No`, and
  explains the classification from issue evidence plus current code/test/doc
  evidence.
- The result treats all Batch C issues as closed historical evidence and does
  not modify or reinterpret their closure state.
- The result distinguishes obsolete XPC and prototype-era implementation
  mechanisms from current socket/protobuf, CALayerHost, and restored Ghostboard
  evidence.
- The result distinguishes Ghostboard GUI-owned parity findings from Roamium,
  Chromium, webtui, website, packaging, and docs-only findings.
- The result carries forward relevant Issue 810 findings where Batch C overlaps
  current Ghostboard risk, especially keyboard/mouse input, URL synchronization,
  active-pane/focus state, multi-pane and multi-profile routing, overlay
  geometry, resize behavior, context menus, mode/keybinding behavior, app
  activation, installation, and named/default browser startup.
- The result explicitly handles duplicate or recurring themes, including
  keyboard input, active pane tracking, CALayerHost audits, app icons, rename
  cleanup, and mode/keybinding work, while still classifying every issue folder
  exactly once.
- The result groups or summarizes related repeated findings after the table, but
  the table itself must still contain one row per Batch C issue.
- The result identifies the next audit slice after Batch C.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/10-batch-c-product-hardening.md
  ```

- Whitespace check passes:

  ```bash
  git diff --check
  ```

- A fresh-context completion review approves the completed result before the
  result commit.
- All real completion-review findings are fixed and recorded in this experiment
  file.
- The result commit is made after completion-review approval and before any next
  experiment is designed.

Fail criteria:

- Any Batch C issue is omitted or classified more than once.
- The experiment edits historical issue files, application code, generated code,
  scripts, tests, screenshots, website assets, or build configuration.
- The result treats obsolete XPC-era implementation details as current
  Ghostboard requirements without mapping them to the current socket/protobuf
  architecture.
- The result treats Roamium, Chromium, webtui, website, packaging, or docs-only
  behavior as a Ghostboard GUI bug without a direct current Ghostboard ownership
  path.
- The result treats older Ghostboard prototype status, rename history, or
  archived docs as proof of current restored Ghostboard behavior.

## Design Review

Mencius reviewed the design and approved it with no required findings.

The review verified that the plan is audit-only, linked from the Issue 810
README as `Designed`, covers the exact eighty-issue Batch C inventory with no
omissions or duplicates, requires the Experiment 4 row schema, has concrete
pass/fail criteria, preserves closed historical issue immutability, and requires
distinguishing obsolete XPC/prototype work and non-Ghostboard-owned areas from
current Ghostboard GUI parity risk.

## Result

**Result:** Pass

Batch C was audited as the TermSurf/Ghostboard product-hardening era. The audit
read every issue from `0600` through `0679`, mapped each durable lesson to the
current restored Ghostboard implementation, and classified current risk without
editing application code or historical issues.

### Classification Table

| Source issue                      | Batch | Subsystem                      | Durable lesson                                                                                 | Current Ghostboard relevance                                                                                                                                        | Evidence paths                                                                                                              | Likelihood      | Risk or impact                                                                                            | Recommended follow-up                                                                                          | Historical classification note                                                                            |
| --------------------------------- | ----- | ------------------------------ | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | --------------- | --------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `0600-termsurf-ghost`             | C     | Board foundation               | The board must be a real Ghostty fork with browser integration owned in the terminal app.      | Restored Ghostboard exists as a Ghostty-derived app and current TermSurf socket/overlay code lives in `ghostboard/`.                                                | Issue 600 README; `ghostboard/`; `ghostboard/src/apprt/termsurf.zig`; Issues 808/809.                                       | `No`            | Low current risk for the foundation itself.                                                               | None.                                                                                                          | Original XPC/IOSurface details are obsolete, but the board-foundation invariant is present.               |
| `0601-zig-xpc`                    | C     | IPC transport                  | The GUI needs a native IPC listener and message dispatcher.                                    | XPC is obsolete; current Ghostboard uses Unix sockets and protobuf in Zig.                                                                                          | Issue 601 README; `proto/termsurf.proto`; `ghostboard/src/apprt/termsurf.zig`.                                              | `No`            | Low current risk for transport foundation.                                                                | None.                                                                                                          | Historical XPC mechanism is superseded by socket/protobuf.                                                |
| `0602-pink-texture`               | C     | Overlay prototype              | Pane identity, grid coordinates, and cleanup must exist before live browser content.           | Current Ghostboard has pane state, overlay presentation, clear paths, and Issue 809 geometry evidence.                                                              | Issue 602 README; `ghostboard/src/apprt/termsurf.zig`; `SurfaceView_AppKit.swift`; Issue 809.                               | `No`            | Low current risk for the prototype overlay invariant.                                                     | Keep Issue 809 geometry coverage.                                                                              | Pink texture was a prototype; current CALayerHost path supersedes it.                                     |
| `0603-box-demo`                   | C     | Live browser overlay           | Browser process lifecycle, CA/IOSurface content, resize, and disconnect cleanup must work.     | Current Ghostboard opens Roamium, receives `CaContext`, presents a CALayerHost, and clears overlays.                                                                | Issue 603 README; `ghostboard/src/apprt/termsurf.zig`; `SurfaceView_AppKit.swift`; Issues 808/809.                          | `No`            | Low current risk for basic live rendering.                                                                | None beyond current geometry guard.                                                                            | FrameSink/IOSurface streaming is obsolete; live overlay invariant remains covered.                        |
| `0604-two-panes`                  | C     | Multi-pane routing             | Multiple panes need independent pane state and routing.                                        | Current state tables route by pane/profile/browser/tab and Issue 809 exercised split geometry.                                                                      | Issue 604 README; `PaneState`; `TabLookupState`; Issue 809 split rows.                                                      | `No`            | Low current risk for ordinary multi-pane routing.                                                         | Keep split-pane geometry and input rows in regression coverage.                                                | Old XPC serial queue is obsolete; durable per-pane routing is current.                                    |
| `0605-two-profiles`               | C     | Multi-profile routing          | Different profiles must map to separate browser state.                                         | Current state keys include profile/browser, but earlier Issue 810 rows still identify multi-profile DevTools/last-pane proof gaps.                                  | Issue 605 README; `findServer(profile, browser)`; Experiment 7/9 multi-profile findings.                                    | `Maybe`         | Multi-profile ordinary browsing may work, but last-pane and DevTools targeting can still be wrong.        | Fold into the multi-profile QueryLast/DevTools runtime follow-up.                                              | Historical success does not prove restored Ghostboard edge cases.                                         |
| `0606-mouse-input`                | C     | Mouse input / cursor           | Click, scroll, drag selection, focus, cursor shape, and activation gating must all work.       | Current AppKit and Zig forward mouse/scroll input; Issue 809 covers broad mouse behavior, but `CursorChanged` remains unhandled.                                    | Issue 606 README; `forwardMouseEvent`; `forwardMouseMove`; `forwardScrollEvent`; Experiment 5/8 `CursorChanged` finding.    | `Highly likely` | Browser cursor shape likely does not update even if ordinary mouse input works.                           | Prioritize the existing `CursorChanged` follow-up with hover/link/text-field coverage.                         | Classified high for cursor sync only, not for the entire mouse pipeline.                                  |
| `0607-keyboard-input`             | C     | Keyboard input                 | Browser text fields require key forwarding plus navigation lifecycle recovery.                 | Current AppKit forwards key down/up and Zig sends `KeyEvent`; runtime proof for the full historical keyboard matrix is still thin.                                  | Issue 607 README; `forwardTermSurfKeyEvent`; `forwardKeyEvent`; Roamium `Msg::KeyEvent`; Issue 804 keyboard learnings.      | `Maybe`         | Character input and shortcut coverage can regress without a dedicated keyboard matrix.                    | Add a browser keyboard regression matrix covering text, Enter, Tab, Backspace, arrows, and modifiers.          | Navigation-freeze root cause was engine-side and later solved, but the keyboard matrix remains important. |
| `0608-search-input`               | C     | Navigation lifecycle           | Form submissions and renderer swaps must keep content and input alive.                         | Current Roamium direct path owns navigation and renderer lifecycle; Ghostboard mainly hosts `CaContext` and forwards input.                                         | Issue 608 README; Roamium `Navigate`; `CaContext`; Experiment 2 direct-browser ownership rule.                              | `No`            | Low Ghostboard-owned risk; engine regression belongs in Roamium coverage.                                 | Include search-submit in browser-engine smoke, not a Ghostboard-specific fix issue.                            | Historical root cause was stale Chromium capture target, not current GUI behavior.                        |
| `0609-keyboard-input-2`           | C     | Keyboard shortcuts             | Cmd-key editing commands need special handling and Chromium editing-command support.           | Current AppKit forwards Cmd-key events through `keyDown` after `performKeyEquivalent`; Roamium forwards `KeyEvent`, but full shortcut matrix needs runtime proof.   | Issue 609 README; `performKeyEquivalent`; `forwardTermSurfKeyEvent`; Roamium `Msg::KeyEvent`.                               | `Maybe`         | Cmd+A/C/V/X/Z and Tab can silently regress.                                                               | Add the Issue 609 matrix to the durable keyboard guard.                                                        | Durable lesson remains valid under socket/protobuf.                                                       |
| `0610-app-icon`                   | C     | Branding / app icon            | App identity needs a complete icon pipeline.                                                   | Restored Ghostboard still has Ghostty macOS assets and product naming mixed with TermSurf/Ghostboard context.                                                       | Issue 610 README; `ghostboard/macos/Assets.xcassets`; `Ghostty-Info.plist`; Experiment 8 branding/build findings.           | `Maybe`         | Installed app identity may be visually or metadata-inconsistent.                                          | Audit restored Ghostboard app bundle identity, icon, and display name in the app walkthrough.                  | Branding issue, not runtime browser behavior.                                                             |
| `0611-rename`                     | C     | Branding / rename              | Fork naming must be coherent across source, bundle, docs, and scripts.                         | Current restored app intentionally still carries upstream Ghostty names in many files while TermSurf/Ghostboard names exist elsewhere.                              | Issue 611 README; `ghostboard/macos`; `ghostboard/src/config`; root scripts.                                                | `Maybe`         | User-facing names and config paths may be inconsistent.                                                   | Include naming/config path audit in the parity issue conclusion follow-up list.                                | Rename history is not proof of a runtime bug, but it is a likely audit item.                              |
| `0612-icon`                       | C     | Icon pipeline                  | Generated app icon assets must match the current product.                                      | Current icon assets are not proven against restored Ghostboard packaging.                                                                                           | Issue 612 README; `ghostboard/macos/Assets.xcassets`; Experiment 8 app icon finding.                                        | `Maybe`         | Packaging polish may be wrong.                                                                            | Verify bundle icon in the app walkthrough.                                                                     | Same branding risk as Issue 610.                                                                          |
| `0613-rename-directories`         | C     | Source layout / rename         | Directory and path names should match the active product to avoid stale scripts and docs.      | Root scripts omit Ghostboard as a build/install component and current source retains upstream naming.                                                               | Issue 613 README; `scripts/build.sh`; `scripts/install.sh`; Experiment 8 scripts finding.                                   | `Highly likely` | Developer workflows likely miss restored Ghostboard because scripts still target Wezboard/Roamium/webtui. | Create a later build-script/workflow issue for restored Ghostboard.                                            | Classified high for workflow coverage, not for app runtime.                                               |
| `0614-docs-review`                | C     | Documentation                  | Docs must match the active app and workflow.                                                   | Current root docs still identify Wezboard as active while restored Ghostboard work is underway.                                                                     | Issue 614 README; `AGENTS.md`; root `README.md`; Experiment 8 docs finding.                                                 | `Maybe`         | Developers may follow stale build/test paths.                                                             | Add restored-Ghostboard docs cleanup to the follow-up list.                                                    | Docs-only risk.                                                                                           |
| `0615-xdg`                        | C     | Config/state paths             | Config, data, state, and cache paths must be separated and branded correctly.                  | Current Ghostboard config code says default config is `$XDG_CONFIG_HOME/termsurf/config`, while user workflow has also used restored app-specific config locations. | Issue 615 README; `ghostboard/src/config/Config.zig`; `Ghostty.Config.swift`; user-provided config-path context.            | `Maybe`         | Config parity may fail if Ghostboard reads a different path than expected.                                | Explicitly test config discovery and all TermSurf-specific config options in the app walkthrough/config audit. | Historical XDG lesson remains directly relevant.                                                          |
| `0616-web-features`               | C     | Browser feature set            | Browser UX includes loading state, navigation shortcuts, URL sync, mode changes, and chrome.   | webtui and Roamium own much of this via direct browser IPC; Ghostboard fallback dispatch is partial.                                                                | Issue 616 README; Experiment 2 browser chrome/status `Maybe`; webtui direct `BrowserConnection`.                            | `Maybe`         | Some browser chrome features may depend on the direct path and lack fallback or regression proof.         | Reuse Experiment 3 direct-browser follow-ups for URL/loading/title/target/console/status behavior.             | Broad feature umbrella; specific risks are tracked in protocol rows.                                      |
| `0617-alpha`                      | C     | Release readiness              | Alpha readiness requires a walkthrough, not only source presence.                              | Current Issue 810 is exactly discovering gaps before app usage; not all Batch C user workflows have runtime proof.                                                  | Issue 617 README; Issue 810 acceptance criteria; Issues 808/809.                                                            | `Maybe`         | Unwalked features may surprise users.                                                                     | Keep final Issue 810 conclusion focused on a later app walkthrough and targeted fix issue.                     | Process finding, not a single defect.                                                                     |
| `0618-url-sync`                   | C     | URL synchronization            | Browser navigation must update the TUI URL bar.                                                | Direct Roamium-to-webtui path handles `UrlChanged`; Issue 809 observed `UrlChanged` after navigation.                                                               | Issue 618 README; `webtui/src/ipc.rs`; `scripts/ghostboard-geometry-matrix.sh`; Experiment 2/3.                             | `No`            | Low current risk for the normal direct URL sync path.                                                     | Keep navigation URL-sync smoke in the direct-browser guard.                                                    | Ghostboard relay fallback remains separate from normal direct path.                                       |
| `0619-input-latency`              | C     | Input latency                  | Browser input should feel native and should not add avoidable relay cost.                      | Current path forwards AppKit input to Ghostboard and then browser, while post-ready browser chrome is direct; no latency benchmark exists.                          | Issue 619 README; `SurfaceView_AppKit.swift`; `forwardKeyEvent`; `forwardMouseEvent`.                                       | `Maybe`         | Input may work but still feel slow under load or VM conditions.                                           | Add lightweight latency/perceived-responsiveness checks only after functional parity is proven.                | Performance risk, not proven functional failure.                                                          |
| `0620-zig-content-shell`          | C     | Chromium embedding research    | Embedding experiments should not displace the working browser engine path.                     | Current Roamium/libtermsurf_chromium path supersedes old content-shell experiments.                                                                                 | Issue 620 README; `roamium/src`; `chromium/src/content/libtermsurf_chromium`.                                               | `No`            | Low Ghostboard GUI risk.                                                                                  | None.                                                                                                          | Browser-engine research, not current GUI parity.                                                          |
| `0621-single-process`             | C     | Browser process model          | Process model choices affect stability and profile isolation.                                  | Current architecture is one browser process per profile; Ghostboard tracks servers by profile/browser.                                                              | Issue 621 README; `ServerState`; `findServer`; AGENTS architecture.                                                         | `No`            | Low current risk for the high-level model.                                                                | Keep multi-profile runtime proof as a separate follow-up.                                                      | Historical implementation details superseded.                                                             |
| `0622-javascript-is-slow`         | C     | Engine performance             | Avoid unnecessary JavaScript control paths for browser internals.                              | Current Roamium uses Rust/C FFI and protobuf, not JavaScript injection for core control.                                                                            | Issue 622 README; `roamium/src/dispatch.rs`; `chromium/src/content/libtermsurf_chromium`.                                   | `No`            | Low Ghostboard risk.                                                                                      | None.                                                                                                          | Engine lesson, not GUI-owned.                                                                             |
| `0623-viz-display-serialization`  | C     | Chromium rendering internals   | Rendering transport should avoid fragile serialized compositor state.                          | Current CALayerHost/CAContext path supersedes this research.                                                                                                        | Issue 623 README; `CaContext`; `SurfaceView_AppKit.swift`; Issue 809.                                                       | `No`            | Low current risk.                                                                                         | None.                                                                                                          | Obsolete rendering experiment.                                                                            |
| `0624-chromium-ipc`               | C     | Browser IPC                    | Chromium IPC should have a stable, narrow integration boundary.                                | Current boundary is Roamium/libtermsurf_chromium plus protobuf sockets; Ghostboard is only the host.                                                                | Issue 624 README; `roamium/src`; `proto/termsurf.proto`.                                                                    | `No`            | Low Ghostboard-owned risk.                                                                                | None.                                                                                                          | Browser-engine issue, not current GUI parity.                                                             |
| `0625-calayerhost`                | C     | CALayerHost rendering          | Zero-copy remote layers need correct layer creation, scale, and cleanup.                       | Current AppKit has a CALayerHost bridge and Issue 809 proved broad geometry.                                                                                        | Issue 625 README; `termsurf_present_overlay`; `SurfaceView_AppKit.swift`; Issue 809.                                        | `No`            | Low current risk for basic CALayerHost presentation.                                                      | Keep Issue 809 geometry matrix.                                                                                | Later rows cover remaining CALayerHost edge cases.                                                        |
| `0626-x-y-calayerhost`            | C     | Overlay positioning            | Coordinate systems need a flipped layer and top-origin positioning.                            | Current AppKit uses root/positioning/host layer state and Issue 809 verified geometry across panes and resize.                                                      | Issue 626 README; `termsurfOverlayPositioningLayer`; `termSurfOverlayPoint`; Issue 809.                                     | `No`            | Low current risk for ordinary positioning.                                                                | Keep geometry guard.                                                                                           | Pixel-perfect positioning has strong current evidence.                                                    |
| `0627-resize-calayerhost`         | C     | Overlay resize                 | Pane/window resize must update both host layer and browser view size.                          | Current Ghostboard has AppKit presented-pixel corrective resize plus Issue 809 resize/fullscreen evidence.                                                          | Issue 627 README; `overlayPresentedPixels`; `sendResize`; `scripts/ghostboard-geometry-matrix.sh`.                          | `No`            | Low current risk for resize path.                                                                         | Keep resize rows in the matrix.                                                                                | Current socket/CALayerHost path supersedes XPC resize handler.                                            |
| `0628-navigation-calayerhost`     | C     | Navigation rendering           | Navigation must not blank the remote CALayerHost.                                              | Later issues fixed the historical blank, but current restored Ghostboard has limited visual flicker proof beyond geometry/log assertions.                           | Issue 628 README; Issues 630-633; Issue 809 navigation row.                                                                 | `Maybe`         | Brief navigation flicker or blanking could remain visually despite protocol success.                      | Include real visual navigation smoke in the app walkthrough.                                                   | Historical issue was unresolved at the time but superseded by later fixes.                                |
| `0629-understand-nav-calayerhost` | C     | Navigation diagnosis           | Hidden windows and view swaps affect CAContext lifecycle.                                      | Current Roamium/libtermsurf owns Chromium window/compositor details; Ghostboard should verify visual behavior, not reapply old XPC fixes.                           | Issue 629 README; Roamium/libtermsurf source; Issues 630-633.                                                               | `No`            | Low direct Ghostboard GUI risk.                                                                           | Covered by the navigation visual smoke above.                                                                  | Research issue led to later fixes.                                                                        |
| `0630-nav-calayerhost-6`          | C     | Navigation blank fix           | CALayer mutations must be main-thread safe and view swaps must re-register callbacks.          | Current AppKit presents layers on the app side; Roamium owns view-swap callbacks. Visual flicker remains an unproven edge.                                          | Issue 630 README; `SurfaceView_AppKit.swift`; Roamium/libtermsurf.                                                          | `Maybe`         | Navigation may still have cosmetic flicker or callback lifecycle edge cases.                              | Same visual navigation follow-up as Issue 628.                                                                 | Historical fix is partially engine-owned and partially GUI-layer discipline.                              |
| `0631-continue-nav-calayerhost`   | C     | Navigation flicker research    | GUI-side CALayer tricks cannot fix a dead old CAContext.                                       | Current audit should not reopen old failed GUI approaches without fresh evidence.                                                                                   | Issue 631 README; Issue 633 conclusion.                                                                                     | `No`            | Low current risk beyond the broader navigation visual smoke.                                              | None separate.                                                                                                 | Research narrowed the true fix path.                                                                      |
| `0632-nav-flicker-calayerhost`    | C     | Persistent compositor research | Stable CAContext requires Chrome-like parent-layer compositor behavior.                        | Current engine path is Roamium/libtermsurf; Ghostboard simply hosts CAContext IDs.                                                                                  | Issue 632 README; Issue 633 README; Roamium/libtermsurf.                                                                    | `No`            | Low Ghostboard-owned risk.                                                                                | None separate.                                                                                                 | Browser-engine architecture lesson.                                                                       |
| `0633-persistent-compositor`      | C     | Persistent compositor          | One stable compositor per tab avoids navigation flicker.                                       | Per-tab CAContext ownership is engine-side; Ghostboard maps tab IDs to pane overlays.                                                                               | Issue 633 README; `handleCaContext`; `TabLookupState`.                                                                      | `No`            | Low GUI risk if engine emits correct CAContext.                                                           | Keep multi-pane and navigation visual smoke.                                                                   | Later per-tab issue covers multi-pane isolation.                                                          |
| `0634-calayerhost-audit`          | C     | Feature audit                  | After rendering migration, every browser feature needs a retest.                               | Issue 809 covers geometry/input slices, but not every historical feature from the CALayerHost audit.                                                                | Issue 634 README; Issue 809; Experiments 7-9.                                                                               | `Maybe`         | Untested browser edge cases may remain even though geometry is strong.                                    | Use this row to justify the later app walkthrough.                                                             | Process/coverage finding.                                                                                 |
| `0635-multi-pane-calayerhost`     | C     | Multi-tab isolation            | A persistent compositor must be per tab, not shared across all tabs.                           | Current Ghostboard routes by browser tab ID and pane; Issue 809 exercised multi-pane geometry.                                                                      | Issue 635 README; `findTabLookup`; `upsertTabLookup`; Issue 809 split rows.                                                 | `No`            | Low current risk for the GUI mapping model.                                                               | Keep multi-pane/multi-profile runtime rows.                                                                    | Engine compositor implementation is outside Ghostboard.                                                   |
| `0636-calayerhost-audit`          | C     | Feature audit resumed          | Full feature audit should pass after multi-pane fix.                                           | Current restored Ghostboard has not repeated all twenty historical audit items.                                                                                     | Issue 636 README; Issue 809; Experiment 10 rows.                                                                            | `Maybe`         | Some historical feature checks remain unwalked.                                                           | Fold remaining items into the final app walkthrough checklist.                                                 | Coverage finding, not a direct defect.                                                                    |
| `0637-editable-url-bar`           | C     | webtui URL editing             | URL editing belongs in the TUI and sends navigation requests.                                  | Current webtui uses edtui and direct browser navigation; Ghostboard only provides the socket/bootstrap path.                                                        | Issue 637 README; `webtui/src/main.rs`; `webtui/src/ipc.rs`.                                                                | `No`            | Low Ghostboard-owned risk.                                                                                | None.                                                                                                          | TUI-owned feature.                                                                                        |
| `0638-page-title`                 | C     | Page title                     | Page title should display in the TUI viewport.                                                 | Direct browser path supports `TitleChanged`, but this specific display path lacks a current Ghostboard visual assertion.                                            | Issue 638 README; `webtui/src/ipc.rs`; Experiment 2 browser chrome/status `Maybe`.                                          | `Maybe`         | Title display may regress unnoticed.                                                                      | Include page-title assertion in direct-browser chrome smoke.                                                   | Mostly TUI/direct-browser, but visible in Ghostboard usage.                                               |
| `0639-open-in-same-tab`           | C     | New-window handling            | `target=_blank` and `window.open()` should remain usable before full tab UI exists.            | This is Roamium/libtermsurf browser behavior, not Ghostboard GUI routing.                                                                                           | Issue 639 README; Roamium/libtermsurf ownership.                                                                            | `No`            | Low Ghostboard-owned risk.                                                                                | Cover in browser-engine feature smoke if needed.                                                               | Engine-owned feature.                                                                                     |
| `0640-project-cleanup`            | C     | Project cleanup                | Active docs and directories should not point users at obsolete prototypes.                     | Current repo still presents Wezboard as active while Ghostboard restoration is in progress.                                                                         | Issue 640 README; `AGENTS.md`; root `README.md`; Experiment 8 docs finding.                                                 | `Maybe`         | Contributors may follow stale workflows.                                                                  | Include docs cleanup in final follow-up list.                                                                  | Docs/workflow risk only.                                                                                  |
| `0641-chromium-patches`           | C     | Chromium patch archive         | Chromium changes must be portable as patches.                                                  | Current Chromium patch/workspace process is governed by the Chromium skill and README; not a Ghostboard GUI parity item.                                            | Issue 641 README; `chromium/README.md`; `chromium/patches`.                                                                 | `No`            | Low Ghostboard risk.                                                                                      | None.                                                                                                          | Chromium workflow issue.                                                                                  |
| `0642-zig-profile-server`         | C     | Browser engine architecture    | Rewriting the profile server in Zig was not worth destabilizing the working path.              | Current Roamium/libtermsurf path supersedes the failed Zig profile-server attempt.                                                                                  | Issue 642 README; `roamium/src`; `chromium/src/content/libtermsurf_chromium`.                                               | `No`            | Low Ghostboard GUI risk.                                                                                  | None.                                                                                                          | Historical dead end.                                                                                      |
| `0643-zig-profile-server-2`       | C     | Browser engine architecture    | Build-system success does not imply end-to-end browser integration.                            | Same as Issue 642; current browser process is Roamium.                                                                                                              | Issue 643 README; Roamium/libtermsurf.                                                                                      | `No`            | Low Ghostboard risk.                                                                                      | None.                                                                                                          | Historical dead end.                                                                                      |
| `0644-simplified-cpp`             | C     | Browser engine simplification  | Infrastructure cleanup should yield to user-facing feature work when the current engine works. | Current browser engine work is outside restored Ghostboard GUI parity.                                                                                              | Issue 644 README; Roamium/libtermsurf.                                                                                      | `No`            | Low Ghostboard risk.                                                                                      | None.                                                                                                          | Engine cleanup issue.                                                                                     |
| `0645-audit-xdg`                  | C     | XDG paths                      | Config/state/cache/data paths must be correct and branded.                                     | Current Ghostboard config claims `$XDG_CONFIG_HOME/termsurf/config`; expected restored app config locations need proof.                                             | Issue 645 README; `ghostboard/src/config/Config.zig`; `Ghostty.Config.swift`.                                               | `Maybe`         | User config may load from an unexpected location.                                                         | Add explicit config-path verification to the config audit.                                                     | Same durable risk as Issue 615.                                                                           |
| `0646-normal-insert`              | C     | TUI modes / Esc                | TUI mode transitions should be precise and visible.                                            | Current webtui has Browser/Control/Edit/Command modes and receives `ModeChanged`; not Ghostboard-owned except mode signal.                                          | Issue 646 README; `webtui/src/main.rs`; `handleModeChanged`; `sendFocusChanged`.                                            | `No`            | Low Ghostboard-owned risk.                                                                                | Keep mode-change smoke in keyboard/input guard.                                                                | TUI-owned behavior with small GUI focus dependency.                                                       |
| `0647-tui-restructure`            | C     | TUI layout                     | Browser content, URL bar, title, and profile layout should be ergonomic.                       | Current webtui layout is TUI-owned; Ghostboard only supplies viewport geometry.                                                                                     | Issue 647 README; `webtui/src/main.rs`; Issue 809 geometry.                                                                 | `No`            | Low Ghostboard-owned risk.                                                                                | None.                                                                                                          | TUI-owned feature.                                                                                        |
| `0648-devtools-research`          | C     | DevTools UX                    | DevTools should open in a terminal split with reliable target discovery.                       | Current DevTools split works in Issue 809, but duplicate guard and multi-profile target proof remain open.                                                          | Issue 648 README; Experiment 7/9 DevTools findings; Issue 809 DevTools rows.                                                | `Maybe`         | DevTools can target incorrectly or duplicate in edge cases.                                               | Combine with one-DevTools-per-tab and multi-profile DevTools follow-ups.                                       | Research finding reinforced by later concrete rows.                                                       |
| `0649-control-mode`               | C     | TUI startup mode               | Browser TUI should start in control mode, not immediately capture all keys.                    | Current webtui initializes `Mode::Control`; Ghostboard forwards mode changes from TUI.                                                                              | Issue 649 README; `webtui/src/main.rs`; `handleModeChanged`.                                                                | `No`            | Low current risk.                                                                                         | None.                                                                                                          | TUI-owned behavior.                                                                                       |
| `0650-installation`               | C     | Installation/runtime paths     | The app must run outside the source tree with bundled browser and TUI paths.                   | Root build/install scripts omit Ghostboard and named browser launch is not implemented.                                                                             | Issue 650 README; `scripts/build.sh`; `scripts/install.sh`; `handleSetOverlay`; Experiment 8/9 browser-resolution findings. | `Highly likely` | Restored Ghostboard likely cannot be built/installed/launched through the standard root scripts.          | Create a build/install workflow issue for restored Ghostboard and browser discovery.                           | Classified high for current workflow and launch resolution gaps.                                          |
| `0651-bundle-identifier`          | C     | macOS bundle identity          | Debug and release apps need distinct bundle identity to avoid Launch Services confusion.       | Current restored Ghostboard bundle identity has not been audited after restoration.                                                                                 | Issue 651 README; `ghostboard/macos/Ghostty-Info.plist`; root scripts.                                                      | `Maybe`         | macOS may launch the wrong build or show stale identity.                                                  | Include bundle ID/display-name/debug-vs-release checks in packaging follow-up.                                 | Packaging risk.                                                                                           |
| `0652-termsurf-cli`               | C     | CLI binary naming              | Bundle CLI names and app gateway names must match the product.                                 | Current root scripts do not install Ghostboard; app still carries Ghostty-oriented bundle files.                                                                    | Issue 652 README; `ghostboard/macos`; `scripts/install.sh`.                                                                 | `Maybe`         | CLI/app naming may be inconsistent.                                                                       | Packaging workflow audit.                                                                                      | XPC gateway conflict is obsolete; naming invariant remains.                                               |
| `0653-xpc-gateway`                | C     | App/TUI discovery              | Debug/release IPC discovery must not collide.                                                  | XPC gateway is obsolete; current discovery is `TERMSURF_SOCKET` plus Unix socket listener.                                                                          | Issue 653 README; `env_key = "TERMSURF_SOCKET"`; socket code.                                                               | `No`            | Low current risk for the old gateway collision.                                                           | None.                                                                                                          | Transport superseded by socket/protobuf.                                                                  |
| `0654-cmd-h`                      | C     | macOS keybindings              | User keybindings should beat menu shortcuts unless unbound.                                    | Current `performKeyEquivalent` checks bindings before menu dispatch and AppKit key forwarding exists.                                                               | Issue 654 README; `SurfaceView_AppKit.swift` `performKeyEquivalent`.                                                        | `No`            | Low current risk for the historical Cmd+H bug.                                                            | Include Cmd-key browser and terminal shortcuts in keyboard smoke.                                              | Upstream Ghostty-style behavior appears present.                                                          |
| `0655-substack-blank`             | C     | Browser API binders            | Missing browser interface binders can kill renderers on real sites.                            | Engine-owned Roamium/libtermsurf issue; Ghostboard only hosts the tab and crash/status UI.                                                                          | Issue 655 README; Roamium/libtermsurf; Experiment 2 renderer-crash `Maybe`.                                                 | `No`            | Low Ghostboard-owned risk.                                                                                | Keep renderer-crash smoke in browser-engine/direct-browser coverage.                                           | Engine-owned feature.                                                                                     |
| `0656-rename-script`              | C     | Rename tooling                 | Rename scripts must be re-runnable after upstream merges.                                      | Current root scripts include Wezboard rename tooling but no analogous restored Ghostboard workflow.                                                                 | Issue 656 README; `scripts/rename-wezterm.sh`; `scripts/build.sh`.                                                          | `Maybe`         | Future upstream merges may leave names inconsistent.                                                      | Track under restored-Ghostboard workflow/docs cleanup.                                                         | Workflow risk only.                                                                                       |
| `0657-url-edit-color`             | C     | TUI visual state               | URL edit mode should be visibly distinct.                                                      | Current webtui owns mode colors and styling; not Ghostboard GUI.                                                                                                    | Issue 657 README; `webtui/src/main.rs`.                                                                                     | `No`            | Low Ghostboard risk.                                                                                      | None.                                                                                                          | TUI-owned feature.                                                                                        |
| `0658-edtui-improvements`         | C     | TUI editor UX                  | URL/command editor submodes, clipboard, and indicators need stable behavior.                   | Current webtui contains edtui mode handling and clean clipboard wrapper.                                                                                            | Issue 658 README; `webtui/src/main.rs`.                                                                                     | `No`            | Low Ghostboard-owned risk.                                                                                | None beyond app walkthrough of visible TUI flows.                                                              | TUI-owned feature.                                                                                        |
| `0659-command-mode`               | C     | TUI command mode               | Multi-character commands need an isolated command editor and dispatch.                         | Current webtui has `Mode::Command`, command dispatch, and `:quit`, `:dark`, `:viewport`, `:devtools` command handling.                                              | Issue 659 README; `webtui/src/main.rs`.                                                                                     | `No`            | Low Ghostboard-owned risk.                                                                                | None.                                                                                                          | TUI-owned feature.                                                                                        |
| `0660-lazyvim-tokyonight-colors`  | C     | TUI colors                     | Mode colors should be recognizable and consistent.                                             | Current webtui defines Tokyo Night palette and submode colors.                                                                                                      | Issue 660 README; `webtui/src/main.rs`.                                                                                     | `No`            | Low Ghostboard risk.                                                                                      | None.                                                                                                          | TUI-owned visual feature.                                                                                 |
| `0661-title-spacing`              | C     | TUI polish                     | UI text spacing should be tight and intentional.                                               | TUI-owned layout/polish; Ghostboard only hosts the terminal.                                                                                                        | Issue 661 README; `webtui/src/main.rs`.                                                                                     | `No`            | Low Ghostboard risk.                                                                                      | None.                                                                                                          | TUI-owned feature.                                                                                        |
| `0662-context-menu`               | C     | Browser context menu           | Browser right-click menus require deliberate AppKit/TUI/browser ownership.                     | No current protocol or Ghostboard evidence proves a browser-specific context menu; ordinary right-click forwarding/menu behavior is ambiguous.                      | Issue 662 README; `SurfaceView_AppKit.swift`; `Surface.zig`; protocol has no context-menu message.                          | `Maybe`         | Users may lack browser Back/Forward/Reload context menu behavior.                                         | Test right-click over browser content and decide whether browser context menus are in current parity scope.    | Historical issue was deferred, so this is a feature gap candidate, not proven regression.                 |
| `0663-js-context-menu`            | C     | Browser context menu           | JavaScript-injected menus were a possible low-friction context-menu path.                      | Same context-menu uncertainty as Issue 662; implementation would be engine-owned if chosen.                                                                         | Issue 663 README; Roamium/libtermsurf; protocol.                                                                            | `Maybe`         | Browser context menu may be absent.                                                                       | Same context-menu follow-up as Issue 662.                                                                      | Deferred browser-engine approach.                                                                         |
| `0664-clap`                       | C     | web CLI parsing                | CLI arguments should use structured parsing and preserve compatibility.                        | Current webtui uses clap with subcommands and global flags.                                                                                                         | Issue 664 README; `webtui/src/main.rs`.                                                                                     | `No`            | Low Ghostboard risk.                                                                                      | None.                                                                                                          | TUI-owned CLI feature.                                                                                    |
| `0665-esc`                        | C     | Escape key / mode exit         | Esc should exit browse mode without breaking editor Esc behavior.                              | Current AppKit deliberately does not forward Esc to browser and webtui unified mode handling exists; runtime keyboard proof remains broader `Maybe`.                | Issue 665 README; `forwardTermSurfKeyEvent` Esc guard; `webtui/src/main.rs`.                                                | `No`            | Low specific Esc-path risk.                                                                               | Cover Esc in keyboard regression matrix.                                                                       | Specific Esc design appears present.                                                                      |
| `0666-devils-esc`                 | C     | Event loop latency             | XPC/socket messages and terminal events must wake the TUI without polling latency.             | Current webtui uses unified `LoopEvent` channel for terminal and IPC messages.                                                                                      | Issue 666 README; `LoopEvent`; webtui reader thread; `CompositorMessage::ModeChanged`.                                      | `No`            | Low current risk for the old 250ms polling bug.                                                           | None.                                                                                                          | Transport changed, but unified event-loop lesson survived.                                                |
| `0667-active-pane`                | C     | Active pane indicator          | Focus indicators must not be built on a broken resize baseline.                                | Current upstream Ghostty focus dimming exists; custom TermSurf split borders are not proven but not core browser parity.                                            | Issue 667 README; `SurfaceView.swift`; Issue 668/669.                                                                       | `Maybe`         | Active-pane visual parity may be incomplete if custom border configs are expected.                        | Include pane focus indicator in app walkthrough; avoid treating it as browser blocker.                         | Historical issue closed after discovering unrelated resize breakage.                                      |
| `0668-fix-resize`                 | C     | TUI resize                     | Crossterm reader must forward resize events, not only keys.                                    | Current webtui forwards `Event::Resize` through `LoopEvent::Terminal`.                                                                                              | Issue 668 README; `webtui/src/main.rs` reader thread.                                                                       | `No`            | Low current risk for the exact TUI resize bug.                                                            | None.                                                                                                          | TUI-owned fix present.                                                                                    |
| `0669-active-pane`                | C     | Active pane indicator          | Pane borders/desaturation should default off and not break resize.                             | Current Ghostboard has upstream unfocused split overlay behavior, but TermSurf-specific custom border options are not proven.                                       | Issue 669 README; `SurfaceView.swift`; `Ghostty.Config.swift`.                                                              | `Maybe`         | Visual active-pane parity may differ from historical TermSurf GUI.                                        | Verify focus/border/desaturation expectations in app walkthrough.                                              | Visual polish gap candidate.                                                                              |
| `0670-click-to-focus`             | C     | Click activation               | Click-to-focus should not pass the activation click through unexpectedly.                      | Issue 809 and AppKit forwarding provide broad click/focus evidence, but browser text-selection specifics remain separate.                                           | Issue 670 README; Issue 809; `forwardTermSurfMouseEvent`.                                                                   | `No`            | Low incremental risk for ordinary click-to-focus.                                                         | Keep text-selection and drag coverage in keyboard/mouse guard.                                                 | Later click-suppression rows already track browser-specific edge cases.                                   |
| `0671-app-icon`                   | C     | Icon scripts                   | Icon regeneration and clean build scripts should be reliable.                                  | Current root scripts omit Ghostboard; icon pipeline for restored app is unverified.                                                                                 | Issue 671 README; `scripts/build.sh`; `ghostboard/macos/Assets.xcassets`.                                                   | `Maybe`         | App icon updates may be manual or stale.                                                                  | Packaging/build workflow follow-up.                                                                            | Packaging/tooling risk.                                                                                   |
| `0672-border-padding`             | C     | Pane border geometry           | Decorative borders must not steal cells or misalign overlays.                                  | Issue 809 strongly covers overlay geometry; custom border padding status is unclear.                                                                                | Issue 672 README; Issue 809; `SurfaceView.swift`.                                                                           | `Maybe`         | If custom borders are reintroduced, overlays could misalign.                                              | Keep border/padding checks in visual walkthrough, but do not duplicate Issue 809 geometry proof.               | Visual polish risk only.                                                                                  |
| `0673-consolidate-scripts`        | C     | Scripts                        | Build/install scripts should be centralized and cover active components.                       | Scripts are centralized but omit Ghostboard as a component.                                                                                                         | Issue 673 README; `scripts/build.sh`; `scripts/install.sh`; Experiment 8 scripts finding.                                   | `Highly likely` | Restored Ghostboard is outside the standard component build/install workflow.                             | Same restored-Ghostboard build/install workflow issue as Issues 613/650.                                       | Classified high because current scripts make the gap visible.                                             |
| `0674-homepage`                   | C     | Homepage config                | `web` should open a configurable homepage when no URL is supplied.                             | Current webtui supports hello homepage fallback, but Ghostboard `HelloReply` initializes no homepage field.                                                         | Issue 674 README; `sendHelloReply`; `webtui/src/main.rs`.                                                                   | `Highly likely` | Configured homepage likely cannot flow from restored Ghostboard to webtui.                                | Add homepage/config option verification and implement/fix in a later issue if confirmed.                       | Current code evidence shows an empty reply path.                                                          |
| `0675-hello-message`              | C     | Live config discovery          | `web` should get live GUI config such as homepage and browser list from hello.                 | Ghostboard handles `HelloRequest` but sends an empty initialized `HelloReply`; browser list is not populated.                                                       | Issue 675 README; `sendHelloReply`; `webtui/src/ipc.rs`; `webtui/src/main.rs`.                                              | `Highly likely` | Default browser list and homepage discovery likely do not work as intended.                               | Prioritize with the named/default browser launch and config discovery follow-up.                               | Strong current source evidence.                                                                           |
| `0676-url-normalization`          | C     | URL resolution                 | `web google.com` and localhost inputs should normalize correctly.                              | Current webtui has `resolve_input` and file/URL handling; not Ghostboard-owned.                                                                                     | Issue 676 README; `webtui/src/main.rs`; Experiment 9 smart URL row.                                                         | `No`            | Low Ghostboard risk.                                                                                      | None.                                                                                                          | TUI-owned feature.                                                                                        |
| `0677-website-deps`               | C     | Website dependencies           | Website dependencies should stay current.                                                      | Website-only; no Ghostboard parity impact.                                                                                                                          | Issue 677 README; `website/`.                                                                                               | `No`            | No Ghostboard risk.                                                                                       | None.                                                                                                          | Unrelated subsystem.                                                                                      |
| `0678-website-lint-format`        | C     | Website lint/format            | Website should have lint/format checks.                                                        | Website-only; no Ghostboard parity impact.                                                                                                                          | Issue 678 README; `website/`.                                                                                               | `No`            | No Ghostboard risk.                                                                                       | None.                                                                                                          | Unrelated subsystem.                                                                                      |
| `0679-license`                    | C     | License/trademark              | Code and brand must have clear license and trademark terms.                                    | Repo-level licensing is not a Ghostboard runtime parity issue.                                                                                                      | Issue 679 README; `LICENSE`; `TRADEMARKS.md`; `NOTICE`.                                                                     | `No`            | Low Ghostboard runtime risk.                                                                              | None from this audit.                                                                                          | Legal/project metadata, not app behavior.                                                                 |

### Ranked Findings

`Highly likely`:

- Browser cursor shape updates remain likely missing under restored Ghostboard
  because `CursorChanged` is still not handled.
- Restored Ghostboard build/install workflow coverage is likely incomplete: root
  scripts omit Ghostboard, and named/default browser launch is still not
  implemented.
- GUI hello/config discovery is likely incomplete: `HelloReply` is sent without
  homepage or browser-list fields.

`Maybe`:

- Full browser keyboard parity needs a durable matrix covering text, shortcuts,
  Tab, Enter, arrows, Backspace, and Esc.
- Multi-profile and DevTools targeting need runtime proof beyond ordinary
  single-profile browsing.
- Navigation visual smoothness, page title display, browser feature chrome,
  context menus, active-pane visual polish, config paths, app identity, and
  packaging remain plausible gaps for the later walkthrough.
- Some findings are deliberately scoped as coverage/process risks, not proven
  bugs: release readiness, broad CALayerHost feature audit coverage, and stale
  docs/workflow.

`No`:

- Core socket/protobuf transport, basic live overlay rendering, ordinary
  multi-pane geometry, resize, URL normalization, TUI command/editor behavior,
  and most browser-engine research rows do not currently map to Ghostboard-owned
  parity bugs.
- Website and license issues are unrelated to Ghostboard runtime parity.

### Verification

Commands run:

```bash
for d in issues/0600-* issues/0601-* issues/0602-* issues/0603-* issues/0604-* issues/0605-* issues/0606-* issues/0607-* issues/0608-* issues/0609-* issues/0610-* issues/0611-* issues/0612-* issues/0613-* issues/0614-* issues/0615-* issues/0616-* issues/0617-* issues/0618-* issues/0619-* issues/0620-* issues/0621-* issues/0622-* issues/0623-* issues/0624-* issues/0625-* issues/0626-* issues/0627-* issues/0628-* issues/0629-* issues/0630-* issues/0631-* issues/0632-* issues/0633-* issues/0634-* issues/0635-* issues/0636-* issues/0637-* issues/0638-* issues/0639-*; do
  sed -n '/^# /p;/^## Goal/,+8p;/^## Conclusion/,$p' "$d/README.md" | sed -n '1,120p'
done

for d in issues/0640-* issues/0641-* issues/0642-* issues/0643-* issues/0644-* issues/0645-* issues/0646-* issues/0647-* issues/0648-* issues/0649-* issues/0650-* issues/0651-* issues/0652-* issues/0653-* issues/0654-* issues/0655-* issues/0656-* issues/0657-* issues/0658-* issues/0659-* issues/0660-* issues/0661-* issues/0662-* issues/0663-* issues/0664-* issues/0665-* issues/0666-* issues/0667-* issues/0668-* issues/0669-* issues/0670-* issues/0671-* issues/0672-* issues/0673-* issues/0674-* issues/0675-* issues/0676-* issues/0677-* issues/0678-* issues/0679-*; do
  sed -n '/^# /p;/^## Goal/,+8p;/^## Conclusion/,$p' "$d/README.md" | sed -n '1,120p'
done

rg -n "sendMouse|Mouse|mouse|sendKey|KeyEvent|keyTo|ForwardKeyboard|FocusChanged|focus|CursorChanged|cursor|SetOverlay|SetDevtoolsOverlay|CaContext|Resize|resize|HelloReply|homepage|browsers|isAbsolutePath|named browser|QueryLast|QueryDevtools|ModeChanged|SetGuiActive|Context|menu|right" \
  ghostboard/src/apprt/termsurf.zig \
  "ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift" \
  "ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView.swift" \
  webtui/src/main.rs webtui/src/ipc.rs roamium/src/dispatch.rs \
  proto/termsurf.proto scripts/build.sh scripts/install.sh scripts/uninstall.sh

rg -n "homepage|HelloReply|browsers|default_browser|roamium|config" \
  ghostboard/src/apprt/termsurf.zig ghostboard/src/config \
  ghostboard/macos/Sources/Ghostty/Ghostty.Config.swift

prettier --write --prose-wrap always --print-width 80 \
  issues/0810-ghostboard-preventive-parity-audit/README.md \
  issues/0810-ghostboard-preventive-parity-audit/10-batch-c-product-hardening.md

git diff --check
```

Verification results:

- All eighty Batch C issues are represented exactly once in the classification
  table.
- Every row uses the Experiment 4 schema.
- All Batch C issues are treated as closed historical evidence.
- No historical issue files, application code, generated code, scripts, tests,
  screenshots, website assets, or build configuration were edited.
- The result distinguishes obsolete XPC/prototype mechanisms from current
  socket/protobuf and direct-browser evidence.
- The result distinguishes Ghostboard GUI ownership from Roamium, Chromium,
  webtui, website, packaging, docs, and workflow ownership.

## Conclusion

Batch C reinforces three current high-confidence follow-up areas:

1. `CursorChanged` remains the clearest browser UX gap from the historical mouse
   input work.
2. Restored Ghostboard still needs first-class build/install/browser-discovery
   workflow support.
3. `HelloReply` should be audited and likely fixed so homepage and browser-list
   configuration reach `web`.

It also adds several important walkthrough items: full keyboard form/shortcut
parity, navigation visual smoothness, page-title/chrome state, browser context
menus, active-pane visual polish, config-path behavior, and macOS bundle
identity.

The next audit slice should move backward to Batch B (`0400`-`0515`), because
Batch C references the ts5 and late prototype work as its baseline and those
issues likely contain the earlier input, focus, compositing, and protocol
lessons that shaped the Ghostboard port.

## Completion Review

Volta reviewed the completed result and approved it with no required findings.

The review verified that the Batch C table contains exactly eighty rows, one
each for `0600` through `0679`; the rows use the Experiment 4 schema; the README
marks Experiment 10 as `Pass`; the diff is audit-only; all Batch C historical
issues remain closed and unedited; and the contentious classifications are
defensible, including cursor sync, build/install workflow, empty `HelloReply`,
keyboard coverage, CALayerHost/navigation visual risk, and context-menu
ownership.
