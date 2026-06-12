# Experiment 145: Phase H — custom-shader loader hookup

## Description

Finish the user-facing custom-shader path for the live Metal renderer.

Experiment 144 added the runtime screen-pass foundation: custom-shader state,
front/back textures, Shadertoy sampler options, a custom uniform buffer,
post-process pipeline construction, and compositor ping-pong. That path is still
test-only because no code reads `custom-shader` config files, compiles them, or
feeds the resulting post-process pipelines into live presentation.

Upstream Ghostty's custom-shader loader does four things:

1. read each configured `custom-shader` file, preserving order and skipping only
   missing optional paths;
2. wrap Shadertoy-style source with `shadertoy_prefix.glsl`;
3. compile the full GLSL fragment shader to SPIR-V;
4. cross-compile SPIR-V to the renderer target (`msl` for Metal), then build
   post-process pipelines with standard `full_screen_vertex` and custom fragment
   `main0`.

This experiment ports that path to Roastty's Rust/Metal renderer. Because no
local `glslangValidator` or `spirv-cross` executable is available, the
implementation should use Rust-side compiler bindings rather than shelling out.
The intended dependency shape is:

- `shaderc` for GLSL/Shadertoy-prefix source to SPIR-V;
- `spirv-cross2` (or a similarly maintained SPIRV-Cross Rust binding) for SPIR-V
  to MSL with MSL decoration binding enabled, matching upstream's
  `SPVC_COMPILER_OPTION_MSL_ENABLE_DECORATION_BINDING`.

If implementation finds that one of these crates cannot build reliably in this
repo, the experiment may stop at a documented `Partial`, but it should not
substitute a non-cross-compiled `.metal`-only path as a pass.

Out of scope:

- shader animation scheduling policy beyond drawing when frames are already
  presented;
- OpenGL/GLSL output support;
- user-facing shader diagnostics UI beyond log/error propagation;
- link highlighting and debug overlay work.

## Changes

- `roastty/Cargo.toml` and `Cargo.lock`
  - Add focused shader compiler dependencies for runtime custom-shader loading.
    Prefer `shaderc` for GLSL-to-SPIR-V and `spirv-cross2` for SPIR-V-to-MSL,
    unless implementation discovers a narrower or more reliable maintained Rust
    binding.
  - Avoid depending on external command-line tools being present.
- `roastty/src/renderer/shadertoy.rs`
  - Port upstream `loadFromFiles`, `loadFromFile`, `glslFromShader`,
    `spirvFromGlsl`, and `mslFromSpv` behavior into Rust:
    - preserve shader order;
    - skip missing optional config paths but return an error for missing
      required paths;
    - reject files larger than upstream's 4 MiB read limit;
    - prepend `shadertoy_prefix.glsl` and require `mainImage`;
    - compile as a fragment shader entry point named `main`;
    - convert to MSL for `Target::Msl`;
    - enable MSL decoration binding so uniforms and texture bindings match the
      already-implemented Metal render-pass slots;
    - return owned MSL source strings ready for Metal library compilation.
  - Include the upstream `shadertoy_prefix.glsl` content with `include_str!`, or
    add an equivalent Rust string constant that is byte-for-byte faithful where
    practical.
  - Add unit tests covering prefix generation, valid sample shader conversion,
    invalid shader diagnostics, required missing path errors, optional missing
    path skip, order preservation, and the 4 MiB limit.
- `roastty/src/renderer/metal/shaders.rs`
  - Add a production post-process pipeline builder that accepts the generated
    MSL source strings, compiles each into a Metal library, and builds `main0`
    pipelines using the existing Experiment 144 post-process pipeline values.
  - Match upstream behavior on shader compile failure: log or surface the error
    and fall back to an empty post-process list so Roastty still renders.
- `roastty/src/renderer/metal/compositor.rs`
  - Add a production-facing draw path that takes the compositor-owned
    post-process pipeline list produced from config and feeds it into the
    Experiment 144 custom shader screen-pass path.
  - Preserve direct rendering when the loader yields no pipelines.
- `roastty/src/lib.rs`, `roastty/src/renderer/frame_renderer.rs`, and/or
  adjacent live renderer state
  - Add renderer-owned custom shader pipeline state tied to current `Config`.
  - Detect `custom_shader` path-list changes and rebuild pipelines, matching
    upstream's config-change-triggered shader reload.
  - Feed current `CustomShaderUniforms` into live presentation when pipelines
    are active, using the existing `FrameRebuildPlan::apply_custom_shader_frame`
    update path so time, resolution, cursor, focus, palette, and color uniforms
    are not stale.
  - Keep failures non-fatal: failed shader load/compile should disable custom
    shaders for that frame instead of preventing terminal rendering.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After the result, update Phase H to mark custom-shader loader/cross-compiler
    hookup complete if config paths demonstrably produce live post-process
    pipelines.

## Verification

- Format markdown:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/145-custom-shader-loader-hookup.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Format Rust:
  - `cargo fmt`
- Run focused shader loader and renderer tests:
  - `cargo test -p roastty shadertoy -- --test-threads=1`
  - `cargo test -p roastty custom_shader -- --test-threads=1`
  - `cargo test -p roastty metal::compositor -- --test-threads=1`
  - `cargo test -p roastty metal::shaders -- --test-threads=1`
- Run ABI harness:
  - `cargo test -p roastty --test abi_harness`
- Run full Roastty Rust coverage:
  - `cargo test -p roastty -- --test-threads=1`
- Run hosted app coverage:
  - `cd roastty && macos/build.nu --action test`
- Run hygiene checks:
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/145-custom-shader-loader-hookup.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = a configured Shadertoy-style `custom-shader` GLSL file is read from
config, prefixed, compiled to SPIR-V, cross-compiled to MSL, built into a Metal
`main0` post-process pipeline, and applied by the live compositor's custom
shader screen-pass path; multiple configured shaders preserve order and
ping-pong through the existing pass sequence; optional missing files are
skipped; required missing/invalid files disable custom shaders without breaking
terminal rendering; uniform updates feed the custom shader path;
focused/full/hosted checks pass.

**Partial** = shader file loading and conversion work, but live config-change
rebuild or uniform-fed presentation needs a follow-up.

**Fail** = runtime GLSL-to-MSL compilation cannot be made reliable in this repo
without a broader build-system or dependency strategy.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Kepler`, fresh context.

**Verdict:** Approved.

**Findings and fixes:**

- No findings.
