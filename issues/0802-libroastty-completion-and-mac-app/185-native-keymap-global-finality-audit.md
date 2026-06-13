# Experiment 185: Phase G — native keymap and global finality audit

## Description

Close, or precisely fail to close, the last required Phase G checklist item:
native keymaps (`keycodes`, `KeymapDarwin`) plus app-level key handling.

Earlier experiments split and proved most of this surface: key remapping,
`macos-option-as-alt`, keyboard-layout reload, live layout probing,
`KeymapDarwin`, app-owned keymap state, copied-app text scope, hosted preedit
state, dead-key route synthesis, app-key dispatch, global event-tap dispatch,
and event-tap installation state. After Experiments 183 and 184, the README says
the remaining native-key gap is permission-dependent live global shortcut
receipt on hosts where macOS grants Accessibility permission.

This experiment is an audit/proof gate for that final required item. It should
check the native-keymap/global-shortcut roadmap item only if current source
evidence and focused tests prove the native keymap/app-key surface enough for
Issue 802, and if the live-global-shortcut caveat is either directly validated
on this host or explicitly resolved as a host-permission boundary already
covered by dispatch plus installation-state tests. It must not claim that the
optional debug overlay is complete.

## Changes

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After verification, mark it `Pass`, `Partial`, or `Fail`.
  - Check the native-keymap/global-shortcut roadmap item only if the audit
    proves the remaining required native-key scope is complete enough to close.
  - Leave the optional debug overlay unchanged unless a later experiment
    explicitly chooses to implement it.

- `issues/0802-libroastty-completion-and-mac-app/185-native-keymap-global-finality-audit.md`
  - Record source evidence, host-permission evidence if checked, command output,
    test results, result, conclusion, and AI completion review.

- Production code
  - No code change is expected. If the audit finds a real missing behavior,
    record the gap and design a follow-up implementation experiment.

## Verification

Before verification:

- Codex-native adversarial design review approves this experiment.
- Commit the reviewed plan separately from the result.

Source audit:

- Confirm native keycode mapping, key-remap application, option-as-alt
  translation, layout reload, app-owned keymap state, and `KeymapDarwin` are
  present:

  ```bash
  rg -n "NATIVE_TO_KEY|RemapSet|key_remap|roastty_surface_key_translation_mods|macos_option_as_alt|AppKeymap|KeymapDarwin|roastty_app_keyboard_changed|roastty_current_keyboard_layout" \
    roastty/src
  ```

- Confirm copied-app text handling remains AppKit-provided and the raw ABI
  handoff preserves app-provided UTF-8/composing state:

  ```bash
  rg -n "interpretKeyEvents|setMarkedText|insertText|committedPreeditText|withCValue|surface_key_by_value_utf8|roastty_surface_preedit|roastty_surface_ime_point" \
    roastty/src roastty/macos/Sources roastty/macos/Tests roastty/macos/RoasttyUITests
  ```

- Confirm app-level key handling and global event-tap dispatch/installation
  state are wired:

  ```bash
  rg -n "roastty_app_key|roastty_app_has_global_keybinds|GlobalEventTap|handleCapturedEvent|tapFactory|retryScheduler|isInstalled|isRetryPending" \
    roastty/src roastty/macos/Sources roastty/macos/Tests
  ```

Focused tests:

- `cargo test -p roastty key_remap`
- `cargo test -p roastty key_translation_mods`
- `cargo test -p roastty keyboard_layout`
- `cargo test -p roastty keymap_darwin`
- `cargo test -p roastty app_keymap`
- `cargo test -p roastty app_key`
- `cargo test -p roastty surface_key_by_value_utf8_reaches_child_pty`
- `cargo test -p roastty preedit`
- `cargo test -p roastty surface_preedit`
- `cargo test -p roastty --test abi_harness`
- `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/KeyboardLayoutTests`
- `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/SurfaceKeyTextTests`
- `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/SurfaceViewAppKitTests`
- `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/GlobalEventTapTests`

Live-global-shortcut host check:

- Inspect whether this host can support a permission-dependent live global
  shortcut receipt check without changing product behavior:
  - whether the current process/app is Accessibility-trusted;
  - whether existing UI/event-tap tests already include a live receipt selector;
  - whether a focused live receipt test can be run safely with the existing
    harness.
- If permission or harness support is absent, record that as host evidence and
  decide whether the checklist item can still close from the non-permission
  state-machine plus captured-event dispatch proofs. Do not fabricate live
  receipt proof.

Regression and hygiene:

- `cargo fmt --check -p roastty`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/185-native-keymap-global-finality-audit.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `git diff --check`

**Pass** = source audit and focused tests prove the native keymap/app-key
surface listed in the roadmap, hosted macOS tests prove layout/text/preedit and
event-tap dispatch/state, and the remaining permission-dependent live global
shortcut receipt question is either directly validated on this host or
explicitly resolved as a host-permission boundary that should not keep Issue 802
open.

**Partial** = native keymap/app-key behavior is mostly proved, but live global
shortcut receipt still lacks a direct or accepted boundary proof, a focused
hosted test remains Partial in a way that blocks the checklist item, or a
specific native-key behavior remains unproved.

**Fail** = source audit or focused tests contradict the claim that the
native-keymap/global-shortcut roadmap item is complete enough to check.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Helmholtz the 2nd`,
fresh context.

**Verdict:** Approved.

Findings: None. The reviewer confirmed the README links Experiment 185 as
`Designed`, the experiment has the required sections, the scope is limited to
the final Phase G native-keymap/global-shortcut audit, optional debug overlay
and broader Issue 802 completion are not overclaimed, verification covers native
keymap, app text/preedit, app-key, event-tap, live permission-boundary, hygiene,
and plan/result commit gates, and the pass/partial criteria are honest about
live Accessibility permission and the existing Partial UI-oracle history.
