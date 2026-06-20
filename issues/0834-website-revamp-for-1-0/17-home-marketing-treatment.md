# Experiment 17: Home / marketing page treatment (Phase 2)

## Description

The Phase-2 deliverable "the home/marketing page treatment for 1.0 (the headline
UX story)." The current homepage (`src/pages/index.astro`) is thin — a logo
hero, a one-line description, and a single screenshot — and it carries a **real
accuracy bug**: it claims TermSurf renders with "Chromium **and WebKit**" and is
"the only browser that supports multiple profiles **and multiple engines** in
the same window at the same time." Only **Chromium (Roamium)** ships in 1.0;
WebKit/Surfari, Gecko, and Ladybird are Planned/Researched (per the engine table
in the root `CLAUDE.md`). So the live homepage overstates the shipped product.

This experiment rebuilds the homepage as a proper 1.0 marketing landing built
from the documented design system (Exp 13) — hero with the headline UX pitch, a
capability grid, the screenshot showcase, and clear CTAs — with **every claim
macOS-accurate and engine-accurate** (Chromium shipped; other engines clearly
"planned"). It is **treatment/template** work: tight, accurate marketing copy
that links out to the docs for depth — the deep feature/UX documentation is
Phase 4.

## Key decisions

1. **Scope: the home page only, as design treatment.** Rebuild
   `src/pages/index.astro` into a sectioned landing. Keep copy tight and link to
   `/docs` for depth (Phase 4 owns the deep UX story, the `web` TUI reference,
   protocol, engines, roadmap). No new components are required — compose with
   existing Tailwind + design-system tokens and the same `Base.astro` shell (so
   the Exp-15 skip link / landmarks / a11y baseline apply automatically). The
   docs landing (`/docs`) and per-section index template are a **separate**
   later Phase-2 experiment.
2. **Headline UX story.** Lead with the pitch from the issue inventory: a
   browser that lives in your terminal — `web <url>` opens a real browser as a
   GPU overlay in a pane; **no alt+tab, no context switch**; modal,
   keyboard-driven navigation. This is the site's center of gravity, stated
   once, crisply, on the home page, linking to **`/docs`** for depth. (The
   dedicated "How TermSurf Works" page is a Phase-4 deliverable and is **not yet
   built**, so the home page must not link to it — review point; link `/docs`
   until it exists.)
3. **Accuracy — the load-bearing constraint.** Fix the overclaims and state only
   what 1.0 ships (macOS-only):
   - **Engine:** "real Chromium" (Roamium). Multiple **engines** are an
     architectural goal but **not shipped** — present WebKit/Gecko/Ladybird only
     under a clearly-marked "planned" treatment, never as current capability.
   - **Multiple profiles in one window** _is_ shipped (isolated cookies/storage
     per profile, incognito) — keep the capability, but drop both the "multiple
     engines … at the same time" present-tense claim **and** the unsubstantiated
     "**the only browser that…**" superlative (review point). State the
     capability plainly ("isolated profiles, side by side in one window")
     without an unverifiable absolute. No "only browser" / "the only" / "first"
     superlatives anywhere on the page.
   - **macOS-only:** no cross-platform claims (Ghostboard is a macOS Ghostty
     fork; scope decision 5).
   - Shipped capabilities to feature (all from the issue's verified inventory):
     the `web` TUI chrome (URL bar, modes), GPU/zero-copy Metal compositing,
     split **pane borders** (Ghostboard's addition over Ghostty), profiles +
     incognito, DevTools-in-a-split, dark-mode forwarding, the multi-process
     Unix-socket + protobuf architecture.
   - **Roadmap items** (bookmarks, tabs, history, downloads, PDF, more
     engines/frontends) appear only if explicitly labeled "Planned."
4. **Built from the design system, zero new JS.** Hero, a responsive capability
   grid (CSS grid, `md:` breakpoints consistent with Exp 14), the screenshot
   showcase, and CTAs (Docs / Install / GitHub), all using the semantic Tokyo
   Night tokens (now AA-compliant after Exp 16) and the type scale — **no
   hardcoded colors**, no new client JS. Light/dark via the existing
   `prefers-color-scheme` tokens.
5. **CTAs point at real targets.** "Read the docs" → `/docs`; "Install" →
   `/docs/getting-started` (the existing install/setup page); "GitHub" → the
   repo. No links to unbuilt pages.

## Changes

Files in `website/`:

1. **`src/pages/index.astro`** — rebuild as the sectioned marketing landing
   (hero + UX pitch, capability grid, screenshot showcase, CTAs), accurate +
   macOS/Chromium-only, design-system tokens, zero new JS. Replaces the thin
   hero-and-single-paragraph layout and **removes the WebKit / "multiple engines
   at the same time" overclaims**.

No other files change: no new components, no content-collection/schema/nav
changes, no fork changes. `Base.astro`, `Header.astro`, `Footer.astro`,
`style.css`, and all docs are untouched (the home page already uses
`Base.astro`, so the header/footer/skip-link/a11y baseline come for free).

## Verification

1. **Accuracy.** Built `/` contains **no** "WebKit" present-tense capability
   claim, **no** "multiple engines … at the same time" claim, and **no**
   unsubstantiated superlative (grep for "only browser", "the only", "first" —
   none present); engine claims say Chromium/Roamium; any mention of other
   engines or of bookmarks/tabs/history/downloads/PDF is explicitly labeled
   "Planned"; no non-macOS platform claim. Cross-check the feature list against
   the issue's shipped inventory — every featured capability is a shipped one.
2. **Design system, zero new JS.** The page uses semantic token utilities
   (`text-*`/`bg-*`/`border-*`) — a grep finds **no** hardcoded hex colors in
   `index.astro`; no `<astro-island>` / `<script>` is emitted on `/` (the home
   page has no client JS). Light/dark both resolve via tokens.
3. **Responsive.** The capability grid is single-column on mobile and
   multi-column at `md`+ (same breakpoint convention as Exp 14); the screenshot
   is fluid (`w-full`). No horizontal overflow at narrow widths (class-level
   check).
4. **CTAs resolve.** Every link on `/` targets a built page (`/docs`,
   `/docs/getting-started`, the GitHub URL) — dead-link crawl over `/` clean.
5. **Build + checks.** `bun run build` 76 pages (home is a fixed route — count
   unchanged); `bunx astro check` 0 errors; `gen:references --check` +
   `import:vt --check` exit 0.
6. **Accessibility of new content.** Built `/` has exactly one `<h1>` with
   correctly ordered `<h2>`s below it (no skipped heading levels across hero +
   capability grid + showcase), and every `<img>` has descriptive `alt`. The
   skip link + `<main id="main-content">` are inherited from `Base.astro`.
7. **No regressions.** `/welcome`, docs, search, nav unchanged; the header logo
   still links home; footer intact.

A full pass gives 1.0 an accurate, on-brand marketing home page that tells the
headline UX story and routes into the docs — completing the home/marketing
Phase-2 deliverable. Remaining Phase-2: the docs-landing / section-index page
template. (Deep feature/UX content is Phase 4; the `/welcome` on-black contrast
fix remains a logged follow-up.)

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE WITH CHANGES.** The
reviewer confirmed the diagnosis of the live overclaim, verified the featured
capability list is fully backed by the README shipped inventory (and that risky
shipped features like HTTP auth / crash recovery are correctly omitted from
marketing copy), confirmed both CTA targets exist (`/docs`,
`/docs/getting-started`) and the GitHub URL matches `Header.astro`, confirmed
`Base.astro` supplies the skip link + `<main id="main-content">` (a11y for
free), confirmed scope is genuinely Phase-2 (no component/Base/nav/schema
changes) and the zero-JS / semantic-token posture matches the site, and
confirmed `bun run build` = 76 pages. One **Required** + two **Optional**, all
folded in:

1. **(Required) Unsubstantiated superlative.** Deleting only the "multiple
   engines" clause would leave "the only browser that supports multiple profiles
   … in the same window" — an unverifiable absolute, and verification had no
   check for it. **Resolved:** decision 3 now drops the "only browser that…"
   superlative entirely (state the capability plainly), and verification 1 greps
   the built `/` for "only browser"/"the only"/"first" — none allowed.
2. **(Optional) Link to an unbuilt page.** The headline pointed at the Phase-4
   "How TermSurf Works" doc, which doesn't exist. **Resolved:** decision 2 links
   `/docs` until that page is built.
3. **(Optional) Home-specific a11y.** Added verification 6: single `<h1>`,
   ordered `<h2>`s (no skipped levels), descriptive `alt` on every image.

## Result

**Result:** Pass

The homepage is rebuilt as an accurate, design-system marketing landing; all
seven criteria pass.

### What was built

`src/pages/index.astro` — a sectioned landing on the existing `Base.astro`
shell: hero (logo + "Terminal + Browser" tagline, the `ROOT ACCESS TO THE 'NET.`
headline as the single `<h1>`, the no-alt+tab pitch with an inline
`web example.com`, and Install / Read the docs / GitHub CTAs); the
`screenshot3.webp` showcase with a caption; a six-card "What ships in 1.0"
capability grid (`md:grid-cols-2`); a "How it works" protocol blurb linking to
`/docs`; and a clearly-labeled **Planned** roadmap list. All copy is macOS- and
Chromium-accurate; the WebKit / "multiple engines … at the same time" / "the
only browser" overclaims are gone.

### Verification results

1. **Accuracy** — built `/` has **no** "the only" / "only browser" / "first"
   superlative, **no** "multiple engines" present-tense claim, **no**
   Linux/Windows claim; the single "WebKit" occurrence is inside the **Planned**
   roadmap list ("More browser engines — WebKit (Surfari), …"); engine copy is
   Chromium/Roamium; every capability-grid item is from the shipped inventory.
   **Pass.**
2. **Design system, zero new JS** — `grep` finds **no** hardcoded hex in
   `index.astro` (semantic `text-*`/`bg-*`/`border-*` only); built `/` emits
   **0** `<script>`/`astro-island`. **Pass.**
3. **Responsive** — capability grid is one column on mobile, `md:grid-cols-2` at
   `md`+; CTAs `flex-wrap`; screenshot `w-full`. **Pass.**
4. **CTAs resolve** — home-link crawl: `/docs`, `/docs/getting-started`, and the
   GitHub URL all resolve; 0 broken internal links. **Pass.**
5. **Build + checks** — `bun run build` 76 pages; `bunx astro check` 0 errors /
   0 warnings / 3 pre-existing hints; `gen:references --check` +
   `import:vt --check` exit 0. **Pass.**
6. **Accessibility** — built `/` has exactly one `<h1>`
   (`ROOT ACCESS TO THE 'NET.`) → `<h2>`s (What ships / How it works / Planned)
   → `<h3>` card titles, no skipped levels; all three `<img>`s have descriptive
   `alt`; skip link + `<main id="main-content">` inherited from `Base.astro`.
   **Pass.**
7. **No regressions** — diff is `index.astro` only; `/welcome`, docs, search,
   nav, header, footer unchanged. **Pass.**

## Conclusion

The home/marketing Phase-2 deliverable is done: 1.0 has an accurate, on-brand
landing page that leads with the headline UX story (a real browser in your
terminal, no alt+tab), features only shipped macOS/Chromium capabilities, marks
the roadmap clearly as Planned, and routes visitors into the docs — with zero
new JS and AA-compliant Tokyo Night tokens. The live accuracy bug (WebKit /
"multiple engines" / "only browser") is fixed. Remaining Phase-2: the
docs-landing / section-index page template. (Deep feature/UX content is Phase 4;
the `/welcome` on-black contrast fix remains a logged follow-up.)

## Completion Review

Independent `adversarial-reviewer` at the result gate. **Verdict: APPROVE** (no
Required/Optional/Nit findings). The reviewer rebuilt the site and independently
verified: the accuracy constraint holds (greps for "the only"/"only
browser"/"first"/"multiple engines"/"linux"/"windows"/"cross-platform" all
empty; the sole "WebKit"/engine-name mentions sit inside the **Planned** list;
every capability card maps to a README shipped-inventory line; risky shipped
features like HTTP auth / crash recovery correctly omitted); roadmap correctly
labeled; no hex / zero client JS / uses `Base.astro`; one `<h1>` with heading
sequence `[1,2,3,3,3,3,3,3,2,2]` (no skips) and descriptive `alt` on all three
images; CTAs resolve with `rel="noopener noreferrer"` on GitHub; 76 pages,
`astro check` 0 errors, drift checks exit 0; diff is `index.astro` only with
`showHeaderBrand={false}` intentional. One out-of-scope note (not a finding):
the footer `astrohacker.com` link lacks `rel="noopener"`, but that lives in the
untouched `Footer.astro`.
