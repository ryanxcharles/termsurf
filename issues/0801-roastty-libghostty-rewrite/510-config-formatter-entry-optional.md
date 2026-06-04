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

# Experiment 510: the optional-recurse field-dispatch case (EntryFormatter::entry_optional)

## Description

Continuing the config formatter port (Experiments 491–509), this experiment adds
the **optional** generic field-dispatch case to `EntryFormatter` as
`entry_optional`. Upstream's generic `formatEntry` handles a `?T` field by
recursing into the inner value's formatter (same `name`) when present, and
writing the void line `name = \n` when absent. This is a reusable helper (like
the packed-struct `entry_flags` of Experiment 499), not tied to one config type
— many aggregate `Config` fields are `?T`.

## Upstream behavior

The generic `formatEntry` optional branch (`config/formatter.zig:62`):

```zig
.optional => |info| {
    if (value) |inner| {
        try formatEntry(info.child, name, inner, writer);
    } else {
        try writer.print("{s} = \n", .{name});
    }
    return;
},
```

So a `?T`:

- when `Some(inner)` → recurses into `formatEntry` for the inner type with the
  **same `name`** (e.g. an `?Color` writes `name = #rrggbb\n`).
- when `None` → writes `name = \n` (identical to the `void` branch,
  `formatter.zig:57`).

## Rust mapping (`roastty/src/config/formatter.rs`)

The inner type varies per field, so the recursion is expressed with a closure
the caller supplies (mirroring upstream's comptime dispatch on `info.child`).
The `None` arm reuses `entry_void` (which already writes `name = \n`, the exact
text of upstream's `else` arm):

```rust
/// `name = <inner>\n` when present, else `name = \n` (upstream the `optional`
/// case): when `Some`, recurse into the inner value's formatter with the same
/// name; when `None`, write the void line.
pub(crate) fn entry_optional<T>(&mut self, value: Option<T>, fmt_inner: impl FnOnce(T, &mut Self)) {
    match value {
        Some(inner) => fmt_inner(inner, self),
        None => self.entry_void(),
    }
}
```

`value: Option<T>` accepts both `Option<CopyType>` (passed by value) and
`Option<&OwnedType>` (via the caller's `.as_ref()`, so `T = &OwnedType`). The
`fmt_inner` closure receives the inner value and `&mut Self` and formats it with
the same `name` — exactly upstream's `formatEntry(info.child, name, inner, …)`.
The `None` arm delegates to `entry_void`, matching upstream's `name = \n`.

## Scope / faithfulness notes

- **Ported (bridged)**: the generic `formatEntry` **optional** branch, as the
  reusable `EntryFormatter::entry_optional` helper.
- **Faithful**: `Some` recurses into the inner formatter with the same `name`
  (caller-supplied closure standing in for upstream's comptime `info.child`
  dispatch); `None` writes `name = \n` (the same text as the `void` branch, via
  `entry_void`).
- **Faithful adaptation**: upstream's comptime recursion on `info.child` → a
  generic closure parameter; the `None` `writer.print("{s} = \n")` →
  `self.entry_void()`.
- **Deferred**: the generic float `{d}` field-dispatch case (float-formatting
  blocked, Experiment 509), `QuickTerminalSize`, and the full config loader /
  aggregate per-field dispatch.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/formatter.rs`: add `entry_optional<T>` to
   `EntryFormatter`.
2. Tests (in `formatter.rs`): `Some` recurses (e.g.
   `entry_optional(Some("v"), |v, f| f.entry_str(v))` → `"a = v\n"`;
   `entry_optional(Some(true), |v, f| f.entry_bool(v))` → `"a = true\n"`) and
   `None` writes the void line
   (`entry_optional(None::<bool>, |v, f| f.entry_bool(v))` → `"a = \n"`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty entry_optional
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `entry_optional` recurses into the inner formatter (same `name`) for `Some`
  and writes `name = \n` for `None` — faithful to upstream's optional branch;
- the tests pass (both `Some` and `None`, recursing through a couple of inner
  formatters), and the existing tests still pass;
- the float / aggregate field-dispatch cases stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the behavior diverges from upstream's optional
branch, an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the closure-based helper is a faithful Rust stand-in
for the upstream optional branch — `Some(inner)` delegates formatting of the
child value using the same formatter/name, and `None` writes the same empty
field as the void branch (`name = \n`) (`formatter.zig:57`/`:62`); passing the
inner by value is fine for this helper shape (callers pass `Option<T>` for copy
values or `Option<&T>` via `.as_ref()` for owned values); and the proposed tests
cover string, bool, and `None`.

Review artifacts:

- Prompt: `logs/codex-review/20260604-164052-d510-prompt.md` (design)
- Result: `logs/codex-review/20260604-164052-d510-last-message.md` (design)
