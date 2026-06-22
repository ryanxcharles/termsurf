# Experiment 25: Split pane borders (Phase 4)

## Description

A Phase-4 experiment documenting the **split pane border** feature — the
TermSurf/Ghostboard terminal addition the user explicitly called out ("the other
features we've added, like the pane border feature"). Ghostboard adds
configuration to draw colored borders around split panes and visually
de-emphasize unfocused ones, so you can always see which pane is active. None of
this is documented on the site yet.

The feature's config options are **fork-verified** (in
`ghostboard/src/config/Config.zig`, and present in the generated config
reference):

- `focused-split-border-color` (`Config.zig:1091`) — border color of the focused
  split; unset = no border.
- `unfocused-split-border-color` (`Config.zig:1095`) — border color of unfocused
  splits; unset = no border.
- `split-border-width` (`Config.zig:1098`) — border width in points; `0`
  disables; clamped to `0..10` (`Config.zig:4638`).
- `unfocused-split-saturation` (`Config.zig:1102`) — color saturation of
  unfocused splits; `1.0` full color, `0.0` grayscale; clamped `0..1`
  (`Config.zig:4641`); only applies in a split layout.

These four are **Ghostboard additions over Ghostty**, on real evidence (not, as
an earlier draft wrongly claimed, "per root `CLAUDE.md`" — which does not
mention them): the issue-834 inventory lists them as Ghostboard's split-border
additions (README "Ghostboard terminal additions over Ghostty"), the fork
carries a "Port split border config" commit series, and — unlike genuine
upstream split options such as `split-divider-color` (`Config.zig:1086`,
annotated "Available since: 1.1.0") — these four carry **no** "Available since"
tag, consistent with fork-only additions. By contrast,
`unfocused-split-opacity`/ `unfocused-split-fill` predate them and are inherited
from Ghostty — the page documents the four additions and links the reference for
the rest, and does **not** attribute the framing to a doc that doesn't support
it.

## Key decisions

1. **New page `split-pane-borders.mdx`, `section: "Features"`, `order: 2`.**
   Route `/docs/split-pane-borders`. It joins the Features group (after the
   Features overview at order 1), so the sidebar reads **Features → Split Pane
   Borders**. No nav-data change (Features is already in `SECTION_ORDER`).
2. **Frame it as TermSurf's addition.** Open by stating these split-border
   options are Ghostboard's addition over Ghostty (framed as the fork adding
   them — **not** "per the project's docs"), then explain the behavior: set a
   focused and/or unfocused border color and a width to outline panes, and lower
   unfocused saturation to fade inactive panes. Keep it at feature-overview
   depth.
3. **Accuracy — verified options, exact text linked.** Document the four options
   with their effect and accepted values (colors: hex `#RRGGBB` or `RRGGBB`, or
   a named X11 color; width in points, `0` disables; saturation `0..1`),
   matching the generated reference wording, and **link
   `/docs/reference/config`** for the authoritative per-option text rather than
   restating it verbatim. State the clamped ranges (width `0..10`, saturation
   `0..1`) since those are real constraints. No invented options.
4. **macOS-accurate; no overclaim.** A terminal-rendering feature on macOS; no
   Linux/GTK content (scope decision 5). Don't claim border behavior beyond what
   the options do (e.g., don't assert exact pixel rendering details).
5. **Design system, zero JS.** Plain MDX → `prose-termsurf`; a small config
   example using the existing `bg-background-dark` `<pre>` token style; semantic
   tokens only; links only to **built** pages (`/docs/reference/config`,
   `/docs/reference/configuration`, `/docs/features`).

## Changes

Files in `website/`:

1. **`src/content/docs/split-pane-borders.mdx`** — new Features page (the four
   options + a config example + reference links). Appears under the Features
   sidebar group and in the generated `/docs` index automatically via
   `getDocsNav()`.

No other files change: schema, `docs-nav.ts`, generated references, the existing
Features overview, and the fork are untouched. Page count **81 → 82**.

## Verification

1. **Builds + placed correctly.** `bun run build` emits the
   `/docs/split-pane-borders` route; total pages **82**. The Features group
   (sidebar + `/docs` index) reads **Features → Split Pane Borders** (orders 1,
   2). `bunx astro check` 0 errors.
2. **Accuracy (fork-verified).** The four options and their effects/ranges match
   `Config.zig` (`:1091`/`:1095`/`:1098`/`:1102`, clamps `:4638`/`:4641`) and
   the generated reference; the "TermSurf addition over Ghostty" framing rests
   on the fork evidence (no "Available since" tag vs upstream
   `split-divider-color` @1.1.0; the "Port split border config" commit series;
   the issue-834 inventory) — **not** root `CLAUDE.md`, which doesn't mention
   them. The built page makes **no** "per the project's docs"-style attribution.
   No invented options; exact per-option text linked, not restated. Spot-check
   each against the source/reference.
3. **macOS-accurate.** No Linux/GTK text; no overclaimed rendering specifics.
4. **Design system, zero JS, links resolve.** `prose-termsurf`; no hardcoded hex
   **in prose** (a hex color may appear inside the config-example `<pre>` as a
   sample value, which is content, not styling); no `<astro-island>` beyond the
   inherited Pagefind search; dead-link crawl over `/docs/split-pane-borders` =
   0 broken.
5. **a11y.** Exactly one `<h1>` ("Split Pane Borders"), ordered `<h2>`s (no
   skipped levels); descriptive link text.
6. **No regressions.** `gen:references --check` + `import:vt --check` exit 0;
   the new Features entry is the only nav addition; search/`/`/`/welcome`/other
   pages unchanged.

A full pass documents the user-requested pane-border feature, fork-verified and
macOS-scoped. Next Phase-4 candidates: Browser Engines (Roamium + roadmap), the
protocol refresh, and the consolidated roadmap.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE WITH CHANGES.** The
reviewer confirmed all four options, their semantics, the clamps
(`Config.zig:4638` width 0..10, `:4641` saturation 0..1), the
generated-reference anchors, the Features placement (order 2 after
`features.mdx` order 1), the link targets, and the macOS framing. The
inherited-vs-added distinction (`unfocused-split-opacity`/`fill` inherited) is
correct. One **Required** + two **Optional**, folded in:

1. **(Required) False citation.** The draft attributed the "Ghostboard additions
   over Ghostty" framing to root `CLAUDE.md`, which contains **no** mention of
   these options (grep clean). The claim is nonetheless **true** and was
   re-grounded on real evidence the reviewer surfaced and I verified: the four
   options carry **no** "Available since" tag (vs upstream `split-divider-color`
   annotated "Available since: 1.1.0" at `Config.zig:1086`), the fork has a
   "Port split border config" commit series (`463bb9b16` + related), and the
   issue-834 README inventory lists them. The design and the built page now cite
   that evidence and make no "per the project's docs" attribution.
2. **(Optional) Color form.** Added the bare `RRGGBB` form alongside `#RRGGBB`
   and named X11 colors (matches `ghostty.5.md`).
3. **(Optional) Page count.** "81 → 82" is a build-time check, confirmed at the
   result gate, not a stated fact.

## Result

**Result:** Pass

The user-requested split-pane-border feature is documented; all criteria pass.

### What was built

`src/content/docs/split-pane-borders.mdx` (`section: Features`, `order: 2`) —
raw-HTML MDX in `prose-termsurf`: an intro framing the options as the Ghostboard
fork's addition over Ghostty (no "per the project's docs" attribution); an
**Options** table for the four options with effects + accepted values + clamped
ranges; a usage sentence; a config example (`<pre>`); and links to the config
reference (incl. the `#split-border-width` anchor), the configuration guide, and
the Features overview.

### Verification results

1. **Builds + placed** — `bun run build` 82 pages; `/docs/split-pane-borders`
   emitted; the `/docs` Features group reads **Features → Split Pane Borders**
   (orders 1, 2); `astro check` 0 errors. **Pass.**
2. **Accuracy (fork-verified)** — the four options/effects/ranges match
   `Config.zig` + the generated reference (verified at the design gate); the
   built page contains **no** "per the project"/"CLAUDE.md"/"Available since"
   attribution, and no invented options. **Pass.**
3. **macOS-accurate** — no Linux/GTK text; no overclaimed rendering specifics.
   **Pass.**
4. **Design system, zero JS, links resolve** — `prose-termsurf`; the only hex
   literals are the two sample colors inside the config `<pre>` (`#7aa2f7`,
   `#3b4261` — content, not styling); 0 `astro-island`; dead-link + anchor crawl
   = 0 broken (`#split-border-width` resolves). **Pass.**
5. **a11y** — one `<h1>` ("Split Pane Borders") → `<h2>` (Options), no skipped
   levels; descriptive link text. **Pass.**
6. **No regressions** — `gen:references --check` + `import:vt --check` exit 0;
   only the new page added; search/`/`/`/welcome`/other pages unchanged.
   **Pass.**

## Conclusion

The split pane border feature the user specifically asked for is now documented
— the four fork-added options (focused/unfocused border colors, width, unfocused
saturation) with their ranges, a config example, and links to the generated
reference — framed accurately as Ghostboard's addition over Ghostty on real fork
evidence. Next Phase-4 candidates: Browser Engines (Roamium + the engine
roadmap), the protocol refresh, and the consolidated roadmap.

## Completion Review

Independent `adversarial-reviewer` at the result gate. **Verdict: APPROVE** (no
findings). Against a fresh 82-page build the reviewer confirmed all four options
and their ranges/semantics match `Config.zig` (1089-1102, clamps 4638/4641) and
`config.md`; the built page has **no** "CLAUDE"/"per the project"/"Available
since" citation (grep clean) and the fork-addition claim is independently true
(the four carry no version tag, unlike upstream `split-divider-color` @1.1.0);
no Linux/GTK or rendering overclaim; Features → Split Pane Borders placement;
all links resolve incl. `#split-border-width` (id exists in the built config
page); one `<h1>` + ordered headings; the only hex literals are the two sample
colors inside the `<pre>`; 0 `astro-island`; `astro check` 0 errors; drift
checks exit 0; scope only the new file.
