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

# Experiment 543: environment-variable composition (os::env)

## Description

Continuing the `os` module (Experiments 541–542), this experiment ports the
**environment-variable composition helpers** from upstream `os/env.zig`:
`append_env`, `append_env_always`, and `prepend_env`. These build `PATH`-style
variables — joining a new value onto an existing one with the platform path
delimiter (`:` on macOS) — which the eventual termio layer uses to set up the
child process environment (e.g. prepend roastty's resources to `PATH`, append to
`MANPATH`). They are pure string logic with upstream tests and are genuinely
unported.

## Upstream behavior

`os/env.zig` (the composition helpers; `std.fs.path.delimiter` is `:` on macOS):

```zig
/// Append a value to an environment variable such as PATH. Always allocates.
pub fn appendEnv(alloc, current: []const u8, value: []const u8) ![]u8 {
    if (current.len == 0) return try alloc.dupe(u8, value);   // no prior value ⇒ as-is
    return try appendEnvAlways(alloc, current, value);
}

/// Always append, even when current is empty. Useful for vars (like MANPATH) that want an
/// empty prefix to preserve existing values.
pub fn appendEnvAlways(alloc, current: []const u8, value: []const u8) ![]u8 {
    return try std.fmt.allocPrint(alloc, "{s}{c}{s}", .{ current, std.fs.path.delimiter, value });
}

/// Prepend a value to an environment variable such as PATH. Always allocates.
pub fn prependEnv(alloc, current: []const u8, value: []const u8) ![]u8 {
    if (current.len == 0) return try alloc.dupe(u8, value);
    return try std.fmt.allocPrint(alloc, "{s}{c}{s}", .{ value, std.fs.path.delimiter, current });
}
```

- `appendEnv`: empty `current` ⇒ `value` as-is; else `current:value`.
- `appendEnvAlways`: always `current:value` (so empty `current` ⇒ `:value`).
- `prependEnv`: empty `current` ⇒ `value` as-is; else `value:current`.

The upstream tests (the macOS/posix arm): `appendEnv("", "foo") == "foo"`;
`appendEnv("a:b", "foo") == "a:b:foo"`; `prependEnv("", "foo") == "foo"`;
`prependEnv("a:b", "foo") == "foo:a:b"`.

## Rust mapping (`roastty/src/os/env.rs`)

Byte-faithful `&OsStr -> OsString` helpers (env values are byte sequences on
POSIX and may contain non-UTF-8 path components), with a `:` delimiter pushed
via `OsString::push`. The allocator-returning `![]u8` becomes an owned
`OsString` (no caller free), matching the "always allocated" contract:

```rust
//! Environment-variable helpers (port of upstream `os/env`).

use std::ffi::{OsStr, OsString};

/// The platform `PATH`-style delimiter (`std.fs.path.delimiter`; `:` on macOS).
const DELIMITER: &str = ":";

/// Append `value` to an environment variable such as `PATH` (upstream `os.env.appendEnv`).
/// An empty `current` returns `value` as-is; otherwise `current:value`.
pub(crate) fn append_env(current: &OsStr, value: &OsStr) -> OsString {
    if current.is_empty() {
        return value.to_os_string();
    }
    append_env_always(current, value)
}

/// Always append `value`, even when `current` is empty (upstream `os.env.appendEnvAlways`).
/// Useful for vars like `MANPATH` that want an empty prefix to preserve existing values, so
/// an empty `current` yields `:value`.
pub(crate) fn append_env_always(current: &OsStr, value: &OsStr) -> OsString {
    let mut result = OsString::with_capacity(current.len() + DELIMITER.len() + value.len());
    result.push(current);
    result.push(DELIMITER);
    result.push(value);
    result
}

/// Prepend `value` to an environment variable such as `PATH` (upstream `os.env.prependEnv`).
/// An empty `current` returns `value` as-is; otherwise `value:current`.
pub(crate) fn prepend_env(current: &OsStr, value: &OsStr) -> OsString {
    if current.is_empty() {
        return value.to_os_string();
    }
    let mut result = OsString::with_capacity(value.len() + DELIMITER.len() + current.len());
    result.push(value);
    result.push(DELIMITER);
    result.push(current);
    result
}
```

**`&OsStr` vs `&str`**: upstream operates on `[]const u8`, and these helpers
target environment / `PATH`-style values, which are byte sequences on
POSIX/macOS and may contain non-UTF-8 path components. `&OsStr -> OsString`
(with `OsString::push` for the delimiter) preserves those bytes exactly, where
`&str -> String` would bake in a UTF-8 assumption.

## Scope / faithfulness notes

- **Ported (bridged)**: `os.env.appendEnv` → `os::env::append_env`;
  `os.env.appendEnvAlways` → `os::env::append_env_always`; `os.env.prependEnv` →
  `os::env::prepend_env`.
- **Faithful**: empty-`current` passthrough for `append_env` / `prepend_env`;
  the `current:value` / `value:current` composition; `append_env_always` always
  emitting the delimiter (so empty ⇒ `:value`); the `:` macOS delimiter.
- **Faithful adaptation**: the allocator-returning `![]u8` → owned `OsString`
  (always allocated, no caller free); `std.fmt.allocPrint("{s}{c}{s}")` →
  `OsString::push` concatenation; `[]const u8` → `&OsStr` / `OsString`
  (byte-faithful — see note above).
- **Deferred**: `getenv` / `getenvNotEmpty` (the config loader already has a
  private `env_nonempty` equivalent — consolidation into `os::env` deferred to
  avoid a near- duplicate); `setenv` / `unsetenv` (they mutate the global
  process environment, whose threading model in roastty needs deciding);
  `getEnvMap` (its flatpak special-case is Linux-only); the `GetEnvResult`
  Windows-allocation wrapper (macOS-only).
- No C ABI/header/ABI-inventory change (internal Rust). New `os::env` module.

## Changes

1. `roastty/src/os/env.rs` (new): `DELIMITER`, `append_env`,
   `append_env_always`, `prepend_env`.
2. `roastty/src/os/mod.rs`: add `pub(crate) mod env;`.
3. Tests (in `env.rs`): port the upstream macOS-arm tests plus the
   `append_env_always` cases and a non-UTF-8 byte-preservation test (all over
   `OsStr`) —
   - `append_env("", "foo") == "foo"`; `append_env("a:b", "foo") == "a:b:foo"`.
   - `append_env_always("", "foo") == ":foo"`;
     `append_env_always("a:b", "foo") == "a:b:foo"`.
   - `prepend_env("", "foo") == "foo"`;
     `prepend_env("a:b", "foo") == "foo:a:b"`.
   - non-UTF-8:
     `append_env(OsStr::from_bytes(b"a:\xff"), OsStr::from_bytes(b"\xfeb"))`
     yields the bytes `a:\xff:\xfeb` (via `OsStrExt` / `OsStringExt`), locking
     in the byte-preserving signature.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty env
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/os/env.rs roastty/src/os/mod.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `append_env` / `append_env_always` / `prepend_env` compose with the `:`
  delimiter faithfully to `os/env.zig` (empty passthrough where applicable;
  always-delimiter for `append_env_always`);
- the tests pass, and the existing tests still pass;
- the getenv / setenv / getEnvMap parts stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the composition semantics or the delimiter diverges
from upstream, an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex's first design review raised **one Required** finding, now fixed; the
corrected design was **re-reviewed and approved with no findings**.

- **Byte-faithful signatures (Required, fixed)**: the design used
  `&str -> String`, but upstream operates on `[]const u8` and these helpers
  target environment / `PATH`-style values (byte sequences on POSIX, possibly
  non-UTF-8). Fixed by switching to `&OsStr -> OsString` with `OsString::push`
  for the delimiter, preserving bytes exactly; a non-UTF-8 test was added (the
  Optional suggestion) to lock the signature down.

On re-review Codex confirmed the `&OsStr -> OsString` design matches upstream's
byte-oriented helpers while keeping the macOS `:` delimiter behavior exact, the
empty- `current` passthrough is preserved, results are owned/allocated, the
non-UTF-8 test is the right coverage, and the deferrals (getenv/getenvNotEmpty,
setenv/unsetenv, getEnvMap, GetEnvResult) remain sound.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d543-prompt.md` (design),
  `logs/codex-review/20260604-d543b-prompt.md` (design re-review)
- Result: `logs/codex-review/20260604-d543-last-message.md` (design),
  `logs/codex-review/20260604-d543b-last-message.md` (design re-review)

## Result

**Result:** Pass

`os::env` was added with `append_env` / `append_env_always` / `prepend_env` over
`&OsStr -> OsString` (byte-faithful, `:` delimiter via `OsString::push`, owned
results, empty-`current` passthrough for `append_env` / `prepend_env`). The
module is registered in `os/mod.rs`. Six tests: the upstream macOS-arm cases
(`append_env` empty/existing, `prepend_env` empty/existing), the
`append_env_always` always-delimiter cases (empty ⇒ `:foo`), and a non-UTF-8
byte-preservation test (`a:\xff` + `\xfeb` ⇒ `a:\xff:\xfeb`).

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3049 passed, 0 failed (six new tests; no regressions,
  up from 3043).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + os/env.rs + os/mod.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **one Nit** (no
Required or Optional findings): the doc had `## Result` but no `## Conclusion` —
fixed by adding the conclusion below. Codex confirmed the implementation matches
upstream `env.zig` and the approved design: `&OsStr -> OsString` preserves POSIX
byte semantics, `:` is the correct macOS delimiter, `append_env` / `prepend_env`
preserve the empty-`current` passthrough, and `append_env_always` always emits
the delimiter; the non-UTF-8 test directly covers the earlier review finding and
the rest cover the upstream string cases.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r543-prompt.md` (result)
- Result: `logs/codex-review/20260604-r543-last-message.md` (result)

## Conclusion

`os::env` now holds the environment-variable composition helpers (`append_env`,
`append_env_always`, `prepend_env`), faithfully ported from `os/env.zig` over
byte-exact `&OsStr -> OsString`, adding to the `os` module from Experiments
541–542. These build the child-process `PATH`-style variables the eventual
termio layer will need (wiring deferred, along with
`getenv`/`setenv`/`getEnvMap` — see Deferred). The OS-utility frontier still has
clean self-contained slices (`pipe`, `file`, `i18n_locales`, `TempDir`, and the
getenv/setenv remainder of `env.zig`). The config `loadDefaultFiles` stays
deferred pending roastty's naming decision; `background-image-opacity` stays
float-blocked.
