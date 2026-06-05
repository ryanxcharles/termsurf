+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 625: os locale Cocoa probe

## Description

Port the Cocoa-probing portion of upstream `os/locale.zig` into
`roastty/src/os/locale.rs`, without yet implementing full `ensureLocale`.

Upstream `ensureLocale` does three separable things: pre-populates `LANG` from
macOS `NSLocale` when needed, calls C `setlocale`, and falls back through
environment/system/default locale choices. This experiment takes only the first
Cocoa slice so later `ensure_locale` work can reuse a tested source of macOS
locale values.

The new Rust module should expose two library-internal helpers:

- `macos_lang_from_cocoa()` — read `NSLocale::currentLocale()`, `languageCode`,
  and `countryCode`, returning `<language>_<country>.UTF-8` when both pieces are
  present.
- `macos_language_from_cocoa()` — read `NSLocale::preferredLanguages()`,
  canonicalize each BCP-47 language through the existing `os::i18n` helpers, and
  return the gettext `LANGUAGE` value joined with colons.

This experiment does not mutate `LANG`/`LANGUAGE`, call `setlocale`, bind
gettext, or implement fallback locale selection.

## Upstream behavior (`os/locale.zig`)

```zig
fn setLangFromCocoa() void {
    const NSLocale = objc.getClass("NSLocale") orelse return;

    const locale = NSLocale.msgSend(objc.Object, objc.sel("currentLocale"), .{});
    const lang = locale.getProperty(objc.Object, "languageCode");
    const country = locale.getProperty(objc.Object, "countryCode");

    if (lang.value == null or country.value == null) return;

    const c_lang = lang.getProperty([*:0]const u8, "UTF8String");
    const c_country = country.getProperty([*:0]const u8, "UTF8String");

    // Format our locale as "<lang>_<country>.UTF-8" and set it as LANG.
    const env_value = std.fmt.bufPrintZ(&buf, "{s}_{s}.UTF-8", .{ z_lang, z_country });
    _ = internal_os.setenv("LANG", env_value);

    if (preferredLanguageFromCocoa(&buf, NSLocale)) |pref| {
        _ = internal_os.setenv("LANGUAGE", pref);
    }
}

fn preferredLanguageFromCocoa(buf: []u8, NSLocale: objc.Class) error{NoSpaceLeft}!?[:0]const u8 {
    const preferred = NSLocale.msgSend(objc.Object, objc.sel("preferredLanguages"), .{});
    for (0..preferred.getCount()) |i| {
        const c_str = preferred.getValueAtIndex(macos.foundation.String, i).cstring(...);
        const canon = try i18n.canonicalizeLocale(fbs.buffer[fbs.pos..], c_str);
        _ = writer.writeAll(".UTF-8") catch return error.NoSpaceLeft;
    }
    if (fbs.pos == 0) return null;
    return slice[0 .. slice.len - 1 :0];
}
```

## Rust mapping (`roastty/src/os/locale.rs`)

```rust
pub(crate) fn macos_lang_from_cocoa() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let locale = objc2_foundation::NSLocale::currentLocale();
        let language = locale.languageCode().to_string();
        #[allow(deprecated)]
        let country = locale.countryCode()?.to_string();
        lang_env_value(&language, &country)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub(crate) fn macos_language_from_cocoa() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let preferred = objc2_foundation::NSLocale::preferredLanguages();
        let values = (0..preferred.count()).map(|i| preferred.objectAtIndex(i).to_string());
        language_env_value(values)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn lang_env_value(language: &str, country: &str) -> Option<String> {
    if language.is_empty() || country.is_empty() {
        None
    } else {
        Some(format!("{language}_{country}.UTF-8"))
    }
}

fn language_env_value<I, S>(values: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    i18n::gettext_language_list(values)
}
```

### Notes / deviations

- This is a probe-only slice. Environment mutation and `setlocale(LC_ALL, "")`
  stay deferred so the global process-locale behavior can be reviewed and tested
  as its own experiment.
- The Rust port uses typed `objc2-foundation` bindings instead of raw
  Objective-C class lookup and selectors. `roastty/Cargo.toml` should add the
  minimal `NSLocale` feature to the existing `objc2-foundation` dependency.
- Upstream calls the now-deprecated `countryCode` selector. This slice keeps
  that selector, with a narrow `#[allow(deprecated)]`, to preserve the upstream
  behavior exactly before any intentional migration to `regionCode`.
- Non-macOS hosts return `None`. Issue 801 is not adding Linux/BSD locale
  behavior.
- Preferred-language formatting delegates to `os::i18n::gettext_language_list`,
  which already canonicalizes BCP-47 input and appends `.UTF-8`. This call is
  wrapped in `language_env_value` so the `LANGUAGE` formatting path is tested
  without depending on Cocoa.

## Changes

- `roastty/Cargo.toml` — add `NSLocale` to the existing
  `objc2-foundation.features` list.
- `roastty/src/os/locale.rs` — add the Cocoa `LANG`/`LANGUAGE` probe helpers and
  deterministic pure-format tests for both env values.
- `roastty/src/os/mod.rs` — expose the new `locale` module.

## Verification

- `cargo test -p roastty os::locale::tests` — new tests cover:
  - `lang_env_value("en", "US")` returns `en_US.UTF-8`;
  - empty language or country returns `None`;
  - `language_env_value` reuses existing canonicalization (`en-US`, `zh-Hant-HK`
    → `en_US.UTF-8:zh_HK.UTF-8`) and returns `None` for an empty list;
  - macOS smoke probes return non-empty values when Cocoa reports the expected
    language/country/preferred-language data;
  - non-macOS public probes return `None`.
- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — full Roastty test suite stays green.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on touched source — clean.
- `git diff --check` — clean.

Pass = Roastty has a typed Cocoa source for future `LANG` and `LANGUAGE`
initialization without yet changing process-global locale state.

## Design Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

Initial review found one Required issue: the design promised deterministic
preferred-language formatting tests, but only routed formatting through the
macOS Cocoa probe. The design now adds `language_env_value<I, S>(values)`,
routes `macos_language_from_cocoa()` through that helper, and verifies
`LANGUAGE` formatting without depending on Cocoa.

Follow-up review approved the probe-only scope, the `NSLocale` feature addition,
the typed `objc2-foundation` API, deferred env mutation/`setlocale`, the narrow
deprecated `countryCode` use to match upstream, and non-macOS `None` stubs.

## Result

**Result:** Pass

`roastty/src/os/locale.rs` now provides the Cocoa probe slice for future locale
initialization. `macos_lang_from_cocoa()` reads `NSLocale::currentLocale()`,
`languageCode`, and upstream-compatible `countryCode`, then formats
`<language>_<country>.UTF-8` through the pure `lang_env_value` helper.
`macos_language_from_cocoa()` reads `NSLocale::preferredLanguages()` and routes
the list through `language_env_value`, which delegates to the existing
`os::i18n` canonicalization and gettext `.UTF-8` formatting.

The helpers are cfg-gated: macOS uses typed `objc2-foundation` `NSLocale`, while
non-macOS hosts return `None`. The experiment adds only the `NSLocale` feature
to the existing `objc2-foundation` dependency, exposes `os::locale`, and does
not mutate process environment or call `setlocale`.

Gates (all green):

- `cargo test -p roastty os::locale::tests` — **5 passed / 0 failed** focused
  tests on this macOS host, including the Cocoa smoke probe.
- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — **3454 passed / 0 failed** unit tests, plus **1
  passed / 0 failed** ABI harness test and **0** doc tests.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on `roastty/src/os/locale.rs`, `roastty/src/os/mod.rs`, and
  `roastty/Cargo.toml` — clean.
- `git diff --check` — clean.

## Conclusion

Roastty now has a typed, test-covered Cocoa source for future `LANG` and
`LANGUAGE` initialization. The process-global pieces of upstream `ensureLocale`
remain deferred for a later experiment: env mutation, `setlocale(LC_ALL, "")`,
invalid-`LANG` recovery, and final `en_US.UTF-8` fallback.

## Completion Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

Initial completion review found no code or documentation issues, but noted the
new `roastty/src/os/locale.rs` file was still untracked and therefore had to be
staged before the result commit. After staging the full result and confirming
`git diff --cached --check` was clean, follow-up review approved the result with
no remaining Required fixes.

Codex confirmed the implementation matches the approved design:
`macos_lang_from_cocoa()` uses typed `NSLocale::currentLocale()`,
`languageCode`, deprecated `countryCode`, and `lang_env_value`;
`macos_language_from_cocoa()` uses `NSLocale::preferredLanguages()` and routes
through `language_env_value`; non-macOS probes return `None`; only the
`NSLocale` feature was added to the existing Foundation dependency; and no
environment mutation or `setlocale` behavior was added.
