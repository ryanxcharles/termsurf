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

# Experiment 400: the preedit cell

## Description

IME **preedit** text (dead-key / composition text) is drawn over the cursor:
each preedit codepoint renders a glyph plus an underline, so the user sees what
they are composing. Upstream's `addPreeditCell` renders the codepoint via
`renderCodepoint` (now `SharedGrid::render_codepoint`, Experiment 392), adds the
glyph as a text cell, and underlines it (a second underline on the next column
for a wide codepoint). This experiment ports `addPreeditCell` as
`add_preedit_cell`, reusing `render_codepoint` and `add_underline`. The preedit
**state** (`Preedit` / `PreeditRange`, `renderer/state.rs`) is already ported;
this is the cell rendering for one codepoint. The placement loop (iterating the
preedit codepoints over the `PreeditRange`) and the under-preedit cell skipping
are deferred.

## Upstream behavior

`addPreeditCell` (`renderer/generic.zig`):

```zig
fn addPreeditCell(self, cp, coord, screen_fg) !void {
    // Render the glyph for our preedit text.
    const render = (self.font_grid.renderCodepoint(
        self.alloc, @intCast(cp.codepoint), .regular, .text,
        .{ .grid_metrics = self.grid_metrics },
    ) catch { … return; }) orelse { … return; };

    // Add our text.
    try self.cells.add(self.alloc, .text, .{
        .atlas = .grayscale,
        .grid_pos = .{ coord.x, coord.y },
        .color = .{ screen_fg.r, screen_fg.g, screen_fg.b, 255 },
        .glyph_pos = .{ render.glyph.atlas_x, render.glyph.atlas_y },
        .glyph_size = .{ render.glyph.width, render.glyph.height },
        .bearings = .{ render.glyph.offset_x, render.glyph.offset_y },
    });

    // Add underline.
    try self.addUnderline(coord.x, coord.y, .single, screen_fg, 255);
    if (cp.wide and coord.x < self.cells.size.columns - 1) {
        try self.addUnderline(coord.x + 1, coord.y, .single, screen_fg, 255);
    }
}
```

So: render the codepoint (`.regular`, `.text`, no `cell_width`); if no font has
it, draw nothing. Add the glyph as a grayscale text cell at the coordinate,
colored `screen_fg` (alpha 255), with the glyph's atlas pos/size/bearings (no
shaper offset — preedit has no shaper cell). Then add a single underline at the
coordinate, and — for a **wide** codepoint that is not in the last column — a
second single underline on the next column.

## Rust mapping (`roastty/src/renderer/cell.rs`)

```rust
/// Render one preedit (IME) codepoint into `contents` at `coord` with `screen_fg`:
/// the glyph (via [`SharedGrid::render_codepoint`], skipped if no font has it) as a
/// grayscale text cell, plus a single underline — and a second underline on the
/// next column for a wide codepoint that is not in the last column. Faithful port
/// of upstream `addPreeditCell`. `cols` is the row's column count (for the
/// wide/last-column check).
#[allow(clippy::too_many_arguments)]
pub(crate) fn add_preedit_cell(
    contents: &mut Contents,
    grid: &mut SharedGrid,
    codepoint: u32,
    wide: bool,
    coord: [u16; 2],
    cols: u16,
    screen_fg: [u8; 3],
) -> Result<(), ResolverRenderError> {
    let opts = RenderOptions {
        grid_metrics: grid.metrics,
        cell_width: None,
        constraint: Constraint::default(),
        constraint_width: 1,
        thicken: false,
        thicken_strength: 255,
    };
    let Some(render) =
        grid.render_codepoint(codepoint, Style::Regular, Some(Presentation::Text), &opts)?
    else {
        // No font has the codepoint — draw nothing (upstream logs and returns).
        return Ok(());
    };

    contents.add(
        Key::Text,
        CellTextVertex {
            glyph_pos: [render.glyph.atlas_x, render.glyph.atlas_y],
            glyph_size: [render.glyph.width, render.glyph.height],
            bearings: [
                i16::try_from(render.glyph.offset_x).expect("preedit x bearing fits i16"),
                i16::try_from(render.glyph.offset_y).expect("preedit y bearing fits i16"),
            ],
            grid_pos: coord,
            color: [screen_fg[0], screen_fg[1], screen_fg[2], 255],
            atlas: CellTextAtlas::Grayscale,
            flags: CellTextFlags::new(false, false),
            _padding: [0, 0],
        },
    );

    // A single underline at the cell, and a second on the next column for a wide
    // codepoint (when it fits).
    add_underline(contents, grid, coord, Underline::Single, screen_fg, 255)?;
    if wide && coord[0] + 1 < cols {
        add_underline(
            contents,
            grid,
            [coord[0] + 1, coord[1]],
            Underline::Single,
            screen_fg,
            255,
        )?;
    }
    Ok(())
}
```

`render_codepoint` (Experiment 392), `add_underline`, `Contents::add`, and the
imports (`Style`, `Presentation`, `RenderOptions`, `Constraint`, the GPU cell
types) are already in scope. The glyph cell is grayscale with
`is_cursor_glyph = false`/`no_min_contrast = false` (upstream sets no bools) and
the glyph's own bearings (no shaper offset, as with the cursor).

## Scope / faithfulness notes

- **Ported (bridged)**: `add_preedit_cell` (upstream `addPreeditCell`) — the per
  preedit codepoint glyph + underline(s).
- **Faithful**: the codepoint renders via `render_codepoint(.., Regular, Text)`
  with no `cell_width` (upstream omits it), and a missing glyph draws nothing;
  the glyph is a grayscale text cell at `coord` with `screen_fg` (alpha 255) and
  the glyph's atlas pos/size/bearings (no shaper offset); a single underline is
  added at the cell, and a second on the next column only for a wide codepoint
  that fits (`coord.x + 1 < cols`, equivalent to upstream's
  `coord.x < columns - 1` without underflow). The cell flags are both `false`,
  as upstream.
- **Faithful adaptation**: `cols` is a parameter (upstream reads
  `self.cells.size.columns`); `coord.x + 1 < cols` avoids the unsigned underflow
  of `columns - 1`. The bearings are `i16::try_from(...)` (upstream `@intCast`).
  The glyph render uses `render_codepoint(…)?`, **propagating** a render error
  rather than catching and logging it as upstream does — intentional and
  consistent with the lock cursor (Experiment 393) and the other `?`-using
  renderer helpers; only the **`None`** (no font has the glyph) case is the
  inline no-op draw-nothing, matching upstream's no-cell outcome.
- **Deferred**: the preedit placement loop (iterating the codepoints over a
  `PreeditRange`, computing each coordinate and `screen_fg`) and the
  under-preedit cell skipping in `rebuild_viewport`; the Metal upload. (Consumed
  by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`: add the `add_preedit_cell` function.
2. Tests (in `cell.rs`):
   - a narrow preedit codepoint `'A'` (which Menlo has) at `coord = [1, 0]`,
     `cols = 4` → two foreground cells at `[1, 0]`: the glyph (grayscale,
     `screen_fg` at alpha 255, matching a directly-rendered
     `render_codepoint('A')`) and a single underline (cache identity vs
     `Sprite::Underline`);
   - a **wide** codepoint (`wide = true`) at `[1, 0]`, `cols = 4` → the glyph +
     a single underline at `[1, 0]` **and** a second single underline at
     `[2, 0]`;
   - a wide codepoint in the **last column** (`coord = [3, 0]`, `cols = 4`) →
     the glyph + one underline at `[3, 0]`, **no** second underline (`3 + 1 < 4`
     is false);
   - a codepoint **no font has** (`0xE000`, discovery off) → nothing added.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty add_preedit_cell
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `add_preedit_cell` renders the codepoint glyph as a grayscale text cell at the
  coordinate (skipped when no font has it) and adds a single underline (a second
  on the next column for a wide codepoint that fits) — faithful to upstream's
  `addPreeditCell`;
- the tests pass (the narrow glyph + underline; the wide second underline; the
  last-column no-second-underline; the missing-codepoint no-op), and the
  existing tests still pass;
- the preedit placement loop and the Metal upload stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the cell is wrong (the wrong color/atlas/bearings,
the underline missing or mis-placed, the wide second underline wrong, a missing
codepoint not skipped), or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Low** finding (no Required), now addressed:

- **Low (addressed):** `add_preedit_cell` uses `render_codepoint(…)?`, which
  propagates render errors, whereas upstream's `addPreeditCell` catches the
  error, logs, and returns without drawing that cell. This is acceptable as the
  same Rust `Result` adaptation used elsewhere; the design now records the
  choice explicitly (as the lock cursor, Experiment 393, did), and the `None`
  path is the correct no-op.

Codex confirmed everything else is faithful: `Regular` + `Text`, no
`cell_width`, the grayscale text cell, the raw glyph bearings with no shaper
offset, `screen_fg` alpha 255, flags `false`/`false`, a single underline at
`coord`, and a second underline only for a wide cell when `coord[0] + 1 < cols`
— a good underflow-safe equivalent to upstream's `coord.x < columns - 1`. It
judged the tests sufficient for the core behavior.

Review artifacts:

- Prompt: `logs/codex-review/20260604-061426-274567-prompt.md` (design)
- Result: `logs/codex-review/20260604-061426-274567-last-message.md` (design)

## Result

**Result:** Pass

The preedit cell is now rendered.

- `roastty/src/renderer/cell.rs`:
  `add_preedit_cell(contents, grid, codepoint, wide, coord, cols, screen_fg)` —
  builds the render options with `cell_width: None`, renders the codepoint via
  `render_codepoint(.., Style::Regular, Some(Presentation::Text))` (a `None`
  result is a no-op), adds a grayscale text cell at `coord` (raw glyph bearings,
  `screen_fg` at alpha 255, `CellTextFlags::new(false, false)`), then a single
  underline at `coord` and — for `wide && coord[0] + 1 < cols` — a second single
  underline at the next column. `pub(crate)` and not yet called in production
  (the preedit placement loop is deferred), reachable in the library crate, so
  no dead-code warning.

Test (in `cell.rs`): `add_preedit_cell_renders_glyph_and_underline` — a narrow
`'A'` at column 1 → the glyph (grayscale, `screen_fg` at alpha 255, matching a
direct `render_codepoint('A')`) + a single underline (cache identity vs
`Sprite::Underline`); a wide cell at column 1 → grid-pos columns `[1, 1, 2]`
(glyph, underline, second underline); a wide cell in the last column (3) → 2
cells (no second underline); and `0xE000` (no font) → nothing.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2859 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer) clean; `git diff --check` clean.

## Conclusion

The preedit cell rendering — the IME composition glyph + underline(s) drawn over
the cursor — is now ported faithfully as `add_preedit_cell`, reusing
`render_codepoint` (Experiment 392) and `add_underline`. Combined with the
ported `Preedit`/`PreeditRange` state, the per-codepoint preedit rendering is
complete; the placement loop (iterating the codepoints over a `PreeditRange`,
computing each coordinate and `screen_fg`, and skipping the under-preedit cells)
and the Metal upload remain.

The remaining renderer-bridge work: the preedit placement loop; and the **Metal
upload** of `Contents` (the GPU buffer/uniform upload — the renderer boundary,
which depends on the GUI's Metal layer that roastty already has the
buffer/uniform types for).

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation matches the approved design
(the `cell_width: None` render options, `Regular`/`Text`, the `None` no-op, the
grayscale text vertex at `coord` with raw glyph bearings / `screen_fg` alpha 255
/ `CellTextFlags::new(false, false)`, the single underline at `coord`, and the
second underline only when `wide && coord[0] + 1 < cols`), that the
design-review Low is addressed (render errors intentionally propagate via `?`,
consistent with the lock cursor and the current Rust renderer-helper style,
while missing glyphs stay a no-op like upstream), and that the tests cover the
narrow, wide, last-column, and missing-codepoint behavior — internal Rust only,
no public C ABI/header impact. Nothing needed to change before the result
commit.

Review artifacts:

- Result review: `logs/codex-review/20260604-061712-569004-last-message.md`
