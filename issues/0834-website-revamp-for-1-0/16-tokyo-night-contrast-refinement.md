# Experiment 16: Tokyo Night contrast refinement (Phase 2)

## Description

Pays down the contrast debt recorded by Experiment 15. The Exp 15 audit found
the light "Tokyo Night Day" palette fails WCAG AA (4.5:1 normal text) for every
accent/secondary text token, and `--color-muted` fails the 3:1 floor in **both**
modes:

| Token (light)     | Old ratio vs bg | AA-text      |
| ----------------- | --------------- | ------------ |
| `foreground-dark` | 3.57            | FAIL         |
| `primary`         | 3.11            | FAIL         |
| `secondary`       | 3.33            | FAIL         |
| `accent`          | 4.26            | FAIL         |
| `success`         | 4.04            | FAIL         |
| `warning`         | 4.29            | FAIL         |
| `caution`         | 3.79            | FAIL         |
| `muted`           | 2.54            | FAIL (< 3:1) |

Dark mode passes AA-text for all text tokens **except `muted`** (2.76).

Per scope decision 4 ("keep & **refine** Tokyo Night — do not reinvent the look
or explore new palettes"), this experiment **refines** the failing tokens to
pass AA while preserving each token's hue and saturation — only lightness is
reduced (light) or raised (dark `muted`). No new colors, no hue shifts, no
structural/markup changes.

## Key decisions

1. **Refine, don't repaint.** For each failing token, hold hue + saturation
   constant (HSL) and move only lightness until it clears the AA-text threshold
   with a small safety margin (target ≥ 4.55:1). This keeps the Tokyo Night
   identity (same colors, slightly deeper) — the standard accessible-variant
   approach, squarely within "refine."
2. **Target the _darker_ background, not just the page background.** Several
   tokens render as text on **two** surfaces: the page background
   (`--color-background` #e1e2e7 light) _and_ `--color-background-dark` (#d5d6db
   light) — code blocks (`.prose-termsurf pre` is `bg-background-dark`;
   `text-primary`/`text-success`/`text-foreground-dark` spans live there) and
   callout titles (`.markdown-alert` is `bg-background-dark`; note→primary,
   important→secondary, tip→success, warning→warning, caution→caution as the
   title text). So **every light text token is tuned to ≥ 4.55:1 against the
   darker `#d5d6db`**, which guarantees ≥ AA on the lighter `#e1e2e7` too. The
   `text-success` status cell in `architecture.mdx` (on the page bg) is covered
   by the same value.
3. **Dark mode: only `muted`.** Every other dark token already passes AA-text;
   only `--color-muted` (#565f89, 2.76) fails. In dark mode `muted` text sits on
   the **lighter** `--color-background` (#1a1b26) — the harder surface for light
   text — so tune it to ≥ 4.55:1 there (it then clears the darker `#16161e`
   too). No other dark token changes.
4. **Non-text uses keep passing.** `--color-accent` also serves the focus ring
   (non-text, needs 3:1) and the `.skip-link` background (its text is
   `--color-background`, needs 4.5:1). The refined accent clears both
   comfortably (5.20:1 vs bg). `--color-border`/`--color-background*` are not
   text and are unchanged. The `VTSequence.astro` diagram is also covered: its
   `dt` (muted) and param `dd` (accent) text sit on the page background (5.11 /
   5.20), and the `.vtsequence-unimplemented` badge uses `secondary` as a
   _background_ with background-colored text, which the refinement _improves_
   (3.33 → 5.17).
5. **The `/welcome` scene IS affected — and is explicitly out of audit scope
   (review correction).** `WelcomePage.tsx` uses the Tailwind utilities
   `text-foreground-dark` and `text-accent`, which derive from these `@theme`
   tokens, so deepening the light values changes the welcome page too — it is
   **not** "untouched." The welcome modal renders on a near-black surface
   (`bg-black/85`); because `welcome.astro`'s `class="dark"` is **inert**
   (`style.css` has no `.dark` selector — theme switching is
   `@media (prefers-color-scheme: dark)` only), in the default light-OS case it
   shows the **light** tokens on black. Deepening them _reduces_ contrast there:
   the heading (`foreground-dark`) **4.54 → 3.13** vs black (crosses below AA),
   and the accent links (`text-sm`, normal) **3.81 → 3.12** (already sub-AA
   before this change). This is a **pre-existing** welcome-page defect (light
   tokens on a black surface via an inert `class="dark"`; the accent link was
   already < AA), which a site-wide token refinement cannot fix and the standing
   rule forbids touching ("Do not modify when changing site-wide styles" applies
   to `/welcome`). **This experiment therefore explicitly excludes the
   `/welcome` modal from its AA scope** and logs the welcome-modal contrast
   issue as separate debt for a dedicated follow-up experiment (which will need
   a welcome-page change — make the always-dark scene consume the dark tokens).
   No real doc/home content surface regresses; the regression is confined to the
   one carved-out scene, documented here with numbers.

### Proposed values (verified)

All refined light values are computed to clear ≥ 4.55:1 vs `#d5d6db` (and
therefore ≥ 5.1:1 vs `#e1e2e7`); dark `muted` clears ≥ 4.55:1 vs `#1a1b26`.

| Token             | Mode  | Old       | New       | New vs bg / bg-dark |
| ----------------- | ----- | --------- | --------- | ------------------- |
| `foreground-dark` | light | `#6172b0` | `#495993` | 5.18 / 4.62         |
| `primary`         | light | `#2e7de9` | `#1359b8` | 5.16 / 4.60         |
| `secondary`       | light | `#9854f1` | `#761bec` | 5.17 / 4.61         |
| `accent`          | light | `#007197` | `#006385` | 5.20 / 4.63         |
| `success`         | light | `#587539` | `#4b6330` | 5.19 / 4.63         |
| `warning`         | light | `#8f5e15` | `#7f5313` | 5.16 / 4.60         |
| `caution`         | light | `#c64343` | `#a73333` | 5.13 / 4.57         |
| `muted`           | light | `#848cb5` | `#525a88` | 5.11 / 4.56         |
| `muted`           | dark  | `#565f89` | `#7982ab` | 4.55 / 4.79         |

(`--color-foreground` light 9.62 and all other dark tokens already pass —
unchanged.)

## Changes

Files in `website/`:

1. **`src/styles/style.css`** — in the `@theme` block, replace the eight failing
   light token values with the refined hexes above; in the
   `@media (prefers-color-scheme: dark)` block, replace `--color-muted`. No rule
   bodies, selectors, or structure change — only nine custom-property values.
2. **`website/CLAUDE.md`** — update the Design system color-token list and the
   Exp 15 contrast table to the post-refinement ratios (all text tokens now
   PASS), noting Exp 16 resolved the debt.

No markup, component, content, nav, schema, or fork changes. The `/welcome` page
is not edited, but it **consumes** `--color-foreground-dark`/`--color-accent`
via Tailwind utilities, so its rendering does change (see decision 5 — the
welcome modal is explicitly out of this experiment's AA scope and its contrast
issue is deferred to a follow-up).

## Verification

1. **AA pass, recomputed.** A script recomputes WCAG contrast for every changed
   token against **both** its backgrounds; **all** light text tokens ≥ 4.5:1 vs
   `#d5d6db` (and vs `#e1e2e7`), and dark `muted` ≥ 4.5:1 vs `#1a1b26`. The
   non-text accent uses (focus ring, skip-link bg) stay ≥ 3:1 / ≥ 4.5:1.
2. **Only values changed.** `git diff` shows exactly nine changed
   custom-property values in `style.css` (eight light, one dark) and the
   `CLAUDE.md` doc update — no selectors, no new rules, no markup.
3. **Hue preserved.** Each new value has the same HSL hue (± rounding) as its
   old value — a refinement, not a repaint (spot-check in the verification
   script).
4. **Build + checks.** `bun run build` 76 pages; `bunx astro check` 0 errors;
   `bun run gen:references --check` + `bun run import:vt --check` exit 0;
   dead-link crawl clean.
5. **No content regressions.** Layout, fonts, components, callouts, search, nav
   all unchanged (only colors deepen); dark mode unchanged except `muted`; the
   homepage and code-block token colors still render (now AA). The **only**
   contrast regression is the carved-out `/welcome` modal (decision 5),
   documented and deferred — every real doc/home surface improves to AA.

A full pass closes the "accessibility baseline" loop for all site content: the
documented contrast debt from Exp 15 is paid down and the Tokyo Night palette is
AA-compliant in both modes across docs + home. Remaining Phase-2: page templates
(article/reference/section-index) and the home/marketing treatment; plus a
follow-up for the `/welcome` modal's pre-existing on-black contrast (needs a
welcome-page change).

## Design Review

Independent `adversarial-reviewer`. **Verdict: APPROVE WITH CHANGES.** The
reviewer **independently recomputed every proposed value** with the WCAG sRGB
formula and confirmed all quoted ratios exactly (tightest: light `muted` #525a88
= 4.56 vs `#d5d6db`; dark `muted` #7982ab = 4.55 vs `#1a1b26`), confirmed **hue
drift ≤ 0.9°** and saturation held ≤ 0.5pp for every token (a genuine refine,
not a repaint — including the flagged `muted`-light and `caution`), confirmed
the non-text accent uses pass, confirmed the darker-background coverage is
complete (code blocks, alert titles, status cells, VTSequence) with no missed
darker surface, and confirmed scope is values-only with Tailwind utilities
inheriting automatically.

One **Required** finding, fixed:

- **(Required) The "`/welcome` untouched" claim was false.** `WelcomePage.tsx`
  consumes `text-foreground-dark`/`text-accent` from these tokens, and its
  `class="dark"` is inert, so the welcome modal shows light tokens on black;
  deepening them drops the heading 4.54 → 3.13 and accent links 3.81 → 3.12 vs
  black. **Resolved** by rewriting decision 5 + the Changes note: the welcome
  modal is now explicitly **out of AA scope** (it's the carved-out
  "do-not-modify" scene with a pre-existing on-black defect — the accent link
  was already sub-AA), documented with numbers, and deferred to a follow-up
  experiment that will fix the welcome page itself.

One **Optional** finding, folded in:

- **(Optional)** Added `VTSequence.astro` to the covered-surfaces note (its text
  is covered; the unimplemented badge actually improves).
