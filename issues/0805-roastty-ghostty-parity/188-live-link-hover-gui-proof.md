# Experiment 188: Live link hover GUI proof

## Description

Experiment 187 left one CFG-223 gap, `RUNTIME-012B2B2B2B2B3C`. The remaining
link-related slices are:

- real link hover/cursor pixels;
- native link preview display.

Earlier experiments already proved the copied macOS hover-banner source plumbing
and the deterministic surface-side `mouse_shape` / `mouse_over_link` dispatch.
This experiment will drive a real debug Roastty window with a deterministic
terminal link under the mouse and try to prove the live GUI bridge from mouse
movement to visible or traceable hover effects.

The primary success path is to close the live link-hover/banner slice with a
guard that proves:

- the mouse was moved over a known link cell in the exact debug-app window;
- the app received the expected pointer-shape request;
- the app received the expected hovered URL;
- the URL hover banner is present in the live window, preferably by screenshot
  evidence, or by a narrowly env-gated SwiftUI/app trace if screenshot evidence
  is not deterministic in this VM.

The experiment will not claim OS notification delivery, audible bell output,
dock attention, bell border/title pixels, or external Launch Services handler
delivery. If true OS cursor pixels or native preview display cannot be proven
deterministically, those slices must remain explicit gaps.

## Changes

- Add a focused live guard, tentatively
  `issues/0805-roastty-ghostty-parity/macos_live_link_hover_runtime.py`.
  - Launch the built debug `Roastty.app` with isolated config/defaults and
    `link-previews = true`.
  - Create a terminal running a deterministic child process that prints a known
    URL at a known row/column, writes a marker file, and sleeps.
  - Use the proven accessibility/CGWindowID targeting pattern from the titlebar,
    window-padding, cursor-pixel, and native-menu guards.
  - Move the mouse to the known link cell using the existing CGEvent injection
    helper or System Events path, with exact-window coordinate evidence.
  - Wait for trace evidence that `cursorShape` changed to the pointer shape and
    `setMouseOverLink` received the exact URL. Add a narrowly env-gated trace in
    `Roastty.App.setMouseOverLink` only if the current trace surface does not
    already expose the hovered URL.
  - Attempt an OCR-free screenshot proof of the bottom URL hover banner: derive
    the terminal/window geometry, sample the expected bottom banner region, and
    fail if the screenshot is blank, from the wrong window, or lacks
    non-background/banner pixels. If the URL text cannot be proven by pixels,
    record that limitation and keep native preview display as a gap.
  - Check for new Roastty crash reports.
- Update `config_runtime_inventory.py` according to the outcome:
  - If live hover URL and pointer-shape request proof passes but screenshot
    banner/native preview proof does not, split out an Oracle-complete
    live-hover-dispatch-to-app row and leave a narrower visual/native preview
    gap.
  - If screenshot banner proof also passes, split out the live hover-banner
    display row and leave only the genuinely unproven OS-controlled slices.
  - Do not collapse the remaining gap unless every listed OS-controlled effect
    has deterministic evidence.
- Update `notification_link_bell_gui_residual_parity.py` and any stale CFG-223
  count assertions to enforce the new split.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update Issue 805 `README.md` Learnings and Experiments index after the result
  is known.

## Verification

Pass criteria:

- The live guard proves exact debug-app launch, isolated config/defaults,
  focused-window-to-CGWindowID mapping, command-marker evidence, and no new
  Roastty crash report.
- The guard proves the mouse moved over the intended link cell, not just
  somewhere in the window.
- The guard proves the expected URL reached the live app hover path and that the
  pointer-shape request was emitted.
- Any claim of visible URL hover-banner/native preview display must be backed by
  deterministic screenshot/pixel evidence or a specifically env-gated app trace
  that cannot fire without the SwiftUI/app display path being evaluated.
- Inventory counts and remaining gap IDs are updated exactly and asserted by
  guards.

Commands:

```bash
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_live_link_hover_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/notification_link_bell_gui_residual_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for guard in issues/0805-roastty-ghostty-parity/*_parity.py issues/0805-roastty-ghostty-parity/*_residual_audit.py issues/0805-roastty-ghostty-parity/macos_*_runtime.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1
done
python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/188-live-link-hover-gui-proof.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

The result must state the exact runtime row count, Oracle-complete count, closed
count, incomplete count, gap count, CFG-223 status, and remaining gap IDs.

## Design Review

Fresh-context Codex adversarial reviewer `Carver the 3rd` reviewed the design
against the issue workflow, prior link-hover experiments, relevant macOS source,
guard coverage, scope, and overclaiming risks.

Verdict: **Approved**.

Findings: none.

## Result

**Result:** Partial

The live link-hover dispatch slice is now proven in the real debug macOS app,
and the experiment found and fixed a Roastty parity bug.

Roastty's copied macOS app sends mouse coordinates in AppKit points, but
Roastty's renderer/grid geometry is stored in backing pixels. Before this
experiment, live mouse movement reached `SurfaceView_AppKit.mouseMoved`, and
Command modifiers arrived correctly, but the Rust core mapped the point-space
coordinates directly against pixel-space grid geometry. On the 2x VM display,
that missed the intended link cell and prevented live link hover dispatch.

The fix applies the surface content scale before point-to-cell conversion in the
Rust surface mouse paths. The regression guard
`link_hover_preview_dispatch_scales_macos_point_coordinates` proves a
point-space mouse coordinate only reaches the URL cell after the 2x scale is
applied.

`macos_live_link_hover_runtime.py` now launches the built debug app with an
isolated config, creates a live terminal, prints
`https://example.com/issue805-exp188-link-hover`, injects Command-modified mouse
movement into the focused CGWindowID, and records the live app trace:

- `mouseMoved ... mods=1048576`
- `cursorShape raw=3 pointerStyle=link`
- `mouseOverLink url=https://example.com/issue805-exp188-link-hover`

This proves live regular-link hover dispatch, cursor-shape request construction,
and exact hovered-URL routing to the macOS app. It does not prove native link
preview display or real OS cursor pixels.

Inventory outcome:

- `runtime_rows=92`
- `oracle_complete=88`
- `closed=91`
- `audit_covered=0`
- `incomplete=1`
- `gap=1`
- `cfg223=Gap`
- Remaining gap ID: `RUNTIME-012B2B2B2B2B3C`

Verification performed:

```bash
cargo test --manifest-path roastty/Cargo.toml link_hover_preview_dispatch -- --test-threads=1
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_live_link_hover_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/notification_link_bell_gui_residual_parity.py
python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/188-live-link-hover-gui-proof.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

Evidence logs:

- `logs/issue805-exp188-link-hover-rust-tests-2.log`
- `logs/issue805-exp188-link-hover-scale-test-1.log`
- `logs/issue805-exp188-build-5.log`
- `logs/issue805-exp188-live-link-hover-7.log`
- `logs/issue805-exp188-live-link-hover-rerun-1.log`
- `logs/issue805-exp188-config-runtime-inventory-2.log`
- `logs/issue805-exp188-residual-guard-2.log`
- `logs/issue805-exp188-py-compile-2.log`
- `logs/issue805-exp188-prettier-check-2.log`
- `logs/issue805-exp188-diff-check-1.log`
- `logs/issue805-exp188-broad-guard-sweep-failfast-3.log`

## Conclusion

Experiment 188 closes the live link-hover dispatch part of
`RUNTIME-012B2B2B2B2B3C` and splits it into the new Oracle-complete row
`RUNTIME-012B2B2B2B2B3C3`.

The remaining CFG-223 gap is narrower: actual OS notification
delivery/banner/sound, audible bell output, measurable dock-attention state,
bell border/title visible effects, real OS cursor pixels, native link preview
display, and external Launch Services handler delivery.

## Completion Review

Fresh-context Codex adversarial reviewer `Hypatia the 3rd` reviewed the
completed experiment, implementation diff, issue workflow, matrix updates, and
verification evidence.

Verdict: **Approved**.

Findings: none.
