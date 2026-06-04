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

# Experiment 396: the block-cursor position uniform

## Description

A **block** cursor is drawn by the shader from two uniforms: `cursor_pos` (the
cell it covers) and `cursor_wide` (whether it spans two cells). Upstream
computes them from the cursor's viewport position and the under-cursor cell's
`wide` kind — a **spacer tail** moves the cursor back one column (it sits over
the wide character), and the cursor is "wide" for a wide cell or its spacer
tail. This experiment ports that computation as `block_cursor_pos`, the CPU-side
counterpart of the block-cursor uniforms. The uniform buffer that carries the
values to the shader (and the only-for-block gating) is part of the deferred
Metal upload.

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`), inside `if (style == .block)`:

```zig
const wide = state.cursor.cell.wide;

self.uniforms.cursor_pos = .{
    switch (wide) {
        .narrow, .spacer_head, .wide => cursor_vp.x,
        .spacer_tail => cursor_vp.x -| 1,
    },
    @intCast(cursor_vp.y),
};

self.uniforms.bools.cursor_wide = switch (wide) {
    .narrow, .spacer_head => false,
    .wide, .spacer_tail => true,
};
```

So `cursor_pos` is `[x', y]` where `x'` is the cursor column, moved back one
(saturating) only for a **spacer tail** (so a cursor on a wide character's tail
renders over the character itself); `narrow`/`spacer_head`/`wide` keep `x`.
`cursor_wide` is true for a **wide** cell or its **spacer tail**, false for
**narrow** or **spacer head**.

## Rust mapping (`roastty/src/renderer/cell.rs`)

```rust
/// Compute a **block** cursor's position uniforms — `cursor_pos` (the cell it
/// covers) and `cursor_wide` (whether it spans two cells) — from the cursor's
/// viewport `(x, y)` and the under-cursor cell's [`Wide`] kind (upstream's
/// block-cursor `uniforms.cursor_pos`/`bools.cursor_wide`). A spacer tail moves
/// the cursor back one column (saturating — it sits over the wide character); the
/// cursor is "wide" for a wide cell or its spacer tail.
pub(crate) fn block_cursor_pos(x: u16, y: u16, wide: Wide) -> ([u16; 2], bool) {
    let cursor_x = match wide {
        Wide::SpacerTail => x.saturating_sub(1),
        Wide::Narrow | Wide::SpacerHead | Wide::Wide => x,
    };
    let cursor_wide = matches!(wide, Wide::Wide | Wide::SpacerTail);
    ([cursor_x, y], cursor_wide)
}
```

`Wide` is already imported. The result is the two block-cursor uniform values;
the caller (deferred) computes them only for a block cursor and uploads them to
the shader.

## Scope / faithfulness notes

- **Ported (bridged)**: the block-cursor position uniforms (upstream's
  `cursor_pos`/`cursor_wide`) as `block_cursor_pos` — the spacer-tail column
  adjustment and the wide flag.
- **Faithful**: `cursor_pos.x` subtracts one (saturating) only for a
  `SpacerTail` and keeps `x` for `Narrow`/`SpacerHead`/`Wide`; `cursor_pos.y` is
  the cursor row; `cursor_wide` is `Wide`/`SpacerTail` → true,
  `Narrow`/`SpacerHead` → false — upstream's exact `switch (wide)` arms. The
  saturating subtraction matches upstream's `-|`.
- **Faithful adaptation**: the function returns the `(pos, wide)` pair (upstream
  writes two separate uniform fields); the cursor `(x, y)` is passed in (roastty
  has no cursor viewport state object here). It is computed unconditionally —
  the only-for-block gating is the caller's, as upstream guards with
  `if (style == .block)`.
- **Deferred**: the uniform buffer that carries these to the shader (and the
  block-only gating), part of the Metal upload; the `cursor_color` uniform (the
  under-cursor recolor, Experiment 394, already computed); the column-ordered
  decoration merge + link double-underline. (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`: add the `block_cursor_pos` function.
2. Tests (in `cell.rs`): a `block_cursor_pos_*` test —
   - **Narrow** at `(5, 2)` → `([5, 2], false)`;
   - **Wide** at `(5, 2)` → `([5, 2], true)`;
   - **SpacerTail** at `(5, 2)` → `([4, 2], true)` (moved back one, wide);
   - **SpacerHead** at `(5, 2)` → `([5, 2], false)`;
   - **SpacerTail** at `(0, 0)` → `([0, 0], true)` (saturating, no underflow).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty block_cursor_pos
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `block_cursor_pos` computes the block-cursor position (spacer-tail
  one-column-back saturating, else `x`) and the wide flag (wide/spacer-tail →
  true) — faithful to upstream's block-cursor uniforms;
- the tests pass (the `Wide` matrix incl. the spacer-tail back-step and the
  saturating edge), and the existing tests still pass;
- the uniform upload, the block-only gating, and the Metal upload stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the position is wrong (the back-step on the wrong
`Wide` kind, missing saturation, the wide flag inverted), or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the mapping is faithful: only `Wide::SpacerTail` backs
`x` up with `saturating_sub(1)` (matching upstream's `-|`), while
`Narrow`/`SpacerHead`/`Wide` keep `x`; and `cursor_wide` is correct (`Wide` and
`SpacerTail` → true, `Narrow` and `SpacerHead` → false). It agreed that
returning `([u16; 2], bool)` is a clean adaptation of the two uniform fields,
that passing `x`/`y`/`wide` directly is the right scoped shape, and that
computing it unconditionally while leaving the block-only gating and the uniform
upload deferred is consistent with the prior color-computation experiments. It
judged the tests sufficient (all `Wide` cases plus the saturating edge).

Review artifacts:

- Prompt: `logs/codex-review/20260603-210959-513065-prompt.md` (design)
- Result: `logs/codex-review/20260603-210959-513065-last-message.md` (design)
