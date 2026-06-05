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

# Experiment 544: temp-path generation (os::file)

## Description

Continuing the `os` module (Experiments 541–543), this experiment ports the
**temp-path helpers** from upstream `os/file.zig`: `tmp_dir` (the recommended
temp directory), `random_basename` (a random filesystem-safe base64 basename),
and `random_tmp_path` (`{tmp}/{prefix}{random}`). These generate one-shot
temporary file / socket paths — exactly what roastty needs for its PID-scoped
Unix socket paths. Porting `random_basename`'s url-safe base64 encoding also
adds a small faithful base64 encoder (roastty currently only has a private kitty
base64 _decoder_).

The `fixMaxFiles` / `restoreMaxFiles` rlimit helpers in the same file are
deferred (they mutate the process file-descriptor limit — a startup concern, not
a temp-path one).

## Upstream behavior

`os/file.zig` (the temp-path helpers; the POSIX/macOS arm):

```zig
/// Recommended temp dir; trailing path separator stripped so callers can join with their
/// own. POSIX: $TMPDIR / $TMP / "/tmp", no allocation.
pub fn allocTmpDir(allocator) ![]const u8 {
    const tmpdir = posix.getenv("TMPDIR") orelse posix.getenv("TMP") orelse return "/tmp";
    return std.mem.trimEnd(u8, tmpdir, &.{std.fs.path.sep});
}

const random_basename_bytes = 16;
const b64_encoder = std.base64.url_safe_no_pad.Encoder;
pub const random_basename_len = b64_encoder.calcSize(random_basename_bytes);   // = 22

/// A random filesystem-safe base64 basename of length `random_basename_len`.
pub fn randomBasename(buf: []u8) RandomBasenameError![]const u8 {
    if (buf.len < random_basename_len) return error.BufferTooSmall;
    var rand_buf: [random_basename_bytes]u8 = undefined;
    std.crypto.random.bytes(&rand_buf);
    return b64_encoder.encode(buf[0..random_basename_len], &rand_buf);
}

/// `{TMPDIR}/{prefix}{random}` (allocated). Nothing is created on disk.
pub fn randomTmpPath(allocator, prefix: []const u8) ![]u8 {
    const tmp_dir = try allocTmpDir(allocator);
    defer freeTmpDir(allocator, tmp_dir);
    var name_buf: [random_basename_len]u8 = undefined;
    const basename = randomBasename(&name_buf) catch unreachable;
    return std.fmt.allocPrint(allocator, "{s}{c}{s}{s}", .{ tmp_dir, std.fs.path.sep, prefix, basename });
}
```

- `allocTmpDir`: `$TMPDIR`, else `$TMP`, else `/tmp`; the env value's trailing
  `/` is trimmed (all trailing separators). On POSIX no allocation (returns the
  env slice or the `/tmp` literal); `freeTmpDir` is a POSIX no-op.
- `random_basename_len = calcSize(16) = 22`. `randomBasename` fills 16 CSPRNG
  bytes and url-safe-no-pad base64-encodes them into 22 chars (alphabet
  `A-Za-z0-9-_`).
- `randomTmpPath`: `{tmp}` + `/` + `prefix` + `{random basename}`.

The upstream test: `randomBasename` returns a 22-char name whose chars are all
alphanumeric or `-`/`_`; a too-small buffer yields `error.BufferTooSmall`.

## Rust mapping (`roastty/src/os/file.rs`)

Owned `OsString` results (no caller free; `freeTmpDir` drops away),
`libc::arc4random_buf` as the macOS CSPRNG (faithful to `std.crypto.random`, no
dependency, never fails), and a small url-safe-no-pad base64 encoder (faithful
to `std.base64.url_safe_no_pad.Encoder`):

```rust
//! Temporary-path helpers (port of upstream `os/file`).

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};

/// The url-safe, no-padding base64 alphabet (`std.base64.url_safe_no_pad`).
const BASE64_URL: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Number of random bytes behind a basename, and the encoded length (`calcSize(16) = 22`).
const RANDOM_BASENAME_BYTES: usize = 16;
pub(crate) const RANDOM_BASENAME_LEN: usize = (RANDOM_BASENAME_BYTES * 4 + 2) / 3;

/// Encode bytes as url-safe base64 without padding (upstream
/// `std.base64.url_safe_no_pad.Encoder`).
fn base64_url_no_pad(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() * 4 + 2) / 3);
    let mut chunks = input.chunks_exact(3);
    for chunk in &mut chunks {
        let n = (chunk[0] as u32) << 16 | (chunk[1] as u32) << 8 | chunk[2] as u32;
        out.push(BASE64_URL[(n >> 18 & 63) as usize] as char);
        out.push(BASE64_URL[(n >> 12 & 63) as usize] as char);
        out.push(BASE64_URL[(n >> 6 & 63) as usize] as char);
        out.push(BASE64_URL[(n & 63) as usize] as char);
    }
    let rem = chunks.remainder();
    match rem.len() {
        1 => {
            let n = (rem[0] as u32) << 16;
            out.push(BASE64_URL[(n >> 18 & 63) as usize] as char);
            out.push(BASE64_URL[(n >> 12 & 63) as usize] as char);
        }
        2 => {
            let n = (rem[0] as u32) << 16 | (rem[1] as u32) << 8;
            out.push(BASE64_URL[(n >> 18 & 63) as usize] as char);
            out.push(BASE64_URL[(n >> 12 & 63) as usize] as char);
            out.push(BASE64_URL[(n >> 6 & 63) as usize] as char);
        }
        _ => {}
    }
    out
}

/// A random filesystem-safe base64 basename of length `RANDOM_BASENAME_LEN` (upstream
/// `os.file.randomBasename`). Always allocated (Rust owns the buffer), so the upstream
/// `BufferTooSmall` case does not arise.
pub(crate) fn random_basename() -> String {
    let mut bytes = [0u8; RANDOM_BASENAME_BYTES];
    // arc4random_buf is a CSPRNG on macOS (faithful to std.crypto.random) and never fails.
    unsafe { libc::arc4random_buf(bytes.as_mut_ptr() as *mut libc::c_void, bytes.len()) };
    base64_url_no_pad(&bytes)
}

/// The recommended temp directory with any trailing separator stripped (upstream
/// `os.file.allocTmpDir`): `$TMPDIR`, else `$TMP`, else `/tmp`.
pub(crate) fn tmp_dir() -> OsString {
    resolve_tmp_dir(std::env::var_os("TMPDIR").or_else(|| std::env::var_os("TMP")))
}

/// The temp-dir resolution core, parameterized over the resolved env value for testability.
fn resolve_tmp_dir(value: Option<OsString>) -> OsString {
    match value {
        Some(dir) => trim_end_separators(&dir),
        None => OsString::from("/tmp"),
    }
}

/// Strip all trailing `/` bytes (faithful to `std.mem.trimEnd(.., '/')`).
fn trim_end_separators(dir: &OsStr) -> OsString {
    let bytes = dir.as_bytes();
    let end = bytes.iter().rposition(|&b| b != b'/').map_or(0, |i| i + 1);
    OsStr::from_bytes(&bytes[..end]).to_os_string()
}

/// `{tmp}/{prefix}{random}` (upstream `os.file.randomTmpPath`). Nothing is created on disk.
pub(crate) fn random_tmp_path(prefix: &OsStr) -> OsString {
    let tmp = tmp_dir();
    let basename = random_basename();
    let mut path = OsString::with_capacity(tmp.len() + 1 + prefix.len() + basename.len());
    path.push(&tmp);
    path.push("/");
    path.push(prefix);
    path.push(basename);
    path
}
```

The base64 encoder mirrors `std.base64.url_safe_no_pad.Encoder` (3-byte groups ⇒
4 chars; the 1- and 2-byte tails ⇒ 2 and 3 chars, no padding). `random_basename`
always owns its buffer, so upstream's `BufferTooSmall` guard is unnecessary (and
the `random_basename_len` constant is exposed for callers). `resolve_tmp_dir` is
a behavior-preserving test seam (as in Experiment 542's `expand_in`) so the
trailing-separator trimming can be checked hermetically.

## Scope / faithfulness notes

- **Ported (bridged)**: `os.file.allocTmpDir` → `os::file::tmp_dir`;
  `os.file.randomBasename` → `os::file::random_basename` (+
  `RANDOM_BASENAME_LEN`); `os.file.randomTmpPath` → `os::file::random_tmp_path`;
  plus a faithful `base64_url_no_pad` encoder.
- **Faithful**: the `$TMPDIR` / `$TMP` / `/tmp` resolution with trailing-`/`
  trimming; the 22-char url-safe-no-pad base64 basename from 16 CSPRNG bytes;
  the `{tmp}/{prefix}{random}` composition.
- **Faithful adaptation**: the allocator-returning `![]u8` / caller-buffer forms
  → owned `OsString` / `String` (the POSIX no-alloc and `freeTmpDir` no-op
  collapse away); `std.crypto.random.bytes` → `libc::arc4random_buf` (macOS
  CSPRNG); the `BufferTooSmall` case drops (Rust owns the buffer); `[]const u8`
  → `&OsStr` / `OsString` for the path values.
- **Deferred**: `fixMaxFiles` / `restoreMaxFiles` (process rlimit — a startup
  concern); the Windows `allocTmpDir` / `GetTempPathW` and `freeTmpDir`
  allocation (macOS-only).
- No C ABI/header/ABI-inventory change (internal Rust). New `os::file` module.

## Changes

1. `roastty/src/os/file.rs` (new): `BASE64_URL`, `RANDOM_BASENAME_LEN`,
   `base64_url_no_pad`, `random_basename`, `tmp_dir`, `resolve_tmp_dir`,
   `trim_end_separators`, `random_tmp_path`.
2. `roastty/src/os/mod.rs`: add `pub(crate) mod file;`.
3. Tests (in `file.rs`):
   - **base64 known vectors**: `base64_url_no_pad(b"Man") == "TWFu"`;
     `b"Ma" == "TWE"`; `b"M" == "TQ"`; `&[0u8; 16]` ⇒ 22 `A`s (validates the
     3/2/1-byte tails and length).
   - **random_basename**: length `== RANDOM_BASENAME_LEN` (22); every char is
     alphanumeric or `-`/`_`; two calls differ (randomness).
   - **resolve_tmp_dir**: `Some("/foo/")` ⇒ `/foo`; `Some("/foo//")` ⇒ `/foo`;
     `Some("/tmp")` ⇒ `/tmp`; `None` ⇒ `/tmp`.
   - **random_tmp_path**: starts with `tmp_dir()`; contains the prefix; ends
     with a 22-char basename (total length = `tmp + 1 + prefix + 22`); two calls
     differ.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty os::file
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/os/file.rs roastty/src/os/mod.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `tmp_dir` resolves `$TMPDIR` / `$TMP` / `/tmp` with trailing-`/` trimming;
  `random_basename` returns a 22-char url-safe base64 basename from 16 CSPRNG
  bytes; `random_tmp_path` composes `{tmp}/{prefix}{random}` — all faithful to
  `os/file.zig`;
- the base64 encoder matches the known vectors, and all tests pass with the
  existing tests still passing;
- the rlimit helpers and Windows arms stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the temp-dir resolution, the base64 encoding, or the
path composition diverges from upstream, an unrelated item changes, or any
public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. Codex confirmed `tmp_dir` preserves the `$TMPDIR` / `$TMP` / `/tmp`
order and trims all trailing `/` from env-derived values (with owned `OsString`
the right adaptation of the POSIX no-alloc / free-no-op shape); the base64
encoder is correct for `std.base64.url_safe_no_pad` (URL-safe alphabet, no
padding, 3-byte ⇒ 4-char, 1-byte tail ⇒ 2-char, 2-byte tail ⇒ 3-char,
`calcSize(16) = 22`); `arc4random_buf` is an acceptable macOS CSPRNG substitute
for `std.crypto.random.bytes` with no added dependency; dropping
`BufferTooSmall` is fine since Rust owns the output buffer; and the
`resolve_tmp_dir` seam, the `OsStr`/`OsString` path handling, and the
appropriately-scoped hand-rolled encoder (roastty has only a private decoder
today) are all sound.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d544-prompt.md` (design)
- Result: `logs/codex-review/20260604-d544-last-message.md` (design)
