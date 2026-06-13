# Experiment 179: Phase C — renderer options propagation

## Description

Bring Roastty's live render surface closer to Ghostty's renderer-thread mailbox
semantics for the remaining Phase C `Options` item: focus, visibility/occlusion,
and config-change propagation.

Upstream sends renderer-thread messages for `.focus`, `.visible`, and
`.change_config`. The renderer thread uses them to update QoS/display-link
state, restart or stop focus-sensitive timers, redraw immediately when a surface
becomes visible again, mark custom shader focus changes, and apply derived
renderer configuration before future frames. Roastty does not have a separate
renderer thread yet, but it now has a continuous present driver and an
in-process live renderer. The equivalent state changes therefore need to land on
`Surface` and `SurfaceLiveRenderer` directly.

There is also an ABI semantic mismatch to fix before richer option propagation
is trustworthy: Ghostty's `ghostty_surface_set_occlusion(surface, visible)`
takes a `visible` boolean, and the copied Swift caller passes
`window.occlusionState.contains(.visible)`. Roastty's Rust implementation names
that parameter `occluded` and stores it directly. That means the live surface
can record a visible window as occluded and an invisible window as visible. This
experiment should make the Rust side match the upstream/caller contract without
changing the public symbol.

This is a scoped Phase C slice. It should not retire the interim
`render_state_update` pull path, redesign presentation around a real Rust
renderer thread, or check the final ASCII-terminal milestone.

## Changes

- `roastty/src/lib.rs`
  - Rename the `Surface` visibility field and helper logic so the stored state
    is `visible` rather than `occluded`, matching upstream's ABI semantics and
    the copied Swift caller.
  - Make `roastty_surface_set_occlusion(surface, visible)` treat its bool as
    visibility: unchanged values are no-ops; becoming visible marks live
    `NSView` surfaces dirty, wakes the app, and allows the next present-driver
    tick to rebuild/present immediately; becoming invisible suppresses live
    presentation work.
  - Gate `present_live` and present-driver frame submission on visibility for
    live `NSView` surfaces. Timers may keep firing, matching upstream's
    low-cost-timer choice, but invisible surfaces should not rebuild cells or
    submit Metal frames until visible again.
  - Keep focus behavior from Experiment 178, but route it through a small
    renderer-options helper so focus changes update cursor blink state,
    custom-shader focus-change state, dirty state, and app wakeup in one place.
  - Extend config-change propagation so `roastty_surface_update_config` not only
    updates terminal/config fields but also marks the live renderer's
    renderer-derived state dirty. At minimum, custom shader config must resync
    on the next live frame, the live frame must be requested, and any renderer
    state that cannot be safely updated in place must be rebuilt explicitly.
  - Add focused unit tests for:
    - the occlusion ABI bool being interpreted as `visible`, not `occluded`;
    - invisible live surfaces not submitting/rebuilding live frames;
    - becoming visible marking a live surface dirty and preserving ABI-only
      no-op behavior for surfaces without an `NSView`;
    - config updates requesting a live frame or renderer rebuild when live
      renderer-derived state is affected;
    - focus changes still mark custom shader focus changes and cursor blink
      state without regressing the ABI-only focus behavior fixed in
      Experiment 178.

- `roastty/macos/Sources/Features/Terminal/BaseTerminalController.swift`
  - No behavior change should be needed if the Rust ABI is corrected. Only edit
    comments or wrapper naming if the implementation reveals misleading local
    wording that would make the visible/occluded polarity unclear.

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After the run, mark it `Pass`, `Partial`, or `Fail`.
  - Check the Phase C renderer mailbox / `Options` item only if focus,
    visibility/occlusion, and config-change propagation are all implemented and
    verified by deterministic tests plus live smoke.

- `issues/0802-libroastty-completion-and-mac-app/179-renderer-options-propagation.md`
  - Record implementation details, verification output, live artifact paths,
    result, conclusion, and AI completion review.

## Verification

Before implementation:

- Codex-native adversarial design review approves this experiment.
- Commit the reviewed plan separately from the result.

Focused tests:

- `cargo test -p roastty live_renderer_options -- --test-threads=1`
- `cargo test -p roastty live_cursor_blink -- --test-threads=1`

Regression checks:

- `cargo test -p roastty --test abi_harness`
- `cargo test -p roastty -- --test-threads=1`
- `cargo fmt --check -p roastty`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/179-renderer-options-propagation.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `git diff --check`

Live sanity:

- Rebuild the copied app:

  ```bash
  cd roastty && macos/build.nu --action build
  ```

- Re-run the known-good smoke proof so option propagation does not regress the
  copied app startup/render path:

  ```bash
  scripts/roastty-app/stop-app.sh
  TERMSURF_AB_HOLD_SECONDS=10 \
  ROASTTY_PRESENT_DRIVER_LOG=1 \
    scripts/roastty-app/live-ab-smoke.sh \
      --recipe smoke \
      --comparison-region content \
      --max-mismatch-ratio 1 \
      --max-mean-channel-delta 255
  ```

**Pass** = the occlusion ABI polarity matches upstream/caller semantics, live
visibility suppresses presentation while invisible and requests an immediate
frame when visible again, focus/config propagation are covered by deterministic
tests, full regression checks pass, the copied app rebuilds, live smoke still
renders with `present-driver=display-link reason=core-video`, and the Phase C
renderer mailbox / `Options` checklist item can be checked.

**Partial** = a subset is implemented and verified, but one of focus,
visibility/occlusion, or config-change propagation remains unproven or too
weakly wired to check the roadmap item. Record the exact missing piece.

**Fail** = the change breaks rendering, app startup, display-link presentation,
cursor/focus semantics, config reload behavior, or the Rust/ABI test gates.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Maxwell`, fresh context.

**Verdict:** Approved.

Findings: None. The reviewer confirmed the design links Experiment 179 as
`Designed`, has the required sections, stays scoped to the Phase C renderer
mailbox / `Options` item, matches upstream focus / visible / `change_config`
semantics closely enough for this slice, and includes concrete verification and
hygiene gates.
