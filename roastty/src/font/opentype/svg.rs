//! The `SVG ` (SVG glyph descriptions) table.
//!
//! Faithful port of upstream `font/opentype/svg.zig`. This is **not** a general
//! SVG-table reader — it reads only what is needed to answer "is glyph N in the
//! table?" (used by the color-font detection to flag SVG color glyphs).
//!
//! Reference: <https://learn.microsoft.com/en-us/typography/opentype/spec/svg>

use std::cmp::Ordering;

use super::sfnt::{OpenTypeError, Reader};

/// The parsed `SVG ` table: the inclusive glyph-id span it covers and its raw
/// 12-byte document records (each an `SVGDocumentRecord`, big-endian:
/// `startGlyphID: u16, endGlyphID: u16, svgDocOffset: u32, svgDocLength: u32`).
/// The records are sorted by glyph id, so [`Svg::has_glyph`] can binary-search.
#[derive(Debug)]
pub(crate) struct Svg {
    /// The first glyph id covered by the table (record 0's `startGlyphID`).
    start_glyph_id: u16,
    /// The last glyph id covered by the table (the final record's `endGlyphID`).
    end_glyph_id: u16,
    /// The raw document records, in glyph-id order.
    records: Vec<[u8; 12]>,
}

impl Svg {
    /// Parse the `SVG ` table from its raw bytes. Returns
    /// [`OpenTypeError::UnsupportedVersion`] for a non-zero table version and
    /// [`OpenTypeError::EndOfStream`] if the data is truncated.
    pub(crate) fn from_bytes(data: &[u8]) -> Result<Svg, OpenTypeError> {
        let mut r = Reader::new(data);

        // Version: only 0 is defined.
        if r.read_u16()? != 0 {
            return Err(OpenTypeError::UnsupportedVersion);
        }

        // Offset to the SVG document list.
        let offset = r.read_u32()? as usize;

        // Seek to the document list and read its records.
        let list = data.get(offset..).ok_or(OpenTypeError::EndOfStream)?;
        let mut lr = Reader::new(list);
        let len = lr.read_u16()? as usize;
        if len == 0 {
            return Err(OpenTypeError::EndOfStream);
        }
        let mut records = Vec::with_capacity(len);
        for _ in 0..len {
            records.push(lr.read_bytes::<12>()?);
        }

        // The covered span: record 0's start, the final record's end (which is
        // record 0's end when there is a single record).
        let (start, _) = glyph_range(&records[0]);
        let (_, end) = glyph_range(&records[len - 1]);
        Ok(Svg {
            start_glyph_id: start,
            end_glyph_id: end,
            records,
        })
    }

    /// Whether `glyph_id` has an SVG document in the table.
    pub(crate) fn has_glyph(&self, glyph_id: u16) -> bool {
        // Fast path: outside the covered span.
        if glyph_id < self.start_glyph_id || glyph_id > self.end_glyph_id {
            return false;
        }
        // Fast path: the span endpoints.
        if glyph_id == self.start_glyph_id || glyph_id == self.end_glyph_id {
            return true;
        }
        // Slow path: binary-search the records by their `[start, end]` range.
        self.records
            .binary_search_by(|record| {
                let (start, end) = glyph_range(record);
                if glyph_id < start {
                    Ordering::Greater
                } else if glyph_id > end {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
            .is_ok()
    }
}

/// The `[startGlyphID, endGlyphID]` of a 12-byte document record.
fn glyph_range(record: &[u8; 12]) -> (u16, u16) {
    (
        u16::from_be_bytes([record[0], record[1]]),
        u16::from_be_bytes([record[2], record[3]]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal `SVG ` table: version 0, a document list at `offset`, and
    /// the given `(start, end)` records (each with zeroed doc offset/length).
    fn build_table(offset: usize, records: &[(u16, u16)]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&0u16.to_be_bytes()); // version
        v.extend_from_slice(&(offset as u32).to_be_bytes()); // doc-list offset
                                                             // Pad to `offset` (the header is 6 bytes; `offset` is >= 6 in practice).
        while v.len() < offset {
            v.push(0);
        }
        v.extend_from_slice(&(records.len() as u16).to_be_bytes()); // numEntries
        for &(s, e) in records {
            v.extend_from_slice(&s.to_be_bytes());
            v.extend_from_slice(&e.to_be_bytes());
            v.extend_from_slice(&0u32.to_be_bytes()); // svgDocOffset
            v.extend_from_slice(&0u32.to_be_bytes()); // svgDocLength
        }
        v
    }

    #[test]
    fn from_bytes_single_record() {
        // Mirrors upstream's JuliaMono assertion (glyph 11482) without the font.
        let table = build_table(6, &[(11482, 11482)]);
        let svg = Svg::from_bytes(&table).expect("parses");
        assert_eq!(svg.start_glyph_id, 11482);
        assert_eq!(svg.end_glyph_id, 11482);
        assert!(svg.has_glyph(11482));
        assert!(!svg.has_glyph(11481));
        assert!(!svg.has_glyph(11483));
    }

    #[test]
    fn from_bytes_multi_record() {
        let table = build_table(6, &[(10, 12), (20, 22), (40, 42)]);
        let svg = Svg::from_bytes(&table).expect("parses");
        assert_eq!(svg.start_glyph_id, 10);
        assert_eq!(svg.end_glyph_id, 42);
        // Inside each range, including a non-endpoint middle record.
        assert!(svg.has_glyph(10));
        assert!(svg.has_glyph(11));
        assert!(svg.has_glyph(12));
        assert!(svg.has_glyph(21), "non-endpoint middle record");
        assert!(svg.has_glyph(40));
        assert!(svg.has_glyph(42));
        // In the gaps between ranges.
        assert!(!svg.has_glyph(13));
        assert!(!svg.has_glyph(30));
        assert!(!svg.has_glyph(43));
        assert!(!svg.has_glyph(9));
    }

    #[test]
    fn from_bytes_offset_beyond_header() {
        // A document list placed past the 6-byte header (extra padding between).
        let table = build_table(16, &[(5, 7)]);
        let svg = Svg::from_bytes(&table).expect("parses");
        assert!(svg.has_glyph(6));
        assert!(!svg.has_glyph(8));
    }

    #[test]
    fn from_bytes_bad_version() {
        let mut table = build_table(6, &[(1, 2)]);
        table[0] = 0;
        table[1] = 1; // version = 1
        assert!(matches!(
            Svg::from_bytes(&table),
            Err(OpenTypeError::UnsupportedVersion)
        ));
    }

    #[test]
    fn from_bytes_truncated() {
        // Too short for the header.
        assert!(matches!(
            Svg::from_bytes(&[0u8; 3]),
            Err(OpenTypeError::EndOfStream)
        ));
        // Offset points past the end (drop the document list).
        let mut table = build_table(6, &[(1, 2)]);
        table.truncate(6);
        assert!(matches!(
            Svg::from_bytes(&table),
            Err(OpenTypeError::EndOfStream)
        ));
        // Declares more records than the data holds: numEntries is at byte
        // `offset` (6); bump it to 3 with only 1 record present.
        let mut table = build_table(6, &[(1, 2)]);
        table[6] = 0;
        table[7] = 3;
        assert!(matches!(
            Svg::from_bytes(&table),
            Err(OpenTypeError::EndOfStream)
        ));
        // Zero records.
        let mut empty = Vec::new();
        empty.extend_from_slice(&0u16.to_be_bytes());
        empty.extend_from_slice(&6u32.to_be_bytes());
        empty.extend_from_slice(&0u16.to_be_bytes()); // numEntries = 0
        assert!(matches!(
            Svg::from_bytes(&empty),
            Err(OpenTypeError::EndOfStream)
        ));
    }
}
