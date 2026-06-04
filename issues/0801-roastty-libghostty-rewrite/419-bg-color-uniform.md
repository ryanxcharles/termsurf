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

# Experiment 419: the background-color uniform update (update_bg_color)

## Description

Experiments 415–418 ported the geometry and cursor uniform groups. This
experiment ports the **`bg_color`** uniform — the terminal background color
combined with the window background opacity (as the alpha) — which upstream sets
each frame from the terminal state and config. It is a small, self-contained
port (one uniform field from a background `Rgb` and an opacity); the macOS glass
override (which forces the alpha to **transparent** — `bg_color[3] = 0` — under
glass styles, because the glass effect supplies the opacity) is a config-blur
concern and stays deferred.

## Upstream behavior

In `updateFrame` (`renderer/generic.zig`), the `bg_color` uniform is set from
the terminal's background color and the config's background opacity:

```zig
self.uniforms.bg_color = .{
    self.terminal_state.colors.background.r,
    self.terminal_state.colors.background.g,
    self.terminal_state.colors.background.b,
    @intFromFloat(@round(self.config.background_opacity * 255.0)),
};
```

The RGB is the terminal background; the alpha is the window background opacity
(`[0, 1]`) scaled to a byte (`round(opacity * 255)`). A subsequent macOS
`background_blur` glass check may override the alpha to **transparent**
(`bg_color[3] = 0`, since the glass effect handles the opacity) — that is
deferred.

## Rust mapping (`roastty/src/renderer/metal/shaders.rs`)

`Rgb` is already imported (Experiment 417). `update_bg_color` sets the field
from the background color and the opacity:

```rust
impl MetalUniforms {
    /// Update the background-color uniform (upstream `updateFrame`): the terminal
    /// `background` color, with the window `opacity` (`[0, 1]`) as the alpha
    /// (`round(opacity * 255)`). The macOS glass-style override is deferred.
    pub(crate) fn update_bg_color(&mut self, background: Rgb, opacity: f64) {
        self.bg_color = [
            background.r,
            background.g,
            background.b,
            (opacity * 255.0).round() as u8,
        ];
    }
}
```

The alpha is `round(opacity * 255)` truncated to a byte — `f64::round` rounds
half away from zero, matching Zig's `@round`; the `as u8` of the in-range
`[0, 255]` value matches `@intFromFloat`. Only `bg_color` is touched.

## Scope / faithfulness notes

- **Ported (bridged)**: `MetalUniforms::update_bg_color` — the `bg_color`
  uniform (the terminal background RGB + the opacity-derived alpha), upstream's
  `updateFrame` `bg_color` assignment.
- **Faithful**: `bg_color = [r, g, b, round(opacity * 255)]` — the terminal
  background channels and the rounded opacity alpha, the only field that
  assignment touches; the rounding (`@round` half-away-from-zero, then
  `@intFromFloat`) is reproduced by `(opacity * 255.0).round() as u8`.
- **Faithful adaptation**: `update_bg_color` mutates an existing `MetalUniforms`
  (upstream mutates `self.uniforms`) and takes the background `Rgb` and the
  `opacity` as parameters (upstream reads
  `self.terminal_state.colors.background` and `self.config.background_opacity`).
  The opacity is assumed clamped to `[0, 1]` (the caller / config load clamps,
  as in Experiment 405).
- **Deferred**: the macOS `background_blur` glass override (forcing the alpha
  **transparent**, `bg_color[3] = 0`, under glass styles), the
  config/terminal-state plumbing that supplies the background and opacity, the
  config-derived group (min-contrast, color-space and blending bools), a full
  production `MetalUniforms` constructor, and the live call site. (Consumed by a
  later slice; this experiment lands and tests the background-color update.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/metal/shaders.rs`:
   - add
     `MetalUniforms::update_bg_color(&mut self, background: Rgb, opacity: f64)`
     setting `bg_color` from the background channels and the opacity alpha.
     (`Rgb` is already imported.)
2. Tests (in `shaders.rs`):
   - `update_bg_color` with `Rgb(10, 20, 30)` and `opacity = 0.5` →
     `bg_color == [10, 20, 30, 128]` (`round(127.5) = 128`); with
     `opacity = 1.0` → alpha `255`; with `opacity = 0.0` → alpha `0`; and the
     other uniform fields (e.g. `screen_size`, `grid_size`, `cursor_color`)
     untouched.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty update_bg_color
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `update_bg_color` sets `bg_color` to the terminal background channels with the
  rounded opacity alpha (`round(opacity * 255)`) and touches nothing else —
  faithful to upstream's `updateFrame` `bg_color` assignment;
- the tests pass (the half-rounding `0.5 → 128`, the `1.0 → 255` and `0.0 → 0`
  endpoints, the untouched fields), and the existing tests still pass;
- the macOS glass override and the other uniform groups stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the alpha rounding differs from upstream, the RGB
channels are wrong, an unrelated uniform field is changed, or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Low** finding (no Required), now addressed:

- **Low (addressed):** the deferred macOS glass note was inaccurate — upstream's
  glass branch sets `bg_color[3] = 0` (forcing the background alpha
  **transparent**, since the glass effect supplies the opacity), not opaque. The
  doc now describes the deferred override correctly so the later slice ports the
  right behavior.

Codex confirmed the setter itself is faithful:
`[r, g, b, round(opacity * 255)]`, touching only `bg_color`; `f64::round()`
matches Zig `@round` for the in-range clamped opacity values, so `0.5 → 128`,
`1.0 → 255`, and `0.0 → 0` are correct; and passing the background `Rgb` and the
opacity as parameters is the right boundary, with the config/state plumbing
deferred.

Review artifacts:

- Prompt: `logs/codex-review/20260604-083301-d419-prompt.md` (design)
- Result: `logs/codex-review/20260604-083301-d419-last-message.md` (design)

## Result

**Result:** Pass

The background-color uniform update is now live.

- `roastty/src/renderer/metal/shaders.rs`:
  `MetalUniforms::update_bg_color(&mut self, background: Rgb, opacity: f64)`
  sets
  `bg_color = [background.r, background.g, background.b, (opacity * 255.0).round() as u8]`
  — the terminal background channels with the rounded opacity alpha (the only
  field upstream's `updateFrame` `bg_color` assignment touches).

Test (in `shaders.rs`):
`update_bg_color_sets_channels_and_rounded_opacity_alpha` —
`update_bg_color(Rgb(10, 20, 30), 0.5)` → `[10, 20, 30, 128]`
(`round(127.5) = 128`); `(Rgb(1, 2, 3), 1.0)` → `[1, 2, 3, 255]`;
`(Rgb(4, 5, 6), 0.0)` → `[4, 5, 6, 0]`; and `screen_size` / `grid_size` /
`cursor_color` unchanged.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2896 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + `lib.rs`/header/`abi_harness.c`)
  clean; `git diff --check` clean.

## Conclusion

The per-frame uniforms now cover the geometry trio (`screen_size`, `cell_size`,
`grid_size`), the cursor group, and the background color. The remaining
uniform-update work: the config-derived group (min-contrast and the
color-space/blending bools, which first need the config `colorspace`/`blending`
enums), the `padding_extend` flags (the row-by-row extend computation in
`rebuildCells`), and the macOS glass override (`bg_color[3] = 0`); then a full
production `MetalUniforms` constructor composing all the groups, and the live
per-frame call sites that supply the terminal state and config and run the
updates.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation matches the approved design and
the upstream assignment: `bg_color` is set to
`[background.r, background.g, background.b, round(opacity * 255)]`, and the
method body touches only `bg_color`; the rounding is correct
(`255 * 0.5 = 127.5`, and both Zig `@round` and Rust `f64::round()` round that
to `128` before conversion). It judged the test to cover the important cases
(the half rounding, the `1.0 → 255` and `0.0 → 0` endpoints, the RGB channel
propagation, and representative untouched fields), and confirmed the prior Low
finding is resolved (the doc now records the deferred macOS glass override as
`bg_color[3] = 0` / transparent). No public C ABI/header impact; nothing needed
to change before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-083503-r419-prompt.md` (result)
- Result: `logs/codex-review/20260604-083503-r419-last-message.md` (result)
