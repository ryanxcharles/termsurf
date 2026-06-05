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

# Experiment 617: wire StringMap to the screen producer

## Description

Exp 616 ported `StringMap` (regex search over a flattened screen string) but
built it only manually in tests. This slice closes the loop: a `Screen` producer
that builds a `StringMap` from a selection, mirroring upstream's
`Screen.selectionString(..., map: &mut StringMap)`. roastty already has the
machinery —
`PageList::screen_format_string_with_pin_map(selection, …) -> PageStringWithPinMap { text, pin_map }`,
where `pin_map` is **per byte** (`text.len() == pin_map.len()`, asserted in
existing tests) — exactly `StringMap`'s one-pin-per-byte invariant. So this is a
thin constructor + a `Screen` convenience, no new crate.

(Note: this stays within the `regex`-crate area from Exp 616. The default
URL-detection regex in `config/url` uses **variable-length look-behind**
(`(?<!\$\d*)`), which neither the `regex` crate nor `fancy-regex` supports —
only a full PCRE/oniguruma engine does — so `config/url` is a separate
dependency question and is not part of this slice.)

## Upstream behavior

Upstream's `Screen.selectionString` (in `Screen.zig`) takes a selection and an
optional `*StringMap` out-parameter; it flattens the selection to a string and
fills the per-byte pin map. roastty's `screen_format_string_with_pin_map` is the
same flatten-with-pin-map operation; this slice adapts its output into a
`StringMap`.

## Rust mapping

### `StringMap` (`string_map.rs`)

```rust
use super::page_list::PageStringWithPinMap;

impl StringMap {
    /// Build a `StringMap` from a screen's flattened selection text + per-byte pin map (upstream's
    /// `selectionString` map output).
    pub(in crate::terminal) fn from_page_string(p: PageStringWithPinMap) -> StringMap {
        StringMap::new(p.text.into_bytes(), p.pin_map)
    }
}
```

### `Screen` (`screen.rs`)

```rust
/// Flatten `selection` to a `StringMap` (text + per-byte screen pins) for regex search (upstream
/// `Screen.selectionString` with a `StringMap` out-parameter). `unwrap` is always `true` (matching
/// upstream's `selectionString`, so soft-wrapped lines join); `trim` is exposed as a parameter
/// (upstream's option) — link detection passes `false` for raw line content.
pub(in crate::terminal) fn selection_string_map(
    &self,
    selection: selection::Selection,
    trim: bool,
) -> StringMap {
    let p = self.pages.screen_format_string_with_pin_map(
        Some(selection),
        trim,
        true, // unwrap (upstream `selectionString` always unwraps)
        PageOutputFormat::Plain,
        None, // palette
        None, // codepoint_map
    );
    StringMap::from_page_string(p)
}
```

`screen.rs` imports `StringMap` from `super::string_map`; `PageOutputFormat` and
`PageStringWithPinMap` are already imported there.

### Notes / deviations

- No new dependency: reuses the existing `regex` crate (Exp 616) and the
  existing `screen_format_string_with_pin_map` producer.
- `pin_map` is already per-byte (`text.len() == pin_map.len()`), so
  `StringMap::from_page_string` is a direct adoption (the `StringMap::new`
  `assert_eq!` guards it).
- `Plain` format; `unwrap=true` (upstream `selectionString` always unwraps, so a
  match can span soft-wrapped rows); `trim` is a parameter (link detection
  passes `false`); palette / codepoint-map are `None` (irrelevant to text
  bytes).
- The `config/url` URL regex (needs oniguruma-class look-behind) and the
  `input/Link` matching are separate slices / dependency questions.

## Verification

- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — no regressions; new tests:
  - `selection_string_map_searches_a_real_selection` — write `"hi https://x.y"`
    on a screen, select the row (cells `0..=13`), build a `StringMap` via
    `selection_string_map`, regex `https?://\S+` → one match whose selection
    spans the URL cells (start at the `h` of `https`, end at the final `y`).
  - `selection_string_map_simple_match` — a `[A-B]{2}` match over a selected row
    with `"xABy"` maps to the `A`/`B` cells.
  - `from_page_string_preserves_byte_count` — the produced `StringMap`'s string
    length equals its map length (the per-byte invariant), via a real selection.
  - `selection_string_map_unwraps_soft_wrap` (adopted Optional) — a soft-wrapped
    line: `unwrap=true` joins the wrapped rows (no artificial newline) and a
    regex match resolves across the wrap to the right pins.
  - `selection_string_map_multibyte_invariant` (adopted Optional) — a selection
    containing a multibyte/wide cell still yields `string.len() == map.len()`
    (the producer is per-byte), so `from_page_string` does not trip the assert.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on touched source — clean.
- `git diff --check` — clean.

Pass = `Screen::selection_string_map` produces a `StringMap` from a real
selection whose regex matches resolve to the correct screen pins — closing the
Exp 616 loop end-to-end.

## Design Review

Codex reviewed the design and raised **two Required** findings, both adopted:

- **Required (adopted)**: use `unwrap=true` — upstream `Screen.selectionString`
  always initializes the formatter with `unwrap = true`, so soft-wrapped
  selections join (matters for regex/link matching across wrapped lines).
- **Required (adopted)**: expose `trim` as a parameter rather than hard-coding
  `false` — upstream's `selectionString` has `trim` as an option (default
  `true`); `selection_string_map(&self, selection, trim: bool)` lets link
  detection pass `false` for raw line content while staying a faithful general
  producer.
- **Optional (adopted)**: a soft-wrap test (`unwrap=true` removes the artificial
  newline; the pin map still resolves matches across the wrap) and a
  multibyte/wide-cell test (the per-byte invariant holds).

Codex confirmed the per-byte invariant is sound for all inputs (including
multibyte UTF-8 and wide cells — existing tests assert
`text.len() == pin_map.len()`), so `StringMap::new`'s `assert_eq!` is
appropriate fail-closed behavior, and that the `config/url` URL-regex dependency
question is correctly deferred.

Review artifacts:

- Prompt: `logs/codex-review/20260605-d617-prompt.md`
- Result: `logs/codex-review/20260605-d617-last-message.md`
