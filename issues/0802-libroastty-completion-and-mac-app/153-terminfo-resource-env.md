# Experiment 153: Phase I — terminfo resource env

## Description

Finish the terminfo-resource half of the last Phase I polish item. Upstream
Ghostty ships a generated terminfo source, compiles it into the app bundle, uses
`terminfo/78/xterm-ghostty` as the macOS resources-dir sentinel, and sets child
process environment so shells see the bundled database:

- `TERM=<configured term>`
- `COLORTERM=truecolor`
- `TERMINFO=<bundle resources>/terminfo`
- `GHOSTTY_RESOURCES_DIR=<bundle resources>/ghostty`

Roastty already copies shell-integration resources and creates an empty
`terminfo/78/xterm-roastty` sentinel for resource discovery, but it does not
ship a real terminfo source/database and `Termio::spawn_with_options` does not
set `TERM`, `COLORTERM`, `TERMINFO`, or `ROASTTY_RESOURCES_DIR` from the
discovered resource directory. That leaves bundled-resource detection and child
terminal identity only partially faithful.

This experiment should make the bundled terminfo path real without committing
compiled terminfo database binaries. The `os/cf_release_thread` performance half
of the roadmap line remains later work; Roastty's current CoreText code mostly
uses Rust `CFRetained` ownership and needs a separate design before introducing
a release thread.

## Changes

- `roastty/resources/terminfo/roastty.terminfo`
  - Add a text terminfo source derived from the pinned Ghostty
    `ghostty.terminfo`, mechanically renamed so the primary TERM entry is
    `xterm-roastty` and aliases include `roastty` / `Roastty`.
  - Preserve upstream capabilities except for the mechanical names.
- `roastty/macos/Roastty.xcodeproj/project.pbxproj`
  - Extend the existing "Copy Shell Integration Resources" build phase into a
    bundled resources phase:
    - copy `resources/shell-integration` to `Contents/Resources/roastty`;
    - copy `resources/terminfo/roastty.terminfo` into
      `Contents/Resources/terminfo/`;
    - run `tic -x -o "$resources/terminfo" "$src_terminfo"` so the app bundle
      contains compiled entries such as `terminfo/78/xterm-roastty`;
    - fail loudly if `tic` is unavailable or the compiled sentinel is missing,
      instead of creating an empty placeholder.
- `roastty/src/termio.rs`
  - When `TermioSpawnOptions.resource_dir` is present, set:
    - `ROASTTY_RESOURCES_DIR` to that resource dir;
    - `TERM` to the configured/default term value;
    - `COLORTERM=truecolor`;
    - `TERMINFO` to the sibling `terminfo` directory
      (`resource_dir.parent()/terminfo`).
  - When no resource dir is present, fall back to `TERM=xterm-256color` and
    `COLORTERM=truecolor`, matching upstream's resource-missing behavior.
  - Preserve caller-provided env override semantics deliberately: document and
    test whether explicit env entries supplied by app/user code are overwritten
    by Roastty's terminal identity values, matching upstream's `env.put`
    behavior.
  - Keep shell-integration setup working after the env additions.
- `roastty/src/config/mod.rs`
  - Change Roastty's default `term` value and empty-term finalize fallback from
    `xterm-ghostty` to `xterm-roastty`, so the default advertised `TERM` matches
    the bundled renamed database.
  - Update config parser/formatter/finalize tests that intentionally assert the
    default term value.
- `roastty/macos/Tests/Roastty/ShellIntegrationResourceTests.swift`
  - Update or add hosted tests that prove the built app bundle contains the real
    terminfo source and the compiled `terminfo/78/xterm-roastty` entry rather
    than an empty sentinel.
- `scripts/roastty-app/verify-terminfo-source.py`
  - Add a small verifier that compares
    `roastty/resources/terminfo/roastty.terminfo` with the pinned upstream
    `vendor/ghostty/zig-out/share/terminfo/ghostty.terminfo` after the exact
    allowed mechanical rename:
    - `xterm-ghostty` → `xterm-roastty`;
    - `ghostty` → `roastty`;
    - `Ghostty` → `Roastty`.
  - Use this verifier in tests/checks so a truncated or hand-edited terminfo
    source cannot satisfy the experiment by merely compiling.
- `roastty/src/os/resources_dir.rs`
  - Keep `terminfo/78/xterm-roastty` as the sentinel, but add/adjust tests if
    the expected resource layout changes.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Mark the terminfo half of the final Phase I line as complete, while leaving
    `os/cf_release_thread` open.

## Verification

- `cargo fmt`
- `cargo test -p roastty termio -- --test-threads=1`
- `cargo test -p roastty config -- --test-threads=1`
- `cargo test -p roastty resources_dir -- --test-threads=1`
- `scripts/roastty-app/verify-terminfo-source.py`
- `cargo test -p roastty -- --test-threads=1`
- `cd roastty && macos/build.nu --action test`
- Inspect the built app bundle:
  - `test -s roastty/macos/build/Debug/Roastty.app/Contents/Resources/terminfo/roastty.terminfo`
  - `test -s roastty/macos/build/Debug/Roastty.app/Contents/Resources/terminfo/78/xterm-roastty`
  - `TERMINFO=roastty/macos/build/Debug/Roastty.app/Contents/Resources/terminfo infocmp -x xterm-roastty`
- `cargo fmt --check`
- `git diff --check`

**Pass** = the Roastty terminfo source is mechanically faithful to the pinned
upstream Ghostty source, the Roastty app bundle contains that real renamed
source and compiled `xterm-roastty` database entry, resource discovery still
resolves the bundle's `roastty` resource subdir, default config/finalize now use
`xterm-roastty`, child processes get faithful TERM / COLORTERM / TERMINFO /
ROASTTY_RESOURCES_DIR values when resources exist and the upstream fallback when
they do not, shell-integration tests still pass, all listed Rust and hosted
macOS checks pass, and the README marks only the terminfo half complete.

**Partial** = terminfo resources are bundled but child env wiring or hosted app
verification remains incomplete, or the implementation must keep a placeholder
sentinel for now.

**Fail** = the generated app bundle lacks a compiled `xterm-roastty` entry,
resource discovery regresses, shell startup env is wrong, or verification cannot
prove the bundled terminfo path.

## Design Review

**Reviewer:** Codex-native adversarial subagent `Helmholtz` with fresh context,
using the `adversarial-review` skill's Codex path
(`multi_agent_v1.spawn_agent`), not Claude's named `adversarial-reviewer` agent.

**Initial verdict:** Changes required.

**Required findings:**

- The first plan could produce a mismatched default TERM/database pair: it
  planned a bundled `xterm-roastty` database but did not change Roastty's
  existing `xterm-ghostty` config default and empty-term finalize fallback.
- The first verification plan did not prove the new terminfo source remained a
  faithful mechanical rename of the pinned upstream source; a minimal compilable
  file could satisfy the non-empty and `infocmp` checks.

**Fixes:**

- Added `roastty/src/config/mod.rs` to the scope, requiring the default and
  finalize fallback term value to become `xterm-roastty` with matching tests.
- Added a dedicated `scripts/roastty-app/verify-terminfo-source.py` verifier
  that compares Roastty's terminfo source with the pinned upstream source after
  the exact allowed mechanical rename.
- Added config tests and the verifier to the required verification list, and
  tightened Pass criteria to require default config/finalize and source-fidelity
  proof.

**Re-review:** Approved. The reviewer confirmed the config default/fallback
scope and verifier requirement resolve the prior findings, and found no new
Required findings.

**Final verdict:** Approved.

## Result

**Result:** Pass

Roastty now ships a real renamed terminfo source and compiles it into the macOS
app bundle during the existing resource-copy build phase. The bundled database
contains `terminfo/78/xterm-roastty`, and `infocmp` resolves `xterm-roastty`
from the built app's `Contents/Resources/terminfo` directory. The source is
verified as a mechanical rename of the pinned upstream Ghostty
`ghostty.terminfo`, not a hand-written placeholder.

`Termio::spawn_with_options` now installs the terminal identity before shell
integration setup. With a discovered resource directory it sets
`ROASTTY_RESOURCES_DIR`, `TERM=<configured term>`, `COLORTERM=truecolor`, and
`TERMINFO=<resources parent>/terminfo`; without resources it falls back to
`TERM=xterm-256color` and `COLORTERM=truecolor`. Explicit stale caller env
values for those keys are overwritten to match upstream's terminal identity
behavior. Roastty's default config term and empty-term finalize fallback are now
`xterm-roastty`, matching the bundled database.

Verification run:

- `cargo test -p roastty termio -- --test-threads=1` — pass, 38 tests.
- `cargo test -p roastty resources_dir -- --test-threads=1` — pass, 12 tests.
- `cargo test -p roastty config -- --test-threads=1` — pass, 459 tests.
- `scripts/roastty-app/verify-terminfo-source.py` — pass.
- `cargo fmt` — pass.
- `cargo test -p roastty -- --test-threads=1` — pass, 4,843 tests; 4 ignored;
  ABI harness and doc-tests passed. Existing enum-conversion warnings and
  `[unknown](scope): message` remained.
- `cd roastty && macos/build.nu --action test` — pass, 211 hosted macOS tests,
  `TEST SUCCEEDED`. Existing SwiftLint/main-thread/pasteboard warnings remained.
- `test -s roastty/macos/build/Debug/Roastty.app/Contents/Resources/terminfo/roastty.terminfo`
  — pass.
- `test -s roastty/macos/build/Debug/Roastty.app/Contents/Resources/terminfo/78/xterm-roastty`
  — pass.
- `TERMINFO=roastty/macos/build/Debug/Roastty.app/Contents/Resources/terminfo infocmp -x xterm-roastty`
  — pass; output begins `xterm-roastty|roastty|Roastty,`.
- `cargo fmt --check` — pass.
- `git diff --check` — pass.

## Conclusion

The terminfo-resource half of the remaining Phase I polish line is complete.
Roastty's default terminal identity, child PTY environment, resource discovery
sentinel, bundled terminfo source, and compiled app-bundle database now agree on
`xterm-roastty`. The remaining item on that roadmap line is the separate
`os/cf_release_thread` performance work.

## Completion Review

**Reviewer:** Codex-native adversarial subagent `Popper` with fresh context,
using the `adversarial-review` skill's Codex path
(`multi_agent_v1.spawn_agent`), not Claude's named `adversarial-reviewer` agent.

**Verdict:** Approved.

**Findings:** No Required findings.

**Independent checks:** The reviewer confirmed the result changes were still
uncommitted on top of the plan commit, verified the terminfo source with
`scripts/roastty-app/verify-terminfo-source.py`, independently compared the
renamed source with upstream, ran `cargo fmt --check`, `git diff --check`, and
the targeted `termio`, `config`, and `resources_dir` test filters, and confirmed
the built bundle's `infocmp -x xterm-roastty` probe resolves to
`xterm-roastty|roastty|Roastty,`.
