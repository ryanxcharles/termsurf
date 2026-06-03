//! Shared SFNT scalar types and a big-endian reader.
//!
//! Faithful port of the parts of upstream `font/opentype/sfnt.zig` needed by the
//! `head`/`hhea`/`post` table parsers (scalar types, `Fixed`, `Version16Dot16`,
//! and a big-endian reader). `F26Dot6` and the whole-file `SFNT`
//! table-directory reader are deferred to later slices.

/// An error parsing an OpenType table — the analog of upstream
/// `error{EndOfStream}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenTypeError {
    /// The input ended before a field could be read in full.
    EndOfStream,
}

/// A 16.16 signed fixed-point number (`Fixed` in the spec), stored as its raw
/// `i32`. Used by `head.fontRevision`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fixed(pub i32);

impl Fixed {
    /// Convert to a float by dividing out the 16 fractional bits.
    pub(crate) fn to_f64(self) -> f64 {
        self.0 as f64 / 65536.0
    }

    /// Convert from a float, rounding to the nearest 16.16 value.
    pub(crate) fn from_f64(v: f64) -> Fixed {
        Fixed((v * 65536.0).round() as i32)
    }
}

/// A `Version16Dot16` version number: a `u32` whose high 16 bits are the major
/// version and low 16 bits the minor. Used by `post.version`.
///
/// Upstream is `packed struct(u32) { minor: u16, major: u16 }`; a Zig packed
/// struct fills from the least-significant bit, so `minor` is the low half and
/// `major` the high half.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Version16Dot16 {
    pub major: u16,
    pub minor: u16,
}

impl Version16Dot16 {
    /// Split a big-endian-read `u32` into `{ major (high 16), minor (low 16) }`.
    pub(crate) fn from_u32(raw: u32) -> Version16Dot16 {
        Version16Dot16 {
            major: (raw >> 16) as u16,
            minor: (raw & 0xFFFF) as u16,
        }
    }
}

/// A minimal big-endian cursor over a byte slice — the faithful, safe analog of
/// Zig's `fixedBufferStream(data).reader().readStructEndian(T, .big)`. Each
/// `read_*` consumes the next N bytes big-endian and returns
/// [`OpenTypeError::EndOfStream`] if fewer than N remain.
///
/// The SFNT type aliases map directly: `uint16`/`UFWORD` → [`read_u16`], `int16`/
/// `FWORD` → [`read_i16`], `uint32` → [`read_u32`], `int32` → [`read_i32`], and
/// `LONGDATETIME` → [`read_i64`].
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Reader<'a> {
        Reader { data, pos: 0 }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], OpenTypeError> {
        let end = self.pos + N;
        if end > self.data.len() {
            return Err(OpenTypeError::EndOfStream);
        }
        let mut buf = [0u8; N];
        buf.copy_from_slice(&self.data[self.pos..end]);
        self.pos = end;
        Ok(buf)
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16, OpenTypeError> {
        Ok(u16::from_be_bytes(self.take::<2>()?))
    }

    pub(crate) fn read_i16(&mut self) -> Result<i16, OpenTypeError> {
        Ok(i16::from_be_bytes(self.take::<2>()?))
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32, OpenTypeError> {
        Ok(u32::from_be_bytes(self.take::<4>()?))
    }

    pub(crate) fn read_i32(&mut self) -> Result<i32, OpenTypeError> {
        Ok(i32::from_be_bytes(self.take::<4>()?))
    }

    pub(crate) fn read_i64(&mut self) -> Result<i64, OpenTypeError> {
        Ok(i64::from_be_bytes(self.take::<8>()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_round_trip() {
        // 0.05499267578125 * 65536 == 3604 exactly.
        let f = Fixed::from_f64(0.054_992_675_781_25);
        assert_eq!(f.0, 3604);
        assert_eq!(f.to_f64(), 0.054_992_675_781_25);
        // 1.0 is the raw value 65536.
        assert_eq!(Fixed::from_f64(1.0).0, 65536);
    }

    #[test]
    fn reader_big_endian() {
        let bytes = [
            0x12, 0x34, // u16 = 0x1234
            0xFF, 0xFE, // i16 = -2
            0x00, 0x01, 0x00, 0x00, // u32 = 0x10000
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2A, // i64 = 42
        ];
        let mut r = Reader::new(&bytes);
        assert_eq!(r.read_u16().unwrap(), 0x1234);
        assert_eq!(r.read_i16().unwrap(), -2);
        assert_eq!(r.read_u32().unwrap(), 0x0001_0000);
        assert_eq!(r.read_i64().unwrap(), 42);
    }

    #[test]
    fn reader_end_of_stream() {
        let mut r = Reader::new(&[0x00]); // only 1 byte
        assert_eq!(r.read_u16(), Err(OpenTypeError::EndOfStream));

        let mut r = Reader::new(&[0x00, 0x00, 0x00]); // 3 bytes, need 4
        assert_eq!(r.read_u32(), Err(OpenTypeError::EndOfStream));
    }

    #[test]
    fn version16dot16_layout() {
        // major is the high 16 bits, minor the low 16.
        assert_eq!(
            Version16Dot16::from_u32(0x0002_0000),
            Version16Dot16 { major: 2, minor: 0 }
        );
        assert_eq!(
            Version16Dot16::from_u32(0x0001_0005),
            Version16Dot16 { major: 1, minor: 5 }
        );
    }
}
