# Experiment 14: Responsive mobile docs nav (Phase 2)

## Description

A Phase-2 experiment fixing a real, twice-flagged limitation (Exps 11 & 13): the
docs sidebar is `hidden … md:block`, so **below the `md` breakpoint there is no
navigation and no search** — mobile/narrow-viewport readers can't move between
the ~76 doc pages or search them. This makes the docs usable on mobile, with
**zero JavaScript** (native `<details>`).

## Key decisions

1. **Single sidebar, zero-JS disclosure — `<details open>` always (revised after
   review).** Wrap the sidebar's content (`<Search />` + generated `<nav>`) in a
   native `<details class="docs-sidebar" open>` + `<summary>` ("Documentation").
   The `open` attribute is **always present**, so content is visible at every
   width in every engine — no fragile "force-show a _closed_ details" CSS. (The
   original draft overrode the UA's closed-details hiding on the child `<div>`;
   the review showed modern browsers hide via
   `::details-content { content-visibility: hidden }`, a pseudo a descendant
   **cannot** un-skip — that would have left the desktop sidebar invisible in
   current Chromium. `open` sidesteps it.)
   - **Mobile (< `md`):** the `<summary>` shows; tapping it collapses/expands
     the sidebar (starts expanded — fine, since most items are nested in the
     already- collapsed `.docs-nav-sub` VT subsections).
   - **`md`+ (desktop):** CSS hides only the summary
     (`.docs-sidebar > summary { display: none }`); content stays visible
     because the details is `open`, so desktop is unchanged. All new rules use
     the **child combinator** (`.docs-sidebar > summary`) so the nested
     `.docs-nav-sub` VT disclosures are untouched (review point).
2. **Responsive layout.** `<div class="flex gap-8">` → `md:flex md:gap-8` (stack
   on mobile, row on `md`+); the `<aside>` becomes
   `block mb-6 md:mb-0 md:w-48 md:shrink-0` (full-width mobile → fixed column on
   `md`+, replacing `hidden … md:block`). The `<article>` keeps
   `data-pagefind-body`, `prose-termsurf`, `min-w-0 flex-1` (inert on the mobile
   block parent; lets it shrink in the `md`+ flex row).
3. **Scope.** Mobile nav only. The header (logo + GitHub/Docs links) is already
   responsive (flex, small). Other page templates / the home treatment / a11y
   audit are separate Phase-2 experiments. This removes the "no search/nav below
   `md`" caveat recorded in Exps 11/13.

## Changes

Files in `website/`:

1. **`src/components/DocPage.astro`** — restructure the sidebar: `md:flex`
   layout; `<aside>` responsive widths; wrap `<Search/>` + `<nav>` in
   `<details class="docs-sidebar" open><summary>Documentation</summary>…</details>`
   (the `open` attribute is always emitted).
2. **`src/styles/style.css`** — `.docs-sidebar` styling: a tappable summary on
   mobile (marker + label, Tokyo Night) via the child combinator, and the `md`+
   rule `.docs-sidebar > summary { display: none }` (content stays visible
   because the details is `open` — no force-show needed). Scoped so the nested
   `.docs-nav-sub` disclosures are untouched.

No content/fork/schema/nav-data changes; `docs-nav.ts` and all pages are
untouched.

## Verification

1. **Markup.** Built doc pages contain
   `<details class="docs-sidebar" open><summary>…Documentation…</summary>`
   wrapping the search box + the generated `<nav>` (all sidebar items still
   present, in order). The `open` attribute is present (content visible at all
   widths by spec).
2. **Zero JS.** No `<astro-island>` / `<script>` added by this change; the
   disclosure is native `<details>` (Pagefind's existing `is:inline` search
   script is the only JS, unchanged).
3. **Desktop unchanged.** At `md`+ the CSS hides only the `<summary>`
   (`.docs-sidebar > summary { display: none }`); the content stays visible
   because the details is `open`, so the sidebar renders as the left column it
   is today; the rendered nav section order is unchanged (Configuration →
   Terminal API → Components → Protocol after ungrouped). The nested
   `.docs-nav-sub` summaries are **not** hidden (child-combinator scope).
   Spot-check in a current Chromium that the desktop sidebar is fully visible
   (the `open` approach is spec-guaranteed, unlike the rejected force-show).
4. **Build + checks.** `bun run build` 76 pages; `astro check` 0 errors;
   `gen:references --check` + `import:vt --check` exit 0; dead-link crawl clean.
5. **No regressions.** `/` and `/welcome` (no DocPage) unaffected; search still
   builds; existing pages render.

A full pass makes the docs navigable + searchable on mobile with no JS, clearing
the Exp-11/13 limitation. Remaining Phase-2: other page templates, the
home/marketing treatment, and an accessibility baseline.

## Design Review

**Pass 1 — REJECT.** The reviewer found the load-bearing mechanism wrong: the
original plan force-showed a **closed** `<details>` by overriding
`content-visibility`/`display` on the child `<div>`, but evergreen browsers hide
collapsed content via
`details:not([open])::details-content { content-visibility: hidden }` — a
pseudo-element whose skipped subtree a descendant **cannot** un-skip — so the
desktop sidebar would be invisible in current Chromium. The verification merely
restated the broken mechanism. Resolved by switching to the reviewer's
recommended robust approach: render `<details open>` **always** and only hide
the `<summary>` on `md`+ (content is visible at every width because the details
is open; mobile can still collapse it). Also folded in: child-combinator scoping
so the nested `.docs-nav-sub` disclosures aren't clobbered; the explicit `aside`
mobile width; and a cross-engine desktop-visibility check.

**Pass 2 — APPROVE.** A fresh reviewer confirmed the revised mechanism is
spec-robust ("an open `<details>` renders its content" is the most universally
implemented behavior; the `:not([open])` UA hiding guard never fires); the child
combinator `.docs-sidebar > summary` does not match the nested
`.docs-nav-sub > summary` (verified DOM depth), so VT disclosures keep working;
the site is a plain MPA (no view transitions), so the `open` state resets per
navigation; and the layout is sound on both block-mobile and flex-`md`+, with
Pagefind's `#docs-search` init unaffected. Two non-blocking items, folded into
implementation: (a) the new `.docs-sidebar > summary` gets the same marker reset
as `.docs-nav-sub` (`list-style:none` +
`::-webkit-details-marker {display:none}`) so no double triangle; (b) **known
minor edge** — if a user manually collapses on mobile then live-resizes across
the `md` breakpoint without navigating, the summary becomes `display:none` while
closed → content hidden until any navigation (self-healing; acceptable for a
docs site).

## Result

**Result:** Pass

The sidebar is now a `<details open>` with a mobile "Documentation" summary
toggle; desktop is unchanged. All criteria pass.

### What was built

- `src/components/DocPage.astro` — `<div class="md:flex md:gap-8">`,
  `<aside class="mb-6 block md:mb-0 md:w-48 md:shrink-0">`, sidebar content
  wrapped in
  `<details class="docs-sidebar" open><summary>Documentation</summary><Search/> +
  <nav>…</nav></details>`.
- `src/styles/style.css` — `.docs-sidebar > summary` marker reset (`list-style`
  - `::-webkit-details-marker`) with a `☰` glyph; `@media (min-width:768px)`
    hides the summary (child-combinator scoped, so `.docs-nav-sub` is
    untouched).

### Verification results

1. **Markup** — built pages emit `<details class="docs-sidebar" open>` with the
   "Documentation" summary wrapping the `#docs-search` box + the generated
   `<nav>`; `open` attribute present (content visible at all widths). **Pass.**
2. **Zero JS** — the only scripts on a doc page are the pre-existing Pagefind
   ones; no `<astro-island>`; the disclosure is native `<details>`. **Pass.**
3. **Desktop unchanged** — built CSS has `.docs-sidebar>summary{display:none}`
   under `@media (min-width:768px)`; nav section order is intact (Configuration
   → Terminal API → Components → Protocol); the nested `.docs-nav-sub` summaries
   are not hidden (still present on `vt/csi/cup`). **Pass.**
4. **Build + checks** — `bun run build` 76 pages; `astro check` 0 errors;
   `gen:references --check` + `import:vt --check` exit 0. **Pass.**
5. **No regressions** — `/`, `/welcome` (no DocPage) unaffected; search builds.
   **Pass.**

## Conclusion

The docs are navigable and searchable on mobile with **zero JS** — the
twice-flagged Exp-11/13 limitation is cleared via an always-`open` `<details>`
whose summary is hidden on `md`+. Remaining Phase-2: other page templates
(home/article/reference/section-index treatments), the home/marketing page, and
an accessibility baseline. Then Phases 3–4 build the content into the IA.
