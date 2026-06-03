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

# Experiment 339: the shaper's glyph offsets

## Description

`Face::shape_codepoints` (Experiment 338) extracts each glyph and its source
string index, but leaves `x_offset`/`y_offset` at `0`. Upstream computes these
from the glyph **positions** and accumulated **advances** — the rendering nudge
of each glyph relative to its pen position. This experiment ports that per-glyph
offset computation (the advance-based positioning core), so combining marks and
positioned glyphs carry their offsets. The cluster→cell `x` mapping (the
terminal-coupled part) stays deferred.

## Upstream behavior (`shaper/coretext.zig` `Shaper.shape`)

```zig
var run_offset: Offset = .{};   // pen x (sum of advance.width) across the line
for (line.getGlyphRuns()) |ctrun| {
    const positions = ctrun.getPositions(...);
    const advances  = ctrun.getAdvances(...);
    for (glyphs, advances, positions, indices) |glyph, advance, position, index| {
        // …cell_offset (cluster→cell) tracking… (deferred here)
        const x_offset = position.x - cell_offset.x;   // for a per-cell run,
                                                        // cell_offset.x == pen x
        cell_buf.append(.{
            .x = …cluster…,
            .x_offset = @intFromFloat(@round(x_offset)),
            .y_offset = @intFromFloat(@round(position.y)),
            .glyph_index = glyph,
        });
        run_offset.x += advance.width;   // advance the pen (applies to the next)
    }
}
```

`positions[k]` is the glyph's x/y relative to the line origin; `advances[k]` is
its pen advance. The pen position (`run_offset.x`) accumulates `advance.width`
across **all** runs in the line. For a run where each codepoint is its own cell,
`cell_offset.x` equals the pen at the glyph's cell start, so
`x_offset = position.x − pen`. For plain monospace text this is `0` (the glyph
sits exactly at the pen); combining/positioned glyphs get a non-zero nudge.
`y_offset = round(position.y)` (`0` for baseline glyphs, non-zero for marks).

## Rust mapping (`roastty/src/font/face/coretext.rs`)

- Read the run's positions (`CGPoint`) and advances (`CGSize`) alongside the
  glyphs/indices, via the `positions_ptr`/`positions(range, buf)` and
  `advances_ptr`/`advances(range, buf)` ptr-or-copy pattern (new
  `run_positions`/ `run_advances` helpers, mirroring `run_glyphs`).
- Track a line-wide `pen: f64` (the accumulated `advance.width` across runs).
  For each glyph `k`:
  ```rust
  cells.push(shape::Cell {
      x: indices[k].max(0) as u16,
      x_offset: (positions[k].x - pen).round() as i16,
      y_offset: positions[k].y.round() as i16,
      glyph_index: glyphs[k] as u32,
  });
  pen += advances[k].width;
  ```
  The `pen` accumulates across runs (declared before the run loop), matching
  upstream's line-wide `run_offset.x`.

## Scope / faithfulness notes

- **Ported**: the per-glyph `x_offset`/`y_offset` — `round(position.x − pen)`
  and `round(position.y)` with the line-wide pen accumulation — the
  advance-based positioning core of `Shaper.shape`.
- **Faithful simplification (still deferred)**: the **cluster→cell mapping**
  (the `cell_offset` reset logic with the ligature heuristic that maps glyphs to
  terminal cells, and sets `Cell.x` to the cluster). This slice keeps `x` = the
  UTF-16 string index and computes `x_offset` against the running pen — which
  equals the upstream `cell_offset.x` for a per-codepoint-cell run (the common
  case), so the offsets match for non-ligature text. The full cell mapping needs
  the terminal grid and is a later experiment.
- **Deferred**: the special-font path, RTL/non-monotonic sorting, the `Shaper`
  struct + `RunIterator`, the variation-axis score, and variations application.
- No C ABI/header/ABI-inventory change (internal Rust; the needed objc2 features
  are already enabled).

## Changes

1. `roastty/src/font/face/coretext.rs`: add `run_positions`/`run_advances`
   helpers; read positions/advances in `shape_codepoints`; compute the offsets
   with the line-wide pen.
2. Tests (in `coretext.rs`):
   - `shape_plain_offsets_zero`: Menlo `"ABC"` shapes to cells whose `x_offset`
     and `y_offset` are all `0` (plain monospace glyphs sit exactly at the pen,
     on the baseline) — proving the position/advance reading and the
     `position − pen` formula are wired correctly (a wrong formula or mis-read
     would not yield uniform zeros at the right glyphs).
   - `shape_advances_monotonic`: across the run, the implied pen positions are
     non-decreasing — i.e. the cells' `x` (string indices) increase and the run
     produces one cell per codepoint (the advance accumulation does not corrupt
     the per-glyph output). The existing `shape_ascii_monospace`/`shape_single`/
     `shape_empty` tests still pass (now also exercising the offset path).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty shape
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `shape_codepoints` computes `x_offset = round(position.x − pen)` and
  `y_offset = round(position.y)` with the line-wide pen accumulation, faithful
  to upstream;
- the plain-offsets-zero and advances tests pass, and the existing shaping tests
  still pass;
- the cluster→cell mapping, the special-font path, the `Shaper`/`RunIterator`,
  the variation-axis score, and variations stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the host font produces non-zero offsets for
plain ASCII (unexpected; the offset computation is still exercised).

The experiment **fails** if the offset formula or the pen accumulation diverges
from upstream, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It verified: `x_offset = round(position.x − pen)` matches upstream
in the per-codepoint, non-ligature case (upstream resets `cell_offset.x` to the
current `run_offset.x` at each normal cluster start, and `pen` is that same
line-wide `run_offset.x`); `y_offset = round(position.y)` is a direct match;
declaring `pen` **before** the `CTRun` loop is correct (`CTRun` positions are
line-origin-relative while advances describe pen movement through the whole
line, so resetting per run would be wrong); `pen += advances[k].width` **after**
emitting the cell matches upstream's "advance applies to the next glyph"; plain
Menlo ASCII should round to zero offsets (positions sit at the cumulative
advance on `y = 0`); and `round() as i16` is the intended local equivalent of
`@intFromFloat(@round(..))` for this bounded domain. It confirmed the
`run_positions`/`run_advances` ptr-or-copy helpers mirror the already-reviewed
glyph/index helpers and are sound under the same lifetime model. The only caveat
is the already-documented, properly-deferred cluster→cell mapping
(ligatures/reordering) — fine for this non-ligature offset core.

Review artifacts:

- Prompt: `logs/codex-review/20260603-130316-951763-prompt.md`
- Result: `logs/codex-review/20260603-130316-951763-last-message.md`

## Result

**Result:** Pass

The shaper now carries each glyph's positioning nudge.

- `roastty/src/font/face/coretext.rs`: `shape_codepoints` declares a line-wide
  `pen: f64 = 0.0` before the `CTRun` loop and, per glyph `k`, emits
  `x_offset: (positions[k].x - pen).round() as i16` and
  `y_offset: positions[k].y.round() as i16`, then accumulates
  `pen += advances[k].width` after pushing the cell — faithful to upstream's
  `position.x − run_offset.x` / `round(position.y)` with the advance applying to
  the next glyph. Two free helpers `run_positions` (→ `Vec<CGPoint>`) and
  `run_advances` (→ `Vec<CGSize>`) factor the ptr-or-copy reads, mirroring
  `run_glyphs`/`run_string_indices`.

Tests: `shape_plain_offsets_zero` (Menlo `"ABC"` → all `x_offset`/`y_offset`
`== 0`, proving the `position − pen` formula and the position/advance reads are
wired correctly), `shape_advances_monotonic` (`"xyz"` → 3 cells with
non-decreasing `x`).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2744 passed, 0 failed (+2, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The advance-based positioning core of `Shaper.shape` is ported: glyphs now carry
`x_offset`/`y_offset` computed from their CoreText positions against a line-wide
pen. Plain monospace ASCII rounds to zero offsets (glyphs sit exactly at the
pen, on the baseline), confirming the wiring; combining/positioned glyphs would
carry their nudge.

The remaining shaper work builds the orchestration around this core: the
**cluster→cell mapping** (the `cell_offset` reset with the ligature heuristic
that maps glyphs to terminal cells and sets `Cell.x` to the cluster); the
**special-font** fast path (codepoint == glyph); **RTL/non-monotonic** run
sorting; and the `Shaper` struct with its run state, caching, and the
**`RunIterator`** over terminal cells (which threads in the terminal
grid/render- state types). The deferred **variation-axis** `score()` refinement
and **variations** application also remain.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no Required findings**. It verified: `pen` is line-wide (declared before the
`CTRun` loop), matching upstream's `run_offset.x` across all runs;
`x_offset = round(position.x − pen)` faithfully matches upstream's
`position.x − cell_offset.x` for the per-codepoint-cell case where
`cell_offset.x == run_offset.x`; `y_offset = round(position.y)` is a direct
match; `pen += advances[k].width` after emitting the cell matches upstream's
"advance applies to the next glyph"; the `run_positions`/`run_advances`
ptr-or-copy helpers are sound under the same lifetime model as the glyph/index
helpers (`n` from `glyph_count`, fast-path slices copied while the run is alive,
fallback buffers sized before CoreText fills them); and the Menlo zero-offset
test validates the simple-case wiring while the monotonic test guards output
order. The cluster→cell mapping (ligatures/reordering) remains correctly
deferred.

Review artifacts:

- Result review: `logs/codex-review/20260603-130556-764352-last-message.md`
