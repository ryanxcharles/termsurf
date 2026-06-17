# Experiment 8: Batch E Wezboard Implementation Audit

## Description

Classify Batch E from Experiment 4: issues `0715`-`0742`. This batch covers the
initial Wezboard implementation era and the Ghostboard archive transition:
WezTerm fork setup, build warnings, Cocoa/objc2 migration, wgpu/dependency
updates, split pane borders, TermSurf protocol implementation, CALayerHost
overlay rendering and lifecycle, multi-webview behavior, remaining protocol
coverage, Roamium install/process lifecycle, build scripts, branding, text
selection, split protocol, and the decision to archive Ghostboard.

This experiment should read every Batch E issue and map each durable lesson to
current Ghostboard risk using the schema defined in Experiment 4. The output is
a classification table, not fixes.

This is an audit/documentation experiment only. It must not change application
code, generated code, historical issue files, closed issue files, scripts, test
harnesses, screenshots, website assets, or build configuration.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/08-batch-e-wezboard-implementation.md`
  - record this experiment design, design review, Batch E classification result,
    completion review, and conclusion;
  - classify every issue in Batch E using the Experiment 4 historical audit row
    schema.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 8 to the `## Experiments` index with status `Designed`, then
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

- The result audits every Batch E issue exactly once:
  - `0715-wezboard`
  - `0716-wezboard-warnings`
  - `0717-remove-cocoa-crate`
  - `0718-finish-cocoa-removal`
  - `0719-wezboard-code-smells`
  - `0720-wezboard-manual-test`
  - `0721-wgpu-upgrade`
  - `0722-cargo-deps`
  - `0723-pane-borders`
  - `0724-wezboard-protocol`
  - `0725-wezboard-overlay`
  - `0726-wezboard-overlay-lifecycle`
  - `0727-wezboard-second-webview`
  - `0728-wezboard-remaining-protocol`
  - `0729-wezboard-reposition-and-protocol`
  - `0730-roamium-standalone-install`
  - `0731-wezboard-scroll-crash`
  - `0732-wezboard-reopen-tab`
  - `0733-ghostboard-shutdown`
  - `0734-build-scripts`
  - `0735-ghostboard-release-icon`
  - `0736-roamium-process-leak`
  - `0737-wezboard-icon`
  - `0738-wezboard-text-selection`
  - `0739-build-warnings`
  - `0740-wezboard-display-name`
  - `0741-protocol-split`
  - `0742-archive-ghostboard`
- The result uses the Experiment 4 row schema for every classification: source
  issue, batch, subsystem, durable lesson, current Ghostboard relevance,
  evidence paths, likelihood, risk or impact, recommended follow-up, and
  historical classification note.
- The result classifies each row as `Highly likely`, `Maybe`, or `No`, and
  explains the classification from issue evidence plus current code/test/doc
  evidence.
- The result treats all Batch E issues as closed historical evidence and does
  not modify or reinterpret their closure state.
- The result distinguishes Wezboard implementation lessons from current
  Ghostboard evidence. Wezboard protocol or overlay work is not automatically
  proof that restored Ghostboard has parity, and Wezboard-specific build/UI work
  is not automatically a Ghostboard bug.
- The result carries forward relevant Issue 810 findings where Batch E overlaps
  current Ghostboard risk, especially protocol message coverage, overlay
  lifecycle, multi-webview/tab routing, browser process cleanup, shutdown
  semantics, split protocol assumptions, text selection/input behavior, and
  branding/build-script evidence.
- The result explicitly evaluates Issue `0742` as historical evidence about why
  Ghostboard was archived, without treating the archive decision itself as a
  current defect.
- The result groups or summarizes related repeated findings after the table, but
  the table itself must still contain one row per Batch E issue.
- The result identifies the next audit slice after Batch E.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/08-batch-e-wezboard-implementation.md
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

- Any Batch E issue is omitted or classified more than once.
- The experiment edits historical issue files, application code, generated code,
  scripts, tests, screenshots, website assets, or build configuration.
- The result treats Wezboard historical fixes as current Ghostboard proof
  without current Ghostboard evidence.
- The result treats the historical Ghostboard archive as proof that restored
  Ghostboard is defective without current restored-Ghostboard evidence.
- The result labels build/branding/dependency issues as Ghostboard runtime bugs
  without a direct current product path.
- The result expands into other historical batches before Batch E is concluded.

## Design Review

Fresh-context adversarial design review returned **APPROVED**.

Reviewer checks confirmed:

- The issue README links Experiment 8 as `Designed`.
- The experiment has `Description`, `Changes`, and `Verification`.
- Batch E matches Experiment 4 exactly: `0715`-`0742`, twenty-eight issues.
- Scope is audit-only and limited to Issue 810 docs.
- Issue `0742-archive-ghostboard` is handled as historical archive evidence, not
  proof of current restored-Ghostboard defects.
- Verification includes the Experiment 4 schema, pass/fail criteria, markdown
  formatting, `git diff --check`, completion review, and separate plan/result
  commit gates.
- `git diff --check` passed.
- The plan commit had not yet been made before review.

Findings: none.

## Result

**Result:** Pass

Batch E was audited as the Wezboard implementation and Ghostboard archive
transition slice. The classification unit is each historical issue folder, so
the table below has exactly twenty-eight rows: one for every issue from `0715`
through `0742`.

### Classification Table

| Source issue                            | Batch | Subsystem                         | Durable lesson                                                                                                                           | Current Ghostboard relevance                                                                                                                                  | Evidence paths                                                                                                                                  | Likelihood      | Risk or impact                                                                                                        | Recommended follow-up                                                                                                                         | Historical classification note                                                                                                   |
| --------------------------------------- | ----- | --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `0715-wezboard`                         | E     | Board architecture / socket IPC   | A board needs its own socket listener, pane/server registry, browser process launch, and protocol dispatch.                              | Restored Ghostboard has a TermSurf socket listener and pane/server state in Zig; ordinary browsing was later exercised in Issues 808/809.                     | Issue 715 README; `ghostboard/src/apprt/termsurf.zig`; Issue 808/809 results.                                                                   | `No`            | Low current risk for the basic board foundation.                                                                      | None beyond later batch-specific gaps.                                                                                                        | Wezboard proved board-agnostic architecture; current Ghostboard has independent implementation evidence.                         |
| `0716-wezboard-warnings`                | E     | Build hygiene                     | Warning-free builds keep fork drift manageable.                                                                                          | Wezboard-specific warning cleanup does not prove or disprove current Ghostboard warnings.                                                                     | Issue 716 README; current root scripts omit a `ghostboard` component.                                                                           | `Maybe`         | Restored Ghostboard may have warning/build hygiene debt not covered by current root scripts.                          | Add Ghostboard to build-warning hygiene checks when build scripts are restored.                                                               | Historical warning count was Wezboard-only, but the durable lesson still applies to restored Ghostboard.                         |
| `0717-remove-cocoa-crate`               | E     | Wezboard macOS dependencies       | Migrating ObjC interop should be staged and mechanically verified.                                                                       | Current Ghostboard is a Ghostty/Zig+Swift app, not the Wezboard Rust macOS layer that used `cocoa`.                                                           | Issue 717 README; `ghostboard/macos/Sources`; `ghostboard/build.zig`.                                                                           | `No`            | No direct current Ghostboard risk.                                                                                    | None.                                                                                                                                         | Dependency migration was Wezboard-specific.                                                                                      |
| `0718-finish-cocoa-removal`             | E     | Wezboard macOS dependencies       | Removing legacy ObjC dependencies needs complete dependency-tree verification.                                                           | Current Ghostboard does not carry the Wezboard `cocoa`/`objc` 0.2 dependency path.                                                                            | Issue 718 README; current Ghostboard source layout.                                                                                             | `No`            | No direct current Ghostboard risk.                                                                                    | None.                                                                                                                                         | Wezboard dependency lesson, not restored-Ghostboard evidence.                                                                    |
| `0719-wezboard-code-smells`             | E     | Wezboard ObjC interop cleanup     | Mechanical migrations should be followed by a smell pass for unsafe boilerplate, magic numbers, and unwraps.                             | Current Ghostboard may have its own Zig/Swift smells, but this issue's specific objc2 migration smells do not transfer directly.                              | Issue 719 README; current Ghostboard uses different language/runtime boundaries.                                                                | `No`            | No specific Ghostboard defect identified.                                                                             | Rely on later source-code audit stage for Ghostboard-native smells.                                                                           | The lesson is general, but this historical issue is not evidence of a current app bug.                                           |
| `0720-wezboard-manual-test`             | E     | Manual macOS regression testing   | Large macOS interop migrations require a manual walkthrough of window, input, clipboard, monitor, and lifecycle behavior.                | Issue 809 covers much of the current Ghostboard window/input/geometry matrix, but not every manual item from the Wezboard checklist.                          | Issue 720 README; Issue 809 geometry matrix; Experiment 7 clipboard and multi-display findings.                                                 | `Maybe`         | Clipboard edge cases, multi-monitor behavior, IME, drag/drop, and other manual app behaviors may remain under-tested. | Fold the surviving manual checklist items into the later app walkthrough, avoiding duplicate coverage for Issue 809 rows.                     | Current proof is broad but not identical to the historical Wezboard manual checklist.                                            |
| `0721-wgpu-upgrade`                     | E     | Wezboard rendering dependencies   | Renderer dependency upgrades should proceed in small, buildable steps.                                                                   | Current Ghostboard does not use Wezboard's wgpu renderer stack.                                                                                               | Issue 721 README; Ghostboard renderer/build layout.                                                                                             | `No`            | No direct Ghostboard runtime risk.                                                                                    | None.                                                                                                                                         | Wezboard rendering-dependency issue.                                                                                             |
| `0722-cargo-deps`                       | E     | Wezboard Rust dependencies        | Dependency audits should distinguish testable cross-platform dependencies from untested platform-only ones.                              | Current Ghostboard's core app is Zig/Swift; Wezboard cargo dependency drift does not imply a restored-Ghostboard parity bug.                                  | Issue 722 README; `ghostboard/build.zig`; `ghostboard/macos`.                                                                                   | `No`            | No direct Ghostboard runtime risk.                                                                                    | None.                                                                                                                                         | General maintenance lesson, but not current Ghostboard evidence.                                                                 |
| `0723-pane-borders`                     | E     | Split pane UI                     | Split-pane decorations must not steal terminal cells or misalign overlays.                                                               | Current Ghostboard uses Ghostty split UI and Issue 809 covers split overlay geometry; later Wezboard border-specific regressions were audited in Batch F.     | Issue 723 README; Issue 809 split rows; Experiment 7 split-border rows.                                                                         | `No`            | Low current Ghostboard risk from this historical Wezboard feature.                                                    | Keep Issue 809 geometry matrix as the guard.                                                                                                  | Wezboard-specific split-border implementation does not transfer directly.                                                        |
| `0724-wezboard-protocol`                | E     | Protocol core / CALayerHost       | Protocol parity starts with state management, browser spawning, tab lifecycle, forwarding, and CAContext handling.                       | Current Ghostboard has source evidence for these core paths and runtime evidence from Issues 808/809.                                                         | Issue 724 README; `ghostboard/src/apprt/termsurf.zig`; `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`; Issues 808/809.         | `No`            | Low current risk for the core protocol foundation.                                                                    | None beyond specific message gaps already tracked.                                                                                            | The broad foundation exists; later rows capture remaining message-specific gaps.                                                 |
| `0725-wezboard-overlay`                 | E     | Overlay rendering / protocol gaps | Visible overlays require an AppKit-hosted layer bridge, real cell metrics, input, resize, queries, DevTools, focus, and cursor handling. | Current Ghostboard has overlay/runtime evidence for most items, but `CursorChanged` remains a high-confidence current gap from prior Issue 810 findings.      | Issue 725 README; Experiment 3 and Batch H `CursorChanged` findings; `ghostboard/src/apprt/termsurf.zig`; Issue 809 geometry matrix.            | `Highly likely` | Browser cursor shape likely does not update under Ghostboard even though overlay rendering itself works.              | Prioritize the existing `CursorChanged` follow-up and include it in browser hover regression tests.                                           | Classified high because this row reinforces an already evidenced current message gap, not because overlays are generally broken. |
| `0726-wezboard-overlay-lifecycle`       | E     | Overlay lifecycle / multi-pane    | Overlay visibility, query handlers, and multi-pane positioning need explicit lifecycle hooks, not only initial creation.                 | Current Ghostboard has strong geometry/lifecycle proof, but native cursor and resize-flash-style polish were not exhaustively proven.                         | Issue 726 README; Issue 809 lifecycle/geometry rows; Experiment 7 cursor/native popup findings.                                                 | `Maybe`         | Some visual polish or untested lifecycle edge cases may remain even though ordinary multi-pane geometry works.        | Keep this as secondary coverage for the app walkthrough; do not reopen ordinary geometry without stronger evidence.                           | Most historical bugs are covered; remaining risk is limited to untested edge cases and known cursor gap overlap.                 |
| `0727-wezboard-second-webview`          | E     | Multi-webview geometry            | Pane-relative grid coordinates must convert to window/screen pixels with correct backing scale and per-window overlay views.             | Issue 809 covers current Ghostboard split, tab, window, resize, zoom, and backing-scale geometry, with the known single-display caveat.                       | Issue 727 README; Issue 809 conclusion; Experiment 7 multi-display caveat.                                                                      | `No`            | Ordinary multi-webview geometry is covered by current evidence.                                                       | Keep multi-display as the separate follow-up already recorded.                                                                                | Current Ghostboard evidence is stronger than a direct Wezboard port assumption.                                                  |
| `0728-wezboard-remaining-protocol`      | E     | Input/focus/cursor protocol       | Interactive browsing requires keyboard, mouse, scroll, cursor changes, and focus state.                                                  | Current Ghostboard has source/runtime evidence for input and focus, but still lacks active `CursorChanged` handling evidence.                                 | Issue 728 README; `ghostboard/src/apprt/termsurf.zig` input/focus functions; Experiment 3 and Batch H `CursorChanged` findings; Issue 809.      | `Highly likely` | Cursor feedback is likely missing under browser hover in restored Ghostboard.                                         | Same `CursorChanged` follow-up as Issue 725; add hover cursor assertion to the durable regression guard.                                      | Input/focus portions are covered; cursor message coverage remains a current high-confidence gap.                                 |
| `0729-wezboard-reposition-and-protocol` | E     | Resize, DevTools, OpenSplit       | Resize updates must come from window lifecycle, and DevTools/OpenSplit need explicit protocol handlers.                                  | Current Ghostboard has AppKit geometry correction, `OpenSplit`, `SetDevtoolsOverlay`, and `CreateDevtoolsTab` paths, plus Issue 809 DevTools/geometry proof.  | Issue 729 README; `ghostboard/src/apprt/termsurf.zig`; `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`; Issue 809.              | `No`            | Low current risk for the named features.                                                                              | None beyond multi-profile DevTools follow-up already recorded in Batch F.                                                                     | Current source and runtime evidence cover the broad row.                                                                         |
| `0730-roamium-standalone-install`       | E     | Browser discovery / install       | Boards should discover an installed Roamium without dev-path hardcodes, or accept an explicit browser path.                              | Restored Ghostboard currently launches only absolute browser paths; named/default browser launch is logged as not implemented.                                | Issue 730 README; `ghostboard/src/apprt/termsurf.zig` `isAbsolutePath` and named-browser warning; current install script paths.                 | `Highly likely` | Installed/default Roamium discovery may fail unless the TUI passes an absolute browser path.                          | Add a focused Ghostboard browser-resolution issue: default/named browser lookup, installed Roamium path, and explicit path behavior.          | Strong static evidence shows the named-browser path is not implemented in current Ghostboard.                                    |
| `0731-wezboard-scroll-crash`            | E     | Raw scroll input                  | Browser scroll events need raw phase/momentum data with correct 64-bit macOS types.                                                      | Current Ghostboard forwards scroll with `u64` phase and momentum fields, and Issue 809 exercises browser/scroll behavior.                                     | Issue 731 README; `ghostboard/src/apprt/termsurf.zig` `forwardScrollEvent`; `scripts/ghostboard-geometry-matrix.sh`; Issue 809.                 | `No`            | Low current risk for this historical crash class.                                                                     | None beyond existing input/geometry regression coverage.                                                                                      | Current implementation reflects the historical type lesson.                                                                      |
| `0732-wezboard-reopen-tab`              | E     | Browser tab/server lifecycle      | Last-tab close must not leave stale server state that blocks future browser opens.                                                       | Current Ghostboard sends `CloseTab` and clears pane/overlay state; reopening after close has some matrix coverage, but last-tab/server reuse is not isolated. | Issue 732 README; `ghostboard/src/apprt/termsurf.zig` `paneClosed`, `cleanupTuiPanes`, `sendCloseTab`; Issue 809 close/reopen-style rows.       | `Maybe`         | Reopening after the final browser tab closes could still regress if server state is stale.                            | Add a narrow last-browser-tab close/reopen smoke to the lifecycle guard.                                                                      | Historical `Shutdown` framing changed later, so classify the current lifecycle invariant rather than the obsolete mechanism.     |
| `0733-ghostboard-shutdown`              | E     | Browser process shutdown          | Browser shutdown should be graceful; force-killing Roamium risks skipping Chromium cleanup.                                              | The historical `Shutdown` protobuf was later removed; current Roamium exits on socket EOF, and Ghostboard closes client sockets on IPC stop.                  | Issue 733 README; Issue 736 README; `roamium/src/ipc.rs`; `ghostboard/src/apprt/termsurf.zig`; `proto/termsurf.proto`.                          | `No`            | The old missing-`Shutdown` bug is obsolete under the current protocol.                                                | Ensure docs do not reintroduce `Shutdown` as a current requirement when closing Issue 810.                                                    | This issue is historical evidence, superseded by Issue 736's EOF model.                                                          |
| `0734-build-scripts`                    | E     | Build/install scripts             | Component build/install/uninstall scripts should cover every active component consistently.                                              | Restored Ghostboard exists in the tree, but root scripts still list only `wezboard`, `roamium`, `webtui`, `chromium`, and `all`.                              | Issue 734 README; `scripts/build.sh`; `scripts/install.sh`; `scripts/uninstall.sh`; current `ghostboard/` tree.                                 | `Highly likely` | Developers cannot build/install/test restored Ghostboard through the standard component scripts.                      | Open a focused build-script parity issue for adding Ghostboard to build/install/uninstall or documenting the intentional exception.           | This is a direct current tooling mismatch, not a historical Wezboard-only concern.                                               |
| `0735-ghostboard-release-icon`          | E     | Ghostboard branding               | Release/debug app icons should be explicit and verified against macOS icon caches.                                                       | Later Ghostboard branding work was accepted in Issue 808, and current asset sources include Ghostboard/TermSurf icon assets.                                  | Issue 735 README; Issue 808 branding result; `assets/ghostboard-1.png`; `ghostboard/macos/Assets.xcassets`.                                     | `No`            | Low current branding risk from this historical icon issue.                                                            | None unless a later app walkthrough observes an incorrect icon.                                                                               | Current branding evidence supersedes this historical icon task.                                                                  |
| `0736-roamium-process-leak`             | E     | Browser process lifecycle         | Browser engines should exit on GUI socket EOF so GUI crashes do not leave orphan Roamium processes.                                      | Current Roamium IPC still treats EOF/error as a quit trigger; Ghostboard passes both GUI and browser listen sockets when spawning.                            | Issue 736 README; `roamium/src/ipc.rs`; `ghostboard/src/apprt/termsurf.zig` `spawnBrowserProcess`.                                              | `No`            | Low current risk for GUI-crash orphaning from the historical cause.                                                   | Include orphan-process checks in lifecycle testing if standard build scripts are restored.                                                    | Current source preserves the EOF shutdown model.                                                                                 |
| `0737-wezboard-icon`                    | E     | Wezboard branding                 | App templates and plist icon references must agree.                                                                                      | Wezboard icon work does not affect current Ghostboard parity.                                                                                                 | Issue 737 README.                                                                                                                               | `No`            | None for Ghostboard behavior.                                                                                         | None.                                                                                                                                         | Wezboard-only branding issue.                                                                                                    |
| `0738-wezboard-text-selection`          | E     | Browser mouse drag/text selection | Browser text selection needs click-count tracking, drag button modifiers, and continued drag events outside the overlay.                 | Current Ghostboard has browser mouse proof, but drag selection and outside-overlay continuation are not specifically proven.                                  | Issue 738 README; Issue 809 mouse input rows; `ghostboard/src/apprt/termsurf.zig` mouse forwarding.                                             | `Maybe`         | Browser text selection may fail even if ordinary click and mouse input work.                                          | Add text selection, double-click, triple-click, and drag-outside-overlay cases to the input regression guard.                                 | Current mouse proof is broad but not specific enough for this selection behavior.                                                |
| `0739-build-warnings`                   | E     | Build hygiene                     | Release builds for all shipped components should be warning-free.                                                                        | Current root `all` build does not include restored Ghostboard, so warning-free release evidence for Ghostboard is not part of the standard script.            | Issue 739 README; `scripts/build.sh`; current `ghostboard/` tree.                                                                               | `Maybe`         | Ghostboard warning regressions could be missed by normal TermSurf build hygiene.                                      | Tie warning checks to the build-script parity follow-up from Issue 734.                                                                       | Wezboard/Roamium warning cleanup does not prove restored-Ghostboard warning status.                                              |
| `0740-wezboard-display-name`            | E     | Wezboard branding / display name  | macOS app display names can come from multiple sources and need explicit checks.                                                         | Wezboard display-name work does not directly affect Ghostboard; current Ghostboard branding was handled by later restored-Ghostboard work.                    | Issue 740 README; Issue 808 branding result.                                                                                                    | `No`            | Low current Ghostboard risk.                                                                                          | None unless app walkthrough observes a display-name mismatch.                                                                                 | Wezboard-only display-name task, with later Ghostboard branding evidence.                                                        |
| `0741-protocol-split`                   | E     | Direct browser connection         | TUI↔Browser content messages should use the browser socket directly; the GUI owns only GUI responsibilities.                             | Current Ghostboard sends `BrowserReady` with `browser_socket`, and webtui handles direct browser events.                                                      | Issue 741 README; `ghostboard/src/apprt/termsurf.zig` `sendBrowserReady`; `webtui/src/ipc.rs`; `webtui/src/main.rs`; `roamium/src/dispatch.rs`. | `No`            | Low current risk for the direct-browser handoff itself.                                                               | Keep separate browser-state/API `Maybe` findings from Experiments 3, 6, and 7.                                                                | The direct handoff is present; later API rows cover unproven content-message details.                                            |
| `0742-archive-ghostboard`               | E     | Project status / scripts / docs   | Archiving a GUI should simplify maintenance, but reviving it later requires scripts and docs to stop treating it as absent.              | Restored Ghostboard exists, yet some repo docs/scripts still describe Ghostboard as archived or omit it from component scripts.                               | Issue 742 README; `AGENTS.md`; `scripts/build.sh`; `scripts/install.sh`; current `ghostboard/` tree.                                            | `Maybe`         | Developer workflow and product status can drift if restored Ghostboard remains outside standard docs/scripts.         | Address docs/scripts status together with the build-script parity follow-up; do not treat the 2026-03-11 archive as a current runtime defect. | The archive decision is historical; the current risk is stale workflow/documentation around the restored app.                    |

### Findings Summary

`Highly likely` findings:

- Browser cursor shape remains likely missing in current Ghostboard. Batch E
  reinforces the existing Issue 810 `CursorChanged` finding through Issues 725
  and 728.
- Installed/default Roamium discovery is likely incomplete because current
  Ghostboard only launches absolute browser paths and logs named-browser launch
  as unimplemented.
- Standard root build/install/uninstall scripts likely do not cover restored
  Ghostboard, even though `ghostboard/` exists again.

`Maybe` findings:

- Build hygiene and warning checks may miss Ghostboard until scripts include it.
- The old Wezboard manual checklist has surviving under-tested behaviors:
  clipboard edge cases, IME, drag/drop, multi-monitor, and other app-level
  walkthrough items not already covered by Issue 809.
- Some overlay lifecycle polish remains under-tested, though ordinary geometry
  should not be reopened without stronger evidence.
- Last-tab close/reopen deserves a narrow lifecycle smoke.
- Browser text selection deserves dedicated input coverage: double-click,
  triple-click, drag selection, and drag outside the overlay.
- Revived-Ghostboard documentation and workflow may still carry archived-era
  assumptions.

`No` findings:

- Wezboard-only ObjC, wgpu, cargo dependency, split-border, icon, and display
  name issues do not directly map to restored-Ghostboard runtime bugs.
- Core protocol foundation, overlay presentation, input/focus forwarding, scroll
  phase typing, direct-browser `BrowserReady`, DevTools/OpenSplit, and Roamium
  EOF shutdown have current source and/or runtime evidence.
- Historical `Shutdown` as a protocol message is obsolete after Issue 736's EOF
  shutdown model.

### Verification

Commands run:

```bash
for d in issues/0715-wezboard issues/0716-wezboard-warnings issues/0717-remove-cocoa-crate issues/0718-finish-cocoa-removal issues/0719-wezboard-code-smells issues/0720-wezboard-manual-test issues/0721-wgpu-upgrade issues/0722-cargo-deps issues/0723-pane-borders issues/0724-wezboard-protocol issues/0725-wezboard-overlay issues/0726-wezboard-overlay-lifecycle issues/0727-wezboard-second-webview issues/0728-wezboard-remaining-protocol issues/0729-wezboard-reposition-and-protocol issues/0730-roamium-standalone-install issues/0731-wezboard-scroll-crash issues/0732-wezboard-reopen-tab issues/0733-ghostboard-shutdown issues/0734-build-scripts issues/0735-ghostboard-release-icon issues/0736-roamium-process-leak issues/0737-wezboard-icon issues/0738-wezboard-text-selection issues/0739-build-warnings issues/0740-wezboard-display-name issues/0741-protocol-split issues/0742-archive-ghostboard; do
  sed -n '/^# /p;/^## Goal/,+8p;/^## Conclusion/,$p' "$d/README.md" | sed -n '1,160p'
done

rg -n \
  "Shutdown|shutdown|socket EOF|EOF|close|child_pid|recordServerChild|spawnBrowserProcess|removePane|pane_count|SetGuiActive|CursorChanged|FocusChanged|KeyEvent|MouseEvent|ScrollEvent|MouseMove|BrowserReady|browser_socket|listen_socket" \
  ghostboard/src/apprt/termsurf.zig roamium/src webtui/src proto/termsurf.proto

sed -n '480,620p' ghostboard/src/apprt/termsurf.zig
sed -n '864,2075p' ghostboard/src/apprt/termsurf.zig
sed -n '1,260p' ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift

sed -n '1,240p' scripts/build.sh
sed -n '1,240p' scripts/install.sh
sed -n '1,240p' scripts/uninstall.sh

rg -n "components:|ghostboard|wezboard|roamium|webtui|chromium|all\\)|usage|Usage|COMPONENT" \
  scripts/build.sh scripts/install.sh scripts/uninstall.sh README.md AGENTS.md
```

Verification results:

- All twenty-eight Batch E issues are represented exactly once in the
  classification table.
- Every row uses the Experiment 4 schema.
- All Batch E issues are treated as closed historical evidence.
- No historical issue files, application code, generated code, scripts, tests,
  screenshots, website assets, or build configuration were edited.
- The result distinguishes Wezboard implementation lessons from current
  Ghostboard evidence.
- The result treats Issue `0742` as historical archive evidence, not proof that
  restored Ghostboard is defective.

## Conclusion

Batch E adds two high-signal current follow-ups beyond the already-known cursor
gap: restored Ghostboard likely needs standard build/install/uninstall script
coverage, and its browser launch path likely needs installed/default Roamium
resolution instead of requiring absolute browser paths.

Batch E also confirms that several broad Wezboard-era foundations are no longer
useful as current bug candidates: Ghostboard now has its own socket/protocol
implementation, direct-browser `BrowserReady` handoff, input/focus forwarding,
DevTools/OpenSplit paths, and Roamium EOF shutdown evidence.

The next audit slice should move backward to Batch D (`0700`-`0714`), because it
covers the socket transition, CALayerHost/XPC cleanup, and pre-Wezboard protocol
stabilization work that can still expose restored-Ghostboard parity risks.

## Completion Review

Fresh-context adversarial completion review returned **APPROVED**.

Reviewer checks confirmed:

- Batch E rows match `0715`-`0742` exactly once.
- Rows follow the Experiment 4 schema.
- The issue README marks Experiment 8 as `Pass`.
- Only Issue 810 docs are changed.
- `git diff --check` passed.
- The result commit had not yet been made before review.
- The key classifications are defensible from the cited source evidence.

Findings: none.
