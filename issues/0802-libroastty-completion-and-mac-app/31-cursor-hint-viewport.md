+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 31: Phase C — viewport-gate the cursor run-shaping hint

## Description

Exp 24 fixed the cursor **draw** to be viewport-aware (no stray block in
scrollback), but the Exp-24 review noted the **run-segmentation hint** in
`page_list.rs::shape_run_options` still uses the active-vs-viewport mismatch:
`let cursor_x = cursor.and_then(|(cx, cy)| (cy == y).then_some(cx))` compares
the **active** cursor row `cy` against the **viewport** row index `y`. When
scrolled, it places the run-shaping break (`cursor_x`) on a _history_ row. This
has **no cursor-draw effect** (the visible block was fixed in Exp 24) — it only
segments the shaper's runs to isolate the cursor cell (`font/run.rs:395`). It is
**segmentation-only**, but **not strictly harmless**: breaking a run at a column
disables ligature/contextual shaping across that boundary, so a stray hint on a
scrolled history row could break a ligature there (with a ligature font) — a
marginal but real effect that strengthens the case for this fix.

## Approach

Apply the same viewport gating as the Exp-24 cursor draw, reusing the
**existing** `cursor_viewport_row` (`page_list.rs:2373`, added in Exp 24):
compute the cursor's **viewport** row once before the loop and emit `cursor_x`
only there (or nowhere when the cursor is off-viewport):

```rust
let cursor_viewport = cursor.and_then(|(cx, cy)| self.cursor_viewport_row(cy).map(|vy| (cx, vy)));
// in the loop:
let cursor_x = cursor_viewport.and_then(|(cx, vy)| (vy == y).then_some(cx));
```

Unscrolled (viewport == active), `cursor_viewport_row(cy) == cy`, so the hint
lands on the same row as before — no behavior change. Scrolled, it lands on the
cursor's actual viewport row, or `None` when the cursor is off-viewport. **Only
`libroastty`** (`page_list.rs`, one site). No app change; the last faithfulness
loose-end from Exp 24.

## Verification

1. **Headless regression test** (the hint is a model accessor — fully headless,
   no screen needed): feed text + a cursor; **unscrolled** → exactly one row's
   `RunOptions.cursor_x == Some(cx)` at the cursor row (unchanged); **scrolled
   into scrollback** (fill past the screen, scroll up) → **no** row carries
   `cursor_x` (the active cursor is off-viewport), where pre-fix a history row
   got a stray `cursor_x`. Fails pre-fix, passes after. `cargo test -p roastty`
   (full) green — the existing `shape_run_options` cursor tests (`:20124`+)
   still pass (unscrolled unchanged).
2. **No live confirmation needed** — `cursor_x` has no cursor-draw effect
   (segmentation-only; the visible block was fixed in Exp 24); the only effect
   (marginal ligature-shaping on scrolled rows) is precisely what the headless
   model assertion (`cursor_x` value per row) proves corrected.
3. Faithful to upstream's viewport-relative cursor (the same `cursor.viewport`
   gating Exp 24 cited).

**Pass** = the hint is viewport-gated (reusing `cursor_viewport_row`), the
headless test (unscrolled-emits / scrolled-suppresses) passes, and the suite is
green. (No Partial-pending-live — the effect is segmentation-only, fully
provable by the model assertion, so it completes headless.)

**Partial** = unscrolled is correct + tested but the scrolled gating needs more
(unlikely — reuses the Exp-24 accessor).

**Fail** = `cursor_viewport_row` can't be reused here (documented).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** Confirmed: reusing `cursor_viewport_row` is
correct (it's `y`-independent, so computing it **once** before the loop is
behavior-identical + avoids an O(rows²) rescan; `shape_run_options` already
reads the viewport (Exp 23) so gating the cursor the same way is the consistent
fix); **unscrolled = no change** (viewport row `cy` pins same as active row `cy`
→ `Some(cy)`, existing cursor tests at page_list.rs:20124 still pass);
**`cursor_x` does not draw** (its only consumer is `run.rs:395`,
segmentation-only); borrow clean (owned Copy). One Optional folded in: the hint
is **not strictly harmless** — run segmentation disables ligature/contextual
shaping across the break, so a stray hint on a scrolled row could break a
ligature there (marginal but real), which **strengthens** the fix; wording
softened from "harmless/no visible effect" accordingly. The headless
unscrolled-emits/scrolled-suppresses test (fails pre-fix) + suite-green gate are
sound.

## Result

**Result:** Pass — the cursor run-shaping hint is now viewport-gated (fully
headless; no live needed).

### Change (only `libroastty`)

`page_list.rs::shape_run_options` computes
`cursor_viewport = cursor.and_then(|(cx, cy)| self.cursor_viewport_row(cy).map(|vy| (cx, vy)))`
**once** before the row loop (reusing the Exp-24 `cursor_viewport_row`), and the
per-row hint becomes
`cursor_x = cursor_viewport.and_then(|(cx, vy)| (vy == y).then_some(cx))` — so
the run-shaping break sits on the cursor's **viewport** row, or nowhere when the
cursor is scrolled off-viewport (no stray hint on a history row).

### Verification

- **Headless regression test** `cursor_shaping_hint_gated_by_viewport`
  (`terminal.rs`): unscrolled → exactly one row carries `cursor_x` at the cursor
  row (unchanged); scrolled into history → **no** row carries `cursor_x`. Fails
  pre-fix (a history row got a stray hint), passes after.
- **Full `cargo test -p roastty`:** lib **4414 passed**, 0 failures — all
  existing `shape_run_options` cursor/selection/snapshot tests still pass
  (unscrolled is identical).
- **No live confirmation needed** — segmentation-only (no cursor-draw effect;
  the visible block was Exp 24); the only effect, a marginal ligature-shaping
  break on a scrolled row, is exactly what the model assertion proves corrected.
  This refinement therefore **completes fully while the screen is locked**.

## Conclusion

The cursor handling is now fully viewport-consistent — both the draw (Exp 24)
and the run-shaping hint (here) gate on the cursor's viewport position. This
closes the Exp-24 loose end. A clean, fully-headless Pass — useful progress
while the live-confirmation items (Exp 29 CJK, Exp 30 shift-click) + closing
remain blocked on the locked screen.

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** Verified: the test is **load-bearing**
(pre-fix `cy=5` after 40 lines in a 6-row screen compared against viewport row
index `y`, so a scrolled-into-history row 5 got a stray `Some(cx)`; post-fix
`cursor_viewport_row` returns `None` off-viewport, so `all(cursor_x.is_none())`
genuinely fails pre-fix; the unscrolled `len()==1` at `cy` is non-vacuous); **no
regression** (full lib **4414 passed, 0 failed**; existing
`shape_run_options_assembles_rows` still asserts unscrolled
`row0.cursor_x==Some(1)`/`row1==None` — identical since `cursor_viewport_row`
returns the same row unscrolled); the **page_list.rs diff is exactly**
compute-once + gated `cursor_x`; **Pass is honest/headless** (`cursor_x` feeds
only run segmentation, not the cursor block — Exp 24); hygiene clean
(`fmt --check` 0, no new "ghostty" literals). Optional: the working tree also
carried an **incidental `cargo fmt` re-indent in `lib.rs`** (an Exp-30 fmt
drift) — fmt-mandated, kept per the no-revert-fmt rule.
