+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 329: font discovery — bold/italic refinement

## Description

`Score.score` (Experiment 328) currently derives a candidate's bold/italic-ness
from the **symbolic traits** alone. Upstream **refines** those guesses with more
reliable data read from the font itself: the `head` table's `macStyle` bits and
the `OS/2` table's `fsSelection` bits (and, for variable fonts, the variation
axes — deferred here). This experiment ports the `head` + `OS/2` refinement —
roastty already has the `Head` and `Os2` parsers — so `is_bold`/`is_italic`
reflect the font's tables, not just CoreText's symbolic summary.

## Upstream behavior (`discovery.zig` `Score.score`)

```zig
var is_italic = symbolic_traits.italic;
var is_bold = symbolic_traits.bold;

// 'head' table macStyle.
if (font.copyTable("head") |head|) {
    is_bold = is_bold or (head.macStyle & 1 == 1);
    is_italic = is_italic or (head.macStyle & 2 == 2);
}
// 'OS/2' table fsSelection.
if (font.copyTable("OS/2") |os2|) {
    is_bold = is_bold or os2.fsSelection.bold;
    is_italic = is_italic or os2.fsSelection.italic;
}
// variation axes … (deferred — overwrites is_bold/is_italic for variable fonts)

self.bold = desc.bold == is_bold;
self.italic = desc.italic == is_italic;
```

The `head`/`OS/2` contributions are **`or`-ed** into the symbolic guesses (more
evidence can only turn a flag on). `macStyle` bit `0` is bold, bit `1` is
italic; `fsSelection` exposes `bold`/`italic` bits.

## Rust mapping (`roastty/src/font/discovery.rs`)

- A `copy_table(font: &CTFont, tag: &[u8; 4]) -> Option<Vec<u8>>` helper
  (replicating `Face::copy_table`:
  `font.table(u32::from_be_bytes(tag), CTFontTableOptions(0))?.to_vec()`).
- In `score`, after the symbolic `is_bold`/`is_italic`:
  ```rust
  if let Some(head) = copy_table(&font, b"head").and_then(|d| Head::from_bytes(&d).ok()) {
      is_bold |= head.mac_style & 1 == 1;
      is_italic |= head.mac_style & 2 == 2;
  }
  if let Some(os2) = copy_table(&font, b"OS/2").and_then(|d| Os2::from_bytes(&d).ok()) {
      is_bold |= os2.fs_selection.bold();
      is_italic |= os2.fs_selection.italic();
  }
  ```
  (`Head::mac_style: u16`; `Os2::fs_selection: FsSelection` with `.bold()`/
  `.italic()`.) The `self.bold = self.bold == is_bold` / `self.italic = … `
  comparisons are unchanged.

## Scope / faithfulness notes

- **Ported**: the `head` (`macStyle`) and `OS/2` (`fsSelection`) bold/italic
  refinement, OR-ed into the symbolic guesses.
- **Deferred**: the **variation-axis** derivation (`wght > 600`, `ital > 0.5`,
  `slnt <= -5` — which _overwrites_ `is_bold`/`is_italic` for variable fonts).
  That is the heaviest CoreText FFI (reading `kCTFontVariationAxesAttribute` /
  `kCTFontVariationAttribute` dictionaries) and is the next experiment; for
  non-variable fonts (the common case) `head` + `OS/2` is the full story. The
  style `exact_style`/`fuzzy_style` match and `sortMatchingDescriptors` stay
  deferred too.
- No C ABI/header/ABI-inventory change (internal Rust; the `Head`/`Os2` parsers
  already exist).

## Changes

1. `roastty/src/font/discovery.rs`: add the `copy_table` helper; add the
   `head`/`OS/2` refinement to `score`. Import `Head`, `Os2`, `CTFont` table
   access.
2. Tests (in `discovery.rs`) — scoring **resolved** Menlo candidates from
   `discover_descriptors`:
   - `score_detects_bold_variant`: among the Menlo candidates, at least one is
     detected as **bold** — i.e. some candidate `c` has
     `Descriptor { bold: true, .. }.score(&c).bold == true` **and**
     `Descriptor { bold: false, .. } .score(&c).bold == false` (which holds iff
     `is_bold` for `c` is true). Menlo ships a Bold face, so this is
     deterministic and proves the end-to-end bold detection (symbolic ∪ head ∪
     OS/2) fires.
   - `score_detects_italic_variant`: likewise, some Menlo candidate is detected
     as **italic** (Menlo ships an Italic face).
   - `score_regular_not_bold_italic`: the regular Menlo candidate (family
     `"Menlo"`, and _not_ detected bold/italic) still scores `bold`/`italic`
     correctly against a non-bold/non-italic request (the refinement does not
     spuriously flip a regular face) — i.e. there exists a candidate detected as
     neither bold nor italic.
   - (Isolating the `head`/`OS/2` contribution from the symbolic traits is not
     host-deterministic — Menlo's bold/italic faces also report symbolic
     bold/italic — so the tests assert the integrated detection; the
     `head`/`OS/2` reads themselves are covered by the existing `Head`/`Os2`
     parser tests and the review.)
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty score
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `score` refines `is_bold`/`is_italic` from the `head` `macStyle` and `OS/2`
  `fsSelection` bits, OR-ed into the symbolic guesses, faithful to upstream;
- the bold-variant, italic-variant, and regular tests pass;
- the variation-axis derivation, the style match, and the sort stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a system-font-dependent assertion is
non-deterministic on the test host (the refinement is still exercised).

The experiment **fails** if the `head`/`OS/2` bit reads or the OR-in logic
diverges from upstream, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It confirmed the slice matches upstream — the symbolic
`is_bold`/`is_italic` are refined by OR-ing `head.macStyle` bit 0 / bit 1 and
`OS/2.fsSelection` bold / italic — and that the proposed `copy_table` helper
mirrors the existing CoreText face table access and is the right way to feed the
existing `Head`/`Os2` parsers. It confirmed `Head.mac_style`,
`FsSelection::bold()`, and `FsSelection::italic()` line up with the OpenType bit
positions. It agreed that deferring the variation-axis derivation is an
acceptable, correctly-flagged partial — upstream's variable-font path
**overwrites** the accumulated bold/italic guesses, so it can matter for
variable fonts but not for the Menlo non-variable integration tests — and that
the integration tests are reasonable system-font checks even though they do not
isolate the table-derived evidence from the symbolic traits.

Review artifacts:

- Prompt: `logs/codex-review/20260603-115415-506565-prompt.md`
- Result: `logs/codex-review/20260603-115415-506565-last-message.md`
