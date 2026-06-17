# Experiment 11: Batch B Feasibility and Ghostboard Iterations Audit

## Description

Classify Batch B from Experiment 4: issue folders `0400`-`0515`. This batch
covers Chromium/Electron feasibility research and Ghostboard iteration work:
browser-engine choice, programming-language choice, terminal-emulator choice,
Swift/Rust/C++ ownership boundaries, early Chromium proofs of concept, profile
isolation, Electron patch experiments, XPC receivers, Ghostty-vs-WezTerm
selection, repo restructuring, ts5 rename work, Web TUI foundations, pink
texture and checkerboard overlay demos, Chromium frame delivery, multi-profile
scaling, vsync, Ctrl+Esc, mouse input, and drag behavior.

This experiment should read every Batch B issue folder and map each durable
lesson to current Ghostboard risk using the schema defined in Experiment 4. The
output is a classification table, not fixes.

Batch B has duplicate issue numbers in the historical archive: `0401` and `0410`
each appear in two distinct folders. Those folders must be audited as distinct
rows.

This is an audit/documentation experiment only. It must not change application
code, generated code, historical issue files, closed issue files, scripts, test
harnesses, screenshots, website assets, or build configuration.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/11-batch-b-feasibility-and-iterations.md`
  - record this experiment design, design review, Batch B classification result,
    completion review, and conclusion;
  - classify every issue folder in Batch B using the Experiment 4 historical
    audit row schema.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 11 to the `## Experiments` index with status `Designed`, then
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

- The result audits every Batch B issue folder exactly once:
  - `0400-a-new-hope`
  - `0401-chromium-feasibility`
  - `0401-programming-language`
  - `0402-wezterm-vs-alacritty`
  - `0403-swift-rust-cpp`
  - `0404-terminal-emulator`
  - `0405-architecture-comparison`
  - `0406-chromium`
  - `0407-chromium-poc`
  - `0408-two-profiles`
  - `0409-electron-patch`
  - `0410-partial-electron`
  - `0410-two-profiles-2`
  - `0411-two-profiles-3`
  - `0412-one-profile`
  - `0413-one-profile-2`
  - `0414-two-profiles-xpc`
  - `0415-swift-receiver`
  - `0416-rust-receiver`
  - `0417-ghostty-vs-wezterm`
  - `0418-repo-restructure`
  - `0500-rename`
  - `0501-two-profiles`
  - `0502-attach-delay`
  - `0503-one-two-three`
  - `0504-web-tui`
  - `0505-pink-texture`
  - `0506-xpc-gateway`
  - `0507-chromium`
  - `0508-checkerboard`
  - `0509-chromium`
  - `0510-two-profiles`
  - `0511-three-profiles`
  - `0512-vsync`
  - `0513-ctrl-esc`
  - `0514-mouse`
  - `0515-drag`
- The result uses the Experiment 4 row schema for every classification: source
  issue, batch, subsystem, durable lesson, current Ghostboard relevance,
  evidence paths, likelihood, risk or impact, recommended follow-up, and
  historical classification note.
- The result classifies each row as `Highly likely`, `Maybe`, or `No`, and
  explains the classification from issue evidence plus current code/test/doc
  evidence.
- The result treats all Batch B issues as closed historical evidence and does
  not modify or reinterpret their closure state.
- The result distinguishes feasibility research and abandoned Electron/XPC
  mechanisms from current socket/protobuf, Roamium, and restored Ghostboard
  evidence.
- The result distinguishes Ghostboard GUI-owned parity findings from Roamium,
  Chromium, webtui, website, packaging, docs, and historical prototype findings.
- The result carries forward relevant Issue 810 findings where Batch B overlaps
  current Ghostboard risk, especially browser-engine selection, profile
  isolation, browser startup delays, Web TUI discovery, overlay geometry,
  input/mouse/drag behavior, vsync/latency, and old XPC receiver lessons.
- The result explicitly handles duplicate issue numbers by folder slug, while
  still classifying each issue folder exactly once.
- The result groups or summarizes related repeated findings after the table, but
  the table itself must still contain one row per Batch B issue folder.
- The result identifies the next audit slice after Batch B.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/11-batch-b-feasibility-and-iterations.md
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

- Any Batch B issue folder is omitted or classified more than once.
- Duplicate issue numbers are collapsed into one row instead of being audited by
  folder slug.
- The experiment edits historical issue files, application code, generated code,
  scripts, tests, screenshots, website assets, or build configuration.
- The result treats obsolete Electron or XPC implementation details as current
  Ghostboard requirements without mapping them to the current socket/protobuf
  architecture.
- The result treats Roamium, Chromium, webtui, website, packaging, docs, or
  prototype behavior as a Ghostboard GUI bug without a direct current Ghostboard
  ownership path.

## Design Review

Tesla reviewed the design and approved it with no findings.

The review verified that the plan is audit-only, the README links Experiment 11
as `Designed`, the Batch B list has thirty-seven rows matching Experiment 4 and
the filesystem, duplicate numeric prefixes `0401` and `0410` are preserved as
separate folder rows, the Experiment 4 row schema is required, closed historical
issue immutability is preserved, obsolete Electron/XPC mechanisms must be mapped
to the current architecture, non-Ghostboard ownership boundaries are explicit,
and the fail criteria are clear.

## Result

**Result:** Pass

Batch B was audited as thirty-seven closed historical issue folders. The
duplicate numeric prefixes `0401` and `0410` were kept as separate folder rows.

| Source issue                   | Batch | Subsystem                                 | Durable lesson                                                                                                                                   | Current Ghostboard relevance                                                                                                                                                    | Evidence paths                                                                                                                                                                                  | Likelihood    | Risk or impact                                                                                                   | Recommended follow-up                                                                                          | Historical classification note                                                                      |
| ------------------------------ | ----- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- | ---------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `0400-a-new-hope`              | B     | Architecture reset                        | The ts4 restart prioritized owning the window/event loop/compositor and moving past CEF constraints.                                             | Current Ghostboard no longer follows the exact own-everything plan, but it keeps the important lesson by using a terminal fork plus out-of-process browser engine.              | `issues/0400-a-new-hope/README.md`; `AGENTS.md`; `ghostboard/src/apprt/termsurf.zig`; `roamium/src/dispatch.rs`                                                                                 | No            | No direct Ghostboard GUI gap; the historical architecture was superseded.                                        | No action beyond retaining the architecture-history context.                                                   | The issue is architectural planning evidence, not a current parity requirement.                     |
| `0401-chromium-feasibility`    | B     | Chromium feasibility                      | Direct Chromium embedding is feasible but hard; frame delivery, input, and build integration need explicit ownership.                            | Roamium/libtermsurf_chromium owns the Chromium side; Ghostboard consumes protocol and CALayerHost messages rather than implementing Content API embedding.                      | `issues/0401-chromium-feasibility/README.md`; `roamium/src/dispatch.rs`; `ghostboard/src/apprt/termsurf.zig`; `proto/termsurf.proto`                                                            | No            | No Ghostboard-owned bug; failures here would be Roamium/Chromium issues unless the GUI drops protocol messages.  | No Ghostboard follow-up from this row.                                                                         | Feasibility research is superseded by the current engine split.                                     |
| `0401-programming-language`    | B     | Process and language boundaries           | The protocol boundary matters more than any single implementation language.                                                                      | Current Ghostboard uses Zig/Swift for GUI, Rust/C++ for Roamium, and protobuf/socket IPC; that preserves the language-boundary lesson.                                          | `issues/0401-programming-language/README.md`; `ghostboard/src/apprt/termsurf.zig`; `roamium/src/ipc.rs`; `webtui/src/ipc.rs`                                                                    | No            | No direct gap; architecture has changed language choices but not the protocol split.                             | No action.                                                                                                     | Historical recommendation was adapted rather than ported verbatim.                                  |
| `0402-wezterm-vs-alacritty`    | B     | Terminal choice                           | Terminal selection was a strategic choice tied to rendering, input, and integration cost.                                                        | Current restored Ghostboard deliberately uses Ghostty lineage, while Wezboard remains the mature reference; the row does not identify a missing feature.                        | `issues/0402-wezterm-vs-alacritty/README.md`; `issues/0417-ghostty-vs-wezterm/README.md`; `ghostboard/`; `wezboard/`                                                                            | No            | No app behavior risk.                                                                                            | No action.                                                                                                     | The terminal-choice decision was superseded by later Ghostty and Wezboard work.                     |
| `0403-swift-rust-cpp`          | B     | Cross-language prototype                  | Swift/AppKit, Rust, and C++ processes can coordinate rendering through IPC while preserving ownership boundaries.                                | Current Ghostboard still crosses Swift/Zig/Rust/C++ boundaries, but the active IPC is socket/protobuf and CALayerHost rather than the prototype XPC/IOSurface path.             | `issues/0403-swift-rust-cpp/README.md`; `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`; `ghostboard/src/apprt/termsurf.zig`; `proto/termsurf.proto`                   | No            | No current gap; the architectural lesson is already represented.                                                 | No action.                                                                                                     | Prototype mechanics are obsolete, but the boundary lesson remains satisfied.                        |
| `0404-terminal-emulator`       | B     | Terminal emulator selection               | Ghostty and WezTerm were the relevant terminal bases; feature inheritance reduces GUI scope.                                                     | Restored Ghostboard is a Ghostty-derived app and Wezboard remains the mature Rust reference. The row does not add a concrete missing feature.                                   | `issues/0404-terminal-emulator/README.md`; `ghostboard/`; `wezboard/`                                                                                                                           | No            | No direct parity risk.                                                                                           | No action.                                                                                                     | Selection research is historical context.                                                           |
| `0405-architecture-comparison` | B     | Architecture choice                       | A terminal fork with an out-of-process browser avoids rebuilding terminal features from scratch.                                                 | Current restored Ghostboard matches this model: Ghostty-derived GUI plus Roamium browser process over protocol messages.                                                        | `issues/0405-architecture-comparison/README.md`; `ghostboard/src/apprt/termsurf.zig`; `roamium/src/main.rs`                                                                                     | No            | No direct gap; current architecture follows the winning option.                                                  | No action.                                                                                                     | Historical architecture recommendation remains aligned.                                             |
| `0406-chromium`                | B     | Profile isolation research                | Multiple profiles are a Chromium/CEF architecture concern; one-process-per-profile remains a useful product boundary.                            | Ghostboard tracks servers by browser/profile and can reuse attachable servers by profile, but runtime multi-profile proof is still broader than this row alone.                 | `issues/0406-chromium/README.md`; `ghostboard/src/apprt/termsurf.zig`; `AGENTS.md`                                                                                                              | Maybe         | If the profile/server mapping is wrong, users could get incorrect profile isolation or reuse.                    | Carry into a focused profile-isolation and server-reuse regression issue.                                      | The old CEF question is closed, but profile isolation remains a product invariant.                  |
| `0407-chromium-poc`            | B     | Chromium PoC                              | In-process multi-WebContents rendering hit hidden-view throttling; profile isolation was proven but performance was not.                         | Current Roamium/CALayerHost architecture avoids the old in-process hidden-view path, so this is not a Ghostboard bug by itself.                                                 | `issues/0407-chromium-poc/README.md`; `roamium/src/dispatch.rs`; `ghostboard/src/apprt/termsurf.zig`                                                                                            | No            | No current GUI gap; old throttling path is obsolete.                                                             | No action beyond profile tests tracked elsewhere.                                                              | The PoC explains why later architecture moved away from the failed path.                            |
| `0408-two-profiles`            | B     | Electron patch research                   | Full Electron patch adoption was explored to solve 60fps profile rendering.                                                                      | Current architecture does not depend on Electron patches; Roamium and CALayerHost replace that route.                                                                           | `issues/0408-two-profiles/README.md`; `roamium/`; `ghostboard/src/apprt/termsurf.zig`                                                                                                           | No            | No Ghostboard-owned risk.                                                                                        | No action.                                                                                                     | Abandoned patch strategy is not a parity requirement.                                               |
| `0409-electron-patch`          | B     | Chromium patch workflow                   | Electron's full patch set was not buildable in isolation; minimal targeted patches are safer.                                                    | No current Ghostboard dependency on Electron patches exists. Chromium branch workflow is outside this audit row.                                                                | `issues/0409-electron-patch/README.md`; `chromium/README.md`; `roamium/`                                                                                                                        | No            | No GUI feature risk.                                                                                             | No Ghostboard action.                                                                                          | Build-strategy lesson is historical and Chromium-owned.                                             |
| `0410-partial-electron`        | B     | Throttling patch experiment               | Partial Electron throttling patches built but did not solve the rendering problem.                                                               | Current Ghostboard uses CALayerHost and protocol routing, not this throttling-bypass path.                                                                                      | `issues/0410-partial-electron/README.md`; `ghostboard/src/apprt/termsurf.zig`; `roamium/src/dispatch.rs`                                                                                        | No            | No direct risk.                                                                                                  | No action.                                                                                                     | Failed implementation path is obsolete.                                                             |
| `0410-two-profiles-2`          | B     | Hidden-view race research                 | The root issue was a visibility/race problem in the old multi-WebContents approach, not missing Electron patches.                                | Current architecture avoids that approach, but its lesson still supports testing multi-pane/profile runtime behavior under load.                                                | `issues/0410-two-profiles-2/README.md`; `ghostboard/src/apprt/termsurf.zig`; `issues/0809-ghostboard-viewport-geometry/README.md`                                                               | Maybe         | A different race could appear in current server attach, CAContext, or pane routing paths under multi-pane usage. | Include multi-pane/profile startup and reuse in a later focused regression matrix.                             | The exact race is obsolete; the concurrent-startup risk remains plausible.                          |
| `0411-two-profiles-3`          | B     | No-Electron fallback                      | The old two-profile route still failed without isolating the cause, so the next safe step was a simpler one-profile baseline.                    | Current implementation already moved to process/profile separation and protocol routing; no direct GUI gap is proven.                                                           | `issues/0411-two-profiles-3/README.md`; `ghostboard/src/apprt/termsurf.zig`; `roamium/src/dispatch.rs`                                                                                          | No            | No immediate risk beyond the profile matrix already captured from related rows.                                  | No separate action.                                                                                            | Historical dead end was superseded by later ts5 and current architecture.                           |
| `0412-one-profile`             | B     | Baseline isolation                        | Establish a one-profile 60fps baseline before scaling complexity.                                                                                | Current single-pane/single-browser geometry was recently verified in Issue 809, and current Ghostboard has normal CAContext handling.                                           | `issues/0412-one-profile/README.md`; `issues/0809-ghostboard-viewport-geometry/README.md`; `ghostboard/src/apprt/termsurf.zig`                                                                  | No            | No direct gap.                                                                                                   | No action.                                                                                                     | Baseline lesson is covered by later geometry verification.                                          |
| `0413-one-profile-2`           | B     | Two-profile iteration                     | Adding a second BrowserContext/WebContents inside one process exposed lifecycle and visibility limits.                                           | Current one-process-per-profile design avoids that specific failure, but still needs explicit profile isolation proof.                                                          | `issues/0413-one-profile-2/README.md`; `AGENTS.md`; `ghostboard/src/apprt/termsurf.zig`; `roamium/src/dispatch.rs`                                                                              | Maybe         | Incorrect profile-to-server mapping would compromise isolation or route tabs to the wrong server.                | Fold into profile-isolation/server-reuse regression coverage.                                                  | The old one-process experiment is obsolete; the product invariant persists.                         |
| `0414-two-profiles-xpc`        | B     | Multi-process profile rendering           | Two profile servers and a receiver can render at 60fps when frame routing, lifetime, Retina sizing, and color space are handled carefully.       | Current Ghostboard has server/profile state, CAContext presentation, and Issue 809 geometry evidence, but profile reuse and multi-profile runtime proof remain only partial.    | `issues/0414-two-profiles-xpc/README.md`; `ghostboard/src/apprt/termsurf.zig`; `issues/0809-ghostboard-viewport-geometry/README.md`                                                             | Maybe         | Multi-profile pane combinations could misroute CAContext, resize, focus, or lifecycle messages.                  | Add a focused multi-profile/multi-pane runtime proof with screenshots/logs.                                    | XPC/IOSurface is obsolete; multi-profile routing requirements remain relevant.                      |
| `0415-swift-receiver`          | B     | Swift/AppKit receiver                     | Swift/AppKit can receive and display externally produced browser surfaces without a heavy copy path.                                             | Current Swift/AppKit SurfaceView owns overlay layers and calls into Zig for event forwarding; CALayerHost replaces the old receiver app.                                        | `issues/0415-swift-receiver/README.md`; `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`; `ghostboard/src/apprt/termsurf.zig`                                           | No            | No direct gap.                                                                                                   | No action.                                                                                                     | Prototype implementation is superseded, but the current AppKit bridge satisfies the durable lesson. |
| `0416-rust-receiver`           | B     | Rust receiver alternative                 | A Rust/wgpu receiver was viable after launch/invocation fixes, but it added unsafe FFI and rendering-loop complexity.                            | Current restored Ghostboard does not use the Rust receiver design.                                                                                                              | `issues/0416-rust-receiver/README.md`; `ghostboard/`; `wezboard/`                                                                                                                               | No            | No Ghostboard GUI risk.                                                                                          | No action.                                                                                                     | Alternative receiver architecture is historical.                                                    |
| `0417-ghostty-vs-wezterm`      | B     | Terminal fork decision                    | Ghostty was selected for integration depth while WezTerm retained advantages around maturity and cross-platform behavior.                        | Current work intentionally restored Ghostboard from Ghostty while using Wezboard as parity reference. No missing feature is created by the selection itself.                    | `issues/0417-ghostty-vs-wezterm/README.md`; `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/README.md`; `issues/0810-ghostboard-preventive-parity-audit/README.md`                          | No            | No direct risk.                                                                                                  | No action.                                                                                                     | This validates why a Ghostboard parity audit exists.                                                |
| `0418-repo-restructure`        | B     | Repository structure                      | Chromium, vendor repos, and Ghostty-derived code need clear workspace boundaries and documented paths.                                           | Current repo has new `ghostboard/`, `wezboard/`, `roamium/`, `chromium/`, and script layout, but restored Ghostboard build/install workflow gaps were already found in Batch C. | `issues/0418-repo-restructure/README.md`; `AGENTS.md`; `issues/0810-ghostboard-preventive-parity-audit/10-batch-c-product-hardening.md`                                                         | Maybe         | Path drift can cause build instructions, scripts, or issue references to launch old binaries.                    | Carry forward the existing build/install workflow follow-up; do not open a duplicate from this row.            | Restructure history reinforces a previously recorded workflow risk.                                 |
| `0500-rename`                  | B     | App rename and identity                   | Renaming a Ghostty fork requires app bundle, CLI text, config paths, about text, icon, build system, and selective upstream names.               | Current restored Ghostboard is intentionally not fully product-renamed in all places; config and app identity parity remain plausible user-visible gaps.                        | `issues/0500-rename/README.md`; `issues/0810-ghostboard-preventive-parity-audit/10-batch-c-product-hardening.md`; `ghostboard/`                                                                 | Maybe         | Users may see wrong names, read wrong config paths, or launch the wrong installed binary.                        | Include identity/config/build naming in a focused Ghostboard hardening issue.                                  | Historical rename checklist maps directly to current restored-fork polish work.                     |
| `0501-two-profiles`            | B     | Two profile baseline                      | Two profile servers must render independently in one app session.                                                                                | Current Ghostboard has profile-keyed state and attachable server lookup, but Issue 810 has not yet proven independent profile behavior end to end.                              | `issues/0501-two-profiles/README.md`; `ghostboard/src/apprt/termsurf.zig`; `proto/termsurf.proto`                                                                                               | Maybe         | Profile leakage or wrong CAContext routing would be severe for real browsing.                                    | Add multi-profile side-by-side proof to follow-up list.                                                        | Historical feature remains a current product requirement.                                           |
| `0502-attach-delay`            | B     | Browser startup timing                    | Browser attachment should be event-driven; arbitrary startup delays hide race conditions.                                                        | Current `handleSetOverlay` can spawn or attach servers and has attachable-server lookup, but startup race coverage is not proven beyond existing geometry runs.                 | `issues/0502-attach-delay/README.md`; `ghostboard/src/apprt/termsurf.zig`; `roamium/src/dispatch.rs`                                                                                            | Maybe         | Slow startup or reused server timing could produce blank overlays or lost first resize/navigation.               | Include cold-start, warm-attach, and immediate resize in a later regression matrix.                            | Old XPC timing fix is obsolete; event-driven startup remains relevant.                              |
| `0503-one-two-three`           | B     | Scaling from one to many                  | One profile, two profiles, multiple tabs, and generic server lifecycle should all work together.                                                 | Current Ghostboard contains pane, server, and tab lookup state, but this combined matrix has not been proven in Issue 810.                                                      | `issues/0503-one-two-three/README.md`; `ghostboard/src/apprt/termsurf.zig`; `webtui/src/main.rs`                                                                                                | Maybe         | Combined multi-profile/multi-tab lifecycle bugs could drop overlays or leave stale servers.                      | Add the one/two/three-profile matrix to follow-up regression coverage.                                         | Historical scaling milestone remains a product invariant.                                           |
| `0504-web-tui`                 | B     | Web TUI foundations                       | The TUI owns browser chrome and reports viewport coordinates; the GUI must make it discover the socket and honor its messages.                   | Current webtui and Ghostboard use `TERMSURF_SOCKET`, `Hello`, `SetOverlay`, resize, and mode messages, but `HelloReply` content is already known incomplete.                    | `issues/0504-web-tui/README.md`; `webtui/src/main.rs`; `webtui/src/ipc.rs`; `ghostboard/src/apprt/termsurf.zig`                                                                                 | Maybe         | Missing default browser/homepage data can degrade webtui startup and config parity.                              | Reuse the existing `HelloReply` follow-up from Experiments 2 and 10.                                           | Web TUI itself is current; the incomplete GUI reply is a current overlap.                           |
| `0505-pink-texture`            | B     | Overlay geometry demo                     | The GUI must place browser overlays at exact pane pixel coordinates and clean them up on resize/exit.                                            | Issue 809 directly verified Ghostboard viewport geometry; current SurfaceView and Zig overlay paths implement CALayerHost positioning.                                          | `issues/0505-pink-texture/README.md`; `issues/0809-ghostboard-viewport-geometry/README.md`; `ghostboard/src/apprt/termsurf.zig`                                                                 | No            | No new gap from this row.                                                                                        | No action.                                                                                                     | Later automated geometry evidence is stronger than the old pink-texture demo.                       |
| `0506-xpc-gateway`             | B     | Launch and environment bridge             | The GUI must expose a discoverable IPC endpoint so TUIs can connect from normal app launches.                                                    | XPC gateway is obsolete, but the current equivalent is `TERMSURF_SOCKET`; prior Batch C already found build/install/default launch workflow risk.                               | `issues/0506-xpc-gateway/README.md`; `ghostboard/src/apprt/termsurf.zig`; `webtui/src/ipc.rs`; `issues/0810-ghostboard-preventive-parity-audit/10-batch-c-product-hardening.md`                 | Maybe         | If normal launches do not set socket env correctly, `web` cannot connect reliably.                               | Carry into the build/install/launch workflow follow-up.                                                        | XPC is obsolete; launch discoverability remains relevant.                                           |
| `0507-chromium`                | B     | End-to-end Chromium surface               | Full browser rendering can work at 60fps, but surface lifetime must be correct.                                                                  | Current CALayerHost path avoids per-frame IOSurface ownership, and Issue 809 provides visual geometry evidence.                                                                 | `issues/0507-chromium/README.md`; `ghostboard/src/apprt/termsurf.zig`; `roamium/src/dispatch.rs`; `issues/0809-ghostboard-viewport-geometry/README.md`                                          | No            | No current GUI gap.                                                                                              | No action.                                                                                                     | The old lifetime failure is not part of current CALayerHost design.                                 |
| `0508-checkerboard`            | B     | Surface lifetime and alignment            | IOSurface lifetime and row alignment must be handled when copying surfaces manually.                                                             | Current Ghostboard embeds browser layers through CAContext/CALayerHost rather than copying checkerboard IOSurfaces.                                                             | `issues/0508-checkerboard/README.md`; `ghostboard/src/apprt/termsurf.zig`; `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`                                             | No            | No current risk.                                                                                                 | No action.                                                                                                     | Obsolete rendering mechanism.                                                                       |
| `0509-chromium`                | B     | Frame delivery, resize, and server storms | Server spawning must be deduplicated, resize must reach the engine, and rendering should recover after navigation.                               | Current Ghostboard has browser/profile server lookup, resize forwarding, and CAContext handling, but named browser launch remains incomplete from Experiment 3.                 | `issues/0509-chromium/README.md`; `ghostboard/src/apprt/termsurf.zig`; `issues/0810-ghostboard-preventive-parity-audit/03-direct-browser-paths.md`                                              | Maybe         | Wrong spawn/reuse behavior can create duplicate servers or fail to launch named browsers.                        | Reuse the direct-browser/default-browser follow-up already identified.                                         | The old frame-copy fixes are obsolete; launch/reuse lessons remain relevant.                        |
| `0510-two-profiles`            | B     | Concurrent two-profile routing            | Profile propagation and serialized peer connection handling were needed for stable side-by-side profiles.                                        | Current Ghostboard uses socket threads and shared state instead of XPC peer connections, but two-profile runtime behavior still lacks a dedicated proof.                        | `issues/0510-two-profiles/README.md`; `ghostboard/src/apprt/termsurf.zig`; `proto/termsurf.proto`                                                                                               | Maybe         | Concurrent profile startup or routing bugs could affect real multi-pane browsing.                                | Add two-profile side-by-side proof to follow-up coverage.                                                      | XPC queue details are obsolete; concurrency and routing remain relevant.                            |
| `0511-three-profiles`          | B     | Server reuse and tab routing              | Profile-keyed server reuse, pane-to-tab mapping, resize routing, and lifecycle cleanup are required when multiple panes share or split profiles. | Current Ghostboard has `ServerState`, `PaneState`, and `TabLookupState`, but the full three-pane reuse/lifecycle matrix has not been proven.                                    | `issues/0511-three-profiles/README.md`; `ghostboard/src/apprt/termsurf.zig`; `webtui/src/main.rs`                                                                                               | Maybe         | Misrouting or stale lifecycle state can leave panes blank or terminate the wrong server.                         | Add a three-pane lifecycle proof with close/reopen cases.                                                      | Current code has the shape, but the historical matrix should become a regression guard.             |
| `0512-vsync`                   | B     | Render pacing                             | Copy-based IOSurface rendering needed redraw/backpressure fixes; oversampling improved old visual smoothness.                                    | Current CALayerHost path is compositor-owned and does not use the old copy loop, while input latency/performance can still be measured separately.                              | `issues/0512-vsync/README.md`; `ghostboard/src/apprt/termsurf.zig`; `issues/0809-ghostboard-viewport-geometry/README.md`                                                                        | No            | No specific parity gap.                                                                                          | No action unless later runtime testing shows visible latency.                                                  | Old vsync mechanism does not apply directly to CALayerHost.                                         |
| `0513-ctrl-esc`                | B     | Mode and focus sync                       | The window must know browse/edit mode and forward focus changes so Ctrl+Esc and input routing behave correctly.                                  | Current Ghostboard handles `ModeChanged`, sends `FocusChanged`, and AppKit refuses to forward Esc as browser input.                                                             | `issues/0513-ctrl-esc/README.md`; `ghostboard/src/apprt/termsurf.zig`; `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`; `webtui/src/main.rs`                           | No            | No current gap identified by static audit.                                                                       | Keep covered by input regression tests, but no new issue from this row.                                        | Current implementation carries the historical mode/focus lesson.                                    |
| `0514-mouse`                   | B     | Mouse and cursor feedback                 | Click, scroll, hover, and cursor appearance all matter for a usable embedded browser.                                                            | Ghostboard forwards click/move/scroll events, and Roamium can emit `CursorChanged`; static audit still finds no Ghostboard dispatch path for `CursorChanged`.                   | `issues/0514-mouse/README.md`; `ghostboard/src/apprt/termsurf.zig`; `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`; `roamium/src/dispatch.rs`; `proto/termsurf.proto` | Highly likely | Cursor shape may not update over links/text, making the browser feel broken despite mouse forwarding.            | Open or include a focused cursor-feedback parity issue; also add mouse hover/click/scroll regression coverage. | The row maps directly to an existing high-confidence Issue 810 finding.                             |
| `0515-drag`                    | B     | Text selection and drag lifecycle         | Drag selection needed correct focus lifecycle, drag event delivery, and suppression of terminal selection during browser drag.                   | Current Ghostboard forwards mouse down/up/move and focus changes, but no Issue 810 evidence proves full drag selection against Roastty/Ghostboard after the restoration.        | `issues/0515-drag/README.md`; `ghostboard/src/apprt/termsurf.zig`; `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`                                                     | Maybe         | Text selection could fail, select terminal text instead of page text, or lose focus during drag.                 | Add drag-to-select and terminal-selection-suppression checks to the input regression matrix.                   | Code shape exists; runtime proof is missing.                                                        |

### Ranked Findings

`Highly likely`:

- `0514-mouse`: cursor feedback is still likely missing from Ghostboard. Roamium
  emits `CursorChanged` and the protocol defines it, but the Ghostboard
  dispatcher does not handle it beyond naming the message for logs.

`Maybe`:

- Profile isolation, server reuse, and multi-pane lifecycle need focused proof:
  `0406-chromium`, `0410-two-profiles-2`, `0413-one-profile-2`,
  `0414-two-profiles-xpc`, `0501-two-profiles`, `0503-one-two-three`,
  `0510-two-profiles`, and `0511-three-profiles`.
- Startup, attach, named/default browser, and socket discovery need focused
  workflow proof: `0418-repo-restructure`, `0500-rename`, `0502-attach-delay`,
  `0504-web-tui`, `0506-xpc-gateway`, and `0509-chromium`.
- Input coverage should include drag selection and browser mouse behavior:
  `0515-drag`, plus the click/scroll/hover portions of `0514-mouse`.

`No`:

- Most early Chromium, Electron patch, XPC receiver, and copy-based IOSurface
  rows are obsolete implementation routes. They are not current Ghostboard bugs
  unless a later focused test proves the same user-visible invariant regressed
  in the socket/protobuf, Roamium, or CALayerHost architecture.

### Verification Performed

- Confirmed Batch B's authoritative range from Experiment 4 is `0400`-`0515`
  with count 37.
- Audited each Batch B folder exactly once by folder slug, preserving duplicate
  numeric prefixes `0401` and `0410`.
- Rechecked current Ghostboard/protocol evidence for the overlapping code paths:
  `sendHelloReply`, `handleSetOverlay`, `serverRegisterProfile`,
  `findAttachableServerByProfile`, `handleModeChanged`, `sendFocusChanged`,
  `forwardKeyEvent`, `forwardMouseEvent`, `forwardMouseMove`, `handleCaContext`,
  `CursorChanged`, and the Swift AppKit forwarding bridge.
- Confirmed the next historical audit slice is Batch A, range `0001`-`0350`,
  count 73, from Experiment 4.

## Conclusion

Batch B does not add a brand-new class of high-confidence Ghostboard breakage,
but it reinforces three follow-up themes already emerging from Issue 810:

1. Cursor feedback is the clearest high-confidence missing GUI behavior.
2. Multi-profile/multi-pane lifecycle behavior has credible code shape, but the
   old issues show it needs a focused runtime matrix rather than static
   confidence.
3. Startup and launch workflow risks should be tracked with the existing
   direct-browser/default-browser and `HelloReply` findings.

The next audit slice should be Batch A: early prototypes and architecture, range
`0001`-`0350`.

## Completion Review

Mendel reviewed the completed experiment with fresh context and approved it with
no findings.

The review verified that the diff is audit-only, the README marks Experiment 11
as `Pass`, the Batch B result has exactly thirty-seven rows matching Experiment
4, duplicate numeric prefixes `0401` and `0410` are separate rows, the required
table schema is present, obsolete Electron/XPC/IOSurface rows are mapped as
historical rather than current requirements, cursor feedback remains
high-confidence, profile reuse, launch workflow, `HelloReply`, and drag
selection remain `Maybe`, the next slice is Batch A `0001`-`0350`,
`git diff --check` passes, and the result commit had not yet been made.
