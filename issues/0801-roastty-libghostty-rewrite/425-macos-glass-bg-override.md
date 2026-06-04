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

# Experiment 425: the macOS-glass bg_color override (apply_macos_glass_bg_override)

## Description

Experiment 419 set the `bg_color` uniform (the terminal background + the
opacity-derived alpha) but left the macOS-glass override deferred; Experiment
424 ported the `BackgroundBlur` config enum with `is_macos_glass`. This
experiment ports the override — on a macOS glass style the background alpha is
forced to `0` so the glass effect itself supplies the opacity — completing the
`bg_color` group. roastty is macOS-only, so the upstream `comptime` macOS guard
is omitted.

## Upstream behavior

In `updateFrame` (`renderer/generic.zig`), right after the `bg_color`
assignment:

```zig
// If we're on macOS and have glass styles, we remove
// the background opacity because the glass effect handles it.
if (comptime builtin.os.tag == .macos) switch (self.config.background_blur) {
    .@"macos-glass-regular",
    .@"macos-glass-clear",
    => self.uniforms.bg_color[3] = 0,

    else => {},
};
```

So under a glass `background_blur`, the alpha channel of `bg_color` is zeroed
(the RGB channels are untouched); for any non-glass blur it is a no-op.

## Rust mapping (`roastty/src/renderer/metal/shaders.rs`)

`BackgroundBlur::is_macos_glass` (Experiment 424) is the override's predicate:

```rust
impl MetalUniforms {
    /// Apply the macOS glass `bg_color` override (upstream `updateFrame`): under a
    /// macOS glass `blur` style, the background alpha is zeroed (the glass effect
    /// supplies the opacity); for a non-glass blur it is a no-op. macOS-only.
    pub(crate) fn apply_macos_glass_bg_override(&mut self, blur: BackgroundBlur) {
        if blur.is_macos_glass() {
            self.bg_color[3] = 0;
        }
    }
}
```

It zeroes only the alpha channel (`bg_color[3]`), leaving the RGB — exactly
upstream. The caller runs it after `update_bg_color` (which set the alpha from
the opacity).

## Scope / faithfulness notes

- **Ported (bridged)**: `MetalUniforms::apply_macos_glass_bg_override` — the
  macOS-glass alpha override of `bg_color`, completing the `bg_color` group
  begun in Experiment 419.
- **Faithful**: under a glass style (`is_macos_glass`) the override sets
  `bg_color[3] = 0` and leaves the RGB; for any non-glass blur it is a no-op —
  matching upstream's `switch`. (roastty is macOS-only, so the `comptime macos`
  guard is omitted, per the project's macOS-only policy.)
- **Faithful adaptation**: the override is a method taking the `BackgroundBlur`
  config as a parameter (upstream reads `self.config.background_blur`), using
  `is_macos_glass` (Experiment 424) for the glass condition.
- **Deferred**: the live call site that runs `update_bg_color` then this
  override each frame from the terminal state and config; a full production
  `MetalUniforms` constructor; `parseCLI` / `cval` of `BackgroundBlur`; and the
  rest of config. (Consumed by a later slice; this experiment lands and tests
  the override.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/metal/shaders.rs`:
   - add
     `MetalUniforms::apply_macos_glass_bg_override(&mut self, blur: BackgroundBlur)`
     zeroing `bg_color[3]` under a glass style. Import `BackgroundBlur` from
     `crate::config`.
2. Tests (in `shaders.rs`):
   - after `update_bg_color(Rgb(10, 20, 30), 1.0)` (alpha `255`),
     `apply_macos_glass_bg_override(MacosGlassRegular)` →
     `bg_color == [10, 20, 30, 0]` (alpha zeroed, RGB kept); then **restore the
     alpha to `255`** (re-run `update_bg_color`) and
     `apply_macos_glass_bg_override(MacosGlassClear)` → alpha `0` again (so a
     regular-only implementation would fail this arm);
     `apply_macos_glass_bg_override(True)` and `(Radius(5))` on a nonzero alpha
     leave `bg_color` unchanged; and the other uniform fields untouched.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty macos_glass
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `apply_macos_glass_bg_override` zeroes `bg_color[3]` (leaving the RGB) under a
  glass style and is a no-op for a non-glass blur — faithful to upstream's
  `updateFrame` glass `switch`;
- the tests pass (the two glass styles zero the alpha; the non-glass blurs leave
  `bg_color`; the RGB and other fields untouched), and the existing tests still
  pass;
- the live call site and a full constructor stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the override zeroes the wrong channel, fires for a
non-glass blur, changes the RGB, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Low** finding (no Required), now addressed:

- **Low (addressed):** the glass-style test should reset `bg_color[3]` to a
  nonzero value before the `MacosGlassClear` check — otherwise, applying
  `MacosGlassRegular` first leaves the alpha `0`, and a regular-only
  implementation could still pass the `MacosGlassClear` arm. The test now
  restores the alpha to `255` (re-running `update_bg_color`) before the
  `MacosGlassClear` assertion, directly protecting both upstream `switch` arms.

Codex confirmed the method itself is faithful: under `blur.is_macos_glass()` it
zeroes only `bg_color[3]`, leaves the RGB untouched, and is a no-op for
non-glass blur values; omitting upstream's `comptime macos` guard is acceptable
for this roastty Metal/macOS-only slice; and the caller-order expectation
(`update_bg_color` followed by this override) is the right boundary.

Review artifacts:

- Prompt: `logs/codex-review/20260604-090309-d425-prompt.md` (design)
- Result: `logs/codex-review/20260604-090309-d425-last-message.md` (design)
