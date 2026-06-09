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

# Experiment 21: Phase C — enable font-fallback discovery (CJK + emoji)

## Description

Exp 20 found CJK (`日本語`) and emoji (`🎉`) render as `?` replacement chars.
Root cause (confirmed in source): `build_live_renderer` builds the `SharedGrid`
from a `CodepointResolver` over a collection of **only Menlo**, and
`CodepointResolver::new` defaults `discover_enabled: false`
(`codepoint_resolver.rs:103`) — so for a codepoint no loaded face covers, the
resolver never runs the CoreText **discovery fallback** (`discover_faces`, the
analog of upstream's `discoverFallback` in `CodepointResolver.zig`). Menlo has
no CJK/emoji glyphs, and with discovery off there is no fallback, so those
codepoints resolve to the replacement glyph.

## Approach

Enable the discovery-based fallback on the live renderer's resolver, so
codepoints Menlo doesn't cover are satisfied by a discovered system face (e.g. a
CJK face, Apple Color Emoji).

1. **`build_live_renderer`** (`lib.rs`): after constructing the
   `CodepointResolver`, call `resolver.set_discover_enabled(true)` before
   `SharedGrid::new`. (The resolver already exposes this; the discovery
   machinery — `discover_faces` / descriptor cache — already exists.)
   - Build the resolver as a `let mut`, enable discovery, then pass to
     `SharedGrid::new`.
2. **Verify the color-emoji path**: emoji are color glyphs (Apple Color Emoji,
   `sbix`/`CBDT`), which must rasterize into the **color** atlas
   (`Format::Bgra`) the compositor already samples; CJK are monochrome → the
   grayscale atlas. If discovery alone makes CJK render but emoji still fail,
   that pins a separate color-glyph gap (documented, not necessarily fixed
   here).
3. **Out of scope (noted follow-ups):** sprite metrics (box-drawing/powerline,
   `set_sprite_metrics`) — the smoke test didn't probe it; and
   `Collection::set_point_size` (the CJK ideographic-width fine-tune
   `SizeAdjustment::IcWidth`) is never called by the live renderer
   (pre-existing) — CJK renders at the inherited size without it. Neither is
   part of this experiment's Pass.

This touches **only `libroastty`** (`build_live_renderer`). No app changes.

## Verification

1. **`cargo test -p roastty`** (full) green — `build_live_renderer` isn't
   exercised headlessly (it needs a Metal device + nsview), so this is mainly a
   no-regression check; add a unit test asserting the resolver has discovery
   enabled if cheaply feasible.
2. **App launch (the proof):** rebuild + launch the **unicode probe** from Exp
   20 (`printf` of `日本語 🎉 café` via the `ZDOTDIR/.zshrc` drive method),
   capture (full-screen + `crop.swift`, window by `list-windows.swift`). **CJK
   and emoji now render as glyphs** (not `?`); `café` still correct. Compare to
   the Exp-20 `e20-unicode.png` (before). Kill the app + descendant tree (0
   dangling); shots out-of-repo.
3. If emoji render in color, note it; if CJK works but emoji don't, record the
   color-glyph gap.

**Pass** = discovery is enabled, the suite is green, and the launched app
renders CJK **and** emoji as real glyphs (not `?`).

**Partial** = CJK renders but emoji don't (color-glyph path gap) — documented as
the next gap, with CJK fixed.

**Fail** = discovery doesn't resolve the fallback faces from this harness
(documented — e.g. the discovery path needs more than the flag).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** It traced the full discovery chain and **ran
the discovery tests (41 pass**, including live CoreText
`discovery_fallback_finds_emoji` / `discovery_fallback_resolves_cjk`): (1)
`set_discover_enabled(true)` alone suffices — `get_index` gates discovery solely
on the flag (`codepoint_resolver.rs:267`), and the backend is real
(`CTFontCreateForString` rejecting `LastResort`; a ranked `CTFontCollection`),
not a stub; (2) the **color-emoji path is wired** — `render_glyph` routes
`Presentation::Emoji` → `atlas_color` (BGRA, Display-P3 premultiplied), which
`build_live_renderer` already seeds the compositor from; (3) **no Retina size
bug** — the discovered face inherits the primary's `font_size*scale` via
`CTFontCreateForString`, emoji are cell-cover-constrained; (4) **safe globally**
— Menlo-covered cps return before the discovery branch (Latin path untouched),
and discovery is cached per-cp (no unbounded growth / per-glyph query). One
Optional + one Nit, folded in:

- **Optional — `set_point_size` is never called**, so the CJK ideographic-width
  fine-tune (`SizeAdjustment::IcWidth`) isn't applied; CJK still renders at the
  inherited Retina-correct size. Pre-existing for the whole live renderer;
  **noted as a known fidelity follow-up** (not a Pass criterion) — the result
  reviewer should eyeball CJK glyph size.
- **Nit — line cite** `:104`→`:103`. Fixed.

## Result

**Result:** Pass — enabling discovery fixed the Exp-20 gap: the live app now
renders **CJK and emoji** instead of `?`.

### Change (only `libroastty`)

`build_live_renderer` (`lib.rs`) builds the resolver as `let mut`, calls
`resolver.set_discover_enabled(true)`, then passes it to `SharedGrid::new` — a
3-line change.

### Evidence (live launch, out-of-repo capture, app + descendant tree killed — 0 dangling)

Same unicode probe as Exp 20 (`printf '日本語 🎉 café\n'` via `ZDOTDIR/.zshrc`):

- **`日本語` renders as real CJK glyphs** (was `?` in Exp-20 `e20-unicode.png`).
- **`🎉` renders in COLOR** — the party-popper emoji, proving the color-glyph
  path (`Presentation::Emoji` → BGRA color atlas) works end to end.
- `café` (Latin + combining accent) still correct; ASCII unaffected.

(Known fidelity follow-up, per the design review: the CJK glyphs look slightly
**tight** — the `Collection::set_point_size` ideographic-width fine-tune
(`SizeAdjustment::IcWidth`) is never applied by the live renderer, so wide-cell
advance isn't fully tuned. The glyphs render correctly at the Retina-correct
inherited size; the exact wide-cell pitch is a separate, pre-existing refinement
— not part of this Pass.)

### Verification

- **Full `cargo test -p roastty`:** lib 4403 + abi_harness 1, **0 failures** —
  the change isn't exercised headlessly (needs a Metal device + nsview), so this
  is a no-regression check; the discovery machinery itself is covered by the
  existing 41 discovery tests (the design review ran them).

## Conclusion

Font fallback now works in the live app: any codepoint Menlo doesn't cover (CJK,
emoji, and other scripts) is satisfied by a CoreText-discovered system face,
with color emoji routed through the BGRA atlas. One of the two Exp-20 gaps is
closed. **Next: Exp 22 — the `clear` gap** (post-clear content + prompt don't
render). Noted refinements: the CJK ideographic-width fine-tune
(`set_point_size`) and sprite/box-drawing glyphs.

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED (no findings).** It verified the diff is exactly
the described 3-line resolver change (no app/collateral edits, no "ghostty"
literal), **ran the full suite** (`4403 passed; 0 failed` + abi_harness
`1 passed` — matching the recorded count), and confirmed from the PNGs:
`e20-unicode.png` (before) = `??? WIDE ? café`; `e21-unicode.png` (after) =
`日本語 WIDE 🎉 café` with real distinguishable Japanese glyphs + a **color**
party-popper emoji (not a black box) + correct `café`. It judged
Pass-over-Partial defensible (CJK glyphs fully formed/readable at the inherited
Retina size; only the wide-cell pitch is the disclosed `set_point_size`
follow-up) and the no-regression framing honest (discovery is flag-gated with an
early return for Menlo-covered codepoints + per-codepoint caching, so the
global-enable risk is bounded).
