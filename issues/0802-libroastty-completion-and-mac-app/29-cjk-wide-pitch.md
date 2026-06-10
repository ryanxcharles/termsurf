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

_(to be added after the run.)_

## Conclusion

_(to be added after the run.)_
