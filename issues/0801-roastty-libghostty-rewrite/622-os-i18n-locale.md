+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 622: os i18n locale canonicalization foundation

## Description

Port the self-contained locale pieces of `os/i18n.zig` and `os/i18n_locales.zig`
into `roastty/src/os/i18n.rs`: the supported-locale table, locale membership,
macOS Chinese BCP-47 canonicalization fixes, and helper logic for building
gettext-compatible preferred-language entries.

This is intentionally not the full gettext binding. `bindtextdomain`,
`textdomain`, `dgettext`, and `ensureLocale` / Cocoa `NSLocale` environment
probing remain later slices. This experiment provides the pure string behavior
those slices need.

## Upstream behavior

`os/i18n_locales.zig` defines the supported locale list:

```zig
pub const locales = [_][:0]const u8{
    "zh_CN", "de", "fr", "ja", "nl", "nb", "ru", "uk", "pl", "ko_KR",
    "mk", "tr", "id", "es_BO", "es_AR", "es_ES", "pt_BR", "ca", "it",
    "bg", "ga", "hu", "he", "zh_TW", "hr", "lt", "lv", "vi", "kk",
    "be", "eu",
};
```

`os/i18n.zig` provides the locale normalization behavior used by `os/locale.zig`
preferred-language handling:

```zig
pub fn canonicalizeLocale(buf: []u8, locale: []const u8) error{NoSpaceLeft}![:0]const u8 {
    if (fixZhLocale(locale)) |fixed| {
        // copy fixed into buf
        return fixed;
    }

    // Otherwise gettext's _libintl_locale_name_canonicalize mutates in place.
    // The darwin test documents the important observed cases:
    //   en_US -> en_US
    //   zh-Hans -> zh_CN
    //   zh-Hant -> zh_TW
    //   zh-Hans-CN -> zh_CN
    //   zh-Hans-SG -> zh_SG
    //   zh-Hant-TW -> zh_TW
    //   zh-Hant-HK -> zh_HK
    //   zh-Hant-MO -> zh_MO
    //   en_US.UTF-8 -> en_US.UTF_8
}

fn fixZhLocale(locale: []const u8) ?[:0]const u8 {
    var it = std.mem.splitScalar(u8, locale, '-');
    const name = it.next() orelse return null;
    if (!std.mem.eql(u8, name, "zh")) return null;
    const script = it.next() orelse return null;
    const region = it.next() orelse return null;

    if (std.mem.eql(u8, script, "Hans")) {
        if (std.mem.eql(u8, region, "SG")) return "zh_SG";
        return "zh_CN";
    }
    if (std.mem.eql(u8, script, "Hant")) {
        if (std.mem.eql(u8, region, "MO")) return "zh_MO";
        if (std.mem.eql(u8, region, "HK")) return "zh_HK";
        return "zh_TW";
    }
    return null;
}
```

## Rust mapping (`roastty/src/os/i18n.rs`)

```rust
pub(crate) const SUPPORTED_LOCALES: &[&str] = &[ ... ];

pub(crate) fn is_supported_locale(locale: &str) -> bool { ... }

pub(crate) fn canonicalize_locale(locale: &str) -> String {
    if let Some(fixed) = fix_zh_locale(locale) {
        return fixed.to_owned();
    }

    // Minimal gettext-compatible fallback for the pure string cases we need:
    // replace remaining '-' separators with '_' so BCP-47 preferred-language
    // values become gettext/POSIX-shaped before `.UTF-8` is appended.
    locale.replace('-', "_")
}

fn fix_zh_locale(locale: &str) -> Option<&'static str> { ... }

pub(crate) fn gettext_language_entry(locale: &str) -> String {
    let canonical = canonicalize_locale(locale);
    if canonical.contains('.') {
        canonical
    } else {
        format!("{canonical}.UTF-8")
    }
}

pub(crate) fn gettext_language_list<I, S>(locales: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{ ... }
```

### Notes / deviations

- The source module uses Roastty-neutral terminology (`SUPPORTED_LOCALES`,
  `canonicalize_locale`) and does not expose upstream product names.
- The full `_libintl_locale_name_canonicalize` binding is deferred with gettext
  initialization. For this slice, the fallback is deliberately limited to the
  documented test cases and to the pure string behavior needed by
  preferred-language composition. Because upstream's no-region `zh-Hans` and
  `zh-Hant` results come from libintl rather than `fixZhLocale`, the Rust
  `fix_zh_locale` includes explicit no-region mappings: `zh-Hans` → `zh_CN`,
  `zh-Hant` → `zh_TW`.
- `gettext_language_entry` models the behavior in `os/locale.zig`: after
  canonicalization, append `.UTF-8` to entries that do not already include an
  encoding.
- `gettext_language_list` mirrors preferred-language composition by joining
  non-empty entries with `:`, returning `None` if there are no usable locales.

## Changes

- `roastty/src/os/i18n.rs` — add supported-locale table, membership check,
  locale canonicalization, and gettext language-entry/list helpers.
- `roastty/src/os/mod.rs` — expose the new `i18n` module.

## Verification

- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — new tests cover:
  - `SUPPORTED_LOCALES` matches the upstream list exactly, including order;
  - supported locale membership for all upstream list entries;
  - unsupported locales return false;
  - Chinese canonicalization cases from upstream's darwin test: `zh-Hans`,
    `zh-Hant`, `zh-Hans-CN`, `zh-Hans-SG`, `zh-Hant-TW`, `zh-Hant-HK`,
    `zh-Hant-MO`;
  - ordinary canonicalization cases: `en_US`, `en_US.UTF-8`, `en-US`;
  - gettext language entry appends `.UTF-8` only when no encoding is present;
  - gettext language list joins non-empty entries with `:` and returns `None`
    for an empty list.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on touched source — clean.
- `git diff --check` — clean.

Pass = Roastty has the pure locale canonicalization and supported-locale
foundation needed by later locale environment and gettext initialization slices.

## Design Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

Initial review found two Required issues. First, the draft fallback could not
produce upstream's no-region Chinese results (`zh-Hans` → `zh_CN`, `zh-Hant` →
`zh_TW`) because upstream gets those from libintl rather than `fixZhLocale`; the
design now handles those mappings explicitly in the pure Rust fixup path.
Second, the supported-locale test needed to assert exact order, because upstream
documents the table order as semantic for incomplete locale matching.

The design also updated the fallback comment to match broad BCP-47 separator
replacement and added `en-US` as a non-Chinese BCP-47 case. Follow-up review
approved the scope with no Required findings: no gettext binding, no Cocoa
`NSLocale` probing, only the pure string foundation and preferred-language
composition helpers.
