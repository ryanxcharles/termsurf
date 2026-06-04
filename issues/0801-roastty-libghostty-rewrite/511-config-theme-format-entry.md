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

# Experiment 511: the Theme config formatter (theme)

## Description

Continuing the config formatter port (Experiments 491–510), this experiment
ports `format_entry` for `Theme` — the `theme` config value. `Theme` is a struct
with `light` / `dark` theme-name strings and a **custom `formatEntry`**: it
writes the single name when light and dark match, else the paired
`light:…,dark:…` string. This is a prerequisite for the aggregate config
formatter (the next experiment), where `theme: Option<Theme>` is one of the
fields. Grounded by the `EntryFormatter` from Experiment 491.

(The Design Review of the would-be aggregate experiment flagged that
`Theme::format_entry` did not yet exist; this experiment ports it first.)

## Upstream behavior

Upstream `Theme.formatEntry` (`Config.zig:9898`):

```zig
pub fn formatEntry(self: Theme, formatter: anytype) !void {
    var buf: [4096]u8 = undefined;
    if (std.mem.eql(u8, self.light, self.dark)) {
        try formatter.formatEntry([]const u8, self.light);
        return;
    }
    const str = std.fmt.bufPrint(&buf, "light:{s},dark:{s}", .{ self.light, self.dark }) catch
        return error.OutOfMemory;
    try formatter.formatEntry([]const u8, str);
}
```

So `Theme`:

- when `light == dark` → writes the single name as a string: `name = {light}\n`
  (no length cap).
- otherwise → writes `name = light:{light},dark:{dark}\n` (the string branch,
  `name = value\n`).

The unequal branch uses a `[4096]u8` stack buffer: if `light:…,dark:…` exceeds
4096 bytes, `bufPrint` fails and `formatEntry` returns `error.OutOfMemory`,
which propagates and aborts the whole config dump. This 4096-byte guard can only
trigger for theme names totalling ~4 KB; it is unreachable for real theme names
(short identifiers like `catppuccin-mocha`). Because the Rust `format_entry` API
is infallible (`-> ()`), the unreachable abort path is **not modeled** — see the
faithfulness note.

## Rust mapping (`roastty/src/config/mod.rs`)

`Theme` holds owned `String`s (it is `Clone`, not `Copy`), so `format_entry`
takes `&self`:

```rust
impl Theme {
    /// Format as a config entry (upstream `Theme.formatEntry`): the single name
    /// when light and dark match, else `light:{light},dark:{dark}`.
    pub(crate) fn format_entry(&self, formatter: &mut EntryFormatter) {
        if self.light == self.dark {
            formatter.entry_str(&self.light);
            return;
        }
        formatter.entry_str(&format!("light:{},dark:{}", self.light, self.dark));
    }
}
```

`entry_str(value)` writes `name = value\n`, matching upstream's string
`formatEntry` for both arms. The equal arm passes `light` (upstream's
`self.light`); the unequal arm passes the `light:…,dark:…` string.

## Scope / faithfulness notes

- **Ported (bridged)**: `Theme::format_entry` (upstream's custom `formatEntry`).
- **Faithful**: the two arms map to upstream's two cases — the single `light`
  name when `light == dark`, else `light:{light},dark:{dark}` — each written as
  `name = value\n` (the string `formatEntry` shape).
- **Faithful adaptation**: `std.mem.eql(u8, light, dark)` →
  `self.light == self.dark`; `bufPrint("light:{s},dark:{s}", …)` →
  `format!("light:{},dark:{}", …)`; `formatter.formatEntry([]const u8, …)` →
  `entry_str(…)`. `format_entry` takes `&self` (owned `String`s; upstream's
  strings are arena slices).
- **Documented narrowing**: upstream's unequal branch errors (`OutOfMemory`,
  aborting the dump) if `light:…,dark:…` exceeds 4096 bytes. The Rust API is
  infallible and `format!` is unbounded, so this guard is not modeled. It is
  **unreachable** for real theme names (it needs ~4 KB of theme-name text); the
  narrowing has no effect on any reachable input.
- **Deferred**: the aggregate `Config` formatter (next experiment, which
  consumes this), `background-image-opacity` (float-blocked),
  `QuickTerminalSize`, and the config loader.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `format_entry(&self, …)` to `Theme`'s
   existing `impl` (alongside `single`).
2. Tests (in `config/mod.rs`): `Theme::single("foo".into())` → `"a = foo\n"`
   (equal arm); `Theme { light: "day".into(), dark: "night".into() }` →
   `"a = light:day,dark:night\n"` (unequal arm).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty theme_format_entry
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Theme::format_entry` writes the single name when `light == dark` and
  `light:{light},dark:{dark}` otherwise — faithful to upstream's `formatEntry`;
- the tests pass (both arms), and the existing tests still pass;
- the aggregate formatter and the float field stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if an arm's output diverges from upstream, an unrelated
item changes, or any public C API/ABI changes.

## Design Review

This experiment's first form was the aggregate config formatter; Codex's design
review of that flagged a **Required** issue — `Theme::format_entry` was not yet
ported (only `Theme::single` existed) — so the work was split: this Experiment
511 ports `Theme::format_entry` first, and the aggregate formatter becomes the
next experiment.

Codex then reviewed this `Theme::format_entry` design and **approved** it with
**no findings**. Both content arms are faithful for normal formatter output —
equal light/dark writes the single theme name, and unequal writes
`light:{light},dark:{dark}` as one string entry (`Config.zig:9898`); the tests
cover both upstream branches. Leaving the unequal-branch 4096-byte `bufPrint`
failure unmodeled is acceptable within the infallible formatter layer; Codex
explicitly advised **not** to "skip the entry" (emitting the full string is more
faithful than dropping it), and that exact fixed-buffer OOM behavior, if ever a
goal, should be a formatter-wide `Result` design rather than ad hoc per-entry
dropping.

Review artifacts:

- Prompt: `logs/codex-review/20260604-165116-d511-prompt.md` (design)
- Result: `logs/codex-review/20260604-165116-d511-last-message.md` (design)

## Result

**Result:** Pass

`format_entry(&self, …)` was added to `Theme`'s existing `impl`: the equal arm
(`light == dark`) writes the single name via `entry_str(&self.light)`, and the
unequal arm writes `light:{light},dark:{dark}`. The 4096-byte `bufPrint` cap is
left unmodeled (unreachable; infallible API) per the design review. The new test
`theme_format_entry` covers both arms.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 2997 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation matches upstream `Theme.formatEntry` — equal
light/dark writes the single theme string, and unequal writes
`light:{light},dark:{dark}` as one string entry (`Config.zig:9898`); leaving the
fixed-buffer OOM path unmodeled is consistent with the infallible Rust formatter
API; the test covers both branches; gates are clean. "Approved with no
findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-165250-r511-prompt.md` (result)
- Result: `logs/codex-review/20260604-165250-r511-last-message.md` (result)

## Conclusion

`Theme::format_entry` — the last per-field config formatter needed before the
aggregate dump — now formats its `light` / `light:…,dark:…` cases faithfully.
Every non-float `Config` leaf type now has a `format_entry`. The next experiment
is the **aggregate per-field config formatter** (`Config::format_config`),
walking the `Config` struct in upstream declaration order and emitting each
field via its `format_entry` (or `entry_bool` / `entry_optional`), omitting only
the float-blocked `background-image-opacity` (Experiment 509); the field-order
analysis for all 44 keys is already done and Codex-verified. After that comes
the config loader (`loadCli`, file I/O).
