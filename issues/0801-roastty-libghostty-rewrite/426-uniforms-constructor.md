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

# Experiment 426: the production MetalUniforms constructor (new)

## Description

The per-frame uniform-update surface is now fully ported (Experiments 415–425),
but `MetalUniforms` still has only `#[cfg(test)]` constructors. This experiment
ports the **production constructor** — upstream's uniform-init literal in the
renderer `init` — which sets the config-derived fields (`min_contrast`,
`bg_color`, the color-space/blending bools, the cursor sentinel,
`padding_extend`) and leaves the geometry fields to be filled by the geometry
update methods. It composes the already-ported `update_bg_color`
(Experiment 419) and `update_color_config` (Experiment 421), tying the uniform
group together.

## Upstream behavior

In the renderer `init` (`renderer/generic.zig`), the uniforms are initialized:

```zig
.uniforms = .{
    .projection_matrix = undefined,
    .cell_size = undefined,
    .grid_size = undefined,
    .grid_padding = undefined,
    .screen_size = undefined,
    .padding_extend = .{},                    // default — all edges off
    .min_contrast = options.config.min_contrast,
    .cursor_pos = .{ maxInt(u16), maxInt(u16) },
    .cursor_color = undefined,
    .bg_color = .{
        config.background.r, config.background.g, config.background.b,
        @intFromFloat(@round(config.background_opacity * 255.0)),
    },
    .bools = .{
        .cursor_wide = false,
        .use_display_p3 = config.colorspace == .@"display-p3",
        .use_linear_blending = config.blending.isLinear(),
        .use_linear_correction = config.blending == .@"linear-corrected",
    },
},
```

The five geometry fields are `undefined` (the renderer calls
`updateScreenSizeUniforms` / `updateFontGridUniforms` and the `rebuildCells`
resize to fill them before the first frame). `cursor_color` is `undefined` (set
by `addCursor` when a cursor is drawn). The rest are config-derived.

## Rust mapping (`roastty/src/renderer/metal/shaders.rs`)

`MetalUniforms::new` mirrors the init literal, composing the ported update
methods for the `bg_color` and color-bool groups. The `undefined` geometry
fields become zeroed (Rust has no `undefined`; they are overwritten by
`update_screen_size` / `update_font_grid` / `update_grid_size` before the first
draw, exactly as upstream):

```rust
impl MetalUniforms {
    /// Create the per-frame uniforms from the config-derived values (upstream's
    /// renderer `init` literal). The geometry fields (`projection_matrix`,
    /// `screen_size`, `cell_size`, `grid_size`, `grid_padding`) are zeroed here
    /// and filled by `update_screen_size` / `update_font_grid` /
    /// `update_grid_size` before the first draw (upstream's `undefined`).
    pub(crate) fn new(
        min_contrast: f32,
        background: Rgb,
        background_opacity: f64,
        colorspace: WindowColorspace,
        blending: AlphaBlending,
    ) -> Self {
        let mut uniforms = Self {
            projection_matrix: [[0.0; 4]; 4],
            screen_size: [0.0, 0.0],
            cell_size: [0.0, 0.0],
            grid_size: [0, 0],
            _padding0: [0; 12],
            grid_padding: [0.0; 4],
            padding_extend: 0,
            _padding1: [0; 3],
            min_contrast,
            cursor_pos: [u16::MAX, u16::MAX],
            cursor_color: [0, 0, 0, 0],
            bg_color: [0, 0, 0, 0],
            bools: MetalUniformBools {
                cursor_wide: false,
                use_display_p3: false,
                use_linear_blending: false,
                use_linear_correction: false,
            },
            _padding2: [0; 8],
        };
        uniforms.update_bg_color(background, background_opacity);
        uniforms.update_color_config(colorspace, blending);
        uniforms
    }
}
```

`min_contrast` / `cursor_pos` / `padding_extend` are set per the literal;
`bg_color` and the color bools are set by the ported update methods (the same
values the literal computes). `cursor_color` is zeroed (upstream `undefined`,
overwritten by the cursor group when drawn).

## Scope / faithfulness notes

- **Ported (bridged)**: `MetalUniforms::new` — the production uniform
  constructor, upstream's renderer `init` uniform literal, composing
  `update_bg_color` and `update_color_config`.
- **Faithful**: the config-derived fields match the init literal —
  `min_contrast` direct, `cursor_pos = (max, max)`, `padding_extend` default
  `0`, `bg_color` = `[r, g, b, round(opacity * 255)]`, and the bools from the
  colorspace/blending; `cursor_wide` false.
- **Faithful adaptation**: the five geometry fields and `cursor_color` are
  `undefined` upstream; roastty zeroes them (no `undefined` in safe Rust) — they
  are overwritten by the geometry update methods / the cursor group before use,
  exactly as upstream. `new` takes the config-derived values as parameters
  (upstream reads `options.config`).
- **Deferred**: the live renderer `init` that supplies the config values and
  runs the geometry/atlas updates; the custom-shader uniforms; and the rest of
  the renderer init. (Consumed by a later slice; this experiment lands and tests
  the constructor.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/metal/shaders.rs`:
   - add
     `MetalUniforms::new(min_contrast, background, background_opacity, colorspace, blending) -> Self`
     per the mapping above. (`Rgb` and the config enums are already imported.)
2. Tests (in `shaders.rs`):
   - `MetalUniforms::new(4.5, Rgb(10, 20, 30), 0.5, DisplayP3, LinearCorrected)`
     → `min_contrast == 4.5`; `cursor_pos == [u16::MAX, u16::MAX]`;
     `padding_extend == 0`; `bg_color == [10, 20, 30, 128]` (`round(127.5)`);
     `bools` =
     `{ cursor_wide: false, use_display_p3: true, use_linear_blending: true, use_linear_correction: true }`;
     the geometry fields (`projection_matrix`, `screen_size`, `cell_size`,
     `grid_size`, `grid_padding`) and `cursor_color` are zeroed;
   - a second case with `Srgb` + `Native` → all three color bools false.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty uniforms_new
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `MetalUniforms::new` sets the config-derived fields per upstream's init
  literal (composing `update_bg_color` / `update_color_config`), the cursor
  sentinel, and the `padding_extend` default, leaving the geometry fields and
  `cursor_color` zeroed — faithful to upstream's renderer init;
- the tests pass (the constructed field values; the zeroed
  geometry/cursor_color; both colorspace/blending cases), and the existing tests
  still pass;
- the live renderer init and the geometry/atlas updates stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a constructed field differs from upstream's init
literal, a geometry field is set to a non-zero/non-undefined value, or any
public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the design is faithful to upstream's init literal: the
config-derived fields are mapped correctly (`min_contrast`, the `cursor_pos`
sentinel, `padding_extend = 0`, `bg_color` via the existing rounded-opacity
setter, and the color/blending bools via `update_color_config` with
`cursor_wide` left false). It confirmed zeroing the upstream-`undefined`
geometry fields and `cursor_color` is the right Rust adaptation (safe Rust
cannot represent Zig's intentionally-uninitialized values, and the design keeps
the same lifecycle contract — geometry filled by `update_screen_size` /
`update_font_grid` / `update_grid_size`, `cursor_color` set by the cursor path
before use). It noted the constructor should **not** apply the macOS-glass
override (upstream leaves that to `updateFrame`, so keeping it out of `new` is
correct). It judged the tests sufficient (the literal-derived values, both
color/blending bool cases, the rounded alpha, the cursor sentinel, the padding
default, and the zeroed placeholder fields).

Review artifacts:

- Prompt: `logs/codex-review/20260604-090931-d426-prompt.md` (design)
- Result: `logs/codex-review/20260604-090931-d426-last-message.md` (design)

## Result

**Result:** Pass

The production `MetalUniforms` constructor is now live.

- `roastty/src/renderer/metal/shaders.rs`:
  `MetalUniforms::new(min_contrast, background, background_opacity, colorspace, blending) -> Self`
  builds the init literal — `min_contrast` direct, `cursor_pos` the
  `[u16::MAX, u16::MAX]` sentinel, `padding_extend` `0`, the geometry fields and
  `cursor_color` zeroed (upstream's `undefined`) — then composes
  `update_bg_color` (the background + rounded-opacity alpha) and
  `update_color_config` (the color-space/blending bools).

Tests (in `shaders.rs`):

- `uniforms_new_matches_the_init_literal` —
  `new(4.5, Rgb(10, 20, 30), 0.5, DisplayP3, LinearCorrected)` → `min_contrast`
  `4.5`, `cursor_pos [u16::MAX, u16::MAX]`, `padding_extend 0`,
  `bg_color [10, 20, 30, 128]`,
  `bools { cursor_wide: false, use_display_p3: true, use_linear_blending: true, use_linear_correction: true }`,
  and the geometry fields + `cursor_color` zeroed.
- `uniforms_new_srgb_native_leaves_color_bools_false` —
  `new(1.0, Rgb(0, 0, 0), 1.0, Srgb, Native)` → the three color bools false.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2907 passed, 0 failed (+2, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

`MetalUniforms` now has a production constructor that mirrors upstream's
renderer init literal and ties the uniform-update methods together. The
per-frame uniform machinery is complete: `new` (the init defaults) plus the
update groups (geometry, cursor, background + glass override, contrast,
color-space bools, padding-extend) — all the uniform state `drawFrame` and the
renderer init build. The remaining renderer-bridge work is the live wiring: the
renderer `init` that supplies the config and runs the geometry/atlas updates,
the per-frame `updateFrame` / `drawFrame` call sites (which depend on the live
render `State`), the full `neverExtendBg` (awaiting the renderer's terminal-core
row/cell representation) and its per-row `padding_extend` refinement, and the
custom-shader uniforms; beyond the renderer, the other subsystems of the
libghostty→libroastty rewrite.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed `MetalUniforms::new` matches upstream's init
literal: `min_contrast` direct, `cursor_pos` the `[u16::MAX, u16::MAX]`
sentinel, `padding_extend` `0`, `bg_color` set through the rounded-opacity path,
and the color-space/blending bools through `update_color_config` with
`cursor_wide` false; and that it correctly does **not** apply the macOS-glass
override (matching upstream's split between init and `updateFrame`). It
confirmed zeroing the upstream-`undefined` geometry fields and `cursor_color` is
a faithful Rust adaptation for this staged renderer (those fields are populated
by the geometry and cursor update paths before use), and judged the tests to
cover the literal-derived fields, the zeroed placeholders, the rounded alpha,
and both the Display P3 / linear-corrected and sRGB / native bool cases. No
public C ABI/header impact; nothing needed to change before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-091133-r426-prompt.md` (result)
- Result: `logs/codex-review/20260604-091133-r426-last-message.md` (result)
