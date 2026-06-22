# Experiment 21: Features overview (Phase 3)

## Description

A Phase-3 (Ghostty-parity) experiment adding the **Features** section. Ghostty's
docs have a Features section (color theme, shell integration, SSH, AppleScript).
TermSurf's Ghostboard fork **inherits all four**, and all are macOS-applicable —
but the site documents none of them. This experiment adds a single **Features**
overview page covering the four inherited, **fork-verified** features, each
linking to the generated config reference for exact option syntax (so the page
stays accurate and concise, deferring precise details to the fork-sourced
reference).

Each feature was verified present in the fork before writing:

- **Color themes** — the `theme` config option (`ghostty.5.md` line 630).
  `termsurf +list-themes` and the user themes dir `~/.config/termsurf/themes`
  were confirmed against the **running `termsurf` binary** in the built
  `TermSurf.app` (not just the man page, whose app-resources path text is stale
  — see decision 3).
- **Shell integration** — `shell-integration` + `shell-integration-features`
  (line 3132): working-directory inheritance, prompt marking, cursor/title/sudo
  features.
- **SSH integration** — the `ssh-env` and `ssh-terminfo` shell-integration
  features (line 3180) + the `+ssh-cache` CLI action: terminfo install and TERM
  compatibility on remote hosts.
- **AppleScript automation (macOS)** — the bundled `Ghostty.sdef` scripting
  dictionary (in `TermSurf.app/Contents/Resources/`) with
  `Sources/Features/AppleScript/*` implementations: classes
  `application`/`window`/`tab`/`terminal` and commands such as `new window`,
  `new tab`, `split`, `focus`, `input text`, `send key`, `perform action`.

## Key decisions

1. **One overview page `features.mdx`, `section: "Features"`, `order: 1`.**
   Route `/docs/features`. `SECTION_ORDER` already lists "Features" (after
   Configuration, before Terminal API), so it slots correctly with no nav-data
   change. A single overview page now; individual features can split into their
   own pages in later experiments if depth warrants.
2. **Four fork-verified, macOS-applicable subsections** (`<h2>` each): Color
   themes, Shell integration, SSH integration, AppleScript automation. Describe
   what each feature does and how to turn it on at a useful overview depth;
   **link to the generated config reference** (`/docs/reference/config`) for the
   exact option text rather than restating it (the generated page is the
   authoritative, fork-sourced detail — Exp 2).
3. **Accuracy — defer specifics to the generated reference; don't hardcode
   man-page-verbatim details that diverge from the fork.** The fork's man page
   mixes patched ("TermSurf", `~/.config/termsurf`) and **stale inherited**
   strings — e.g. it gives the built-in themes path as
   `Ghostty.app/Contents/Resources/ghostty/themes`, but the fork actually
   installs them under **`TermSurf.app/Contents/Resources/ghostty/themes`**
   (review finding), and the terminfo is the inherited `xterm-ghostty`. To avoid
   asserting anything wrong:
   - The themes section **does not hardcode the app-bundle resources path** — it
     says built-in themes ship with the app and points users to
     `termsurf +list-themes` (verified against the running binary) and the user
     themes dir `~/.config/termsurf/themes` (verified patched). Only the fork's
     **actual** `TermSurf.app` path may be stated if a bundle path is mentioned
     at all — never the man page's `Ghostty.app` text.
   - The SSH section states terminfo/TERM behavior **generically** (SSH
     integration installs the terminal's terminfo on remote hosts and keeps TERM
     compatible) without hardcoding a specific TERM string.
   - Exact option syntax is **linked** to `/docs/reference/config`. No invented
     options.
4. **macOS-accurate; no Linux/GTK.** Themes mention the macOS resources path
   only; AppleScript is explicitly macOS automation. Omit Ghostty's Linux/GTK
   notes (scope decision 5).
5. **Design system, zero JS.** Plain MDX → `prose-termsurf`; semantic tokens
   only; links only to **built** pages (`/docs/reference/config`,
   `/docs/reference/configuration`, `/docs/reference/keybindings`). The
   AppleScript subsection lists scriptable classes/commands as prose, not a code
   dump.

## Changes

Files in `website/`:

1. **`src/content/docs/features.mdx`** — new overview page (frontmatter + the
   four subsections). Appears under a new "Features" sidebar group (between
   Configuration and Terminal API) and in the generated `/docs` index
   automatically via `getDocsNav()`.

No other files change: schema, `docs-nav.ts` (Features already in
`SECTION_ORDER`), generated references, and the fork are untouched. Page count
**78 → 79**, and a new "Features" section heading appears in the nav.

## Verification

1. **Builds + placed correctly.** `bun run build` emits `/docs/features`; total
   pages **79**. The sidebar + the generated `/docs` index show a **Features**
   group between **Configuration** and **Terminal API** (per `SECTION_ORDER`),
   containing the Features page. `bunx astro check` 0 errors.
2. **Accuracy (fork-verified).** Each of the four features documented is present
   in the fork (theme/shell-integration/SSH features in `ghostty.5.md`;
   AppleScript via the bundled `Ghostty.sdef` + `Sources/Features/AppleScript`).
   No invented options; exact option syntax is **linked** to
   `/docs/reference/config`, not restated. Crucially, **no man-page-verbatim
   path or CLI string that diverges from the fork is hardcoded** — in particular
   the page must not state `Ghostty.app/...` (the fork is `TermSurf.app`), and
   any path/command it does state matches the installed `TermSurf.app` /
   `termsurf` binary; TERM/terminfo is described generically. Spot-check each
   subsection against the fork.
3. **macOS-accurate.** No Linux/GTK text; AppleScript framed as macOS
   automation.
4. **Design system, zero JS, links resolve.** `prose-termsurf`; no hardcoded
   hex; no `<astro-island>` beyond the inherited Pagefind search; dead-link
   crawl over `/docs/features` = 0 broken (every cross-link resolves).
5. **a11y.** Exactly one `<h1>` ("Features"), ordered `<h2>`s (no skipped
   levels); descriptive link text.
6. **No regressions.** `gen:references --check` + `import:vt --check` exit 0;
   the new "Features" group/entry is the only nav addition;
   search/`/`/`/welcome`/ other pages unchanged.

A full pass adds the Features section at Ghostty parity (macOS-applicable,
fork-verified). Next Phase-3 candidates: Help (terminfo, macOS platform notes,
synchronized output) and Sponsor; individual feature pages can be split out
later if needed.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE WITH CHANGES.** The
reviewer independently confirmed all four features against the **installed**
fork (not just the man page): `theme` (man page 630 / `config.md:513`) with
`termsurf +list-themes` exit 0 on the built `TermSurf.app` binary;
`shell-integration` + features cursor/sudo/title/ssh-env/ssh-terminfo/path and
behaviors (man page 3131 / `config.md:2910`); SSH `ssh-env`/`ssh-terminfo` +
`+ssh-cache` (3180–3192); AppleScript classes (application/window/tab/terminal)
and commands (new window/tab, split, focus, input text, send key, perform
action) in `Ghostty.sdef`, which is bundled at
`TermSurf.app/Contents/Resources/Ghostty.sdef`. Placement confirmed (`Features`
in `SECTION_ORDER` between Configuration and Terminal API; entry id →
`/docs/ features`; single-item group renders normally); all three link targets
exist; page-count 78 → 79 consistent. One **Required** + two follow-ons, folded
in:

1. **(Required) Stale resources path.** The man page (and generated reference)
   give the built-in themes path as
   `Ghostty.app/Contents/Resources/ghostty/ themes`, but the fork actually ships
   them at **`TermSurf.app/Contents/ Resources/ghostty/themes`** (534 entries,
   verified). Decision 3 now forbids restating the man page's `Ghostty.app`
   path: the themes section doesn't hardcode the bundle path (it uses
   `termsurf +list-themes` + the user themes dir), and only the fork's actual
   `TermSurf.app` path may appear if any bundle path is stated.
2. **(Optional) Broadened accuracy gate.** Verification 2 now asserts no
   man-page-verbatim path/CLI that diverges from the fork is hardcoded, and any
   path/command matches the installed `TermSurf.app`/`termsurf`.
3. **(Nit) Provenance.** The intro now notes `termsurf +list-themes` + themes
   dir were confirmed against the running binary, not just the man page.

## Result

**Result:** Pass

The Features section is added at Ghostty parity (four fork-verified,
macOS-applicable features); all criteria pass.

### What was built

`src/content/docs/features.mdx` (`section: Features`, `order: 1`) — raw-HTML MDX
in `prose-termsurf` with four `<h2>` subsections: **Color themes** (`theme`,
`termsurf +list-themes`, `~/.config/termsurf/themes`, light/dark — no app-bundle
path hardcoded; links to the config reference); **Shell integration**
(auto-injection + working-dir inheritance, prompt marking, the `cursor`/`title`/
`sudo`/`path` features; links to config + Keybindings); **SSH integration**
(`ssh-env`/`ssh-terminfo`, generic terminfo/TERM behavior,
`termsurf +ssh-cache`; links to config); **AppleScript automation (macOS)** (the
bundled scripting dictionary — `application`/`window`/`tab`/`terminal` +
`new window`/`new tab`/ `split`/`focus`/`close`,
`input text`/`send key`/mouse/`perform action` — with an `osascript` example;
links to Keybind Actions).

### Verification results

1. **Builds + placed** — `bun run build` 79 pages; `/docs/features` emitted; the
   `/docs` index section order is Overview → Configuration → **Features** →
   Terminal API → Components → Protocol (per `SECTION_ORDER`); `astro check` 0
   errors. **Pass.**
2. **Accuracy (fork-verified)** — all four features confirmed in the fork at the
   design gate against the installed app; the built page hardcodes **no**
   divergent man-page string: greps for `ghostty.app`, `gtk`, `linux`,
   `xterm-ghostty` are all empty; exact option syntax is linked to
   `/docs/reference/config`. **Pass.**
3. **macOS-accurate** — no Linux/GTK text; AppleScript framed as macOS
   automation. **Pass.**
4. **Design system, zero JS, links resolve** — `prose-termsurf`; no hardcoded
   hex; 0 `astro-island`; dead-link crawl over `/docs/features` = 0 broken (all
   cross-links resolve). **Pass.**
5. **a11y** — one `<h1>` ("Features") → four ordered `<h2>`s, no skipped levels;
   descriptive link text. **Pass.**
6. **No regressions** — `gen:references --check` + `import:vt --check` exit 0;
   only `features.mdx` added (a new "Features" nav group is the sole nav
   addition); search/`/`/`/welcome`/other pages unchanged. **Pass.**

## Conclusion

The Features section now exists at Ghostty parity — color themes, shell
integration, SSH integration, and AppleScript automation — documented at
overview depth, every feature confirmed present in the actual fork,
macOS-scoped, with exact option syntax deferred to the fork-sourced generated
config reference and no stale `Ghostty.app`/TERM strings carried over. Next
Phase-3 candidates: Help (terminfo, macOS platform notes, synchronized output)
and Sponsor.

## Completion Review

Independent `adversarial-reviewer` at the result gate. **Verdict: APPROVE** (no
findings). Against a fresh 79-page build the reviewer verified every claim with
man-page/sdef line cites: themes (`theme` 630, light/dark 633, `+list-themes`
657, user dir 647 — no app-bundle path, never "Ghostty.app"); shell integration
(shells 3155; working-dir 3137, prompt marking + `jump_to_prompt` 3140,
no-confirm 3142, resize 3145; features cursor/title/sudo/path; default
`detect`); SSH (`ssh-env` 3180, `ssh-terminfo` 3187, `+ssh-cache` 3192; TERM
described generically — `xterm-ghostty` absent from the page); AppleScript (sdef
classes application/window/tab/terminal + the named commands; `perform action`
runs a keybind action; `osascript` example targets "TermSurf"). Confirmed: zero
`ghostty.app`/`gtk`/`linux`/`xterm-ghostty` on the page; Features group placed
between Configuration and Terminal API; all four links resolve; one `<h1>` +
four ordered `<h2>`; no hex; 0 `astro-island`; `astro check` 0 errors; drift
checks exit 0; scope only `features.mdx`.
