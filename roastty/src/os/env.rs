//! Environment-variable helpers (port of upstream `os/env`).

use std::ffi::{OsStr, OsString};

/// The platform `PATH`-style delimiter (`std.fs.path.delimiter`; `:` on macOS).
const DELIMITER: &str = ":";

/// Append `value` to an environment variable such as `PATH` (upstream `os.env.appendEnv`).
/// An empty `current` returns `value` as-is; otherwise `current:value`. Always allocated.
pub(crate) fn append_env(current: &OsStr, value: &OsStr) -> OsString {
    // If there is no prior value, we return it as-is.
    if current.is_empty() {
        return value.to_os_string();
    }
    append_env_always(current, value)
}

/// Always append `value`, even when `current` is empty (upstream `os.env.appendEnvAlways`).
/// Useful for vars like `MANPATH` that want an empty prefix to preserve existing values, so
/// an empty `current` yields `:value`. Always allocated.
pub(crate) fn append_env_always(current: &OsStr, value: &OsStr) -> OsString {
    let mut result = OsString::with_capacity(current.len() + DELIMITER.len() + value.len());
    result.push(current);
    result.push(DELIMITER);
    result.push(value);
    result
}

/// Prepend `value` to an environment variable such as `PATH` (upstream `os.env.prependEnv`).
/// An empty `current` returns `value` as-is; otherwise `value:current`. Always allocated.
pub(crate) fn prepend_env(current: &OsStr, value: &OsStr) -> OsString {
    // If there is no prior value, we return it as-is.
    if current.is_empty() {
        return value.to_os_string();
    }
    let mut result = OsString::with_capacity(value.len() + DELIMITER.len() + current.len());
    result.push(value);
    result.push(DELIMITER);
    result.push(current);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    fn os(s: &str) -> &OsStr {
        OsStr::new(s)
    }

    #[test]
    fn append_env_empty() {
        assert_eq!(append_env(os(""), os("foo")), OsString::from("foo"));
    }

    #[test]
    fn append_env_existing() {
        assert_eq!(append_env(os("a:b"), os("foo")), OsString::from("a:b:foo"));
    }

    #[test]
    fn append_env_always_emits_delimiter_even_when_empty() {
        assert_eq!(append_env_always(os(""), os("foo")), OsString::from(":foo"));
        assert_eq!(
            append_env_always(os("a:b"), os("foo")),
            OsString::from("a:b:foo"),
        );
    }

    #[test]
    fn prepend_env_empty() {
        assert_eq!(prepend_env(os(""), os("foo")), OsString::from("foo"));
    }

    #[test]
    fn prepend_env_existing() {
        assert_eq!(prepend_env(os("a:b"), os("foo")), OsString::from("foo:a:b"));
    }

    #[test]
    fn append_env_preserves_non_utf8_bytes() {
        let current = OsStr::from_bytes(b"a:\xff");
        let value = OsStr::from_bytes(b"\xfeb");
        let result = append_env(current, value);
        assert_eq!(result.into_vec(), b"a:\xff:\xfeb".to_vec());
    }
}
