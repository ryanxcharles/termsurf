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

# Experiment 437: the font-shaping-break config type and its run-option consumer (FontShapingBreak, apply_break_config)

## Description

The renderer reads `config.font_shaping_break` and applies it to the shaper's
run options each `rebuildCells` (`run_iter_opts.applyBreakConfig(...)`). This
experiment ports that config type — `FontShapingBreak` (a single `cursor` flag,
default `true`) — **and its consumer**, `RunOptions::apply_break_config`, which
clears the run's `cursor_x` (disabling the break-around-cursor) when the flag is
off. roastty already has `RunOptions { cursor_x: Option<u16> }` (a faithful port
of upstream `shape.RunOptions`), so this slice lands both the type and its
behavior; the `rebuildCells` call site that invokes it stays deferred.

## Upstream behavior

In `config/Config.zig`, the config type and its `Config` field:

```zig
@"font-shaping-break": FontShapingBreak = .{},

pub const FontShapingBreak = packed struct {
    cursor: bool = true,
};
```

`FontShapingBreak` has one field, `cursor`, defaulting to `true` (the type's own
field default; the `Config` field `.{}` adopts it). In `font/shape.zig`, the run
options apply it:

```zig
pub const RunOptions = struct {
    // ...
    /// The cursor position within this row. ... This can be disabled by setting
    /// this to null.
    cursor_x: ?usize = null,

    /// Apply the font break configuration to the run.
    pub fn applyBreakConfig(
        self: *RunOptions,
        config: configpkg.FontShapingBreak,
    ) void {
        if (!config.cursor) self.cursor_x = null;
    }
};
```

When `cursor` is off, `applyBreakConfig` clears `cursor_x`, so the run iterator
does not break shaping at the cursor; when `cursor` is on (the default), it
leaves `cursor_x` as is.

## Rust mapping

`roastty/src/config/mod.rs` — the config type, with the intrinsic `true` default
implemented (Rust's derived `Default` for `bool` is `false`, so it is written by
hand to match upstream's `cursor: bool = true`):

```rust
/// The `font-shaping-break` config (upstream `FontShapingBreak`): which
/// boundaries break a shaping run. `cursor` (default `true`) breaks the run
/// around the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FontShapingBreak {
    /// Break a shaping run around the cursor.
    pub cursor: bool,
}

impl Default for FontShapingBreak {
    /// Upstream's field default `cursor: bool = true`.
    fn default() -> Self {
        Self { cursor: true }
    }
}
```

`roastty/src/font/run.rs` — the consumer on the existing `RunOptions`:

```rust
impl RunOptions {
    /// Apply the font break configuration to the run (upstream
    /// `RunOptions.applyBreakConfig`): when `cursor` breaking is off, clear
    /// `cursor_x` so the run iterator does not break shaping at the cursor.
    pub(crate) fn apply_break_config(&mut self, config: FontShapingBreak) {
        if !config.cursor {
            self.cursor_x = None;
        }
    }
}
```

`if !config.cursor { self.cursor_x = None; }` is upstream's
`if (!config.cursor) self.cursor_x = null;` exactly; when `cursor` is on,
`cursor_x` is left unchanged.

## Scope / faithfulness notes

- **Ported (bridged)**: the `FontShapingBreak` config type (`config/Config.zig`)
  and its consumer `RunOptions::apply_break_config` (`font/shape.zig`'s
  `applyBreakConfig`).
- **Faithful**: `FontShapingBreak` has the one `cursor` field defaulting to
  `true` (upstream's field default); `apply_break_config` clears `cursor_x` iff
  `cursor` is off, leaving it unchanged otherwise — exactly upstream.
- **Faithful adaptation**: upstream is a `packed struct` (bit-packed storage);
  in Rust it is a plain value struct (no ABI involved — internal config), so a
  derived layout is fine. The default is implemented by hand because Rust's
  derived `Default` for `bool` is `false`, not upstream's `true`.
- **Deferred**: the `Config` struct / parsing, and the renderer's `rebuildCells`
  call site that invokes `apply_break_config` on the run options
  (`run_iter_opts.applyBreakConfig(self.config.font_shaping_break)`). (Consumed
  by a later slice; this experiment lands the type and the run-option behavior.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add `pub(crate) struct FontShapingBreak { pub cursor: bool }` and a hand-
     written `impl Default` (`cursor: true`).
2. `roastty/src/font/run.rs`:
   - add `RunOptions::apply_break_config(&mut self, config: FontShapingBreak)`
     (`if !config.cursor { self.cursor_x = None; }`). Import `FontShapingBreak`
     from `crate::config`.
3. Tests:
   - `FontShapingBreak` (in `config/mod.rs`):
     `FontShapingBreak::default().cursor == true`; a `{ cursor: false }` value
     `!=` the default and round-trips `Copy`/`Eq`.
   - `apply_break_config` (in `font/run.rs`): a
     `RunOptions { cursor_x: Some(3), .. }`:
     - `apply_break_config(FontShapingBreak { cursor: false })` →
       `cursor_x == None`;
     - `apply_break_config(FontShapingBreak::default())` (cursor `true`) leaves
       a fresh `cursor_x == Some(3)` unchanged;
     - with `cursor_x == None` already, `cursor: false` keeps it `None`;
     - and `cells` / `selection` are untouched.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty font_shaping_break
cargo test -p roastty apply_break_config
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `FontShapingBreak` has the one `cursor` field defaulting to `true`, and
  `apply_break_config` clears `cursor_x` iff `cursor` is off (else leaves it) —
  faithful to upstream's type and `applyBreakConfig`;
- the tests pass (the default; the clear-on-off, leave-on-on, and already-`None`
  cases; the untouched fields), and the existing tests still pass;
- the `Config` struct and the `rebuildCells` call site stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the default is `false` (not `true`),
`apply_break_config` clears `cursor_x` when it should not (or fails to when
`cursor` is off), an unrelated field changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream: the config default
`@"font-shaping-break": FontShapingBreak = .{}` (`Config.zig:374`) with the
packed struct's `cursor: bool = true` (`Config.zig:8563`) is faithfully modeled
as a Rust value struct with a hand-written `Default { cursor: true }` (a derived
`Default` would wrongly make `cursor` false); `apply_break_config` exactly ports
`applyBreakConfig` (`shape.zig:88`,
`if (!config.cursor) self.cursor_x = null;`); the placement (type in
`config/mod.rs`, consumer on `RunOptions`) mirrors upstream's Config /
font-shape split, with the `rebuildCells` call site (`generic.zig:2672`)
appropriately deferred; and the tests cover the important behavior (default
`true`, distinctness, clear-on-off, leave-on-on, already-`None`, untouched
`cells`/`selection`).

Review artifacts:

- Prompt: `logs/codex-review/20260604-101558-d437-prompt.md` (design)
- Result: `logs/codex-review/20260604-101558-d437-last-message.md` (design)
