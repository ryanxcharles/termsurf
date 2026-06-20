# Experiment 24: How TermSurf Works (Phase 4)

## Description

A Phase-4 experiment adding the **How TermSurf Works** page — the end-to-end UX
story, the documentation's stated center of gravity alongside the `web` TUI (Exp
23). Today the docs explain the pieces (Web TUI, Architecture) but have no
single narrative that answers "what actually happens when I run `web <url>`?"
This page is that narrative: the no-alt-tab pitch and the request-to-overlay
flow, told conceptually and **linking** to the Web TUI and Architecture pages
for depth rather than duplicating them.

## Key decisions

1. **Top-level (ungrouped) MDX page `how-termsurf-works.mdx`, `order: 1.6`.**
   Route `/docs/how-termsurf-works`. With no `section`, it joins the ungrouped
   intro cluster; `order: 1.6` makes the nav read **Getting Started (1) → About
   (1.5) → How TermSurf Works (1.6) → Architecture (2)** without renumbering
   existing pages. (The IA's eventual "TermSurf" group — folding in Web TUI,
   Architecture, Protocol, etc. — is a separate later restructure;
   `website/CLAUDE.md` says it happens "when its landing page exists." This page
   does not start that restructure.)
2. **Narrative, not a re-spec.** Lead with the pitch (a real browser in a
   terminal pane; no alt+tab, no context switch), then walk the flow when you
   run `web example.com`:
   1. The `web` TUI starts in the pane and **connects to the GUI (Ghostboard)
      over a Unix socket** (it reads `TERMSURF_SOCKET`).
   2. The GUI **launches or reuses a browser engine process for your profile**
      (one engine process per profile).
   3. The engine renders the page; the GUI **composites it as a zero-copy GPU
      overlay** at the pane's pixel position (on macOS, via `CALayerHost`).
   4. The TUI **draws the chrome** (URL bar, status) and forwards your input;
      you switch between **Control** and **Browse** modes.

   Each technical claim links to its authoritative page (Architecture for
   multi-process/IPC/GPU, Web TUI for modes/keys) — this page stays conceptual
   and **does not restate** the socket wire format, the topology diagram, or the
   `CALayerHost` mechanics.

3. **Accuracy — consistent with the verified pages.** Every claim matches
   `architecture.mdx` (Unix-socket + protobuf via `TERMSURF_SOCKET`, one engine
   per profile, `CALayerHost` zero-copy compositing on macOS) and the
   source-verified Web TUI page (Control/Browse modes). macOS-scoped (GPU
   compositing is described as the macOS path; scope decision 5). The shipped
   engine is Chromium/Roamium; multi-engine is mentioned only as the protocol's
   design, with other engines clearly **planned** (consistent with Exp 17/19),
   linking Architecture's engine table rather than restating statuses.
4. **Design system, zero JS.** Plain MDX → `prose-termsurf`; an optional
   numbered list for the flow; semantic tokens only; links only to **built**
   pages (`/docs/getting-started`, `/docs/components/webtui`,
   `/docs/architecture`, `/docs/protocol/overview`, `/docs/components/roamium`).

## Changes

Files in `website/`:

1. **`src/content/docs/how-termsurf-works.mdx`** — new top-level narrative page
   (the pitch + the four-step flow + cross-links). Appears in the ungrouped
   intro cluster (sidebar) and the generated `/docs` index automatically via
   `getDocsNav()`.

No other files change: schema, `docs-nav.ts`, `DocPage`, other content, and the
fork are untouched. The home page is **not** modified (it links `/docs`, which
now lists this page); wiring the home hero to it directly is a possible later
tweak. Page count **80 → 81**.

## Verification

1. **Builds + placed correctly.** `bun run build` emits the
   `/docs/how-termsurf-works` route; total pages **81**. The ungrouped intro
   cluster (sidebar + the `/docs` "Overview" list) reads **Getting Started →
   About → How TermSurf Works → Architecture** (orders 1 / 1.5 / 1.6 / 2).
   `bunx astro check` 0 errors.
2. **Accuracy / consistency.** Every flow claim matches `architecture.mdx` and
   the Web TUI page — Unix socket (`TERMSURF_SOCKET`), one engine per profile,
   `CALayerHost` zero-copy on macOS, Control/Browse modes. No non-macOS GPU
   claim; no non-Chromium engine presented as shipped (multi-engine framed as
   design/planned, linking Architecture). No unsubstantiated superlative
   (consistent with Exp 17).
3. **No duplication.** The page summarizes-and-links; it does **not** restate
   the socket wire format, the topology ASCII diagram, or the `CALayerHost`
   internals (those stay on Architecture), nor the per-mode key tables (those
   stay on Web TUI).
4. **Design system, zero JS, links resolve.** `prose-termsurf`; no hardcoded
   hex; no `<astro-island>` beyond the inherited Pagefind search; dead-link
   crawl over `/docs/how-termsurf-works` = 0 broken (every cross-link resolves).
5. **a11y.** Exactly one `<h1>` ("How TermSurf Works"), ordered `<h2>`s (no
   skipped levels); descriptive link text.
6. **No regressions.** `gen:references --check` + `import:vt --check` exit 0;
   the new page is the only nav addition; search/`/`/`/welcome`/other pages
   unchanged.

A full pass adds the headline UX narrative, satisfying the issue's emphasis on
"the basic UX of how TermSurf works." Next Phase-4 candidates: the protocol
refresh, Browser Engines (Roamium + roadmap), Ghostboard's pane-border
additions, and the roadmap.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE** (no Required/Optional
findings). The reviewer verified each of the four flow steps against the
already-verified pages and the fork: (a) the TUI connects over a Unix socket via
`TERMSURF_SOCKET` (`architecture.mdx:73`); (b) one engine process per profile,
**launched or reused** — independently confirmed in the fork
(`apprt/termsurf.zig:1350` `findServer(profile, browser)` reuses an existing
server and only spawns when none exists) and in `protocol/overview.mdx`; (c)
zero-copy `CALayerHost` overlay scoped to macOS (`architecture.mdx:92`); (d) the
TUI draws chrome + Control/Browse modes (`webtui.mdx`). Confirmed macOS
accuracy, no overclaim/superlative, the summarize-and-link no-duplication
boundary, the `order: 1.6` placement (Getting Started → About → How TermSurf
Works → Architecture), all five cross-links resolve, single new MDX file, zero
JS, and that not wiring the home hero is no dangling promise (home links
`/docs`, which auto-lists the page). One **Nit** (no change needed): the fork's
reuse key is actually profile+browser, but "one engine process per profile" is
the faithful simplification every page uses — the page keeps that wording.
