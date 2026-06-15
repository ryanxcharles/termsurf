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

## Result

**Result:** Pass

Experiment 177 added
`issues/0805-roastty-ghostty-parity/macos_window_padding_pixel_runtime.py`, a
live macOS GUI guard for screenshot-level `window-padding-*` pixel proof.

The guard launches the debug Roastty app with isolated config/defaults and large
asymmetric padding:

- `window-padding-x = 96,64`
- `window-padding-y = 72,136`
- `window-padding-balance = false`
- `window-padding-color = background`
- `macos-titlebar-style = hidden`

It creates a terminal running a deterministic Python painter. The painter writes
a marker file, hides the cursor, disables autowrap, paints the visible terminal
grid with a bright truecolor background, and sleeps. The guard waits for the
marker file before screenshot capture.

The screenshot proof follows the Experiment 176 exact-window pattern: it proves
the frontmost process is the launched debug-app PID, reads the focused
accessibility window bounds, maps those bounds to one PID-owned layer-0
CoreGraphics window, and captures that exact CGWindowID with `screencapture -l`.

The Swift sampler detects the broad bright terminal-content region using row and
column thresholds so small bright chrome artifacts, such as the debug-build
warning icon, cannot define the terminal content bounds. It then validates the
observed content edges against configured padding converted to screenshot pixels
with a narrow tolerance and derives sample rectangles from those expected edges.
The passing run proved:

- top, bottom, left, and right padding strips are background-dominant;
- content strips just inside all four padded-grid edges are bright-dominant;
- the screenshot is nonblank and tied to the focused Roastty window;
- the debug JSON records terminal id, command path, marker path, focused bounds,
  CGWindowID, configured padding, sample rectangles, and observed counts.

Debug artifacts from the passing run:

- `/tmp/termsurf-issue805-exp177-window-padding.png`
- `/tmp/termsurf-issue805-exp177-window-padding.json`

Latest focused guard output:

```text
macos_window_padding_pixel_runtime=pass terminal=7EE06896-1CA6-4FE5-85FE-370570BB1C78
```

Representative metric summary from the passing debug JSON:

```text
brightBounds = {x: 192, y: 208, width: 1279, height: 719}
gaps = {left: 192, right: 129, top: 208, bottom: 273}
expectedPaddingPixels = {left: 192, right: 128, top: 144, bottom: 272}
expectedEdges = {left: 192, top: 208, right: 1471, bottom: 927}
top/bottom/left/right padding samples: background-dominant
top/bottom/left/right content samples: bright-dominant
```

Inventory impact:

- Added `RUNTIME-008B2B2B2B2C` for focused live window-padding pixel proof.
- CFG-223 now has 83 runtime rows.
- CFG-223 now has 76 `Oracle complete` rows.
- CFG-223 now has 79 closed rows.
- CFG-223 still has 4 incomplete rows, all `Gap`.
- CFG-223 remains `Gap`.
- `RUNTIME-008B2B2B2B2B` remains open for actual app/GUI cursor pixels and
  broader GUI/pixel parity.

Verification run:

```bash
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_window_padding_pixel_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py \
  --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md \
  --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/platform_runtime_classification.py \
  --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md \
  --output issues/0805-roastty-ghostty-parity/platform-runtime-classification.md
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
```

Regenerated inventory output:

```text
runtime_rows=83
oracle_complete=76
closed=79
audit_covered=0
incomplete=4
gap=4
cfg223=Gap
```

## Conclusion

Focused live screenshot-level window-padding pixel proof is now covered. The
remaining renderer-visible gap no longer includes padding screenshot proof; it
still needs actual app/GUI cursor screenshots and broader GUI/pixel parity.

## Result Review

Fresh-context adversarial reviewer `Bernoulli the 3rd` reviewed the completed
experiment result and initially returned `CHANGES REQUIRED`.

Required finding:

- The first completed guard only required observed gaps to be at least a
  fraction of configured padding, then anchored padding/content sample
  rectangles to the measured bright bounds. That proved some padding existed,
  but did not prove the content was at the configured padding edges.

First fix:

- The sampler added explicit expected-edge checks and recorded expected edges in
  debug JSON. The reviewer correctly found that the right/bottom slack terms
  were still derived from observed `maxX`/`maxY`, making those edges circular.

Final fix:

- The sampler no longer computes observed-derived slack. It computes expected
  right and bottom edges directly from screenshot dimensions and configured
  padding, compares observed right/bottom gaps directly to configured
  screenshot-pixel padding with a narrow tolerance, and anchors the right/bottom
  sample rectangles to those independently computed edges.
- The live config now uses asymmetric but fixed-window-compatible padding:
  `window-padding-x = 96,64` and `window-padding-y = 72,136`.

Re-review verdict: **Approved**. The reviewer confirmed the prior required
finding was resolved and that no new required findings were introduced.

Final result verdict: **Approved**.
