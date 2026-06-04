+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 435: the custom-shader target enum (Target) and the Metal target constant

## Description

Experiments 428–434 ported the custom-shader _uniforms_. This experiment ports
the custom-shader **`Target`** enum — the output language the shader loader
cross-compiles to (`glsl` for OpenGL, `msl` for Metal) — and the **Metal
renderer constant** that selects it (`custom_shader_target = .msl`). It is a
small, self-contained slice that establishes the target type the deferred shader
loader (`loadFromFiles`) will switch on; the loader itself stays deferred.

## Upstream behavior

In `renderer/shadertoy.zig`:

```zig
/// The target to load shaders for.
pub const Target = enum { glsl, msl };
```

`Target` is the output language for the shader cross-compile pipeline (GLSL →
SPIR-V → target). `loadFromFile` switches on it at the end:

```zig
return switch (target) {
    .glsl => try glslFromSpv(alloc_gpa, spirv),
    .msl => try mslFromSpv(alloc_gpa, spirv),
};
```

Each renderer picks its target as a compile-time constant
(`renderer/Metal.zig:32`, `renderer/OpenGL.zig:26`):

```zig
// Metal.zig
pub const custom_shader_target: shadertoy.Target = .msl;
// OpenGL.zig
pub const custom_shader_target: shadertoy.Target = .glsl;
```

Metal cross-compiles custom shaders to MSL; OpenGL keeps them as GLSL. roastty
is Metal-only, so its renderer target is `Target::Msl`.

## Rust mapping

The enum lives in the shadertoy module (upstream `shadertoy.zig`); the renderer
constant lives in the Metal module (upstream `Metal.zig`).

`roastty/src/renderer/shadertoy.rs`:

```rust
/// The output language the custom-shader loader cross-compiles to (upstream
/// `shadertoy.Target`): `Glsl` for OpenGL, `Msl` for Metal. The shader loader
/// (deferred) switches on it (GLSL → SPIR-V → target).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Glsl,
    Msl,
}
```

`roastty/src/renderer/metal/mod.rs`:

```rust
use crate::renderer::shadertoy::Target;

/// The custom-shader target for the Metal renderer (upstream
/// `Metal.zig`'s `custom_shader_target`): Metal cross-compiles custom shaders to
/// MSL.
pub(crate) const CUSTOM_SHADER_TARGET: Target = Target::Msl;
```

The `Glsl` variant is preserved (it is part of the upstream enum) even though
roastty, being Metal-only, only uses `Msl` — keeping the enum faithful so the
deferred loader's `match target { Glsl => …, Msl => … }` ports cleanly.

## Scope / faithfulness notes

- **Ported (bridged)**: the `Target` enum (`shadertoy.zig:43`) and the Metal
  renderer's `CUSTOM_SHADER_TARGET` constant (`Metal.zig:32`).
- **Faithful**: `Target` has the two upstream variants `Glsl`/`Msl`; the Metal
  constant is `Msl` (upstream `Metal.zig`'s `.msl`).
- **Faithful adaptation**: the constant lives in the Metal module (upstream's
  `Metal.zig`); roastty is Metal-only, so the OpenGL constant (`OpenGL.zig`'s
  `.glsl`) is not ported, but the `Glsl` variant is kept so the enum and the
  deferred loader's switch stay faithful.
- **Deferred**: the shader loader (`loadFromFiles` / `loadFromFile` and the GLSL
  → SPIR-V → target conversion that switches on `Target`), and the
  `custom_shader_y_is_down` Metal constant (already applied as a literal in
  Experiment 430's cursor math). (Consumed by a later slice; this experiment
  lands the target type and the Metal selection.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/shadertoy.rs`:
   - add `pub(crate) enum Target { Glsl, Msl }` (derive
     `Debug, Clone, Copy, PartialEq, Eq`).
   - update the stale module doc comment (it currently says the `Target` enum
     and the update methods are "ported in later slices" — the update methods
     landed in Experiments 429–434 and `Target` lands here; only the shader
     loading is deferred).
2. `roastty/src/renderer/metal/mod.rs`:
   - add `pub(crate) const CUSTOM_SHADER_TARGET: Target = Target::Msl;` and
     `use crate::renderer::shadertoy::Target;`.
3. Tests:
   - `Target` (in `shadertoy.rs`): the two variants are distinct
     (`Target::Glsl != Target::Msl`) and `Copy`/`Eq` (a trivial round-trip).
   - `CUSTOM_SHADER_TARGET` (in `metal/mod.rs`):
     `CUSTOM_SHADER_TARGET == Target::Msl` (Metal's target is MSL).
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty target
cargo test -p roastty custom_shader_target
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Target` has exactly the two upstream variants `Glsl`/`Msl`, and
  `CUSTOM_SHADER_TARGET == Target::Msl` (the Metal renderer's target) — faithful
  to upstream's `shadertoy.Target` and `Metal.zig`'s `custom_shader_target`;
- the tests pass (the distinct variants; the Metal constant value), and the
  existing tests still pass;
- the shader loader stays deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if `Target` is missing a variant or has an extra one,
the Metal constant is not `Msl`, the constant is placed against upstream's
structure, an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **one
Low finding** (now folded into the Changes), no Required or Recommended
findings. It verified the design directly against the vendored upstream:
`Target = enum { glsl, msl }` (`shadertoy.zig:43`) makes `Target::{Glsl, Msl}`
in `shadertoy.rs` the right home (keeping both variants for the deferred
loader's switch); the Metal constant `custom_shader_target = .msl`
(`Metal.zig:32`) makes `CUSTOM_SHADER_TARGET: Target = Target::Msl` in
`renderer/metal/mod.rs` the right Metal-only constant; keeping the `Glsl`
variant is correct (part of the upstream enum and the future loader's switch);
and the tests (variant distinctness + `CUSTOM_SHADER_TARGET == Target::Msl`) are
adequate for the slice.

- **Low (fixed)**: the `shadertoy.rs` module doc says the `Target` enum is
  "ported in later slices" — stale now that this experiment ports it (and the
  update methods, which it also names, landed in 429–434). Folded into the
  Changes: the module doc is updated so only the shader loading is described as
  deferred.

Review artifacts:

- Prompt: `logs/codex-review/20260604-100339-d435-prompt.md` (design)
- Result: `logs/codex-review/20260604-100339-d435-last-message.md` (design)

## Result

**Result:** Pass

The custom-shader `Target` enum and the Metal target constant are now live.

- `roastty/src/renderer/shadertoy.rs`: `pub(crate) enum Target { Glsl, Msl }`
  (derive `Debug, Clone, Copy, PartialEq, Eq`) — upstream `shadertoy.Target`.
  The module doc was updated (the update methods landed in 429–434, `Target`
  lands here; only the shader loading is deferred).
- `roastty/src/renderer/metal/mod.rs`:
  `pub(crate) const CUSTOM_SHADER_TARGET: Target = Target::Msl;` (upstream
  `Metal.zig`'s `custom_shader_target = .msl`), with
  `use crate::renderer::shadertoy::Target;` and a new `#[cfg(test)] mod tests`.

Tests:

- `target_variants_are_distinct` (in `shadertoy.rs`) —
  `Target::Glsl != Target::Msl`, plus a `Copy`/`Eq` round-trip.
- `custom_shader_target_is_msl` (in `metal/mod.rs`) —
  `CUSTOM_SHADER_TARGET == Target::Msl`.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2920 passed, 0 failed (+2, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

The custom-shader target type is in place: `Target { Glsl, Msl }` and the
Metal-only `CUSTOM_SHADER_TARGET = Msl`. The remaining custom-shader work is the
shader loader (`loadFromFiles` / `loadFromFile` and the GLSL → SPIR-V → target
conversion that switches on `Target`, via glslang/spirv-cross) — a larger slice
that pulls in external shader-compilation dependencies, so it stays deferred.
With the uniforms (428–434) and the target (435) ported, the remaining renderer
work is the `dirty` gate, the live per-frame call sites (tying `FrameState`
sync/draw to live state and these uniform updaters), and the `neverExtendBg`
terminal-core row/cell access; beyond the renderer, the other subsystems.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the module-doc Low is resolved (`Target` is now
described as present, only shader loading deferred), and verified faithfulness
against the vendored upstream: `Target::{Glsl, Msl}` matches `shadertoy.zig:43`,
`CUSTOM_SHADER_TARGET = Target::Msl` matches `Metal.zig:32`, and keeping `Glsl`
is correct (part of the upstream enum and the deferred loader's switch). It
judged the tests adequate for the slice (variant distinctness / `Copy`/`Eq` and
the Metal constant mapping). No public C ABI/header impact; nothing needed to
change before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-100614-r435-prompt.md` (result)
- Result: `logs/codex-review/20260604-100614-r435-last-message.md` (result)
