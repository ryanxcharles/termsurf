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

# Experiment 443: the click-action config enums (RightClickAction, MiddleClickAction)

## Description

This experiment ports the mouse click-action config enums the surface reads:
`RightClickAction` (what a right-click does) and `MiddleClickAction` (what a
middle-click does). Both are **dispatch enums** — their consumers are
side-effectful `Surface` switches (modify the selection, paste, copy, show a
menu), not pure functions — so this slice ports the enums and their exact
variant sets (no extracted method); the surface dispatch call sites stay
deferred. It continues diversifying the config-type family into the input/mouse
subsystem.

## Upstream behavior

In `config/Config.zig`, the two enums and their `Config` fields:

```zig
@"right-click-action": RightClickAction = .@"context-menu",
@"middle-click-action": MiddleClickAction = .@"primary-paste",

pub const RightClickAction = enum {
    /// No action is taken on right-click.
    ignore,
    /// Pastes from the system clipboard.
    paste,
    /// Copies the selected text to the system clipboard.
    copy,
    /// Copies the selected text ... and pastes the clipboard if no text is selected.
    @"copy-or-paste",
    /// Shows a context menu with options.
    @"context-menu",
};

pub const MiddleClickAction = enum {
    /// Paste from the selection/standard clipboard per `copy-on-select`.
    @"primary-paste",
    /// No action is taken on middle click.
    ignore,
};
```

The surface dispatches on each (`Surface.zig` —
`switch (self.config.right_click_action)` and the middle-click handler): e.g.
`right-click-action` of `context-menu` selects a word / link and shows the menu,
`copy`/`paste`/`copy-or-paste` touch the clipboard, `ignore` does nothing;
`middle-click-action` of `primary-paste` pastes per `copy-on-select`, `ignore`
does nothing. The dispatch bodies are imperative side effects, not pure logic.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// The `right-click-action` config (upstream `RightClickAction`): what a
/// right-click does. The `Config` default is `ContextMenu`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RightClickAction {
    /// No action on right-click.
    Ignore,
    /// Paste from the system clipboard.
    Paste,
    /// Copy the selected text to the system clipboard.
    Copy,
    /// Copy the selected text, or paste the clipboard if no text is selected.
    CopyOrPaste,
    /// Show a context menu.
    ContextMenu,
}

/// The `middle-click-action` config (upstream `MiddleClickAction`): what a
/// middle-click does. The `Config` default is `PrimaryPaste`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MiddleClickAction {
    /// Paste from the selection/standard clipboard per `copy-on-select`.
    PrimaryPaste,
    /// No action on middle-click.
    Ignore,
}
```

Both are plain enums (the dispatch is imperative, ported with the surface call
sites later). The names map the upstream hyphenated tags to Rust `CamelCase`
(`copy-or-paste` → `CopyOrPaste`, `context-menu` → `ContextMenu`,
`primary-paste` → `PrimaryPaste`).

## Scope / faithfulness notes

- **Ported (bridged)**: the `RightClickAction` and `MiddleClickAction` config
  enums (`config/Config.zig`).
- **Faithful**: `RightClickAction` has the five upstream variants (`ignore`,
  `paste`, `copy`, `copy-or-paste`, `context-menu`); `MiddleClickAction` has the
  two (`primary-paste`, `ignore`); the CamelCase names map the hyphenated tags
  exactly.
- **Faithful adaptation**: the `Config` field defaults (`.context-menu` /
  `.primary-paste`) are documented on the enums but kept off them (the other
  config types keep defaults on the deferred `Config` struct). No method is
  extracted — the consumers are imperative `Surface` dispatch (selection /
  clipboard / menu side effects), not pure functions, so they port with the call
  sites.
- **Deferred**: the `Config` struct / parsing (and the field defaults), and the
  surface dispatch call sites (the right-click / middle-click handlers and their
  selection / clipboard / context-menu side effects). (Consumed by a later
  slice; this experiment lands the enums.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add
     `pub(crate) enum RightClickAction { Ignore, Paste, Copy, CopyOrPaste, ContextMenu }`
     and `pub(crate) enum MiddleClickAction { PrimaryPaste, Ignore }` (both
     derive `Debug, Clone, Copy, PartialEq, Eq`).
2. Tests (in `config/mod.rs`):
   - `RightClickAction`: an array listing **every** variant with
     `assert_eq!(len, 5)` (locks the exact upstream set); plus a representative
     `assert_ne!` and a `Copy`/`Eq` round-trip.
   - `MiddleClickAction`: an array listing **every** variant with
     `assert_eq!(len, 2)`; plus `assert_ne!(PrimaryPaste, Ignore)` and a
     `Copy`/`Eq` round-trip.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty click_action
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `RightClickAction` has exactly the five upstream variants and
  `MiddleClickAction` exactly the two — faithful to `config/Config.zig`;
- the tests pass (the exact variant sets), and the existing tests still pass;
- the `Config` struct and the surface dispatch call sites stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if either enum is missing a variant or has an extra/
misnamed one, a default is wrongly encoded onto an enum, an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream:
`RightClickAction { Ignore, Paste, Copy, CopyOrPaste, ContextMenu }` matches
`ignore/paste/copy/copy-or-paste/context-menu` (`Config.zig:8633`);
`MiddleClickAction { PrimaryPaste, Ignore }` matches `primary-paste/ignore`
(`Config.zig:8652`); the defaults are correctly documented as deferred
Config-field defaults (`Config.zig:2433` / `:2443`); plain enums are the right
shape (the consumers are imperative `Surface` dispatch paths with selection,
clipboard, render, and menu side effects, `Surface.zig:4007` / `:4048`, so
extracting a pure helper would be artificial); porting the pair together is
appropriately bounded; and the exact-variant tests are adequate.

Review artifacts:

- Prompt: `logs/codex-review/20260604-104644-d443-prompt.md` (design)
- Result: `logs/codex-review/20260604-104644-d443-last-message.md` (design)

## Result

**Result:** Pass

The click-action config enums are now live.

- `roastty/src/config/mod.rs`:
  `pub(crate) enum RightClickAction { Ignore, Paste, Copy, CopyOrPaste, ContextMenu }`
  (upstream `RightClickAction`) and
  `pub(crate) enum MiddleClickAction { PrimaryPaste, Ignore }` (upstream
  `MiddleClickAction`), both deriving `Debug, Clone, Copy, PartialEq, Eq`. Plain
  enums (the consumers are imperative `Surface` dispatch, ported with the call
  sites later); the `Config` field defaults (`.context-menu` / `.primary-paste`)
  documented but kept off the enums.

Tests (in `config/mod.rs`):

- `right_click_action_has_the_five_upstream_variants` — an array of all five
  variants, `assert_eq!(len, 5)`, `assert_ne!(Ignore, ContextMenu)`,
  `Copy`/`Eq`.
- `middle_click_action_has_the_two_upstream_variants` — an array of both
  variants, `assert_eq!(len, 2)`, `assert_ne!(PrimaryPaste, Ignore)`,
  `Copy`/`Eq`.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2931 passed, 0 failed (+2, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

The config layer now carries the mouse click-action enums `RightClickAction` and
`MiddleClickAction`. These are dispatch enums (no extracted method — the
consumers are imperative `Surface` selection / clipboard / menu side effects),
so they land as plain enums with exact-variant-set tests, like the
background-image placement pair (Experiment 436). The `Config` struct / parsing
and the surface dispatch call sites stay deferred. The config-type family
remains a clean, gated way to advance the rewrite while the larger coupled
subsystems stay deferred.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed `RightClickAction` and `MiddleClickAction` carry
the exact upstream variant sets with the correct CamelCase mapping; keeping them
as plain enums is appropriate (their behavior belongs in the later `Surface`
dispatch port); the defaults are correctly documented but not encoded on the
enums; and the tests reference every variant. No public C ABI/header impact;
nothing needed to change before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-104839-r443-prompt.md` (result)
- Result: `logs/codex-review/20260604-104839-r443-last-message.md` (result)
