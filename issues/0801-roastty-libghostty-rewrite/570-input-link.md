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

# Experiment 570: the input Link type (regex → action)

## Description

This experiment ports upstream `input/Link.zig` — `Link`, a clickable terminal
element: a regular expression over terminal text that, when matched and clicked,
triggers an action. roastty has no `Link` type yet. It lands at `input::link`.
The one piece that depends on an external library — `oniRegex` (compiles the
regex with oniguruma) — is **deferred**, since roastty has no regex binding yet;
the data type, `equal`, and `clone` are ported.

## Upstream behavior

`input/Link.zig` is a struct with three fields and three methods:

- `regex: []const u8` — the regex to match (the caller owns this memory; the
  link never frees it).
- `action: Action` — a tagged union: `open` (open the matched value with the
  default opener) or `_open_osc8` (open the OSC8 hyperlink under the mouse; the
  leading underscore marks it internal-only, not user-specifiable).
- `highlight: Highlight` — when the link is highlighted (and thus clickable):
  `always`, `hover`, `always_mods: Mods` (highlight when the given modifiers are
  held — for `always`, all links highlight when the mods are pressed regardless
  of hover), or `hover_mods: Mods`.
- `oniRegex()` — builds an `oni.Regex` (oniguruma) from `regex`.
- `clone(alloc)` — deep clone, duplicating the `regex` bytes (action / highlight
  are copied).
- `equal(other)` — `std.meta.eql` on `action` and `highlight` plus `std.mem.eql`
  on `regex`.

(The `Mods` modifier set is `input/key.zig`'s `Mods`.)

## Rust mapping (`roastty/src/input/link.rs`)

A direct transcription. The regex is byte-oriented (`Vec<u8>`, mirroring
`[]const u8`); `clone` is the derived `Clone` (which duplicates the `Vec`,
exactly as upstream's `alloc.dupe`); and `equal` delegates to the derived
`PartialEq` (which compares all three fields — the same as upstream's
`meta.eql` + `mem.eql`).

```rust
//! A clickable terminal link: a regex over terminal text that triggers an action (port of
//! upstream `input/Link`).

use super::key_mods::Mods;

/// The action triggered when a link is clicked (upstream `Link.Action`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// Open the full matched value with the default opener (e.g. `open` on macOS).
    Open,
    /// Open the OSC8 hyperlink under the mouse. Internal-only (upstream's leading-underscore
    /// `_open_osc8` — not user-specifiable).
    OpenOsc8,
}

/// When a link is highlighted (and thus clickable) (upstream `Link.Highlight`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Highlight {
    /// Always highlight the link.
    Always,
    /// Only highlight while the mouse hovers over it.
    Hover,
    /// Highlight whenever the given modifiers are held (regardless of hover). Note: "shift" never
    /// matches in TUI programs that capture the mouse (the capture strips shift).
    AlwaysMods(Mods),
    /// Highlight while hovering with the given modifiers held.
    HoverMods(Mods),
}

/// A clickable link: a regex match over terminal text that triggers an action (upstream `Link`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Link {
    /// The regex used to match the link (byte string, mirroring upstream `[]const u8`).
    pub(crate) regex: Vec<u8>,
    /// The action triggered when the link is clicked.
    pub(crate) action: Action,
    /// When the link is highlighted / clickable.
    pub(crate) highlight: Highlight,
}

impl Link {
    /// Whether two links are equal (upstream `equal`): same action, highlight, and regex bytes.
    /// Delegates to the derived `PartialEq`, which compares all three fields.
    pub(crate) fn equal(&self, other: &Link) -> bool {
        self == other
    }
}
```

## Scope / faithfulness notes

- **Ported**: `input/Link.zig`'s data type → `input::link` (`Link`, `Action`,
  `Highlight`), plus `equal` and `clone`.
- **Faithful**: the `Action` variants (`open` → `Open`; the internal
  `_open_osc8` → `OpenOsc8`), the `Highlight` variants (`always` / `hover` /
  `always_mods(Mods)` / `hover_mods(Mods)`), `equal`'s three-field comparison
  (action + highlight + regex bytes), and `clone`'s deep copy of the regex are
  reproduced.
- **Faithful adaptation**: `regex: []const u8` → `regex: Vec<u8>`
  (byte-oriented; upstream's base `Link` borrows the caller-owned regex while
  `clone` duplicates it — roastty owns the bytes throughout, so the derived
  `Clone` is the deep clone); upstream's `equal` (`std.meta.eql` +
  `std.mem.eql`) becomes the derived `PartialEq` (an `equal` method is kept for
  API parity); `Mods` is `input::key_mods::Mods`.
- **Deferred**: `oniRegex` — building an `oni.Regex` requires an oniguruma (or
  equivalent) regex binding, which roastty does not have yet. The `regex` bytes
  are stored; compilation is left to a future experiment that introduces a regex
  dependency. This is the only piece not ported.
- No C ABI/header/ABI-inventory change (internal Rust). Adds `input::link`.

## Changes

1. `roastty/src/input/link.rs` (new): `Action`, `Highlight`, `Link` with `equal`
   as above.
2. `roastty/src/input/mod.rs`: add `#[allow(dead_code)] mod link;`
   (alphabetical).
3. Tests (in `link.rs`):
   - **equal**: two `Link`s with identical fields are `equal`; differing regex,
     action, or highlight makes them unequal.
   - **clone**: a cloned `Link` equals the original and owns a separate `regex`
     `Vec`.
   - **highlight mods**: `AlwaysMods` / `HoverMods` carrying `Mods` construct
     and compare by value (different mods ⇒ unequal).
   - **action variants**: `Open` and `OpenOsc8` are distinct.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty input::link
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/input/link.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Link` reproduces upstream's fields and variants (`Action`, `Highlight`),
  `equal`'s three-field comparison, and `clone`'s deep copy — faithful to
  `input/Link.zig` (with `oniRegex` deferred);
- the tests pass (equal / clone / highlight mods / action variants), and the
  existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the fields, variants, `equal`, or `clone` semantics
diverge from upstream, an unrelated item changes, or any public C API/ABI
changes.

## Design Review

Codex reviewed the design and **approved it with no findings**. It confirmed the
type shape is faithful (`Action::{Open, OpenOsc8}`, all four `Highlight` cases
carrying `Mods`, byte-oriented regex storage, and the derived `PartialEq` /
`Clone` matching upstream's field-wise `equal` and regex-byte-duplicating
`clone`), that owning `Vec<u8>` throughout is an acceptable documented
adaptation, and that deferring `oniRegex` is reasonable given there is no regex
binding yet. The planned tests cover equality, clone depth, modifier-payload
comparison, and action distinction.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d570-prompt.md`
- Result: `logs/codex-review/20260604-d570-last-message.md`

## Result

**Result:** Pass

`input::link` was added: `Action` (`Open` / `OpenOsc8`), `Highlight` (`Always` /
`Hover` / `AlwaysMods(Mods)` / `HoverMods(Mods)`, using
`input::key_mods::Mods`), and `Link` (`regex: Vec<u8>`, `action`, `highlight`,
deriving `Clone` / `PartialEq` / `Eq`) with an `equal` method delegating to
`==`. `oniRegex` is deferred (no regex binding yet). Registered via
`pub(crate) mod link;` in `input/mod.rs` (which carries crate-level
`#![allow(dead_code)]`).

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3155 passed, 0 failed (four new tests; no
  regressions, up from 3151).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + input/link.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

The four new tests: `equal` comparing all three fields (identical equal;
differing regex / action / highlight unequal), `clone` as a deep copy (clone
equals the original and owns a separate buffer — mutating one doesn't affect the
other), `Highlight` mods comparing by value
(`AlwaysMods(ctrl) == AlwaysMods(ctrl)`, `!= AlwaysMods(shift)`,
`AlwaysMods(ctrl) != HoverMods(ctrl)`), and `Action::Open` ≠ `Action::OpenOsc8`.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (one Nit: the `## Result` / `## Conclusion` sections were
not yet in the saved file — added here as part of result recording). Codex
confirmed the implementation matches upstream's data shape and the approved
adaptation — the action/highlight variants are correct, the `Mods` payloads
compare by value, `regex` is byte-oriented, the derived `Clone` deep-copies the
`Vec`, and `equal` is equivalent to upstream's field-wise comparison — and that
the tests cover the equality, clone, modifier, and action cases.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r570-prompt.md` (result)
- Result: `logs/codex-review/20260604-r570-last-message.md` (result)

## Conclusion

`input::link::Link` is ported from `input/Link.zig` — a clickable
regex-over-terminal-text link (`Action`, `Highlight`, `equal`, `clone`). The
byte-faithfulness theme continued (`regex: Vec<u8>` for upstream's
`[]const u8`), and Rust's derived `PartialEq` / `Clone` exactly capture
upstream's `std.meta.eql` + `std.mem.eql` `equal` and regex-duplicating `clone`.
The one external-dependency piece, `oniRegex`, is **deferred** — it needs an
oniguruma (or equivalent) regex binding that roastty does not yet have; that
binding (and wiring `Link` into URL/hyperlink detection) is a natural future
slice. Other unported leaves include `terminal/ScreenSet`, `config/edit`,
`terminal/osc/parsers/clipboard_operation`; the big-ticket subsystems remain
`datastruct/split_tree` (2517 lines) and the terminal **search subsystem**
(coupled to `PageList` / `Pin` / `Screen` / `Selection` / `PageFormatter`). The
objc/bundle-id helpers, the `home()` resolver, and config `loadDefaultFiles`
remain deferred pending roastty's naming decision; `background-image-opacity`
stays float-blocked.
