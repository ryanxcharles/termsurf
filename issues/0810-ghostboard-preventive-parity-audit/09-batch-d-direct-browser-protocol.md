# Experiment 9: Batch D Direct Browser and Protocol Audit

## Description

Classify Batch D from Experiment 4: issues `0680`-`0714`. This batch covers the
direct-browser and protocol stabilization era before Wezboard: dark mode,
process/quit commands, direct TUI-to-browser IPC, visited links, DevTools,
multi-profile tracking, Chromium crash recovery, tab lifecycle, file and smart
URL resolution, pane-vs-tab identity, click suppression, Unix socket migration,
protobuf-c generation, Roamium extraction, multi-engine research, Ghostboard and
webtui naming, engine labels, website rename cleanup, and issue-numbering
workflow.

This experiment should read every Batch D issue and map each durable lesson to
current Ghostboard risk using the schema defined in Experiment 4. The output is
a classification table, not fixes.

This is an audit/documentation experiment only. It must not change application
code, generated code, historical issue files, closed issue files, scripts, test
harnesses, screenshots, website assets, or build configuration.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/09-batch-d-direct-browser-protocol.md`
  - record this experiment design, design review, Batch D classification result,
    completion review, and conclusion;
  - classify every issue in Batch D using the Experiment 4 historical audit row
    schema.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 9 to the `## Experiments` index with status `Designed`, then
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

- The result audits every Batch D issue exactly once:
  - `0680-dark-mode`
  - `0681-quitall`
  - `0682-direct-xpc`
  - `0683-visited-links`
  - `0684-devtools`
  - `0685-multi-profile-tracking`
  - `0686-chromium-crash`
  - `0687-one-devtools`
  - `0688-devtools-split`
  - `0689-tab-lifecycle`
  - `0690-devtools-split`
  - `0691-devtools-direct-command`
  - `0692-file-subcommand`
  - `0693-smart-resolve`
  - `0694-tab-id-chromium`
  - `0695-suppress-activation-drag`
  - `0696-double-click-suppression`
  - `0697-update-docs`
  - `0698-unix-sockets`
  - `0699-protobuf-build`
  - `0700-tui-gui-sockets`
  - `0701-chromium-sockets`
  - `0702-socket-cleanup`
  - `0703-remove-click-suppression`
  - `0704-browser-bindings`
  - `0705-browser-bindings`
  - `0706-plusium-devtools`
  - `0707-roamium`
  - `0708-roamium-only`
  - `0709-wezboard`
  - `0710-gecko-webkit-ladybird`
  - `0711-rename-ghostboard-webtui`
  - `0712-engine-label`
  - `0713-rename-homepage-website`
  - `0714-seven-digit-issues`
- The result uses the Experiment 4 row schema for every classification: source
  issue, batch, subsystem, durable lesson, current Ghostboard relevance,
  evidence paths, likelihood, risk or impact, recommended follow-up, and
  historical classification note.
- The result classifies each row as `Highly likely`, `Maybe`, or `No`, and
  explains the classification from issue evidence plus current code/test/doc
  evidence.
- The result treats all Batch D issues as closed historical evidence and does
  not modify or reinterpret their closure state.
- The result distinguishes obsolete XPC-era mechanisms from current Unix
  socket/protobuf evidence.
- The result distinguishes browser-engine/Roamium findings from Ghostboard-owned
  GUI parity findings.
- The result carries forward relevant Issue 810 findings where Batch D overlaps
  current Ghostboard risk, especially direct-browser handoff, multi-profile
  routing, DevTools, tab lifecycle, cursor/input/click suppression, smart URL
  resolution, color scheme state, crash recovery, and generated protobuf
  coverage.
- The result explicitly handles duplicate historical themes, including duplicate
  DevTools split issues and duplicate browser-bindings issues, while still
  classifying every issue folder exactly once.
- The result groups or summarizes related repeated findings after the table, but
  the table itself must still contain one row per Batch D issue.
- The result identifies the next audit slice after Batch D.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/09-batch-d-direct-browser-protocol.md
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

- Any Batch D issue is omitted or classified more than once.
- The experiment edits historical issue files, application code, generated code,
  scripts, tests, screenshots, website assets, or build configuration.
- The result treats obsolete XPC-era implementation details as current
  Ghostboard requirements without mapping them to the current socket/protobuf
  architecture.
- The result treats Roamium or webtui behavior as a Ghostboard GUI bug without a
  direct current Ghostboard ownership path.
- The result treats older Ghostboard archive-era status or naming docs as proof
  of current restored-Ghostboard runtime defects.
- The result expands into other historical batches before Batch D is concluded.

## Design Review

Fresh-context adversarial design review returned **APPROVED**.

Reviewer checks confirmed:

- The issue README links Experiment 9 as `Designed`.
- The experiment has `Description`, `Changes`, and `Verification`.
- Batch D exactly matches Experiment 4: `0680`-`0714`, thirty-five issues.
- The design requires the Experiment 4 schema, obsolete XPC/current socket
  distinction, Roamium/webtui versus Ghostboard ownership distinction,
  duplicate-theme handling, pass/fail criteria, markdown formatting,
  `git diff --check`, completion review, and separate plan/result commit gates.
- Scope is audit-only and planned changes are limited to Issue 810 docs.
- `git diff --check` passed.
- The plan commit had not yet been made before review.

Findings: none.

## Result

**Result:** Pass

Batch D was audited as the direct-browser, protocol, Roamium, and migration
setup slice. The classification unit is each historical issue folder, so the
table below has exactly thirty-five rows: one for every issue from `0680`
through `0714`.

### Classification Table

| Source issue                    | Batch | Subsystem                           | Durable lesson                                                                                                          | Current Ghostboard relevance                                                                                                                                                        | Evidence paths                                                                                                                 | Likelihood      | Risk or impact                                                                                             | Recommended follow-up                                                                                         | Historical classification note                                                                                 |
| ------------------------------- | ----- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | --------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `0680-dark-mode`                | D     | Color scheme state                  | Initial tab creation and dynamic updates both need preferred color scheme propagation.                                  | Roamium and webtui support `SetColorScheme`, but current Ghostboard creates browser/devtools tabs with `dark = 0` and has no active GUI-side system appearance forwarding evidence. | Issue 680 README; `ghostboard/src/apprt/termsurf.zig` `create_tab.dark = 0`; `webtui/src/ipc.rs`; `roamium/src/dispatch.rs`.   | `Maybe`         | System/default dark-mode parity may be wrong even if manual TUI color commands work.                       | Add Ghostboard color-scheme smoke covering initial dark state, `:colorscheme`, and system appearance changes. | XPC transport is obsolete; the color-state invariant remains relevant.                                         |
| `0681-quitall`                  | D     | webtui command dispatch             | Subsequence command matching needs deterministic tie-breaking and a global quit command.                                | Command dispatch is webtui-owned and does not indicate a current Ghostboard GUI parity bug.                                                                                         | Issue 681 README; `webtui/src/main.rs`.                                                                                        | `No`            | None for Ghostboard behavior.                                                                              | None.                                                                                                         | TUI-only command issue.                                                                                        |
| `0682-direct-xpc`               | D     | Architecture / direct browser IPC   | Direct TUI-to-browser transport should be justified by real relay cost and ownership boundaries.                        | The rejected XPC design was later superseded by socket-based direct browser handoff, which current Ghostboard implements via `BrowserReady`.                                        | Issue 682 README; Issue 741 README; `ghostboard/src/apprt/termsurf.zig` `sendBrowserReady`; `webtui/src/ipc.rs`.               | `No`            | Low current risk.                                                                                          | None.                                                                                                         | Historical XPC analysis is superseded by the current socket architecture.                                      |
| `0683-visited-links`            | D     | Browser history / visited links     | Visited link styling needs browser-side history and visited-link infrastructure.                                        | This remains Roamium/Chromium engine infrastructure, not Ghostboard GUI protocol behavior.                                                                                          | Issue 683 README; current Roamium/Chromium ownership boundaries.                                                               | `No`            | Product feature may still be absent, but not a Ghostboard-owned parity bug.                                | Track visited links only in a browser-engine feature issue.                                                   | Engine-owned feature gap.                                                                                      |
| `0684-devtools`                 | D     | DevTools / last-tab tracking        | DevTools and `last` need reliable active-tab tracking, including profile-aware lookup.                                  | Current Ghostboard still keeps a single `last_browser_pane`; profile-filtered `QueryLast` only checks that pane.                                                                    | Issue 684 README; `ghostboard/src/apprt/termsurf.zig` `last_browser_pane` and `fillQueryLastReply`.                            | `Maybe`         | `web last --profile` and auto-targeting can fail in multi-profile workflows.                               | Add a multi-profile QueryLast/DevTools targeting smoke to the existing multi-profile follow-up.               | Historical limitation appears structurally similar in current code, but runtime proof is still needed.         |
| `0685-multi-profile-tracking`   | D     | Multi-profile tracking              | Bare last-tab lookup and profile-filtered lookup need different semantics.                                              | The current single-global tracker still does not search all panes for a requested profile.                                                                                          | Issue 685 README; `ghostboard/src/apprt/termsurf.zig` `fillQueryLastReply`; Experiment 7 multi-profile findings.               | `Maybe`         | Profile-specific `web last` or DevTools lookup may return no target despite an eligible pane.              | Combine with Issue 684 follow-up; test default and named profiles with multiple panes.                        | The old clap bug is gone, but the per-profile search limitation appears relevant.                              |
| `0686-chromium-crash`           | D     | DevTools duplicate sessions         | Chromium allows only one DevTools frontend per inspected page.                                                          | Current `QueryDevtools` validates that the inspected tab exists, but does not reject an existing DevTools pane for the same inspected tab.                                          | Issue 686 README; `ghostboard/src/apprt/termsurf.zig` `sendQueryDevtoolsReply` and `handleSetDevtoolsOverlay`.                 | `Highly likely` | Opening duplicate DevTools for one tab may recreate the historical Chromium crash class.                   | Prioritize a one-DevTools-per-tab guard and regression test in restored Ghostboard.                           | Classified high because the current guard evidence looks absent.                                               |
| `0687-one-devtools`             | D     | DevTools duplicate guard            | Duplicate DevTools sessions should be rejected before the TUI starts.                                                   | The historical guard does not appear present in current Ghostboard's `QueryDevtools` path.                                                                                          | Issue 687 README; `ghostboard/src/apprt/termsurf.zig` `fillQueryDevtoolsSuccess`.                                              | `Highly likely` | Same as Issue 686: duplicate DevTools may be allowed.                                                      | Same one-DevTools-per-tab follow-up; include error propagation and launch-time rejection expectations.        | This is the positive historical fix whose current counterpart appears missing.                                 |
| `0688-devtools-split`           | D     | Tab lifecycle / DevTools split      | DevTools split depends on reliable tab close, otherwise orphaned tabs crash on reopen.                                  | Current Ghostboard sends `CloseTab` on pane/TUI cleanup, but the exact DevTools close/reopen path should remain covered.                                                            | Issue 688 README; `ghostboard/src/apprt/termsurf.zig` `paneClosed`, `cleanupTuiPanes`, `sendCloseTab`; Issue 809 close rows.   | `Maybe`         | DevTools close/reopen may regress if tab lifecycle or duplicate guard is incomplete.                       | Include DevTools close/reopen in the lifecycle guard with the duplicate-DevTools test.                        | Current close-tab evidence is strong; duplicate-DevTools uncertainty keeps this as coverage risk.              |
| `0689-tab-lifecycle`            | D     | Browser tab lifecycle               | Closing a GUI pane must close the corresponding browser tab in the engine.                                              | Current Ghostboard sends `CloseTab`, removes tab lookups, and clears overlays on pane/TUI cleanup.                                                                                  | Issue 689 README; `ghostboard/src/apprt/termsurf.zig` `sendCloseTab`, `paneClosed`, `cleanupTuiPanes`; Issue 809 close rows.   | `No`            | Low current risk for ordinary tab close.                                                                   | Keep close/reopen rows in lifecycle regression coverage.                                                      | Current code directly carries the durable invariant.                                                           |
| `0690-devtools-split`           | D     | DevTools split / main-thread UI     | Split creation must be dispatched through the GUI's main-thread app surface and launch the correct command.             | Current Ghostboard has an `OpenSplit` Swift bridge that creates a split with `config.command`, and Issue 809 exercised DevTools split.                                              | Issue 690 README; Issue 691 README; `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`; Issue 809.                | `No`            | Low current risk for basic DevTools split creation.                                                        | None beyond duplicate-DevTools and multi-profile DevTools follow-ups.                                         | Broad feature has current evidence; narrower DevTools risks are captured elsewhere.                            |
| `0691-devtools-direct-command`  | D     | DevTools split command lifecycle    | DevTools panes should launch `web devtools` directly and close without a leftover shell.                                | Current `termsurf_open_split` sets the split surface command directly.                                                                                                              | Issue 691 README; `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`.                                             | `No`            | Low current risk.                                                                                          | None.                                                                                                         | Current Swift bridge reflects the historical lesson.                                                           |
| `0692-file-subcommand`          | D     | webtui local file resolution        | Local files need canonical path resolution and `file://` URL formatting.                                                | Current webtui still canonicalizes explicit `web file` paths and path-like input.                                                                                                   | Issue 692 README; `webtui/src/main.rs` `canonicalize` and `file://` handling.                                                  | `No`            | Low Ghostboard risk; this is TUI-owned.                                                                    | Include local-file URL in future app walkthrough only as a user workflow, not a Ghostboard bug candidate.     | TUI feature with current source evidence.                                                                      |
| `0693-smart-resolve`            | D     | webtui URL/file resolution          | Bare input needs deterministic URL/file/devtools resolution with clear failures.                                        | Current `resolve_input` implements the smart resolver.                                                                                                                              | Issue 693 README; `webtui/src/main.rs` `resolve_input`.                                                                        | `No`            | Low Ghostboard risk.                                                                                       | None.                                                                                                         | TUI-owned feature.                                                                                             |
| `0694-tab-id-chromium`          | D     | Tab identity / routing              | Chromium should route by `tab_id`; GUI owns pane-to-tab mapping.                                                        | Current protocol and Ghostboard code route browser messages by `(profile, browser, tab_id)` and pane lookup.                                                                        | Issue 694 README; `proto/termsurf.proto`; `ghostboard/src/apprt/termsurf.zig` `findTabLookup`, `upsertTabLookup`.              | `No`            | Low current risk for the identity model itself.                                                            | Keep multi-profile collision smoke from Batch F.                                                              | Current implementation extends the historical tab-id lesson with browser/profile keys.                         |
| `0695-suppress-activation-drag` | D     | Mouse activation / drag             | Activation clicks should not swallow browser drag/move input.                                                           | Issue 809 proves broad current mouse input and focus behavior; no specific activation-drag suppression evidence remains.                                                            | Issue 695 README; Issue 809 mouse/focus matrix; `ghostboard/src/apprt/termsurf.zig` mouse forwarding.                          | `No`            | Low current risk for the historical activation-suppression bug.                                            | Text-selection drag coverage remains tracked separately from Batch E.                                         | Current evidence covers ordinary mouse forwarding; selection specifics are a separate `Maybe`.                 |
| `0696-double-click-suppression` | D     | Mouse activation / click count      | Focus/activation state should not consume double-clicks.                                                                | Current broad mouse evidence exists, while dedicated double-click text-selection coverage is already a Batch E `Maybe`.                                                             | Issue 696 README; Issue 809 mouse matrix; Experiment 8 text-selection finding.                                                 | `No`            | Low incremental risk beyond the existing text-selection follow-up.                                         | Cover double-click in the text-selection guard from Experiment 8.                                             | Avoids duplicating the same follow-up as a separate finding.                                                   |
| `0697-update-docs`              | D     | Documentation                       | Docs should be audited after major feature waves.                                                                       | Stale restored-Ghostboard docs/scripts were already identified in Batch E; this issue adds no new current runtime evidence.                                                         | Issue 697 README; Experiment 8 `0742` and build-script findings.                                                               | `Maybe`         | Users and agents may follow stale docs around Ghostboard status or build flows.                            | Fold into the docs/scripts follow-up from Experiment 8.                                                       | Documentation-only risk, not a runtime bug.                                                                    |
| `0698-unix-sockets`             | D     | Socket/protobuf architecture        | Unix sockets plus protobuf should replace platform-specific IPC and be proven across languages.                         | Current protocol, Ghostboard, webtui, and Roamium all use Unix sockets plus protobuf.                                                                                               | Issue 698 README; `proto/termsurf.proto`; `ghostboard/src/apprt/termsurf.zig`; `webtui/src/ipc.rs`; `roamium/src/ipc.rs`.      | `No`            | Low current risk for this architecture foundation.                                                         | None.                                                                                                         | XPC-era blocker was resolved by later work.                                                                    |
| `0699-protobuf-build`           | D     | Generated protobuf / build          | Generated protobuf-c files must live inside the app build root so the macOS build links them.                           | Current Ghostboard carries generated protobuf-c files under `ghostboard/src/protobuf`.                                                                                              | Issue 699 README; `ghostboard/src/protobuf/termsurf.pb-c.c`; `ghostboard/src/protobuf/termsurf.pb-c.h`.                        | `No`            | Low current risk for generated binding presence.                                                           | Regeneration hygiene belongs in build-script parity follow-up if Ghostboard scripts are restored.             | Current file layout preserves the historical build lesson.                                                     |
| `0700-tui-gui-sockets`          | D     | TUI-to-GUI IPC                      | TUI↔GUI IPC should be pure Rust Unix socket/protobuf rather than ObjC/XPC.                                              | Current webtui uses socket IPC and Ghostboard listens for TermSurf socket clients.                                                                                                  | Issue 700 README; `webtui/src/ipc.rs`; `ghostboard/src/apprt/termsurf.zig`.                                                    | `No`            | Low current risk.                                                                                          | None.                                                                                                         | Historical XPC replacement is complete in current architecture.                                                |
| `0701-chromium-sockets`         | D     | GUI-to-browser IPC                  | GUI↔browser IPC should also use socket/protobuf with connection type tagging and cleanup.                               | Current Ghostboard handles browser `ServerRegister`, `TabReady`, `CaContext`, input, and tab messages over sockets.                                                                 | Issue 701 README; `ghostboard/src/apprt/termsurf.zig`; `roamium/src/ipc.rs`.                                                   | `No`            | Low current risk for the transport foundation.                                                             | None.                                                                                                         | Later direct-browser/content paths are tracked by specific findings.                                           |
| `0702-socket-cleanup`           | D     | XPC removal / socket cleanup        | Dead XPC fallback code and fixed connection limits should be removed after socket migration.                            | Current Ghostboard TermSurf paths are socket/protobuf based; no current XPC fallback requirement was identified.                                                                    | Issue 702 README; `ghostboard/src/apprt/termsurf.zig`; `proto/termsurf.proto`.                                                 | `No`            | Low current risk for XPC cleanup.                                                                          | None.                                                                                                         | Obsolete transport cleanup appears superseded.                                                                 |
| `0703-remove-click-suppression` | D     | Mouse activation / immediate input  | Browser clicks, drags, and scrolls should propagate immediately rather than needing an activation click.                | Issue 809 proves broad current mouse and scroll behavior under Ghostboard.                                                                                                          | Issue 703 README; Issue 809 mouse/scroll/focus rows; `ghostboard/src/apprt/termsurf.zig` mouse/scroll forwarding.              | `No`            | Low current risk for ordinary immediate mouse forwarding.                                                  | Keep text-selection-specific coverage from Experiment 8.                                                      | Current general mouse proof is enough for this historical activation bug.                                      |
| `0704-browser-bindings`         | D     | Browser binary selection            | Boards should treat browser binaries as opaque protocol-compatible processes selected by `--browser`.                   | Current Ghostboard's named/default browser launch path is not implemented; only absolute paths spawn.                                                                               | Issue 704 README; `ghostboard/src/apprt/termsurf.zig` `isAbsolutePath` and named-browser warning; Experiment 8 `0730`.         | `Highly likely` | Default `roamium` or named browser selection may fail without an absolute path.                            | Same browser-resolution follow-up from Experiment 8.                                                          | This reinforces the current named-browser gap.                                                                 |
| `0705-browser-bindings`         | D     | Browser bindings / DevTools routing | Browser-specific routing must include profile and browser identity, and DevTools must avoid unsafe pointer boundaries.  | Current Ghostboard has profile/browser keys, but multi-profile DevTools and duplicate-DevTools runtime proof remain missing.                                                        | Issue 705 README; `ghostboard/src/apprt/termsurf.zig`; Experiment 7 multi-profile DevTools finding; Issues 686/687 rows above. | `Maybe`         | DevTools may target or duplicate incorrectly in multi-browser/profile scenarios.                           | Combine with multi-profile and one-DevTools follow-ups.                                                       | Engine pointer-boundary fix is Roamium/Chromium-owned; GUI routing remains a coverage gap.                     |
| `0706-plusium-devtools`         | D     | Browser FFI / DevTools              | Async browser operations must not store raw C++ pointers passed across a C API boundary.                                | Roamium/libtermsurf_chromium now uses tab IDs for DevTools creation; this is engine-owned and not a Ghostboard GUI bug.                                                             | Issue 706 README; `roamium/src/dispatch.rs`; `chromium/src/content/libtermsurf_chromium`.                                      | `No`            | Low Ghostboard risk.                                                                                       | None from this audit.                                                                                         | Durable engine-FFI rule, not current GUI parity evidence.                                                      |
| `0707-roamium`                  | D     | Roamium architecture                | A small Rust browser binary can control Chromium through a C API and the TermSurf protocol.                             | Roamium exists and is the active Chromium engine; Ghostboard parity depends on launch/discovery and protocol integration, tracked separately.                                       | Issue 707 README; `roamium/src`; `ghostboard/src/apprt/termsurf.zig`.                                                          | `No`            | Low current risk for Roamium existence.                                                                    | Keep launch/discovery as the separate high-confidence finding.                                                | Architecture is current; specific gaps are classified elsewhere.                                               |
| `0708-roamium-only`             | D     | Default browser registry            | Roamium should be the default browser and legacy profile-server/Plusium paths should be removed.                        | webtui gets `roamium` from `HelloReply`, but current Ghostboard cannot launch non-absolute named browsers.                                                                          | Issue 708 README; `webtui/src/main.rs`; `ghostboard/src/apprt/termsurf.zig` `sendHelloReply`, `handleSetOverlay`.              | `Highly likely` | Default browser startup can fail in restored Ghostboard without explicit absolute `--browser`.             | Same browser-resolution follow-up from Experiment 8.                                                          | The registry/default invariant is present at the TUI layer but broken at current Ghostboard launch resolution. |
| `0709-wezboard`                 | D     | Wezboard research                   | The protocol should support multiple terminal GUI implementations.                                                      | Wezboard research does not imply a restored-Ghostboard bug.                                                                                                                         | Issue 709 README; later Wezboard issues; current Issue 810 Batch E/F results.                                                  | `No`            | None for current Ghostboard behavior.                                                                      | None.                                                                                                         | Research issue.                                                                                                |
| `0710-gecko-webkit-ladybird`    | D     | Multi-engine research               | The `libtermsurf_*` plus Rust-binary pattern generalizes across browser engines, with different compositing strategies. | Future engine research is outside current Ghostboard/Roamium parity unless a new engine is implemented.                                                                             | Issue 710 README; `AGENTS.md` engine table.                                                                                    | `No`            | No current Ghostboard bug.                                                                                 | Revisit when implementing Surfari/other engines.                                                              | Future-engine research.                                                                                        |
| `0711-rename-ghostboard-webtui` | D     | Naming / docs / app identity        | Component names and user-visible macOS app names need one source of truth.                                              | Later branding work and Issue 808 cover current Ghostboard branding; stale archive/docs risk is already captured in Batch E.                                                        | Issue 711 README; Issue 808 result; Experiment 8 `0742` row.                                                                   | `No`            | Low incremental risk.                                                                                      | None beyond docs/scripts follow-up from Experiment 8.                                                         | Naming history, not a new runtime parity finding.                                                              |
| `0712-engine-label`             | D     | Engine label / default selection    | The viewport should show the active browser engine and the TUI should use the GUI's advertised default.                 | Current webtui renders browser labels and Ghostboard sends `BrowserReady.browser`, but default named browser launch remains broken.                                                 | Issue 712 README; `webtui/src/main.rs`; `ghostboard/src/apprt/termsurf.zig`; Experiment 8 `0730` and Batch D `0708`.           | `Maybe`         | Label may work after an explicit absolute browser path, but default engine selection can fail before that. | Verify label display as part of browser-resolution follow-up.                                                 | Label path has source evidence; default launch dependency keeps a small coverage risk.                         |
| `0713-rename-homepage-website`  | D     | Website directory naming            | The website directory should have a clear stable name.                                                                  | Website directory naming does not affect Ghostboard runtime parity.                                                                                                                 | Issue 713 README; current `website/` directory.                                                                                | `No`            | None for Ghostboard behavior.                                                                              | None.                                                                                                         | Website-only issue.                                                                                            |
| `0714-seven-digit-issues`       | D     | Issue numbering workflow            | Issue filename conventions should avoid churn and preserve cross-reference stability.                                   | Current issue workflow uses four-digit folder numbers and immutable closed issues; this historical numbering scheme is superseded.                                                  | Issue 714 README; `AGENTS.md`; `issues/0810-ghostboard-preventive-parity-audit/04-historical-issue-inventory.md`.              | `No`            | None for Ghostboard runtime behavior.                                                                      | None.                                                                                                         | Historical workflow issue; current issue-folder rules differ.                                                  |

### Findings Summary

`Highly likely` findings:

- Duplicate DevTools sessions for one inspected tab appear likely allowed in
  current Ghostboard. Historical Issues 686 and 687 show this can crash
  Chromium; current `QueryDevtools` evidence only verifies target existence, not
  duplicate prevention.
- Default/named browser launch remains likely incomplete. Batch D Issues 704 and
  708 reinforce the Batch E browser-resolution finding: webtui can select
  `roamium`, but current Ghostboard only spawns absolute browser paths.

`Maybe` findings:

- Color-scheme parity needs proof for initial dark state, TUI commands, and
  system appearance changes.
- Multi-profile `web last` and DevTools targeting still need runtime proof
  because current `QueryLast` uses a single global pane tracker.
- DevTools close/reopen should be tested with tab lifecycle and duplicate guard
  behavior.
- Documentation around restored Ghostboard status remains a possible workflow
  risk, already aligned with the Batch E docs/scripts finding.
- Engine label display likely works from source evidence but depends on the
  browser-resolution path for default launches.

`No` findings:

- Most XPC-era socket/protobuf work is superseded by current Unix
  socket/protobuf architecture.
- webtui-owned command dispatch, file resolution, smart URL resolution, and
  website/issue-numbering workflow items do not map to Ghostboard runtime bugs.
- Roamium existence, engine FFI safety, tab-id routing, CloseTab, OpenSplit, and
  direct-browser handoff have current source and/or runtime evidence.

### Verification

Commands run:

```bash
for d in issues/0680-* issues/0681-* issues/0682-* issues/0683-* issues/0684-* issues/0685-* issues/0686-* issues/0687-* issues/0688-* issues/0689-* issues/0690-* issues/0691-* issues/0692-* issues/0693-* issues/0694-* issues/0695-* issues/0696-* issues/0697-* issues/0698-* issues/0699-* issues/0700-* issues/0701-* issues/0702-* issues/0703-* issues/0704-* issues/0705-* issues/0706-* issues/0707-* issues/0708-* issues/0709-* issues/0710-* issues/0711-* issues/0712-* issues/0713-* issues/0714-*; do
  sed -n '/^# /p;/^## Goal/,+8p;/^## Conclusion/,$p' "$d/README.md" | sed -n '1,180p'
done

rg -n \
  "BrowserReady|browser_socket|TargetUrlChanged|CursorChanged|QueryDevtools|QueryTabs|QueryLast|SetColorScheme|Navigate|UrlChanged|LoadingState|TitleChanged|RendererCrashed|SetGuiActive|FocusChanged|CloseTab|CreateTab|CreateDevtoolsTab|OpenSplit|SetOverlay|SetDevtoolsOverlay|tab_id|pane_id|profile|browser|dark|dark mode|file|smart|resolve|visited" \
  proto/termsurf.proto ghostboard/src/apprt/termsurf.zig webtui/src roamium/src scripts \
  issues/0810-ghostboard-preventive-parity-audit/0*.md

sed -n '680,870p' ghostboard/src/apprt/termsurf.zig
sed -n '1010,1090p' ghostboard/src/apprt/termsurf.zig

rg -n "resolve_input|normalize_url|SetColorScheme|send_set_color_scheme|colorscheme|QueryDevtools|devtools|file://|canonicalize|--browser|browser|HelloReply|browsers" \
  webtui/src ghostboard/src/apprt/termsurf.zig roamium/src proto/termsurf.proto

rg -n "dark =|create_tab\\.dark|set_color_scheme|SetColorScheme|preferred_color_scheme|color scheme|dark" \
  ghostboard/src/apprt/termsurf.zig roamium/src chromium/src/content/libtermsurf_chromium
```

Verification results:

- All thirty-five Batch D issues are represented exactly once in the
  classification table.
- Every row uses the Experiment 4 schema.
- All Batch D issues are treated as closed historical evidence.
- No historical issue files, application code, generated code, scripts, tests,
  screenshots, website assets, or build configuration were edited.
- The result distinguishes obsolete XPC-era details from current socket/protobuf
  evidence.
- The result distinguishes Roamium/webtui ownership from Ghostboard GUI
  ownership.
- Duplicate historical themes are handled as separate issue rows but grouped in
  the findings summary.

## Conclusion

Batch D adds one new high-confidence runtime follow-up: restore the
one-DevTools-per-tab guard before duplicate DevTools sessions can recreate the
historical Chromium crash class.

It also reinforces the Batch E high-confidence browser-resolution follow-up:
current Ghostboard likely cannot launch the default named `roamium` browser
without an explicit absolute browser path.

The next audit slice should move backward to Batch C (`0600`-`0679`), because it
covers the late Ghostboard/XPC interaction work immediately before the socket
migration and can reveal older input, focus, lifecycle, and GUI-state lessons
that still matter under the restored Ghostboard implementation.

## Completion Review

Ramanujan reviewed the completed result and approved it with no findings.

The review verified that Batch D (`0680`-`0714`) is covered exactly once, all
rows match the Experiment 4 schema, historical Batch D issues remain untouched
and closed, the diff is limited to Issue 810 docs, the README marks Experiment 9
as `Pass`, `git diff --check` passes, the latest commit is still the Experiment
9 plan commit, and the contentious classifications are defensible against
current code evidence.
