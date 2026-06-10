+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 59: Phase F — mouse behavior config

## Description

Experiments 56-58 made clipboard and selection behavior config app-facing. The
next narrow Phase-F slice is the adjacent mouse behavior group that Ghostty
stores on `Surface` and that Roastty already mostly models with hardcoded
defaults:

- `mouse-reporting`
- `mouse-scroll-multiplier`
- `click-repeat-interval`

Roastty currently has a `Surface::mouse_reporting` runtime flag, wheel-scrolling
paths, and click-repeat timing for selection gestures, but the values are
hardcoded (`mouse-reporting = true`, scroll multiplier effectively `1.0`, and
repeat-click interval `500ms`). This experiment represents the three fields on
aggregate config and routes them into those existing paths.

This experiment intentionally excludes cursor rendering config
(`cursor-opacity`, `cursor-style`, `cursor-style-blink`), cursor-click-to-move,
mouse-hide-while-typing, scroll-to-bottom, and renderer/app mouse visibility.
Those require different ownership and should be separate slices.

## Changes

- `roastty/src/config/mod.rs`
  - Add upstream defaults:
    - `mouse-reporting = true`
    - `mouse-scroll-multiplier = MouseScrollMultiplier::default()` with
      `precision = 1.0`, `discrete = 3.0`
    - `click-repeat-interval = 0`
  - Port a `MouseScrollMultiplier` leaf type matching upstream parse/format
    behavior:
    - bare float sets both precision and discrete;
    - `precision:<float>` and `discrete:<float>` update only that side;
    - comma-separated prefixed values are allowed;
    - unknown prefixes, invalid floats, empty segments, and missing values
      produce config diagnostics.
  - Route all three fields through `Config::set`, `format_config`, CLI/file
    loading, clone/equality, and diagnostics.
  - Extend `Config::finalize` for this mouse slice:
    - resolve `click-repeat-interval = 0` to `500` milliseconds for now,
      matching Ghostty's non-test fallback while deferring OS click-interval
      lookup to later platform/finalize work;
    - clamp `mouse-scroll-multiplier.precision` and
      `mouse-scroll-multiplier.discrete` to Ghostty's `[0.01, 10000.0]` range.
  - Preserve local/upstream formatter order around the mouse group:
    `mouse-shift-capture`, `mouse-reporting`, `mouse-scroll-multiplier`, and the
    existing click/copy group containing `click-repeat-interval` near
    `copy-on-select`.
  - Add aggregate config tests for defaults, parser routing, formatter order,
    CLI loading, file loading, invalid values, finalize resolution, and
    multiplier clamping.
- `roastty/src/lib.rs`
  - Cache `mouse-reporting`, `mouse-scroll-multiplier`, and
    `click_repeat_interval_ns = finalized_click_repeat_interval_ms * 1_000_000`
    on `Surface`, following the existing cached app/surface config update
    pattern.
  - Refresh cached mouse behavior through `roastty_app_update_config` and
    `roastty_surface_update_config`.
  - Initialize new surfaces from the app's parsed config snapshot.
  - Replace the hardcoded `mouse_reporting: true` surface default with
    configured `mouse-reporting`.
  - Replace hardcoded click-repeat timing in `should_shift_extend` and
    `selection_press` with the cached nanosecond interval from finalized config.
  - Rework scroll delta calculation to match upstream `Surface.scrollCallback`:
    compute vertical and horizontal `ScrollAmount`s once, then feed those deltas
    into the existing alt-screen cursor-key branch, mouse-reporting branch, and
    viewport-scrollback branch. Vertical precision scroll uses
    `mouse-scroll-multiplier.precision`; vertical discrete scroll uses
    `mouse-scroll-multiplier.discrete` and current cell height. Horizontal
    behavior remains upstream-shaped: non-precision horizontal scroll rounds the
    raw offset, while precision horizontal scroll accumulates pixels against the
    cell width. On macOS, keep Ghostty's discrete vertical minimum-magnitude
    behavior (`abs(yoff) >= 1`) when translating wheel ticks to pixels.
  - Do not change `toggle_mouse_reporting` semantics; it remains a runtime
    toggle from the configured/current state.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, add any durable operating note for mouse behavior
    config.

## Verification

- Run formatting:
  - `cargo fmt -- roastty/src/config/mod.rs roastty/src/lib.rs`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/59-mouse-behavior-config.md`
- Run targeted tests:
  - `cargo test -p roastty mouse_behavior_config`
  - `cargo test -p roastty mouse_scroll_multiplier`
  - `cargo test -p roastty mouse_behavior_finalize`
  - `cargo test -p roastty config_format_config`
  - `cargo test -p roastty mouse_scroll`
  - `cargo test -p roastty mouse_reporting`
  - `cargo test -p roastty double_click_word`
  - `cargo test -p roastty shift_click`
  - `cargo test -p roastty app_and_surface_update_config`
- Add concrete test cases proving:
  - `click-repeat-interval = 0` finalizes to `500` ms;
  - out-of-range `mouse-scroll-multiplier` values clamp low/high in finalize
    instead of failing parse;
  - configured vertical scroll multipliers affect viewport scrollback,
    alt-screen cursor-key repeats, and mouse-reporting wheel event counts;
  - horizontal wheel behavior remains upstream-shaped and does not use the
    vertical multiplier;
  - cached mouse behavior refreshes through both app and surface config updates.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = the three mouse behavior fields are represented on `Config`,
round-trip through config loading/formatting, refresh through app/surface config
updates, and replace only the intended mouse-reporting, wheel-scroll, and
click-repeat hardcoded behavior; targeted and full tests pass.

**Partial** = config representation and one runtime behavior land, but scroll
multiplier or click-interval fidelity exposes a bounded prerequisite such as OS
click interval lookup or precision-scroll classification.

**Fail** = current mouse/runtime ownership cannot safely route the fields
without a broader app mouse-state refactor.

## Design Review

Reviewed by Codex adversarial reviewer (`Erdos`,
`019eb33f-b84f-76a3-ae54-5de168aaa4f2`).

**Initial verdict:** Changes required.

- **Required:** The original design did not explicitly require upstream finalize
  behavior for the new mouse fields. Upstream resolves
  `click-repeat-interval = 0` during finalize and clamps both scroll multipliers
  to `[0.01, 10000.0]`.
- **Required:** The original verification did not prove finalize resolution,
  multiplier clamping, or multiplier effects across reporting, alt-scroll,
  viewport scrolling, and horizontal scroll behavior.
- **Optional:** The click interval runtime wording was easy to misimplement
  because upstream stores milliseconds in config and converts to nanoseconds on
  the surface.

Fix:

- Added explicit `Config::finalize` scope for `click-repeat-interval` resolution
  and `mouse-scroll-multiplier` clamping.
- Added concrete verification cases for finalize/clamping and the affected
  scroll/click runtime branches.
- Stated the cached surface click interval unit explicitly as nanoseconds:
  `click_repeat_interval_ns = finalized_click_repeat_interval_ms * 1_000_000`.

**Final verdict:** Approved.

No findings.
