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

# Experiment 388: the search-highlight color arms

## Description

Upstream's per-cell `selected` state has four cases — `false`, `selection`,
`search`, `search_selected` — and the background/foreground switches have a
`.search` and `.search_selected` arm in addition to `.selection`. This
experiment adds the **search** color machinery: the full `Selected` enum, the
four `search-*` color config values (with upstream's defaults), and a
`selected_colors` dispatcher that computes any selected state's colors. The key
observation is that the `.search`/`.search_selected` switch arms are
**byte-identical** to the `.selection` arms — they differ only in that the
search config is non-optional (no plain-reverse default) — so the dispatcher
reuses `selection_colors` (Experiment 385), passing the search config wrapped in
`Some`. This experiment is the color computation; producing
`Search`/`SearchSelected` states (the per-row search highlight ranges) and
wiring the dispatcher into the passes are follow-ups.

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`), the background switch's search arms
(the foreground's are the analogous `final_bg`/`fg_style` forms, identical to
the `.selection` foreground arm):

```zig
.search => switch (self.config.search_background) {
    .color => |color| color.toTerminalRGB(),
    .@"cell-foreground" => if (style.flags.inverse) bg_style else fg_style,
    .@"cell-background" => if (style.flags.inverse) fg_style else bg_style,
},
.search_selected => switch (self.config.search_selected_background) { … same … },
```

These arms are exactly the `.selection` arms (Experiment 385) **minus** the
`if (config) |v| … else default` wrapper — `search_background` etc. are plain
`TerminalColor`s (always a value), so there is no plain-reverse default. The
config defaults (`config/Config.zig`):

```
search-foreground          = .color (0x00, 0x00, 0x00)   // black
search-background          = .color (0xFF, 0xE0, 0x82)   // amber
search-selected-foreground = .color (0x00, 0x00, 0x00)   // black
search-selected-background = .color (0xF2, 0xA5, 0x7E)   // salmon
```

The `selected` enum is `false | selection | search | search_selected`; a `false`
cell uses the base `cell_colors`, the other three use the selection/search arms.

## Rust mapping (`roastty/src/renderer/cell.rs`)

The full `Selected` enum, four new `SelectionConfig` fields with a faithful
hand-written `Default`, and a `selected_colors` dispatcher:

```rust
/// The per-cell selected state (upstream's `selected` enum). `False` uses the
/// base [`cell_colors`]; the three selected states use [`selected_colors`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Selected {
    False,
    Selection,
    Search,
    SearchSelected,
}

/// The selection/search color config. `selection-*` is optional (`None` → a plain
/// reverse); the `search-*`/`search-selected-*` values are non-optional (upstream
/// `TerminalColor`s with concrete defaults).
#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectionConfig {
    pub background: Option<SelectionColor>,
    pub foreground: Option<SelectionColor>,
    pub search_background: SelectionColor,
    pub search_foreground: SelectionColor,
    pub search_selected_background: SelectionColor,
    pub search_selected_foreground: SelectionColor,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            background: None,
            foreground: None,
            // Upstream config defaults.
            search_background: SelectionColor::Color(Rgb::new(0xFF, 0xE0, 0x82)),
            search_foreground: SelectionColor::Color(Rgb::new(0, 0, 0)),
            search_selected_background: SelectionColor::Color(Rgb::new(0xF2, 0xA5, 0x7E)),
            search_selected_foreground: SelectionColor::Color(Rgb::new(0, 0, 0)),
        }
    }
}

/// Compute a cell's colors for a `selected` state. `False` returns `None` (the
/// caller uses the base [`cell_colors`]); the three selected states delegate to
/// [`selection_colors`] with the matching config — `Selection` uses the optional
/// `selection-*` config (`None` → a plain reverse), while `Search`/
/// `SearchSelected` wrap their non-optional `search-*` config in `Some`. The
/// `.search`/`.search_selected` switch arms are the `.selection` arms without the
/// reverse default, so this reuses one computation.
pub(crate) fn selected_colors(
    selected: Selected,
    style: TermStyle,
    default_fg: Rgb,
    default_bg: Rgb,
    palette: &Palette,
    bold: Option<BoldColor>,
    config: &SelectionConfig,
) -> Option<CellColors> {
    let (background, foreground) = match selected {
        Selected::False => return None,
        Selected::Selection => (config.background, config.foreground),
        Selected::Search => (Some(config.search_background), Some(config.search_foreground)),
        Selected::SearchSelected => (
            Some(config.search_selected_background),
            Some(config.search_selected_foreground),
        ),
    };
    Some(selection_colors(
        style, default_fg, default_bg, palette, bold, background, foreground,
    ))
}
```

The `is_selected` predicate, the row passes, and their behavior are
**unchanged** — they still produce only the `Selection` case (via `is_selected`)
and call `selection_colors` directly. This experiment only adds the dispatcher
and the config, unit-tested in isolation; a follow-up derives the full
`Selected` state (from the search highlight ranges) and routes the passes
through `selected_colors`.

## Scope / faithfulness notes

- **Ported (bridged)**: the `.search`/`.search_selected` color arms and the
  `Selected` enum — `selected_colors` computes any selected state's colors,
  reusing `selection_colors` because the search arms are the selection arms with
  a non-optional config.
- **Faithful**: the search arms are `selection_colors` with the search config in
  `Some` (so the plain-reverse `None` default never applies), matching
  upstream's non-optional `search_background`/`search_foreground` (and the
  `search-selected-*` pair); the four config defaults are upstream's
  `config/Config.zig` values (amber/salmon backgrounds, black foregrounds); a
  `False` state returns `None` so the caller keeps the base `cell_colors`
  (covering twist intact). The `.selection` dispatch is unchanged
  (`config.background`/`foreground`, still optional).
- **Faithful adaptation**: roastty reuses one `selection_colors` computation for
  all three selected states (upstream writes the arms out per state, but they
  are textually identical bar the optionality); `SelectionConfig` gains the four
  search values with a hand-written `Default` (the derived `Default` no longer
  applies once non-`Option` fields are present).
- **Deferred**: deriving the `Search`/`SearchSelected` states from per-row
  search highlight ranges (not yet on `RunOptions`) and routing the passes
  through `selected_colors`; the lock-cursor glyph + under-cursor recolor; the
  column-ordered decoration merge + link double-underline; the Metal upload.
  (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`:
   - add the `Selected` enum;
   - extend `SelectionConfig` with the four `search_*` fields and a hand-written
     `Default` (drop `Default` from the derive); the existing
     `SelectionConfig::default()` call sites are unaffected (selection stays
     `None`/`None`);
   - add the `selected_colors` dispatcher.
2. Tests (in `cell.rs`): a `selected_colors_*` test —
   - `False` → `None`;
   - `Selection` with the default config → the plain reverse (equal to
     `selection_colors(..., None, None)` / `cell_colors`-reverse:
     `bg = Some(default_fg)`, `fg = default_bg`);
   - `Search` with the default config → `bg = Some(amber)`, `fg = black` (the
     `.color` arms);
   - `SearchSelected` with the default config → `bg = Some(salmon)`,
     `fg = black`;
   - a `Search` with a **`CellForeground`/`CellBackground`** search config (and
     an inverse case) → the cell's resolved colors swapped, proving the search
     arm reuses the same inner switch as selection.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty selected_colors
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `selected_colors` returns `None` for `False` and the correct colors for
  `Selection`/`Search`/`SearchSelected` — reusing `selection_colors` with the
  per-state config, the search config non-optional, and the four defaults
  matching upstream;
- the tests pass (the four states + the `CellForeground`/`CellBackground` search
  case), and the existing tests still pass (the `SelectionConfig::default()`
  call sites unaffected);
- the search-range derivation, the pass routing, and the Metal upload stay
  deferred; the passes and `is_selected` are unchanged;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a search color is wrong (the search default applied
a plain reverse, the wrong default color, the inner switch diverging from
selection), the `False` case not returning `None`, the existing
`SelectionConfig::default()` behavior changing, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the byte-identical claim is correct — upstream's
`.search`/`.search_selected` arms use the same inner `TerminalColor` switch as
`.selection`, only without the optional `else` plain-reverse fallback — so
reusing `selection_colors` with `Some(search_config)` is faithful (the `None`
branch can never trigger for search states). It confirmed the four defaults
match upstream `Config.zig` (search background `#FFE082`, search foreground
black, search-selected background `#F2A57E`, search-selected foreground black).
It agreed that `selected_colors -> Option<CellColors>` is a clean dispatcher
shape (`False → None` lets callers preserve the base `cell_colors` path,
covering twist intact, while the three selected states route through the shared
computation), that leaving the passes and `is_selected` unchanged is sound for
this additive experiment, that the hand-written `Default` preserves existing
call-site behavior (`background`/ `foreground` stay `None`, so normal selection
still defaults to a plain reverse), and that the tests cover the four states
plus the important `CellForeground`/`CellBackground` search case including
inverse.

Review artifacts:

- Prompt: `logs/codex-review/20260603-201930-372328-prompt.md` (design)
- Result: `logs/codex-review/20260603-201930-372328-last-message.md` (design)

## Result

**Result:** Pass

The search-highlight color machinery is now in place.

- `roastty/src/renderer/cell.rs`:
  - the `Selected` enum (`False` / `Selection` / `Search` / `SearchSelected`,
    upstream's `selected` enum);
  - `SelectionConfig` gains four non-optional `search_*` fields
    (`search_background`/`search_foreground`/`search_selected_background`/
    `search_selected_foreground`) and a hand-written `Default` (replacing the
    derived one): `background`/`foreground` stay `None` (so the existing
    `SelectionConfig::default()` call sites keep their plain-reverse selection),
    and the four search defaults are upstream's `config/Config.zig` values
    (amber `#FFE082` / black search, salmon `#F2A57E` / black search-selected);
  - `selected_colors(selected, …, config) -> Option<CellColors>`: `False → None`
    (the caller keeps the base `cell_colors`, covering twist intact);
    `Selection` → `selection_colors(…, config.background, config.foreground)`;
    `Search` / `SearchSelected` → `selection_colors` with their non-optional
    config wrapped in `Some`, so the plain-reverse `None` default never applies
    — faithful to upstream's non-optional search config.
  - `is_selected` and both row passes are unchanged (still selection-only,
    calling `selection_colors` directly).

Test (in `cell.rs`): `selected_colors_dispatches_selection_and_search` —
`False → None`; `Selection` (default config) → the plain reverse
(`bg = Some(default_fg)`, `fg = default_bg`); `Search` → `bg = Some(amber)`,
`fg = black`; `SearchSelected` → `bg = Some(salmon)`, `fg = black`; and a
`CellForeground` search config proving the search arm reuses the selection inner
switch (non-inverse → `a`/`a`, inverse → `b`/`b`).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2845 passed, 0 failed (+1, no regressions; the
  hand-written `Default` keeps the existing `SelectionConfig::default()` call
  sites compiling and behaving identically).
- `cargo build -p roastty` → no warnings (the `pub(crate)` dispatcher is
  reachable in the library crate, so no dead-code warning despite no production
  caller yet).
- No-`ghostty`-name gates (font + renderer) clean; `git diff --check` clean.

## Conclusion

A search-highlighted cell's colors are now computable: `selected_colors` covers
all four `Selected` states, reusing `selection_colors` for the three selected
ones (the search arms being the selection arms with a non-optional config) and
returning `None` for `False`. The four search defaults match upstream. The
dispatcher returns the shared `CellColors`, ready for the passes to route
through it once the search highlight ranges are derived.

The remaining renderer-bridge work: deriving the `Search`/`SearchSelected`
states from per-row search highlight ranges (not yet on `RunOptions`) and
routing the passes through `selected_colors`; the lock-cursor glyph +
under-cursor text recolor; the column-ordered decoration merge + link
double-underline; and the **Metal upload** of `Contents`.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation matches the approved design:
`Selected` has the four upstream states; `SelectionConfig` carries the
non-optional search/search-selected colors with the correct upstream defaults
and a hand-written `Default` that preserves the existing selection behavior
(`background`/`foreground` stay `None`); `selected_colors` is faithful
(`False → None` preserving the base `cell_colors` path and covering twist,
`Selection` using the optional selection config, `Search`/`SearchSelected`
wrapping their non-optional configs in `Some` so the plain-reverse fallback
cannot apply); and the row passes and `is_selected` are unchanged, as intended
for this additive experiment. It confirmed the test covers the four dispatcher
states, the upstream default search colors, and a non-color search config
including inverse — enough to prove the search path reuses the same inner switch
rather than only the hardcoded color arms — with the diff internal Rust only and
no dead-code or ABI/header regression. Nothing needed to change before the
result commit.

Review artifacts:

- Result review: `logs/codex-review/20260603-202132-099337-last-message.md`
