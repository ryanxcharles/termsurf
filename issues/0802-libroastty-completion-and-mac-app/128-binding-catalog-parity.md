# Experiment 128: Phase G — binding catalog parity

## Description

Prove and finish the remaining upstream binding/default-action tail for the
macOS Roastty path.

The Phase G notes still list "the full upstream binding/default-action tail" as
remaining after Exp 127. A fresh comparison shows that Roastty's current
`DEFAULT_BINDINGS` table appears to match the pinned upstream macOS
`Keybinds.init` defaults, and the parser has grown most of the upstream
`input.Binding.Action` union over prior experiments. What is missing is a
single, durable audit that ties those two facts to tests and closes the
checklist item only if the code proves it.

This experiment audits the pinned upstream action union and default initializer
against Roastty, adds compact regression tests for catalog coverage and macOS
default reverse-trigger parity, and fixes any discovered missing parser,
canonicalization, or default-binding rows. It is intentionally limited to the
single-key configured/default binding catalog. Native keymaps, keyboard-layout
reload, platform global shortcut registration, and command-palette UI execution
remain separate Phase G work.

## Changes

- `roastty/src/lib.rs`
  - Add an exhaustive pinned-upstream coverage fixture derived from
    `vendor/ghostty/src/input/Binding.zig`'s `input.Binding.Action` union: every
    upstream action tag must be represented exactly once as supported or as an
    explicit exclusion.
  - Exercise `canonical_config_binding_action` for every supported upstream
    config-bindable action and every finite enum parameter value. Open-ended
    string and numeric parameters use representative values, but finite domains
    must be complete:
    - `copy_to_clipboard:{plain,vt,html,mixed}`;
    - `write_{scrollback,screen,selection}_file` action and format variants;
    - `navigate_search:{previous,next}`;
    - every `adjust_selection` direction;
    - every close-tab, split-direction, split-focus, goto-window,
      resize-direction, inspector-mode, fullscreen-mode, float-window,
      secure-input, key-table, key-sequence, and crash enum value exposed by the
      Roastty ABI or parser.
  - Exercise representative open-ended parameter values for:
    - strings: `text`, `csi`, `esc`, `search`, titles, key-table names;
    - numbers/floats: font sizes, scroll rows/fractions/lines, prompt jumps, tab
      indexes, tab movement, split resize amount.
  - Include the main action families:
    - transport/input actions: `ignore`, `text`, `csi`, `esc`, `cursor_key`, and
      `reset`;
    - mutation-only binding actions such as `unbind` as explicit exclusions
      unless the audit finds Roastty should expose them through
      `canonical_config_binding_action`;
    - clipboard/file/font/search/scroll/selection actions;
    - tab/window/split/title/app-runtime actions;
    - key-table, key-sequence, chain-compatible actions;
    - `crash:{main,io,render}`.
  - Record explicit exclusions for action names that are intentionally not
    executable binding actions in Roastty's embedded path, and add tests proving
    excluded names are rejected or handled by a non-action parser path.
  - Add a macOS default-binding parity test that checks the current
    `DEFAULT_BINDINGS` table against the pinned upstream
    `vendor/ghostty/src/config/Config.zig` macOS branch for:
    - trigger key kind and value;
    - modifier mask;
    - canonical action string;
    - performable flag where upstream uses `putFlags`.
  - Add reverse-trigger tests for the app/menu-facing defaults whose ordering is
    subtle, such as `open_config`, `reload_config`, `copy_to_clipboard:mixed`,
    `paste_from_clipboard`, `goto_tab:{1..8}`, `last_tab`, `close_tab:this`,
    `new_split:{right,down}`, `toggle_fullscreen`, `toggle_command_palette`,
    `start_search`, and `end_search`.
  - If the audit finds missing parser/canonical/default rows, add the smallest
    faithful implementation needed for parity and tests.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - If implementation proves parity, update the Phase G checklist and operating
    notes so the remaining Phase G gaps no longer include the upstream
    binding/default-action tail.

Out of scope:

- Native keymaps (`keycodes`, `KeymapDarwin`) and keyboard-layout reload.
- Native global shortcut registration.
- Command-palette UI behavior and command execution from the copied app.
- Non-macOS default binding tables.
- Runtime implementation of actions that already parse/canonicalize but are
  app-callback no-ops in the current test harness.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/128-binding-catalog-parity.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Run targeted tests:
  - `cargo test -p roastty binding_action_catalog`
  - `cargo test -p roastty default_binding`
  - `cargo test -p roastty config_trigger`
  - `cargo test -p roastty command_palette`
- Run full Roastty tests:
  - `cargo test -p roastty -- --test-threads=1`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run the same Prettier command with `--check`.

**Pass** = Roastty has exhaustive test-backed coverage for every pinned upstream
binding-action tag, every finite enum parameter value, and the macOS default
binding/reverse-trigger table; any discovered missing rows are fixed; and the
issue checklist can remove the "full upstream binding/default-action tail" from
the remaining Phase G gaps.

**Partial** = the audit proves most coverage but finds a larger missing action
family or default-table mismatch that needs its own follow-up experiment.

**Fail** = the binding/default catalog cannot be proven without first
implementing native keymaps, native global shortcuts, or command-palette UI
behavior.

## Design Review

**Reviewer:** Codex-native adversarial reviewer, fresh context
(`multi_agent_v1.spawn_agent`, agent `019eb850-950a-7431-bb2d-8d4279fa8230`)

**Initial verdict:** Changes required.

**Required finding:** The original plan only required representative
parameterized action variants. The reviewer pointed out that finite upstream
enum domains are part of the binding catalog; a representative-only test could
pass while missing legal variants such as clipboard formats, search navigation
directions, selection adjustments, split/window modes, or crash locations.

**Fix:** The design now requires an exhaustive pinned-upstream coverage fixture:
every upstream action tag must be represented exactly once as supported or as an
explicit exclusion, every finite enum parameter value must be covered, and only
open-ended string/numeric domains may use representative values. The pass
criteria now explicitly require exhaustive action-tag, finite-variant, and macOS
default/reverse-trigger parity coverage.

**Final verdict:** Approved. The reviewer confirmed the required finding and
optional pass-criteria finding were resolved and reported no new required
findings.
