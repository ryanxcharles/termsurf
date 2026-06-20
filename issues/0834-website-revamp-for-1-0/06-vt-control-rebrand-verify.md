# Experiment 6: VT Control subsection — TermSurf rebrand + fork verification (Phase 1)

## Description

The second per-subsection VT verification pass (after Concepts, Exp 5):
**Control** (5 pages: `bel`, `bs`, `cr`, `lf`, `tab`). Same method as Exp 5 —
verify each product claim against the Ghostboard fork, rebrand confirmed claims
to TermSurf, soften/remove stale ones, and move the pages to the importer's
`VERIFIED` set (hand-maintained).

A survey shows **only `bel.mdx` has product claims** ("Ghostty"); `bs`, `cr`,
`lf`, `tab` are pure spec descriptions of the control characters (backspace,
carriage return, line feed, tab) with no product-specific assertions — they need
no rebrand, only confirmation that they contain no claim and addition to
`VERIFIED`.

`bel.mdx`'s three claims were pre-verified against the fork:

| Claim (bel.mdx)                                                           | Fork evidence                                                                                                        | Disposition                            |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| "implemented in [Ghostty]"                                                | `terminal/stream.zig:771` `.BEL => self.handler.vt(.bell, {})` — BEL is handled                                      | **Confirmed** → rebrand                |
| "most behaviors … disabled by default"                                    | `Config.zig:9055-9061` `BellFeatures`: system/audio/border = false, attention/title = true → 3 of 5 off (a majority) | **Confirmed** ("most" = 3/5) → rebrand |
| "[it] will terminate any responses with `BEL`" (OSC terminator echo-back) | `osc.zig:243-276` (`Terminator` matches the request's last byte; `0x07 => .bel`, returns `"\x07"`)                   | **Confirmed** → rebrand                |

These are core terminal / config behaviors (`src/terminal/`, `src/config/`),
platform-agnostic, so they apply to TermSurf on macOS.

**Softening (per design review):** the second claim's "most behaviors … are
disabled" rests on a bare 3-of-5 majority, and the two enabled-by-default
features (`attention`, `title`) are the most user-visible. To avoid leaning on a
60% majority, rebrand **and** soften to "several of these behaviors are disabled
by default" — accurate and not misleading.

## Changes

Files in `website/`:

1. **`src/content/docs/vt/control/bel.mdx`** — rebrand the three confirmed
   "Ghostty" product references to "TermSurf" (the `bell-features` link already
   points at `/docs/reference/config`). `bs`/`cr`/`lf`/`tab` unchanged (no
   claims).
2. **`scripts/import-vt.ts`** — add the 5 `control/*` rel-paths to the
   `VERIFIED` set so the importer no longer regenerates/`--check`s them.

No fork source changes; no nav/schema changes.

## Verification

1. **Claims fork-cited.** bel's three claims map to the fork evidence in the
   table; nothing rebranded without a citation. **Pass/Fail.**
2. **No unverified TermSurf assertion.** Built Control pages contain no product
   "Ghostty"; bel reads TermSurf; `bs/cr/lf/tab` unchanged.
3. **Importer coherence.** `import:vt --check` exits 0 (now skips 4 concepts + 5
   control = 9 verified); a re-run leaves Control pages intact.
4. **Builds + links.** `bun run build` 76 pages; `astro check` 0 errors; Control
   pages render; no dead links/fragments.
5. **No regressions.** Other pages unchanged.

A full pass leaves Control fork-verified and TermSurf-branded; remaining VT
passes: CSI → ESC → OSC → top-level.

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE** (no Required findings).
Independently confirmed: the "only `bel` has product claims" survey is correct
(`bs`/`cr`/`lf`/`tab` are pure spec — `cr`'s Validation cases are VT-conformance
tests, not product assertions); all three bel claims are fork-backed
(`stream.zig:771` BEL handled — and :772-775 also implement bs/cr/lf/tab, making
the claim-free docs the conservative choice; `Config.zig:9056-9060` 3/5 bell
features off; `osc.zig:243-276` BEL echo-back); the `VERIFIED`-set mechanism is
sound and battle-tested from Concepts. Two non-blocking items, both folded in:
soften "most" → "several" (above); add the `osc.zig:243-276` line cite (above).

## Result

**Result:** Pass

`bel.mdx`'s three fork-confirmed claims were rebranded to TermSurf (with "most"
→ "several of these behaviors are disabled by default"); `bs`/`cr`/`lf`/`tab`
were left unchanged (no product claims); all 5 `control/*` pages were added to
the importer's `VERIFIED` set.

- **Claims fork-cited** — all three per the table (BEL handled, 3/5 bell
  features off, BEL echo-back), independently re-confirmed at the design gate.
  **Pass.**
- **No unverified TermSurf assertion** — grep of `control/` shows no product
  "Ghostty" remaining (only spec text + the rebranded bel). **Pass.**
- **Importer coherence** — `import:vt --check` exits 0; a re-run wrote 55 pages
  (9 verified skipped) and left the rebranded `bel` intact. **Pass.**
- **Builds + links** — `bun run build` 76 pages; `astro check` 0 errors; bel
  renders "implemented in TermSurf" / "several of these behaviors". **Pass.**
- **No regressions** — other pages unchanged. **Pass.**

## Conclusion

Control is fork-verified and TermSurf-branded — a small pass (only `bel` carried
claims). The per-subsection pattern continues to flow: remaining VT passes are
CSI (29 pages — the largest), ESC, OSC, then the top-level pages, after which
the VT index framing note can drop. Then the rest of Phase 1 and Phases 2–4.
