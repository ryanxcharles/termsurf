# Experiment 5: VT Concepts subsection — TermSurf rebrand + fork verification (Phase 1)

## Description

Experiment 4 imported the full VT corpus with product claims **upstream-
attributed** (still naming Ghostty). This experiment delivers the user's
decision — **rebrand to TermSurf and verify every product claim against the
Ghostboard fork** — for the **Concepts** subsection (the first of the
per-subsection passes: Concepts → Control → CSI → ESC → OSC → top-level).

Concepts has 4 pages: `colors`, `cursor`, `screen`, `sequences`. Their product
claims (from the committed pages) and the fork source each must be checked
against:

| Page      | Claim                                                                                                            | Verify against                                                      |
| --------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| colors    | 256-color palette + **5 special colors**                                                                         | `terminal/color.zig` (`Special` enum)                               |
| colors    | **"supports three" dynamic colors, queried/modified with OSC 10-12**                                             | `terminal/color.zig` (`Dynamic` enum)                               |
| colors    | own color parser; uses the X11/X color-name database                                                             | `terminal/x11_color.zig`, `color.zig`                               |
| colors    | "does not yet recognize 16-bit color channels" (with `#` syntax)                                                 | `terminal/color.zig` (`#` parse path)                               |
| cursor    | "only supports a single cursor at any given moment"                                                              | `terminal/cursor.zig`, `Screen.zig`                                 |
| sequences | SOS/PM "ignored"; APC "only one supported"; non-numeric **OSC** ids unsupported; OSC BEL-termination + echo-back | `terminal/Parser.zig`, `apc.zig`, `dcs.zig`, `osc.zig`, `kitty.zig` |
| screen    | (stub — `# Screen` + "TODO"; no claims)                                                                          | —                                                                   |

The design reviewer pre-checked several of these against the fork (to be
re-confirmed during implementation): **5 special colors** is _confirmed_
(`color.zig` `Special`, OSC 4 idx 256-260); **"three dynamic colors / OSC 10-12"
is stale** — the fork's `Dynamic` enum has **10** entries (OSC 10-19), so this
softens; **"16-bit channels not recognized" is false** — the fork's `#` parser
handles `#rrrrggggbbbb` (16-bit), so this is removed/corrected; **"APC only one
supported" is stale** — the fork has a `glyph` APC alongside `kitty`, so this
softens.

These are **core terminal** behaviors in `src/terminal/` (platform-agnostic), so
unlike Exp 4's flagged GTK-only timeout they apply to TermSurf on macOS — but
each is still confirmed against the fork before rebranding, per the user
decision.

## Key decisions

1. **Per-claim verification, then rebrand.** For each claim above, read the
   cited fork source and record the finding (file:line) in the Result.
   - **Confirmed** (fork behaves as the claim says): rebrand "Ghostty" →
     "TermSurf".
   - **Differs / unconfirmable**: soften to spec-neutral language or remove the
     specific assertion — do **not** assert it of TermSurf. Generic terminal
     statements that don't name a product (e.g. "only supports a single cursor")
     stay as-is if the fork confirms them; otherwise softened.
2. **Keep upstream references.** External `github.com/ghostty-org/...` URLs
   (`cursor.mdx`) stay — they point at the upstream project. Example
   window-title text (`sequences.mdx`'s `OSC 2 ; 👻 Ghostty 👻 ST`) is rebranded
   to TermSurf (it's illustrative text, not an upstream reference).
3. **Concepts becomes hand-maintained (importer exclusion).** Per Exp 4 decision
   7, verified pages are no longer regenerated. Add a `VERIFIED` set to
   `scripts/import-vt.ts` containing the 4 `concepts/*` page rel-paths; the
   importer **skips them in both** the write loop **and** the `--check`
   comparison — including the **orphan scan** (the existing `--check` flags any
   committed VT file not in `outputs` as orphaned; verified pages are removed
   from `outputs`, so they must be explicitly skipped in the orphan loop too, or
   `--check` would wrongly report them orphaned and exit 1). This keeps
   `import:vt --check` covering only the still-mechanical (unverified) pages.
   The VT index framing note stays until all subsections are verified.
4. **Adversarial accuracy gate.** After editing, a per-file adversarial accuracy
   review checks each Concepts page: every retained claim is fork-cited, no
   unverified TermSurf assertion remains, voice is TermSurf, links still
   resolve. Findings fixed before the result commit.

## Changes

Files in `website/` unless noted:

1. **`scripts/import-vt.ts`** — add a `VERIFIED` set containing the 4
   `concepts/*` page ids; the importer skips them on write and `--check`.
2. **`src/content/docs/vt/concepts/{colors,cursor,sequences}.mdx`** —
   fork-verified TermSurf rebrand (claims confirmed/softened per decision 1);
   `screen.mdx` left as the stub (no claims).
3. **(Result only)** the fork-verification findings table (claim → fork
   file:line → confirmed/softened) recorded in this experiment file.

No Ghostboard fork _source_ changes (read-only verification). No nav/schema
changes (Exp 4 built those). No other subsections touched.

## Verification

Run from `website/`.

1. **Every Concepts claim is fork-cited.** The Result records, for each claim in
   the table above, the fork file:line and a confirmed/softened disposition. No
   claim is rebranded to TermSurf without a citation.
   - **Pass:** all claims cited + dispositioned. **Fail:** any unbacked claim.
2. **No unverified TermSurf assertion.** Built Concepts pages contain no
   TermSurf product claim that isn't fork-confirmed; softened claims read
   spec-neutral; the only "Ghostty" left is the upstream
   `github.com/ghostty-org` URL in `cursor`.
   - **Pass:** holds. **Fail:** any unverified TermSurf claim or stray product
     "Ghostty".
3. **Importer coherence.** `import:vt --check` exits 0 (it now skips the 4
   verified Concepts pages); re-running `import:vt` does **not** overwrite them.
   - **Pass:** `--check` 0, concepts untouched by a re-run. **Fail:** drift or
     overwrite.
4. **Builds + links.** `bun run build` (still 76 pages) and `astro check` (0
   errors); the Concepts pages render; a head+body crawl finds no dead internal
   links/fragments on them.
   - **Pass:** all hold. **Fail:** build/check error or dead link.
5. **No regressions.** Other VT pages, non-VT pages, `/`, `/welcome` unchanged.
   - **Pass:** unchanged. **Fail:** any regression.

A full pass makes the Concepts subsection an accurate, fork-verified TermSurf
reference and establishes the per-subsection verification pattern for Control →
CSI → ESC → OSC → top-level.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE WITH CHANGES** — approach
sound (claim enumeration complete; `src/terminal/**` is the right source for
Concepts, no `apprt/` dependence so the Exp-4 GTK lesson doesn't bite), all
three findings fixed:

1. **(Required) `--check` orphan scan.** Excluding verified pages from `outputs`
   would make the orphan loop flag them as orphaned and fail `--check`. Fixed in
   decision 3: skip `VERIFIED` in **both** the staleness and orphan loops.
2. **(Required) colors row conflation.** Split the "256/5-special" claim
   (confirmed) from the "3 dynamic colors / OSC 10-12" claim (stale — fork
   `Dynamic` has 10, OSC 10-19), so the false embedded assertion can't ride
   along on a confirmed paragraph. Table now has separate rows.
3. **(Optional) sequences label.** The non-numeric-id claim is **OSC**
   (`sequences.mdx:128`), not APC/DCS — relabeled, verify against `osc.zig`.

The reviewer also pre-confirmed (to re-verify in implementation): 5 special
colors confirmed; "3 dynamic / OSC 10-12" stale (→10/OSC 10-19); "16-bit not
recognized" false (fork parses `#rrrrggggbbbb`); "APC only one" stale (kitty +
glyph). These corroborate that the soften-on-mismatch policy is doing real work.
