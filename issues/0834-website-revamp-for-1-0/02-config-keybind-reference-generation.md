# Experiment 2: Config & keybind reference generation (Phase 1)

## Description

Scope decision 2: the config-option and keybind-action references are
**auto-generated from the Ghostboard fork's source**, not hand-written. This
experiment delivers **both**, from a single fork artifact with a single parser.

### What the fork artifact actually contains (corrected after design review)

The fork emits `ghostboard/zig-out/share/ghostty/doc/ghostty.5.md` (the config
man page, ~191 KB) via its existing `mdgen` build step. Verified structure:

| Section                                 | Heading line | Entries                |
| --------------------------------------- | ------------ | ---------------------- |
| `# CONFIGURATION OPTIONS`               | 123          | **209** config options |
| `# KEYBIND ACTIONS`                     | 4248         | **85** keybind actions |
| `# FILES`, `# ENVIRONMENT`, `# BUGS`, … | 5089+        | not captured           |

So **both** references come from this one file — the man page _does_ enumerate
keybind actions (the original design wrongly claimed it did not and put a `294`
count on the config section; the real split is 209 + 85 = 294). The artifact is
already TermSurf-customized (config path `~/.config/termsurf/config`, "TermSurf
Ghostboard"). **No fork changes are needed for either reference.**

### Entry format and the grouped-bodyless-term subtlety

Each entry is a pandoc definition-list item:

```
**`option-name`**

:   first line of description
    continued body, 4-space indented (paragraphs, lists, links, code spans,
    and 8-space-indented code blocks)
```

Some options have **no body of their own** — they are bare `**`name`**` headers
with no `:` block. These are stylistic/paired variants (`font-family-bold`,
`font-family-italic`, `font-family-bold-italic`, `font-style-bold/italic/...`,
`font-variation-*`, `adjust-cell-height`, `selection-background`,
`search-background`, `search-selected-background`, `window-width`,
`window-position-y`, `clipboard-write`, …). Verified by inspection of the actual
file, a bodyless term inherits the **preceding** bodied option, not the
following one:

```
**`font-family`**            <- bodied
:   ...long description...

**`font-family-bold`**       <- bodyless  → relates to font-family (above)
**`font-family-italic`**     <- bodyless  → font-family
**`font-family-bold-italic`**<- bodyless  → font-family
**`font-style`**             <- bodied (its own description)
:   ...
```

(Confirmed again with the `font-style` / `font-style-bold|italic|bold-italic` /
`font-synthetic-style` run, and the natural pairs height→width, x→y,
foreground→background, read→write.) A naive "header immediately followed by `:`"
parser would silently **drop these 16 config options**; attaching them to the
_following_ bodied entry would **mis-attribute** them (e.g. `font-family-bold`
getting `font-style`'s text). The parser must therefore: emit every header as
its own entry, and for a bodyless term link it to the most recent **preceding**
bodied entry rather than dropping it or copying a wrong body.

### Approach: a committed-output generator (Ghostty's `sync-webdata` model)

Cloudflare Pages has **no fork checkout**, so — like Ghostty's own
`make sync-webdata` — the generator runs **locally**, reads the fork artifact,
and writes generated `.md` pages into the content collection, **committed** to
the repo. The build then consumes the committed files with no fork dependency. A
`--check` mode (regenerate to memory, diff against the committed file, exit
nonzero on drift) makes staleness detectable in CI.

**`.md`, not `.mdx`, for generated pages:** option/action bodies contain
arbitrary characters that are JSX hazards in MDX. The review confirmed every
`<…>` inside option bodies is inside a code span (`raw:<string>`, `path:<path>`,
`<table>`) and the only bare autolinks (`<https://…>`, `<m@…>`) live in the
trailing `# BUGS`/`# AUTHOR` sections that are **not** captured; there are 0
markdown tables in the captured ranges. Plain Markdown with the Experiment-1
`markdown: { syntaxHighlight: false }` setting renders these correctly. `.md`
already matches the collection glob (`**/*.{md,mdx}`), so no schema/config
change is needed.

## Changes

Files in `website/` unless noted:

1. **`scripts/gen-references.ts`** (new) — a Bun/TypeScript generator:
   - Reads the fork man page. Path: `--in <path>` arg or `GHOSTTY_DOC` env, else
     default `../ghostboard/zig-out/share/ghostty/doc/ghostty.5.md` (relative to
     `website/`). Errors clearly if missing (tells the user to build Ghostboard
     docs).
   - **Section-bounded parsing.** Captures entries only between
     `# CONFIGURATION OPTIONS` and `# KEYBIND ACTIONS` for the config page, and
     between `# KEYBIND ACTIONS` and `# FILES` for the keybind page. Everything
     before `# CONFIGURATION OPTIONS` and from `# FILES` onward is ignored.
   - **Entry parsing.** Within a section, each `**`name`**` line is an entry.
     The body is the following `:   …` definition block (the lines from `:   `
     up to the next `**`name`**` header or `# ` section heading), **de-indented
     by stripping the leading 4 columns from every body line** (so `:   text`→
     `text`, 4-space continuations→flush, 8-space code→4-space code preserved,
     6-space sub-bullets→2-space nested preserved — relative indentation kept).
     A header with no `:` block before the next header/section is a **bodyless**
     entry.
   - **Bodyless terms** are emitted with their heading plus a single line
     `See [\`<preceding-bodied-name>\`](#<preceding-bodied-name-slug>).`
     pointing at the most recent preceding bodied entry in the same section
     (never dropped, never given a wrong body). Slugs use GitHub-style slugging
     to match Astro's auto-generated heading ids.
   - **Output.** Emits two Markdown pages (files 2 and 3): a frontmatter block,
     a generated-file banner comment, a one-line lead, then per entry a
     `### \`name\`` heading followed by its (de-indented) body or the bodyless
     "See …" line. Headings give stable anchor ids for deep links.
   - **Idempotent** (same input → byte-identical output) and supports `--check`.
2. **`src/content/docs/reference/config.md`** (new, generated, committed) —
   frontmatter `title: Configuration Options`, `navLabel: Config Options`,
   `description: …generated from the TermSurf fork.`, `section: Reference`,
   `order: 2`. ~209 `### ` entries.
3. **`src/content/docs/reference/keybind-actions.md`** (new, generated,
   committed) — frontmatter `title: Keybind Actions`,
   `navLabel: Keybind Actions`, `description: …`, `section: Reference`,
   `order: 3`. ~85 `### ` entries. (The hand-written
   `reference/configuration.mdx` guide stays at `order: 1`; the fuller
   Configuration overview/reference IA split is a later IA experiment.)
4. **`package.json`** — add
   `"gen:references": "bun run scripts/gen-references.ts"`.
5. **`website/CLAUDE.md`** — document the generator: input artifact, that output
   is committed, the `--check` mode, and that it must be re-run when the fork's
   options/actions change.

No changes to the Ghostboard fork, the Astro config, the content schema, or the
nav code. The Experiment-1 substrate renders and lists the new entries
automatically.

## Verification

Run from `website/` with the fork artifact present.

1. **Generator completeness + boundaries.** `bun run gen:references` writes both
   pages. `config.md` has **209** `### ` headings (all config options, including
   the 16 grouped bodyless ones — spot-check `font-family`, `font-family-bold`,
   `font-style`, `window-width`, `clipboard-write`, `keybind`, `theme`,
   `window-decoration`). `keybind-actions.md` has **85** `### ` headings
   (spot-check `ignore`, `unbind`, `csi`, `new_split`, `crash`).
   - **Pass:** counts 209 / 85, spot-checks present on the correct page.
   - **Fail:** wrong counts or a known entry missing.
2. **Boundary correctness (no cross-contamination).** `config.md` contains
   **none** of the keybind-action-only entries (e.g. no `### \`csi\``, `###
   \`unbind\``, `###
   \`crash\``) and `keybind-actions.md`contains none of the config-only options (e.g. no`###
   \`font-family\``). This proves the `# KEYBIND ACTIONS`/`# FILES` boundaries
   are respected.
   - **Pass:** no cross-section entries. **Fail:** any leak.
3. **Bodyless terms handled, not dropped or mis-attributed.**
   `### \`font-family-bold\``is present in`config.md` and its body is the "See [`font-family`](#font-family)" link — **not** `font-style`'s
   text and **not** empty/absent.
   - **Pass:** present and linked to the correct preceding option.
   - **Fail:** dropped, empty, or linked to the wrong option.
4. **TermSurf provenance.** `config.md` references `~/.config/termsurf/config`
   (from the fork artifact, not vanilla Ghostty).
   - **Pass:** present. **Fail:** absent / says vanilla Ghostty.
5. **Idempotent + check mode.** Running the generator twice yields
   byte-identical files; `bun run gen:references --check` exits 0 when committed
   output matches.
   - **Pass:** no diff, `--check` exits 0. **Fail:** churn or false drift.
6. **Builds + renders + nav.** `bun run build` succeeds, `astro check` reports 0
   errors; `/docs/reference/config` and `/docs/reference/keybind-actions` are
   emitted; the Reference sidebar shows Configuration, then Config Options, then
   Keybind Actions; no Shiki inline styles in the generated pages.
   - **Pass:** all hold. **Fail:** build/check error, missing page, wrong nav,
     or Shiki output.
7. **No regressions.** The other 8 doc pages, `/`, and `/welcome` still build at
   their existing URLs.
   - **Pass:** unchanged. **Fail:** any regression.

A full pass means TermSurf regenerates accurate, complete config **and** keybind
references from its own fork, fulfilling scope decision 2 and establishing the
generation pattern the VT import (next experiment) and future references follow.

## Design Review

Two passes by independent `adversarial-reviewer` agents (separate contexts,
read-only) at the design gate.

**Pass 1 — REJECT.** The original design (config-only, "294 options", keybind
deferred on the false premise that the man page does not enumerate actions) was
rejected with four blocking, factual findings, all verified against
`ghostty.5.md`:

1. The man page **does** enumerate keybind actions in `# KEYBIND ACTIONS` (line
   4248, 85 actions, same format) — the premise was false.
2. The config section has **209** options, not 294 (294 = 209 config + 85
   keybind; the count conflated both sections).
3. 16 grouped **bodyless** options would be silently dropped by a naive
   "header-then-`:`" parser, or mis-attributed if linked to the following entry.
4. Verification's "≥290 of 294" threshold could only pass by committing the
   over-capture bug.

The design was rewritten in response: both references now in scope from one
artifact; correct 209/85 counts and section boundaries
(`# CONFIGURATION OPTIONS` @123 → `# KEYBIND ACTIONS` @4248 → `# FILES` @5089);
bodyless terms linked to the **preceding** bodied option; verification
thresholds set to 209/85 with explicit cross-section leak checks.

**Pass 2 — APPROVE.** A fresh reviewer independently reproduced every
load-bearing claim against the real artifacts: boundaries and counts (209/85);
all 16 bodyless terms have a correct preceding-bodied parent (none starts a
section, none is preceded only by bodyless terms — e.g. `window-width` →
`window-height`, `clipboard-write` → `clipboard-read`); 0 bodyless terms in the
keybind section; the de-indent rule is safe (every body line is ≥4-space
indented); and — the flagged highest-risk assumption — Astro **does**
auto-generate heading ids (`@astrojs/markdown-remark` runs `rehypeHeadingIds`
with `github-slugger` unconditionally, and collects code-span heading text), so
the bodyless "See [`name`](#slug)" links resolve. No blocking findings.

Three nits to apply during implementation (folded in):

- Use the **`github-slugger`** package in the generator (the same one Astro
  uses) rather than a hand-rolled slug function, so anchor targets match
  byte-for-byte.
- Make the de-indent robust for blank/short lines (clamp the strip at line
  length; do not index past the end).
- Guard the bodyless-term case where there is no preceding bodied entry in the
  section (emit no dangling `#` link), even though the real data never hits it.
