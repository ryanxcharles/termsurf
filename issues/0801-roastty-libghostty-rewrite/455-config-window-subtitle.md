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

# Experiment 455: the window-subtitle config enum and its predicate (WindowSubtitle, shows_working_directory)

## Description

This experiment ports the `window-subtitle` config enum —
`WindowSubtitle { False, WorkingDirectory }` — **and the predicate** the apprt
window uses to decide whether the window subtitle shows the working directory.
Upstream's window switches on the config: `false` shows no subtitle, and
`working-directory` shows the process's working directory. This experiment
captures the `== .working-directory` decision as a
`WindowSubtitle::shows_working_directory` method; the apprt subtitle string
handling (duplicating the pwd) stays deferred.

## Upstream behavior

In `config/Config.zig`, the enum and its `Config` field (default `.false`):

```zig
@"window-subtitle": WindowSubtitle = .false,

pub const WindowSubtitle = enum {
    false,
    @"working-directory",
};
```

In the apprt window (`apprt/gtk/class/window.zig`), the subtitle is chosen from
the config and the working directory:

```zig
return switch (config.@"window-subtitle") {
    .false => null,
    .@"working-directory" => pwd: {
        const pwd = pwd_ orelse return null;
        break :pwd glib.ext.dupeZ(u8, std.mem.span(pwd));
    },
};
```

`false` shows no subtitle (`null`); `working-directory` shows the working
directory (the supplied `pwd`, when present). The config's decision is whether
to show the working directory — `== .working-directory`.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// The `window-subtitle` config (upstream `WindowSubtitle`): what the window
/// subtitle shows. The `Config` default is `False`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowSubtitle {
    /// No subtitle.
    False,
    /// Show the working directory.
    WorkingDirectory,
}

impl WindowSubtitle {
    /// Whether the subtitle shows the working directory (upstream's apprt
    /// `== .working-directory` decision): `true` only for `WorkingDirectory`.
    pub(crate) fn shows_working_directory(self) -> bool {
        matches!(self, WindowSubtitle::WorkingDirectory)
    }
}
```

`shows_working_directory` is the `== .working-directory` decision (`true` only
for `WorkingDirectory`). The `matches!` is exhaustive. `WindowSubtitle` is
`Copy`/`Eq`.

## Scope / faithfulness notes

- **Ported (bridged)**: the `WindowSubtitle` config enum (`config/Config.zig`)
  and its predicate (`WindowSubtitle::shows_working_directory`, upstream's apprt
  `== .working-directory` decision).
- **Faithful**: the enum has the two upstream variants (`false`,
  `working-directory`); `shows_working_directory` returns `true` only for
  `WorkingDirectory` — exactly the upstream decision (the `false` arm shows no
  subtitle).
- **Faithful adaptation**: the upstream tag `working-directory` (not a valid
  Rust identifier) maps to `WorkingDirectory`; the consumer is modeled as a
  method returning the "show the working directory" decision (upstream then
  duplicates the pwd string, the deferred apprt part).
- **Deferred**: the `Config` struct / parsing (and the `.false` field default),
  and the apprt subtitle string handling (resolving / duplicating the pwd) that
  consumes the decision. (Consumed by a later slice; this experiment lands the
  enum and the predicate.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add `pub(crate) enum WindowSubtitle { False, WorkingDirectory }` (derive
     `Debug, Clone, Copy, PartialEq, Eq`) and
     `WindowSubtitle::shows_working_directory(self) -> bool`
     (`matches!(self, WindowSubtitle::WorkingDirectory)`).
2. Tests (in `config/mod.rs`):
   - `shows_working_directory`: `False.shows_working_directory() == false`,
     `WorkingDirectory.shows_working_directory() == true`; the variants distinct
     and a `Copy`/`Eq` round-trip.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty window_subtitle
cargo test -p roastty shows_working_directory
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `WindowSubtitle` has the two upstream variants and `shows_working_directory`
  returns `true` only for `WorkingDirectory` — faithful to upstream's enum and
  the apprt `== .working-directory` decision;
- the tests pass (the predicate; the distinct variants), and the existing tests
  still pass;
- the `Config` struct and the apprt subtitle string handling stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a variant is missing/extra,
`shows_working_directory` maps a variant the wrong way, an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream: the variants match
exactly (`false`, `working-directory`, `Config.zig:5277`); `WorkingDirectory` is
the right Rust mapping for the hyphenated tag; the default `.false` is correctly
documented as deferred to the future `Config` field (`Config.zig:2110`);
`shows_working_directory()` correctly extracts the app-side decision (`false`
yields no subtitle, `working-directory` uses `pwd` if present,
`window.zig:1245`); deferring the actual `pwd` string handling is the right
boundary; and the two-variant test is adequate.

Review artifacts:

- Prompt: `logs/codex-review/20260604-113721-d455-prompt.md` (design)
- Result: `logs/codex-review/20260604-113721-d455-last-message.md` (design)

## Result

**Result:** Pass

The window-subtitle config enum and its predicate are now live.

- `roastty/src/config/mod.rs`:
  `pub(crate) enum WindowSubtitle { False, WorkingDirectory }` (upstream
  `WindowSubtitle`) and `WindowSubtitle::shows_working_directory(self) -> bool`
  (`matches!(self, WindowSubtitle::WorkingDirectory)`), the extraction of
  upstream's apprt `== .working-directory` decision.

Test (in `config/mod.rs`):
`window_subtitle_shows_working_directory_only_for_that_variant` —
`False.shows_working_directory() == false`,
`WorkingDirectory.shows_working_directory() == true`; the variants distinct;
`Copy`/`Eq`.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2943 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

The config layer now carries `WindowSubtitle` and its predicate — the first
config slice to reach the window / apprt subtitle. The `Config` struct / parsing
and the apprt subtitle string handling (resolving / duplicating the pwd) stay
deferred. The config-type family — now sixteen enums/flag-structs with consumers
plus three color value types — remains a clean, gated way to advance the rewrite
while the larger coupled subsystems stay deferred.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed `WindowSubtitle { False, WorkingDirectory }`
faithfully maps the upstream enum; `shows_working_directory()` correctly
captures the only non-false behavior; deferring the actual `pwd` resolution and
string handling remains the right scope; and the test covers both variants and
value semantics. No public C ABI/header impact; nothing needed to change before
the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-113910-r455-prompt.md` (result)
- Result: `logs/codex-review/20260604-113910-r455-last-message.md` (result)
