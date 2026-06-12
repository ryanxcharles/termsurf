# Experiment 140: Phase G — hosted preedit surface view

## Description

Add hosted macOS test coverage for the copied app's `NSTextInputClient`
marked-text / preedit handoff.

Experiment 139 corrected the native-key scope: the copied embedded app keeps
using AppKit / `interpretKeyEvents` to produce text, while Rust `KeymapDarwin`
is app-owned layout/reload state and a translation primitive. The remaining
native-key gap is now hosted dead-key/preedit runtime validation through the
copied app path.

This experiment takes the smallest direct step at that gap. It does not try to
automate a real keyboard layout switch, dead-key sequence, or IME session.
Instead, it exercises the `NSTextInputClient` methods that AppKit calls during
those sessions:

- `setMarkedText(_:selectedRange:replacementRange:)`
- `hasMarkedText()`
- `markedRange()`
- `unmarkText()`
- `insertText(_:replacementRange:)` only if it can be tested without synthetic
  event fragility

The key behavior to prove is that a real hosted `Roastty.SurfaceView` can accept
marked text, reflect that state through the copied Swift methods, and push the
preedit string through `roastty_surface_preedit` into the underlying
`libroastty` surface. The concrete C-facing observation point is
`roastty_surface_ime_point`: after initializing nonzero surface cell geometry,
its `width` should reflect the marked-text character count times the cell width,
then return to zero after `unmarkText()` clears preedit. Clearing marked text
should clear both the Swift marked state and the Rust surface preedit state.

If a direct `Roastty.SurfaceView` construction is unstable under the hosted
non-UI test runner, add the narrowest test seam needed in
`SurfaceView_AppKit.swift` to expose the marked-text/preedit transition without
changing copied app behavior. Any such seam must be internal/testable, match the
existing copied logic, and leave production behavior unchanged.

## Changes

- `roastty/macos/Tests/Roastty/SurfaceViewAppKitTests.swift`
  - Add hosted tests that construct a temporary `roastty_app_t` with
    `TemporaryConfig`, create a `Roastty.SurfaceView`, and exercise
    `setMarkedText` / `unmarkText`.
  - Assert the Swift `NSTextInputClient` state changes: `hasMarkedText()` and
    `markedRange()` report the marked text length after `setMarkedText`, then
    clear after `unmarkText`.
  - Assert the underlying surface preedit state through
    `roastty_surface_ime_point`: configure nonzero surface/cell dimensions, set
    marked text with a distinct character count, verify the reported IME width
    becomes nonzero and matches the expected preedit width, then verify it
    returns to zero after `unmarkText`.
  - Cover both plain `String` and `NSAttributedString` marked text if direct
    construction remains stable.
  - Add `insertText` coverage only if the test can supply a legitimate current
    AppKit event without broad UI automation or brittle global state.
- `roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift`
  - Prefer no production changes.
  - If direct hosted testing is blocked by private state or the lack of a stable
    observation point, add only a narrow internal test helper around the
    existing marked-text/preedit logic. Do not change `keyDown`,
    `interpretKeyEvents`, `syncPreedit`, or public app behavior.
- `roastty/src/lib.rs`
  - Prefer no Rust changes.
  - If the hosted test needs a stable C-facing observation point for preedit
    state beyond `roastty_surface_ime_point`, add a narrow test-only helper only
    after proving the current C-facing IME point cannot observe preedit
    reliably.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After the result, update Phase G notes to distinguish this hosted
    `NSTextInputClient` preedit coverage from full dead-key/IME UI automation
    and from permission-dependent live global shortcut installation.

Out of scope:

- Real dead-key or IME UI automation.
- Changing keyboard layouts in the test.
- Installing or authorizing a live global event tap.
- Changing copied `keyDown` behavior or replacing AppKit text generation with
  Rust `KeymapDarwin.translate`.
- Adding new public C ABI unless direct evidence shows no existing observation
  path can prove the preedit state.

## Verification

- Format markdown:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/140-hosted-preedit-surface-view.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Run targeted hosted Swift tests:
  - `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/SurfaceViewAppKitTests`
- Run broader hosted Swift coverage:
  - `cd roastty && macos/build.nu --action test`
- Run Swift lint after Swift edits:
  - `cd roastty/macos && swiftlint`
- If Rust code changes, format Rust:
  - `cargo fmt`
- If any Rust or C-facing observation code changes, run targeted Rust tests:
  - `cargo test -p roastty preedit`
  - `cargo test -p roastty surface_preedit`
- Run full Roastty Rust coverage if Rust code changes:
  - `cargo test -p roastty -- --test-threads=1`
- Run checks:
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/140-hosted-preedit-surface-view.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = hosted app tests prove `SurfaceView` marked-text state updates and
clears, and `roastty_surface_ime_point` proves the underlying
`roastty_surface_preedit` state updates to the expected width and clears through
the copied `NSTextInputClient` path, with no behavior changes outside a narrow
test seam if one is required.

**Partial** = the copied Swift marked-text state is hosted-test verified, but
the underlying Rust preedit observation point cannot be made stable without a
larger UI harness or new public ABI.

**Fail** = direct hosted `SurfaceView` construction is not viable, a narrow test
seam would alter production behavior, or the copied preedit path does not update
the underlying `libroastty` surface state.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Aquinas`, fresh context.

**Verdict:** Approved after fixes.

**Findings and fixes:**

- **Required:** The first design left the underlying Rust preedit proof too
  vague by saying tests could read rendered text or another C-facing observation
  point. The reviewer noted that `roastty_surface_preedit` stores surface
  preedit separately from terminal grid text, and that the existing concrete
  C-facing signal is `roastty_surface_ime_point` width. Fixed by making
  `roastty_surface_ime_point` the required observation path, with nonzero
  surface/cell geometry, expected width from marked-text character count times
  cell width, and a zero-width clear check after `unmarkText()`.
- **Required:** The first design omitted Swift lint despite editing Swift tests
  and possibly `SurfaceView_AppKit.swift`. Fixed by adding
  `cd roastty/macos && swiftlint` after Swift edits.
- **Optional:** The first design allowed possible Rust edits but only listed
  `cargo fmt --check`. Fixed by adding conditional `cargo fmt` when Rust code
  changes.

The reviewer rechecked the fixes, confirmed each prior finding was resolved, and
reported no new required findings.

## Result

**Result:** Pass

Implemented hosted `SurfaceView` preedit coverage and fixed the Rust-side cell
metric propagation needed for `roastty_surface_ime_point` to observe preedit
width in a real hosted app surface.

`roastty/macos/Tests/Roastty/SurfaceViewAppKitTests.swift` now constructs real
hosted `Roastty.SurfaceView` instances backed by temporary `roastty_app_t`
values, calls the copied `NSTextInputClient` marked-text methods, and verifies:

- plain `String` marked text updates `hasMarkedText()` and `markedRange()`;
- `NSAttributedString` marked text follows the same path;
- `roastty_surface_ime_point` reports nonzero preedit width equal to the current
  surface cell width times the marked text's display-cell width;
- `unmarkText()` clears both Swift marked-text state and the underlying IME
  preedit width back to zero.

The first hosted test draft proved direct `SurfaceView` construction was stable
but failed because `roastty_surface_ime_point` reported width `0`: Roastty's
public app-facing size path updated pixel size but never copied live font cell
metrics into `RoasttySurfaceSize`. `roastty/src/lib.rs` now copies the live
renderer `SharedGrid` cell size into `surface.size.cell_width_px` /
`cell_height_px` and derives `columns` / `rows` after the live renderer exists.
That matches the upstream `ghostty_surface_size` / `Surface.imePoint` contract,
where IME width is based on preedit cell width and the surface's current cell
metrics.

After that fix, the hosted test expected the real `roastty_surface_size`
`cell_width_px` instead of a hand-set Swift `cellSize`, and the targeted suite
passed.

The completion reviewer then found a real gap: Roastty's public IME point used
Unicode scalar count for preedit width, while upstream `Surface.imePoint` uses
the preedit display-cell width. `roastty/src/lib.rs` now computes the public
preedit width with `unicode-width`, the hosted Swift test expects `"かな"` to
occupy four terminal cells, and the Rust ABI-facing unit coverage verifies
`roastty_surface_preedit("かな")` reports `4 * cell_width`.

Verification run:

- `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/SurfaceViewAppKitTests`
  — passed, 5 tests in 1 suite
- `cd roastty/macos && swiftlint` — exited 0; it still reports the pre-existing
  warning at `SurfaceView_AppKit.swift:738`
- `cargo test -p roastty preedit` — passed, 45 library tests plus the filtered
  ABI harness
- `cargo test -p roastty surface_preedit` — passed, 10 library tests plus the
  filtered ABI harness
- `cd roastty && macos/build.nu --action test` — passed, 210 tests in 21 suites
- `cargo test -p roastty -- --test-threads=1` — passed, 4762 library tests, the
  ABI harness, and doc tests
- `cargo fmt --check` — passed
- `git diff --check` — passed

The hosted Swift runs still include pre-existing warnings/noise: the SwiftLint
opening-brace warning in `SurfaceView_AppKit.swift:738`, Swift 6 Sendable /
main-actor warnings, Main Thread Checker messages in unrelated SplitTree tests,
App Intents `linkd` messages, and pasteboard background-thread warnings. The
commands exited successfully.

## Conclusion

This experiment proves the copied app's hosted `NSTextInputClient` marked-text
path updates both Swift marked state and the underlying `libroastty` preedit
state observable through `roastty_surface_ime_point`.

It also fixes a real app-facing size gap: live renderer cell metrics now
populate `roastty_surface_size`, which is required for IME geometry and
downstream resize/overlay behavior. Full dead-key/IME UI automation remains
later work, as does permission-dependent live global shortcut installation.

## Completion Review

Codex-native adversarial reviewer `Pauli` initially reported **Changes
Required**: public IME geometry counted Unicode scalars, while upstream counts
preedit display cells. The experiment was fixed to compute display-cell width
with `unicode-width`, expect `"かな"` to occupy four cells in hosted Swift
coverage, and verify the raw `roastty_surface_preedit` /
`roastty_surface_ime_point` path from Rust.

Codex-native adversarial reviewer `Kuhn` re-reviewed the completed experiment
after those fixes and reported:

**Verdict:** Approved

Required findings: none.
