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

**Result:** Partial — **atlas coherence is fixed and proven by a discriminating
GPU-readback test**: the present now samples the `SharedGrid`'s own rasterized
atlases, so glyphs reach the target. But implementing the test surfaced a
**second, separate gap** that still blocks live text: the live path never sets
up the **projection / screen-size uniforms**, so glyphs render off-screen until
that is wired (Exp 18).

### Changes (only `libroastty`)

- **`frame_rebuild.rs`** — removed `grayscale_atlas`/`color_atlas` from
  `FramePreparedPresentationInput`; `rebuild_and_present_frame` now passes
  `&targets.grid.atlas_grayscale`/`atlas_color` to the present (the `&mut grid`
  rebuild borrow has ended, so the immutable re-borrow is sound).
- **`lib.rs`** — `build_live_renderer` seeds the compositor from
  `&shared_grid.atlas_*` (not a standalone `Atlas`); `SurfaceLiveRenderer`
  dropped its `grayscale_atlas`/`color_atlas` fields; `present_live` drops the
  removed args.
- **Callers** — the 7 `frame_renderer.rs`
  `update_and_present_*`/`render_and_present_frame_presents` test sites + the 2
  `frame_rebuild.rs` test sites updated to the new (atlas-less) signature.
- **`compositor.rs`** — `target_bytes` made `pub(crate)` (`#[cfg(test)]`) for
  the cross-module readback test.

### The discriminating test (the review's Required correction)

`present_samples_grid_atlas_so_glyphs_reach_the_target` (`frame_renderer.rs`):
presents a cursor-hidden 2×1 terminal of bright-fg glyphs through
`render_and_present_frame` into a grid-sized target, reads back the GPU target
(`target_bytes`), and asserts it is **non-uniform** (a glyph drew foreground
over the background). **Verified it discriminates:** with the fix it passes (95
distinct colors — the antialiased glyph over black); when the present is
reverted to sample a fresh empty `Atlas`, it **fails** (1 distinct color —
uniform background). This is the GPU-side assertion the review required (the
earlier "grid atlas non-empty" idea was vacuous).

### Discovery (→ Exp 18): the projection/uniform setup is also missing

Getting the test to render at all required setting up the uniforms the **rebuild
does not touch** — `update_screen_size` (the orthographic `projection_matrix` +
`screen_size`) and the cell size (`test_with_grid`/`update_font_grid`). With
`test_with_grid`'s default **identity** projection, glyphs map far outside NDC
and nothing draws (the first test runs returned a uniform **black** target with
`fg_count=2` — vertices emitted, but off-screen). The live path
(`build_live_renderer` / `present_live`) **never calls
`update_screen_size`/`update_font_grid`**, so even with atlas coherence the live
app would render off-screen. Wiring those uniforms into the live present path
(from the surface size + the grid metrics, updated on resize) is **Exp 18** —
after which the launched app should finally show terminal text.

### Verification

- **Full `cargo test -p roastty`:** lib **4402 passed** (incl. the new readback
  test) + `abi_harness` **1 passed**, **0 failures**.
- Discrimination of the new test verified (break→fail→restore).
- App launch deferred to Exp 18 (it would still render off-screen without the
  projection wiring; no point capturing a known-black frame — the gap is pinned
  from source).

## Conclusion

Atlas coherence — one of the two things standing between the wired present path
and real text — is fixed and locked in by a test that genuinely fails if the
present samples the wrong atlas. The remaining blocker is now precisely pinned:
the live present path must set the **screen-size / font-grid uniforms**
(`update_screen_size` + `update_font_grid`) so the orthographic projection maps
the grid onto the NSView, instead of relying on an identity projection that puts
every glyph off-screen. That is **Exp 18**; with both in place, `present_live`
should put the shell prompt on screen — the first real terminal frame from
libroastty. (Exp 19 remains the continuous `CVDisplayLink` driver for live
updates.)

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** It independently reran the gates (both
`present_samples_grid_atlas_…` and `render_and_present_frame_presents` pass on a
**real Metal device** — the 4.7s runtime proves the `metal_device()` guard
didn't early-return; `4402` lib + `abi_harness` 1, `cargo fmt --check` clean)
and audited the routing: `rebuild_and_present_frame` is the **only** live
present path and samples `&targets.grid.atlas_*` — no standalone empty atlas
survives in the present path (the remaining standalone `Atlas` constructions are
compositor _seeds_ or isolated `present_metal_frame` unit tests). The test
**discriminates by construction** (target sized exactly to the grid, black bg,
cursor hidden → the only non-uniformity source is a rasterized glyph). It
confirmed from source that `build_live_renderer`/`present_live` never call
`update_screen_size`/`update_font_grid` (so live glyphs render off-screen until
Exp 18) and that the Partial framing is honest, with no weakened coverage and no
`target_bytes` leak outside `#[cfg(test)]`. Two Optional findings:

- **It could not re-run the break→fail check** (read-only, can't edit tracked
  source) — flagged for legibility. (The implementer DID run it: reverting the
  present to a fresh empty `Atlas` drops the readback to **1 distinct color**
  and the test FAILS; restored.)
- **Optional hardening — the `distinct.len() > 1` assertion** was slightly
  weaker than "a foreground pixel is non-background." **Applied:** the test now
  asserts a **bright-fg glyph pixel** specifically (BGRA red channel `> 100`,
  which only a rendered glyph produces over the black bg), so it survives later
  setup changes.
