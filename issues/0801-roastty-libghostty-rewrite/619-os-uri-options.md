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

# Experiment 619: os/uri options (MAC-address + raw_path)

## Description

The second slice of the URI hand-port: `os/uri.zig`'s `parse(text, options)` —
the MAC-address fallback/repair and the `raw_path` option — layered on the Exp
618 RFC-3986 parser. This completes the URI area. The Exp 618 parser already
returns verbatim borrowed component slices into `text`, so the upstream
`@intFromPtr` offset arithmetic becomes simple `as_ptr()` differences.

## Upstream behavior (`os/uri.zig`)

```zig
pub fn parse(text, options) ParseError!std.Uri {
    var uri = std.Uri.parse(text) catch |err| uri: {
        // InvalidPort + mac_address → re-parse with a MAC host.
        if (err != error.InvalidPort or !options.mac_address) return err;
        const scheme_end = std.mem.indexOf(u8, text, "://") orelse return error.InvalidFormat;
        const scheme = text[0..scheme_end];
        const host_start = scheme_end + 3;
        const host_end = std.mem.indexOfScalarPos(u8, text, host_start, '/') orelse text.len;
        const mac = text[host_start..host_end];
        if (!isValidMacAddress(mac)) return error.InvalidMacAddress;
        var u = try std.Uri.parseAfterScheme(scheme, text[host_end..]);
        u.host = .{ .percent_encoded = mac };
        break :uri u;
    };

    // Repair: std.Uri parsed a MAC's last octet as a numeric port.
    if (options.mac_address and uri.host != null) mac: {
        const host = uri.host.?.percent_encoded;
        if (host.len != 14 or std.mem.count(u8, host, ":") != 4) break :mac;
        const port = uri.port orelse break :mac;
        if (port > 99) break :mac;
        const host_start = ptrOffset(host, text);
        const path_start = ptrOffset(uri.path, text);
        const mac = text[host_start..path_start];
        if (!isValidMacAddress(mac)) return error.InvalidMacAddress;
        uri.host = .{ .percent_encoded = mac }; uri.port = null;
    }

    if (options.raw_path) {  // everything after the authority, incl. query+fragment
        const path_start = ptrOffset(uri.path, text);
        uri.path = .{ .raw = text[path_start..] }; uri.query = null; uri.fragment = null;
    }
    return uri;
}

fn isValidMacAddress(s) bool {  // len 17; index%3==2 → ':'; else hex digit
    if (s.len != 17) return false;
    for (s, 0..) |c, i| if (i % 3 == 2) { if (c != ':') return false; }
        else switch (c) { '0'...'9','A'...'F','a'...'f' => {}, else => return false };
    return true;
}
```

## Rust mapping (`roastty/src/os/uri.rs`, additions)

```rust
/// `os/uri` parse options.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ParseOptions {
    /// Parse MAC addresses in the host (macOS "Private Wi-Fi address" rotating hostnames).
    pub(crate) mac_address: bool,
    /// Return the full raw path (incl. query + fragment) in `path`; clear `query`/`fragment`.
    pub(crate) raw_path: bool,
}

// `ParseError` gains a variant:
//   InvalidMacAddress,

/// Parse a URI with `os/uri`'s MAC-address + raw_path extensions (upstream `os.uri.parse`).
pub(crate) fn parse_with_options(text: &str, options: ParseOptions) -> Result<Uri<'_>, ParseError> {
    let mut uri = match parse(text) {
        Ok(u) => u,
        Err(ParseError::InvalidPort) if options.mac_address => {
            // Re-parse with a MAC host (std.Uri's port parse choked on the MAC's last octet).
            let scheme_end = text.find("://").ok_or(ParseError::InvalidFormat)?;
            let scheme = &text[..scheme_end];
            let host_start = scheme_end + 3;
            let host_end = text[host_start..]
                .find('/')
                .map_or(text.len(), |i| host_start + i);
            let mac = &text[host_start..host_end];
            if !is_valid_mac_address(mac) {
                return Err(ParseError::InvalidMacAddress);
            }
            let mut u = parse_after_scheme(scheme, &text[host_end..])?;
            u.host = Some(mac);
            u
        }
        Err(e) => return Err(e),
    };

    // Repair: a MAC's last octet was parsed as a numeric port (host 14 chars / 4 colons / port ≤99).
    if options.mac_address {
        if let Some(host) = uri.host {
            let four_colons = host.bytes().filter(|&b| b == b':').count() == 4;
            if host.len() == 14 && four_colons {
                if let Some(port) = uri.port {
                    if port <= 99 {
                        let host_start = offset_in(text, host);
                        let path_start = offset_in(text, uri.path);
                        let mac = &text[host_start..path_start];
                        if !is_valid_mac_address(mac) {
                            return Err(ParseError::InvalidMacAddress);
                        }
                        uri.host = Some(mac);
                        uri.port = None;
                    }
                }
            }
        }
    }

    // raw_path: path = everything after the authority (incl. query + fragment).
    if options.raw_path {
        let path_start = offset_in(text, uri.path);
        uri.path = &text[path_start..];
        uri.query = None;
        uri.fragment = None;
    }

    Ok(uri)
}

/// Byte offset of `slice` (which must be a subslice of `text`) within `text`.
fn offset_in(text: &str, slice: &str) -> usize {
    let base = text.as_ptr() as usize;
    let start = slice.as_ptr() as usize;
    debug_assert!(start >= base && start + slice.len() <= base + text.len());
    start - base
}

/// Whether `s` is a valid MAC address `12:34:56:ab:cd:ef` (upstream `isValidMacAddress`).
pub(crate) fn is_valid_mac_address(s: &str) -> bool {
    if s.len() != 17 {
        return false;
    }
    for (i, b) in s.bytes().enumerate() {
        if i % 3 == 2 {
            if b != b':' {
                return false;
            }
        } else if !b.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}
```

### Notes / deviations

- `@intFromPtr(slice.ptr) - @intFromPtr(text.ptr)` →
  `slice.as_ptr() as usize - text.as_ptr() as usize`. The Exp 618 components are
  verbatim subslices of `text` (including in the MAC-fallback path, where
  `parse_after_scheme` slices `text[host_end..]`), so the offsets are valid.
- `port: Option<u16>` (parsed), so the `port > 99` check is on the integer; the
  repaired `mac` is `text[host_start..path_start]` (the full address up to the
  path), validated.
- `is_valid_mac_address` uses `is_ascii_hexdigit()` (= `0-9A-Fa-f`, matching
  upstream's ranges).
- `ParseError` gains `InvalidMacAddress` (upstream
  `std.Uri.ParseError || error{InvalidMacAddress}`).
- The `Component.raw` vs `.percent_encoded` distinction is still dropped (Exp
  618); `raw_path` just stores the verbatim `text[path_start..]` slice.

## Verification

- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — no regressions; new tests mirror `os/uri.zig`'s
  corpus:
  - **mac_address**: numeric MAC without/with port (`00:12:34:56:78:90[:999]`),
    alphabetic without/with port (`ab:cd:ef:ab:cd:ef[:999]`), no path
    (`00:12:34:56:78:90`), and the invalid cases (`zz:…:00`, `zz:…:zz`) →
    `InvalidMacAddress`. Each asserts the restored host + the port (null or
    999).
  - **raw_path**: `file://localhost/path??#fragment` with `raw_path=false` →
    path `/path`, query `?`, fragment `fragment`; with `raw_path=true` → path
    `/path??#fragment`, query `None`, fragment `None`; `file://localhost` with
    `raw_path=true` → empty path, no query/fragment.
  - `is_valid_mac_address` table (`01:23:45:67:89:Aa` ✓, `Aa:Bb:Cc:Dd:Ee:Ff` ✓;
    `""`, `00:23:45`, `…:Xx:Yy:Zz`, `01-23-45-…`, `…:Aa:Bb` ✗).
  - **combined** `mac_address + raw_path` (`file://00:12:34:56:78:90/p?q#f` →
    host restored, port null, raw path `/p?q#f`) — the option composition holds.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on touched source — clean.
- `git diff --check` — clean.

Pass = `parse_with_options` reproduces `os/uri`'s MAC-address fallback/repair
and `raw_path` behavior across the upstream corpus — completing the URI area.

## Design Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED. Required: none.

Codex confirmed the MAC fallback is faithful (only triggers on `InvalidPort` +
`mac_address`; covers the alphabetic-MAC/no-port case correctly), the numeric
MAC repair is correct (host 14 / 4 colons / port ≤99; correctly does **not**
fire for explicit `:999`), `offset_in`'s pointer arithmetic is sound because
every component is a verbatim subslice of `text` (including the MAC-fallback
path), and `raw_path` / `is_valid_mac_address` / error passthrough all match the
source.

Two Optional findings, both adopted: (1) a `debug_assert!` in `offset_in` that
the slice lies within `text` (catches future misuse early); (2) a combined
`mac_address + raw_path` test. Added to the design above.

## Result

**Result:** Pass

`parse_with_options`, `ParseOptions`, `offset_in`, `is_valid_mac_address`, and
`ParseError::InvalidMacAddress` landed in `roastty/src/os/uri.rs` on top of the
Exp 618 RFC-3986 parser. The MAC fallback (on `InvalidPort` + `mac_address`),
the numeric-MAC repair (host 14 / 4 colons / port ≤99), and the `raw_path`
rewrite all reproduce `os/uri.zig`'s behavior across the upstream corpus.

Gates (all green):

- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — **3420 passed / 0 failed** (was 3407; +13 new tests:
  the MAC numeric/alphabetic × with/without-port matrix, no-path, the
  `InvalidMacAddress` fallback + repair paths, `mac_disabled` passthrough, the
  `raw_path` off/on/empty cases, the combined `mac_address + raw_path`, and the
  `is_valid_mac_address` table).
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on `roastty/src/os/uri.rs` — clean.
- `git diff --check` — clean.

## Completion Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED — no Required, Optional, or Nit findings. Codex confirmed
`parse_with_options` faithfully ports the MAC fallback, the numeric-MAC repair
(correct boundaries: 14-char/4-colon + port ≤99 repairs; 17-char host with
`:999` stays host+port), and `raw_path` rewriting; that `offset_in` is sound for
the verbatim-subslice model (including the fallback's `parse_after_scheme` path)
with the `debug_assert!` the right guard; that `is_valid_mac_address` matches
upstream; and that the error/passthrough behavior and the 13 tests are correct.

## Conclusion

The URI area is **complete** — Exp 618 ported the RFC-3986 foundation
(`std.Uri.parse`) and Exp 619 layered `os/uri`'s MAC-address + `raw_path`
options. This was the pure-Rust hand-port chosen over the `url` crate (which
can't represent MAC-address hosts or hand back verbatim `raw_path` slices).

Next, the remaining regex-area work: the `config/url` default URL regex needs an
oniguruma-class engine (its `(?<!\$\d*)` variable-length look-behind is
supported by neither `regex` nor `fancy-regex`) — a separate dependency/engine
decision — and `input/Link` matching builds on whatever that resolves to. Beyond
regex, the unported `os/` leaves (locale, resourcesdir, kernel_info, args,
cf_release_thread) and the config-directory naming remain, each
dependency/branding/scope-gated.
