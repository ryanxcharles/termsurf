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

# Experiment 241: Port Font `Key` and `ModifierSet`

## Description

Port the `Key` enum and the `ModifierSet` type from upstream `font/Metrics.zig`.
`Key` names each modifiable metric; `ModifierSet` maps a `Key` to a `Modifier`.
Together with `Modifier` (Exps 239–240) they are the inputs to the (deferred)
`Metrics::apply`. This is a small foundation slice that unblocks `apply` (Exp
242).

### `Key` (upstream lines 585–605)

Upstream generates `Key` reflectively: one variant per `Metrics` field whose
type is `u32`, `i32`, or `f64`, with the variant value equal to the field's
index. All seventeen `Metrics` fields qualify, so `Key` has seventeen variants
with values `0..=16`, in the struct's field order:

`cell_width(0)`, `cell_height(1)`, `cell_baseline(2)`, `underline_position(3)`,
`underline_thickness(4)`, `strikethrough_position(5)`,
`strikethrough_thickness(6)`, `overline_position(7)`, `overline_thickness(8)`,
`box_thickness(9)`, `cursor_thickness(10)`, `cursor_height(11)`,
`icon_height(12)`, `icon_height_single(13)`, `face_width(14)`,
`face_height(15)`, `face_y(16)`.

Rust has no field reflection, so `Key` is hand-written with the seventeen
variants and explicit discriminants `0..=16` matching the field order. It must
align with the `Metrics` field set; a guard test keeps them in sync.

### `ModifierSet` (upstream line 448)

Upstream `ModifierSet = std.AutoHashMapUnmanaged(Key, Modifier)`. The Rust
analog is
`pub(crate) type ModifierSet = std::collections::HashMap<Key, Modifier>`. `Key`
derives `Hash, Eq` so it can be a map key.

### Rust mapping

- `pub(crate) enum Key { CellWidth = 0, …, FaceY = 16 }`, `#[repr(u8)]`,
  deriving `Debug, Clone, Copy, PartialEq, Eq, Hash`.
- `pub(crate) type ModifierSet = HashMap<Key, Modifier>`
  (`use std::collections::HashMap;`).

### Faithfulness and scope notes

- The explicit discriminants `0..=16` mirror upstream's field-index values; they
  are not load-bearing for the map (which keys on `Eq`/`Hash`), but preserving
  them keeps fidelity with the upstream generated enum.
- No `Metrics::apply`/`addFloatToInt`/`init`/`hash`/`parseCLI`/`formatEntry`
  behavior.
- No C ABI, header, or ABI inventory changes; no new dependencies (std only).

## Changes

1. `roastty/src/font/metrics.rs`:
   - `use std::collections::HashMap;`.
   - Add `pub(crate) enum Key { … }` (17 variants, `#[repr(u8)]`,
     `Debug, Clone, Copy, PartialEq, Eq, Hash`).
   - Add `pub(crate) type ModifierSet = HashMap<Key, Modifier>;`.

2. Tests in `roastty/src/font/metrics.rs`:
   - `key_discriminants`: assert **all seventeen** variants' values in field
     order (`CellWidth as u8 == 0`, `CellHeight == 1`, … `FaceY == 16`), so a
     swapped or wrong middle discriminant is caught.
   - `key_matches_metrics_field_count`: a guard that there are exactly seventeen
     `Key` variants (e.g. an exhaustive `match` over an array of all variants,
     or asserting the count), so adding/removing a `Metrics` field forces
     updating `Key`.
   - `modifier_set_insert_get`: build a `ModifierSet`, insert
     `Key::CellWidth -> Modifier::Percent(1.2)` and
     `Key::OverlinePosition -> Modifier::Absolute(-2)`, and read both back.

3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo test -p roastty font
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Key` has the seventeen variants in field order with discriminants `0..=16`,
  deriving `Hash`/`Eq`;
- `ModifierSet` is a `HashMap<Key, Modifier>`;
- the discriminant, variant-count guard, and map insert/get tests pass;
- no `Metrics::apply` scope is pulled in;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if `Metrics::apply` reveals `Key` needs a
field→key mapping helper that should be its own change.

The experiment **fails** if a `Key` variant/discriminant diverges from the
`Metrics` field order, if `apply` behavior leaks in, or if any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation.

Review artifacts:

- Prompt: `logs/codex-review/20260602-085326-531079-prompt.md`
- Result: `logs/codex-review/20260602-085326-531079-last-message.md`

Codex confirmed the seventeen variants and discriminants match the Rust
`Metrics` field order (`cell_width = 0` … `face_y = 16`), that all fields are
`u32`/`i32`/`f64` so all qualify, and that `HashMap<Key, Modifier>` with
`Hash + Eq` is the right analog (hash algorithm/order differences do not matter
here).

Two Low findings, fixed in the design above before this commit:

1. the discriminant test now asserts **all seventeen** variants in order (not a
   spot-check), to catch a swapped/wrong middle discriminant.
2. the `Description` heading was `#` (H1); corrected to `##` to match the
   experiment format.
