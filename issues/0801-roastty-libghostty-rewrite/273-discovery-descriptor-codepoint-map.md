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

# Experiment 273: Descriptor + CodepointMap — the font-search data layer

## Description

Both the resolver's **codepoint overrides** and **discovery-based fallback**
(deferred from Experiment 272) build on a font-search **`Descriptor`** and, for
overrides, a **`CodepointMap`** (codepoint range → descriptor). This experiment
ports those pure data types: the `Descriptor` (`font/discovery.zig`), the
`Variation` (font-variation axis, `font/face.zig`), and the `CodepointMap`
(`font/CodepointMap.zig`) with its reverse-priority range lookup. No FFI; the
discovery logic that _consumes_ a descriptor is a later sub-area.

## Upstream behavior

- `Variation` (`face.zig`):
  `{ id: Id (packed u32 from a 4-char code), value: f64 }`. `Id.init([4]u8)`
  packs the bytes; `Id` round-trips a `u32` (e.g. `"wght" = 2003265652`).
- `Descriptor` (`discovery.zig`):
  `{ family: ?str, style: ?str, codepoint: u32 = 0, size: f32 = 0, bold/italic/monospace: bool = false, variations: []Variation }`
  — describes a font to search for.
- `CodepointMap` (`CodepointMap.zig`): a list of
  `{ range: [u21; 2], descriptor }` entries. `add` asserts
  `range[0] <= range[1]` and appends. `get(cp)` does a **reverse** linear scan
  (later entries win) and returns the first descriptor whose range contains
  `cp`, else `null`.

## Rust mapping

- `roastty/src/font/discovery.rs` (new):
  - `struct Variation { id: u32, value: f64 }` with
    `Variation::id_from_tag(tag: &[u8; 4]) -> u32` (`u32::from_be_bytes(*tag)` —
    the packed 4-char code; a `wght` tag yields `2003265652`).
  - `struct Descriptor { family: Option<String>, style: Option<String>, codepoint: u32, size: f32, bold: bool, italic: bool, monospace: bool, variations: Vec<Variation> }`
    with `Default` matching upstream (`codepoint = 0`, `size = 0.0`, bools
    `false`, empty variations, `None` names).
- `roastty/src/font/codepoint_map.rs` (new):
  - `struct MapEntry { range: [u32; 2], descriptor: Descriptor }` (upstream
    `u21` → `u32`; values stay within `u21`).
  - `struct CodepointMap { entries: Vec<MapEntry> }` (`Default` empty).
  - `add(&mut self, range: [u32; 2], descriptor: Descriptor)`:
    `assert!(range[0] <= range[1])`, then push.
  - `get(&self, cp: u32) -> Option<&Descriptor>`: iterate `entries` **in
    reverse** (`.iter().rev()`), return the first whose
    `range[0] <= cp <= range[1]`, else `None`.
- `roastty/src/font/mod.rs`: declare both modules.

## Scope / faithfulness notes

- **Deferred**: the discovery logic that turns a `Descriptor` into a loaded face
  (CoreText font matching — the `discovery` sub-area), `CodepointMap::clone`/
  `hash` (arena/hash plumbing not needed yet), and `Variation::Id::str()` (the
  reverse tag decode). These are utility/consumer concerns.
- `Descriptor` is a plain data carrier (owned `String`s replace upstream's
  caller-owned `[:0]const u8`).
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/discovery.rs` (new): `Variation` (+ `id_from_tag`) and
   `Descriptor` (+ `Default`).
2. `roastty/src/font/codepoint_map.rs` (new): `MapEntry`, `CodepointMap`, `add`,
   `get`.
3. `roastty/src/font/mod.rs`: declare `discovery` and `codepoint_map`.
4. Tests:
   - `variation_id_from_tag`: `Variation::id_from_tag(b"wght") == 2003265652`
     and `b"slnt" == 1936486004` (the upstream-verified values).
   - `descriptor_default`: `Descriptor::default()` has `codepoint == 0`,
     `size == 0.0`, all bools `false`, empty variations, `None` family/style.
   - `codepoint_map_get`: a range `[0x41, 0x5A]` → a descriptor; `get(0x41)` /
     `get(0x5A)` are `Some`, `get(0x40)` / `get(0x5B)` are `None`.
   - `codepoint_map_reverse_priority`: add `[0, 0xFFFF]` → D1 then
     `[0x41, 0x41]` → D2; `get(0x41)` returns **D2** (the later entry wins) and
     `get(0x42)` returns D1 (distinguished by a unique `family`).
   - `codepoint_map_rejects_inverted_range` (`#[should_panic]`):
     `add([0x5A, 0x41], …)` panics on the `range[0] <= range[1]` assertion.
5. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty codepoint_map
cargo test -p roastty discovery
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Variation::id_from_tag` packs a 4-char tag to the upstream `u32`,
  `Descriptor` carries the search fields with upstream defaults, and
  `CodepointMap`'s `add` guards inverted ranges while `get` does the
  reverse-priority range lookup;
- the discovery consumer and the `clone`/`hash` utilities are cleanly deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the `Descriptor`/`Variation` shape needs fields
beyond this minimal set for the deferred discovery consumer.

The experiment **fails** if `CodepointMap`'s priority/range semantics diverge
from upstream or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-230133-037811-prompt.md`
- Result: `logs/codex-review/20260602-230133-037811-last-message.md`

Codex confirmed the reverse-priority lookup matches upstream (later entries win
via a reverse scan returning the first containing range), the inverted-range
`assert!` matches the upstream `add` guard, the `Descriptor` fields/defaults
match the upstream data shape (owned `String`s a reasonable replacement for the
borrowed Zig strings), and that `Variation::id_from_tag` via
`u32::from_be_bytes` is correct — upstream's packed `{ d, c, b, a }` with
`a = tag[0] … d = tag[3]` bitcasts to the big-endian four-character code, so
`"wght"` becomes `0x77676874 == 2003265652`. Using `u32` for the `u21` codepoint
domain is acceptable for this data layer, and the tests cover priority, boundary
inclusion/exclusion, the inverted-range panic, defaults, and tag packing.
