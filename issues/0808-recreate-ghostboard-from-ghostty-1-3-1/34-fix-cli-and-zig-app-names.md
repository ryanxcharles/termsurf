# Experiment 34: Fix CLI and Zig app names

## Description

Experiment 33 found the remaining Issue 808 blocker: the user-facing macOS app
bundle executable is already `termsurf`, but Ghostboard's Zig build metadata
still contains two stale Ghostty names:

- `ghostboard/src/build/GhosttyExe.zig` still names the standalone executable
  target `ghostty`;
- `ghostboard/src/build/GhosttyXcodebuild.zig` still expects the Xcode-built app
  bundle to be `Ghostty.app` and tries to run `Contents/MacOS/ghostty`, even
  though the Xcode project now builds `TermSurf.app` with
  `Contents/MacOS/termsurf`.

This experiment will make the minimum build/name corrections needed for the
current Ghostboard port:

- the Zig standalone executable target name becomes `termsurf`;
- the Zig macOS app build wrapper copies and runs `TermSurf.app`;
- local agent build instructions stop telling future runs to expect
  `Ghostty.app`.

This experiment will not change `build.zig` install semantics. In particular, it
will not make `emit-exe` mean anything other than the existing upstream build
option behavior. The intended change is artifact naming, not when or how
artifacts are installed.

## Changes

Expected files:

- `ghostboard/src/build/GhosttyExe.zig`
  - change the executable artifact name from `ghostty` to `termsurf`;
  - update directly adjacent user-facing build warning text if it names the
    resulting binary.
- `ghostboard/src/build/GhosttyXcodebuild.zig`
  - change the Xcode app path from `macos/build/{config}/Ghostty.app` to
    `macos/build/{config}/TermSurf.app`;
  - change the Zig `run` helper executable path from `Contents/MacOS/ghostty` to
    `Contents/MacOS/termsurf`;
  - update directly adjacent step labels or comments if they describe the
    user-facing app as Ghostty.
- `ghostboard/macos/AGENTS.md`
  - update local build/run examples that still point at `Ghostty.app` so they
    match the current `TermSurf.app` output.
- `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/34-fix-cli-and-zig-app-names.md`
  - record the experiment result.
- `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/README.md`
  - add Experiment 34 to the experiment index.

No changes are planned to:

- `webtui/`;
- `roamium/`;
- `chromium/`;
- `proto/termsurf.proto`;
- TermSurf protocol handling;
- `build.zig` install gating or `emit-exe` semantics.

## Verification

Pass criteria:

- `zig fmt` succeeds on the changed Zig files.
- `prettier --write --prose-wrap always --print-width 80` succeeds on the
  changed Markdown files.
- `git diff --check` is clean.
- Static source checks show:
  - `ghostboard/src/build/GhosttyExe.zig` uses `.name = "termsurf"`;
  - `ghostboard/src/build/GhosttyXcodebuild.zig` uses `TermSurf.app`;
  - `ghostboard/src/build/GhosttyXcodebuild.zig` uses `Contents/MacOS/termsurf`;
  - the same files no longer contain `Ghostty.app`, `Contents/MacOS/ghostty`, or
    `.name = "ghostty"` for the main executable target.
- `ghostboard/macos/AGENTS.md` no longer points build/run examples at
  `Ghostty.app`, and uses `TermSurf.app` where it names the current app bundle
  output.
- `cd ghostboard && rm -rf zig-out && zig build -Demit-macos-app=true` succeeds
  and installs/copies `zig-out/TermSurf.app`.
- The installed app bundle contains
  `zig-out/TermSurf.app/Contents/MacOS/termsurf`.
- There is no installed `zig-out/Ghostty.app`.
- The built app bundle metadata still reports:
  - `CFBundleName = TermSurf`;
  - `CFBundleExecutable = termsurf`.
- If local GTK runtime dependencies are still unavailable, the experiment may
  record
  `cd ghostboard && zig build -Dapp-runtime=gtk -Demit-macos-app=false -Demit-xcframework=false -Demit-exe=true`
  as an environment-limited check, but it must not use that failure to hide the
  source-level executable-name fix.
- If the GTK runtime build is available, it must produce `zig-out/bin/termsurf`
  and must not produce `zig-out/bin/ghostty`.

Fail criteria:

- `build.zig` install semantics are changed.
- The macOS app copy path still expects `Ghostty.app`.
- The Zig `run` helper still tries to execute `Contents/MacOS/ghostty`.
- The main standalone executable target is still named `ghostty`.
- Verification claims the CLI command requirement is fully satisfied without
  either building a standalone `zig-out/bin/termsurf` or explicitly recording
  why the local standalone runtime build could not be completed.
- Any product change outside the declared files is made.

## Design Review

A fresh-context adversarial reviewer returned **APPROVED** with no required
findings.

The reviewer raised one optional finding: the design planned to update
`ghostboard/macos/AGENTS.md`, but the verification section did not explicitly
check that its examples stopped pointing at `Ghostty.app`. The finding was
accepted, and the verification criteria now require the local macOS agent guide
to use `TermSurf.app` where it names the current app bundle output.
