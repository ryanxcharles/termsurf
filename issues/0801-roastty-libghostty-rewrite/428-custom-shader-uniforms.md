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

# Experiment 428: the custom-shader uniforms value type (CustomShaderUniforms)

## Description

Custom (shadertoy-style) shaders read a large uniform struct
(`shadertoy.Uniforms`) — resolution, time, channels, mouse, the palette, and the
cursor/colors. The renderer `init` constructs it with specific defaults. This
experiment ports that value type, `CustomShaderUniforms`: the
`#[repr(C, align(16))]` struct matching upstream's `extern struct` layout
(verified by offset/size tests), plus a `new()` constructor with the init
defaults. The per-frame/state update methods
(`updateCustomShaderUniformsFromState` / `…ForFrame`, which need the live render
`State`), the `Target` enum, and the shader loading stay deferred.

## Upstream behavior

`shadertoy.Uniforms` (`renderer/shadertoy.zig`) is an `extern struct` with
explicit alignment (the std140-style GPU layout):

```zig
pub const Uniforms = extern struct {
    resolution: [3]f32 align(16),
    time: f32 align(4), time_delta: f32 align(4), frame_rate: f32 align(4), frame: i32 align(4),
    channel_time: [4][4]f32 align(16), channel_resolution: [4][4]f32 align(16),
    mouse: [4]f32 align(16), date: [4]f32 align(16),
    sample_rate: f32 align(4),
    current_cursor: [4]f32 align(16), previous_cursor: [4]f32 align(16),
    current_cursor_color: [4]f32 align(16), previous_cursor_color: [4]f32 align(16),
    current_cursor_style: i32 align(4), previous_cursor_style: i32 align(4),
    cursor_visible: i32 align(4), cursor_change_time: f32 align(4),
    time_focus: f32 align(4), focus: i32 align(4),
    palette: [256][4]f32 align(16),
    background_color: [4]f32 align(16), foreground_color: [4]f32 align(16),
    cursor_color: [4]f32 align(16), cursor_text: [4]f32 align(16),
    selection_background_color: [4]f32 align(16), selection_foreground_color: [4]f32 align(16),
};
```

The renderer `init` literal sets it (the values not yet driven by state are
defaults):

```zig
.resolution = .{ 0, 0, 1 }, .time = 0, .time_delta = 0, .frame_rate = 60, .frame = 0,
.channel_time = 0, .channel_resolution = 0, .mouse = 0, .date = 0, .sample_rate = 0,
.current_cursor = 0, .previous_cursor = 0, .current_cursor_color = 0, .previous_cursor_color = 0,
.current_cursor_style = 0, .previous_cursor_style = 0, .cursor_visible = 0, .cursor_change_time = 0,
.time_focus = 0, .focus = 1, .palette = 0, .background_color = 0, .foreground_color = 0,
.cursor_color = 0, .cursor_text = 0, .selection_background_color = 0, .selection_foreground_color = 0,
```

So all fields default to `0` except `resolution = [0, 0, 1]`, `frame_rate = 60`,
and `focus = 1`.

## Layout

The `align(16)` fields force the std140 offsets. Computing the `extern struct`
layout (each field at the next offset that is a multiple of its alignment;
scalars `align(4)`, vectors/matrices/palette `align(16)`):

| field                                                      | offset            | end  |
| ---------------------------------------------------------- | ----------------- | ---- |
| `resolution` `[f32;3]`                                     | 0                 | 12   |
| `time` / `time_delta` / `frame_rate` / `frame`             | 12 / 16 / 20 / 24 | 28   |
| _(pad)_                                                    | 28                | 32   |
| `channel_time` `[[f32;4];4]`                               | 32                | 96   |
| `channel_resolution`                                       | 96                | 160  |
| `mouse`                                                    | 160               | 176  |
| `date`                                                     | 176               | 192  |
| `sample_rate`                                              | 192               | 196  |
| _(pad)_                                                    | 196               | 208  |
| `current_cursor` … `previous_cursor_color`                 | 208               | 272  |
| `current_cursor_style` … `focus` (6 scalars)               | 272               | 296  |
| _(pad)_                                                    | 296               | 304  |
| `palette` `[[f32;4];256]`                                  | 304               | 4400 |
| `background_color` … `selection_foreground_color` (6 vec4) | 4400              | 4496 |

So `size_of == 4496`, `align_of == 16`, with three padding regions (4, 12, 8
bytes). The Rust `#[repr(C, align(16))]` struct uses explicit `[u8; N]` padding
fields to reproduce these offsets exactly (Rust's `[f32; 4]` has alignment 4, so
the padding — not the field alignment — places the vectors at their 16-aligned
offsets).

## Rust mapping (`roastty/src/renderer/shadertoy.rs`, new)

```rust
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CustomShaderUniforms {
    pub(crate) resolution: [f32; 3],
    pub(crate) time: f32,
    pub(crate) time_delta: f32,
    pub(crate) frame_rate: f32,
    pub(crate) frame: i32,
    _pad0: [u8; 4],
    pub(crate) channel_time: [[f32; 4]; 4],
    pub(crate) channel_resolution: [[f32; 4]; 4],
    pub(crate) mouse: [f32; 4],
    pub(crate) date: [f32; 4],
    pub(crate) sample_rate: f32,
    _pad1: [u8; 12],
    pub(crate) current_cursor: [f32; 4],
    pub(crate) previous_cursor: [f32; 4],
    pub(crate) current_cursor_color: [f32; 4],
    pub(crate) previous_cursor_color: [f32; 4],
    pub(crate) current_cursor_style: i32,
    pub(crate) previous_cursor_style: i32,
    pub(crate) cursor_visible: i32,
    pub(crate) cursor_change_time: f32,
    pub(crate) time_focus: f32,
    pub(crate) focus: i32,
    _pad2: [u8; 8],
    pub(crate) palette: [[f32; 4]; 256],
    pub(crate) background_color: [f32; 4],
    pub(crate) foreground_color: [f32; 4],
    pub(crate) cursor_color: [f32; 4],
    pub(crate) cursor_text: [f32; 4],
    pub(crate) selection_background_color: [f32; 4],
    pub(crate) selection_foreground_color: [f32; 4],
}

impl CustomShaderUniforms {
    /// The renderer-init defaults: all zero except `resolution = [0, 0, 1]`,
    /// `frame_rate = 60`, and `focus = 1`.
    pub(crate) fn new() -> Self {
        Self {
            resolution: [0.0, 0.0, 1.0],
            time: 0.0, time_delta: 0.0, frame_rate: 60.0, frame: 0,
            _pad0: [0; 4],
            channel_time: [[0.0; 4]; 4], channel_resolution: [[0.0; 4]; 4],
            mouse: [0.0; 4], date: [0.0; 4], sample_rate: 0.0,
            _pad1: [0; 12],
            current_cursor: [0.0; 4], previous_cursor: [0.0; 4],
            current_cursor_color: [0.0; 4], previous_cursor_color: [0.0; 4],
            current_cursor_style: 0, previous_cursor_style: 0,
            cursor_visible: 0, cursor_change_time: 0.0, time_focus: 0.0, focus: 1,
            _pad2: [0; 8],
            palette: [[0.0; 4]; 256],
            background_color: [0.0; 4], foreground_color: [0.0; 4],
            cursor_color: [0.0; 4], cursor_text: [0.0; 4],
            selection_background_color: [0.0; 4], selection_foreground_color: [0.0; 4],
        }
    }
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: the `CustomShaderUniforms` value type (the
  `extern struct` layout) and its `new()` init defaults — upstream's
  `shadertoy.Uniforms` and the renderer-init literal.
- **Faithful**: the field order, types, and offsets match the `extern struct`
  (`align(16)` reproduced by explicit padding); `size_of == 4496`,
  `align_of == 16`; `new()` matches the init literal (all zero except
  `resolution = [0, 0, 1]`, `frame_rate = 60`, `focus = 1`).
- **Faithful adaptation**: Rust `[f32; 4]` has alignment 4 (not 16), so explicit
  `_pad0`/`_pad1`/`_pad2` `[u8; N]` fields place the vectors/matrices/palette at
  their std140 offsets; `#[repr(C, align(16))]` makes the struct 16-aligned
  (size rounds to a 16-multiple — already `4496`). A new `renderer/shadertoy.rs`
  module (upstream's `renderer/shadertoy.zig` home).
- **Deferred**: the per-frame/state update methods
  (`updateCustomShaderUniformsFromState` / `…ForFrame`, which need the live
  render `State`), the `Target` enum, the shader loading (`loadFromFiles`), and
  the GPU custom-shader pipeline. (Consumed by a later slice; this experiment
  lands the value type + its defaults, verified by the layout.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/shadertoy.rs` (new, `#![allow(dead_code)]`): the
   `CustomShaderUniforms` struct and `new()`.
2. `roastty/src/renderer/mod.rs`: add `pub(crate) mod shadertoy;`.
3. Tests (in `shadertoy.rs`):
   - **layout**: `size_of::<CustomShaderUniforms>() == 4496`, `align_of == 16`,
     and `offset_of!` for the key fields — `resolution` 0, `frame` 24,
     `channel_time` 32, `channel_resolution` 96, `mouse` 160, `date` 176,
     `sample_rate` 192, `current_cursor` 208, `current_cursor_style` 272,
     `focus` 292, `palette` 304, `background_color` 4400,
     `selection_foreground_color` 4480;
   - **defaults**: `new()` has `resolution == [0, 0, 1]`, `frame_rate == 60`,
     `focus == 1`, and representative zeroed fields (`time`, `frame`,
     `cursor_visible`, `palette[0]`, `background_color`).
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty custom_shader_uniforms
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `CustomShaderUniforms` matches upstream's `extern struct` layout
  (`size_of == 4496`, `align_of == 16`, the verified field offsets), and `new()`
  matches the renderer-init defaults — faithful to upstream;
- the tests pass (the layout offsets/size/align; the `new()` defaults), and the
  existing tests still pass;
- the update methods, the `Target` enum, and the shader loading stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates (now including
  `roastty/src/renderer/shadertoy.rs`) and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a field offset / the size / the alignment differs
from the `extern struct`, a `new()` default is wrong, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**, verifying the offset math: `resolution` 0..12, then the scalars
through `frame` end at 28 so `_pad0: [u8; 4]` moves `channel_time` to 32;
`sample_rate` ends at 196 so `_pad1: [u8; 12]` moves `current_cursor` to 208;
`focus` ends at 296 so `_pad2: [u8; 8]` moves `palette` to 304; `palette` is
`256 × 4 × 4 = 4096` bytes (`304..4400`); the six trailing vec4 fields end at
`4496`, already a multiple of 16, so `repr(C, align(16))` gives size `4496` /
align `16`. It confirmed the Rust layout strategy is sound — since Rust arrays
have element alignment (`[f32; 4]` align 4), `repr(C, align(16))` alone would
not reproduce Zig's field-level `align(16)` offsets, so the explicit padding is
necessary and correctly placed — and that the planned `offset_of!` checks cover
the important boundaries (each padding region via the field before and after
it). It confirmed `new()` matches the upstream init literal (all zero except
`resolution = [0, 0, 1]`, `frame_rate = 60.0`, `focus = 1`).

Review artifacts:

- Prompt: `logs/codex-review/20260604-092242-d428-prompt.md` (design)
- Result: `logs/codex-review/20260604-092242-d428-last-message.md` (design)
