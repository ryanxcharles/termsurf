# Experiment 15: Accessibility baseline (Phase 2)

## Description

The Phase-2 deliverable "Responsive + light/dark behavior, accessibility
baseline." Responsive nav landed in Exp 14; this experiment establishes the
**accessibility baseline** for the whole site with a small set of standard,
low-risk, zero-JS fixes, and **documents a WCAG-AA contrast audit** of the Tokyo
Night tokens so the design system has an accessibility record.

Concrete gaps found by auditing the current build:

1. **No skip-to-content link.** Every doc page renders the full sidebar (header
   nav + a "Documentation" disclosure containing ~76 links) _before_ the
   article. A keyboard or screen-reader user has to traverse all of it on every
   page to reach the content. A skip link is the canonical baseline fix.
2. **Unlabeled `<nav>` landmarks.** Two `<nav>` landmarks (header primary nav in
   `Header.astro`; the docs sidebar nav in `DocPage.astro`) have no accessible
   name, so assistive tech announces two indistinguishable "navigation" regions.
3. **No `prefers-reduced-motion` handling.** Hover/color transitions and any CSS
   animation play regardless of the user's reduced-motion preference. (The
   Three.js `/welcome` scene is explicitly out of scope — "Do not modify when
   changing site-wide styles" — this is the site-wide CSS baseline only.)
4. **No documented contrast record.** Scope decision 4 is "keep & refine Tokyo
   Night"; an AA audit tells us whether any token pair needs refining and leaves
   a record in the design system.

## Key decisions

1. **Skip link (zero-JS, native anchor).** Add a visually-hidden-until-focused
   `<a href="#main-content" class="skip-link">Skip to content</a>` as the first
   child of `<body>` in `Base.astro`, and give `<main>` `id="main-content"` plus
   `tabindex="-1"` (so focus actually moves to it for screen readers, not just
   the scroll position). Style `.skip-link` in `style.css` with the standard
   off-screen-until-`:focus` pattern (Tokyo Night accent box on focus). This
   covers **every `Base.astro` page** (home + all docs), which is everything
   with site nav.
   - `/welcome` is **not** a `Base.astro` page — it defines its own standalone
     `<html>/<body>` shell (`welcome.astro`) with no header/footer/nav, so it
     has no skip link and needs none (nothing to skip). It is out of scope ("Do
     not modify when changing site-wide styles").
2. **Label the nav landmarks.** `aria-label="Primary"` on the header `<nav>`
   (`Header.astro`); `aria-label="Documentation"` on the docs sidebar `<nav>`
   (`DocPage.astro`). Distinct accessible names for the two navigation regions.
   No visual change.
3. **`prefers-reduced-motion` baseline.** Add a global
   `@media (prefers-reduced-motion: reduce)` block in `style.css` that reduces
   `transition`/`animation` duration to near-zero and disables
   `scroll-behavior`, scoped broadly (`*`) but additive (no layout change). This
   is the standard reduced-motion baseline; it does not touch the React/Three.js
   `/welcome` island (that scene manages its own animation loop in JS,
   unaffected by CSS, and is out of scope).
4. **Visible focus is preserved, not removed.** Audit confirms no rule sets
   `outline: none` / `outline: 0` (the default UA focus ring is itself an
   accessibility feature). This experiment adds a consistent, token-based
   `:focus-visible` outline (`outline: 2px solid var(--color-accent)`;
   `outline-offset: 2px`) on interactive elements —
   `a, summary, button, [tabindex]:not([tabindex="-1"])` so focus is clearly
   visible in both light and dark — **additive**, never removing the ring. The
   selector **excludes `[tabindex="-1"]`** so the programmatic skip-link target
   (`<main id="main-content" tabindex="-1">`) does **not** draw a 2px ring
   around the whole content area when focus lands on it (review point — main is
   a scroll/SR focus target, not a tabbable control). The accent ring meets the
   AA non-text 3:1 threshold against every background in both modes (verified:
   light 4.26:1 vs `--color-background`, 3.80:1 vs `--color-background-dark`;
   dark ≥9.9:1). Pagefind keeps `resetStyles: false`; its light-DOM controls
   also pick up the additive ring (harmless, confirmed no double-ring at
   implementation).
5. **Contrast audit — record the debt; remediation is its own experiment.** The
   design review computed AA contrast for the load-bearing token pairs and found
   the light "Tokyo Night Day" palette fails AA 4.5:1 for **multiple** text
   tokens (`--color-foreground-dark` 3.57:1 — the prose body color;
   `--color-muted` 2.54:1 — footer/nav/footnotes, also fails the 3:1 floor;
   `--color-primary` 3.11:1 — h2/active nav; `--color-secondary` 3.33:1 — inline
   code; `--color-accent` 4.26:1 — links), and `--color-muted` also fails in
   dark (2.76:1). This is **systemic light-mode contrast debt across several
   tokens**, not a one-token tweak — so a "refine one token" contingency here
   would be false. This experiment therefore **audits and records** the full
   ratio table (light + dark, every text token) in `website/CLAUDE.md` with
   explicit pass/fail and the per-usage WCAG threshold, and **defers the palette
   remediation to a dedicated follow-up, Experiment 16** (contrast refinement).
   Splitting it is correct project hygiene: per scope decision 4 the palette is
   the established brand, so a multi-token "refine Tokyo Night" change is
   brand-sensitive and deserves its own focused design + adversarial review, not
   a tacked-on contingency inside the structural-a11y experiment. Exp 15 makes
   the debt **visible and documented**; Exp 16 pays it down.

## Changes

Files in `website/`:

1. **`src/layouts/Base.astro`** — add the `.skip-link` anchor as the first
   `<body>` child; add `id="main-content"` + `tabindex="-1"` to `<main>`.
2. **`src/components/Header.astro`** — `aria-label="Primary"` on the `<nav>`.
3. **`src/components/DocPage.astro`** — `aria-label="Documentation"` on the
   sidebar `<nav>`.
4. **`src/styles/style.css`** — `.skip-link` off-screen/`:focus` styles; a
   global `:focus-visible` outline (token-based, additive); the
   `prefers-reduced-motion` block.
5. **`website/CLAUDE.md`** — record the full contrast-audit table (every text
   token, light + dark, ratio + pass/fail) in the Design system section, noting
   the systemic light-mode debt and that **Experiment 16** remediates it; and
   note the accessibility baseline (skip link, labeled landmarks, focus-visible,
   reduced-motion).

No content/fork/schema/nav-data changes; `docs-nav.ts`, the generated reference
pages, and all VT pages are untouched. The `/welcome` Three.js scene is not
modified.

## Verification

1. **Skip link.** Built pages emit
   `<a … class="skip-link" href="#main-content">` as the first body child, and
   `<main id="main-content" tabindex="-1">`. The CSS positions `.skip-link`
   off-screen and reveals it on `:focus`. Manual: load a doc page, press Tab
   once — the skip link appears; Enter moves focus to the article. **Pass:**
   present + reveals on focus. **Fail:** missing, or visible at rest.
2. **Labeled landmarks.** Built header has `<nav aria-label="Primary">`; built
   doc pages have `<nav aria-label="Documentation">`. Exactly the two navigation
   landmarks, each named.
3. **Reduced motion.** Built CSS contains a
   `@media (prefers-reduced-motion: reduce)` block neutralizing
   transition/animation/scroll-behavior. No visual change with the preference
   off.
4. **Focus visible.** Built CSS has a `:focus-visible` rule using
   `var(--color-accent)` whose selector **excludes `[tabindex="-1"]`**; a grep
   confirms **no** `outline: none` / `outline: 0` anywhere in `src/` (focus ring
   never suppressed). Activating the skip link does not draw a ring around the
   whole `<main>`.
5. **Contrast audit recorded.** `website/CLAUDE.md` lists the AA ratios for
   **every text token** (light + dark) with pass/fail and the per-usage
   threshold, and states the systemic light-mode failures with remediation
   deferred to **Experiment 16**. (No palette tokens are changed in this
   experiment — the audit is verify-and-record.)
6. **Build + checks.** `bun run build` 76 pages; `bunx astro check` 0 errors;
   `bun run gen:references --check` + `bun run import:vt --check` exit 0;
   dead-link crawl over `dist/docs/**` clean.
7. **No regressions.** `/`, `/welcome`, search, nav, callouts unchanged
   visually; the skip link is invisible at rest on every `Base.astro` page;
   `/welcome` (own shell, no nav) is untouched; no color tokens change (so no
   visual delta from the audit).

A full pass gives the site a documented accessibility baseline (skip link,
labeled landmarks, visible focus, reduced-motion) and an honest contrast record
that surfaces the light-mode AA debt. Remaining Phase-2: **Experiment 16**
(Tokyo Night contrast refinement — pays down the debt this audit records), page
templates (article/reference/section-index), and the home/marketing treatment.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE WITH CHANGES.** The four
structural mechanisms were confirmed sound with evidence: the skip link as first
`<body>` child is the first focusable element and is not clipped (no ancestor
`overflow`), `<main tabindex="-1">` moves focus per spec; no `outline:none/0`
exists in `src/` (additive-focus claim holds); the accent focus ring meets the
3:1 non-text threshold against every background in both modes; the
`prefers-reduced-motion *{}` block is safe (native `<details>` toggling is not a
CSS transition, the Three.js scene is a JS rAF loop unaffected by CSS);
`<html lang="en">` is present and every `src/` `<img>` has `alt`; scope is
minimal and zero-JS.

One **Required** finding, plus two **Optional** — all folded in:

1. **(Required) The contrast remediation was self-contradictory.** Measured AA
   ratios show the light palette fails 4.5:1 for **multiple** text tokens
   (foreground-dark 3.57, muted 2.54 — below even 3:1, primary 3.11, secondary
   3.33, accent 4.26) and muted fails in dark (2.76), so "refine **one** token
   and all body text passes AA" was impossible and clashed with "don't repaint."
   **Resolved** by reframing decision 5: this experiment now **audits and
   records** the full debt (no token changes), and the multi-token Tokyo Night
   refinement is split into its own **Experiment 16** with dedicated design +
   review — correct hygiene for a brand-sensitive palette change under scope
   decision 4.
2. **(Optional) Focus ring on the skip target.** The `[tabindex]` focus-visible
   rule would ring the whole `<main>` on skip activation. **Resolved:** selector
   excludes `[tabindex="-1"]`.
3. **(Optional) Pagefind controls** also receive the additive ring (light-DOM,
   `resetStyles:false`). **Noted:** harmless/additive; confirm no double-ring at
   implementation.

## Result

**Result:** Pass

The site has a documented accessibility baseline; all criteria pass. No color
tokens were changed — the contrast audit is recorded as debt for Exp 16.

### What was built

- `src/layouts/Base.astro` — `.skip-link` ("Skip to content") as the first
  `<body>` child; `<main id="main-content" tabindex="-1">`.
- `src/components/Header.astro` — `aria-label="Primary"` on the header `<nav>`.
- `src/components/DocPage.astro` — `aria-label="Documentation"` on the sidebar
  `<nav>`.
- `src/styles/style.css` — `.skip-link` off-screen/`:focus` reveal;
  `#main-content:focus { outline: none }` (skip target); additive
  `:focus-visible` ring on
  `a, summary, button, [tabindex]:not([tabindex="-1"])`; a
  `@media (prefers-reduced-motion: reduce)` block.
- `website/CLAUDE.md` — Accessibility baseline section + the full WCAG-AA
  contrast table (every text token, light + dark), recording the systemic
  light-mode debt deferred to Exp 16.

### Verification results

1. **Skip link** — built `Base.astro` pages (home + docs) emit
   `<body …> <a href="#main-content" class="skip-link">Skip to content</a>` as
   the first body child and `<main id="main-content" tabindex="-1">`; the built
   CSS positions `.skip-link` at `left:-9999px` and `:focus{left:0}`. `/welcome`
   has its own shell (no `Base.astro`, no nav) and correctly has no skip link.
   **Pass.**
2. **Labeled landmarks** — built home has `<nav … aria-label="Primary">`; built
   doc pages have `<nav … aria-label="Documentation">` (1 each). **Pass.**
3. **Reduced motion** — built CSS contains one
   `@media (prefers-reduced-motion:reduce)` block zeroing animation/transition
   durations + `scroll-behavior`. **Pass.**
4. **Focus visible** — built CSS has
   `a:focus-visible,summary:focus-visible,button:focus-visible,[tabindex]:not([tabindex="-1"]):focus-visible{outline:2px solid var(--color-accent);outline-offset:2px}`;
   the only `outline:none` in `src/` is the intentional `#main-content:focus`
   skip-target rule. **Pass.**
5. **Contrast audit recorded** — `website/CLAUDE.md` has the AA table:
   light-mode AA-text PASS only for `foreground` (9.62); `foreground-dark` 3.57,
   `primary` 3.11, `secondary` 3.33, `accent` 4.26, `success` 4.04, `warning`
   4.29, `caution` 3.79 all FAIL; `muted` 2.54 fails the 3:1 floor. Dark mode
   passes AA-text for all text tokens except `muted` (2.76). Remediation
   deferred to Exp 16; no tokens changed here. **Pass.**
6. **Build + checks** — `bun run build` 76 pages (`find dist -name '*.html'`);
   `bunx astro check` 0 errors / 0 warnings / 3 pre-existing WelcomePage
   Three.js hints; `gen:references --check` + `import:vt --check` exit 0;
   cross-page link + path-qualified-anchor crawl over the 74 docs pages = 0
   broken (asset URLs excluded). The completion review surfaced one
   **pre-existing** bare same-page fragment (`#c1-sequences` in
   `vt/concepts/sequences`) that a path-qualified crawl doesn't cover —
   untouched VT content, not a regression of this experiment; logged for a
   future VT pass. **Pass.**
7. **No regressions** — no color tokens changed (zero visual delta from the
   audit); the skip link is off-screen at rest on every `Base.astro` page; `/`
   and `/welcome` (own shell, untouched) build; search and nav unchanged.
   **Pass.**

## Conclusion

The structural accessibility baseline is in place site-wide (skip link, labeled
nav landmarks, additive token-based focus ring, reduced-motion) with zero new
JS, and an honest WCAG-AA contrast record now lives in the design system. The
audit's key finding — the light "Tokyo Night Day" palette fails AA-text for
nearly every accent/secondary token, and `muted` fails 3:1 in both modes — is
documented as debt and becomes the next experiment. **Experiment 16: Tokyo Night
contrast refinement** pays it down (refine, not repaint, per scope decision 4).
Remaining Phase-2 after that: page templates (article/reference/section-index)
and the home/marketing treatment.

## Completion Review

Independent `adversarial-reviewer` at the result gate. **Verdict: APPROVE WITH
CHANGES.** The reviewer built fresh (76 pages), ran `astro check` (0 errors, 3
pre-existing hints), `gen:references`/`import:vt --check` (exit 0), and
**independently recomputed all 18 token contrast ratios** — they matched the
`website/CLAUDE.md` table to two decimals, including every PASS/FAIL and the
muted <3:1 flags. Confirmed: skip link is the first body child + not clipped;
labeled landmarks; one reduced-motion block (CSS-only, doesn't touch
`<details>`/Pagefind/Three.js); the focus-visible selector matches exactly and
the only `outline:none` in `src/` is the intentional `#main-content:focus` rule;
no color tokens changed; scope limited to the five website files + two issue
docs.

One **Required** finding, fixed (docs-only, no code change):

- **(Required) False "every page" coverage claim.** `/welcome` does **not** use
  `Base.astro` — it has its own standalone shell with no nav — so it has no skip
  link/landmarks, contrary to the design/result/`CLAUDE.md` wording. Corrected
  all three to scope the coverage to `Base.astro` pages (home + docs) and note
  `/welcome` is a standalone shell needing no skip link (out of scope, not
  modified).

One **Optional** finding, addressed:

- **(Optional) Pre-existing VT anchor.** `vt/concepts/sequences` links to a bare
  `#c1-sequences` fragment with no matching id. It is untouched VT content (not
  a regression here) and was missed by the path-qualified crawl; the result's
  build-checks note now records it as pre-existing debt for a future VT pass.
