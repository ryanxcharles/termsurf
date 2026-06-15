# Experiment 163: Custom Shader Output Runtime

## Description

`RUNTIME-008B2B2B2B` still groups several renderer-visible effects together: GUI
cursor pixels, custom shader output, broader GUI/pixel parity, and
screenshot-level padding pixel proof. Custom shader output is narrow enough to
prove without a full app screenshot walkthrough because Roastty's Metal
compositor tests already render into a readback-capable target.

Pinned Ghostty renders normal terminal content into an offscreen texture when
custom shaders are active, then runs post-process pipelines in order,
ping-ponging intermediate textures and writing the final pass to the
presentation target. Its Metal renderer uses linear clamp-to-edge sampling for
custom shader input and resizes intermediate textures with the frame target.

This experiment will split out that deterministic Metal custom-shader output
slice. It will not claim GUI cursor pixels, screenshot-level padding proof,
native window/surface screenshot equivalence, or broader renderer GUI parity.

## Changes

- `issues/0805-roastty-ghostty-parity/custom_shader_output_runtime_parity.py`
  - Add a static/runtime-source guard checking pinned Ghostty anchors:
    - `renderer/generic.zig` routes the normal frame into a custom shader target
      when `frame.custom_shader_state` exists;
    - `renderer/generic.zig` runs `post_pipelines` after the normal frame, syncs
      `custom_shader_uniforms`, samples the prior pass, and writes the final
      pass to `frame.screen.target`;
    - `renderer/metal/shaders.zig` initializes custom shader pipelines from
      post-process shader source;
    - `renderer/Metal.zig` declares Metal's custom shader target and Y-down
      coordinate convention;
    - `renderer/metal/api.zig` uses shader-read texture usage for custom shader
      textures.
  - Check Roastty markers:
    - `draw_frame_with_images_and_custom_shaders`;
    - `MetalCustomShaderInput`;
    - `ensure_custom_shader_state`;
    - `post_process_texture_options`;
    - `draw_custom_shader`;
    - `custom_shader_sampler_descriptor`;
    - `compositor_custom_shader_samples_offscreen_frame_into_final_target`;
    - `compositor_custom_shader_ping_pongs_multiple_passes`;
    - `compositor_custom_shader_resizes_intermediate_textures`;
    - `compositor_custom_shader_uses_shadertoy_sampler_options`;
    - `compositor_image_aware_frame_can_be_custom_shader_source`.
  - Check the regenerated runtime inventory and CFG-223 matrix wording.
- `roastty/src/renderer/metal/compositor.rs`
  - Add a focused non-skipping Metal availability assertion for this experiment,
    so the custom shader output proof fails on machines without a usable
    `MTLCreateSystemDefaultDevice()` instead of silently returning early.
  - Keep the existing broad Metal compositor tests' early-return behavior
    unchanged for general test-suite portability.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-008B2B2B2B` into:
    - an Oracle-complete row for deterministic Metal custom shader output
      readback;
    - a remaining renderer-visible GUI/pixel gap row for GUI cursor pixels,
      broader GUI/pixel parity, and screenshot-level padding pixel proof.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`; this split should keep the
    unresolved gap count at four while increasing the Oracle-complete and closed
    row counts by one.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Update the experiment index and Learnings if the experiment discovers
    reusable guidance.

## Verification

- Run the focused compositor custom shader tests:

  ```bash
  cargo test --manifest-path roastty/Cargo.toml compositor_custom_shader -- --test-threads=1
  ```

- Run the non-skipping Metal availability assertion for this experiment:

  ```bash
  cargo test --manifest-path roastty/Cargo.toml custom_shader_output_requires_metal_device -- --test-threads=1
  ```

- Run the adjacent image-aware custom shader test:

  ```bash
  cargo test --manifest-path roastty/Cargo.toml compositor_image_aware_frame_can_be_custom_shader_source -- --test-threads=1
  ```

- Run Rust formatting:

  ```bash
  cargo fmt --manifest-path roastty/Cargo.toml --check
  ```

- Run the new parity guard:

  ```bash
  PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/custom_shader_output_runtime_parity.py
  ```

- Regenerate and validate the runtime inventory:

  ```bash
  PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
  ```

- Run Markdown and diff hygiene:

  ```bash
  prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/163-custom-shader-output-runtime.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
  git diff --check
  ```

The experiment passes only if every listed command exits successfully, the
focused Metal compositor tests prove deterministic custom shader output pixels
on a non-skipped Metal run, and the inventory splits only that slice out of the
remaining renderer-visible gap. `CFG-223` must remain open.

## Design Review

**Reviewer:** Poincare the 2nd (`019ecaaa-c1ad-7a91-80ef-8cfa41f424f6`)

**Result:** Changes required

The first review found one required issue and one optional hardening point:

- The design relied on existing Metal compositor tests that return early when no
  Metal device exists. A zero-exit test run could therefore be vacuous while the
  experiment claims custom shader output pixels were proven.
- The pass criteria should explicitly require every listed command to pass, not
  only the compositor tests and inventory split.

The design has been updated to require a non-skipping Metal availability
assertion for this experiment and to make all listed verification commands part
of the pass criteria.

**Re-review result:** Approved

The reviewer confirmed both findings were resolved and reported no new required
findings.

## Result

**Result:** Pass

Split deterministic Metal custom shader output readback out of the remaining
renderer-visible gap. Roastty now has an explicit non-skipping Metal
availability test for this proof, so the focused custom shader output tests
cannot satisfy the experiment on a machine without
`MTLCreateSystemDefaultDevice()`.

The existing Metal compositor readback tests prove the scoped behavior:

- `compositor_custom_shader_samples_offscreen_frame_into_final_target` proves a
  single custom shader samples the offscreen terminal frame and changes the
  final target bytes.
- `compositor_custom_shader_ping_pongs_multiple_passes` proves ordered
  multi-pass output through ping-pong textures.
- `compositor_custom_shader_resizes_intermediate_textures` proves intermediate
  custom shader textures track target size.
- `compositor_custom_shader_uses_shadertoy_sampler_options` proves the linear
  clamp-to-edge sampler settings.
- `compositor_image_aware_frame_can_be_custom_shader_source` proves the
  image-aware frame path can feed the custom shader source texture.

Added `custom_shader_output_runtime_parity.py` to statically guard pinned
Ghostty's custom shader render path, Roastty's Metal implementation markers, the
readback tests, and the inventory split.

Verification run:

```bash
cargo test --manifest-path roastty/Cargo.toml custom_shader_output_requires_metal_device -- --test-threads=1
```

Result: 1 passed.

```bash
cargo test --manifest-path roastty/Cargo.toml compositor_custom_shader -- --test-threads=1
```

Result: 4 passed.

```bash
cargo test --manifest-path roastty/Cargo.toml compositor_image_aware_frame_can_be_custom_shader_source -- --test-threads=1
```

Result: 1 passed.

```bash
cargo fmt --manifest-path roastty/Cargo.toml
cargo fmt --manifest-path roastty/Cargo.toml --check
```

Result: both passed.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/custom_shader_output_runtime_parity.py
```

Result: `custom_shader_output_runtime_parity=pass`.

```bash
for guard in issues/0805-roastty-ghostty-parity/*_runtime_parity.py issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do
  [ -f "$guard" ] || continue
  PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1
done
```

Result: all listed guards passed.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
```

Result: `runtime_rows=70`, `oracle_complete=63`, `closed=66`, `incomplete=4`,
`gap=4`, `cfg223=Gap`.

## Conclusion

The custom shader output portion of the renderer-visible gap is now proven by a
non-vacuous Metal readback guard and durable static/source checks. CFG-223
remains open because the remaining renderer-visible row still needs proof for
GUI cursor pixels, broader GUI/pixel parity, and screenshot-level padding pixel
proof. The other three broad gaps also remain open: font renderer output, macOS
app workflow/UI effects, and notification/link/bell GUI effects.

## Completion Review

**Reviewer:** Dewey the 2nd (`019ecab4-d460-7322-ad2d-c5705563b798`)

**Result:** Approved

The reviewer found no required issues. They confirmed the experiment does not
overclaim full GUI/pixel parity, keeps CFG-223 open, leaves GUI cursor pixels,
broader GUI/pixel parity, and screenshot-level padding proof in the remaining
renderer gap, and proves the custom shader slice with a non-vacuous Metal device
test on this machine.

The reviewer independently ran:

- `cargo fmt --manifest-path roastty/Cargo.toml --check`
- `cargo test --manifest-path roastty/Cargo.toml custom_shader_output_requires_metal_device -- --test-threads=1`
- `cargo test --manifest-path roastty/Cargo.toml compositor_custom_shader -- --test-threads=1`
- `cargo test --manifest-path roastty/Cargo.toml compositor_image_aware_frame_can_be_custom_shader_source -- --test-threads=1`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/custom_shader_output_runtime_parity.py`
- `git diff --check`

All checks passed. The reviewer also verified the runtime inventory counts
read-only as `runtime_rows=70`, `oracle_complete=63`, `closed=66`,
`incomplete=4`, `gap=4`, and `cfg223=Gap`.
