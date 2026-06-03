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

# Experiment 376: strikethrough and overline decorations

## Description

Experiment 375 ported the underline. This experiment adds the other two
decorations — **strikethrough** and **overline**. Upstream's `addStrikethrough`
and `addOverline` are byte-for-byte identical to `addUnderline` except the
sprite codepoint and the cell `Key`, so this experiment extracts the shared body
into a private `add_sprite_decoration` helper, refactors `add_underline` to use
it, and adds `add_strikethrough`/`add_overline`.

## Upstream behavior

`addOverline`/`addStrikethrough` (`renderer/generic.zig`) each render their
fixed sprite (`Sprite.overline` / `Sprite.strikethrough`) at `cell_width = 1`
and add a `.overline` / `.strikethrough` cell — grayscale atlas, the cell's grid
position, the **foreground** color (the caller passes `fg`), the sprite's atlas
placement/size, and the sprite glyph's bearings. This is exactly `addUnderline`
with a fixed sprite and a different `Key`. The caller guards each on its bool
flag (`if (style.flags.strikethrough)` / `if (style.flags.overline)`).

## Rust mapping (`roastty/src/renderer/cell.rs`)

Extract the common body (the same code `add_underline` already runs) into a
private helper, and add the two writers:

```rust
/// Render a decoration `sprite` through `grid` and add it to `contents` as a
/// `key` cell at `grid_pos` with `color`/`alpha`. The shared body of the
/// decoration writers (underline/strikethrough/overline): a sprite drawn at
/// `cell_width = 1` into the grayscale atlas, with the sprite glyph's own
/// bearings (a decoration has no shaper offset).
fn add_sprite_decoration(
    contents: &mut Contents,
    grid: &mut SharedGrid,
    grid_pos: [u16; 2],
    sprite: Sprite,
    key: Key,
    color: [u8; 3],
    alpha: u8,
) -> Result<(), ResolverRenderError> {
    let opts = RenderOptions {
        grid_metrics: grid.metrics,
        cell_width: Some(1),
        constraint: Constraint::default(),
        constraint_width: 1,
        thicken: false,
        thicken_strength: 255,
    };
    let render = grid.render_glyph(Index::special(Special::Sprite), sprite as u32, &opts)?;
    contents.add(
        key,
        CellTextVertex {
            glyph_pos: [render.glyph.atlas_x, render.glyph.atlas_y],
            glyph_size: [render.glyph.width, render.glyph.height],
            bearings: [
                i16::try_from(render.glyph.offset_x).expect("decoration x bearing fits i16"),
                i16::try_from(render.glyph.offset_y).expect("decoration y bearing fits i16"),
            ],
            grid_pos,
            color: [color[0], color[1], color[2], alpha],
            atlas: CellTextAtlas::Grayscale,
            flags: CellTextFlags::new(false, false),
            _padding: [0, 0],
        },
    );
    Ok(())
}

// `add_underline` becomes: map the variant → sprite (or return on `None`), then
// `add_sprite_decoration(contents, grid, grid_pos, sprite, Key::Underline, color, alpha)`.

/// Render a cell's strikethrough sprite and add a [`Key::Strikethrough`] cell.
/// Faithful port of upstream `addStrikethrough` (the caller guards the flag).
pub(crate) fn add_strikethrough(
    contents: &mut Contents,
    grid: &mut SharedGrid,
    grid_pos: [u16; 2],
    color: [u8; 3],
    alpha: u8,
) -> Result<(), ResolverRenderError> {
    add_sprite_decoration(
        contents, grid, grid_pos, Sprite::Strikethrough, Key::Strikethrough, color, alpha,
    )
}

/// Render a cell's overline sprite and add a [`Key::Overline`] cell. Faithful
/// port of upstream `addOverline` (the caller guards the flag).
pub(crate) fn add_overline(
    contents: &mut Contents,
    grid: &mut SharedGrid,
    grid_pos: [u16; 2],
    color: [u8; 3],
    alpha: u8,
) -> Result<(), ResolverRenderError> {
    add_sprite_decoration(
        contents, grid, grid_pos, Sprite::Overline, Key::Overline, color, alpha,
    )
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: upstream `addStrikethrough` and `addOverline` — each
  renders its fixed sprite (`Sprite::Strikethrough` / `Sprite::Overline`) and
  adds a `Key::Strikethrough` / `Key::Overline` decoration cell. The shared
  `add_sprite_decoration` helper is the common body (also now used by
  `add_underline`).
- **Faithful**: the sprite, atlas (grayscale), grid position, color/alpha,
  placement/size, and glyph-only bearings match upstream exactly; strikethrough
  and overline are unconditional (the caller guards the bool flag, as upstream),
  so they take no `Option`. The refactor of `add_underline` is
  behavior-preserving (its body is the helper verbatim; its tests are
  unchanged).
- **Faithful adaptation**: the `Key` is passed through to `Contents::add`, which
  routes all foreground kinds (`Text`/`Underline`/`Strikethrough`/`Overline`) to
  the same `fg_rows[y + 1]` list — so the `Key` is faithful to upstream but not
  separately observable in `Contents` (it distinguishes draw layers at the GPU
  upload, deferred). The sprite selection (the real per-decoration logic) is
  verified by the cache-identity technique (Experiment 375).
- **Deferred**: the underline-color resolution (`Style::underline_color`); the
  row/viewport integration (calling the decoration writers per decorated cell);
  the cursor cell; the renderer-layer color adjustments; and the Metal upload.
  (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`: extract `add_sprite_decoration` (the body of
   `add_underline`), refactor `add_underline` to map the variant → sprite then
   call it, and add `add_strikethrough` and `add_overline`.
2. Test (in `cell.rs`): for each of strikethrough (`Sprite::Strikethrough`) and
   overline (`Sprite::Overline`), on a fresh Menlo `SharedGrid`/`Contents`, the
   writer adds one cell to `fg_rows[1]` with `grid_pos [1, 0]`, grayscale atlas,
   and the supplied color; a **same-grid** direct render of the expected sprite
   is a cache hit (matching `glyph_pos`/`glyph_size`/`bearings`) only if the
   writer rendered exactly that sprite. (The existing `add_underline` tests
   still pass after the refactor.)
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty add_strikethrough
cargo test -p roastty add_overline
cargo test -p roastty add_underline
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `add_strikethrough`/`add_overline` render the correct sprite and add a
  `Key::Strikethrough`/`Key::Overline` grayscale decoration cell, and the shared
  helper is the common body (with `add_underline` refactored to use it) —
  faithful to upstream;
- the new tests pass (each writer renders its exact sprite), and the existing
  tests (incl. `add_underline`) still pass;
- the underline-color resolution, integration, cursor, color adjustments, and
  Metal upload stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a writer renders the wrong sprite, the refactor
changes `add_underline`'s behavior, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the extraction is clean and behavior-preserving (as
long as `add_underline` keeps the same `Underline → Sprite` match and delegates
to `add_sprite_decoration` with `Key::Underline`), and that the shared helper is
the right shape for all three writers (upstream's
underline/strikethrough/overline emit paths differ only by sprite and key); that
`add_strikethrough`/`add_overline` are faithful as unconditional helpers
(callers own the style-flag guard, and the writers always render
`Sprite::Strikethrough`/`Sprite::Overline` through
`Index::special(Special::Sprite)` at `cell_width: Some(1)`, grayscale, supplied
color/alpha, glyph-only bearings); that routing through
`Key::Strikethrough`/`Key::Overline` is fine even though `Contents` stores all
foreground rows together (the key preserves the API shape and `Contents::add`
treats all foreground kinds consistently); and that the same-grid cache-identity
tests are sufficient (they prove each writer rendered the exact expected sprite,
while the existing `add_underline` tests protect the refactor).

Review artifacts:

- Prompt: `logs/codex-review/20260603-185417-158621-prompt.md` (design)
- Result: `logs/codex-review/20260603-185417-158621-last-message.md` (design)
