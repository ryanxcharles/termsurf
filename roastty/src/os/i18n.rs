//! Pure locale helpers used by later i18n initialization slices.

pub(crate) const SUPPORTED_LOCALES: &[&str] = &[
    "zh_CN", "de", "fr", "ja", "nl", "nb", "ru", "uk", "pl", "ko_KR", "mk", "tr", "id", "es_BO",
    "es_AR", "es_ES", "pt_BR", "ca", "it", "bg", "ga", "hu", "he", "zh_TW", "hr", "lt", "lv", "vi",
    "kk", "be", "eu",
];

pub(crate) fn is_supported_locale(locale: &str) -> bool {
    SUPPORTED_LOCALES.contains(&locale)
}

pub(crate) fn canonicalize_locale(locale: &str) -> String {
    if let Some(fixed) = fix_zh_locale(locale) {
        return fixed.to_owned();
    }

    locale.replace('-', "_")
}

fn fix_zh_locale(locale: &str) -> Option<&'static str> {
    let mut parts = locale.split('-');
    let name = parts.next()?;
    if name != "zh" {
        return None;
    }

    let script = parts.next()?;
    let region = parts.next();

    match (script, region) {
        ("Hans", Some("SG")) => Some("zh_SG"),
        ("Hans", _) => Some("zh_CN"),
        ("Hant", Some("MO")) => Some("zh_MO"),
        ("Hant", Some("HK")) => Some("zh_HK"),
        ("Hant", _) => Some("zh_TW"),
        _ => None,
    }
}

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
{
    let entries = locales
        .into_iter()
        .filter_map(|locale| {
            let locale = locale.as_ref();
            if locale.is_empty() {
                None
            } else {
                Some(gettext_language_entry(locale))
            }
        })
        .collect::<Vec<_>>();

    if entries.is_empty() {
        None
    } else {
        Some(entries.join(":"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UPSTREAM_LOCALES: &[&str] = &[
        "zh_CN", "de", "fr", "ja", "nl", "nb", "ru", "uk", "pl", "ko_KR", "mk", "tr", "id",
        "es_BO", "es_AR", "es_ES", "pt_BR", "ca", "it", "bg", "ga", "hu", "he", "zh_TW", "hr",
        "lt", "lv", "vi", "kk", "be", "eu",
    ];

    #[test]
    fn supported_locale_table_matches_upstream_order() {
        assert_eq!(SUPPORTED_LOCALES, UPSTREAM_LOCALES);
    }

    #[test]
    fn supported_locale_membership_matches_table() {
        for locale in UPSTREAM_LOCALES {
            assert!(is_supported_locale(locale), "{locale} should be supported");
        }

        assert!(!is_supported_locale(""));
        assert!(!is_supported_locale("en_US"));
        assert!(!is_supported_locale("zh_SG"));
        assert!(!is_supported_locale("fr_FR"));
    }

    #[test]
    fn canonicalize_locale_matches_chinese_darwin_cases() {
        let cases = [
            ("zh-Hans", "zh_CN"),
            ("zh-Hant", "zh_TW"),
            ("zh-Hans-CN", "zh_CN"),
            ("zh-Hans-SG", "zh_SG"),
            ("zh-Hant-TW", "zh_TW"),
            ("zh-Hant-HK", "zh_HK"),
            ("zh-Hant-MO", "zh_MO"),
        ];

        for (input, expected) in cases {
            assert_eq!(canonicalize_locale(input), expected, "{input}");
        }
    }

    #[test]
    fn canonicalize_locale_handles_ordinary_cases() {
        assert_eq!(canonicalize_locale("en_US"), "en_US");
        assert_eq!(canonicalize_locale("en_US.UTF-8"), "en_US.UTF_8");
        assert_eq!(canonicalize_locale("en-US"), "en_US");
        assert_eq!(canonicalize_locale("fr-FR"), "fr_FR");
    }

    #[test]
    fn gettext_language_entry_appends_utf8_when_needed() {
        assert_eq!(gettext_language_entry("fr-FR"), "fr_FR.UTF-8");
        assert_eq!(gettext_language_entry("zh-Hant-HK"), "zh_HK.UTF-8");
        assert_eq!(gettext_language_entry("en_US.UTF-8"), "en_US.UTF_8");
    }

    #[test]
    fn gettext_language_list_joins_non_empty_entries() {
        assert_eq!(
            gettext_language_list(["en-US", "", "zh-Hant-HK"]),
            Some("en_US.UTF-8:zh_HK.UTF-8".to_owned())
        );
        assert_eq!(gettext_language_list(["", ""]), None);
        assert_eq!(gettext_language_list([] as [&str; 0]), None);
    }
}
