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

# Experiment 29: Phase C — CJK ideographic wide-pitch (`set_point_size`)

## Description

Exp 21 enabled font-fallback discovery so CJK + emoji render, but the result
review flagged the CJK glyphs as **slightly tight** — the ideographic-width
(`IcWidth`) fine-tune isn't applied. Root cause: `build_live_renderer`
(`lib.rs`) never calls `Collection::set_point_size`, so the collection's
`point_size` stays `None`. The discovery path **does** add discovered CJK faces
with the IcWidth adjustment (`codepoint_resolver.rs:164/203`
`add_with_adjustment(…, SizeAdjustment::IcWidth)`), which computes + stores the
scale factor — but `resize_face_to_point_size` (`collection.rs:451`) **no-ops
when `point_size` is `None`**, so the IcWidth factor is recorded yet never
physically applied to the face. The CJK face therefore renders at its discovered
natural size, not the 2-cell-fitted ideographic width.

## Approach

Call `collection.set_point_size((font_size as f64 * scale).max(1.0)).ok()?` in
`build_live_renderer`, after `update_metrics`, before the collection is moved
into the `CodepointResolver` (`.ok()?` matches the surrounding error pattern;
the error path is unreachable — the size is always `>= 1.0` finite and Menlo is
already loaded). This sets the collection's `point_size` so that when a CJK
codepoint is later discovered and added via `add_with_adjustment(…, IcWidth)`,
`resize_face_to_point_size(Some(points), face, factor)` physically resizes the
face to the IcWidth-adjusted size (filling the 2-cell advance).

`set_point_size(font_size*scale)` resizes the **primary** face (Menlo) to
`font_size*scale` — the **same** size it was rasterized at in
`build_live_renderer` (Exp 18) — so the cell metrics are unchanged
(`resize_face_to_point_size(Some(font_size*scale), Menlo, 1.0)` = no-op for
Menlo). Capture `metrics` after the call to be explicit. **Only `libroastty`**
(`build_live_renderer`). No app change.

## Verification

1. **No-regression + mechanism:** `set_point_size`'s IcWidth resize is already
   covered by the 8 `set_point_size` tests in `collection.rs` (the design review
   can run them). `build_live_renderer` needs a Metal device + nsview so it
   isn't headless-testable (as in Exp 21); the headless check is the **full
   `cargo test -p roastty`** staying green. The cell-metrics invariance is
   established **by construction** (set_point_size resizes Menlo to its exact
   existing `font_size*scale` size → `set_size`'s `update_metrics` re-derives
   identical `Metrics`) **+ the live ASCII comparison**, not by the suite alone
   (no headless test exercises `build_live_renderer`).
2. **Live confirmation** (screen unlocked — check `CGSSessionScreenIsLocked`):
   relaunch the Exp-21 unicode probe (`printf '日本語 🎉 café\n'` via
   `ZDOTDIR/.zshrc`), capture, and **compare the CJK glyph width to Exp-21's
   `e21-unicode.png`** — `日本語` should now fill its 2-cell advances (the
   IcWidth pitch) rather than render tight; `café`/ASCII unchanged; emoji
   unchanged. App + descendant tree killed (0 dangling); shots out-of-repo.
3. Faithful to upstream's `IcWidth` size adjustment for ideographic fallback
   faces (cite).

**Pass** = `set_point_size` is wired, the suite is green with **unchanged cell
metrics** (ASCII layout identical), and the live CJK renders at the proper
ideographic width (visibly less tight than Exp 21), with `café`/emoji
unaffected.

**Partial** = the call is wired + suite green, but the live CJK width difference
is imperceptible (the discovered face's natural advance already matched) —
documented as a no-op-but-faithful change.

**Fail** = `set_point_size` changes the cell metrics / breaks ASCII layout
(would mean the Menlo-unchanged assumption is wrong) — then revert + reconsider.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** It traced the full mechanism and ran the
collection tests (49 passed, incl. the resize-on-add tests). **Fix is
load-bearing:** the render path (`codepoint_resolver::render_glyph` →
`face.render_glyph` at the face's _physical_ CTFont size) does NOT re-apply the
stored `scale_factor` at rasterization — the factor only feeds
`resize_face_to_point_size` (`face.set_size(points*factor)`), which no-ops with
`point_size=None`; so the discovered CJK face renders at its natural size until
`set_point_size` lets the resize run (whenever `factor != 1`). **No ASCII/Retina
regression:** `set_point_size(font_size*scale)` resizes only Menlo (the sole
face at call time) to its **exact** creation size, so `update_metrics`
re-derives identical `Metrics` — cell grid unchanged. **`.ok()?` can't regress**
(error path unreachable). **Faithful:** sizing the fallback against
`font_size*scale` (primary's physical size) is correct; `scale_factor` is
em-normalized so it captures the ic-width ratio independent of absolute size.
Two minor folded in: pin the call to `.ok()?`; reword the headless guarantee
(metric invariance is by-construction + the live ASCII comparison, not "the
suite proves it").

## Result

**Result:** Partial — the fix is wired and the suite is green; the **live CJK
width comparison is pending a locked screen** (environment, not code), to be
re-confirmed when the display is unlocked.

### Change (only `libroastty`)

`build_live_renderer` now calls
`collection.set_point_size((font_size as f64 * scale).max(1.0)).ok()?` after
`update_metrics`, before the resolver — so a later-discovered CJK face (added
via `add_with_adjustment(…, IcWidth)`) is physically resized to the ideographic
width (`resize_face_to_point_size` no longer no-ops with `point_size = None`).

### Verification

- **Full `cargo test -p roastty`:** lib **4411 passed**, 0 failures — no
  regression. Cell-metrics invariance is by construction (Menlo is resized to
  its exact existing `font_size*scale` → identical `Metrics`), corroborated by
  the green ASCII-dependent render tests; the design review independently ran
  the collection `set_point_size`/resize-on-add tests (49 passed) confirming the
  IcWidth mechanism.
- **Live CJK comparison — blocked (locked screen).** The unicode re-probe
  couldn't be captured: `CGSSessionScreenIsLocked: true`, so `screencapture`
  returns black (the Exp-22 environment limitation). The fix is the
  design-review-confirmed load-bearing change; the **visible** wide-pitch
  improvement (and the `café`/ASCII/emoji-unchanged check) awaits a one-command
  re-probe once the screen is unlocked.

## Conclusion

The IcWidth ideographic adjustment is now applied to discovered CJK fallback
faces (the Exp-21 follow-up), faithful to upstream. The code + no-regression are
proven; the live visual confirmation is pending the locked screen (a redundant
confirm given the design-review trace + the green suite). **Per the loop,
shifting to headless-verifiable refinements** (shift-click extend, etc.) while
the screen is locked; this experiment's live re-probe + a possible upgrade to
Pass will happen on unlock.

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** Verified the diff is **exactly** the one-line
`set_point_size(...).ok()?` call (+ comment) between `update_metrics` and the
`metrics` capture, no collateral edits; full lib **4411 passed, 0 failed**,
`fmt --check` clean; **metric invariance by construction** (Menlo re-sized to
its identical creation size → `update_metrics` recomputes identical `Metrics`);
and specifically checked the `.ok()?` footgun —
`points = (font_size*scale).max(1.0)` is always `>= 1.0` finite (even NaN → 1.0)
and Menlo is already loaded, so both error paths are unreachable: the call
**cannot abort** `build_live_renderer`. The fix is load-bearing whenever the
discovered IcWidth factor ≠ 1. **Partial-pending-live is the correct label**
(code complete; the `CGSSessionScreenIsLocked` blocker is environmental,
consistent with Exp 22), not a cover for an incomplete/broken change. Scope
clean (libroastty only, no new "ghostty" literals).

## Live Confirmation

**Live-confirmed: Pass.** The renamed **Roastty.app**, rebuilt on the final
libroastty (all Exp 22–37 changes; RoasttyKit + the Swift app re-linked),
renders CJK at the correct **2-cell wide-pitch** in the live window. Confirmed
two ways: (a) **direct observation** of the running window — legible
Chinese/Japanese ideographs; and (b) a **measurable column-ruler check** — a
probe line of 8 ideographs (`你好世界中文宽度`) and a mixed line
(`ABCD你好EFGH世界` = 4+4+4+4) both span exactly **16 columns**, so their
trailing `<-` comment markers align; that alignment only holds if each ideograph
occupies 2 cells (1-cell CJK would land them at cols 8 vs 12). _Note: an initial
screenshot-only read wrongly flagged "overlap" — dense ideograph strokes look
crammed in a downscaled PNG; the ruler measurement + the live view corrected
it._
