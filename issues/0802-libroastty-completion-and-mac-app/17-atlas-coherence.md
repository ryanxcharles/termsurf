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

# Experiment 17: Phase C — atlas coherence (sample the grid's glyph atlas)

## Description

Exp 15's result review found, and Exp 16 left standing, the reason the live
present renders **backgrounds but no text**: the rebuild stage rasterizes glyphs
into the `SharedGrid`'s **own** atlases (`grid.atlas_grayscale` /
`grid.atlas_color`, written by `render_glyph`), but the present stage samples
the **separate** `presentation.grayscale_atlas` / `color_atlas` — which
`build_live_renderer` created as standalone **empty** `Atlas::new(512, …)`.
Glyphs go in one atlas; the compositor samples a different, empty one. So no
glyph pixels reach the screen.

The 801 `render_and_present_frame` API decoupled the two (it only ever asserted
`rebuilt_rows`/`width`, never glyph pixels), and the obvious one-liner — pass
`&grid.atlas_grayscale` as the presentation atlas — appears to conflict with the
`&mut grid` rebuild borrow. But it does **not**: in `rebuild_and_present_frame`
(`frame_rebuild.rs`), the rebuild (`run_rebuild_stages`, which holds
`&mut grid`) **completes and returns before** the present
(`present_metal_frame`). So after the rebuild, `grid` can be re-borrowed
**immutably** for the present.

## Approach

Make the present sample the grid's atlases — the same ones the rebuild just
rasterized into — and delete the standalone presentation atlases entirely.

1. **`FramePreparedPresentationInput`** (`frame_rebuild.rs`): remove the
   `grayscale_atlas` / `color_atlas` fields. The presentation no longer takes
   caller-supplied atlases.
2. **`rebuild_and_present_frame`**: after `run_rebuild_stages` returns (its
   `&mut grid` borrow ended), pass `&targets.grid.atlas_grayscale` /
   `&targets.grid.atlas_color` into `present_metal_frame`'s
   `FrameMetalPresentationInput`. These are disjoint from the other present
   inputs (`targets.uniforms`, `targets.contents`), so the borrows are sound.
3. **The compositor's construction-time atlases** (`MetalFrameCompositor::new` →
   `FrameState::new`): the review confirmed from source that `FrameState::sync`
   re-uploads the atlas **per frame** from the presentation input
   (`frame.rs::sync_if_modified`), and `sync_atlas_texture` (`texture.rs`)
   **reallocates + re-uploads the full atlas when it grows** past the seed size
   — so the construction atlas only seeds format/size, and passing the grid's
   atlas at present-time (step 2) is sufficient even as the grid atlas grows. To
   avoid any ambiguity, **seed the compositor at construction from
   `&shared_grid.atlas_grayscale` / `atlas_color`** as well (not a throwaway
   standalone `Atlas`).
4. **Callers** — drop the removed atlas args at **every**
   `FramePreparedPresentationInput` construction (the review enumerated them, so
   the suite doesn't break): `present_live` (`lib.rs`) and the **seven**
   `frame_renderer.rs` test sites (`render_and_present_frame_presents` + the
   `update_and_present_*` tests at lines ~611, 651, 670, 710, 754, 777).
   `SurfaceLiveRenderer` no longer owns `grayscale_atlas` / `color_atlas` (the
   grid owns them) — remove those fields; `build_live_renderer` seeds the
   compositor from the grid's atlases (step 3).
5. **The regression-guard test (Required correction from the review):**
   asserting "the grid atlas is non-empty" is **vacuous** — the rebuild
   rasterizes into the grid atlas in _both_ the buggy and fixed code; the bug
   was only ever which atlas the _present sampled_. The test must read back the
   **GPU side**: mirror `compositor.rs::compositor_draws_foreground_glyph`
   (which reads the rendered target via the `#[cfg(test)] target_bytes` readback
   and asserts a foreground pixel is non-background). Add a `frame_renderer.rs`
   test that presents a terminal with a glyph through `render_and_present_frame`
   and asserts a **foreground (text) pixel is non-background** in the
   compositor's target — which fails if the present samples an empty atlas. This
   needs `target_bytes` (currently a private `#[cfg(test)]` fn on
   `MetalFrameCompositor`) exposed to the `frame_renderer.rs` test module (e.g.
   `pub(crate)` + `#[cfg(test)]`, or a small readback accessor) — wire that.

## Verification

1. **`cargo test -p roastty`** (full) green, including the strengthened present
   test that now asserts glyph data is sampled (not just
   `rebuilt_rows`/`width`).
2. **App launch:** rebuild RoasttyKit + app, launch (the Exp-16 auto-start gives
   a live shell), and capture the window (full-screen `screencapture` + `sips`
   crop, per Exp 15 — the IOSurface layer defeats `-l`). **The shell prompt /
   typed text now renders** (not just the black background). Kill the spawned
   app + children (0 dangling PIDs); screenshot out-of-repo.
3. No regression in the offscreen golden/round-trip render tests.

**Pass** = the present samples the grid's rasterized atlas, the strengthened
test asserts glyph data, the full suite is green, and the launched app **renders
actual terminal text** (the shell prompt) into the NSView — the first real
terminal frame from libroastty.

**Partial** = glyph data is now sampled and the test proves it, but the live
window still doesn't show text for a further, separately-pinned reason (e.g. the
one-shot present fires before the shell emits the prompt — needs the Exp-18
continuous driver) — documented.

**Fail** = the atlases can't be made coherent without a deeper
`FrameState`/atlas-lifecycle change (documented as the real blocker + the next
slice).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: CHANGES REQUIRED → addressed.** It **confirmed the core
fix is sound** from source: (1) the borrow compiles — `run_rebuild_stages` takes
_reborrows_ (`&mut *targets.grid`) so `targets.grid` is intact and the `&mut`
borrow ends before the present, and `&targets.grid.atlas_*` coexists with the
disjoint `targets.uniforms`/`contents` shared borrows; (2) **sufficiency +
growth handled** — `FrameState::sync` re-uploads per frame from the presentation
atlas (`frame.rs:70-87`), and `sync_atlas_texture` reallocates + re-uploads the
full atlas on growth (`texture.rs:285-296`), so sampling the grid atlas works
even as it grows past 512; (3) the one-shot-present-timing Partial risk is
fairly flagged. Three findings, folded in above:

- **Required — the strengthened test was vacuous** (grid-atlas-non-empty passes
  in buggy + fixed code). **Fixed:** assert the **GPU readback** (foreground
  text pixel non-background), mirroring `compositor_draws_foreground_glyph`;
  expose `target_bytes` cross-module.
- **Optional — undercounted call sites:** the field removal breaks 6 more
  `update_and_present_*` test sites + `present_live`. **Fixed:** enumerated.
- **Nit — ambiguous compositor seed.** **Fixed:** seed it from
  `&shared_grid.atlas_*`.

## Result

_(to be added after the run.)_

## Conclusion

_(to be added after the run.)_
