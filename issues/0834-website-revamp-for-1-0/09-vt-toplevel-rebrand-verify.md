# Experiment 9: VT top-level pages — TermSurf rebrand + finalize (Phase 1)

## Description

The final VT verification pass: the 3 **top-level** pages (`index`, `reference`,
`external`). After this, all 64 VT pages are verified, so the interim "being
verified against TermSurf" framing note on the index is no longer true and is
updated (the MIT attribution stays).

Their claims are mostly **project-voice / descriptive** (about the product and
the docs), not specific behavioral assertions like OSC's version tables — so
most are safe TermSurf rebrands. The few **verifiable** claims are checked
against the fork.

### Claim inventory and dispositions

| Page        | Text                                                                                                                                                                                                                                     | Disposition                                                                                                    |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `index`     | description "applications that run in Ghostty"; "an overview of Ghostty's control sequence support"                                                                                                                                      | descriptive → rebrand to TermSurf                                                                              |
| `index`     | framing note: "…product-specific details that still name Ghostty are being verified against TermSurf"                                                                                                                                    | **update** — all VT is now verified; drop the "being verified" caveat, **keep** the MIT attribution to Ghostty |
| `reference` | description + "VT sequences that Ghostty supports"; "Ghostty supports many more sequences than are listed here"                                                                                                                          | descriptive/true (the reference is partial; TermSurf inherits the full VT engine) → rebrand                    |
| `external`  | "Ghostty inherits the rich tradition…"; "If Ghostty's behavior deviates… we treat them as bugs… fixed according to spec"; "Ghostty intentionally deviates"; "protocols that Ghostty supports" + a table listing **OSC 8** and **OSC 21** | project-voice → rebrand; the supported-protocols table is **verifiable**                                       |

**Verifiable claim (external table):** OSC 8 (hyperlinks) and OSC 21 (Kitty
color protocol) are listed as supported. Confirm against the fork: OSC 8 →
`osc.zig`/`hyperlink.zig` (hyperlink parser); OSC 21 → `osc.zig` routes `.@"21"`
to `parsers.kitty_color.parse`. Both confirmed-supported → rebrand the table
intro.

The project-voice statements ("inherits the tradition", "we treat deviations as
bugs", "TermSurf's control sequence support") are descriptive and true for
TermSurf as a Ghostty-fork terminal — no specific fork value to verify; rebrand.

## Changes

Files in `website/`:

1. **`src/content/docs/vt/{index,reference,external}.mdx`** — rebrand
   Ghostty→TermSurf in the descriptive/voice text and the external supported-
   protocols intro; **update the index framing note** to drop the "being
   verified" interim language while keeping the MIT attribution; keep upstream
   URLs and `[ConEmu]` links.
2. **`scripts/import-vt.ts`** — add the 3 top-level rel-paths (`index.mdx`,
   `reference.mdx`, `external.mdx`) to `VERIFIED` (now **64/64** — the whole VT
   corpus is hand-maintained/verified; the importer regenerates nothing, but
   stays as the documented provenance tool + `--check` guard).

No fork source changes; no nav/schema changes.

## Verification

1. **Claims dispositioned.** The external OSC 8 / OSC 21 table is fork-confirmed
   (hyperlink parser; `osc.zig` `.@"21"` → `kitty_color`); descriptive/voice
   statements rebranded; the index framing note keeps the MIT attribution and
   drops "being verified".
2. **No product "Ghostty" left in any VT page** except the upstream attribution
   link/credit (the `[Ghostty](https://ghostty.org)` in the index note and
   `ghostty-org` URLs).
3. **Importer coherence.** `import:vt --check` exits 0 (all 64 verified, 0
   mechanical); a re-run writes 0 pages and leaves everything intact.
4. **Builds + links.** `bun run build` 76 pages; `astro check` 0 errors; no dead
   links/fragments across VT.
5. **No regressions.** Other pages unchanged.

A full pass completes **scope decision 1** — the entire VT/Terminal API
reference is reused-and-extended from Ghostty's MIT docs, fork-verified, and
TermSurf-branded. Phase 1 then has: search (Pagefind), versioning posture, the
full IA/sitemap, and the deploy/`deploy.sh` cleanup.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE** (no Required findings).
Confirmed: the external table's OSC 8 + OSC 21 are fork-supported (`osc.zig:785`
`.@"8"`→hyperlink, `:789` `.@"21"`→kitty_color; both parsers exist); the
inventory is complete (a repo-wide grep finds Ghostty only in these 3 pages +
the already-verified `concepts/cursor` upstream URL); "TermSurf supports many
more sequences than listed" is true (the fork carries parsers/modes beyond the
partial reference); the voice rebrands and framing-note update are sound. One
**optional** coherence fix, folded into implementation: the importer's
`ATTRIBUTION` constant still says "being verified" and becomes **dead** once
`index.mdx` joins `VERIFIED` (never injected) — update its text to the finalized
wording so the importer stays coherent.

## Result

**Result:** Pass

The 3 top-level pages were rebranded to TermSurf, the index framing note
finalized (MIT attribution kept, "being verified" dropped), the importer's dead
`ATTRIBUTION` constant updated to match, and all 3 added to `VERIFIED` (now
**64/64**). This **completes scope decision 1** — the entire VT/Terminal API
reference is fork-verified and TermSurf-branded.

### Dispositions

- `index` — description "…run in TermSurf"; "TermSurf's control sequence
  support"; framing note keeps the `[Ghostty](https://ghostty.org)` MIT credit,
  drops the interim "being verified" clause.
- `reference` — description + "VT sequences that TermSurf supports" + "TermSurf
  supports many more sequences than listed" (true — partial reference, full
  inherited engine).
- `external` — all 5 mentions rebranded; the supported-protocols table intro now
  "protocols that TermSurf supports", with OSC 8 (`osc.zig:785`→hyperlink) and
  OSC 21 (`:789`→kitty_color) fork-confirmed.

### Verification results

1. **Claims dispositioned** — OSC 8/21 fork-confirmed; descriptive/voice
   rebranded; framing note finalized. **Pass.**
2. **No product "Ghostty" left** — a grep of `src/content/docs/vt/` returns only
   the upstream attribution (`ghostty.org` link / "Ghostty fork" credit on the
   index note) and the `ghostty-org` issue URL in `concepts/cursor`. **Pass.**
3. **Importer coherence** — `import:vt --check` exits 0; a re-run wrote **0**
   pages (all 64 verified) and changed nothing. **Pass.**
4. **Builds + links** — `bun run build` 76 pages; `astro check` 0 errors; a
   head+body crawl of all VT pages finds **zero** dead links/fragments.
   **Pass.**
5. **No regressions** — other pages unchanged. **Pass.**

## Conclusion

**Scope decision 1 is complete.** All 64 VT / Terminal API pages are imported
from Ghostty's MIT docs, **fork-verified** claim-by-claim, and TermSurf-branded;
the only Ghostty references remaining are upstream attribution. The five-pass
verification (Concepts → Control → CSI+ESC → OSC → top-level) corrected several
genuinely stale/false upstream claims along the way (dynamic-color count, ConEmu
timeout platform, version cells) — the per-claim fork tracing earned its cost.
The VT reference is the largest single body of the new docs.

Phase 1's remaining pieces: **search** (Pagefind), **versioning** posture, the
full **IA/sitemap**, and the **deploy/`deploy.sh`** cleanup. Then Phase 2
(design system), Phase 3 (Ghostty-parity About/Install/Features/Help/Sponsor),
and Phase 4 (TermSurf-specific UX/web-TUI/protocol/engines/roadmap).
