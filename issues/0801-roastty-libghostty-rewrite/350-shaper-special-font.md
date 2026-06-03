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

# Experiment 350: the special-font fast path

## Description

Upstream's `Shaper.shape` has a fast path at the very top: when the run's font
is a **special** font (the sprite/box-drawing font, whose glyph ids _are_ the
codepoints), it skips CoreText shaping entirely and returns the codepoints as
cells. This experiment ports that transform — a pure function over the
`(codepoint, cluster)` input that emits one cell per real codepoint
(`glyph_index == codepoint`, `x == cluster`), skipping the `codepoint == 0`
padding. _Deciding_ a run is special (`run.font_index.special()`) belongs to the
`Shaper`/`Collection` wiring and stays deferred; this experiment is the
transform.

## Upstream behavior (`shaper/coretext.zig` `Shaper.shape`)

```zig
// Special fonts aren't shaped and their codepoint == glyph so we
// can just return the codepoints as-is.
if (run.font_index.special() != null) {
    self.cell_buf.clearRetainingCapacity();
    for (state.codepoints.items) |entry| {
        // null codepoints pad the UTF-16 list; they aren't part of the run.
        if (entry.codepoint == 0) continue;
        self.cell_buf.appendAssumeCapacity(.{
            .x = @intCast(entry.cluster),
            .glyph_index = @intCast(entry.codepoint),
        });
    }
    return self.cell_buf.items;
}
```

Each input codepoint becomes a cell whose `glyph_index` is the codepoint itself
and whose `x` is the codepoint's cluster; `x_offset`/`y_offset` stay `0`.
Entries with `codepoint == 0` are skipped — upstream uses them to pad the UTF-16
string it builds for CoreText (the surrogate low half), and a special font never
reaches CoreText, so they are not emitted.

## Rust mapping (`roastty/src/font/shape.rs`)

`roastty`'s shaping input (`&[shape::Codepoint]`) is the caller's un-padded
`(codepoint, cluster)` stream (the surrogate padding is an internal detail
`shape_run` adds only when building the CoreText string). So the transform
reduces to: emit a cell per entry with a non-zero codepoint.

```rust
/// Shape a run with a special (sprite) font, whose glyph ids are the codepoints
/// themselves — the fast path that skips CoreText shaping. Each input codepoint
/// becomes a cell (`glyph_index == codepoint`, `x == cluster`); `codepoint == 0`
/// entries are skipped (they only pad the UTF-16 string a real shaping pass
/// builds). Faithful port of upstream `Shaper.shape`'s special-font branch.
pub(crate) fn shape_special(run: &[Codepoint]) -> Vec<Cell> {
    run.iter()
        .filter(|cp| cp.codepoint != 0)
        .map(|cp| Cell {
            // A cluster is a terminal-cell column, always within `u16`. A checked
            // conversion mirrors upstream's `@intCast` (which panics on overflow)
            // rather than silently truncating.
            x: u16::try_from(cp.cluster).expect("a shaped cluster must fit Cell.x (u16)"),
            x_offset: 0,
            y_offset: 0,
            glyph_index: cp.codepoint,
        })
        .collect()
}
```

## Scope / faithfulness notes

- **Ported**: the special-font fast path of `Shaper.shape` — the codepoint→cell
  transform (`glyph_index == codepoint`, `x == cluster`) with the
  `codepoint == 0` skip.
- **Faithful**: skipping `codepoint == 0` matches upstream (it skips the UTF-16
  surrogate padding; roastty's un-padded input has no padding entries, but a
  real `U+0000` is likewise skipped, matching upstream's
  `entry.codepoint == 0`); the cell fields (`x` = cluster, `glyph_index` =
  codepoint, zero offsets) match.
- **Deferred** (unchanged): the _decision_ that a run uses a special font
  (`run.font_index.special()`), which the `Shaper`/`Collection` wiring makes
  when it resolves a run's font index; the `Shaper` struct + `RunIterator`. (The
  transform is consumed by tests now; the font module's `#![allow(dead_code)]`
  covers the not-yet-wired path, as with the other ported-ahead-of-consumer
  primitives.)
- No C ABI/header/ABI-inventory change (internal Rust, no CoreText).

## Changes

1. `roastty/src/font/shape.rs`: add `shape_special`.
2. Tests (in `shape.rs`):
   - `shape_special_codepoint_is_glyph`: `shape_special` over
     `[(0x2500, 0), (0x2502, 1), (0x256C, 2)]` (box-drawing scalars) yields
     three cells with `glyph_index == codepoint`, `x == cluster`, and zero
     offsets.
   - `shape_special_skips_zero`: `[(0, 0), ('A', 1)]` yields one cell — the
     `codepoint == 0` entry is skipped, leaving `glyph_index == 'A'`, `x == 1`.
   - `shape_special_high_plane`: a supplementary-plane sprite scalar (`0x1FB70`,
     cluster `0`) yields one cell with `glyph_index == 0x1FB70` (the codepoint
     survives in the `u32` `glyph_index`) — mirroring upstream's special-font
     surrogate-pair case.
   - `shape_special_empty`: an empty run yields an empty `Vec`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty shape_special
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `shape_special` emits one cell per non-zero-codepoint entry with
  `glyph_index == codepoint`, `x == cluster`, and zero offsets — faithful to
  upstream's special-font branch;
- the codepoint-is-glyph, skip-zero, and empty tests pass, and the existing
  tests still pass;
- the special-font _decision_, the `Shaper` struct, and the `RunIterator` stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the transform diverges from upstream (wrong cell
fields, not skipping `codepoint == 0`), or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **one Required
finding**, now fixed:

- **Required (fixed):** the draft used `cp.cluster as u16`, which silently
  truncates a cluster above `u16::MAX` — upstream's `@intCast(entry.cluster)` is
  checked. Changed to `u16::try_from(cp.cluster).expect(...)` (clusters are
  terminal columns, always within `u16`; the checked conversion mirrors
  upstream's panic-on-overflow rather than truncating).

Codex confirmed the rest: emitting one cell per non-zero un-padded `Codepoint`
is equivalent to upstream iterating the padded `state.codepoints.items` and
skipping `codepoint == 0` (same cells, same order); skipping a real `U+0000` is
faithful (upstream skips any `entry.codepoint == 0`);
`glyph_index = cp.codepoint` and the zero offsets match; and deferring the
`font_index.special()` _decision_ to the later `Shaper`/`Collection` wiring is a
clean split. Per Codex's non-blocking suggestion, a high-plane sprite-codepoint
test (`0x1FB70`) was added to mirror upstream's special-font padding case.

Review artifacts:

- Prompt: `logs/codex-review/20260603-143837-267604-prompt.md` (design)
- Result: `logs/codex-review/20260603-143837-267604-last-message.md` (design)

## Result

**Result:** Pass

The special-font fast path is ported.

- `roastty/src/font/shape.rs`: `shape_special(run: &[Codepoint]) -> Vec<Cell>`
  emits one cell per non-zero-codepoint entry — `glyph_index == codepoint`,
  `x == cluster` (via a checked `u16::try_from`), zero offsets — skipping
  `codepoint == 0` entries. A pure transform that skips CoreText shaping,
  faithful to upstream's special-font branch.

Tests: `shape_special_codepoint_is_glyph` (box-drawing
`0x2500`/`0x2502`/`0x256C` → `glyph_index == codepoint`, `x == cluster`, zero
offsets), `shape_special_skips_zero` (`[(0, 0), ('A', 1)]` → one cell, the NUL
skipped), `shape_special_high_plane` (`0x1FB70` → `glyph_index == 0x1FB70`),
`shape_special_empty`. All pass.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2775 passed, 0 failed (+4, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The special-font fast path — `Shaper.shape`'s top-of-function shortcut for
sprite fonts (codepoint == glyph) — is ported as a pure transform. The shaping
logic for both real CoreText shaping and special fonts is now in place.

The remaining shaper work is the orchestration that _chooses_ between them and
feeds them: the `Shaper` struct (run state, the cached feature dicts, the
`features_no_default` variant) and the `RunIterator` over terminal cells, which
resolves each run's font index from the `Collection` (calling
`font_index.special()` to pick this fast path) — the layer that threads
roastty's `terminal/` grid and `renderer/` cell/state types.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no Required findings**. It confirmed `shape_special` faithfully ports the
upstream special-font transform (preserves order, skips `codepoint == 0`, emits
`glyph_index = codepoint`, `x = cluster`, zero offsets), and that the un-padded
Rust input is equivalent because upstream's only padding-specific behavior is
the same `codepoint == 0` skip. It verified the design-gate fix
(`u16::try_from(cp.cluster).expect(...)` matches upstream's checked `@intCast`,
no silent truncation), that `glyph_index` stays `u32` so high-plane sprite
scalars survive, and that the change is isolated to `shape.rs` with the
`font_index.special()` decision and the `Shaper`/`RunIterator` wiring still
deferred. It ran `cargo test -p roastty shape_special` (4 passed).

Review artifacts:

- Result review: `logs/codex-review/20260603-144056-673798-last-message.md`
