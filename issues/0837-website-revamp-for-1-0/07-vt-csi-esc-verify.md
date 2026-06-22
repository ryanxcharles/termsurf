# Experiment 7: VT CSI + ESC subsections — verify (no claims) + mark verified (Phase 1)

## Description

The third VT verification pass. A survey of the **CSI** (29 pages) and **ESC**
(8 pages) subsections found **zero product claims** — none of the 37 pages names
Ghostty (outside upstream URLs, of which there are none here), and none has an
`## Implementation Status` section, an `unimplemented` `<VTSequence>`, an
"available since" version note, or a "supports / does not support" product
assertion. They are pure VT-spec descriptions of cursor movement, erase,
insert/delete, scroll, tab, keypad, save/restore, etc., plus `## Validation`
conformance test cases.

Because there is **nothing to rebrand**, this experiment introduces **no
unverified TermSurf claim** by construction: the pages are left byte-identical
and simply moved to the importer's `VERIFIED` set (hand-maintained), exactly as
`bs`/`cr`/`lf`/`tab` were in Exp 6 — just at the scale of two whole subsections.

This is deliberately scoped to the two claim-free subsections so the heavier
**OSC** subsection (9 of 15 pages carry real claims — version tables, the
ConEmu/GTK timeout that needs platform-aware verification) gets its own
experiment (Exp 8), followed by the top-level pages (Exp 9).

### Why "no spec re-verification" is the right scope

The user's decision was to verify **product/behavior claims** against the fork
before asserting them of TermSurf. These 37 pages make no product claims — they
describe standard VT sequence behavior. Since nothing is rebranded, no
TermSurf-specific assertion is introduced, so there is nothing to fork-verify
here beyond confirming the **absence** of claims. (Generic VT-spec accuracy is
inherited from the standard and from the unchanged upstream text; deep
per-sequence spec re-validation against the fork is out of scope and is not what
the user asked for.)

## Changes

Files in `website/`:

1. **`scripts/import-vt.ts`** — add the 29 `csi/*` and 8 `esc/*` rel-paths to
   the `VERIFIED` set (bringing it to 4 concepts + 5 control + 29 CSI + 8 ESC =
   46). No content edits — the CSI/ESC pages are unchanged.

No content, fork, nav, or schema changes.

## Verification

1. **Claim-free confirmed.** No `csi/*` or `esc/*` page contains a product
   "Ghostty" mention, an `## Implementation Status` / `## … Status` section, an
   `unimplemented` `<VTSequence>`, or an "available since" / "supports" / "does
   not support" product assertion about the terminal. (Generic VT-spec
   statements like "this sequence does nothing when …" are not product claims.)
   - **Pass:** survey holds on all 37. **Fail:** any product claim found (→ it
     would need rebrand+verify, not a silent mark-verified).
2. **No content change.** `git diff` touches only `scripts/import-vt.ts`; all 37
   pages are byte-identical.
3. **Importer coherence.** `import:vt --check` exits 0 (now skips 46 verified);
   a re-run leaves CSI/ESC pages intact (and does not re-emit them).
4. **Builds.** `bun run build` 76 pages; `astro check` 0 errors; CSI/ESC pages
   render; no dead links/fragments.
5. **No regressions.** Other pages unchanged.

A full pass marks the two claim-free VT subsections verified, leaving OSC
(Exp 8) and the top-level pages (Exp 9) as the remaining VT rebrand passes.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE** (no Required findings).
The reviewer independently checked all 37 pages (not just trusting the survey):
zero `ghostty`/`termsurf`, zero `## … Status` sections, zero `unimplemented`
`<VTSequence>`, zero "available since"/version/first-person assertions; the only
comparative statements describe **other** terminals (`rep.mdx:17-18`, `ich.mdx`,
`decscusr.mdx`) or say "either behavior is acceptable", never a self-claim. The
`VERIFIED` skip is correctly applied in write + stale + orphan paths; count 46
confirmed. Two **optional** notes, folded into implementation:

1. Update the `VERIFIED` comment — the set now holds **two categories**:
   rebranded pages (concepts/control) and **claim-free** pages verified by the
   _absence_ of claims (CSI/ESC), left byte-identical.
2. `esc/deckpam.mdx` is a "work-in-progress" stub; marking it `VERIFIED` freezes
   it from re-import. It has no product claim, so the premise holds; the freeze
   is the accepted tradeoff for all verified pages (a future re-import of an
   updated upstream stub would need it temporarily removed from `VERIFIED`).

## Result

**Result:** Pass

The 29 CSI + 8 ESC pages were added to the importer's `VERIFIED` set (now 46)
and left byte-identical; the comment was updated to document the two categories
(rebranded vs claim-free). No content/fork/nav/schema changes.

- **Claim-free confirmed** — independently verified across all 37 pages at the
  design gate (no product "Ghostty", no Status section, no `unimplemented`, no
  version/first-person assertions; comparatives are about _other_ terminals).
  **Pass.**
- **No content change** — `git status` shows only `scripts/import-vt.ts`
  modified; all 37 CSI/ESC pages byte-identical. **Pass.**
- **Importer coherence** — `import:vt --check` exits 0; a re-run wrote 18 pages
  (64 − 46 verified) and left CSI/ESC intact. **Pass.**
- **Builds** — `bun run build` 76 pages; `astro check` 0 errors. **Pass.**
- **No regressions** — other pages unchanged. **Pass.**

## Conclusion

CSI and ESC are verified claim-free and frozen from re-import — two whole
subsections cleared in one pass because they carry no product claims. 46 of 64
VT pages are now verified. Remaining VT: **OSC** (Exp 8 — the substantive pass
with version/compat tables and the ConEmu timeout needing platform-aware fork
verification) and the **top-level** pages (Exp 9: index/reference/external),
after which the VT index framing note can be removed. Then Phase 1's remaining
pieces (search, versioning, IA, deploy) and Phases 2–4.
