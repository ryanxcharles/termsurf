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

# Experiment 551: tilde home expansion (os::homedir)

## Description

Continuing the `os` module (Experiments 541–550), this experiment opens
`os::homedir` with the **tilde home-directory expansion** from upstream
`os/homedir.zig`: a path that begins with `~/` is rewritten with the user's home
directory (e.g. `~/.config` ⇒ `/Users/me/.config`). This is the expansion config
values like `background-image = ~/pic.png` need. It is ported as a **pure**
`expand_home(path, home_dir)` parameterized over the resolved home directory —
fully testable and faithful to the expansion logic; the `home()` resolution
itself (which on macOS prefers `NSFileManager`) is deferred (see Deferred).

## Upstream behavior

`os/homedir.zig` (the POSIX expansion):

```zig
fn expandHomeUnix(path: []const u8, buf: []u8) ExpandError![]const u8 {
    if (!std.mem.startsWith(u8, path, "~/")) return path;
    const home_dir: []const u8 = if (home(buf)) |home_|
        home_ orelse return error.HomeDetectionFailed
    else |_| return error.HomeDetectionFailed;
    const rest = path[1..]; // Skip the ~ (keep the '/')
    const expanded_len = home_dir.len + rest.len;
    if (expanded_len > buf.len) return Error.BufferTooSmall;
    @memcpy(buf[home_dir.len..expanded_len], rest);
    return buf[0..expanded_len];
}
```

- A path **not** starting with `~/` is returned unchanged (so `~`, `~abc/`,
  `/home/user`, and `""` pass through verbatim).
- Otherwise the `~` is dropped (the `/` is kept: `rest = path[1..]`), and the
  result is `home_dir ++ rest`. Note `home_dir` has no trailing separator and
  `rest` starts with `/`, so `~/` ⇒ `home_dir + "/"` (a trailing separator) and
  `~/x` ⇒ `home_dir + "/x"`.
- `HomeDetectionFailed` (home lookup failed) and `BufferTooSmall` (the fixed
  output buffer) are the only errors.

The upstream test (`expandHomeUnix`): `~/` ⇒ ends with the path separator;
`~/Downloads/…` ⇒ `<home>/Downloads/…`; `~`, `~abc/`, `/home/user`, `""` ⇒
unchanged.

## Rust mapping (`roastty/src/os/homedir.rs`)

A pure, byte-faithful `expand_home(path, home_dir)` returning `Cow<OsStr>` —
borrowing the input when there is no `~/` to expand, owning the rewritten path
otherwise. `home_dir` is a parameter (the resolved home), so the
`HomeDetectionFailed` / `BufferTooSmall` cases don't arise (Rust owns the
output; the home lookup is the caller's concern / deferred):

```rust
//! Home-directory path expansion (port of upstream `os/homedir`).

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;

/// Expand a leading `~/` in `path` to `home_dir` (upstream `os.homedir.expandHomeUnix`,
/// parameterized over the resolved home directory). A `path` that does not begin with `~/`
/// is returned unchanged.
pub(crate) fn expand_home<'a>(path: &'a OsStr, home_dir: &OsStr) -> Cow<'a, OsStr> {
    let bytes = path.as_bytes();
    if !bytes.starts_with(b"~/") {
        return Cow::Borrowed(path);
    }

    // Skip the '~', keeping the '/...'.
    let rest = &bytes[1..];
    let mut expanded = OsString::with_capacity(home_dir.len() + rest.len());
    expanded.push(home_dir);
    expanded.push(OsStr::from_bytes(rest));
    Cow::Owned(expanded)
}
```

`starts_with(b"~/")` on the raw bytes is the faithful form of
`std.mem.startsWith(path, "~/")` (so a lone `~` or `~abc/` is not expanded);
`&bytes[1..]` drops the `~` and keeps the `/` (upstream's `path[1..]`); the
result is built by `OsString::push` (byte-exact, preserving non-UTF-8 path
bytes). The `Cow` borrows in the no-expansion case (the upstream "return
`path`") and owns in the expansion case (the upstream "return
`buf[0..expanded_len]`").

## Scope / faithfulness notes

- **Ported (bridged)**: the `expandHomeUnix` expansion logic →
  `os::homedir::expand_home`, parameterized over the resolved home directory.
- **Faithful**: a path not starting with `~/` returned unchanged; otherwise `~`
  dropped, `/` kept, and `home_dir ++ rest` returned (so `~/` ⇒ trailing
  separator); byte-exact.
- **Faithful adaptation**: `[]const u8` + fixed `buf` → `&OsStr` → `Cow<OsStr>`
  (borrow unchanged / own expanded — no caller buffer, so `BufferTooSmall`
  drops); `std.mem.startsWith` → `<[u8]>::starts_with`; the `home(buf)` call → a
  `home_dir` **parameter** (so `HomeDetectionFailed` drops — the home lookup is
  deferred).
- **Deferred**: `home()` and the public `expandHome` combiner (the macOS
  `home()` chain is `$HOME` → `NSFileManager` → `passwd` → shell-`pwd`; the
  `NSFileManager` step needs an objc binding, so the full resolver is deferred —
  `os::passwd::get` from Experiment 548 already provides one of its fallbacks);
  the Windows arms.
- No C ABI/header/ABI-inventory change (internal Rust). New `os::homedir`
  module.

## Changes

1. `roastty/src/os/homedir.rs` (new): `expand_home`.
2. `roastty/src/os/mod.rs`: add `pub(crate) mod homedir;`.
3. Tests (in `homedir.rs`): port the upstream `expandHomeUnix` cases (with a
   fixed `home_dir = /home/user`) —
   - `expand_home("~/", home)` ⇒ `/home/user/` (ends with the separator).
   - `expand_home("~/Downloads/shader.glsl", home)` ⇒
     `/home/user/Downloads/shader.glsl`.
   - `expand_home("~", home)` ⇒ `~`; `expand_home("~abc/", home)` ⇒ `~abc/`;
     `expand_home("/home/user", home)` ⇒ `/home/user`; `expand_home("", home)` ⇒
     `""` (all unchanged).
   - the unchanged cases return `Cow::Borrowed`; the expanded cases return
     `Cow::Owned`.
   - a non-UTF-8 `home_dir` (`b"/h\xff"`) is preserved byte-for-byte in the
     result.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty os::homedir
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/os/homedir.rs roastty/src/os/mod.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `expand_home` returns a non-`~/` path unchanged (borrowed) and rewrites a
  `~/…` path to `home_dir ++ rest` (owned), keeping the separator — faithful to
  `os/homedir.zig`'s `expandHomeUnix`;
- the tests pass (the upstream cases + borrow/own + non-UTF-8), and the existing
  tests still pass;
- `home()` and the `expandHome` combiner stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the expansion logic diverges from upstream, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. Codex confirmed the expansion logic is faithful — only the raw `~/`
prefix expands, `~` and `~abc/` stay unchanged, `path[1..]` correctly drops `~`
while preserving the separator, and `OsString` / `OsStrExt` keep it byte-exact;
parameterizing `home_dir` is the right bounded slice (a passwd-only resolver
would not match upstream's macOS `home()` order, so deferring the resolver is
cleaner); and the `Cow<OsStr>` shape is appropriate (borrowed for upstream's
unchanged return, owned for the expanded case).

Review artifacts:

- Prompt: `logs/codex-review/20260604-d551-prompt.md` (design)
- Result: `logs/codex-review/20260604-d551-last-message.md` (design)
