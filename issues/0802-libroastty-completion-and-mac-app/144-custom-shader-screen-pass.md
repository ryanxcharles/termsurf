# Experiment 144: Phase H — custom-shader screen pass

## Description

Wire the live Metal renderer's custom-shader screen-pass foundation.

Earlier Issue 802 work already added the `custom-shader` config surface, the
Metal `CUSTOM_SHADER_TARGET`, and the upstream-compatible `CustomShaderUniforms`
value type plus per-frame update helpers. The live Metal compositor still always
renders the terminal frame directly into the final IOSurface target, so there is
nowhere for a Shadertoy-style post-process pass to sample from. Upstream Ghostty
solves this by allocating renderer-owned custom shader state: a front/back
texture pair, a sampler, and a uniform buffer. When custom shaders are active,
the normal terminal frame is drawn into the back texture, then each post-process
pipeline samples the current back texture and renders either into the front
texture or, for the last pass, into the final target, swapping front/back after
each pass.

This experiment ports that screen-pass architecture to Roastty's live Metal
path. It deliberately does not complete the full GLSL file loader or
cross-compiler. Instead, it adds the runtime shape that loader output will plug
into, and proves it with a small test-only/custom MSL pipeline sequence that
exercises the same compositor path a loaded `custom-shader` pipeline will use.

Out of scope:

- parsing or loading user `custom-shader` files beyond the existing config path
  support;
- GLSL-to-MSL translation, shader include handling, or diagnostics;
- `custom-shader-animation` frame scheduling;
- link highlighting and debug overlay work.

## Changes

- `roastty/src/renderer/metal/texture.rs`
  - Add or adjust render-target texture options so custom-shader intermediate
    textures can be both render targets and shader-readable sources. Upstream
    `CustomShaderState` uses its front/back textures for exactly both roles.
  - Keep existing plain render-target test helpers working, or split the options
    into final-target and post-process-target helpers if that is clearer.
- `roastty/src/renderer/metal/shaders.rs`
  - Expose a small helper for compiling Metal source strings into a
    `MetalShaderLibrary` so tests and the later custom-shader loader can build
    fragment libraries without duplicating Objective-C/Metal compile code.
  - Keep standard shader compilation behavior unchanged.
- `roastty/src/renderer/metal/pipeline.rs`
  - Add a post-process pipeline build path that uses the standard
    `full_screen_vertex` function from the standard library, a fragment function
    from a custom shader library, no vertex descriptor, the final target pixel
    format, and blending disabled.
  - Preserve the existing standard pipeline descriptors and vertex input
    behavior.
- `roastty/src/renderer/metal/render_pass.rs`
  - Add a `draw_custom_shader` / equivalent helper that binds a post-process
    pipeline, the custom-shader uniform buffer, the source texture at texture
    slot 0, the sampler at sampler slot 0, and draws a full-screen triangle.
  - Ensure the helper can target either an intermediate `MetalTexture` or the
    final IOSurface target through the existing render-pass attachment API.
- `roastty/src/renderer/metal/compositor.rs`
  - Add a compositor-owned custom shader state mirroring upstream's
    `CustomShaderState`: `front_texture`, `back_texture`, `sampler`, and a
    `MetalBuffer<CustomShaderUniforms>`.
  - Create the custom-shader sampler with upstream Shadertoy behavior:
    `min_filter = Linear`, `mag_filter = Linear`,
    `s_address_mode = ClampToEdge`, and `t_address_mode = ClampToEdge`. Do not
    reuse `MetalSamplerDescriptorOptions::default()`, because that defaults to
    nearest filtering for ordinary image paths.
  - Resize the front/back textures whenever the live target size changes.
  - Add a custom-shader draw entry point used by tests and ready for the loader:
    when the post-process pipeline list is empty, preserve the current direct
    draw-to-final-target behavior; when non-empty, draw the normal terminal
    frame into `back_texture`, sync custom uniforms, run each post-process pass
    with upstream's target selection and swap order, and present the final
    target.
  - Keep the image-aware path ordering from Experiments 141 and 143 intact when
    the normal frame is drawn offscreen.
  - Add readback tests proving:
    - with no post-process pipelines, rendering still goes directly to the final
      target and existing pixels are unchanged;
    - one post-process pass samples the offscreen terminal frame and writes the
      final IOSurface target;
    - two post-process passes ping-pong through front/back and the last pass
      writes the final target;
    - resizing reallocates both intermediate textures to the screen size;
    - the custom-shader sampler descriptor uses linear min/mag filtering and
      clamp-to-edge address modes, matching upstream `Metal.samplerOptions`.
- `roastty/src/renderer/frame_rebuild.rs`,
  `roastty/src/renderer/frame_renderer.rs`, and `roastty/src/lib.rs`
  - Thread custom-shader uniforms through the live-present path only if the
    compositor API needs a production-facing input shape in this slice. Prefer
    avoiding app-level churn until the loader slice can make `custom-shader`
    config actually active.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After the result, update the Phase H checklist note to distinguish the
    completed screen-pass foundation from the still-open loader/cross-compiler
    work, if that work remains.

## Verification

- Format markdown:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/144-custom-shader-screen-pass.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Format Rust:
  - `cargo fmt`
- Run focused custom-shader and Metal renderer tests:
  - `cargo test -p roastty custom_shader -- --test-threads=1`
  - `cargo test -p roastty metal::compositor -- --test-threads=1`
  - `cargo test -p roastty metal::render_pass -- --test-threads=1`
  - `cargo test -p roastty metal::pipeline -- --test-threads=1`
- Run ABI harness:
  - `cargo test -p roastty --test abi_harness`
- Run full Roastty Rust coverage:
  - `cargo test -p roastty -- --test-threads=1`
- Run hosted app coverage:
  - `cd roastty && macos/build.nu --action test`
- Run hygiene checks:
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/144-custom-shader-screen-pass.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = the live Metal compositor has an upstream-shaped custom shader state,
can render the normal terminal frame into an offscreen back texture, can apply
one or more post-process pipelines through front/back ping-pong, writes the last
pass into the final IOSurface target, preserves direct rendering when no
post-process pipelines are active, resizes intermediate textures with the
screen, keeps Kitty/background-image ordering intact, and all
focused/full/hosted checks pass.

**Partial** = the runtime state and one-pass post-process path work, but
multi-pass ping-pong, resize behavior, or image-aware offscreen ordering needs a
follow-up.

**Fail** = Roastty's current Metal abstractions cannot support offscreen
screen-pass composition without a broader compositor redesign.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Epicurus`, fresh
context, with focused re-review by `Carson`.

**Verdict:** Approved after fixes.

**Findings and fixes:**

- **Required:** Epicurus found that the initial design only required a
  custom-shader sampler, but did not specify upstream's Shadertoy sampler
  behavior. Upstream Metal uses linear min/mag filtering with clamp-to-edge
  addressing, while Roastty's current `MetalSamplerDescriptorOptions::default()`
  uses nearest filtering. Fixed by requiring custom shader state to create a
  linear/linear clamp-to-edge sampler and by adding descriptor verification.

Carson re-reviewed the fix and approved the design with no remaining findings.
