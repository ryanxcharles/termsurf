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

# Experiment 340: the shaper's non-LTR run sort

## Description

`Face::shape_codepoints` (Experiments 338–339) emits cells in CoreText's run
order. CoreText, despite an enforced LTR embedding level, can still emit runs
that are **non-monotonic** or **right-to-left** — leaving the cell buffer out of
grid order. Upstream guards against this: it checks each `CTRun`'s status, and
if any run is non-monotonic or RTL, it sorts the whole cell buffer by `x`
afterward. This experiment ports that guard — the run-status check and the final
non-LTR sort.

## Upstream behavior (`shaper/coretext.zig` `Shaper.shape`)

```zig
var non_ltr: bool = false;
// …
const runs = line.getGlyphRuns();
for (0..runs.getCount()) |run_i| {
    const ctrun = runs.getValueAtIndex(macos.text.Run, run_i);
    const status = ctrun.getStatus();
    if (status.non_monotonic or status.right_to_left) non_ltr = true;
    // …emit cells…
}

// If our buffer contains some non-ltr sections we need to sort it :/
if (non_ltr) {
    @branchHint(.cold);   // EXCEPTIONALLY rare
    std.mem.sort(font.shape.Cell, self.cell_buf.items, {}, struct {
        fn lt(_: void, a: font.shape.Cell, b: font.shape.Cell) bool {
            return a.x < b.x;
        }
    }.lt);
}
```

`CTRunGetStatus` returns a bitfield (`CTRunStatus`) with
`kCTRunStatusRightToLeft` (`1<<0`) and `kCTRunStatusNonMonotonic` (`1<<1`). If
any run in the line carries either flag, the per-glyph emission order no longer
matches the grid, so the buffer is sorted by `Cell.x` (a stable post-condition:
cells are grid-ordered). This path is "EXCEPTIONALLY rare" — only
complex-shaping scripts trigger it.

## Rust mapping (`roastty/src/font/face/coretext.rs`)

- In `shape_codepoints`, declare `let mut non_ltr = false;` before the run loop.
- For each run, read its status and OR the flags into `non_ltr`:
  ```rust
  let status = unsafe { run.status() };
  if status.intersects(CTRunStatus::RightToLeft | CTRunStatus::NonMonotonic) {
      non_ltr = true;
  }
  ```
  (`CTRunStatus::intersects` is `true` when **any** of the OR'd bits is set —
  the `bitflags`-style semantics match upstream's
  `non_monotonic or right_to_left`. `contains(A | B)` would require **both**
  bits, diverging from upstream, so `intersects` is the correct method.)
- After the run loop, before returning, sort if needed:
  ```rust
  if non_ltr {
      cells.sort_by(|a, b| a.x.cmp(&b.x));
  }
  ```
  `sort_by` is stable, so cells sharing an `x` keep their CoreText order
  (upstream uses `std.mem.sort`, which is unstable; for equal keys either order
  is grid- valid, and stable is a strict-superset guarantee — see faithfulness
  notes).

## Scope / faithfulness notes

- **Ported**: the run-status check (`CTRunGetStatus` → `RightToLeft` /
  `NonMonotonic`) and the conditional final sort of the cell buffer by `x` — the
  non-LTR ordering guard of `Shaper.shape`.
- **Faithful nuance**: upstream sorts with `std.mem.sort` (unstable); roastty
  uses `sort_by` (stable). Both produce a buffer sorted ascending by `x`; on
  equal keys upstream's order is unspecified, so the stable result is one of the
  permitted orderings — no behavioral divergence in the contract (cells
  grid-ordered by `x`). The sort key is `Cell.x`, matching upstream exactly. (As
  in Exp 338–339, `Cell.x` is still the UTF-16 string index pending the deferred
  cluster→cell mapping; the sort operates on whatever `x` holds, faithfully.)
- **Deferred** (unchanged): the cluster→cell mapping with the ligature
  heuristic, the special-font fast path, the `Shaper` struct + `RunIterator`,
  the variation-axis score, and variations application.
- No C ABI/header/ABI-inventory change (internal Rust; `CTRun` is already a
  bound, enabled type).

## Changes

1. `roastty/src/font/face/coretext.rs`: import `CTRunStatus`; in
   `shape_codepoints`, track `non_ltr` across the run loop (from each run's
   status) and sort `cells` by `x` afterward when set.
2. Tests (in `coretext.rs`):
   - `shape_ltr_stays_sorted`: Menlo `"ABC"` (pure LTR) shapes to cells whose
     `x` are non-decreasing — the sort, whether or not it runs, leaves a
     grid-ordered buffer, and the LTR no-op path does not reorder the 1:1 cells
     (`x` = `0, 1, 2`). Deterministic.
   - `shape_rtl_grid_ordered`: a Hebrew string
     (`"\u{05E9}\u{05DC}\u{05D5}\u{05DD}"`, "שלום") shapes to cells whose `x`
     are non-decreasing. On a host whose CoreText shapes Hebrew RTL, this
     exercises the `non_ltr` branch — without the sort the cells would emerge in
     visual (reversed) order; the sort restores ascending `x`. The asserted
     post-condition (sorted by `x`) holds regardless of which fallback font the
     host uses, so the test is robust.
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

- `shape_codepoints` reads each run's status, sets `non_ltr` on
  `RightToLeft`/`NonMonotonic`, and sorts the cell buffer by `x` when set —
  faithful to upstream;
- the ltr-stays-sorted and rtl-grid-ordered tests pass, and the existing shaping
  tests still pass;
- the cluster→cell mapping, the special-font path, the `Shaper`/`RunIterator`,
  the variation-axis score, and variations stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the host CoreText never sets a non-LTR status
for the Hebrew test (e.g. an unusual fallback), so the `non_ltr` branch is not
exercised at runtime — the sort logic and post-condition are still verified.

The experiment **fails** if the status check or the sort diverges from upstream
(wrong flags, wrong key, sorting unconditionally), or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and found **one Required
finding**, now fixed:

- **Required (fixed):** the draft used
  `status.contains(CTRunStatus::RightToLeft | CTRunStatus::NonMonotonic)`, but
  `bitflags`' `contains(A | B)` is true only when **both** bits are set — the
  opposite of upstream's `non_monotonic or right_to_left` (any). A run carrying
  only one flag would have failed to set `non_ltr`. Changed to
  `status.intersects(RightToLeft | NonMonotonic)`, whose any-bit semantics match
  upstream's `or`.

Codex confirmed the rest is sound: sorting once after the full run loop (not
per-run) matches upstream; `Cell.x` is the correct sort key for this slice;
stable (`sort_by`) vs unstable (`std.mem.sort`) is not a correctness divergence
for the ascending-by-`x` contract; and the Hebrew test is acceptable as a
runtime-exercised branch test given the documented "partial if the host never
sets a non-LTR status" framing.

Review artifacts:

- Prompt: `logs/codex-review/20260603-131031-976423-prompt.md` (design)
- Result: `logs/codex-review/20260603-131031-976423-last-message.md` (design)
