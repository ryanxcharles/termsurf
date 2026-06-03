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

# Experiment 323: the Nerd Font constraint attribute table

## Description

Nerd Font glyphs (powerline separators, icons, symbols) need per-codepoint
**sizing/positioning constraints** so they scale and align correctly in a
terminal cell. Upstream bakes these into a generated
`getConstraint(cp) -> ?Constraint` table (`nerd_font_attributes.zig`, produced
from the Nerd Fonts patcher script by `nerd_font_codegen.py`). roastty already
has the matching [`Constraint`] type (`face/constraint.rs`) with the
**identical** fields, enums, and defaults — so this experiment ports the table
itself: a generated `get_constraint(cp) -> Option<Constraint>` returning the
same constraint for the same codepoint as upstream.

## Upstream behavior (`nerd_font_attributes.zig`)

```zig
pub fn getConstraint(cp: u21) ?Constraint {
    return switch (cp) {
        0x2630 => .{ .size = .cover, .height = .icon, .max_constraint_width = 1,
                     .align_horizontal = .center1, .align_vertical = .center1,
                     .pad_left = 0.05, .pad_right = 0.05, .pad_top = 0.05, .pad_bottom = 0.05 },
        0x276c...0x276d => .{ .size = .cover, ... .relative_width = 0.714..., ... },
        // ... 276 constraint arms total, each a set of codepoints/ranges → a Constraint
        else => null,
    };
}
```

Each arm maps one or more codepoints/ranges to a `Constraint` literal; unset
fields take the `Constraint` struct defaults. There are **276** constraint-
returning arms (`=> .{ … }`) plus the trailing `else => null`. The enum/field
universe actually used across all arms is small and fully covered by roastty's
`Constraint`:

- `size` ∈ `{cover, fit_cover1, stretch}` → `Size::{Cover, FitCover1, Stretch}`
- `height` ∈ `{icon}` → `Height::Icon`
- `align_horizontal`/`align_vertical` ∈ `{center1, end, start}` →
  `Align::{Center1, End, Start}`
- `max_constraint_width` (u8), `max_xy_ratio` (f64 → `Some(..)`),
  `pad_{top,left,right,bottom}` (f64), `relative_{width,height,x,y}` (f64).

roastty's `Constraint::default()` is **field-for-field identical** to upstream's
`.{}` (verified: `size None`, aligns `None`, pads `0.0`,
`relative_width/height 1.0`, `relative_x/y 0.0`, `max_xy_ratio None`,
`max_constraint_width 2`, `height Cell`), so a generated entry that sets only an
arm's fields and `..Default:: default()` produces the exact same `Constraint` as
upstream.

## Data source / generation

The artifact is committed, generated **once** from
`vendor/ghostty/src/font/nerd_font_attributes.zig` (which is itself generated
from the Nerd Fonts patcher). `vendor/` is git-ignored, so the build does
**not** read it — the table is baked into a committed Rust source file (the same
shape upstream bakes into the Zig binary). A one-off parser reads each `switch`
arm (its codepoint patterns and field assignments) and emits a Rust `match` arm:

- Zig `0x2630` → Rust `0x2630`; Zig inclusive range `0x276c...0x276d` → Rust
  `0x276c..=0x276d`; multiple patterns in an arm → a Rust `|`-alternation.
- Zig `.size = .cover` → `size: Size::Cover`, `.height = .icon` →
  `height: Height::Icon`, `.align_* = .center1` → `Align::Center1`,
  `.max_xy_ratio = X` → `max_xy_ratio: Some(X)`, the `f64`/`u8` fields verbatim,
  with `..Default:: default()` for the rest.

## Rust mapping

- `roastty/src/font/face/nerd_font_attributes.rs` (new, committed, generated):
  ```rust
  pub(crate) fn get_constraint(cp: u32) -> Option<Constraint> {
      Some(match cp {
          0x2630 => Constraint { size: Size::Cover, height: Height::Icon,
              max_constraint_width: 1, align_horizontal: Align::Center1,
              align_vertical: Align::Center1, pad_left: 0.05, pad_right: 0.05,
              pad_top: 0.05, pad_bottom: 0.05, ..Default::default() },
          // ... 276 constraint arms ...
          _ => return None,
      })
  }
  ```
  with a header comment recording the provenance (`nerd_font_attributes.zig`,
  generated, DO NOT EDIT BY HAND).
- `roastty/src/font/face/mod.rs`: add `pub(crate) mod nerd_font_attributes;`.

## Scope / faithfulness notes

- **Ported**: the `getConstraint` table — `get_constraint(cp)` returns the exact
  `Constraint` (or `None`) upstream does, for all 276 constraint arms.
- **Deferred**: wiring `get_constraint` into the render/shaper path (the
  consumer that decides to apply a Nerd Font constraint when rendering a glyph).
  This experiment lands the data + lookup; the application is a follow-up,
  mirroring how the emoji-presentation table landed before its full consumer.
- The table is pinned to the vendored `nerd_font_attributes.zig`; refreshing
  Nerd Fonts is a separate, mechanical regeneration.
- No C ABI/header/ABI-inventory change (`Constraint` is internal Rust).

## Changes

1. `roastty/src/font/face/nerd_font_attributes.rs`: the generated
   `get_constraint`.
2. `roastty/src/font/face/mod.rs`: declare the module.
3. Tests (in `nerd_font_attributes.rs`):
   - `get_constraint_known`: spot-check representative arms against the upstream
     values — e.g. `get_constraint(0x2630)` equals the hexagram constraint
     (`Size::Cover`, `Height::Icon`, `max_constraint_width 1`, both aligns
     `Center1`, all pads `0.05`); `get_constraint(0xEA61)` and
     `get_constraint(0xE0C0)` (the two codepoints upstream's own tests probe)
     are `Some(_)`; a representative `fit_cover1` arm (`0xE0A0`) and a `stretch`
     arm (`0xE0B0`) match their upstream constraints.
   - `get_constraint_none`: non-Nerd codepoints (`0x41` `A`, `0x2500` box,
     `0x1F600` emoji) return `None`.
   - `get_constraint_ranges`: a multi-codepoint range arm matches at both ends
     and a single excluded neighbor returns a different/`None` constraint (the
     range boundaries are faithful).
   - `get_constraint_defaults_match`: an arm that sets only a few fields has the
     struct defaults elsewhere (e.g. its `relative_width == 1.0`,
     `max_xy_ratio == None`) — proving the `..Default::default()` tail matches
     upstream's unset-field defaults.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty nerd
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `get_constraint` reproduces every arm of upstream's `getConstraint` (the same
  `Constraint` or `None` per codepoint), with the field/enum/default mapping
  exact;
- the known-codepoint, none, range-boundary, and defaults tests pass;
- the render/shaper wiring stays deferred; the build does not read the txt/zig;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a generated arm cannot be expressed because a
field/enum is missing from roastty's `Constraint` (none expected — the used set
is fully covered).

The experiment **fails** if any arm's codepoints, constraint values, or defaults
diverge from upstream, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It verified against the local upstream/roastty files: baking a
committed generated Rust table is the right shape (the source is
vendored/generated and `vendor/` is unavailable to clean builds); roastty's
`Constraint::default()` matches upstream's `.{}` defaults; the used assignment
universe is fully covered (`cover`/`fit_cover1`/`stretch`; `icon`;
`center1`/`start`/`end`; the numeric pad/relative fields; `max_xy_ratio`;
`max_constraint_width`); the Zig inclusive `0xAAAA...0xBBBB` maps to Rust
`0xAAAA..=0xBBBB` (not `..`); the planned test codepoints are sound
representatives (`0xEA61`/`0xE0C0` are upstream's own exercised cases, `0xE0A0`
covers `fit_cover1`, `0xE0B0` covers `stretch`, the non-Nerd examples are absent
from the table); and deferring the render/shaper wiring is acceptable for a
table-only port (no runtime rendering change yet, explicitly scoped). One
**non-blocking** counting note: the file has **276** constraint-returning arms
(`=> .{ … }`) plus the trailing `else => null` (277 total `=>`); the
implementation should treat the parser output as authoritative rather than
hard-coding a count. Folded in: the doc now says 276 constraint arms, and the
result will report the parser-derived count.

Review artifacts:

- Prompt: `logs/codex-review/20260603-105549-333056-prompt.md`
- Result: `logs/codex-review/20260603-105549-333056-last-message.md`
