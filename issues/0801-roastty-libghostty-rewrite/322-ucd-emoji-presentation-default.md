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
