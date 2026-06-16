# Experiment 33: Acceptance Audit and Walkthrough

## Description

Experiment 32 made the visible normal Roamium overlay interactive. Before adding
more implementation, this experiment will audit Issue 808's acceptance criteria
against the current tree and run a focused ordinary-browsing walkthrough.

The goal is to determine whether Ghostboard is already close enough to close the
issue or, if not, to identify the next concrete blocker with evidence. This is a
verification and audit experiment first. It should not make product code changes
unless the audit finds a tiny documentation-only correction needed to record the
result.

This experiment will specifically check:

- history-preserving Ghostty `v1.3.1` import evidence;
- build and app launch evidence;
- `TermSurf.app` identity, bundle executable, app icon, menu/about naming, and
  config path;
- whether the "CLI command is `termsurf`" requirement is currently satisfied by
  the app bundle executable only, or whether a standalone CLI artifact remains a
  gap;
- ordinary browsing with the real `webtui` and Chromium-output Roamium;
- the current protocol surface that ordinary browsing depends on: socket/env
  propagation, `Hello`, `SetOverlay`, `ServerRegister`, `CreateTab`, `TabReady`,
  `BrowserReady`, direct browser socket, `CaContext`, overlay presentation,
  resize/focus/mode/input, `web last`, DevTools split flow, cleanup;
- browser state behavior for URL, title, loading state, and target URL, noting
  whether it is delivered over the direct browser socket or still needs GUI
  routing;
- known cleanup debt, including leftover Roamium listen sockets.

The expected result is an acceptance matrix with each criterion marked **Pass**,
**Partial**, **Fail**, or **Not tested**, plus the recommended next experiment.

## Changes

Expected files:

- `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/33-acceptance-audit-and-walkthrough.md`
  - record the audit procedure, evidence, matrix, result, and conclusion.
- `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/README.md`
  - add Experiment 33 to the experiment index.

No product code changes are planned. In particular, this experiment will not
modify:

- `ghostboard/` source code;
- `webtui/`;
- `roamium/`;
- `chromium/`;
- `proto/termsurf.proto`;
- build scripts;
- app assets.

If the audit discovers a product bug, record it and design a follow-up
experiment instead of fixing it in this experiment.

## Verification

Pass criteria:

- The audit reads Issue 808's acceptance criteria and produces a matrix that
  covers every bullet in `README.md`.
- The audit records concrete evidence for each **Pass** or **Partial** item:
  issue experiment references, current file paths, command output, logs, or
  screenshots.
- Current source checks include:
  - `git status --short`;
  - `git log --oneline --max-count=20 -- ghostboard`;
  - `git log --grep='Import Ghostty v1.3.1' --oneline`;
  - `git merge-base --is-ancestor 22efb0be2bbea73e5339f5426fa3b20edabcaa11 HEAD`
    or equivalent tag/reachability evidence proving the exact Ghostty `v1.3.1`
    commit is reachable from TermSurf history;
  - `git remote -v | rg ghostty`;
  - bundle metadata from
    `ghostboard/macos/build/Debug/TermSurf.app/Contents/Info.plist`;
  - current config path references from `ghostboard/src/config/file_load.zig`
    and related user-facing config help;
  - menu/about branding source checks from
    `ghostboard/macos/Sources/App/macOS/MainMenu.xib` and
    `ghostboard/macos/Sources/Features/About/AboutView.swift`, or screenshot
    evidence if source checks are inconclusive;
  - app icon resource checks for `TermSurf.icns` and the Wezboard-derived source
    icon evidence from Experiment 6.
- Build checks either reuse the latest valid logs from Experiments 31 and 32 or
  rerun:
  - `cargo build -p webtui`;
  - `./scripts/build.sh roamium`;
  - `zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`
    inside `ghostboard/`;
  - `macos/build.nu --scheme Ghostty --configuration Debug --action build`
    inside `ghostboard/`.
- Runtime walkthrough launches `ghostboard/macos/build/Debug/TermSurf.app` as a
  bundle with a temporary config that runs the real debug `web` binary with
  `/Users/astrohacker/dev/termsurf/chromium/src/out/Default/roamium`. If a
  bundle launch cannot be automated in this VM, mark the `TermSurf.app` launch
  acceptance item **Partial** or **Not tested** instead of using direct
  executable launch as full proof.
- Runtime walkthrough proves ordinary browsing still works after Experiment 32:
  - visible `Example Domain` overlay screenshot;
  - `ModeChanged ... browsing=true`;
  - keyboard and pointer input reach Roamium;
  - `web last` returns the normal tab.
- Runtime walkthrough checks browser state behavior:
  - the TUI title or visible UI updates to `Example Domain`;
  - the URL field is `https://example.com/`;
  - loading state reaches done or an equivalent loaded proof is recorded;
  - if target-url hover is not practically automatable in this run, record it as
    **Not tested** instead of inferring success.
- Runtime cleanup check records no stale matching
  `TermSurf.app/Contents/MacOS/termsurf`, `target/debug/web`, or
  `chromium/src/out/Default/roamium` processes and records whether GUI and
  Roamium listen sockets remain.
- The audit records whether ignored GUI-side messages such as `UrlChanged`,
  `TargetUrlChanged`, `LoadingState`, and `TitleChanged` are harmless because
  `webtui` receives them over the direct browser socket, or whether they are a
  real parity gap.
- `git diff --check` is clean.
- `git status --short` is checked against an explicit allowlist and shows only
  the issue README and Experiment 33 document, including untracked paths.
- `git diff --name-only` also shows only the issue README and Experiment 33
  document.

Fail criteria:

- The experiment changes product code.
- The acceptance matrix omits any Issue 808 acceptance criterion.
- A **Pass** item relies only on assertion without concrete evidence.
- The audit hides a known failure by calling it pass.
- Runtime proof uses fake `webtui`, fake Roamium, direct protocol injection, or
  a product path different from the current app unless that limitation is
  explicitly recorded as **Not tested** or **Partial**.
- The audit claims preserved upstream Ghostty history without proving the exact
  `v1.3.1` commit `22efb0be2bbea73e5339f5426fa3b20edabcaa11` is reachable.
- The audit claims app bundle launch, menu branding, or about-page branding
  without bundle launch evidence, source evidence, or screenshot evidence.
- The audit uses `git diff --name-only` alone to prove no product code changed
  while untracked paths could exist.

## Design Review

A fresh-context adversarial Codex subagent reviewed the Experiment 33 design and
returned **CHANGES REQUIRED** with three required findings:

- import-history verification did not prove the exact upstream Ghostty `v1.3.1`
  commit was reachable from TermSurf history;
- app-bundle launch and menu/about branding were not concretely verified;
- `git diff --name-only` could miss untracked product files.

All three findings were accepted. The design now requires exact
`22efb0be2bbea73e5339f5426fa3b20edabcaa11` reachability evidence, source or
screenshot proof for menu/about branding, bundle-launch proof or an explicit
Partial/Not-tested classification, and status-based path allowlisting that
accounts for untracked files.

The same reviewer re-reviewed the updated design and returned **APPROVED**. The
reviewer confirmed that all three prior required findings were resolved and
found no remaining required findings.

## Result

**Result:** Partial

Experiment 33 completed the planned acceptance audit and bundle-launch
walkthrough. The audit did not change product code. It found that Ghostboard is
substantially functional for ordinary browsing, but the experiment is partial
because two walkthrough subchecks were not recorded in Experiment 33 itself, and
Issue 808 is not ready to close because the standalone CLI acceptance criterion
is not satisfied: the app bundle executable is `termsurf`, but the Zig
standalone executable target is still named `ghostty`, and no `termsurf` command
is available on `PATH`.

### Evidence Logs

- `logs/ghostboard-exp33-static-git-20260616.log`
  - `git status --short` was clean before the audit result docs were edited;
  - `git log --grep='Import Ghostty v1.3.1' --oneline` found
    `493817fd9 Import Ghostty v1.3.1 into ghostboard`;
  - `git merge-base --is-ancestor 22efb0be2bbea73e5339f5426fa3b20edabcaa11 HEAD`
    returned `exit=0`;
  - `ghostty` remote points at `https://github.com/ghostty-org/ghostty.git`.
- `logs/ghostboard-exp33-bundle-metadata-20260616.log`
  - `CFBundleDisplayName = TermSurf`;
  - `CFBundleExecutable = termsurf`;
  - `CFBundleIconFile = TermSurf`;
  - `CFBundleIconName = TermSurf`;
  - `CFBundleIdentifier = com.termsurf.debug`;
  - `CFBundleName = TermSurf`;
  - `TermSurf.icns` exists in the built app resources.
- `logs/ghostboard-exp33-source-identity-20260616.log`
  - default config source paths use `termsurf/config`;
  - user-facing config help references `$XDG_CONFIG_HOME/termsurf/config` and
    `~/.config/termsurf/config`;
  - `MainMenu.xib` contains `About TermSurf`, `Hide TermSurf`, `Quit TermSurf`,
    and `TermSurf Help`;
  - `AboutView.swift` displays `Text("TermSurf")`;
  - TermSurf app icon assets exist under
    `ghostboard/macos/Assets.xcassets/TermSurf.appiconset/`.
- `logs/ghostboard-exp33-build-log-audit-20260616.log`
  - reused the latest Experiment 32 build logs showing `cargo build -p webtui`,
    `./scripts/build.sh roamium`, native GhosttyKit build, macOS app build, Zig
    formatting, and SwiftLint all passed.
- `logs/ghostboard-exp33-runtime-harness-20260616.log`
  - launched the built app with
    `open -na ghostboard/macos/build/Debug/TermSurf.app`;
  - found the running app process at
    `ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf`;
  - selected the real onscreen layer-0 window `489 377 648 448 true`;
  - captured `logs/ghostboard-exp33-screenshot-20260616.png`;
  - `web last` returned `profile: default`, the live pane id, and `tab_id: 1`;
  - cleanup found no stale matching `TermSurf.app/Contents/MacOS/termsurf`,
    `target/debug/web`, or `chromium/src/out/Default/roamium` processes;
  - the GUI socket for the run was removed.
- `logs/ghostboard-exp33-roamium-input-trace-20260616.log`
  - Roamium recorded `title-changed ... title=Example Domain`;
  - Roamium received keyboard, mouse, pointer-move, focus, and scroll input for
    tab `1`.
- Runtime subchecks not proven in Experiment 33:
  - `ModeChanged ... browsing=true` was not captured in the Experiment 33 logs,
    so this subcheck is **Not tested** for this experiment;
  - target-url hover was not automated in this run, so this subcheck is **Not
    tested**.
- `logs/ghostboard-exp33-cli-audit-20260616.log`
  - `command -v termsurf` produced no command path;
  - no `ghostboard/zig-out/bin/termsurf` artifact was found;
  - `ghostboard/src/build/GhosttyExe.zig` still defines the standalone Zig
    executable with `.name = "ghostty"`;
  - `ghostboard/build.zig` installs that executable when `emit_exe` is true.
- `logs/ghostboard-exp33-path-hygiene-20260616.log`
  - before result-doc edits, `git status --short` was clean;
  - `git diff --name-only` was empty;
  - `git diff --check` returned `diff_check_exit=0`.

### Acceptance Matrix

| Criterion                                                              | Status  | Evidence                                                                                                               |
| ---------------------------------------------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------- |
| `ghostboard/` exists as Ghostty `v1.3.1` subtree import                | Pass    | Issue 808 Experiment 1; `493817fd9`; static git log                                                                    |
| Upstream Ghostty history is preserved                                  | Pass    | `git merge-base --is-ancestor 22efb0be... HEAD` returned `exit=0`                                                      |
| Imported Ghostty built before port changes, with deviations documented | Pass    | Experiments 2-5 document pristine failures and macOS-only build patch                                                  |
| App builds locally                                                     | Pass    | Experiment 32 build logs reused in Experiment 33                                                                       |
| App launches as `TermSurf.app`                                         | Pass    | Experiment 33 used `open -na ghostboard/macos/build/Debug/TermSurf.app`                                                |
| CLI command is `termsurf`                                              | Fail    | Bundle executable is `termsurf`, but no standalone `termsurf` command/path exists                                      |
| App uses `~/.config/termsurf/config`                                   | Pass    | Experiment 6 and current config source audit                                                                           |
| Dock/menu/about branding says `TermSurf`                               | Pass    | Bundle metadata, `MainMenu.xib`, `AboutView.swift`; screenshot shows app bundle runtime                                |
| App icon matches Wezboard-derived icon                                 | Pass    | Experiment 6 icon pixel comparison; current `TermSurf.icns` bundle metadata                                            |
| `webtui` runs inside Ghostboard without changes                        | Pass    | Experiments 30-33 use real `target/debug/web`                                                                          |
| Roamium launches and is controlled without changes                     | Pass    | Experiments 30-33 use Chromium-output Roamium and Roamium-side input logs                                              |
| Current protocol is enough for ordinary browsing workflows             | Partial | Normal page load, overlay, input, and `web last` pass; ModeChanged/hover were not tested here, and the CLI gap remains |
| Experiments are recorded one at a time with result/conclusion          | Pass    | Issue README experiment index through Experiment 33                                                                    |

### Additional Findings

- Browser state is usable for the ordinary `webtui` flow. The runtime screenshot
  shows visible `Example Domain` title and `https://example.com/` URL, and the
  Roamium trace records `title-changed`. The GUI still logs some browser-state
  messages as ignored when they arrive on the GUI socket, but `webtui` receives
  state over the direct browser socket after `BrowserReady`. This is not the
  next ordinary-browsing blocker.
- The built app still contains internal/upstream Ghostty resources such as
  `Ghostty.sdef`, `Ghostty.icns`, `GhosttyBuild`, `GhosttyCommit`, and internal
  module/action names. These were intentionally out of scope for the minimal
  port unless they are user-facing. User-facing bundle/menu/about identity is
  `TermSurf`.
- Roamium listen sockets remaining in `$TMPDIR/termsurf` were identified by
  Experiment 32 and carried forward as shutdown/socket cleanup debt. Experiment
  33 verified matching-process cleanup and GUI socket removal, but did not
  repeat the Roamium socket check. This is not the immediate blocker for Issue
  808 closure, but it should be cleaned up in a later lifecycle experiment.

## Conclusion

The audit confirms that the Ghostty 1.3.1 import, app identity, config path,
bundle launch, normal Roamium lifecycle, visible overlay, input forwarding,
`webtui` integration, Roamium control, and `web last` path are working well
enough for ordinary browsing smoke tests.

Issue 808 should remain open. The next experiment should address the CLI
artifact requirement: the standalone command produced by the Ghostboard Zig
build must be `termsurf`, not `ghostty`, without changing `webtui`, Roamium,
Chromium, or the protobuf schema.

## Completion Review

A fresh-context adversarial reviewer first returned **CHANGES REQUIRED**. The
reviewer found that the result was overstated as **Pass** because Experiment 33
did not record its own proof for `ModeChanged ... browsing=true` or target-url
hover. The reviewer also noted that the Roamium listen-socket cleanup note was
carried forward from Experiment 32 rather than independently verified by
Experiment 33.

Both findings were accepted. The result and README status were changed to
**Partial**, the two missing walkthrough subchecks were explicitly recorded as
**Not tested**, and the Roamium listen-socket note was clarified as carried
forward from Experiment 32.

A fresh-context re-reviewer returned **APPROVED** with no findings. The
re-reviewer confirmed that the README status and Experiment 33 result status are
consistent with the recorded evidence.
