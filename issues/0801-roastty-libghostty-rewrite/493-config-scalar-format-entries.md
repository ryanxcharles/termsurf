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

# Experiment 493: scalar config formatters (WorkingDirectory / WindowPadding / BackgroundBlur format_entry)

## Description

Continuing the config **formatter** layer (Experiments 491–492), this experiment
ports three scalar `formatEntry` methods that use the `EntryFormatter`
primitives directly: `WorkingDirectory.formatEntry` (a keyword or path string),
`WindowPadding.formatEntry` (one int, or a `left,right` pair), and
`BackgroundBlur.formatEntry` (a bool, an int radius, or a glass keyword). Each
is a mechanical mirror of its upstream `formatEntry`, grounded by the
`EntryFormatter` from Experiment 491.

## Upstream behavior

In `config/Config.zig`:

```zig
// WorkingDirectory
pub fn formatEntry(self: Self, formatter: formatterpkg.EntryFormatter) !void {
    switch (self) {
        .home, .inherit => try formatter.formatEntry([]const u8, @tagName(self)),
        .path => |path| try formatter.formatEntry([]const u8, path),
    }
}

// WindowPadding
pub fn formatEntry(self: Self, formatter: formatterpkg.EntryFormatter) !void {
    var buf: [128]u8 = undefined;
    if (self.top_left == self.bottom_right) {
        try formatter.formatEntry([]const u8, std.fmt.bufPrint(&buf, "{}", .{self.top_left}) catch ...);
    } else {
        try formatter.formatEntry([]const u8, std.fmt.bufPrint(&buf, "{},{}", .{ self.top_left, self.bottom_right }) catch ...);
    }
}

// BackgroundBlur
pub fn formatEntry(self: BackgroundBlur, formatter: anytype) !void {
    switch (self) {
        .false => try formatter.formatEntry(bool, false),
        .true => try formatter.formatEntry(bool, true),
        .radius => |v| try formatter.formatEntry(u8, v),
        .@"macos-glass-regular" => try formatter.formatEntry([]const u8, "macos-glass-regular"),
        .@"macos-glass-clear" => try formatter.formatEntry([]const u8, "macos-glass-clear"),
    }
}
```

- `WorkingDirectory`: `home` / `inherit` write their tag name; `path` writes the
  path string. All as string entries (`name = …\n`).
- `WindowPadding`: when both edges are equal, write the single value
  (`name = N\n`); otherwise write `name = left,right\n`.
- `BackgroundBlur`: `false` / `true` write a bool entry (`name = false\n` /
  `name = true\n`); `radius` writes the `u8` (`name = v\n`); the glass variants
  write their keyword string.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
impl WorkingDirectory {
    /// Format as a config entry (upstream `WorkingDirectory.formatEntry`): the
    /// `home` / `inherit` keyword, or the path.
    pub(crate) fn format_entry(&self, formatter: &mut EntryFormatter) {
        match self {
            WorkingDirectory::Home => formatter.entry_str("home"),
            WorkingDirectory::Inherit => formatter.entry_str("inherit"),
            WorkingDirectory::Path(path) => formatter.entry_str(path),
        }
    }
}

impl WindowPadding {
    /// Format as a config entry (upstream `WindowPadding.formatEntry`): one value
    /// when both edges are equal, else `left,right`.
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        if self.top_left == self.bottom_right {
            formatter.entry_int(self.top_left);
        } else {
            formatter.entry_str(&format!("{},{}", self.top_left, self.bottom_right));
        }
    }
}

impl BackgroundBlur {
    /// Format as a config entry (upstream `BackgroundBlur.formatEntry`): a bool, an
    /// int radius, or a glass keyword.
    pub(crate) fn format_entry(self, formatter: &mut EntryFormatter) {
        match self {
            BackgroundBlur::False => formatter.entry_bool(false),
            BackgroundBlur::True => formatter.entry_bool(true),
            BackgroundBlur::Radius(v) => formatter.entry_int(v),
            BackgroundBlur::MacosGlassRegular => formatter.entry_str("macos-glass-regular"),
            BackgroundBlur::MacosGlassClear => formatter.entry_str("macos-glass-clear"),
        }
    }
}
```

Each mirrors upstream: `WorkingDirectory` writes the keyword `@tagName` / the
path string; `WindowPadding` writes the single int (`entry_int`, the
`bufPrint("{}")` equivalent) or the `left,right` string; `BackgroundBlur` writes
a bool / int / keyword. `WorkingDirectory::format_entry` takes `&self` (it holds
a non-`Copy` `Path(String)`); `WindowPadding` / `BackgroundBlur` are `Copy`, so
`self` by value.

## Scope / faithfulness notes

- **Ported (bridged)**: `WorkingDirectory::format_entry`,
  `WindowPadding::format_entry`, and `BackgroundBlur::format_entry` (upstream's
  respective `formatEntry`).
- **Faithful**: `WorkingDirectory` — the `home` / `inherit` keyword and the
  path, all as string entries; `WindowPadding` — the single-value `name = N\n`
  (edges equal) vs the `name = left,right\n` pair; `BackgroundBlur` — the bool /
  radius-int / glass-keyword entries — exactly upstream's `formatEntry`.
- **Faithful adaptation**: `formatEntry([]const u8, @tagName/str)` →
  `entry_str`; `formatEntry(bool, …)` → `entry_bool`; `formatEntry(u8, …)` →
  `entry_int`; `bufPrint("{}")` / `bufPrint("{},{}")` → `entry_int` /
  `entry_str(&format!(…))`.
- **Deferred**: the remaining types' `formatEntry` (ported in later slices) and
  the generic field-dispatch `formatEntry`, and the broader config
  parser/formatter.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `WorkingDirectory::format_entry`,
   `WindowPadding::format_entry`, and `BackgroundBlur::format_entry` (each in
   its existing `impl`).
2. Tests (in `config/mod.rs`):
   - `WorkingDirectory`: `Home` → `"a = home\n"`; `Inherit` → `"a = inherit\n"`;
     `Path("/x")` → `"a = /x\n"`.
   - `WindowPadding`: `{5, 5}` → `"a = 5\n"`; `{3, 7}` → `"a = 3,7\n"`.
   - `BackgroundBlur`: `False` → `"a = false\n"`; `True` → `"a = true\n"`;
     `Radius(42)` → `"a = 42\n"`; `MacosGlassRegular` →
     `"a = macos-glass-regular\n"`; `MacosGlassClear` →
     `"a = macos-glass-clear\n"`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty format_entry
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- the three `format_entry` methods write their entries (keyword/path; single-int
  or `left,right`; bool/radius/glass) exactly as upstream's `formatEntry`;
- the tests pass (each method's cases), and the existing tests still pass;
- the other types' `formatEntry` and the generic field-dispatch stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a formatted entry differs from upstream (wrong
keyword/separator/value, wrong equal-vs-pair branch), an unrelated item changes,
or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed all three methods match upstream output semantics:
`WorkingDirectory` writes the exact tag names for `home` / `inherit` and the
path string for `Path` (`Config.zig:5361`); `WindowPadding` writes a single
value when both sides match, else `left,right` with no spaces (`:10142`);
`BackgroundBlur` writes bools for `false` / `true`, a decimal integer for the
radius, and the exact glass keyword strings (`:9740`); using `entry_int` for the
equal `WindowPadding` case is output-equivalent to upstream's `bufPrint("{}")` +
string formatting; and the proposed tests cover the relevant line shapes and
exact keywords.

Review artifacts:

- Prompt: `logs/codex-review/20260604-151722-d493-prompt.md` (design)
- Result: `logs/codex-review/20260604-151722-d493-last-message.md` (design)
