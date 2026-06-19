# Experiment 1: Port Split Pane Border Config

## Description

Restore the Ghostboard Legacy split-pane visual settings in the current
`ghostboard/` codebase:

- `focused-split-border-color`
- `unfocused-split-border-color`
- `split-border-width`
- `unfocused-split-saturation`

The current Ghostboard tree already has the same useful seams as the legacy
implementation:

- `ghostboard/src/config/Config.zig` has upstream split visual settings near
  `unfocused-split-opacity`, `unfocused-split-fill`, and `split-divider-color`;
- `ghostboard/macos/Sources/Ghostty/Ghostty.Config.swift` already bridges those
  settings through `ghostty_config_get`;
- `ghostboard/macos/Sources/Features/Splits/TerminalSplitTreeView.swift` passes
  `isSplit: !isRoot` into each split leaf;
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView.swift` already has
  `SurfaceWrapper.isSplit`, SwiftUI focus state, a last-focused-surface
  fallback, and an unfocused split overlay.

This experiment should port the legacy behavior into those current seams, not
introduce a new split rendering architecture. The border should stay a SwiftUI
overlay with hit testing disabled, while the terminal surface should be inset by
the configured border width so terminal content and TermSurf browser overlays
remain readable and aligned.

If runtime evidence shows that insetting `SurfaceRepresentable` changes browser
overlay geometry incorrectly in the current Ghostboard architecture, this
experiment should stop and record the failure before designing a separate
geometry-aware fix.

## Changes

Planned files:

- `ghostboard/src/config/Config.zig`
  - add `focused-split-border-color: ?Color = null`;
  - add `unfocused-split-border-color: ?Color = null`;
  - add `split-border-width: f64 = 0`;
  - add `unfocused-split-saturation: f64 = 1.0`;
  - clamp `split-border-width` to `0...10`;
  - clamp `unfocused-split-saturation` to `0...1`;
  - keep placement near existing split visual config.
- `ghostboard/macos/Sources/Ghostty/Ghostty.Config.swift`
  - add `focusedSplitBorderColor: Color?`;
  - add `unfocusedSplitBorderColor: Color?`;
  - add `splitBorderWidth: Double`;
  - add `unfocusedSplitSaturation: Double`;
  - follow the existing `ghostty_config_get` bridge style used by
    `unfocusedSplitFill` and `splitDividerColor`.
- `ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView.swift`
  - compute `borderInset = isSplit ? ghostty.config.splitBorderWidth : 0`;
  - pass an inset size to `SurfaceRepresentable`;
  - frame and offset the representable by the inset size;
  - apply `.saturation(...)` to unfocused split panes using the
    last-focused-surface-aware focus predicate, not raw `surfaceFocus` alone;
  - inset the progress bar container by the same border width;
  - draw the split-pane border as a `Rectangle().strokeBorder(...)` overlay;
  - choose focused versus unfocused border color using the same focus predicate;
  - keep `.allowsHitTesting(false)` on the border.
- `scripts/ghostboard-geometry-matrix.sh`
  - add a dedicated border-enabled scenario, tentatively
    `split-right-border-config`;
  - write a temporary `GHOSTTY_CONFIG_PATH` that enables split borders and
    desaturation:
    - `focused-split-border-color = 7dcfff`;
    - `unfocused-split-border-color = 565f89`;
    - `split-border-width = 2`;
    - `unfocused-split-saturation = 0.5`;
  - write a second temporary config variant with out-of-range values
    `split-border-width = 99` and `unfocused-split-saturation = 99` to prove
    clamping through runtime trace evidence;
  - write disabled and missing-color config variants to prove
    `split-border-width = 0` disables borders and that width alone does not draw
    a focused or unfocused border when the relevant color is unset;
  - capture screenshots and logs for focused/unfocused borders, disabled
    behavior, missing-color behavior, content inset, mouse hit testing, and the
    browser-overlay split scenario with borders enabled.
- `issues/0823-ghostboard-split-pane-border-config/01-port-split-pane-border-config.md`
  - record design review, implementation notes, verification, completion review,
    result, and conclusion.
- `issues/0823-ghostboard-split-pane-border-config/README.md`
  - link Experiment 1 in the experiment index.

Reference files:

- `ghostboard-legacy/src/config/Config.zig`
- `ghostboard-legacy/macos/Sources/TermSurf/TermSurf.Config.swift`
- `ghostboard-legacy/macos/Sources/TermSurf/Surface View/SurfaceView.swift`
- `issues/0669-active-pane/README.md`
- `issues/0672-border-padding/README.md`

## Verification

Pass criteria:

- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0823-ghostboard-split-pane-border-config/README.md \
    issues/0823-ghostboard-split-pane-border-config/01-port-split-pane-border-config.md
  prettier --check --prose-wrap always --print-width 80 \
    issues/0823-ghostboard-split-pane-border-config/README.md \
    issues/0823-ghostboard-split-pane-border-config/01-port-split-pane-border-config.md
  ```

- Zig formatting and build pass:

  ```bash
  cd ghostboard
  zig fmt src/config/Config.zig
  zig fmt --check src/config/Config.zig
  zig build -Demit-macos-app=false
  ```

- Swift lint/build checks pass where available:

  ```bash
  cd ghostboard
  swiftlint lint --strict --fix \
    "macos/Sources/Ghostty/Ghostty.Config.swift" \
    "macos/Sources/Ghostty/Surface View/SurfaceView.swift"
  swiftlint lint --strict \
    "macos/Sources/Ghostty/Ghostty.Config.swift" \
    "macos/Sources/Ghostty/Surface View/SurfaceView.swift"
  macos/build.nu --scheme Ghostty --configuration Debug --action build
  ```

- The border-enabled matrix scenario is added and has valid shell syntax:

  ```bash
  bash -n scripts/ghostboard-geometry-matrix.sh
  ```

- Config parsing and Swift bridge evidence are concrete. The experiment must
  launch Ghostboard with a temporary config containing:

  ```text
  focused-split-border-color = 7dcfff
  unfocused-split-border-color = 565f89
  split-border-width = 2
  unfocused-split-saturation = 0.5
  ```

  and must save a log artifact proving all four values were accepted and bridged
  into the macOS UI. Acceptable evidence is a `TERMSURF_GEOMETRY_TRACE` log line
  emitted by `SurfaceView.swift`, for example:

  ```text
  TermSurf geometry layer=appkit event=split_border_config ... is_split=true border_width=2 focused_color_present=true unfocused_color_present=true saturation=0.5
  ```

- Clamp behavior is verified with a temporary config containing out-of-range
  values:

  ```text
  focused-split-border-color = 7dcfff
  unfocused-split-border-color = 565f89
  split-border-width = 99
  unfocused-split-saturation = 99
  ```

  The experiment must save a log artifact proving Swift sees `border_width=10`
  and `saturation=1`, or an equivalent Zig-side test/log that proves the same
  finalized config values.

- Disabled and missing-color behavior are verified with named artifacts:
  - `split-border-width = 0` with both colors set produces no border trace and
    no visible border screenshot;
  - `split-border-width = 2` with no focused color produces no focused border
    trace/screenshot;
  - `split-border-width = 2` with no unfocused color produces no unfocused
    border trace/screenshot.
- Runtime verification proves:
  - a single-pane window does not draw a split-pane border;
  - after creating a split, the focused pane uses `focused-split-border-color`;
  - the unfocused pane uses `unfocused-split-border-color`;
  - focus switching swaps the visible border colors;
  - the border does not intercept mouse input;
  - terminal content is inset and not covered by the border;
  - with `split-border-width = 0`, no border is drawn;
  - with a color missing, that focus state draws no border;
  - a TermSurf browser overlay in a split still fills and follows its viewport
    with borders enabled.
- The dedicated border-enabled browser-overlay scenario passes and names its
  artifacts:

  ```bash
  scripts/ghostboard-geometry-matrix.sh split-right-border-config
  ```

  Required evidence from that run:
  - app log path;
  - Roamium trace path;
  - screenshot of the single-pane no-border baseline;
  - screenshot of split panes with focused and unfocused borders;
  - screenshot or log proving content is inset and not covered;
  - hit-test log proving the border did not intercept mouse input;
  - AppKit/Zig/Roamium geometry logs proving the browser overlay still fills the
    pane viewport with borders enabled.

- Existing adjacent geometry coverage still passes without border config, to
  prove no default-behavior regression:

  ```bash
  scripts/ghostboard-geometry-matrix.sh split-right
  scripts/ghostboard-geometry-matrix.sh split-right-focus-switch
  ```

- `git diff --check` passes.
- The design review is recorded in this experiment file and the plan is
  committed before implementation begins.
- After implementation, verification, and result recording, the completion
  review is recorded in this experiment file and the result commit is made
  before designing or implementing any follow-up experiment.

Fail criteria:

- Current Ghostboard rejects any of the legacy config keys after the change.
- Borders draw in a single-pane window.
- Border width alone draws a border when the relevant color is unset.
- Borders intercept mouse input.
- Borders cover terminal text, progress bars, or browser overlays.
- Insetting the terminal surface causes TermSurf browser overlays to become
  misaligned or incorrectly sized.
- Focus changes do not update focused and unfocused border colors.
- The implementation requires changes outside current Ghostboard unless logs
  prove another component owns the failure.

## Design Review

Fresh-context adversarial design review initially returned **CHANGES REQUIRED**
with three required findings:

- the ordinary split geometry runs did not prove browser-overlay behavior with
  borders enabled;
- config parsing and clamp verification were underspecified;
- runtime evidence for content coverage and hit-test behavior was too vague.

The design was updated to require a dedicated `split-right-border-config`
scenario, concrete runtime trace evidence for bridged and clamped config values,
named disabled/missing-color artifacts, screenshots, hit-test logs, and
border-enabled AppKit/Zig/Roamium geometry evidence.

The same reviewer re-reviewed the fixes and returned **APPROVED** with no
remaining required findings. The reviewer also confirmed the optional formatter
finding was addressed by adding final non-mutating checks such as
`prettier --check`, `zig fmt --check`, strict Swift lint, and
`git diff --check`.
