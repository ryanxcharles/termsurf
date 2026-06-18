# Experiment 8: Add Testable Ghostboard App Install Root

## Description

Experiment 7 added Ghostboard to the repo-level build/install/uninstall scripts,
but the result stayed Partial because the required `/Applications` smoke could
not run without an interactive sudo password. The scripts need a way to verify
the app install/uninstall behavior without touching `/Applications`, while
keeping `/Applications/TermSurf Ghostboard.app` as the default user install
path.

This experiment will add a Ghostboard app install-root override for
`scripts/install.sh ghostboard` and `scripts/uninstall.sh ghostboard`. It will
not change release tarball packaging, Homebrew cask packaging, Roamium install
paths, or default app install behavior.

## Changes

Planned script changes:

- `scripts/install.sh`
  - Add `TERMSURF_APPLICATIONS_DIR`, defaulting to `/Applications`.
  - Use `$TERMSURF_APPLICATIONS_DIR/TermSurf Ghostboard.app` for the Ghostboard
    app destination.
  - Preserve existing default behavior: when `TERMSURF_APPLICATIONS_DIR` is not
    set and the script is not root, it still re-execs through `sudo`.
  - For the direct `ghostboard` component only, allow non-root execution when
    `TERMSURF_APPLICATIONS_DIR` points at a writable non-default directory.
  - Keep the pre-sudo missing-release-app check from Experiment 7.
- `scripts/uninstall.sh`
  - Add the same `TERMSURF_APPLICATIONS_DIR` defaulting to `/Applications`.
  - Use `$TERMSURF_APPLICATIONS_DIR/TermSurf Ghostboard.app` for the Ghostboard
    app removal path.
  - Preserve existing default behavior: when `TERMSURF_APPLICATIONS_DIR` is not
    set and the script is not root, it still re-execs through `sudo`.
  - For the direct `ghostboard` component only, allow non-root execution when
    `TERMSURF_APPLICATIONS_DIR` points at a writable non-default directory.

Planned issue-document changes:

- Add `## Result` and `## Conclusion` after verification.
- Update the Issue 819 README experiment status after verification.

Explicitly out of scope:

- Changing Wezboard, Roamium, or webtui install paths.
- Changing `scripts/release.sh`.
- Changing the Homebrew cask.
- Changing installed Roamium discovery.
- Running the real `/Applications` sudo install/uninstall smoke, unless sudo is
  already available non-interactively.

## Verification

Formatting actions:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0819-ghostboard-packaging-identity-hardening/README.md \
  issues/0819-ghostboard-packaging-identity-hardening/08-add-testable-ghostboard-app-install-root.md
```

Static checks:

```bash
bash -n scripts/install.sh scripts/uninstall.sh
git diff --check
rg -n 'TERMSURF_APPLICATIONS_DIR|TermSurf Ghostboard.app|sudo' scripts/install.sh scripts/uninstall.sh
```

Runtime checks:

1. Ensure the release Ghostboard app exists:

   ```bash
   test -x 'ghostboard/macos/build/Release/TermSurf Ghostboard.app/Contents/MacOS/ghostboard' || \
     scripts/build.sh ghostboard --release
   test -x 'ghostboard/macos/build/Release/TermSurf Ghostboard.app/Contents/MacOS/ghostboard'
   ```

2. Install and uninstall Ghostboard into a temporary applications directory
   without sudo:

   ```bash
   APP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-ghostboard-apps.XXXXXX")"
   TERMSURF_APPLICATIONS_DIR="$APP_ROOT" scripts/install.sh ghostboard
   test -x "$APP_ROOT/TermSurf Ghostboard.app/Contents/MacOS/ghostboard"
   TERMSURF_APPLICATIONS_DIR="$APP_ROOT" scripts/uninstall.sh ghostboard
   test ! -e "$APP_ROOT/TermSurf Ghostboard.app"
   rm -rf "$APP_ROOT"
   ```

3. Confirm the default privileged path still re-execs through the scripts' own
   sudo guard without touching `/Applications`, using a fake `sudo` shim:

   ```bash
   SUDO_SHIM_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-fake-sudo.XXXXXX")"
   SUDO_LOG="$SUDO_SHIM_DIR/sudo.log"
   printf '%s\n' \
     '#!/usr/bin/env bash' \
     'printf "%s\n" "$*" >>"$SUDO_LOG"' \
     'exit 19' >"$SUDO_SHIM_DIR/sudo"
   chmod +x "$SUDO_SHIM_DIR/sudo"
   env -u TERMSURF_APPLICATIONS_DIR SUDO_LOG="$SUDO_LOG" PATH="$SUDO_SHIM_DIR:$PATH" \
     scripts/install.sh ghostboard && exit 1 || test "$?" -eq 19
   grep -F 'scripts/install.sh ghostboard' "$SUDO_LOG"
   : >"$SUDO_LOG"
   env -u TERMSURF_APPLICATIONS_DIR SUDO_LOG="$SUDO_LOG" PATH="$SUDO_SHIM_DIR:$PATH" \
     scripts/uninstall.sh ghostboard && exit 1 || test "$?" -eq 19
   grep -F 'scripts/uninstall.sh ghostboard' "$SUDO_LOG"
   rm -rf "$SUDO_SHIM_DIR"
   ```

   This proves the default direct `ghostboard` install and uninstall paths still
   attempt sudo re-exec when `TERMSURF_APPLICATIONS_DIR` is unset, without
   invoking real sudo and without mutating `/Applications`.

Pass criteria:

- The direct `ghostboard` install/uninstall scripts can copy/remove
  `TermSurf Ghostboard.app` under a temporary `TERMSURF_APPLICATIONS_DIR`
  without sudo.
- The installed temp app contains executable `Contents/MacOS/ghostboard`.
- The uninstall removes only the app under the specified
  `TERMSURF_APPLICATIONS_DIR`.
- With no `TERMSURF_APPLICATIONS_DIR`, the default destination remains
  `/Applications/TermSurf Ghostboard.app`.
- Wezboard, Roamium, webtui, release tarball, Homebrew cask, and installed
  Roamium discovery behavior are unchanged.

Partial criteria:

- The override installs but cannot be codesigned in the temporary directory.
- The default sudo guard cannot be tested because the current process is already
  root and therefore does not take the re-exec path.

Fail criteria:

- The default Ghostboard destination changes away from
  `/Applications/TermSurf Ghostboard.app`.
- The override affects non-Ghostboard components.
- The temporary uninstall can remove paths outside `TERMSURF_APPLICATIONS_DIR`.

## Design Review

This experiment is plan-only until a fresh-context adversarial design review
approves it. Record the reviewer verdict here, fix all real findings, and commit
the approved plan before implementation begins.

Fresh-context adversarial design review by Codex subagent `Rawls the 2nd`:

- **Initial verdict:** Changes required.
- **Required finding:** The planned sudo guard check used outer `sudo -n`, which
  would either skip the script or fail before the script ran, so it did not
  prove the script's own re-exec path. Fixed by replacing it with a fake `sudo`
  shim in `PATH` and asserting `scripts/install.sh ghostboard` invokes that shim
  when `TERMSURF_APPLICATIONS_DIR` is unset.
- **Required finding:** The planned sudo guard verification only covered install
  even though uninstall has its own sudo guard. Fixed by adding the same fake
  `sudo` shim check for `scripts/uninstall.sh ghostboard`.
- **Re-review verdict:** Approved.

## Completion Gate

After implementation and verification:

- add `## Result` and `## Conclusion` to this experiment file;
- update the Issue 819 README experiment status from `Designed` to `Pass`,
  `Partial`, or `Fail`;
- request a fresh-context completion review;
- fix all real completion-review findings and record the final verdict in this
  file; and
- commit the reviewed result separately before designing or implementing the
  next experiment.

## Result

**Result:** Pass

Implemented a Ghostboard app install-root override that makes install/uninstall
behavior testable without touching `/Applications`, while preserving
`/Applications/TermSurf Ghostboard.app` as the default destination.

Changed files:

- `scripts/install.sh`
  - Added `APPLICATIONS_DIR="${TERMSURF_APPLICATIONS_DIR:-/Applications}"`.
  - Uses `$APPLICATIONS_DIR/TermSurf Ghostboard.app` for the Ghostboard app
    install destination only when the direct component is `ghostboard`.
  - Allows direct `ghostboard` installs to run without sudo only when
    `TERMSURF_APPLICATIONS_DIR` is set to a writable non-default directory.
  - Fails fast if the direct `ghostboard` override directory is not writable,
    rather than sudo-reexecing and risking a fallback to `/Applications`.
  - Keeps the existing sudo re-exec path for default `/Applications` installs
    and all other components, including `all`.
- `scripts/uninstall.sh`
  - Added the same `TERMSURF_APPLICATIONS_DIR` override and default.
  - Uses `$APPLICATIONS_DIR/TermSurf Ghostboard.app` for the Ghostboard app
    uninstall path only when the direct component is `ghostboard`.
  - Allows direct `ghostboard` uninstalls to run without sudo only when
    `TERMSURF_APPLICATIONS_DIR` is set to a writable non-default directory.
  - Fails fast if the direct `ghostboard` override directory is not writable,
    rather than sudo-reexecing and risking a fallback to `/Applications`.
  - Keeps the existing sudo re-exec path for default `/Applications` uninstalls
    and all other components, including `all`.

Verification passed:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0819-ghostboard-packaging-identity-hardening/README.md \
  issues/0819-ghostboard-packaging-identity-hardening/08-add-testable-ghostboard-app-install-root.md
bash -n scripts/install.sh scripts/uninstall.sh
git diff --check
rg -n 'TERMSURF_APPLICATIONS_DIR|APPLICATIONS_DIR|TermSurf Ghostboard.app|sudo|needs_root' scripts/install.sh scripts/uninstall.sh
test -x 'ghostboard/macos/build/Release/TermSurf Ghostboard.app/Contents/MacOS/ghostboard' || \
  scripts/build.sh ghostboard --release
test -x 'ghostboard/macos/build/Release/TermSurf Ghostboard.app/Contents/MacOS/ghostboard'
APP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-ghostboard-apps.XXXXXX")"
TERMSURF_APPLICATIONS_DIR="$APP_ROOT" scripts/install.sh ghostboard
test -x "$APP_ROOT/TermSurf Ghostboard.app/Contents/MacOS/ghostboard"
TERMSURF_APPLICATIONS_DIR="$APP_ROOT" scripts/uninstall.sh ghostboard
test ! -e "$APP_ROOT/TermSurf Ghostboard.app"
rm -rf "$APP_ROOT"
SUDO_SHIM_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-fake-sudo.XXXXXX")"
SUDO_LOG="$SUDO_SHIM_DIR/sudo.log"
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'printf "%s\n" "$*" >>"$SUDO_LOG"' \
  'exit 19' >"$SUDO_SHIM_DIR/sudo"
chmod +x "$SUDO_SHIM_DIR/sudo"
env -u TERMSURF_APPLICATIONS_DIR SUDO_LOG="$SUDO_LOG" PATH="$SUDO_SHIM_DIR:$PATH" \
  scripts/install.sh ghostboard && exit 1 || test "$?" -eq 19
grep -F 'scripts/install.sh ghostboard' "$SUDO_LOG"
: >"$SUDO_LOG"
env -u TERMSURF_APPLICATIONS_DIR SUDO_LOG="$SUDO_LOG" PATH="$SUDO_SHIM_DIR:$PATH" \
  scripts/uninstall.sh ghostboard && exit 1 || test "$?" -eq 19
grep -F 'scripts/uninstall.sh ghostboard' "$SUDO_LOG"
rm -rf "$SUDO_SHIM_DIR"
```

The temporary install copied and codesigned `TermSurf Ghostboard.app` under the
temporary applications directory. The temporary uninstall removed only that app
path. The fake-sudo check proved both default direct `ghostboard` install and
uninstall still re-exec through `sudo` when `TERMSURF_APPLICATIONS_DIR` is
unset.

After completion review found safety gaps, additional verification also passed:

```bash
SUDO_SHIM_DIR="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-fake-sudo.XXXXXX")"
SUDO_LOG="$SUDO_SHIM_DIR/sudo.log"
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'printf "%s\n" "$*" >>"$SUDO_LOG"' \
  'exit 19' >"$SUDO_SHIM_DIR/sudo"
chmod +x "$SUDO_SHIM_DIR/sudo"
OVERRIDE_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-ignored-apps.XXXXXX")"
TERMSURF_APPLICATIONS_DIR="$OVERRIDE_ROOT" SUDO_LOG="$SUDO_LOG" PATH="$SUDO_SHIM_DIR:$PATH" \
  scripts/install.sh all && exit 1 || test "$?" -eq 19
grep -F 'scripts/install.sh all' "$SUDO_LOG"
test ! -e "$OVERRIDE_ROOT/TermSurf Ghostboard.app"
rm -rf "$OVERRIDE_ROOT" "$SUDO_SHIM_DIR"
BAD_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-bad-apps.XXXXXX")"
chmod 500 "$BAD_ROOT"
TERMSURF_APPLICATIONS_DIR="$BAD_ROOT" scripts/install.sh ghostboard >"$BAD_ROOT.install.log" 2>&1 && exit 1 || test "$?" -eq 1
grep -F 'TERMSURF_APPLICATIONS_DIR is not writable' "$BAD_ROOT.install.log"
TERMSURF_APPLICATIONS_DIR="$BAD_ROOT" scripts/uninstall.sh ghostboard >"$BAD_ROOT.uninstall.log" 2>&1 && exit 1 || test "$?" -eq 1
grep -F 'TERMSURF_APPLICATIONS_DIR is not writable' "$BAD_ROOT.uninstall.log"
chmod 700 "$BAD_ROOT"
rm -rf "$BAD_ROOT" "$BAD_ROOT.install.log" "$BAD_ROOT.uninstall.log"
```

Those checks prove `TERMSURF_APPLICATIONS_DIR` is ignored for `install.sh all`
and that a non-writable direct override fails before sudo re-exec.

## Conclusion

Ghostboard install/uninstall behavior now has a durable non-privileged
regression guard. The default user-facing app path remains
`/Applications/TermSurf Ghostboard.app`, but tests and local experiments can set
`TERMSURF_APPLICATIONS_DIR` to verify the copy/remove behavior without sudo.
This resolves the verification gap left by Experiment 7 without changing release
packaging, Homebrew packaging, Roamium install paths, or runtime app behavior.

## Completion Review

Fresh-context adversarial completion review by Codex subagent `Cicero the 2nd`:

- **Initial verdict:** Changes required.
- **Required finding:** `TERMSURF_APPLICATIONS_DIR` was not limited to direct
  `ghostboard` install/uninstall; `install.sh all` and `uninstall.sh all` could
  also use the override for Ghostboard. Fixed by using the override only when
  `COMPONENT=ghostboard`; `all` continues to use `/Applications`.
- **Required finding:** If a direct override directory was non-writable, sudo
  re-exec could drop `TERMSURF_APPLICATIONS_DIR` and fall back to
  `/Applications`. Fixed by failing fast on non-writable direct override
  directories before sudo re-exec.
- **Re-review verdict:** Approved.
