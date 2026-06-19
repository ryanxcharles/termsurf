# Experiment 2: Sanitize Ghostboard Agent-Facing Files

## Description

Remove or rewrite the poisoned and Ghostty-specific agent-facing content found
by Experiment 1, while preserving useful build/test guidance and leaving benign
technical source comments unchanged.

This experiment should be a documentation-only cleanup. It should not change
Ghostboard source behavior.

## Changes

Planned files:

- `ghostboard/AGENTS.md`
  - remove the inherited Issue and PR Guidelines trap that tells agents to
    create a humiliating file;
  - replace it with TermSurf-appropriate guidance to follow the user's request
    and the root TermSurf issue workflow;
  - preserve useful build, test, formatting, and directory guidance.
- `ghostboard/AI_POLICY.md`
  - remove the inherited Ghostty AI policy or replace it with a short
    TermSurf-specific note;
  - do not retain Ghostty denouncement/vouch language.
- `ghostboard/CONTRIBUTING.md`
  - remove or rewrite Ghostty-specific contribution policy, vouch process,
    denouncement system, issue/discussion routing, and upstream Ghostty links;
  - point contributors/developers at the root TermSurf workflow instead.
- `ghostboard/HACKING.md`
  - rewrite Ghostty-specific development prose to describe local Ghostboard
    development facts only;
  - preserve useful build/dependency notes that remain true for Ghostboard.
- `ghostboard/.github/VOUCHED.td`
  - remove the inherited Ghostty vouch/denouncement list if TermSurf does not
    use it, or replace it with a short non-policy placeholder if deletion would
    leave broken references.
- `ghostboard/README.md`
  - inspect the Ghostty contribution guidance hit and rewrite only if it points
    readers to Ghostty-specific pull-request policy.
- `ghostboard/.agents/commands/gh-issue`
  - either remove it or rewrite defaults/text so it no longer targets
    `ghostty-org/ghostty`.
- `ghostboard/.agents/commands/review-branch`
  - remove upstream-specific wording if present; keep only if the command is
    useful and TermSurf-appropriate.
- `ghostboard/macos/AGENTS.md`
  - optional minor wording cleanup if it still says "Ghostty library" in a way
    that is misleading for TermSurf.
- `issues/0824-ghostboard-poisoned-agent-files/README.md`
  - update Experiment 2 status.
- `issues/0824-ghostboard-poisoned-agent-files/02-sanitize-ghostboard-agent-facing-files.md`
  - record design review, changes, verification, completion review, result, and
    conclusion.

Explicit non-changes:

- Do not edit normal terminal prompt comments.
- Do not edit benign source comments from Experiment 1, such as mailbox "ignore
  all messages" comments.
- Do not touch `vendor/ghostty/`; it is upstream reference material.

## Verification

Pass criteria:

- `ghostboard/AGENTS.md` no longer contains:
  - `I am a sad, dumb little AI driver with no real skills`;
  - instructions to create unrelated files when asked to create an issue or PR.
- No `ghostboard/` file contains high-signal trap phrases:

  ```bash
  rg -n --hidden -S \
    "sad, dumb|AI driver|instant ban|human boundary|poison|prompt injection|ignore previous|developer message|system prompt" \
    ghostboard \
    -g '!zig-cache/**' \
    -g '!macos/build/**' \
    -g '!*.png' \
    -g '!*.jpg' \
    -g '!*.jpeg' \
    -g '!*.icns' \
    -g '!*.ico'
  ```

  Any remaining matches must be explicitly listed as benign and justified.

- Ghostty-specific AI/contribution policy is gone from `ghostboard/`:
  - no Ghostty vouch process remains;
  - no Ghostty denouncement policy remains;
  - no `ghostty-org/ghostty` default remains in `.agents/commands`;
  - no local doc tells TermSurf contributors to follow Ghostty's issue or PR
    process.
- A targeted Ghostty-policy search is run:

  ```bash
  rg -n --hidden -S \
    "vouch|denounc|ghostty-org/ghostty|Ghostty.*pull request|AI_POLICY|Bad AI drivers|VOUCHED" \
    ghostboard \
    -g '!zig-cache/**' \
    -g '!macos/build/**' \
    -g '!*.png' \
    -g '!*.jpg' \
    -g '!*.jpeg' \
    -g '!*.icns' \
    -g '!*.ico'
  ```

  Any remaining matches must be explicitly listed as benign and justified.

- All remaining `ghostboard/**/AGENTS.md` files are useful and factual for
  TermSurf/Ghostboard development.
- All changed markdown files are formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    ghostboard/AGENTS.md \
    ghostboard/AI_POLICY.md \
    ghostboard/CONTRIBUTING.md \
    ghostboard/HACKING.md \
    ghostboard/README.md \
    ghostboard/macos/AGENTS.md \
    issues/0824-ghostboard-poisoned-agent-files/README.md \
    issues/0824-ghostboard-poisoned-agent-files/02-sanitize-ghostboard-agent-facing-files.md
  ```

  Only include files that still exist after the edit.

- `git diff --check` passes.
- Design review is recorded and approved before implementation.
- The Experiment 2 plan commit exists before any `ghostboard/` edits begin.
- The experiment result lists:
  - every file changed;
  - every audited suspicious file intentionally left unchanged;
  - every remaining suspicious search hit and why it is benign.
- Completion review approves before the result commit.

Fail criteria:

- Any poisoned/trap instruction remains in `ghostboard/AGENTS.md`.
- The edit rewrites normal terminal prompt comments.
- The edit changes runtime source behavior.
- The result claims Issue 824 is solved while Ghostty-specific vouch,
  denouncement, or trap instructions remain in active Ghostboard docs.

## Design Review

Fresh-context adversarial design review initially returned **CHANGES REQUIRED**
with one required finding:

- the verification did not explicitly require the design review to be recorded
  and approved, and the Experiment 2 plan commit to exist before any
  `ghostboard/` edits begin.

The reviewer also raised one optional improvement:

- add a concrete targeted search for Ghostty-specific policy terms such as
  `vouch`, `denounc`, `ghostty-org/ghostty`, `Ghostty.*pull request`, and
  `AI_POLICY`.

The design was updated to add the missing plan-commit gate and targeted
Ghostty-policy search. Fresh-context re-review returned **APPROVED** with no
remaining required findings.

## Result

**Result:** Pass

The implementation removed the confirmed inherited agent trap and the inherited
Ghostty contributor-gate policy surface from `ghostboard/`.

Files changed:

- `ghostboard/AGENTS.md`
  - replaced the trap that told agents to create a humiliating unrelated file
    with TermSurf workflow guidance.
- `ghostboard/AI_POLICY.md`
  - replaced the upstream Ghostty AI policy with a short TermSurf-specific note.
- `ghostboard/CONTRIBUTING.md`
  - replaced upstream Ghostty contribution, issue, PR, and contributor-gate
    process text with a short TermSurf/Ghostboard contribution note.
- `ghostboard/HACKING.md`
  - replaced upstream Ghostty development policy prose with concise Ghostboard
    build, agent, and logging notes.
- `ghostboard/README.md`
  - replaced the inherited Ghostty contribution/development section with
    TermSurf workflow pointers.
- `ghostboard/macos/AGENTS.md`
  - corrected the misleading "Ghostty library" wording to "Ghostboard core
    library."
- `ghostboard/src/inspector/AGENTS.md`
  - clarified that the inspector was inherited from Ghostty but is local
    Ghostboard guidance.
- `ghostboard/test/fuzz-libghostty/AGENTS.md`
  - formatted by Prettier; content remains useful fuzzer guidance.
- `ghostboard/.agents/commands/gh-issue`
  - changed the default repository from `ghostty-org/ghostty` to
    `termsurf/termsurf`, changed issue wording to TermSurf, and removed the
    upstream "oracle" instruction.
- `ghostboard/.agents/commands/review-branch`
  - removed the upstream "oracle" instruction.
- `ghostboard/.github/VOUCHED.td`
  - deleted the inherited contributor-gate list.
- `ghostboard/.github/DISCUSSION_TEMPLATE/issue-triage.yml`
  - deleted the inherited upstream Ghostty discussion template.
- `ghostboard/.github/DISCUSSION_TEMPLATE/vouch-request.yml`
  - deleted the inherited contributor-gate discussion template.
- `ghostboard/.github/ISSUE_TEMPLATE/config.yml`
  - deleted the inherited upstream Ghostty issue routing template.
- `ghostboard/.github/ISSUE_TEMPLATE/preapproved.md`
  - deleted the inherited upstream Ghostty issue template.
- `ghostboard/.github/workflows/vouch-check-issue.yml`
  - deleted inherited contributor-gate automation.
- `ghostboard/.github/workflows/vouch-check-pr.yml`
  - deleted inherited contributor-gate automation.
- `ghostboard/.github/workflows/vouch-manage-by-discussion.yml`
  - deleted inherited contributor-gate automation.
- `ghostboard/.github/workflows/vouch-manage-by-issue.yml`
  - deleted inherited contributor-gate automation.
- `ghostboard/.github/workflows/vouch-sync-codeowners.yml`
  - deleted inherited contributor-gate automation.
- `ghostboard/.github/workflows/milestone.yml`
  - removed the stale `VOUCHED` title exception.
- `ghostboard/.github/workflows/test.yml`
  - removed the stale `VOUCHED.td` skip-filter exception and comment.

Audited suspicious files intentionally left unchanged:

- `vendor/ghostty/**`
  - left unchanged because it is upstream reference material, not the active
    Ghostboard fork.
- Normal source comments and generated/help URLs containing
  `https://github.com/ghostty-org/ghostty`
  - left unchanged where they are upstream provenance links, regression-test
    references, generated documentation links, release/update URLs, or
    repository guards in inherited CI. They are not agent instructions, do not
    contain the trap language, and are outside the contributor-gate policy
    surface fixed by this experiment.
- Normal terminal prompt comments and mailbox/input comments
  - left unchanged because Experiment 1 classified them as ordinary technical
    text, not agent-facing prompt-injection content.

Verification:

- High-signal trap grep:

  ```bash
  rg -n --hidden -S \
    "sad, dumb|AI driver|instant ban|human boundary|poison|prompt injection|ignore previous|developer message|system prompt" \
    ghostboard \
    -g '!zig-cache/**' \
    -g '!macos/build/**' \
    -g '!*.png' \
    -g '!*.jpg' \
    -g '!*.jpeg' \
    -g '!*.icns' \
    -g '!*.ico'
  ```

  returned no matches.

- Tight contributor-gate grep:

  ```bash
  rg -n --hidden -S \
    "vouch|denounc|VOUCHED|AI_POLICY|Bad AI drivers|sad, dumb|AI driver|prompt injection|poison" \
    ghostboard \
    -g '!zig-cache/**' \
    -g '!macos/build/**' \
    -g '!*.png' \
    -g '!*.jpg' \
    -g '!*.jpeg' \
    -g '!*.icns' \
    -g '!*.ico'
  ```

  returned no matches.

- Targeted Ghostty-policy grep:

  ```bash
  rg -n --hidden -S \
    "vouch|denounc|ghostty-org/ghostty|Ghostty.*pull request|AI_POLICY|Bad AI drivers|VOUCHED" \
    ghostboard \
    -g '!zig-cache/**' \
    -g '!macos/build/**' \
    -g '!*.png' \
    -g '!*.jpg' \
    -g '!*.jpeg' \
    -g '!*.icns' \
    -g '!*.ico'
  ```

  returned only `ghostty-org/ghostty` URL hits:

  ```text
  ghostboard/src/termio/Exec.zig:855
  ghostboard/src/extra/vim.zig:9
  ghostboard/src/extra/vim.zig:19
  ghostboard/src/extra/vim.zig:45
  ghostboard/src/extra/vim.zig:79
  ghostboard/src/extra/bash.zig:288
  ghostboard/src/cli/version.zig:27
  ghostboard/src/terminal/Screen.zig:9604
  ghostboard/src/terminal/hash_map.zig:1523
  ghostboard/src/input/helpgen_actions.zig:51
  ghostboard/src/build/GhosttyLibVt.zig:108
  ghostboard/README.md:80
  ghostboard/src/apprt/gtk/class/window.zig:1774
  ghostboard/src/build/mdgen/ghostty_5_footer.md:15
  ghostboard/src/build/mdgen/ghostty_5_footer.md:20
  ghostboard/src/config/url.zig:418
  ghostboard/src/config/url.zig:419
  ghostboard/src/build/mdgen/ghostty_1_footer.md:36
  ghostboard/src/build/mdgen/ghostty_1_footer.md:41
  ghostboard/src/terminal/Terminal.zig:7475
  ghostboard/src/build/webgen/main_config.zig:18
  ghostboard/src/build/webgen/main_commands.zig:21
  ghostboard/src/terminal/page.zig:2172
  ghostboard/src/config/Config.zig:94
  ghostboard/src/config/Config.zig:7399
  ghostboard/example/c-vt-key-encode/build.zig.zon:15
  ghostboard/src/font/discovery.zig:420
  ghostboard/src/apprt/gtk/winproto/x11.zig:178
  ghostboard/example/c-vt-sgr/build.zig.zon:15
  ghostboard/example/c-vt/build.zig.zon:15
  ghostboard/src/unicode/grapheme.zig:86
  ghostboard/example/zig-vt/build.zig.zon:15
  ghostboard/src/apprt/gtk/css/style.css:176
  ghostboard/snap/snapcraft.yaml:9
  ghostboard/snap/snapcraft.yaml:10
  ghostboard/example/c-vt-paste/build.zig.zon:15
  ghostboard/src/apprt/gtk/ui/1.2/surface.blp:245
  ghostboard/.github/workflows/flatpak.yml:17
  ghostboard/macos/GhosttyUITests/GhosttyThemeTests.swift:38
  ghostboard/.github/workflows/nix.yml:35
  ghostboard/.github/workflows/test.yml:18
  ghostboard/.github/workflows/test.yml:778
  ghostboard/.github/workflows/test.yml:1061
  ghostboard/.github/workflows/test.yml:1090
  ghostboard/.github/workflows/test.yml:1122
  ghostboard/.github/workflows/test.yml:1150
  ghostboard/.github/workflows/test.yml:1180
  ghostboard/.github/workflows/test.yml:1208
  ghostboard/.github/workflows/test.yml:1236
  ghostboard/.github/workflows/test.yml:1269
  ghostboard/.github/workflows/test.yml:1297
  ghostboard/.github/workflows/test.yml:1392
  ghostboard/.github/workflows/update-colorschemes.yml:9
  ghostboard/macos/Sources/Ghostty/Ghostty.App.swift:710
  ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift:1030
  ghostboard/src/renderer/Thread.zig:710
  ghostboard/macos/Tests/Update/ReleaseNotesTests.swift:31
  ghostboard/macos/Tests/Update/ReleaseNotesTests.swift:47
  ghostboard/macos/Tests/Update/ReleaseNotesTests.swift:63
  ghostboard/macos/Tests/Update/ReleaseNotesTests.swift:78
  ghostboard/macos/Tests/Update/ReleaseNotesTests.swift:125
  ghostboard/macos/Sources/Helpers/Fullscreen.swift:198
  ghostboard/macos/Sources/Helpers/Fullscreen.swift:236
  ghostboard/macos/Sources/Features/Splits/TerminalSplitTreeView.swift:41
  ghostboard/PACKAGING.md:34
  ghostboard/macos/Sources/Features/QuickTerminal/QuickTerminalWindow.swift:34
  ghostboard/macos/Sources/Features/QuickTerminal/QuickTerminalWindow.swift:44
  ghostboard/macos/Sources/Features/Command Palette/CommandPalette.swift:198
  ghostboard/macos/Sources/Features/Update/UpdateViewModel.swift:308
  ghostboard/macos/Sources/Features/Update/UpdateViewModel.swift:310
  ghostboard/macos/Sources/Features/QuickTerminal/QuickTerminalController.swift:498
  ghostboard/macos/Sources/Features/Terminal/Window Styles/HiddenTitlebarTerminalWindow.swift:42
  ghostboard/macos/Sources/Features/Terminal/Window Styles/TitlebarTabsTahoeTerminalWindow.swift:281
  ghostboard/macos/Sources/Features/Terminal/TerminalController.swift:471
  ghostboard/macos/Sources/Features/Terminal/TerminalController.swift:1161
  ghostboard/macos/Sources/Features/About/AboutView.swift:6
  ```

  These remaining hits are benign for this experiment:

  - source and test comments are upstream regression/provenance references;
  - generated documentation templates and help generators are upstream project
    links, not local contributor instructions;
  - example `build.zig.zon` URLs are commented dependency examples;
  - `.github/workflows/**` hits are inherited CI repository guards that only run
    in `ghostty-org/ghostty`;
  - `snap/snapcraft.yaml`, `PACKAGING.md`, update URLs, and About-view URLs are
    packaging/runtime metadata that are outside the contributor-gate and
    agent-facing policy cleanup.

  No remaining hit is an agent instruction, contributor-gate policy, or local
  issue/PR policy.

- All `ghostboard/**/AGENTS.md` files were re-read after the edit and contain
  useful local build/subsystem guidance.
- Markdown files were formatted with:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    ghostboard/AGENTS.md \
    ghostboard/AI_POLICY.md \
    ghostboard/CONTRIBUTING.md \
    ghostboard/HACKING.md \
    ghostboard/README.md \
    ghostboard/macos/AGENTS.md \
    ghostboard/src/inspector/AGENTS.md \
    ghostboard/test/fuzz-libghostty/AGENTS.md
  ```

- `git diff --check` passed.

## Completion Review

Fresh-context adversarial completion review initially returned **CHANGES
REQUIRED** with one required finding:

- the result summarized the remaining targeted grep hits by category, but the
  experiment's verification criteria required every remaining match to be
  explicitly listed and justified.

The result was updated with the full remaining `ghostty-org/ghostty` hit
inventory and category justification. Re-review returned **APPROVED** with no
findings. The reviewer also independently reran the targeted grep and
`git diff --check`; the listed hits matched and `git diff --check` passed.

## Conclusion

The confirmed poison and inherited contributor-gate policy are removed from the
active Ghostboard fork. The remaining suspicious search hits are upstream
reference URLs or technical provenance, not agent-facing instructions.
