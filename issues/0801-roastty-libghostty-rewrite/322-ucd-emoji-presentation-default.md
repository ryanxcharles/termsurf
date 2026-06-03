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

# Experiment 322: the UCD emoji-presentation default

## Description

When `get_index` is asked for a codepoint with **no explicit presentation**, the
resolver must pick a default. The current placeholder always defaults to
`Presentation::Text`; upstream instead consults the Unicode Character Database —
`uucode.get(.is_emoji_presentation, cp)` — and defaults to `.emoji` for
codepoints whose `Emoji_Presentation` property is `Yes` (e.g. `⌚ U+231A`,
`⚽ U+26BD`, `😀 U+1F600`), `.text` otherwise (e.g. `❤ U+2764` and `☺ U+263A`,
which are emoji but default to text without a VS16 selector). This experiment
ports that lookup — a generated `Emoji_Presentation` range table and an
`is_emoji_presentation` predicate — and wires it into `get_index`'s default
presentation.

## Upstream behavior (`CodepointResolver.zig` `getIndex`)

```zig
const p_mode: Collection.PresentationMode = if (p) |v| .{ .explicit = v } else .{
    .default = if (uucode.get(.is_emoji_presentation, @intCast(cp)))
        .emoji
    else
        .text,
};
```

`uucode`'s `is_emoji_presentation` is derived from the UCD `emoji-data.txt`
`Emoji_Presentation` property (the set of codepoints that render as emoji by
default, without a variation selector).

## Data source

`vendor/uucode/ucd/emoji/emoji-data.txt` (the canonical Unicode
`emoji-data.txt`, dated 2025-07-25) lists the `Emoji_Presentation` property as
296 sorted, non-overlapping ranges (`START..END ; Emoji_Presentation` or
`CP ; Emoji_Presentation`). `vendor/` is **git-ignored**, so the table cannot be
read at build time in a clean checkout or a distributed build — it must be
**generated once into a committed Rust source file** (the data baked into
source, the same shape uucode bakes it into the Zig binary at comptime).

## Rust mapping

- `roastty/src/font/emoji_presentation.rs` (new, committed, generated): a
  `pub(crate) const EMOJI_PRESENTATION: &[(u32, u32)]` — the 296 inclusive
  `(start, end)` ranges sorted by `start`, with a header comment recording the
  provenance (`emoji-data.txt`, the Unicode date, the `Emoji_Presentation`
  property) and noting it is generated. Plus
  `pub(crate) fn is_emoji_presentation(cp: u32) -> bool` — a binary search over
  the sorted ranges (`partition_point` on `start <= cp`, then check
  `cp <= end`).
- `roastty/src/font/mod.rs`: add `pub(crate) mod emoji_presentation;`.
- `roastty/src/font/codepoint_resolver.rs` `get_index`: change the no-explicit-
  presentation default from `PresentationMode::Default(Presentation::Text)` to
  `PresentationMode::Default(if emoji_presentation::is_emoji_presentation(cp) { Presentation::Emoji } else { Presentation::Text })`.
  The sprite check stays **before** this (matching upstream order: codepoint
  overrides → sprite → presentation default).

## Generation

A one-off generator (run during implementation, not committed as a build step)
parses `vendor/uucode/ucd/emoji/emoji-data.txt`: keep non-comment lines whose
property is `Emoji_Presentation`, parse the `START..END` (or single `CP`) hex
range, sort by `start`, and emit the Rust `const`. The committed file is the
artifact; the generator is described here for reproducibility. The build does
**not** read the txt.

## Scope / faithfulness notes

- **Ported**: the `is_emoji_presentation` UCD lookup and its use as the default
  presentation in `get_index` — faithful to upstream's `uucode.get(...)`
  default.
- **Deferred**: codepoint overrides (still a placeholder before the sprite
  check) and discovery-based fallback. The other UCD properties (grapheme break,
  width, etc.) are out of scope — only `Emoji_Presentation` is needed here.
- **Explicitly deferred — VS15/VS16**: this change touches **only** the
  `p == None` default. The variation-selector path (a trailing `U+FE0E`/`U+FE0F`
  forcing text/emoji) is the shaper's job and reaches the resolver as an
  explicit `Some(Presentation)`; `Some(Text)`/`Some(Emoji)` are unchanged here.
  Selector handling is not part of this experiment.
- The table is pinned to the vendored `emoji-data.txt` version; refreshing
  Unicode is a separate, mechanical regeneration.
- No C ABI/header/ABI-inventory change (the table and the resolver are internal
  Rust).

## Changes

1. `roastty/src/font/emoji_presentation.rs`: the generated `EMOJI_PRESENTATION`
   table + `is_emoji_presentation`.
2. `roastty/src/font/mod.rs`: declare the module.
3. `roastty/src/font/codepoint_resolver.rs`: use `is_emoji_presentation` for the
   default presentation.
4. Tests:
   - `is_emoji_presentation_known` (in `emoji_presentation.rs`): the
     emoji-presentation codepoints `0x231A` (watch), `0x26BD` (soccer),
     `0x1F004` (mahjong), `0x1F600` (grinning) are `true`; the text-default
     codepoints `0x2764` (heavy black heart) and `0x263A` (white smiling face) —
     which are emoji but **not** `Emoji_Presentation` — plus `0x41` (`A`),
     `0x2500` (box), and `0x10_FFFF` are `false`. Boundary checks: the first
     range's `start` and `end` are `true`, `start - 1` is `false`.
   - `emoji_presentation_table_sorted` (in `emoji_presentation.rs`): the table
     is sorted by `start`, every `start <= end`, and consecutive ranges do not
     overlap or touch out of order (`prev.end < next.start`) — the invariant the
     binary search relies on. Also asserts the expected length (296).
   - `get_index_default_presentation_emoji` (in `codepoint_resolver.rs`): with a
     resolver whose collection reports an emoji face,
     `get_index(0x1F600, Regular, None)` resolves through the **emoji**
     presentation default (assert the resolved index matches the emoji
     presentation, distinct from the text default) — or, if a full emoji face is
     impractical in the unit test, assert the narrower invariant that
     `get_index` consults `is_emoji_presentation` by checking that a
     `None`-presentation emoji codepoint and an explicit `Some(Emoji)` request
     resolve **identically**, while a non-emoji codepoint's `None` default
     matches `Some(Text)`. (The exact assertion is finalized against the
     available test faces during implementation.)
5. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty emoji
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `is_emoji_presentation` reproduces the UCD `Emoji_Presentation` property from
  the vendored `emoji-data.txt`, and `get_index` uses it for the default
  presentation (emoji for `Emoji_Presentation` codepoints, text otherwise),
  faithful to upstream;
- the known-codepoint, table-invariant, and resolver-default tests pass;
- codepoint overrides and discovery stay deferred; the build does not read the
  txt;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the resolver wiring lands but a full emoji-face
resolver test cannot be expressed with the available test faces (the predicate
and table are still proven directly).

The experiment **fails** if the table diverges from the vendored
`Emoji_Presentation` property, the predicate misclassifies a boundary codepoint,
or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. Verified against the vendored sources:
`uucode.get(.is_emoji_presentation, cp)` maps specifically to the UCD
`Emoji_Presentation` property (not `Emoji` or `Extended_Pictographic`) per
`vendor/uucode/src/build/Ucd.zig`; the vendored `emoji-data.txt` has 296
`Emoji_Presentation` ranges, matching the planned table length; the sample
classifications are correct (`231A`/`26BD`/`1F004`/`1F600` are
`Emoji_Presentation`; `2764`/`263A`/`0041`/`2500`/`10FFFF` are not). It
confirmed that baking the table into committed Rust is the right design given
`vendor/` is unavailable in clean/distributed builds, that a `partition_point`
search over sorted inclusive `(start, end)` ranges is correct with the planned
sorted/non-overlap invariant, and that keeping the sprite check before the
presentation default matches upstream `getIndex` order (overrides → sprite →
`is_emoji_presentation`). One **Optional** note: make the VS15/VS16 deferment
explicit — this change must touch only the `p == None` default, leaving
`Some(Text)`/`Some(Emoji)` (the variation-selector path) unchanged. Folded into
the Scope notes. No Required findings.

Review artifacts:

- Prompt: `logs/codex-review/20260603-104155-273447-prompt.md`
- Result: `logs/codex-review/20260603-104155-273447-last-message.md`

## Result

**Result:** Pass

The UCD emoji-presentation default lands.

- `roastty/src/font/emoji_presentation.rs` (new, committed, generated): the
  `EMOJI_PRESENTATION` table — 296 inclusive `(start, end)` ranges parsed from
  `vendor/uucode/ucd/emoji/emoji-data.txt` (the `Emoji_Presentation` property,
  dated 2025-07-25), sorted by `start` and non-overlapping — plus
  `is_emoji_presentation(cp)`, a `partition_point` binary search. The build
  never reads the txt; the table is baked into source.
- `roastty/src/font/mod.rs`: declares `emoji_presentation`.
- `roastty/src/font/codepoint_resolver.rs`: `get_index` now builds the
  no-explicit-presentation default as
  `PresentationMode::Default(if is_emoji_presentation(cp) { Emoji } else { Text })`,
  keeping the sprite check before it. The module/method docs no longer list the
  UCD default as deferred.

Tests: `is_emoji_presentation_known` (the emoji-presentation `0x231A`/`0x26BD`/
`0x1F004`/`0x1F600` are `true`; the text-default `0x2764`/`0x263A` and the
non-emoji `0x41`/`0x2500`/`0x10_FFFF` are `false`; first-range boundaries);
`emoji_presentation_table_sorted` (`len == 296`, every `start <= end`, strictly
non-overlapping windows); `get_index_default_presentation_emoji` (`U+2614`
umbrella — `Emoji_Presentation = Yes`, present in Menlo as non-color and Apple
Color Emoji as color, both added as **fallback** faces so presentation
discriminates: `None` resolves to the emoji face, `Some(Text)` to the Menlo face
— proving the exact-match default, not the last-resort `Any`, makes the choice).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2685 passed, 0 failed (+3, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

A note on the resolver test: a single-face emoji codepoint's default is
**unobservable** through `get_index`'s final result, because upstream's
last-resort "any presentation" path resolves it regardless of the default. The
two-fallback `U+2614` setup is what makes the default observable (a fallback
entry treats `Default(p)` as `Explicit(p)`), so the test genuinely exercises the
exact-match default-presentation path. This is the full resolver-behavior test
the design anticipated might be impractical — it proved expressible, so the
result is **Pass**, not Partial.

## Conclusion

A presentation-less codepoint now defaults to emoji or text per the UCD
`Emoji_Presentation` property, faithful to upstream's `uucode.get(...)`. The
resolver's `get_index` is closer to complete: the sprite check, the UCD
presentation default, and the regular-style/last-resort fallbacks are all
ported; codepoint overrides and discovery-based fallback remain the deferred
pieces.

The next resolver work is **codepoint overrides** (the placeholder before the
sprite check — a user/config table remapping specific codepoints to specific
faces) and the **discovery-based fallback** (CoreText font matching for a
codepoint no loaded face covers). After the resolver: the shaper, the Nerd Font
attribute table, and SVG color detection.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no Required findings**. It **mechanically compared** the committed table
against `emoji-data.txt` (extracting only `; Emoji_Presentation` lines):
`ucd = 296`, `rust = 296`, `equal = true` — ruling out any accidental inclusion
of the broader `Emoji`, `Emoji_Component`, or `Extended_Pictographic` rows. It
confirmed the `partition_point` binary search is correct across before-first,
inside-range, range-end, gap, and after-last cases, and that the
sorted/non-overlap and boundary tests cover the invariant. It confirmed the
resolver ordering is faithful (sprite before presentation, only `None` changes,
explicit `Some(Text)`/`Some(Emoji)` passed through unchanged) and that the
umbrella two-fallback test soundly proves the exact-match default-presentation
path rather than the last-resort `Any` fallback. No Optional findings.

Review artifacts:

- Result review: `logs/codex-review/20260603-105003-333222-last-message.md`
