# Experiment 181: Background image renderer runtime

## Description

`RUNTIME-008B2B2B2B2B` still groups several renderer-visible effects: background
image rendering/options, `window-colorspace`, `alpha-blending`, and
`scroll-to-bottom.output`. The background image slice is concrete and already
has identifiable upstream anchors in pinned Ghostty's renderer:

- `DerivedConfig` copies `background-image`;
- the derived renderer config stores `background-image-opacity`,
  `background-image-position`, `background-image-fit`, and
  `background-image-repeat`;
- the renderer prepares a background image buffer from those options;
- the Metal draw path renders the background image pass before cells;
- config changes reload or repack the image state.

This experiment will split out only the deterministic background image renderer
runtime slice. It will not claim `window-colorspace`, `alpha-blending`, or
`scroll-to-bottom.output`.

## Changes

- Add `issues/0805-roastty-ghostty-parity/background_image_runtime_parity.py`.
  The guard will statically compare pinned Ghostty background-image anchors with
  Roastty's config-to-renderer image path:
  - Ghostty derived config fields and option copies in
    `vendor/ghostty/src/renderer/generic.zig`;
  - Ghostty image preparation, config-change reload/repack, and draw-pass
    markers in `generic.zig`;
  - Roastty `BackgroundImageConfig::from_config`,
    `BackgroundImageState::update_from_config`, `BackgroundImageConfig::vertex`,
    Metal background-image render-pass draw, live frame renderer wiring, and
    existing focused tests.
- Update `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py` to
  split a new Oracle-complete row for background image renderer runtime:
  `RUNTIME-008B2B2B2B2B2`.
- Narrow the existing `RUNTIME-008B2B2B2B2B` residual row so it continues to
  track only `window-colorspace`, `alpha-blending`, and
  `scroll-to-bottom.output`.
- Regenerate `config-runtime-inventory.md` and the CFG-223 line in
  `config-matrix.md`.
- Update the Issue 805 README learnings and experiment index with the result.

If inspection shows a real implementation gap in the background image path, fix
that gap inside this same narrow slice before promoting the row. Do not promote
background image behavior on static evidence alone if the renderer/runtime tests
do not prove path loading, option packing, draw pass output, and reset/unload
behavior.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml background_image -- --test-threads=1`
  passes and covers parser/formatter helpers, image load/upload/replace/reset,
  live frame rendering/unload, and compositor background-image behavior.
- `cargo test --manifest-path roastty/Cargo.toml bg_image -- --test-threads=1`
  passes and covers shader layout/raw values plus render-pass output.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/background_image_runtime_parity.py`
  passes and fails if the upstream or Roastty anchors for this slice disappear.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/renderer_visual_residual_audit.py`
  passes with background-image removed from the residual and colorspace,
  alpha-blending, and scroll-to-bottom still present.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  regenerates the inventory/matrix without drift.
- `python3 -m py_compile issues/0805-roastty-ghostty-parity/background_image_runtime_parity.py issues/0805-roastty-ghostty-parity/config_runtime_inventory.py issues/0805-roastty-ghostty-parity/renderer_visual_residual_audit.py`
  passes.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes if any Rust
  files are edited.
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/181-background-image-renderer-runtime.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  passes after formatting.
- `git diff --check` passes.

Failure criteria:

- Any guard can pass while Roastty no longer sources the background image path
  or image options from config, no longer draws the Metal background-image pass,
  or no longer resets/unloads the background image when config removes it.
- The experiment promotes `window-colorspace`, `alpha-blending`, or
  `scroll-to-bottom.output`.

## Design Review

Fresh-context Codex adversarial review:

- Initial verdict: **Changes required**.
- Required finding: the verification allowed Rust implementation fixes but did
  not require a Rust formatting check.
- Fix: added `cargo fmt --manifest-path roastty/Cargo.toml --check` as a pass
  criterion when Rust files are edited.
- Re-review verdict: **Approved**. The reviewer confirmed the formatting check
  and review record resolved the finding with no new Required findings.

## Result

**Result:** Pass

Experiment 181 split background image renderer runtime behavior out of
`RUNTIME-008B2B2B2B2B` and into the new Oracle-complete `RUNTIME-008B2B2B2B2B2`
row.

Implementation notes:

- Added `background_image_runtime_parity.py`, which checks pinned Ghostty's
  background-image config, derived renderer config, image preparation,
  config-change, vertex buffer, upload, and draw-pass anchors against Roastty's
  image state, shader layout, Metal render pass, compositor routing, frame
  renderer, live renderer wiring, focused tests, generated inventory row, and
  CFG-223 counts.
- Updated `config_runtime_inventory.py` so the residual renderer row now keeps
  only `window-colorspace`, `alpha-blending`, and `scroll-to-bottom.output`.
- Regenerated `config-runtime-inventory.md` and `config-matrix.md`; CFG-223 now
  reports 86 runtime rows, 79 Oracle-complete rows, 82 closed rows, 4 incomplete
  rows, and 4 runtime gaps.
- Updated adjacent renderer parity guards to expect the new CFG-223 counts and
  to use `window-colorspace` as the remaining concrete renderer-gap sentinel.
- Added a README learning recording that background image parity is now a
  guarded renderer/runtime slice.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml background_image -- --test-threads=1`
  — 15 passed.
- `cargo test --manifest-path roastty/Cargo.toml bg_image -- --test-threads=1` —
  8 passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/background_image_runtime_parity.py`
  — passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/renderer_visual_residual_audit.py`
  — passed.
- Additional affected renderer guards passed:
  `custom_shader_animation_runtime_parity.py`,
  `metal_cursor_pixel_runtime_parity.py`, and
  `custom_shader_output_runtime_parity.py`.
- `python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py` — passed.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` — passed.
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/181-background-image-renderer-runtime.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  — passed.
- `git diff --check` — passed.

## Conclusion

Background image renderer parity is no longer part of the renderer residual. The
remaining `RUNTIME-008B2B2B2B2B` work is now explicitly limited to
`window-colorspace`, `alpha-blending`, and `scroll-to-bottom.output`.

## Completion Review

Fresh-context Codex adversarial result review:

- Initial verdict: **Changes required**.
- Required finding: `macos_window_padding_pixel_runtime.py` still checked the
  whole inventory for `background-image-opacity`, which would pass for the wrong
  reason after background-image moved to its own Oracle-complete row.
- Fix: updated that guard to extract the `RUNTIME-008B2B2B2B2B` residual row,
  require `window-colorspace`, and reject `background-image-opacity` in that
  residual row.
- Re-review verdict: **Approved**. The reviewer confirmed the stale residual
  assertion was fixed and found no new Required issues.
