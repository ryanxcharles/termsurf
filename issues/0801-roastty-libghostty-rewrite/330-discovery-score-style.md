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

# Experiment 330: font discovery — the style match

## Description

`Score.score` (Experiments 328–329) leaves the `exact_style` and `fuzzy_style`
fields at their defaults. This experiment ports upstream's **style-string
match**: read the candidate font's style name (e.g. `"Regular"`,
`"Bold Italic"`), build a list of **desired** style strings from the request,
and score an exact (case-insensitive) match plus a fuzzy substring match. This
is the last `score()` input besides the deferred variation-axis derivation.

## Upstream behavior (`discovery.zig` `Score.score`)

```zig
const style_str = ct_desc.copyAttribute(.style_name) … or "";

const desired_styles = if (desc.style) |s| &.{s}
    else if (desc.bold)
        (if (desc.italic) &.{ "bold italic", "bold", "italic", "oblique" }
         else &.{ "bold", "upright" })
    else if (desc.italic) &.{ "italic", "regular", "oblique" }
    else &.{ "regular", "upright" };

self.exact_style = ascii.eqlIgnoreCase(style_str, desired_styles[0]);

self.fuzzy_style = @intCast(style_str.len);
for (desired_styles) |s|
    if (ascii.indexOfIgnoreCase(style_str, s) != null)
        self.fuzzy_style -|= @intCast(s.len);
self.fuzzy_style = maxInt(u8) -| self.fuzzy_style;
```

`exact_style` is whether the font's style name equals the **first** desired
style (case-insensitive). `fuzzy_style` rewards style names that are **mostly
composed** of desired substrings: it starts at the style-name length, subtracts
(saturating) the length of every desired substring that appears
(case-insensitive), then is `255 - that` (saturating) — so fewer leftover
characters ⇒ a higher score. All comparisons are byte-wise ASCII.

## Rust mapping (`roastty/src/font/discovery.rs`)

- `fn style_name(ct_desc: &CTFontDescriptor) -> String` — read
  `kCTFontStyleNameAttribute` → `CFString` → `to_string`, or `""`.
- `fn desired_styles(style: Option<&str>, bold: bool, italic: bool) -> Vec<&str>`
  — the exact `if/else` chain above (an explicit `style` wins; otherwise the
  bold/italic combination picks the list).
- `fn style_score(style_str: &str, desired: &[&str]) -> (bool, u8)`:
  - `exact = style_str.eq_ignore_ascii_case(desired[0])`.
  - `let lower = style_str.to_ascii_lowercase(); let mut fuzzy = style_str.len().min(u8::MAX as usize) as u8;`
    then for each `ds`, if `lower.contains(&ds.to_ascii_lowercase())`,
    `fuzzy = fuzzy.saturating_sub(ds.len().min(u8::MAX as usize) as u8)`;
    finally `(exact, u8::MAX.saturating_sub(fuzzy))`. (`to_ascii_lowercase` +
    `contains` is the byte-wise ASCII equivalent of `indexOfIgnoreCase`.)
- In `score`, after the bold/italic fields:
  `let (e, f) = style_score(&style_name(ct_desc), &desired_styles(self.style.as_deref(), self.bold, self.italic)); s.exact_style = e; s.fuzzy_style = f;`.

## Scope / faithfulness notes

- **Ported**: the `exact_style` and `fuzzy_style` computation — the style-name
  read, the `desired_styles` list, the case-insensitive exact match, and the
  saturating fuzzy-substring score.
- **Minor deviation**: upstream's `@intCast(style_str.len)` would panic for a
  style name longer than 255 bytes; the port **clamps** to `u8::MAX` (style
  names are short, so this is unreachable in practice — a robustness choice,
  noted). The style-name buffer is `String` (no fixed 128-byte cap).
- **Deferred**: the variation-axis bold/italic derivation and
  `sortMatchingDescriptors` (wiring the now-complete `Score` into discovery).
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/font/discovery.rs`: add `style_name`, `desired_styles`,
   `style_score`; set `exact_style`/`fuzzy_style` in `score`.
2. Tests (in `discovery.rs`):
   - `desired_styles_chain`: the list matches upstream for each branch —
     explicit style (`Some("Foo")` ⇒ `["Foo"]`), `bold+italic`, `bold`,
     `italic`, and the default (`["regular", "upright"]`).
   - `style_score_pure`: synthetic strings exercise the logic deterministically
     — `style_score("Regular", &["regular","upright"]) == (true, 255)` (the
     whole name is consumed);
     `style_score("Bold", &["bold","upright"]) == (true, 255)`;
     `style_score("Regular", &["bold","upright"]) == (false, 248)` (`255 − 7`
     leftover); `style_score("", &["regular","upright"]) == (false, 255)`;
     `style_score("Italic", &["regular","upright"]) == (false, 249)`
     (`255 − 6`).
   - `score_style_exact_integration`: among the resolved Menlo candidates, the
     one whose style name is `"Regular"` scores `exact_style == true` for a
     default request and `exact_style == false` for a `bold` request; and its
     default- request `fuzzy_style` exceeds its bold-request `fuzzy_style` (the
     matching desire consumes more of the name).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty style
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `score` computes `exact_style`/`fuzzy_style` faithfully (the style-name read,
  the `desired_styles` chain, the case-insensitive exact match, the saturating
  fuzzy score);
- the desired-styles, pure-style-score, and integration tests pass;
- the variation-axis derivation and the sort stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the integration assertion is non-deterministic
on the test host (the pure `style_score`/`desired_styles` tests still prove the
logic).

The experiment **fails** if the desired-style chain, the exact match, or the
fuzzy score diverges from upstream, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It confirmed the `desired_styles` chain matches upstream — including
the important ordering where `desired[0]` is the exact-match target — and that
the fuzzy-score algorithm is faithful (start from the byte length, ASCII
case-insensitive substring-match each desired style, saturating-subtract each
matched desired length, then `255 − leftover`, with `to_ascii_lowercase` +
`contains` being the correct equivalent of `indexOfIgnoreCase`). It **verified
the hand-computed test values** (`Regular`/default ⇒ `255`, `Bold`/bold ⇒ `255`,
`Regular`/bold ⇒ `248`, empty ⇒ `255`, `Italic`/default ⇒ `249`). It agreed the
`u8` clamp is an acceptable robustness deviation for unreachable-in-practice
long style names and that `str::len()` (bytes) preserves Zig's byte-length
behavior.

Review artifacts:

- Prompt: `logs/codex-review/20260603-120024-848219-prompt.md`
- Result: `logs/codex-review/20260603-120024-848219-last-message.md`

## Result

**Result:** Pass

The style match lands — `score()` now fills every field but the deferred
variation-axis refinement.

- `roastty/src/font/discovery.rs`: `style_name` (reads
  `kCTFontStyleNameAttribute` → `CFString` → `String`, or `""`);
  `desired_styles(style, bold, italic)` (the upstream precedence chain —
  explicit style wins, else the bold/italic combination);
  `style_score(style_str, desired)` (case-insensitive exact match on
  `desired[0]`, plus the saturating fuzzy-substring score `255 −` leftover).
  `score` sets `exact_style`/`fuzzy_style` from these.

Tests: `desired_styles_chain` (every branch, incl. explicit-style precedence),
`style_score_pure` (`Regular`/default ⇒ `(true, 255)`, `Bold`/bold ⇒
`(true, 255)`, `Regular`/bold ⇒ `(false, 248)`, `Italic`/default ⇒
`(false, 249)`, empty ⇒ `(false, 255)`), `score_style_exact_integration` (the
Regular Menlo candidate exact-matches a default request but not a bold one, and
its default `fuzzy_style` exceeds its bold `fuzzy_style`).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2716 passed, 0 failed (+3, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The discovery `Score.score` is now a near-complete port: glyph count, codepoint
coverage, monospace, the `head`/`OS/2`-refined bold/italic, and the style
exact/fuzzy match. Only the **variation-axis** bold/italic derivation (for
variable fonts) remains deferred.

The next discovery experiment can be either the **variation-axis** derivation
(the heaviest CoreText FFI — `kCTFontVariationAxesAttribute`/
`kCTFontVariationAttribute`) or, since the `Score` is otherwise complete,
**`sortMatchingDescriptors`** — wiring the ranking into `discover_descriptors`
so discovery returns its candidates **best-first** (`score` each candidate, sort
by `Score::int()` descending). After that: the
`DiscoverIterator`/`DeferredFace`, `discoverFallback`, then the resolver's
discovery fallback and codepoint overrides.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no Required findings**. It confirmed `desired_styles` matches upstream exactly
(including explicit-style precedence and the `desired[0]` exact-match target,
with the index safe because every branch is non-empty), that `style_score`
matches the upstream fuzzy semantics (byte-length start, ASCII case-insensitive
substring checks, saturating subtraction per matched desired style, then
saturating `255 − leftover`), that `eq_ignore_ascii_case` and
`to_ascii_lowercase().contains(...)` are faithful replacements for the Zig ASCII
helpers, and that `style_name` is sound and leak-free (retains/downcasts the
copied attribute, converts to an owned `String`, falls back to `""` on
absent/wrong-type). No Optional findings.

Review artifacts:

- Result review: `logs/codex-review/20260603-120237-619620-last-message.md`
