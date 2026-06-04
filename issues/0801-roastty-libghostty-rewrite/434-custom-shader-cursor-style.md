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

# Experiment 434: the custom-shader cursor visibility and style (update_cursor_style)

## Description

Experiment 430 ported the per-frame cursor _position/color_ half of the
custom-shader cursor handling (`current_cursor` / `current_cursor_color`,
shifted to `previous_*` on change, stamping `cursor_change_time`). This
experiment ports the cursor visibility/style block of
`updateCustomShaderUniformsFromState`: **`cursor_visible`** and the
**cursor-style** pair (`current_cursor_style` / `previous_cursor_style`).
`cursor_visible` is set from whether the cursor is visible; the style is the
renderer cursor `Style` encoded as an integer, shifted `current` → `previous`
**unconditionally** every update (upstream does not guard on a change here). The
visibility and style are parameters (deferring the live terminal state). It adds
a `Style::shader_int` encoding on the renderer `Style` enum.

## Upstream behavior

At the end of `updateCustomShaderUniformsFromState` (`renderer/generic.zig`,
after the selection colors):

```zig
// Cursor visibility
uniforms.cursor_visible = @intFromBool(self.terminal_state.cursor.visible);

// Cursor style
const cursor_style: renderer.CursorStyle = .fromTerminal(self.terminal_state.cursor.visual_style);
uniforms.previous_cursor_style = uniforms.current_cursor_style;
uniforms.current_cursor_style = @as(i32, @intFromEnum(cursor_style));
```

`cursor_visible` is `1` when the terminal cursor is visible, `0` otherwise. The
style is the renderer cursor style (`renderer.CursorStyle`, i.e. the renderer
`Style` from `fromTerminal`) as its enum integer, written **unconditionally**:
`previous_cursor_style` is always set to the prior `current_cursor_style`, then
`current_cursor_style` is set to the new style's integer — there is **no**
change-guard here (a repeated same style still copies current → previous). The
upstream `renderer.cursor.Style` enum order is
`block, block_hollow, bar, underline, lock` (0–4), matching roastty's `Style`,
so `@intFromEnum` is the declaration-order integer.

## Rust mapping (`roastty/src/renderer/cursor.rs` + `shadertoy.rs`)

The renderer `Style` enum (Experiment 223 — `Block`, `BlockHollow`, `Bar`,
`Underline`, `Lock`, in that declaration order, mirroring upstream's
`CursorStyle` order) gets a stable integer encoding for the shader:

```rust
impl Style {
    /// The integer encoding the custom shader reads (upstream
    /// `@intFromEnum(cursor_style)`): the `Style` declaration order.
    pub(crate) fn shader_int(self) -> i32 {
        match self {
            Style::Block => 0,
            Style::BlockHollow => 1,
            Style::Bar => 2,
            Style::Underline => 3,
            Style::Lock => 4,
        }
    }
}
```

`update_cursor_style` sets the visibility and the (unconditionally shifted)
style:

```rust
impl CustomShaderUniforms {
    /// Update the cursor visibility and style uniforms (the cursor
    /// visibility/style block of upstream `updateCustomShaderUniformsFromState`):
    /// `cursor_visible` is `1` when `visible`, else `0`; `previous_cursor_style`
    /// is always set to the prior `current_cursor_style`, then
    /// `current_cursor_style` to the new style's `shader_int` — unconditionally
    /// (upstream does not guard on a change).
    pub(crate) fn update_cursor_style(&mut self, visible: bool, style: Style) {
        self.cursor_visible = i32::from(visible);
        self.previous_cursor_style = self.current_cursor_style;
        self.current_cursor_style = style.shader_int();
    }
}
```

`cursor_visible = i32::from(visible)` is upstream's `@intFromBool`; the style is
the renderer `Style`'s `shader_int` (the enum's declaration-order integer,
matching `@intFromEnum`), with the unconditional
`previous = current; current = new` shift — exactly upstream's two assignments.

## Scope / faithfulness notes

- **Ported (bridged)**: `Style::shader_int` (the renderer cursor style's shader
  integer) and `CustomShaderUniforms::update_cursor_style` — the cursor
  visibility/style block of upstream's `updateCustomShaderUniformsFromState`.
- **Faithful**: `cursor_visible = 1`/`0` from the visible state; the style is
  the renderer cursor style's enum integer; `previous_cursor_style` is always
  set to the prior `current_cursor_style`, then `current_cursor_style` to the
  new style — the unconditional two-assignment shift, with no change-guard,
  exactly as upstream.
- **Faithful adaptation**: the visibility and style are parameters (upstream
  reads `self.terminal_state.cursor.visible` and computes the style from
  `self.terminal_state.cursor.visual_style` via `fromTerminal`), as a `bool` and
  a `Style` (the already-resolved renderer style); `shader_int` is the
  declaration-order integer (roastty's `Style` order matches upstream's
  `renderer.cursor.Style` order `block, block_hollow, bar, underline, lock`, so
  it matches `@intFromEnum`); `i32::from(bool)` is `@intFromBool`.
- **Deferred**: `cursor_change_time` is **not** touched here — upstream's
  visibility/style block does not stamp it (the per-frame cursor rect/color
  change, Experiment 430, owns that stamp). Also deferred: the live terminal
  state (the `cursor.visible` bool and the `fromTerminal` resolution of the
  style), the `dirty` gate, and the `has_custom_shaders` gate. (Consumed by a
  later slice; this experiment lands and tests the visibility/style update.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cursor.rs`:
   - add `Style::shader_int(self) -> i32` (the declaration-order encoding).
2. `roastty/src/renderer/shadertoy.rs`:
   - add
     `CustomShaderUniforms::update_cursor_style(&mut self, visible: bool, style: Style)`.
     Import `Style` from `crate::renderer::cursor`.
3. Tests:
   - `Style::shader_int` (in `cursor.rs`): `Block→0`, `BlockHollow→1`, `Bar→2`,
     `Underline→3`, `Lock→4`.
   - `update_cursor_style` (in `shadertoy.rs`): from `new()`
     (`current_cursor_style == 0`, `previous_cursor_style == 0`,
     `cursor_visible == 0`):
     - `update_cursor_style(true, Style::Bar)` → `cursor_visible == 1`,
       `previous_cursor_style == 0` (the prior current),
       `current_cursor_style == 2`;
     - then `update_cursor_style(true, Style::Bar)` again → **unconditional**
       shift: `previous_cursor_style == 2` (the prior current),
       `current_cursor_style == 2`, `cursor_visible == 1`;
     - then `update_cursor_style(false, Style::Underline)` →
       `cursor_visible == 0`, `previous_cursor_style == 2` (the prior current),
       `current_cursor_style == 3`;
     - and `cursor_change_time` stays `0.0` throughout (not touched here), and
       an unrelated field (`focus`) untouched.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty shader_int
cargo test -p roastty update_cursor_style
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `update_cursor_style` sets `cursor_visible` from the visible state and shifts
  the style unconditionally (`previous = prior current`, `current = new`), with
  the style integer from `Style::shader_int` — faithful to upstream's cursor
  visibility/style block;
- `Style::shader_int` returns the declaration-order integer for all five styles;
- the tests pass (the encoding; the visibility; the unconditional shift,
  including a repeated same style still copying current → previous;
  `cursor_change_time` not touched), and the existing tests still pass;
- `cursor_change_time` and the live terminal state stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if `cursor_visible` is wrong, the style integer is
wrong, the shift is guarded (not unconditional) or otherwise wrong,
`cursor_change_time` is touched here, an unrelated field changes, or any public
C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised **two Required
findings**, both fixed:

- **Required (fixed)**: the style update must be **unconditional**, not
  change-guarded. Codex cited the vendored upstream
  (`vendor/ghostty/src/renderer/generic.zig`): `previous_cursor_style` is always
  set to the prior `current_cursor_style`, then `current_cursor_style` to the
  new style — a repeated same style still copies current → previous. Fixed by
  removing the `if new_style != current` guard and writing both assignments
  unconditionally; the test's "repeated same style" case now expects the shift.
- **Required (fixed)**: the style parameter should be `Style`, not
  `Option<Style>`. Upstream always computes the style from
  `self.terminal_state.cursor.visual_style` via `fromTerminal` and writes it,
  independent of visibility — it does not model an absent style. Fixed by taking
  `style: Style`.

Codex confirmed the non-findings: `shader_int` as the explicit declaration-order
mapping (`Block=0`, `BlockHollow=1`, `Bar=2`, `Underline=3`, `Lock=4`) is
correct for `@intFromEnum`, and not touching `cursor_change_time` here is
correct (upstream's visibility/style block does not stamp it; the cursor
rect/color change owns that timestamp). The revised design was verified directly
against the vendored upstream block (`generic.zig`, the cursor visibility/style
at the end of `updateCustomShaderUniformsFromState`) and the upstream
`renderer.cursor.Style` enum order
(`block, block_hollow, bar, underline, lock`).

Review artifacts:

- Prompt: `logs/codex-review/20260604-095535-d434-prompt.md` (design)
- Result: `logs/codex-review/20260604-095535-d434-last-message.md` (design)

## Result

**Result:** Pass

The custom-shader cursor visibility and style are now live, completing
`updateCustomShaderUniformsFromState`.

- `roastty/src/renderer/cursor.rs`: `Style::shader_int(self) -> i32` — the
  declaration-order integer (`Block=0`, `BlockHollow=1`, `Bar=2`, `Underline=3`,
  `Lock=4`), matching upstream `@intFromEnum` of `renderer.cursor.Style`.
- `roastty/src/renderer/shadertoy.rs`:
  `CustomShaderUniforms::update_cursor_style(&mut self, visible: bool, style: Style)`
  sets `cursor_visible = i32::from(visible)`, then **unconditionally**
  `previous_cursor_style = current_cursor_style` and
  `current_cursor_style = style.shader_int()`. Added
  `use crate::renderer::cursor::Style;`.

Tests:

- `shader_int_encodes_declaration_order` (in `cursor.rs`) — the five-style
  encoding.
- `update_cursor_style_sets_visibility_and_shifts_unconditionally` (in
  `shadertoy.rs`) — from `new()`: `update_cursor_style(true, Bar)` →
  `cursor_visible 1`, `previous 0`, `current 2`; repeated `(true, Bar)` →
  `previous 2` (unconditional shift), `current 2`; `(false, Underline)` →
  `cursor_visible 0`, `previous 2`, `current 3`; `cursor_change_time` stays
  `0.0`, `focus` untouched.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2918 passed, 0 failed (+2, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

`updateCustomShaderUniformsFromState` is now fully ported across three
experiments — the palette (432), the colors (433), and the cursor
visibility/style (this one) — all driven by parameters with the live terminal
state and the `dirty` gate deferred. Both `updateCustomShaderUniformsForFrame`
(429–431) and `updateCustomShaderUniformsFromState` (432–434) are now complete
at the uniform level. The remaining custom-shader work — the `Target` enum
(`glsl`/`msl`) and the shader loading (`loadFromFiles`) — stays deferred, along
with the `dirty` gate, the broader live per-frame call sites, and the
`neverExtendBg` terminal-core row/cell access; beyond the renderer, the other
subsystems.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed both Required design findings are resolved:
`update_cursor_style` matches the vendored upstream block —
`cursor_visible = i32::from(visible)`, then `previous_cursor_style` assigned
from the prior `current_cursor_style` unconditionally (no change-guard) and
`current_cursor_style` from the style integer, with no `Option<Style>`. It
confirmed `shader_int` encodes the upstream declaration order (`Block=0` …
`Lock=4`), and that the tests cover the mapping, the unconditional
repeated-style shift, a style change while hidden, and that `cursor_change_time`
is not touched. No public C ABI/header impact; nothing needed to change before
the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-100004-r434-prompt.md` (result)
- Result: `logs/codex-review/20260604-100004-r434-last-message.md` (result)
