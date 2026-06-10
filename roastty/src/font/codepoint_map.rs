//! A map of codepoint ranges to font-search descriptors.
//!
//! Faithful port of upstream `font/CodepointMap.zig`: a list of
//! range → [`Descriptor`] entries with a reverse-priority linear lookup (later
//! entries win). Used to override the font for specific codepoint ranges.

use crate::font::discovery::Descriptor;

/// A single range → descriptor mapping.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MapEntry {
    /// Inclusive Unicode codepoint range; `range[0] <= range[1]`. Upstream uses
    /// `u21`; values stay within that domain.
    pub range: [u32; 2],
    /// The font to use for this range.
    pub descriptor: Descriptor,
}

/// A map of codepoint ranges to font-search [`Descriptor`]s. A linear scan is
/// used since maps are expected to have very few entries.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct CodepointMap {
    entries: Vec<MapEntry>,
}

impl CodepointMap {
    /// Add a range → descriptor entry. Later entries take priority over earlier
    /// ones for overlapping codepoints. Panics if `range[0] > range[1]`.
    pub(crate) fn add(&mut self, range: [u32; 2], descriptor: Descriptor) {
        assert!(range[0] <= range[1], "inverted codepoint range");
        self.entries.push(MapEntry { range, descriptor });
    }

    /// Number of stored range mappings.
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are no stored range mappings.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate range mappings in insertion order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &MapEntry> {
        self.entries.iter()
    }

    /// The descriptor for `cp`, or `None`. Scans entries in **reverse** so a
    /// later-added entry wins for an overlapping range (faithful to upstream).
    pub(crate) fn get(&self, cp: u32) -> Option<&Descriptor> {
        self.entries
            .iter()
            .rev()
            .find(|e| e.range[0] <= cp && cp <= e.range[1])
            .map(|e| &e.descriptor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc(family: &str) -> Descriptor {
        Descriptor {
            family: Some(family.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn codepoint_map_get() {
        let mut m = CodepointMap::default();
        m.add([0x41, 0x5A], desc("Range")); // 'A'..='Z'
        assert!(m.get(0x41).is_some());
        assert!(m.get(0x5A).is_some());
        assert!(m.get(0x40).is_none());
        assert!(m.get(0x5B).is_none());
    }

    #[test]
    fn codepoint_map_reverse_priority() {
        let mut m = CodepointMap::default();
        m.add([0x0, 0xFFFF], desc("D1"));
        m.add([0x41, 0x41], desc("D2"));
        // The later, more specific entry wins for 0x41.
        assert_eq!(m.get(0x41).unwrap().family.as_deref(), Some("D2"));
        // 0x42 is only in the broad range.
        assert_eq!(m.get(0x42).unwrap().family.as_deref(), Some("D1"));
    }

    #[test]
    #[should_panic]
    fn codepoint_map_rejects_inverted_range() {
        let mut m = CodepointMap::default();
        m.add([0x5A, 0x41], desc("bad"));
    }
}
