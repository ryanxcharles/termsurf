# Experiment 36: Fix and install helper CLI

## Description

Experiment 35 proved that installing the `.none` runtime helper CLI reaches the
right build target, but the helper does not compile on Zig 0.15 because
`ghostboard/src/main_ghostty.zig` still calls the removed `std.io.getStdOut()`
API.

This experiment will fix the helper CLI compile error and then install the
helper command as `zig-out/bin/termsurf` when `emit-exe` is true. This is the
remaining Issue 808 CLI-command requirement.

The implementation should use the existing Zig 0.15 stdout pattern already used
by nearby CLI code such as `ghostboard/src/cli/help.zig`: create a fixed buffer,
call `std.fs.File.stdout().writer(&buffer)`, write through
`&stdout_writer.interface`, and flush before exiting.

## Changes

Expected files:

- `ghostboard/src/main_ghostty.zig`
  - replace the `.none` helper usage text's removed `std.io.getStdOut()` call
    with the repository's current Zig 0.15 stdout writer pattern;
  - flush the writer before `posix.exit(0)`.
- `ghostboard/build.zig`
  - in the `config.app_runtime == .none` branch, install the main executable
    when `config.emit_exe` is true, so macOS gets the helper CLI at
    `zig-out/bin/termsurf`;
  - keep existing non-Darwin libghostty install behavior intact;
  - do not move or broaden `resources.install()`, `i18n.install()`, or
    `macos_app.install()` behavior.
- `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/36-fix-and-install-helper-cli.md`
  - record the experiment result.
- `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/README.md`
  - add Experiment 36 to the experiment index.

No changes are planned to:

- `webtui/`;
- `roamium/`;
- `chromium/`;
- `proto/termsurf.proto`;
- TermSurf protocol handling;
- app bundle identity, icon, menu, or config paths.

## Verification

Pass criteria:

- `zig fmt ghostboard/build.zig ghostboard/src/main_ghostty.zig` succeeds.
- `prettier --write --prose-wrap always --print-width 80` succeeds on the
  changed Markdown files.
- `git diff --check` is clean.
- Static source checks show:
  - `ghostboard/src/main_ghostty.zig` no longer uses `std.io.getStdOut()`;
  - `ghostboard/src/main_ghostty.zig` uses `std.fs.File.stdout().writer`;
  - `ghostboard/build.zig` installs `exe` in the `.none` runtime branch only
    behind `config.emit_exe`.
- `cd ghostboard && rm -rf zig-out && zig build -Demit-macos-app=false -Demit-xcframework=false -Demit-docs=false`
  succeeds.
- That build produces executable `ghostboard/zig-out/bin/termsurf`.
- That build does not produce `ghostboard/zig-out/bin/ghostty`.
- Running `ghostboard/zig-out/bin/termsurf` exits successfully and prints the
  helper CLI usage text containing `Usage: termsurf +<action> [flags]`.
- `cd ghostboard && rm -rf zig-out && zig build -Demit-exe=false -Demit-macos-app=false -Demit-xcframework=false -Demit-docs=false`
  succeeds without producing `ghostboard/zig-out/bin/termsurf`, proving the
  helper remains gated by `emit-exe`.
- `cd ghostboard && rm -rf zig-out && zig build -Demit-macos-app=true` succeeds
  and still installs/copies `zig-out/TermSurf.app` with
  `Contents/MacOS/termsurf`.
- `git status --short --untracked-files=all` contains only the declared files.

Fail criteria:

- `emit-exe=false` still installs `zig-out/bin/termsurf`.
- The produced helper command is named `ghostty`.
- The helper command fails to run with no arguments.
- The change causes the macOS app bundle install path to regress.
- The experiment changes app/protocol/runtime behavior outside the helper CLI
  compile and build wiring.

## Design Review

A fresh-context adversarial reviewer returned **APPROVED** with no findings. The
reviewer confirmed that the design follows Experiment 35's Zig 0.15 stdout
failure, matches existing stdout writer patterns in `src/cli/help.zig`, keeps
the `build.zig` wiring narrow and gated by `config.emit_exe`, and verifies
positive CLI install/run, negative `emit-exe=false`, no `ghostty` binary, and
macOS app bundle non-regression.

## Result

**Result:** Pass

Experiment 36 fixed the `.none` helper CLI compile error and installed the
helper command as `zig-out/bin/termsurf` when `emit-exe` is true. The helper now
uses the same Zig 0.15 stdout writer pattern as the other CLI commands, and the
`.none` runtime branch in `build.zig` wires the already-created executable
install step only behind `config.emit_exe`.

### Changes

- `ghostboard/src/main_ghostty.zig`
  - replaced the removed `std.io.getStdOut()` usage with
    `std.fs.File.stdout().writer(&buffer)`;
  - writes through `&stdout_writer.interface`;
  - flushes the writer before `posix.exit(0)`.
- `ghostboard/build.zig`
  - installs the main executable in the `.none` runtime branch only when
    `config.emit_exe` is true.

### Verification

- `zig fmt ghostboard/build.zig ghostboard/src/main_ghostty.zig` succeeded.
- `git diff --check` succeeded.
- `logs/ghostboard-exp36-static-checks-20260616.log` shows:
  - `ghostboard/src/main_ghostty.zig` uses
    `std.fs.File.stdout().writer(&buffer)`;
  - `ghostboard/build.zig` has gated `config.emit_exe` / `exe.install()` paths.
- `logs/ghostboard-exp36-positive-cli-build-20260616.log` shows:
  - command:
    `cd ghostboard && rm -rf zig-out && zig build -Demit-macos-app=false -Demit-xcframework=false -Demit-docs=false`;
  - `build_exit=0`;
  - `zig-out/bin/termsurf` was produced;
  - `termsurf_executable_status=0`;
  - `ghostty_binary_exists_status=1`, meaning `zig-out/bin/ghostty` does not
    exist.
- `logs/ghostboard-exp36-positive-cli-run-20260616.log` and
  `logs/ghostboard-exp36-positive-cli-run-exit-20260616.log` show:
  - `run_exit=0`;
  - helper output contains `Usage: termsurf +<action> [flags]`;
  - helper output tells users to launch `TermSurf.app` for the graphical app.
- `logs/ghostboard-exp36-negative-emit-exe-build-20260616.log` shows:
  - command:
    `cd ghostboard && rm -rf zig-out && zig build -Demit-exe=false -Demit-macos-app=false -Demit-xcframework=false -Demit-docs=false`;
  - `build_exit=0`;
  - `termsurf_exists_status=1`, meaning `zig-out/bin/termsurf` does not exist;
  - `ghostty_exists_status=1`, meaning `zig-out/bin/ghostty` does not exist.
- `logs/ghostboard-exp36-app-bundle-regression-20260616.log` shows:
  - command:
    `cd ghostboard && rm -rf zig-out && zig build -Demit-macos-app=true`;
  - `build_exit=0`;
  - `zig-out/TermSurf.app` exists;
  - `zig-out/bin/termsurf` also exists under the default `emit-exe=true` build;
  - `app_termsurf_executable_status=0`;
  - `ghostty_app_exists_status=1`, meaning `zig-out/Ghostty.app` does not exist;
  - bundle metadata reports `CFBundleName = TermSurf` and
    `CFBundleExecutable = termsurf`.
- `git status --short --untracked-files=all` showed only the declared product
  files before the result docs were edited:
  - `ghostboard/build.zig`;
  - `ghostboard/src/main_ghostty.zig`.

## Conclusion

The Issue 808 CLI-command requirement is now satisfied for the Ghostboard Zig
build: a normal macOS `.none` runtime build produces a runnable
`zig-out/bin/termsurf` helper command, `emit-exe=false` suppresses it, and no
`zig-out/bin/ghostty` command is produced.

Issue 808 still needs a final acceptance check before closure because the last
full audit was Experiment 33, before the CLI command was fixed. The next
experiment should do a short closeout audit against the acceptance matrix,
including the newly passing CLI command, and close the issue if no blockers
remain.

## Completion Review

A fresh-context adversarial reviewer returned **APPROVED** with no required
findings.

The reviewer confirmed that the diff only touches the declared files,
`build.zig` installs the `.none` helper only behind `config.emit_exe`, the
stdout fix uses the Zig 0.15 buffered writer pattern and flushes before exit,
the logs prove positive CLI build/run behavior, `emit-exe=false` suppression, no
`ghostty` binary, and app bundle non-regression, and the result commit had not
yet been made.
