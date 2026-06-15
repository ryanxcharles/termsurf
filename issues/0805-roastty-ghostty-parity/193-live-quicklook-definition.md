# Experiment 193: Live Quick Look definition dispatch

## Description

Experiment 192 split live inactive-app Dock attention request dispatch out of
the remaining `RUNTIME-012B2B2B2B2B3C` gap. The residual row still includes
actual OS notification delivery/banner/sound, audible bell output, OS-visible
Dock attention bounce/state, Quick Look/native preview display, and external
Launch Services handler delivery.

This experiment targets only the Quick Look/native definition slice. Pinned
Ghostty and Roastty both implement `SurfaceView.quickLook(with:)` by reading the
word under the current mouse position through the embedded runtime, converting
the returned terminal coordinates to an AppKit point, and calling
`showDefinition(for:at:)` on the live AppKit surface view.

The expected outcome is either a new Oracle-complete row for live Quick
Look/definition request dispatch, or, if the VM exposes a deterministic native
popover/sheet, a stronger row for visible native definition UI. The experiment
must not claim URL hover banner behavior, external URL handler delivery,
notification delivery, audible sound, or Dock bounce behavior.

## Changes

- Add trace-only instrumentation to
  `roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift` around
  `quickLook(with:)`.
  - Preserve the production Ghostty-compatible behavior and the
    `showDefinition(for:at:)` call.
  - Record the selected text, text length, top-left terminal pixel coordinates,
    converted AppKit point, and whether the optional Quick Look font pointer was
    present.
  - Record explicit fallback reasons when the surface is missing, word lookup
    fails, or the selected text is empty.
- Audit and, if necessary, repair `roastty_surface_quicklook_font` so the Quick
  Look attributed string matches pinned Ghostty's CoreText path.
  - Pinned Ghostty returns a copied primary `CTFont` scaled by content scale
    when the CoreText backend is active.
  - Roastty currently returns null from this ABI; the experiment may not close a
    full Quick Look request-dispatch row unless the live trace proves
    `fontPresent=true` or the result explicitly leaves the missing font
    attribute as a remaining Quick Look gap.
- Add an env-gated AppleScript test action, tentatively `ui_test_quicklook`,
  through `roastty/macos/Sources/Features/AppleScript/ScriptTerminal.swift`.
  - Require `ROASTTY_UI_TEST_ENABLE_QUICKLOOK_ACTION=1`.
  - Invoke a new `@objc` method on the live `SurfaceView` that creates a
    synthetic Quick Look `NSEvent` and calls the same `quickLook(with:)`
    override used by the real app.
  - The hook must be test-only, opt-in, and unavailable unless AppleScript is
    enabled and the environment variable is set.
- Add a focused live guard, tentatively
  `issues/0805-roastty-ghostty-parity/macos_live_quicklook_definition.py`.
  - Launch the built debug Roastty app with isolated config/defaults and
    `ROASTTY_UI_KEY_TRACE_PATH`.
  - Start a real terminal command that paints a deterministic dictionary word,
    such as `serendipity`, at a known row/column and writes a ready marker.
  - Use the existing Swift mouse injection helper to move over the word so
    `roastty_surface_quicklook_word` uses a real mouse position.
  - Invoke `perform action "ui_test_quicklook"` on the focused AppleScript
    terminal.
  - Require trace evidence that the live app selected the exact expected word
    and reached the `showDefinition(for:at:)` call.
  - Attempt to capture before/after screenshots of the focused window and/or
    whole screen after the call. If a native definition popover is
    deterministically visible, record localized pixel deltas and split a
    visible-native-UI row. If no native popover is visible in the VM, record
    that limitation and split only the request-dispatch row.
  - Check for new Roastty crash reports.
- Add or update a parity guard so the experiment checks that pinned Ghostty and
  Roastty still share the same Quick Look source shape: `quickLook(with:)`,
  embedded `*_surface_quicklook_word`, CoreText-backed
  `*_surface_quicklook_font` behavior or an explicit residual classification for
  the missing font attribute, coordinate conversion, and
  `showDefinition(for:at:)`.
- Update `config_runtime_inventory.py` according to the outcome:
  - If only request dispatch is proven, split a new Oracle-complete row from
    `RUNTIME-012B2B2B2B2B3C` for live Quick Look/definition request dispatch and
    keep visible native preview display in the residual gap.
  - If visible native UI pixels are also proven, split the stronger visible
    native Quick Look/definition row and remove that exact slice from the
    residual gap.
  - Keep `RUNTIME-012B2B2B2B2B3C` as a `Gap` for any unproven OS-controlled
    notification, audible bell, Dock bounce, native preview, and external Launch
    Services behavior.
- Update residual guards and stale CFG-223 counts if a new runtime row is split.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update Issue 805 `README.md` Learnings and Experiments index after the result
  is known.

## Verification

Pass criteria:

- The guard proves exact debug-app launch, isolated config/defaults, terminal
  marker evidence, focused-window geometry, and no new Roastty crash report.
- The guard proves a real mouse-position-dependent Quick Look lookup by moving
  over the deterministic word before invoking the test action.
- The guard proves the live Quick Look path selected the exact expected word and
  called `showDefinition(for:at:)` with a converted AppKit point.
- A full Quick Look request-dispatch pass requires the guard to prove
  `fontPresent=true` on this macOS/CoreText build, matching pinned Ghostty's
  copied primary-font behavior. If `fontPresent=false`, the result may only
  split a narrower unfonted word-lookup/showDefinition dispatch row and must
  keep the missing Quick Look font attribute in `RUNTIME-012B2B2B2B2B3C`.
- If the result claims visible native definition UI, that claim must be backed
  by before/after screenshot evidence with localized pixel deltas that cannot be
  explained by terminal repaint alone.
- If the VM does not expose a deterministic visible native popover, the result
  must say so explicitly and must not claim native preview display.
- The experiment result does not claim actual OS notification delivery,
  notification banner/sound, audible bell output, OS-visible Dock bounce/state,
  or external URL delivery.
- Inventory counts and remaining gap IDs are updated exactly and asserted by
  guards.

Commands:

```bash
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_live_quicklook_definition.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/notification_link_bell_gui_residual_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/193-live-quicklook-definition.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

The result must state the exact runtime row count, Oracle-complete count, closed
count, incomplete count, gap count, CFG-223 status, and remaining gap IDs.

## Design Review

Fresh-context Codex adversarial reviewer `Banach the 3rd` reviewed the initial
design against the Issue 805 workflow, the remaining CFG-223 residual, and the
pinned Ghostty/Roastty Quick Look sources.

Initial verdict: **Changes required**.

Required finding accepted and fixed: the original design treated the Quick Look
font pointer as optional and could have passed while Roastty still returned null
from `roastty_surface_quicklook_font`. Pinned Ghostty returns a copied primary
CoreText font for Quick Look on the macOS/CoreText build. The design now
requires `fontPresent=true` for a full Quick Look request-dispatch pass, or else
requires the result to split only a narrower unfonted dispatch row and keep the
missing font attribute in the residual gap.

Re-review verdict after the fix: **Approved**.
