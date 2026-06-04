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

# Experiment 420: the minimum-contrast uniform update (update_min_contrast)

## Description

Experiments 415–419 ported the geometry, cursor, and background-color uniform
groups. This experiment ports the **`min_contrast`** uniform — the
minimum-contrast ratio the shader uses to keep text legible against its
background — which upstream sets from config in `changeConfig`. It is a small,
self-contained scalar port (one `f32` uniform field from config); the
color-space and blending bools that `changeConfig` also sets need the config
`colorspace`/`blending` enums (and a config-module home), so they stay deferred
to a later, deliberate slice.

## Upstream behavior

In `changeConfig` (`renderer/generic.zig`), the minimum contrast is set from
config:

```zig
// Set our new minimum contrast
self.uniforms.min_contrast = config.min_contrast;
```

(`config.min_contrast` is the `minimum-contrast` config value — a contrast ratio
floored at `1`. The same function then sets the color-space and blending bools,
which are separate and deferred.)

## Rust mapping (`roastty/src/renderer/metal/shaders.rs`)

`MetalUniforms::min_contrast` is `f32`. `update_min_contrast` sets it directly:

```rust
impl MetalUniforms {
    /// Update the minimum-contrast uniform (upstream `changeConfig`): the
    /// `min_contrast` ratio the shader uses to keep text legible against its
    /// background.
    pub(crate) fn update_min_contrast(&mut self, min_contrast: f32) {
        self.min_contrast = min_contrast;
    }
}
```

A direct assignment — `min_contrast` is already an `f32`, matching upstream's
`@floatCast`-derived config value. Only `min_contrast` is touched.

## Scope / faithfulness notes

- **Ported (bridged)**: `MetalUniforms::update_min_contrast` — the
  `min_contrast` uniform from the config minimum-contrast value, upstream's
  `changeConfig` `min_contrast` assignment.
- **Faithful**: sets `min_contrast` directly (the only field this assignment
  touches); the value is the config minimum contrast (a ratio ≥ 1; the caller
  supplies it).
- **Faithful adaptation**: `update_min_contrast` mutates an existing
  `MetalUniforms` (upstream mutates `self.uniforms`) and takes the value as a
  parameter (upstream reads `config.min_contrast`).
- **Deferred**: the color-space and blending bools that `changeConfig` also sets
  (`use_display_p3` / `use_linear_blending` / `use_linear_correction`), which
  need the config `WindowColorspace` / `AlphaBlending` enums and a config-module
  home; the `padding_extend` flags; a full production `MetalUniforms`
  constructor; and the live config-change call site. (Consumed by a later slice;
  this experiment lands and tests the minimum-contrast update.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/metal/shaders.rs`:
   - add `MetalUniforms::update_min_contrast(&mut self, min_contrast: f32)`
     setting `min_contrast`.
2. Tests (in `shaders.rs`):
   - `update_min_contrast` over a known value (e.g. `4.5`) sets `min_contrast`
     to that value, and leaves the other uniform fields (e.g. `screen_size`,
     `grid_size`, `bg_color`) untouched.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty update_min_contrast
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `update_min_contrast` sets `min_contrast` to the given value and touches
  nothing else — faithful to upstream's `changeConfig` `min_contrast`
  assignment;
- the test passes (the value set, the other fields untouched), and the existing
  tests still pass;
- the color-space/blending bools, the `padding_extend` flags, and the live call
  site stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if `min_contrast` is set wrong, an unrelated uniform
field is changed, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the design is faithful to upstream's `changeConfig`
assignment: `update_min_contrast` is a direct `f32` write to
`MetalUniforms.min_contrast`, with no conversion and no other fields touched;
passing the value as a parameter is the right adaptation of upstream reading
`config.min_contrast`. It judged splitting this from the color-space/blending
bools acceptable — those bools depend on config enums that do not exist in
roastty yet and are separate assignments in upstream even though they share the
`changeConfig` block — and consistent with the recent single-field uniform
experiments. It judged the planned test to cover the core behavior and the
single-field boundary.

Review artifacts:

- Prompt: `logs/codex-review/20260604-083758-d420-prompt.md` (design)
- Result: `logs/codex-review/20260604-083758-d420-last-message.md` (design)

## Result

**Result:** Pass

The minimum-contrast uniform update is now live.

- `roastty/src/renderer/metal/shaders.rs`:
  `MetalUniforms::update_min_contrast(&mut self, min_contrast: f32)` sets
  `self.min_contrast` directly (the only field upstream's `changeConfig`
  `min_contrast` assignment touches).

Test (in `shaders.rs`): `update_min_contrast_sets_min_contrast_only` —
`update_min_contrast(4.5)` → `min_contrast == 4.5`, and `screen_size` /
`grid_size` / `bg_color` unchanged.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2897 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + `lib.rs`/header/`abi_harness.c`)
  clean; `git diff --check` clean.

## Conclusion

The per-frame uniforms now cover the geometry trio (`screen_size`, `cell_size`,
`grid_size`), the cursor group, the background color, and the minimum contrast.
The remaining uniform-update work: the color-space and blending bools
(`use_display_p3` / `use_linear_blending` / `use_linear_correction`), which need
the config `WindowColorspace` / `AlphaBlending` enums and a config-module home
(a deliberate config-layer slice); the `padding_extend` flags; and the macOS
glass override. Then a full production `MetalUniforms` constructor composing the
groups, and the live per-frame call sites that supply the terminal state and
config and run the updates.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation is the approved design:
`update_min_contrast` directly assigns the provided `f32` to `self.min_contrast`
and touches no other field, matching upstream's
`self.uniforms.min_contrast = config.min_contrast`. It judged the test to verify
the assigned value and representative unrelated fields (`screen_size`,
`grid_size`, `bg_color`) unchanged, and the deferred color-space/blending bools,
`padding_extend`, and live call site correctly out of scope. No public C
ABI/header impact; nothing needed to change before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-083946-r420-prompt.md` (result)
- Result: `logs/codex-review/20260604-083946-r420-last-message.md` (result)
