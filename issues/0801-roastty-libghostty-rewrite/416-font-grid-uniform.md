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

# Experiment 416: the font-grid uniform update (update_font_grid)

## Description

Experiment 415 ported the screen-size uniform group (projection, grid padding,
screen size). This experiment ports the next upstream uniform-update function,
`updateFontGridUniforms`, which sets the **`cell_size`** uniform from the font
grid metrics (the pixel width/height of one glyph cell). It is called whenever
the font grid changes (a font/size/DPI change). It is a small, self-contained
1:1 port — one uniform field from the metrics — matching upstream's separate
per-group update functions; the other uniform groups (grid size, min-contrast,
colors, cursor, color-space bools) have their own updates and stay out of scope.

## Upstream behavior

`updateFontGridUniforms` (`renderer/generic.zig`) sets just the cell size from
the grid metrics:

```zig
fn updateFontGridUniforms(self: *Self) void {
    self.uniforms.cell_size = .{
        @floatFromInt(self.grid_metrics.cell_width),
        @floatFromInt(self.grid_metrics.cell_height),
    };
}
```

It is invoked from `setFontGrid` after the metrics are updated, then a full
rebuild is forced (the rebuild/dirty handling is separate).

## Rust mapping (`roastty/src/renderer/metal/shaders.rs`)

roastty's font `Metrics` exposes `cell_width: u32` and `cell_height: u32`.
`update_font_grid` sets the `cell_size` uniform from them:

```rust
impl MetalUniforms {
    /// Update the font-grid-derived uniform field (upstream
    /// `updateFontGridUniforms`): the `cell_size` (the pixel width/height of one
    /// glyph cell), from the grid `metrics`.
    pub(crate) fn update_font_grid(&mut self, metrics: &Metrics) {
        self.cell_size = [metrics.cell_width as f32, metrics.cell_height as f32];
    }
}
```

`cell_size` is `[f32; 2]` (`[width, height]`), matching upstream's
`@floatFromInt` of the two `u32` metrics. Only `cell_size` is touched.

## Scope / faithfulness notes

- **Ported (bridged)**: `MetalUniforms::update_font_grid` — the `cell_size`
  uniform from the grid metrics, upstream's `updateFontGridUniforms`.
- **Faithful**: sets `cell_size = [cell_width, cell_height]` (as `f32`) from the
  metrics, the only field upstream's function touches; the `[width, height]`
  order matches.
- **Faithful adaptation**: `update_font_grid` mutates an existing
  `MetalUniforms` (upstream mutates `self.uniforms`) and takes the metrics by
  reference (upstream reads `self.grid_metrics`). The `setFontGrid` call site
  (the atlas/dirty handling around it) is deferred.
- **Deferred**: the grid-size uniform (set on a grid resize in `rebuildCells`),
  the config-derived group (min-contrast, color-space and blending bools), the
  background color, the cursor group, a full production `MetalUniforms`
  constructor, and the live call sites. (Consumed by a later slice; this
  experiment lands and tests the font-grid update.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/metal/shaders.rs`:
   - add `MetalUniforms::update_font_grid(&mut self, metrics: &Metrics)` setting
     `cell_size` from the metrics. Import `Metrics` (from the font metrics
     module).
2. Tests (in `shaders.rs`):
   - `update_font_grid` over a `Metrics` with a known `cell_width` /
     `cell_height` sets `cell_size` to `[width, height]` (as `f32`), and leaves
     the other uniform fields (e.g. `screen_size`, `grid_size`, `bg_color`)
     untouched.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty update_font_grid
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `update_font_grid` sets `cell_size` to `[cell_width, cell_height]` (as `f32`)
  from the metrics and touches nothing else — faithful to upstream's
  `updateFontGridUniforms`;
- the test passes (the `cell_size` set, the other fields untouched), and the
  existing tests still pass;
- the grid-size / config / cursor uniform groups and the live call site stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if `cell_size` is set wrong (or in the wrong order), an
unrelated uniform field is changed, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed this is a faithful 1:1 slice of upstream
`updateFontGridUniforms`: `cell_size` is set to `[cell_width, cell_height]` as
`f32` in the correct order, and no other uniform fields are touched; taking
`&Metrics` and mutating an existing `MetalUniforms` is the right Rust adaptation
of upstream reading `self.grid_metrics` and writing `self.uniforms`. It judged
the scope thin but acceptable — upstream keeps this as its own update function,
and Issue 801 is already moving through small bridge experiments per renderer
boundary; keeping the grid-size, config, background-color, cursor, and live
call-site wiring deferred is consistent and avoids mixing separate update
triggers. It judged the planned test sufficient (the value/order and the
protection against accidental mutation of unrelated fields).

Review artifacts:

- Prompt: `logs/codex-review/20260604-081944-d416-prompt.md` (design)
- Result: `logs/codex-review/20260604-081944-d416-last-message.md` (design)
