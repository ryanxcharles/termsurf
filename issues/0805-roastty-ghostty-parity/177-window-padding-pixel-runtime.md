# Experiment 177: Window Padding Pixel Runtime

## Description

`RUNTIME-008B2B2B2B2B` still owns screenshot-level padding pixel proof. Earlier
experiments proved deterministic padding math, renderer uniform construction,
and padding-color shader mechanics, but they did not prove that a real macOS app
window visibly places terminal content away from the configured padding edges.

This experiment will split out one focused live GUI slice: asymmetric
`window-padding-x` and `window-padding-y` produce observable padding pixels in
an exact Roastty window screenshot. It will not claim GUI cursor pixel parity,
full renderer pixel parity, broad font output parity, or full CFG-223
completion.

## Changes

- New guard script:
  `issues/0805-roastty-ghostty-parity/macos_window_padding_pixel_runtime.py`
  - Launch the debug `roastty/macos/build/Debug/Roastty.app` with isolated
    config and defaults.
  - Use a controlled config with:
    - `macos-applescript = true`
    - `quit-after-last-window-closed = true`
    - `font-size = 16`
    - `cursor-style-blink = false`
    - `background = #102030`
    - `foreground = #ffffff`
    - `window-padding-color = background`
    - large asymmetric `window-padding-x` and `window-padding-y` values.
  - Create a terminal running a deterministic child process that writes a marker
    file, paints a bright truecolor rectangle across the terminal grid, and then
    sleeps. The guard must wait for the marker file and record the terminal id,
    command path, and marker path in debug JSON.
  - Prove the screenshot target is the exact launched debug-app PID and the
    captured CGWindowID maps to the focused accessibility window, following the
    stricter Experiment 176 pattern.
  - Capture the exact window with `screencapture -l`.
  - Sample stable regions in the captured PNG:
    - configured top, bottom, left, and right padding regions must be
      background-dominant;
    - terminal content regions just inside the expected padded grid on each side
      must be bright-content-dominant;
    - all sample rectangles must be derived from the captured image dimensions,
      focused-window bounds, configured padding values, and measured/expected
      cell geometry rather than hard-coded lucky regions;
    - the debug JSON must record every sample rectangle, expected color class,
      observed color counts, screenshot dimensions, focused accessibility
      bounds, and captured CGWindowID;
    - the guard must fail if the content reaches the padded edges or if the
      screenshot is blank/wrong-window.
  - Save debug PNG/JSON artifacts under `/tmp/termsurf-issue805-exp177-*`.

- Inventory: `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-008B2B2B2B2B` into:
    - a new `Oracle complete` row for focused live screenshot-level window
      padding pixel proof;
    - the remaining gap row for GUI cursor pixels, broader GUI/pixel parity, and
      renderer-visible effects outside this focused padding proof.
  - Update `EXPECTED_IDS` and CFG-223 counts only for the new passing row.
  - Keep CFG-223 as `Gap`.

- Existing guard scripts:
  - Update expected CFG-223 count text from 75/78 to the new generated counts if
    the new row is added and passing.
  - Narrow stale wording from "screenshot-level padding pixel proof" to broader
    remaining renderer/GUI pixel gaps in scripts that inspect the remaining gap
    row.

- Issue docs:
  - Update this experiment from `Designed` to `Pass`/`Partial`/`Fail` after
    verification.
  - Add a focused learning to the issue README only if the live screenshot guard
    teaches a reusable technique or limitation.

## Verification

- Build the macOS app:

```bash
(cd roastty && macos/build.nu --action build)
```

- Regenerate CFG-223 inventory and matrix:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py \
  --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md \
  --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/platform_runtime_classification.py \
  --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md \
  --output issues/0805-roastty-ghostty-parity/platform-runtime-classification.md
```

- Run the existing CFG-223 guard set:

```bash
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py \
  issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py \
  issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py \
  issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py \
  issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$f" || exit 1
done
```

- Run the live macOS guard subset needed for this issue family:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_window_padding_pixel_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_titlebar_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_split_layout_runtime.py
```

- Syntax, formatting, and hygiene:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/177-window-padding-pixel-runtime.md \
  issues/0805-roastty-ghostty-parity/config-runtime-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md \
  issues/0805-roastty-ghostty-parity/platform-runtime-classification.md
git diff --check
```

Pass criteria:

- The new guard passes only after proving exact debug-app launch, isolated
  config/defaults, no new crash report, frontmost/main-window evidence,
  accessibility-focused-window to CGWindowID mapping, exact-window screenshot
  capture, command-marker evidence, geometry-derived sample rectangles,
  background-dominant top/bottom/left/right configured padding regions, and
  content-dominant grid regions just inside each expected padding edge.
- The pixel oracle cannot pass if the screenshot is blank, captures the wrong
  process/window, paints only background, paints only terminal content, or lets
  terminal content reach any sampled padding edge.
- The new inventory row claims only focused screenshot-level window padding
  pixel proof.
- The remaining `RUNTIME-008B2B2B2B2B` row still owns GUI cursor pixels, broader
  GUI/pixel parity, and renderer-visible effects outside this focused padding
  proof.
- CFG-223 remains `Gap`.

Fail criteria:

- The guard can pass without exact-CGWindowID screenshot evidence tied to the
  focused accessibility window.
- The guard can pass without command-marker evidence that the deterministic
  painter ran.
- The guard can pass without geometry-derived sample rectangles recorded in
  debug JSON.
- The guard can pass without positive content-region samples and negative
  padding-region samples for all four configured padding edges in the same
  screenshot.
- The guard relies on an installed app or non-isolated user config/defaults.
- The experiment claims full renderer pixel parity, GUI cursor parity, broad
  font output parity, or CFG-223 completion.

## Design Review

Fresh-context adversarial reviewer `Ramanujan the 3rd` reviewed the initial
design and returned `CHANGES REQUIRED`.

Required finding:

- The initial pixel oracle claimed asymmetric `window-padding-x` and
  `window-padding-y` proof but only required top/left padding samples, so
  right/bottom live padding pixels could be broken while the guard still passed.

Optional finding:

- The deterministic content paint was under-specified. The first design did not
  require command-marker proof that the child process ran or geometry-derived
  sample rectangles recorded in debug JSON.

Fixes made:

- The design now requires background-dominant samples for top, bottom, left, and
  right padding regions, plus content-dominant samples just inside each expected
  padded-grid edge.
- The guard must wait for a marker file from the deterministic painter and
  record terminal id, command path, marker path, sample rectangles, expected
  color class, observed color counts, screenshot dimensions, focused
  accessibility bounds, and captured CGWindowID in debug JSON.
- The duplicate standalone guard run was removed from the verification list.

Re-review verdict: **Approved**. The reviewer confirmed the all-four-edge
padding requirement, command-marker evidence, geometry-derived sample rectangle
requirements, README link, and `Designed` status.

Final design verdict: **Approved**.
