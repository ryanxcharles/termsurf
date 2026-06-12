# Experiment 137: Phase G — KeymapDarwin translation foundation

## Description

Port the first Rust-side slice of upstream `input/KeymapDarwin.zig`: a macOS
keyboard-layout translation object that can call `UCKeyTranslate` directly and
return UTF-8 text, composing state, and consumed modifiers.

Experiments 130, 131, and 135 wired `macos-option-as-alt`, keyboard-layout
reload state, and a hosted proof that the live Carbon/TIS layout probe returns
the host layout ID. The copied Swift app still relies on AppKit
`NSEvent.characters` / `interpretKeyEvents` for text. Upstream Ghostty's keymap
contract is stricter: a `KeymapDarwin` owns the current `TISInputSource`,
retains the `UCKeyboardLayout` data, translates physical native keycodes through
`UCKeyTranslate`, strips control before translation, tracks dead-key state, and
reports preedit/composing text.

This experiment builds that Rust foundation without changing the copied app's
runtime keyDown behavior yet. The point is to make the platform translation API
real, safe, and testable before a later experiment switches Swift key events or
the C ABI to use it.

## Changes

- `roastty/src/input/keymap_darwin.rs`
  - Add a macOS-only Rust port of the upstream `KeymapDarwin` state shape:
    retained current input source, borrowed `UCKeyboardLayout` data pointer,
    reload/deinit semantics, per-call translation state, and translation result.
  - Call Carbon/TextInputSources and HIToolbox APIs equivalent to upstream:
    `TISCopyCurrentKeyboardLayoutInputSource`,
    `TISGetInputSourceProperty(kTISPropertyUnicodeKeyLayoutData)`,
    `TISGetInputSourceProperty(kTISPropertyInputSourceID)`, `LMGetKbdType`, and
    `UCKeyTranslate`.
  - Match upstream modifier handling: remove control before translation, map
    Roastty `Mods` into Carbon modifier bits, and report the consumed modifiers
    actually used for translation.
  - Match upstream dead-key handling: preserve an opaque dead-key state, return
    composing/preedit text for dead-key presses by probing Space with
    `kUCKeyTranslateNoDeadKeysMask`, and leave later committed composition for a
    follow-up if it depends on host layout availability.
  - Provide a non-macOS stub that compiles and returns an explicit unsupported
    error, so the crate stays cross-platform.
- `roastty/src/input/mod.rs`
  - Export the new module internally.
- `roastty/src/lib.rs`
  - Add test-only helpers, not app-facing ABI yet, only if they are needed to
    validate translation behavior from existing Roastty tests.
  - Do not change `roastty_surface_key`, `roastty_app_key`, copied Swift
    `keyDown`, or `roastty_input_key_s` semantics in this experiment.
- Tests
  - Add deterministic unit coverage for the Carbon modifier bit mapping,
    control-stripping behavior, UTF-16-to-UTF-8 conversion boundary, invalid
    translation errors, reload/source ownership where feasible, and the
    non-macOS unsupported path.
  - Add a macOS host-smoke test that initializes the current keymap, reads its
    source ID, translates a stable physical key such as keycode `0` with no
    modifiers, and asserts only safe invariants that are layout-independent:
    initialization succeeds or returns the documented error, source IDs are
    valid UTF-8 when present, returned text is valid UTF-8 and at most the
    upstream four-UTF-16-code-unit buffer, and the call does not mutate control
    into text.
  - If the host layout is recognized as US or US-International, add stronger
    assertions for keycode `0` producing `a` without modifiers and `A` with
    Shift. Skip those content assertions for unknown host layouts.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After implementation, narrow the Phase G native-key note to distinguish the
    Rust `KeymapDarwin` translation foundation from the still-unwired app ABI
    and dead-key/preedit runtime integration.

Out of scope:

- Replacing the copied Swift app's AppKit text path with Rust-side
  `KeymapDarwin`.
- Changing `roastty_surface_key`, `roastty_app_key`, or the public
  `roastty_input_key_s` ABI.
- Full IME behavior, marked-text synchronization, Korean/Japanese composition,
  and AppKit `interpretKeyEvents` replacement.
- Live native global shortcut installation or Accessibility-permission
  automation.
- Broad keybinding sequence/table changes.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/137-keymap-darwin-translation-foundation.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Run targeted tests:
  - `cargo test -p roastty keymap_darwin`
  - `cargo test -p roastty keyboard_layout`
  - `cargo test -p roastty key_translation_mods`
- Run build coverage:
  - `cargo build -p roastty`
- Run full Roastty tests:
  - `cargo test -p roastty -- --test-threads=1`
- Run checks:
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/137-keymap-darwin-translation-foundation.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = Roastty has a safe, upstream-shaped Rust `KeymapDarwin` translation
foundation that compiles cross-platform, initializes/reloads the current macOS
keyboard layout, translates native keycodes through `UCKeyTranslate` in tests,
preserves upstream modifier/dead-key semantics at the API boundary, and leaves
the copied app runtime behavior unchanged.

**Partial** = the state shape and safe API compile, but host-dependent
`UCKeyTranslate` smoke coverage has to stay weaker than expected because the
current input source is unavailable or not a Unicode keyboard layout.

**Fail** = a faithful Rust `KeymapDarwin` foundation cannot be separated from
rewiring the copied Swift app keyDown path or the public input ABI.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Arendt`, fresh context.

**Verdict:** Approved.

**Findings:** None.

The reviewer confirmed the README links Experiment 137 as `Designed`, the
experiment has the required sections, the scope is narrow and does not overclaim
copied-app runtime text input, IME, dead-key integration, or live global
shortcut installation, and `git diff --check` plus the Prettier check passed.

## Result

**Result:** Pass

Roastty now has a crate-internal `input::keymap_darwin` module that mirrors the
first platform translation layer from upstream `input/KeymapDarwin.zig` without
changing the copied app's keyDown path or the public `roastty_input_key_s` ABI.

Implemented:

- `KeymapDarwin` with retained current `TISInputSource`, borrowed
  `UCKeyboardLayout` data pointer, `new`, `reload`, `source_id`, and `Drop`
  release semantics.
- `State` carrying the opaque dead-key state and `Translation` carrying UTF-8
  text, composing state, and consumed translation modifiers.
- Carbon/TextInputSources and HIToolbox FFI for
  `TISCopyCurrentKeyboardLayoutInputSource`, `TISGetInputSourceProperty`,
  `CFDataGetBytePtr`, `LMGetKbdType`, and `UCKeyTranslate`.
- Upstream-shaped modifier handling: control is stripped before translation,
  Shift/Caps/Option/Command are mapped to the Carbon modifier byte, and the
  returned `mods` are the modifiers used for translation.
- Upstream-shaped dead-key preedit handling: if `UCKeyTranslate` sets a dead-key
  state with no output, Roastty probes Space with
  `kUCKeyTranslateNoDeadKeysMask` and returns composing preedit text.
- A non-macOS stub that compiles and returns `Unsupported`.
- Internal module export for later app/ABI wiring, while leaving all runtime app
  key behavior unchanged in this experiment.

Tests cover Carbon modifier bit mapping, control stripping, UTF-16 decode bounds
and invalid-surrogate handling, the non-macOS unsupported shape, and a macOS
host smoke test that initializes the current keymap, reloads it, reads the
source ID, translates native keycode `0`, and strengthens content assertions for
US / US-International host layouts.

Verification:

- `cargo fmt`
- `cargo test -p roastty keymap_darwin` passed 5 targeted unit tests.
- `cargo test -p roastty keyboard_layout` passed 3 targeted unit tests.
- `cargo test -p roastty key_translation_mods` passed 10 targeted unit tests.
- `cargo build -p roastty` passed.
- `cargo test -p roastty -- --test-threads=1` passed once with 4,756 Rust unit
  tests, the C ABI harness, and doc tests. A final rerun after a crate-internal
  re-export hit the pre-existing
  `surface_foreground_pid_reports_worker_foreground_pid_after_start` race
  (`left: 373`, `right: 305`) with 4,755 tests passing; the exact failed test
  passed immediately on rerun with
  `cargo test -p roastty surface_foreground_pid_reports_worker_foreground_pid_after_start -- --test-threads=1`.
- `cargo fmt --check` passed.
- `git diff --check` passed.

## Conclusion

The native keymap gap is now split at a better boundary. Roastty has a real
Rust-side macOS `UCKeyTranslate` foundation with upstream state, modifier, and
dead-key preedit semantics, and the copied app behavior is intentionally
unchanged. The remaining native-key work is to wire this foundation into the
app-facing key input path, preserve or replace AppKit marked-text/IME behavior
deliberately, and then validate dead-key/preedit runtime behavior from the
hosted macOS app.

## Completion Review

**Reviewer:** Codex-native adversarial review subagent `Aristotle`, fresh
context.

**Verdict:** Approved.

**Findings:** None.

The reviewer inspected the uncommitted result diff from plan commit
`5eff2c2c33c39` and approved the implementation, tests, README status, and
result documentation without required changes.
