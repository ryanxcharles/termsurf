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

# Experiment 541: the hostname validator (first os/ slice)

## Description

This experiment opens roastty's `os` module by porting upstream
`os/hostname.zig` — the **hostname helpers**: `is_valid` (RFC 1123 hostname
validation) and `is_local` (whether a hostname is `localhost` or matches this
machine's `gethostname`). These back OSC 7 / shell integration: when the
terminal receives a remote working-directory report (`ReportPwd { url }`,
already parsed in `terminal::osc`), the frontend uses these to decide whether
the reported host is the local machine. roastty's `osc.rs` only carries the raw
`url` today, with no hostname validation — so this is genuinely unported.

It also establishes the new top-level `roastty::os` module (none exists yet),
the home for the OS-utility layer that issue 801 covers (PTY/IO/os).

## Upstream behavior

`os/hostname.zig`:

```zig
pub const LocalHostnameValidationError = error{ PermissionDenied, Unexpected };

/// Validates a hostname per RFC 1123. (std.net.isValidHostname is too generous —
/// it accepts ".example.com", "exa..mple.com", "-example.com".)
pub fn isValid(hostname: []const u8) bool {
    if (hostname.len == 0) return false;
    if (hostname[0] == '.') return false;
    // Ignore one trailing dot (FQDN); it doesn't count toward length.
    const end = if (hostname[hostname.len - 1] == '.') (if (hostname.len == 1) return false else hostname.len - 1)
               else hostname.len;
    if (end > 253) return false;
    // Dot-separated labels: start+end alphanumeric, body alphanumeric or '-', len 1..=63.
    var label_start = 0; var label_len = 0;
    for (hostname[0..end], 0..) |c, i| switch (c) {
        '.' => {
            if (label_len == 0 or label_len > 63) return false;
            if (!isAlphanumeric(hostname[label_start])) return false;
            if (!isAlphanumeric(hostname[i - 1])) return false;
            label_start = i + 1; label_len = 0;
        },
        '-' => label_len += 1,
        else => { if (!isAlphanumeric(c)) return false; label_len += 1; },
    };
    if (label_len == 0 or label_len > 63) return false;
    if (!isAlphanumeric(hostname[label_start])) return false;
    if (!isAlphanumeric(hostname[end - 1])) return false;
    return true;
}

/// True if hostname is "localhost" or matches this machine's gethostname().
pub fn isLocal(hostname: []const u8) LocalHostnameValidationError!bool {
    if (std.mem.eql(u8, "localhost", hostname)) return true;
    var buf: [posix.HOST_NAME_MAX]u8 = undefined;
    const ourHostname = try posix.gethostname(&buf);   // (macOS / posix arm)
    return std.mem.eql(u8, hostname, ourHostname);
}
```

- `isValid` rejects empty, leading-dot, and over-253 names; validates each
  dot-separated label (1–63 chars, alphanumeric start/end, body alphanumeric or
  `-`); a single trailing dot (FQDN) is allowed and excluded from the length.
- `isLocal` returns `true` for `localhost`, else compares against `gethostname`.
  Its error set is `{ PermissionDenied, Unexpected }`.

## Rust mapping (`roastty/src/os/hostname.rs`)

`is_valid` is a verbatim logic port over `&[u8]`; `is_local` uses
`libc::gethostname` (roastty already depends on `libc`) and maps a failure's
`errno` to the error enum. The Windows arm is dropped (roastty is macOS-only).

```rust
//! Hostname helpers (port of upstream `os/hostname`).

/// Error from validating whether a hostname is local (upstream
/// `os.hostname.LocalHostnameValidationError`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalHostnameError {
    PermissionDenied,
    Unexpected,
}

/// Validates a hostname per RFC 1123 (upstream `os.hostname.isValid`). Stricter than a
/// permissive validator: rejects leading dots, empty/over-long labels, and non-FQDN
/// trailing junk.
pub(crate) fn is_valid(hostname: &[u8]) -> bool {
    if hostname.is_empty() {
        return false;
    }
    if hostname[0] == b'.' {
        return false;
    }

    // Ignore a single trailing dot (FQDN); it doesn't count toward the length.
    let end = if hostname[hostname.len() - 1] == b'.' {
        if hostname.len() == 1 {
            return false;
        }
        hostname.len() - 1
    } else {
        hostname.len()
    };

    if end > 253 {
        return false;
    }

    let mut label_start = 0usize;
    let mut label_len = 0usize;
    for i in 0..end {
        let c = hostname[i];
        match c {
            b'.' => {
                if label_len == 0 || label_len > 63 {
                    return false;
                }
                if !hostname[label_start].is_ascii_alphanumeric() {
                    return false;
                }
                if !hostname[i - 1].is_ascii_alphanumeric() {
                    return false;
                }
                label_start = i + 1;
                label_len = 0;
            }
            b'-' => label_len += 1,
            _ => {
                if !c.is_ascii_alphanumeric() {
                    return false;
                }
                label_len += 1;
            }
        }
    }

    if label_len == 0 || label_len > 63 {
        return false;
    }
    if !hostname[label_start].is_ascii_alphanumeric() {
        return false;
    }
    if !hostname[end - 1].is_ascii_alphanumeric() {
        return false;
    }

    true
}

/// True if `hostname` is `localhost` or matches this machine's `gethostname`
/// (upstream `os.hostname.isLocal`).
pub(crate) fn is_local(hostname: &[u8]) -> Result<bool, LocalHostnameError> {
    if hostname == b"localhost" {
        return Ok(true);
    }

    // `posix.HOST_NAME_MAX` is 72 on the macOS/Darwin family (vendored Zig std), the same
    // bound upstream's `var buf: [posix.HOST_NAME_MAX]u8` uses.
    const HOST_NAME_MAX: usize = 72;
    let mut buf = [0u8; HOST_NAME_MAX];
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if rc != 0 {
        let errno = std::io::Error::last_os_error().raw_os_error();
        return Err(match errno {
            Some(libc::EPERM) => LocalHostnameError::PermissionDenied,
            _ => LocalHostnameError::Unexpected,
        });
    }

    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    Ok(hostname == &buf[..len])
}
```

`is_valid`'s index uses are safe: `hostname[0]` after the empty check;
`hostname[i - 1]` on `.` only reaches there for `i >= 1` (a leading `.` already
returned false); after the loop `label_len != 0` guarantees `label_start < end`,
so `hostname[label_start]` and `hostname[end - 1]` are in range. `is_local`
reads the NUL-terminated name `gethostname` writes and compares the bytes, the
faithful shape of `std.mem.eql(hostname, ourHostname)`.

## Scope / faithfulness notes

- **Ported (bridged)**: `os.hostname.isValid` → `os::hostname::is_valid`;
  `os.hostname.isLocal` → `os::hostname::is_local`;
  `LocalHostnameValidationError` → `LocalHostnameError`.
- **Faithful**: the RFC 1123 rules (empty / leading-dot / >253 rejected; labels
  1–63, alphanumeric start/end, body alphanumeric or `-`; one trailing FQDN dot
  allowed and excluded from length); `is_local` returning `true` for `localhost`
  else the `gethostname` byte comparison; the two error variants.
- **Faithful adaptation**: `posix.gethostname` → `libc::gethostname` + `errno`
  mapping (`EPERM` ⇒ `PermissionDenied`, else `Unexpected`); the buffer is
  `[u8; 72]` to match the vendored-Zig macOS `posix.HOST_NAME_MAX` (the exact
  upstream bound); the Windows branch dropped (macOS-only); `[]const u8` →
  `&[u8]`.
- **Deferred**: wiring `is_valid` / `is_local` into the OSC 7 /
  shell-integration consumer (`terminal::osc` carries the raw `url` today).
- No C ABI/header/ABI-inventory change (internal Rust). New top-level `os`
  module.

## Changes

1. `roastty/src/os/hostname.rs` (new): `LocalHostnameError`, `is_valid`,
   `is_local`.
2. `roastty/src/os/mod.rs` (new): `pub(crate) mod hostname;` (with
   `#![allow(dead_code)]`).
3. `roastty/src/lib.rs`: add `mod os;` alongside the other top-level modules.
4. Tests (in `hostname.rs`): port upstream's suites —
   - **is_valid**: the full valid list (`example`, `example.com`,
     `www.example.com`, `sub.domain.example.com`, `example.com.`,
     `host-name.example.com.`, `123.example.com.`, `a-b.com`, `a.b.c.d.e.f.g`,
     `127.0.0.1`, a 63-char label, a 253-char name) and invalid list
     (``, `.example.com`, `example.com..`, `host..domain`, `-hostname`, `hostname-`, `a.-.b`, `host_name.com`, `.`, `..`,
     a 64-char label, a 254-char name), built with byte vectors for the length
     cases.
   - **is_local**: `b"localhost"` ⇒ `Ok(true)`; the machine's own `gethostname`
     result ⇒ `Ok(true)`; `b"not-the-local-hostname"` ⇒ `Ok(false)`.
5. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty hostname
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/os/hostname.rs roastty/src/os/mod.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `os::hostname::is_valid` matches upstream's RFC 1123 logic (the full
  valid/invalid suite passes), and `is_local` returns `true` for `localhost` /
  the machine hostname and `false` otherwise, with the two-variant error;
- the tests pass, and the existing tests still pass;
- the OSC 7 / shell-integration wiring stays deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the validation logic, the `is_local` semantics, or
the error set diverges from upstream, an unrelated item changes, or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised **one Required**
finding, now fixed:

- **`is_local` buffer size (Required, fixed)**: the design used `[u8; 256]`, but
  upstream's `var buf: [posix.HOST_NAME_MAX]u8` resolves to `HOST_NAME_MAX = 72`
  on the macOS/Darwin family in the vendored Zig std (`c.zig`), so 256 was an
  unfaithful widening. Fixed by using a `const HOST_NAME_MAX: usize = 72`
  buffer, the exact upstream bound.

Codex confirmed the rest with no other findings: `is_valid` matches upstream
(empty and leading dot rejected, one trailing FQDN dot excluded from the
253-byte length check, labels validated at `1..=63`, alphanumeric start/end with
safe indexing); the `gethostname` error mapping is correct — Zig's wrapper maps
`.PERM` ⇒ `PermissionDenied` and all other errno values ⇒ `Unexpected`
(`posix.zig`); and establishing `os/mod.rs` plus `os/hostname.rs` with the OSC 7
wiring deferred is the right structure.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d541-prompt.md` (design)
- Result: `logs/codex-review/20260604-d541-last-message.md` (design)
