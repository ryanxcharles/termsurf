# Experiment 20: Keybindings overview (Phase 3)

## Description

A Phase-3 (Ghostty-parity) experiment that fills a real Configuration-section
gap. Ghostty's docs document keybindings in **two** parts: a **keybindings
overview** (the `keybind = trigger=action` syntax — modifiers, physical keys,
sequences/leader keys, prefixes) and an **auto-generated action reference** (the
list of actions). TermSurf already ships the generated action reference
(`/docs/reference/keybind-actions`, Exp 2) but has **no overview** — nothing
explains _how to write_ a binding, only the list of actions. This experiment
adds the **Keybindings** overview page.

This is the **terminal** (Ghostboard) keybinding system inherited from Ghostty —
distinct from the `web` TUI's own vim-style mode keys (documented on the Web TUI
component page). The overview makes that boundary explicit and links to the
generated action reference for the action list.

## Key decisions

1. **New page `reference/keybindings.mdx`, in the Configuration section,
   `order: 2.5`.** The Configuration section is currently Configuration (1) →
   Config Options (2, generated) → Keybind Actions (3, generated). Insert the
   overview at **2.5** so the order reads **Configuration → Config Options →
   Keybindings → Keybind Actions** — overview before the action reference, like
   Ghostty. `2.5` avoids renumbering the **generated** pages (whose frontmatter
   is produced by `gen-references.ts` and must not be hand-edited). Route
   `/docs/reference/keybindings`. `navLabel: Keybindings`.
2. **Content = the trigger grammar, verified against the fork's generated man
   page** (`ghostboard/zig-out/share/ghostty/doc/ghostty.5.md`, the same
   authoritative source `gen-references.ts` parses). Cover, accurately:
   - **Format** `keybind = trigger=action`; duplicate triggers overwrite — and
     triggers are **not** unique per prefix, so `ctrl+a` and `global:ctrl+a`
     overwrite each other (man-page note, review point).
   - **Trigger** = `+`-separated keys + modifiers (e.g. `ctrl+a`,
     `ctrl+shift+b`, `up`). Modifiers: `shift`, `ctrl` (alias `control`), `alt`
     (alias `opt`/`option`), `super` (alias `cmd`/`command`). Modifiers can't
     repeat; only one key per trigger.
   - **Unicode vs physical keys** — a single codepoint (`a`) matches by
     layout-dependent unmodified codepoint (case-insensitive); **physical** keys
     (`KeyA`, snake-case `key_a`) match a physical position regardless of layout
     and take priority (a `physical:` prefix also exists to force this). And
     `catch_all` matches any otherwise-unbound key.
   - **Sequences / leader keys** — multiple triggers joined by `>` (e.g.
     `ctrl+a>n=new_window`); no length limit; quote in the shell because `>` is
     special; brief note on prefix/override behavior.
   - **Actions** — `action` or `action:param`; special forms `ignore`, `unbind`,
     `csi:`, `esc:`, `text:`; `keybind=clear` resets all bindings. Link to the
     **Keybind Actions** reference for the full action list (don't duplicate
     it).
   - **Prefixes** — `all:` (apply to all surfaces), `global:` (system-wide;
     **macOS** — note it implies `all:` and that sequences aren't allowed with
     `global:`/`all:`), `unconsumed:` (don't swallow the input), `performable:`
     (only consume if the action runs).
3. **macOS-accurate; no GTK/Linux.** The man page annotates `global:` as "1.0.0
   on macOS, 1.2.0 on GTK" — state only the macOS reality (scope decision 5);
   omit GTK/Linux availability notes entirely.
4. **Don't conflate with the `web` TUI.** A short framing note: this page is the
   **terminal**'s `keybind` config (Ghostboard); the `web` browser TUI has its
   own modal keys, documented on the **Web TUI** page
   (`/docs/components/webtui`). Link there; don't restate the TUI keys here.
5. **Design system, zero JS.** Plain MDX → `prose-termsurf`; code samples use
   the existing `bg-background-dark` `<pre>` token-span style; semantic tokens
   only; links only to built pages (`/docs/reference/keybind-actions`,
   `/docs/reference/configuration`, `/docs/components/webtui`).

## Changes

Files in `website/`:

1. **`src/content/docs/reference/keybindings.mdx`** — new hand-authored overview
   (frontmatter + the sections above). Picked up automatically by `getDocsNav()`
   in the Configuration group (sidebar) and the generated `/docs` index.

No other files change: the generated reference pages, `gen-references.ts`, the
schema, `docs-nav.ts`, and the fork are untouched. Page count **77 → 78**.

## Verification

1. **Builds + placed correctly.** `bun run build` emits the
   `/docs/reference/keybindings` route; total pages **78**. In the sidebar
   Configuration group and the `/docs` index, the order reads **Configuration →
   Config Options → Keybindings → Keybind Actions** (orders 1 / 2 / 2.5 / 3).
   `bunx astro check` 0 errors.
2. **Accuracy (verified against the fork man page).** Every syntax claim on the
   built page matches `ghostboard/zig-out/share/ghostty/doc/ghostty.5.md`'s
   `keybind` section: the modifier set + aliases, physical-key priority,
   `>`-sequences, the special actions (`ignore`/`unbind`/`csi:`/`esc:`/`text:`/
   `clear`), and the four prefixes (`all:`/`global:`/`unconsumed:`/
   `performable:`). No invented options. Spot-check each against the man page.
3. **macOS-accurate.** No GTK/Linux text; `global:` is described for macOS only.
4. **No TUI conflation.** The page states it covers the terminal `keybind`
   config and links the `web` TUI page for the TUI's modal keys; it does **not**
   restate the TUI keybindings.
5. **Design system, zero JS, links resolve.** `prose-termsurf`; no hardcoded
   hex; no `<astro-island>` beyond the inherited Pagefind search; dead-link
   crawl over `/docs/reference/keybindings` = 0 broken (all three cross-links
   resolve).
6. **a11y.** Exactly one `<h1>` ("Keybindings"), ordered `<h2>`/`<h3>` (no
   skipped levels); descriptive link text.
7. **No regressions.** `gen:references --check` + `import:vt --check` exit 0
   (the generated pages are untouched); the generated `/docs` index gains only
   the new Keybindings entry; search/`/`/`/welcome`/other pages unchanged.

A full pass brings the Configuration section to Ghostty parity (overview +
generated option reference + keybindings overview + generated action reference).
Next Phase-3 candidates: Features (macOS-applicable, fork-verified), Help, and
Sponsor.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE** (no Required findings).
The reviewer cross-checked **every** syntax claim in decision 2 against the fork
man page (`ghostty.5.md` keybind section, lines ~1778–1999) and confirmed each
is supported: `trigger=action` + overwrite (1780); `+`-separated keys/modifiers
(1784); the modifier set + aliases (1835); no-repeat/one-key (1846, 1851);
Unicode layout-dependent + case-insensitive matching (1787–1801); physical
`KeyA`/`key_a` priority (1809–1826); `catch_all` (1828); `>`-sequences with no
limit + shell-quote caveat (1853–1886); actions + `ignore`/`unbind`/
`csi:`/`esc:`/`text:` (1891–1910); `keybind=clear` (1925); the four prefixes
(1932–1976) and sequences disallowed for `global:`/`all:` (1888). Confirmed
macOS accuracy (omit the GTK `global:` annotation, decision 5), no `web`-TUI
conflation (webtui.mdx is a separate modal-key system; link resolves), and the
`order: 2.5` placement sorts Configuration → Config Options → Keybindings →
Keybind Actions without touching the generated files. Two **Optional**
completeness notes, folded in:

1. Note that triggers are not unique per prefix (`ctrl+a` vs `global:ctrl+a`
   overwrite each other) — man page 1986.
2. Mention the `physical:` trigger prefix exists (man page 1901), alongside the
   `KeyA`/`key_a` physical-key treatment.

## Result

**Result:** Pass

The Keybindings overview is added and the Configuration section reaches Ghostty
parity; all criteria pass.

### What was built

`src/content/docs/reference/keybindings.mdx` (`section: Configuration`,
`order: 2.5`) — raw-HTML MDX in `prose-termsurf`: a framing note (terminal
bindings, not the `web` TUI, with a link to the Web TUI page); **Format**
(`keybind = trigger=action`, overwrite + not-unique-per-prefix); **Triggers**
(`+`-separated; the modifier set + aliases; no-repeat/one-key); **Unicode vs.
physical keys** (layout-dependent codepoints, `KeyA`/`key_a` priority, the
`physical:` prefix, `catch_all`); **Sequences** (`>` leader keys, indefinite
wait, shell-quoting, prefix override); **Actions** (`action:param`,
`ignore`/`unbind`/`csi:`/`esc:`/`text:`, `keybind = clear`, link to the action
reference); **Prefixes** (`all:`/`global:`/`unconsumed:`/`performable:`, macOS
`global:`, sequences disallowed for `global:`/`all:`).

### Verification results

1. **Builds + placed** — `bun run build` 78 pages; `/docs/reference/keybindings`
   emitted; the `/docs` Configuration group reads **Configuration → Config
   Options → Keybindings → Keybind Actions** (orders 1 / 2 / 2.5 / 3);
   `astro check` 0 errors. **Pass.**
2. **Accuracy** — every syntax claim matches the fork man page (`keybind`
   section); no invented options (verified at design gate per-line +
   spot-checked in the built page). **Pass.**
3. **macOS-accurate** — built page has zero "gtk"/"linux" text; `global:` is
   described for macOS only. **Pass.**
4. **No TUI conflation** — the page declares it covers the terminal `keybind`
   config and links the Web TUI page for the TUI's modal keys; no TUI keys
   restated. **Pass.**
5. **Design system, zero JS, links resolve** — `prose-termsurf`; no hardcoded
   hex; 0 `astro-island`; dead-link crawl over the page = 0 broken (all
   cross-links resolve). **Pass.**
6. **a11y** — one `<h1>` ("Keybindings") → ordered `<h2>`/`<h3>` (Format,
   Triggers[Modifiers, Unicode vs. physical], Sequences, Actions, Prefixes), no
   skipped levels; descriptive link text. **Pass.**
7. **No regressions** — `gen:references --check` + `import:vt --check` exit 0
   (generated pages untouched); only `keybindings.mdx` added; search/`/`/
   `/welcome`/other pages unchanged. **Pass.**

## Conclusion

The Configuration section now matches Ghostty's structure: a configuration
overview, the generated option reference, this keybindings overview, and the
generated action reference — with the binding grammar fully and accurately
documented from the fork's own man page, macOS-scoped, and cleanly separated
from the `web` TUI's modal keys. Next Phase-3 candidates: Features
(macOS-applicable — theme, shell integration, SSH, AppleScript — each
fork-verified), Help (terminfo, macOS notes, synchronized output), and Sponsor.

## Completion Review

Independent `adversarial-reviewer` at the result gate. **Verdict: APPROVE WITH
CHANGES.** The reviewer independently cross-checked all 21 syntax claims against
the man page keybind section (1778–1993) and found every one supported with no
invented/misstated options; confirmed macOS accuracy (zero "gtk"/"linux" text,
`global:` macOS-only), no `web`-TUI conflation, the Configuration order
(Configuration → Config Options → Keybindings → Keybind Actions),
`id="prefixes"`

- all cross-links resolve, one `<h1>` with ordered h2/h3, no hardcoded hex, 0
  `astro-island`, 78 pages, `astro check` 0 errors, drift checks exit 0, and
  scope limited to the new file. One **Required** finding, fixed:

* **(Required) Record/artifact mismatch.** The design review committed to
  mentioning the `physical:` trigger prefix and the Result listed it as
  documented, but the MDX omitted it. **Resolved** by adding the `physical:`
  clause to the Unicode-vs-physical paragraph (man page ~1901); rebuilt — the
  prefix now appears on the page, matching the record.
