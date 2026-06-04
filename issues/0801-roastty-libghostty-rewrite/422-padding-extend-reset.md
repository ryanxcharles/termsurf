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

# Experiment 422: the padding-extend reset (WindowPaddingColor + reset_padding_extend)

## Description

The `padding_extend` uniform is a bitfield (`EXTEND_{LEFT, RIGHT, UP, DOWN}`)
telling the shader which edges extend the background color into the window
padding. Upstream sets it in two places: a **full-rebuild reset** (in
`rebuildCells`, from the `padding_color` config) and a per-row refinement (later
in `rebuildCells`, via `rowNeverExtendBg`). This experiment ports the reset: the
`WindowPaddingColor` config enum, the `EXTEND_*` bit constants (matching the
shader), and `MetalUniforms::reset_padding_extend` — on an `extend` /
`extend-always` padding color it sets all four edges, on `background` it is a
no-op (upstream's `switch`). The per-row `rowNeverExtendBg` refinement and the
live call site stay deferred.

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`), within the full-rebuild branch
(after `self.cells.reset()`), `padding_extend` is reset from the padding color:

```zig
switch (self.config.padding_color) {
    .background => {},

    // For extension, assume we are extending in all directions.
    // For "extend" this may be disabled due to heuristics below.
    .@"extend", .@"extend-always" => {
        self.uniforms.padding_extend = .{
            .up = true,
            .down = true,
            .left = true,
            .right = true,
        };
    },
}
```

The padding-color config enum (`config/Config.zig`):

```zig
pub const WindowPaddingColor = enum { background, @"extend", @"extend-always" };
```

The shader reads `padding_extend` as a bitfield (`shaders.metal`):
`EXTEND_LEFT = 1`, `EXTEND_RIGHT = 2`, `EXTEND_UP = 4`, `EXTEND_DOWN = 8`. So
"all four edges" is `1 | 2 | 4 | 8 = 15`.

## Rust mapping

`WindowPaddingColor` joins the config module (Experiment 421):

```rust
// roastty/src/config/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowPaddingColor {
    Background,
    Extend,
    ExtendAlways,
}
```

The `EXTEND_*` bit constants (mirroring `shaders.metal`) and
`reset_padding_extend` join `shaders.rs`:

```rust
/// The `padding_extend` uniform bit flags — which edges extend the background
/// into the window padding. Must match `shaders.metal`.
pub(crate) const EXTEND_LEFT: u8 = 1;
pub(crate) const EXTEND_RIGHT: u8 = 2;
pub(crate) const EXTEND_UP: u8 = 4;
pub(crate) const EXTEND_DOWN: u8 = 8;

impl MetalUniforms {
    /// Reset `padding_extend` from the `padding_color` (upstream `rebuildCells`'s
    /// full-rebuild reset): `extend` / `extend-always` set all four edges (the
    /// per-row `rowNeverExtendBg` refinement may later disable some for `extend`);
    /// `background` is a no-op.
    pub(crate) fn reset_padding_extend(&mut self, padding_color: WindowPaddingColor) {
        match padding_color {
            WindowPaddingColor::Background => {}
            WindowPaddingColor::Extend | WindowPaddingColor::ExtendAlways => {
                self.padding_extend = EXTEND_LEFT | EXTEND_RIGHT | EXTEND_UP | EXTEND_DOWN;
            }
        }
    }
}
```

`extend` / `extend-always` set `padding_extend = 15` (all four bits);
`background` leaves the field unchanged — exactly upstream's `switch`.

## Scope / faithfulness notes

- **Ported (bridged)**: the `WindowPaddingColor` config enum, the `EXTEND_*` bit
  constants (matching the shader), and `MetalUniforms::reset_padding_extend`
  (the full-rebuild `padding_extend` reset) — upstream's config enum + the
  `rebuildCells` reset `switch`.
- **Faithful**: the enum variants match upstream (`background` / `extend` /
  `extend-always`); the bit constants match `shaders.metal`
  (`LEFT=1, RIGHT=2, UP=4, DOWN=8`); the reset sets all four edges for the two
  extend modes and is a no-op for `background`, matching the `switch`.
- **Faithful adaptation**: `reset_padding_extend` mutates an existing
  `MetalUniforms` (upstream mutates `self.uniforms`) and takes the padding color
  as a parameter (upstream reads `self.config.padding_color`). The `EXTEND_*`
  constants are the Rust mirror of the shader's `#define`s.
- **Deferred**: the per-row `rowNeverExtendBg` refinement (which may clear `up`
  / `down` for `extend` based on the row content), the rest of the config
  subsystem, a full production `MetalUniforms` constructor, and the live
  full-rebuild call site. (Consumed by a later slice; this experiment lands and
  tests the reset.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add the
   `WindowPaddingColor { Background, Extend, ExtendAlways }` enum.
2. `roastty/src/renderer/metal/shaders.rs`:
   - add the `EXTEND_LEFT` / `EXTEND_RIGHT` / `EXTEND_UP` / `EXTEND_DOWN` `u8`
     constants;
   - add
     `MetalUniforms::reset_padding_extend(&mut self, padding_color: WindowPaddingColor)`.
     Import `WindowPaddingColor` from `crate::config`.
3. Tests:
   - in `shaders.rs`: the `EXTEND_*` constants are `1` / `2` / `4` / `8`
     (matching `shaders.metal`); `reset_padding_extend(Background)` leaves a
     pre-set `padding_extend` unchanged; `reset_padding_extend(Extend)` and
     `reset_padding_extend(ExtendAlways)` set `padding_extend == 15`; and the
     other uniform fields (e.g. `min_contrast`, `bg_color`) untouched.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty reset_padding_extend
cargo test -p roastty extend_bit
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- the `WindowPaddingColor` enum and the `EXTEND_*` constants match upstream /
  the shader, and `reset_padding_extend` sets all four edges for `extend` /
  `extend-always` and is a no-op for `background` (touching nothing else) —
  faithful to upstream's `rebuildCells` reset;
- the tests pass (the bit constants; the three padding-color cases; the
  untouched fields), and the existing tests still pass;
- the per-row refinement and the live call site stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a bit value or enum variant is wrong, the reset sets
the wrong edges (or touches `background`), an unrelated uniform field is
changed, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the design is faithful to upstream:
`WindowPaddingColor::{Background, Extend, ExtendAlways}` matches
`background`/`extend`/`extend-always`, and the `EXTEND_*` constants match both
upstream and roastty's Metal shader exactly (left `1`, right `2`, up `4`, down
`8`). It confirmed the reset behavior is correct, including the unintuitive
`Background` no-op — upstream's `switch` uses `.background => {}` and only sets
all four bits for `extend`/`extend-always`; it does not clear `padding_extend`
in the reset block, so porting that as a no-op is faithful for this slice, with
the later per-row refinement and live full-rebuild call site deferred. It judged
the planned tests to cover the bit contract, all three enum cases, and the
untouched fields.

Review artifacts:

- Prompt: `logs/codex-review/20260604-084847-d422-prompt.md` (design)
- Result: `logs/codex-review/20260604-084847-d422-last-message.md` (design)
