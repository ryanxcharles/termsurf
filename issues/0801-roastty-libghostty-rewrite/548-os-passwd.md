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

# Experiment 548: the passwd entry (os::passwd)

## Description

Continuing the `os` module (Experiments 541–547), this experiment ports upstream
`os/passwd.zig` — the **passwd database lookup** for the current user: `get()`
returns an `Entry` with the user's login `shell`, `home` directory, and `name`
(from `getpwuid_r`). This is how the terminal determines the default shell and
home directory on Unix (used by `homedir` and shell-integration setup). The
Linux/flatpak host-command branch drops away (macOS-only).

## Upstream behavior

`os/passwd.zig`:

```zig
pub const Entry = struct {
    shell: ?[:0]const u8 = null,
    home: ?[:0]const u8 = null,
    name: ?[:0]const u8 = null,
};

/// Get the passwd entry for the currently executing user.
pub fn get(alloc: Allocator) !Entry {
    var buf: [1024]u8 = undefined;
    var pw: c.struct_passwd = undefined;
    var pw_ptr: ?*c.struct_passwd = null;
    const res = c.getpwuid_r(c.getuid(), &pw, &buf, buf.len, &pw_ptr);
    if (res != 0) {
        log.warn("error retrieving pw entry code={d}", .{res});
        return Entry{};
    }
    if (pw_ptr == null) {
        log.warn("no pw entry to detect default shell, will default to 'sh'", .{});
        return Entry{};
    }

    var result: Entry = .{};
    // (Linux/flatpak: shell out to the host for the real entry — macOS skips this.)
    if (pw.pw_shell) |ptr| result.shell = try alloc.dupeZ(u8, std.mem.sliceTo(ptr, 0));
    if (pw.pw_dir)   |ptr| result.home  = try alloc.dupeZ(u8, std.mem.sliceTo(ptr, 0));
    if (pw.pw_name)  |ptr| result.name  = try alloc.dupeZ(u8, std.mem.sliceTo(ptr, 0));
    return result;
}
```

- `getpwuid_r(getuid(), …)` fills a `passwd` struct from a stack buffer. A
  non-zero return (e.g. `ERANGE`) or a null result pointer yields an **empty**
  `Entry` (upstream logs and returns `Entry{}`).
- Otherwise the `pw_shell` / `pw_dir` / `pw_name` C strings are copied into the
  `Entry` (NUL-terminated).
- The flatpak branch (run `getent passwd` on the host via a PTY) is Linux-only.

The upstream test: `get()` returns a non-null `shell` and `home` for the current
user.

## Rust mapping (`roastty/src/os/passwd.rs`)

`libc::getpwuid_r` filling a `libc::passwd`, with the C-string fields copied
into an owned `Entry` of `OsString`s (byte-faithful — shell/home are paths).
`get` returns the `Entry` directly (no `Result`: Rust has no allocation error,
and a lookup failure is the empty `Entry`, as upstream):

```rust
//! The passwd database entry for the current user (port of upstream `os/passwd`).

use std::ffi::{CStr, OsString};
use std::os::unix::ffi::OsStringExt;

/// The passwd fields we care about for the current user (upstream `os.passwd.Entry`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) shell: Option<OsString>,
    pub(crate) home: Option<OsString>,
    pub(crate) name: Option<OsString>,
}

/// Get the passwd entry for the currently executing user (upstream `os.passwd.get`). On any
/// lookup failure (non-zero `getpwuid_r` or a null result) an empty `Entry` is returned.
pub(crate) fn get() -> Entry {
    let mut buf = [0 as libc::c_char; 1024];
    let mut pw: libc::passwd = unsafe { std::mem::zeroed() };
    let mut pw_ptr: *mut libc::passwd = std::ptr::null_mut();

    let res = unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            &mut pw,
            buf.as_mut_ptr(),
            buf.len(),
            &mut pw_ptr,
        )
    };
    // A non-zero return or a null entry means "no entry"; upstream logs and returns empty.
    if res != 0 || pw_ptr.is_null() {
        return Entry::default();
    }

    Entry {
        shell: cstr_to_os(pw.pw_shell),
        home: cstr_to_os(pw.pw_dir),
        name: cstr_to_os(pw.pw_name),
    }
}

/// Copy a (possibly null) NUL-terminated C string field into an owned `OsString`.
fn cstr_to_os(ptr: *const libc::c_char) -> Option<OsString> {
    if ptr.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes().to_vec();
    Some(OsString::from_vec(bytes))
}
```

`getpwuid_r` is the thread-safe reentrant form (matching upstream); `pw` is
zero-initialized and `pw_ptr` distinguishes "found" from "not found".
`cstr_to_os` copies each field's bytes (the equivalent of `dupeZ` + `sliceTo`),
preserving non-UTF-8 path bytes. The NUL-terminated `[:0]` fields become
`OsString` (no trailing NUL — the idiomatic owned path/shell value; callers add
a NUL when exec'ing, as `std::process::Command` does).

## Scope / faithfulness notes

- **Ported (bridged)**: `os.passwd.Entry` → `os::passwd::Entry`; `os.passwd.get`
  → `os::passwd::get`.
- **Faithful**: `getpwuid_r(getuid())` with a 1024-byte buffer; a non-zero
  return or null result ⇒ empty `Entry`; otherwise `shell` / `home` / `name`
  copied from `pw_shell` / `pw_dir` / `pw_name`.
- **Faithful adaptation**: the `@cImport` `getpwuid_r` / `getuid` → `libc`;
  `dupeZ` + `sliceTo` → `CStr` + `OsString::from_vec` (byte-faithful);
  `[:0]const u8` → `OsString` (owned, no NUL); `!Entry` → `Entry` (no Rust alloc
  error; the failure case is the empty `Entry`); the warn-logs → comments (no
  logger here).
- **Deferred / dropped**: the Linux/flatpak branch (shell out to the host via a
  PTY for the real entry) — Linux-only, not applicable to macOS.
- No C ABI/header/ABI-inventory change (internal Rust). New `os::passwd` module.

## Changes

1. `roastty/src/os/passwd.rs` (new): `Entry`, `get`, `cstr_to_os`.
2. `roastty/src/os/mod.rs`: add `pub(crate) mod passwd;`.
3. Tests (in `passwd.rs`): port the upstream test —
   - **current user has shell + home**: `get()` returns `shell` and `home` as
     `Some(_)` and both are non-empty (the current user always has a passwd
     entry); `name` is also `Some(_)` and non-empty.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty os::passwd
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/os/passwd.rs roastty/src/os/mod.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `os::passwd::get` returns the current user's `shell` / `home` / `name` via
  `getpwuid_r`, with an empty `Entry` on lookup failure — faithful to
  `os/passwd.zig`'s macOS path;
- the test passes (current user has a shell and home), and the existing tests
  still pass;
- the flatpak branch stays dropped;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the lookup semantics or the field extraction
diverges from upstream, an unrelated item changes, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. Codex confirmed the design is faithful to upstream's macOS path:
`getpwuid_r(getuid(), …)` with a 1024-byte stack buffer,
`res != 0 || pw_ptr.is_null()` ⇒ `Entry::default()`, and copying `pw_shell` /
`pw_dir` / `pw_name` only after `pw_ptr` is non-null all match; `OsString` is
the right representation for these byte-oriented NUL-terminated fields once
copied (owned values without the trailing NUL, usable as `OsStr` for
`Command`/exec); `zeroed()` for `libc::passwd` is fine since `getpwuid_r`
overwrites the struct before any field is read; and dropping the Linux/flatpak
branch is properly scoped for macOS-only.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d548-prompt.md` (design)
- Result: `logs/codex-review/20260604-d548-last-message.md` (design)

## Result

**Result:** Pass

`os::passwd` was added with `Entry { shell, home, name }` (all
`Option<OsString>`) and `get`: `libc::getpwuid_r(getuid(), …)` into a zeroed
`libc::passwd` with a 1024-byte buffer, an empty `Entry` on non-zero return or
null result, otherwise the `pw_shell` / `pw_dir` / `pw_name` C strings copied
via `cstr_to_os` (`CStr` + `OsString::from_vec`, byte-faithful). The
Linux/flatpak branch is dropped. The module is registered in `os/mod.rs`. One
test confirms the current user's `shell`, `home`, and `name` are all `Some(_)`
and non-empty.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3059 passed, 0 failed (one new test; no regressions,
  up from 3058).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + os/passwd.rs + os/mod.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **one Nit** (no
Required or Optional findings): the doc had `## Result` but no `## Conclusion` —
fixed by adding the conclusion below. Codex confirmed the implementation matches
upstream `passwd.zig` and the approved design: `getpwuid_r(getuid(), …)` uses
the 1024-byte buffer, `res != 0 || pw_ptr.is_null()` returns an empty `Entry`,
and fields are copied only after a valid result; `CStr::from_ptr(…).to_bytes()`
plus `OsString::from_vec` is byte-faithful and drops the trailing NUL as
intended; the test is sound for the macOS slice; and dropping the Linux/flatpak
branch is correctly scoped.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r548-prompt.md` (result)
- Result: `logs/codex-review/20260604-r548-last-message.md` (result)

## Conclusion

`os::passwd::get` — the `getpwuid_r` lookup of the current user's `shell` /
`home` / `name` — is faithfully ported from `os/passwd.zig`, adding to the `os`
module from Experiments 541–547. This is how roastty will determine the default
shell and home directory for the child process (wiring into homedir /
shell-launch deferred). The Linux/flatpak host-command branch was dropped
(macOS-only). The OS-utility frontier still has a few self-contained slices
(`i18n_locales`, `open`, `locale`, `homedir`'s tilde-expansion). The config
`loadDefaultFiles` stays deferred pending roastty's naming decision;
`background-image-opacity` stays float-blocked.
