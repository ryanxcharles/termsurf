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

# Experiment 253: `Face::get_metrics` — assemble `FaceMetrics` from a CoreText font

## Description

The capstone of the font-metrics path: assemble a `FaceMetrics` from a real
CoreText font and feed it to the already-ported `Metrics::calc`, producing a
full `Metrics` end-to-end. This is **pure logic over the proven FFI** — the
table copy (250), the scalar accessors (251), and the glyph-measurement
accessors (252) — so no new FFI and no new dependencies. It is a faithful port
of upstream `Face.getMetrics` (`font/face/coretext.zig`).

### Upstream behavior (`getMetrics`)

1. Read `head` (with a `bhed` fallback — bitmap fonts use a byte-identical
   `bhed`), `post`, `OS/2`, `hhea` via `copyTable` + the parsers; a missing or
   unparseable table is `null`.
2. `units_per_em` = `head.unitsPerEm` or the CTFont's `units_per_em()`.
   `px_per_em` = the CTFont `size()`. `px_per_unit = px_per_em / units_per_em`.
3. **Vertical metrics** `(ascent, descent, line_gap)` fallback chain:
   - no `hhea` → `(ascent(), -descent(), leading())` (CoreText values, already
     px; note `descent()` is negated since CoreText returns it positive);
   - else, with `hhea_*` in font units:
     - no `OS/2` → `hhea_* * px_per_unit`;
     - `OS/2.use_typo_metrics` → `OS/2.sTypo* * px_per_unit`;
     - `hhea.ascender != 0 || hhea.descender != 0` → `hhea_* * px_per_unit`;
     - `OS/2.sTypoAscender != 0 || sTypoDescender != 0` →
       `OS/2.sTypo* * px_per_unit`;
     - else → `(usWinAscent * ppu, -usWinDescent * ppu, 0.0)` (win descent is
       positive-down, so it is negated).
4. **Underline** from `post`: `broken = underlineThickness == 0`; position is
   `null` when `broken && underlinePosition == 0`, else
   `underlinePosition * ppu`; thickness is `null` when `broken`, else
   `underlineThickness * ppu`.
5. **Strikethrough** from `OS/2.yStrikeout*`: same broken-zero logic.
6. **Cap / ex height**: `OS/2.sCapHeight`/`sxHeight * ppu` when present, else
   the CTFont `cap_height()`/`x_height()`.
7. **Cell width / ASCII height**: map printable ASCII (U+0020..U+007E) to
   glyphs; `cell_width` = max horizontal advance; `ascii_height` = the overall
   bounding rect height.
8. **`ic_width`**: the `水` (U+6C34) glyph's advance, or `null` if the font
   lacks the glyph **or** the glyph's bounding width exceeds its advance (a
   guard against nerd-font-patched CJK fonts with butchered advances).
9. Return
   `FaceMetrics { px_per_em, cell_width, ascent, descent, line_gap, underline_*, strikethrough_*, cap_height, ex_height, ascii_height, ic_width }`.

### Rust mapping (`roastty/src/font/face/coretext.rs`)

Add `pub(crate) fn get_metrics(&self) -> FaceMetrics` to `impl Face`, porting
the above with the ported types/methods:

- Read tables:
  `self.copy_table(b"head").or_else(|| self.copy_table(b"bhed")).and_then(|b| Head::from_bytes(&b).ok())`,
  and `Post`/`Os2`/`Hhea` similarly (`b"OS/2"`, `b"post"`, `b"hhea"`). The four
  parser types are `Copy`, so the `Option<…>`s are reused across the
  computations.
- `units_per_em`, `px_per_em` (`self.size()`), `px_per_unit`.
- The vertical-metrics chain as nested `match`/`if` exactly as above; `descent`
  uses `-self.descent()` in the no-`hhea` branch (CoreText positive → negative).
- Underline / strikethrough with the broken-zero guards.
- `cap_height`/`ex_height` → `Some(…)` from `OS/2` `* ppu` or the CTFont
  accessor.
- `cell_width`/`ascii_height`:
  `let ascii: Vec<u16> = (0x20u16..0x7F).collect();` → `glyphs_for_characters` →
  `advances_for_glyphs` (max via `fold(0.0, f64::max)`) and
  `bounding_rect_for_glyphs(...).1`.
- `ic_width`: `glyphs_for_characters(&[0x6C34])[0]`; if `0` → `None`; else
  `advance = advances_for_glyphs(&[glyph])[0]`,
  `bounds_w = bounding_rect_for_glyphs(&[glyph]).0`; `None` if
  `bounds_w > advance`, else `Some(advance)`.
- Build and return the `FaceMetrics` struct literal (`cap_height`/`ex_height`/
  `ascii_height` are `Some`; the underline/strikethrough/`ic_width` may be
  `None`).

Imports added to `coretext.rs`: `crate::font::metrics::FaceMetrics` and the four
`crate::font::opentype::{head::Head, hhea::Hhea, os2::Os2, post::Post}`.

### Faithfulness and scope notes

- The fallback chains, the `descent`/`usWinDescent` sign flips, the broken-zero
  underline/strikethrough guards, and the `ic_width` bounds-vs-advance discard
  are ported exactly. Where upstream logs a warning (e.g. discarding
  `ic_width`), this port silently yields `None` (no logging subsystem yet) — a
  documented, behavior-equivalent omission.
- `FaceMetrics.cap_height`/`ex_height`/`ascii_height` are `Some` (CoreText
  always supplies a value), matching upstream's non-optional locals stored into
  the optional fields.
- No FFI changes (uses Exp 250–252 accessors), no rasterization.
- No C ABI, header, or ABI inventory changes; no new dependencies.

## Changes

1. `roastty/src/font/face/coretext.rs`: add `get_metrics` to `impl Face` + the
   imports.
2. Tests in `coretext.rs` (live CoreText, macOS):
   - `get_metrics_is_sane`: `Face::new("Menlo", 14.0).get_metrics()` →
     `px_per_em == 14.0`, `cell_width > 0.0`, `ascent > 0.0`, `descent < 0.0`
     (below the baseline), `line_gap >= 0.0`, `cap_height` and `ex_height` are
     `Some(> 0.0)` with `cap_height > ex_height`, `ascii_height` is
     `Some(> 0.0)`.
   - `get_metrics_feeds_calc`: `Metrics::calc(face.get_metrics())` →
     `cell_width > 0`, `cell_height > 0`, `cell_baseline <= cell_height`,
     `underline_thickness >= 1` (a full `Metrics` derived from a real font).

3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo test -p roastty face
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `get_metrics` reads the four tables (with the `bhed` fallback), runs the
  vertical-metrics / underline / strikethrough / cap-ex fallback chains with the
  correct sign conventions and broken-zero guards, measures `cell_width`/
  `ascii_height`/`ic_width`, and returns a faithful `FaceMetrics`;
- a live Menlo face yields sane metrics and `Metrics::calc` produces a valid
  `Metrics`;
- rasterization is cleanly deferred;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a fallback branch needs a metric the current
accessors do not expose.

The experiment **fails** if a sign convention is wrong (e.g. positive descent),
if a fallback branch is mis-ordered, if a broken-zero guard is dropped, or if
any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-202104-900193-prompt.md`
- Result: `logs/codex-review/20260602-202104-900193-last-message.md`

Codex confirmed the design is faithful to upstream `getMetrics`: the
`head`→`bhed` fallback and parser-error-to-`None`, the
`units_per_em`/`px_per_em`/`px_per_unit` derivation, and the vertical fallback
chain's exact branch ordering and sign conventions. Crucially, the **descent
handling is correct** — CoreText `descent()` is negated only in the no-`hhea`
branch, while `hhea.descender` and `os2.s_typo_descender` stay table-native
negative values `* px_per_unit` (no double-negation), and `us_win_descent` is
negated. The underline/strikethrough broken-zero guards, the cap/ex fallback,
the printable-ASCII range `0x20..0x7F`, the max-advance cell width, the overall
ASCII bounding height, and the `水` `ic_width` discard guard all match upstream.
Scope is clean.

## Result

**Result:** Pass

Added `Face::get_metrics(&self) -> FaceMetrics` to `coretext.rs` (plus the
`FaceMetrics` and `Head`/`Hhea`/`Os2`/`Post` imports), a faithful port of
upstream `getMetrics`: read the four tables (`head` with the `bhed` fallback),
derive `units_per_em`/`px_per_em`/`px_per_unit`, run the vertical-metrics
fallback chain (no-`hhea` CoreText with `-descent()`; OS/2-typo when
`use_typo_metrics`; else `hhea`; else OS/2-sTypo; else OS/2-win with
`-us_win_descent`), the underline/strikethrough broken-zero guards, the cap/ex
height fallback, the printable-ASCII `cell_width`(max advance)/`ascii_height`,
and the `水` `ic_width` with the bounds-vs-advance discard.

Tests added (2): `get_metrics_is_sane` (Menlo @ 14 → `px_per_em == 14`,
`cell_width > 0`, `ascent > 0`, **`descent < 0`**, `cap_height > ex_height > 0`,
`ascii_height > 0`) and `get_metrics_feeds_calc` (`Metrics::calc(get_metrics())`
→ `cell_width`/`cell_height > 0`, `cell_baseline <= cell_height`,
`underline_thickness >= 1`).

### Verification

```bash
cargo fmt -p roastty
cargo test -p roastty face
cargo test -p roastty
```

Observed:

- `face`: 7 passed (the FFI accessors + the two `get_metrics` tests).
- Full `roastty`: 2362 unit tests passed (2360 prior + 2 new), plus the C ABI
  harness passed.
- `cargo fmt -p roastty -- --check`: clean.
- `cargo build -p roastty`: no warnings.
- No-`ghostty`-name gates passed for `roastty/src/font` (and
  `lib.rs`/header/abi).
- `git diff --check`: clean.

No C ABI, header, or ABI inventory changes; no new dependencies; glyph
rasterization cleanly deferred.

### Completion Review

Codex reviewed the completed implementation and found **no issues** ("nothing
needs to change before the result commit").

Review artifacts:

- Prompt: `logs/codex-review/20260602-202400-673524-prompt.md`
- Result: `logs/codex-review/20260602-202400-673524-last-message.md`

Codex verified the implementation matches upstream line-by-line: the table reads
(with `head`→`bhed` and parser-error→`None`), the derivation order, and — the
critical check — that **only** CoreText `descent()` and `us_win_descent` are
negated while `hhea.descender`/`os2.s_typo_descender` stay table-native negative
`* px_per_unit` (no double-negation). The underline/strikethrough broken-zero
guards, the cap/ex `Some` fallback, the `0x20..0x7F` ASCII range, the
max-advance cell width, the bounding-rect ASCII height, and the `ic_width`
missing-glyph / bounds-vs-advance guards all match. Scope is clean; the tests
cover the live metrics (including `descent < 0`) and `Metrics::calc`.

## Conclusion

Experiment 253 succeeds — **the font-metrics path is complete end-to-end.** A
real macOS font now flows through the entire ported pipeline: `Face::new` →
`copy_table` (4 OpenType tables) → the parsers → `get_metrics` (fallback
chains + glyph measurement) → `FaceMetrics` → `Metrics::calc` → a valid
`Metrics`, all verified by live `cargo test`. Both Codex gates passed with zero
findings. This closes the `font/face/coretext.zig` metric surface that
`getMetrics` covers.

The next slice is **glyph rasterization** — the other half of the CoreText face:
create a grayscale `CGBitmapContext` sized to a glyph's bounds, draw the glyph
with `CTFontDrawGlyphs` (or `CGContextShowGlyphsAtPositions`), read back the
alpha coverage, and produce a `Glyph` + the bitmap to write into the `Atlas`
(via the already-ported `set`/`set_from_larger`). That adds the `CGContext`
surface of `objc2-core-graphics` and completes the path from a font to a
renderable glyph in the atlas. Above the face, the
`Collection`/`CodepointResolver` and the shaper remain.
