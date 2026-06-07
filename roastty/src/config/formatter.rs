//! Config entry formatting (port of upstream `config/formatter.zig`).
//!
//! `EntryFormatter` writes one `name = value\n` config line. The comptime,
//! field-dispatch generic `formatEntry` (auto-formatting fields with no custom
//! `formatEntry`) is ported later; this is the object the custom `formatEntry`
//! methods call.
#![allow(dead_code)]

use std::fmt::{Display, Write as _};

pub(crate) trait FloatEntry: Copy + Display {
    fn is_nan(self) -> bool;
}

impl FloatEntry for f32 {
    fn is_nan(self) -> bool {
        f32::is_nan(self)
    }
}

impl FloatEntry for f64 {
    fn is_nan(self) -> bool {
        f64::is_nan(self)
    }
}

/// Writes a single `name = value\n` config entry (upstream
/// `config.formatter.EntryFormatter`).
pub(crate) struct EntryFormatter<'a> {
    name: &'a str,
    out: &'a mut String,
}

impl<'a> EntryFormatter<'a> {
    pub(crate) fn new(name: &'a str, out: &'a mut String) -> Self {
        EntryFormatter { name, out }
    }

    /// `name = value\n` (upstream the `[]const u8` / `[:0]const u8` case).
    pub(crate) fn entry_str(&mut self, value: &str) {
        let _ = writeln!(self.out, "{} = {}", self.name, value);
    }

    /// `name = true|false\n` (upstream the `bool` case).
    pub(crate) fn entry_bool(&mut self, value: bool) {
        let _ = writeln!(self.out, "{} = {}", self.name, value);
    }

    /// `name = <decimal>\n` (upstream the `int` case).
    pub(crate) fn entry_int(&mut self, value: impl Display) {
        let _ = writeln!(self.out, "{} = {}", self.name, value);
    }

    /// `name = <shortest-decimal>\n` (upstream the `float` / `{d}` case).
    /// Rust's `f32` and `f64` `Display` use shortest round-trippable decimals;
    /// `NaN` is written `nan` (Zig's spelling) rather than Rust's `NaN`.
    pub(crate) fn entry_float(&mut self, value: impl FloatEntry) {
        if value.is_nan() {
            let _ = writeln!(self.out, "{} = nan", self.name);
        } else {
            let _ = writeln!(self.out, "{} = {}", self.name, value);
        }
    }

    /// `name = \n` (upstream the `void` case).
    pub(crate) fn entry_void(&mut self) {
        let _ = writeln!(self.out, "{} = ", self.name);
    }

    /// `name = [no-]field,[no-]field…\n` (upstream the packed-struct case): each
    /// flag is its keyword, prefixed with `no-` when `false`.
    pub(crate) fn entry_flags(&mut self, fields: &[(&str, bool)]) {
        let joined = fields
            .iter()
            .map(|&(name, on)| {
                if on {
                    name.to_string()
                } else {
                    format!("no-{}", name)
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        self.entry_str(&joined);
    }

    /// `name = <inner>\n` when present, else `name = \n` (upstream the `optional`
    /// case): when `Some`, recurse into the inner value's formatter with the same
    /// name; when `None`, write the void line.
    pub(crate) fn entry_optional<T>(
        &mut self,
        value: Option<T>,
        fmt_inner: impl FnOnce(T, &mut Self),
    ) {
        match value {
            Some(inner) => fmt_inner(inner, self),
            None => self.entry_void(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_formatter_writes_primitive_lines() {
        let mut out = String::new();
        EntryFormatter::new("a", &mut out).entry_str("v");
        assert_eq!(out, "a = v\n");

        let mut out = String::new();
        EntryFormatter::new("a", &mut out).entry_bool(true);
        assert_eq!(out, "a = true\n");

        let mut out = String::new();
        EntryFormatter::new("a", &mut out).entry_bool(false);
        assert_eq!(out, "a = false\n");

        let mut out = String::new();
        EntryFormatter::new("a", &mut out).entry_int(42u8);
        assert_eq!(out, "a = 42\n");

        let mut out = String::new();
        EntryFormatter::new("a", &mut out).entry_void();
        assert_eq!(out, "a = \n");
    }

    #[test]
    fn entry_float_writes_shortest_decimal() {
        for (value, expected) in [
            (1.0_f32, "a = 1\n"),
            (0.5, "a = 0.5\n"),
            (0.25, "a = 0.25\n"),
            (0.0, "a = 0\n"),
            (0.75, "a = 0.75\n"),
        ] {
            let mut out = String::new();
            EntryFormatter::new("a", &mut out).entry_float(value);
            assert_eq!(out, expected);
        }

        // NaN uses Zig's lowercase spelling (not Rust's `NaN`).
        let mut out = String::new();
        EntryFormatter::new("a", &mut out).entry_float(f32::NAN);
        assert_eq!(out, "a = nan\n");

        let mut out = String::new();
        EntryFormatter::new("a", &mut out).entry_float(0.12345678901234568_f64);
        assert_eq!(out, "a = 0.12345678901234568\n");
    }

    #[test]
    fn entry_flags_writes_comma_joined_keywords() {
        let mut out = String::new();
        EntryFormatter::new("x", &mut out).entry_flags(&[("a", true), ("b", false)]);
        assert_eq!(out, "x = a,no-b\n");
    }

    #[test]
    fn entry_optional_recurses_when_some_and_void_when_none() {
        let mut out = String::new();
        EntryFormatter::new("a", &mut out).entry_optional(Some("v"), |v, f| f.entry_str(v));
        assert_eq!(out, "a = v\n");

        let mut out = String::new();
        EntryFormatter::new("a", &mut out).entry_optional(Some(true), |v, f| f.entry_bool(v));
        assert_eq!(out, "a = true\n");

        let mut out = String::new();
        EntryFormatter::new("a", &mut out).entry_optional(None::<bool>, |v, f| f.entry_bool(v));
        assert_eq!(out, "a = \n");
    }
}
