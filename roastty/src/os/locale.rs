//! Locale probing helpers (Cocoa slice of upstream `os/locale`).

use crate::os::i18n;

#[cfg(target_os = "macos")]
use objc2_foundation::NSLocale;

/// Build a `LANG` environment value from macOS system locale preferences.
pub(crate) fn macos_lang_from_cocoa() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let locale = NSLocale::currentLocale();
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

/// Build a gettext `LANGUAGE` environment value from macOS preferred languages.
pub(crate) fn macos_language_from_cocoa() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let preferred = NSLocale::preferredLanguages();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_env_value_formats_language_and_country() {
        assert_eq!(lang_env_value("en", "US"), Some("en_US.UTF-8".to_owned()));
    }

    #[test]
    fn lang_env_value_rejects_empty_parts() {
        assert_eq!(lang_env_value("", "US"), None);
        assert_eq!(lang_env_value("en", ""), None);
        assert_eq!(lang_env_value("", ""), None);
    }

    #[test]
    fn language_env_value_canonicalizes_and_joins() {
        assert_eq!(
            language_env_value(["en-US", "", "zh-Hant-HK"]),
            Some("en_US.UTF-8:zh_HK.UTF-8".to_owned())
        );
    }

    #[test]
    fn language_env_value_rejects_empty_lists() {
        assert_eq!(language_env_value([] as [&str; 0]), None);
        assert_eq!(language_env_value(["", ""]), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn cocoa_probes_smoke_when_values_are_available() {
        if let Some(lang) = macos_lang_from_cocoa() {
            assert!(!lang.is_empty());
            assert!(lang.contains('_'), "{lang}");
            assert!(lang.ends_with(".UTF-8"), "{lang}");
        }

        if let Some(language) = macos_language_from_cocoa() {
            assert!(!language.is_empty());
            assert!(language.contains(".UTF-8"), "{language}");
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn cocoa_probes_return_none_on_non_macos() {
        assert_eq!(macos_lang_from_cocoa(), None);
        assert_eq!(macos_language_from_cocoa(), None);
    }
}
