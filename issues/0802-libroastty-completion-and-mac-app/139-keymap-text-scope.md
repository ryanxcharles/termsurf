# Experiment 139: Phase G — keymap text scope

## Description

Correct the native-key roadmap around `KeymapDarwin` text translation and add a
focused wrapper/raw-ABI test for the copied app's embedded key text handoff.

Experiments 137 and 138 added a faithful Rust `KeymapDarwin` foundation and made
`App` own/reload it for keyboard-layout detection. The issue notes still say the
remaining native-key work includes "Rust-side text translation" for
`roastty_surface_key` / `roastty_app_key`. A fresh read of the pinned upstream
contract shows that is not how the copied embedded app path works:

- `vendor/ghostty/src/apprt/embedded.zig` owns `input.Keymap` on `App`, reloads
  it, and uses `sourceId()` for `keyboardLayout()`.
- The embedded C `KeyEvent.keyEvent()` converts the app-provided C event into
  `App.KeyEvent`, including the `text` pointer and `composing` flag.
- `App.KeyEvent.core()` then builds `input.KeyEvent` directly from that
  app-provided text; it does not call `KeymapDarwin.translate`.
- The copied Swift `SurfaceView_AppKit.keyDown` path uses AppKit text input and
  `interpretKeyEvents` for normal text, dead-key, and IME handling, then passes
  the resulting text through the embedded key event.

That means replacing copied-app text with Rust `KeymapDarwin.translate` in
`roastty_surface_key` or `roastty_app_key` would be a divergence, not a missing
piece of the pinned embedded ABI. This experiment records that scope correction
and adds targeted hosted coverage for the wrapper/raw-ABI handoff that the app
uses once Swift has already produced text. It does not claim to test
`SurfaceView_AppKit.keyDown` or AppKit `interpretKeyEvents` directly; full
dead-key/IME UI automation remains later work.

## Changes

- `roastty/macos/Tests/Roastty/SurfaceKeyTextTests.swift`
  - Add a hosted Swift test file focused on the wrapper/raw-ABI key-text
    boundary.
  - Construct `Roastty.Input.KeyEvent` values with explicit UTF-8 text and
    `composing` state and verify `withCValue` preserves those fields only for
    the closure lifetime.
  - Keep the hosted Swift test at the wrapper boundary; do not synthesize full
    AppKit dead-key or IME sessions in this slice.
- `roastty/src/lib.rs`
  - If needed, add one narrow Rust by-value `roastty_surface_key` regression
    test mirroring the existing opaque-handle
    `surface_key_printable_utf8_reaches_child_pty` test, so the Rust ABI path
    explicitly covers C-provided UTF-8 text instead of only the old handle path.
  - Do not change `input_key_to_event`, keybinding dispatch, keymap ownership,
    or public ABI semantics unless a test exposes a real mismatch.
- `roastty/src/lib.rs` comments
  - Clean up stale comments that still describe full `KeymapDarwin` text
    translation as later embedded-app work, replacing them with the upstream
    scope: app-owned layout/reload state is wired, while copied-app text remains
    AppKit-provided.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - Replace remaining "Rust-side text translation" wording with the upstream
    scope: copied-app text comes from AppKit / `interpretKeyEvents`; Rust
    `KeymapDarwin` remains app-owned layout/reload state plus a translation
    primitive, but is not used to replace embedded-app text.
  - Leave true remaining work visible: hosted dead-key/preedit runtime
    validation through the copied app and permission-dependent live global
    shortcut installation.

Out of scope:

- Changing copied Swift `keyDown` behavior.
- Calling `KeymapDarwin.translate` from `roastty_surface_key` or
  `roastty_app_key`.
- Adding new public C ABI.
- Full dead-key/IME UI automation. This experiment only proves the
  wrapper/raw-ABI text handoff and corrects the implementation target.
- Installing a permission-dependent live global event tap.

## Verification

- Run formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/139-keymap-text-scope.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Run targeted Rust tests:
  - `cargo test -p roastty surface_key_printable`
  - `cargo test -p roastty app_keymap`
  - `cargo test -p roastty keymap_darwin`
- Run the targeted hosted Swift test:
  - `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/SurfaceKeyTextTests`
- Run broader hosted Swift coverage:
  - `cd roastty && macos/build.nu --action test`
- Run full Roastty Rust coverage:
  - `cargo test -p roastty -- --test-threads=1`
- Run checks:
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/139-keymap-text-scope.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = the issue record and source comments accurately reflect upstream
embedded keymap scope, hosted tests prove copied Swift key-event wrappers
preserve app-provided text, Rust tests prove by-value `roastty_surface_key`
accepts UTF-8 text, and no copied-app logic or public ABI semantics change.

**Partial** = the scope correction is clear and Rust by-value coverage passes,
but the hosted Swift wrapper coverage cannot be made stable without a larger app
test harness.

**Fail** = the upstream embedded path actually requires Rust-side
`KeymapDarwin.translate` in `roastty_surface_key` / `roastty_app_key`, or the
existing wrapper/raw-ABI handoff cannot preserve app-provided key text.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Schrodinger`, fresh
context.

**Verdict:** Approved after fixes.

**Findings and fixes:**

- **Required:** The first design claimed the hosted test would prove the copied
  app's `SurfaceView_AppKit.keyDown` / AppKit text boundary, but the proposed
  test only covered `Roastty.Surface.sendKeyEvent` and the wrapper/raw ABI
  handoff. Fixed by narrowing the claim, changes, and pass criteria to the
  wrapper/raw-ABI boundary and leaving direct AppKit dead-key/IME automation as
  later work.
- **Required:** The README still listed Rust-side `KeymapDarwin` text
  translation as remaining native-key work, contradicting the experiment's
  central scope correction. Fixed by updating both the operating note and Phase
  G roadmap to say copied-app key text intentionally remains AppKit-provided,
  with hosted dead-key/preedit validation and live global shortcut installation
  still remaining.
- **Optional:** The existing `roastty_app_keyboard_changed` source comment still
  described full `KeymapDarwin` text translation as later work. Fixed by adding
  source-comment cleanup to this experiment's implementation scope.

The reviewer rechecked the fixes, confirmed no required findings remained, and
reported `git diff --check` plus the Prettier check passing.

## Result

**Result:** Pass

Implemented the scope correction without changing copied-app key handling or
public ABI semantics.

`roastty/macos/Tests/Roastty/SurfaceKeyTextTests.swift` now covers the hosted
Swift wrapper boundary: `Roastty.Input.KeyEvent.withCValue` preserves explicit
UTF-8 text, `composing`, modifier masks, consumed modifiers, unshifted
codepoint, and nil text while building the temporary C event passed to
`roastty_surface_key`.

`roastty/src/lib.rs` now has a by-value raw-ABI regression test,
`surface_key_by_value_utf8_reaches_child_pty`, which sends a multi-byte `é`
through `roastty_surface_key` with a C `RoasttyInputKey` and verifies the child
PTY receives the app-provided text. The stale `roastty_app_keyboard_changed`
comment now describes the actual embedded scope: the app-owned keymap is for
layout/reload state, while copied-app text remains AppKit / `interpretKeyEvents`
provided.

The first targeted hosted Swift run failed to compile because the test used an
invalid forced unwrap on `UnicodeScalar("e")`. Removing the unwrap fixed the
test source; the rerun passed.

Verification run:

- `cargo test -p roastty surface_key_printable` — passed
- `cargo test -p roastty app_keymap` — passed
- `cargo test -p roastty keymap_darwin` — passed
- `cargo test -p roastty surface_key_by_value_utf8_reaches_child_pty` — passed
- `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/SurfaceKeyTextTests`
  — passed, 2 tests in 1 suite after the compile fix
- `cd roastty && macos/build.nu --action test` — passed, 208 tests in 21 suites
- `cargo test -p roastty -- --test-threads=1` — passed, 4761 library tests, the
  ABI harness, and doc tests
- `cargo fmt --check` — passed
- `git diff --check` — passed

The Swift test output still includes pre-existing warnings/noise from the hosted
app build, including the SwiftLint opening-brace warning in
`SurfaceView_AppKit.swift`, a Sendable warning in `RoasttyPackage.swift`, and
App Intents `linkd` messages, but the commands exited successfully.

## Conclusion

The remaining Phase G native-key target is narrower than the prior issue notes
implied. `KeymapDarwin` belongs to app-owned layout/reload state and the
translation primitive; the copied embedded macOS app does not replace its
AppKit-generated text with Rust-side `KeymapDarwin.translate` inside
`roastty_surface_key` or `roastty_app_key`.

Remaining work is hosted dead-key/preedit runtime validation through the copied
app path and permission-dependent live global shortcut installation.

## Completion Review

**Reviewer:** Codex-native adversarial review subagent `Godel`, fresh context.

**Verdict:** Approved.

The reviewer reported no findings and no required fixes. It independently ran
`cargo fmt --check`, `git diff --check`,
`cargo test -p roastty surface_key_by_value_utf8_reaches_child_pty`, and
`cd roastty && macos/build.nu --action test --only-testing RoasttyTests/SurfaceKeyTextTests`,
all passing.

The reviewer also confirmed the upstream embedded key-text boundary:
app-provided text passes through `KeyEvent.core()`, while `KeymapDarwin` is
app-owned layout/reload state and a translation primitive, not a replacement for
copied AppKit text handling.

Codex-native adversarial completion review approved Experiment 139: the
implementation stayed within scope, the upstream embedded key-text boundary is
documented correctly, and the focused Rust/Swift verification passed.
