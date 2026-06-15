# Experiment 154: Non-Glass Opacity Runtime

## Description

`RUNTIME-008B2B2B2` still groups several renderer-visible GUI effects together:
non-glass compositor opacity, GUI cursor pixels, custom shader output, broader
GUI/pixel parity, and screenshot-level padding pixel proof.

This experiment isolates the copied macOS host path for non-glass background
opacity. In pinned Ghostty, this behavior is implemented in the app host rather
than only in the renderer:

- `TerminalWindow.syncAppearance(_:)` makes regular terminal windows non-opaque
  when `background-opacity < 1`, avoids transparency in native fullscreen or
  when the user has toggled the window opaque, uses the 0.001 white background
  workaround, and applies non-glass window blur through the C ABI when the
  background blur mode is not glass;
- `TerminalWindow.preferredBackgroundColor` derives the alpha channel from the
  focused/top-left surface or window derived config and clamps it to
  `0.001...1`;
- `TransparentTitlebarTerminalWindow` applies the same preferred background
  color and alpha behavior to titlebar material when not using glass;
- `QuickTerminalController.syncAppearance()` preserves the same opacity/blur
  behavior for quick terminal windows.

This is narrower than GUI screenshot parity. It will not claim actual pixel
capture equivalence, GUI cursor pixels, custom shader output, screenshot-level
padding proof, or broader renderer visual parity.

## Changes

- Add a focused static parity guard:
  - `issues/0805-roastty-ghostty-parity/non_glass_opacity_runtime_parity.py`
  - Assert that pinned Ghostty and Roastty versions of `TerminalWindow.swift`,
    `TransparentTitlebarTerminalWindow.swift`, and
    `QuickTerminalController.swift` match after expected Ghostty-to-Roastty
    renames.
  - Assert the non-glass opacity markers in those files:
    `backgroundOpacity < 1`, `backgroundBlur.isGlassStyle`, `isOpaque = false`,
    `.white.withAlphaComponent(0.001)`, `ghostty_set_window_background_blur`,
    `roastty_set_window_background_blur`,
    `backgroundOpacity.clamped(to: 0.001...1)`, `preferredBackgroundColor`,
    `withAlphaComponent(alpha)`, and `isBackgroundOpaque` handling.
- Update `config_runtime_inventory.py` to split `RUNTIME-008B2B2B2` into:
  - an Oracle complete copied macOS non-glass opacity host row owned by this
    experiment;
  - a remaining renderer-visible visual gap row for GUI cursor pixels, custom
    shader output, broader GUI/pixel parity, and screenshot-level padding pixel
    proof.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update existing runtime parity guards and `terminal_runtime_residual_audit.py`
  for the new CFG-223 row counts and remaining renderer-visible gap id.
- Update Issue 805 learnings with the non-glass opacity finding after the result
  is known.

## Verification

Pass criteria:

- The new static non-glass opacity parity guard passes:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/non_glass_opacity_runtime_parity.py
```

- The existing macOS glass guard still passes, proving the adjacent glass slice
  remains separate:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_glass_visual_runtime_parity.py
```

- The runtime inventory generator reports one additional Oracle complete row and
  the same total number of unresolved CFG-223 gaps unless this experiment
  discovers a real fixable discrepancy. Expected output after this split:
  `runtime_rows=62`, `oracle_complete=56`, `closed=58`, `incomplete=4`, `gap=4`,
  and `cfg223=Gap`.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
```

- All runtime parity guards still pass:

```bash
for guard in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1
done
```

- The terminal residual audit still passes:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py
```

- Markdown and diff hygiene pass:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/154-non-glass-opacity-runtime.md \
  issues/0805-roastty-ghostty-parity/config-runtime-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Adversarial subagent `019eca16-0764-7a50-9982-9172974ff0c5` reviewed the design
with fresh context and returned `VERDICT: APPROVED`.

Required findings: none.

Optional findings:

- The initial marker list used `*_set_window_background_blur`, which was not a
  literal source marker. The design was tightened to name
  `ghostty_set_window_background_blur` and `roastty_set_window_background_blur`
  explicitly.
- The initial inventory pass criteria described the expected count change
  relatively. The design was tightened to state the expected post-split counts:
  `runtime_rows=62`, `oracle_complete=56`, `closed=58`, `incomplete=4`, `gap=4`,
  and `cfg223=Gap`.
