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

# Experiment 507: the FontShapingBreak packed-struct config formatter (font-shaping-break)

## Description

Continuing the config formatter port (Experiments 491–506), this experiment
ports `format_entry` for `FontShapingBreak` — the `font-shaping-break` config
value. Upstream it is a `packed struct { cursor: bool = true }`, so it formats
through the generic packed-struct branch (`formatter.zig`), which writes the
comma-joined `[no-]field` keywords. This is the same shape already implemented
for `ScrollToBottom` (via the `EntryFormatter::entry_flags` helper, Experiment
499).

## Upstream behavior

Upstream `FontShapingBreak` (`Config.zig:8563`):

```zig
pub const FontShapingBreak = packed struct {
    cursor: bool = true,
};
```

The generic `formatEntry` packed-struct branch (`formatter.zig`) writes, for
each field in order, `no-{field}` when the bool is `false` and `{field}` when
`true`, comma-joined, as a single `name = …\n` line:

- `cursor = true` → `name = cursor\n`.
- `cursor = false` → `name = no-cursor\n`.

(With a single field there is no comma; the helper handles the general N-field
case.)

## Rust mapping (`roastty/src/config/mod.rs`)

The Rust `FontShapingBreak` is `struct { cursor: bool }` and is `Copy`. Its
`format_entry` mirrors `ScrollToBottom`'s, using the `entry_flags` helper:

```rust
impl FontShapingBreak {
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        formatter.entry_flags(&[("cursor", self.cursor)]);
    }
}
```

`entry_flags(&[(field, value), …])` joins each `[no-]field` keyword by `,` and
writes `name = …\n` — exactly the upstream packed-struct branch. With the single
`cursor` field it yields `cursor` or `no-cursor`.

## Scope / faithfulness notes

- **Ported (bridged)**: `FontShapingBreak::format_entry` (upstream's generic
  packed-struct `formatEntry` branch).
- **Faithful**: the single `cursor` field is written as `cursor` / `no-cursor`,
  exactly the upstream packed-struct branch (`[no-]field`, comma-joined). Reuses
  the same `entry_flags` helper proven for `ScrollToBottom`.
- **Faithful adaptation**: the comptime `inline for (fields)` packed-struct loop
  → the explicit `entry_flags(&[("cursor", self.cursor)])` slice; `formatEntry`
  → `entry_flags`.
- **Deferred**: the remaining config types' `format_entry`
  (`CustomShaderAnimation`, `MouseShiftCapture`), the other generic
  field-dispatch cases (float `{d}`, optional recurse), `QuickTerminalSize`, and
  the broader config parser/formatter.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `format_entry(self, …)` to
   `FontShapingBreak` (in a new `impl`, alongside its existing `Default`).
2. Tests (in `config/mod.rs`): `FontShapingBreak { cursor: true }` →
   `"a = cursor\n"`, `FontShapingBreak { cursor: false }` → `"a = no-cursor\n"`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty font_shaping_break_format_entry
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `FontShapingBreak::format_entry` writes `name = cursor\n` /
  `name = no-cursor\n` — faithful to upstream's packed-struct branch;
- the tests pass (both bool states), and the existing tests still pass;
- the remaining config types' formatters stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the output diverges from upstream, an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed `FontShapingBreak` is a one-field packed bool struct
with field name `cursor`, and the upstream packed-struct formatter emits the
field name or `no-` plus the field name (no comma needed for a single field)
(`Config.zig:8563`, `formatter.zig:98`); using
`entry_flags(&[("cursor", self.cursor)])` is the faithful Rust shape, and the
two tests cover both `true` and `false` output.

Review artifacts:

- Prompt: `logs/codex-review/20260604-162913-d507-prompt.md` (design)
- Result: `logs/codex-review/20260604-162913-d507-last-message.md` (design)

## Result

**Result:** Pass

`format_entry(self, …)` was added to `FontShapingBreak`, using the `entry_flags`
helper to write the single `cursor` field as `cursor` / `no-cursor` — exactly
the upstream packed-struct branch. The new test
`font_shaping_break_format_entry` covers both field states.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 2993 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation faithfully uses the packed-struct formatter shape
for the single `cursor` field — `cursor` when true, `no-cursor` when false
(`Config.zig:8563`, `formatter.zig:98`); the test covers both field states;
gates are clean. "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-163053-r507-prompt.md` (result)
- Result: `logs/codex-review/20260604-163053-r507-last-message.md` (result)

## Conclusion

`FontShapingBreak` — the second packed-struct/bool-keyword config formatter
(after `ScrollToBottom`) — now formats its single flag faithfully. The remaining
config-formatter work is `CustomShaderAnimation` and `MouseShiftCapture`, then
the remaining generic field-dispatch cases (float `{d}`, optional recurse),
`QuickTerminalSize`, and the full config loader, continuing toward the full
config formatter and loader.
