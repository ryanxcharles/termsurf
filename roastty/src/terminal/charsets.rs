//! Terminal character set state.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CharsetSlot {
    G0,
    G1,
    G2,
    G3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CharsetGrSlot {
    G1,
    G2,
    G3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CharsetBank {
    Gl,
    Gr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Charset {
    Utf8,
    Ascii,
    British,
    DecSpecial,
}

impl CharsetSlot {
    pub(super) const fn designation_intermediate(self) -> u8 {
        match self {
            Self::G0 => b'(',
            Self::G1 => b')',
            Self::G2 => b'*',
            Self::G3 => b'+',
        }
    }
}

impl CharsetGrSlot {
    pub(super) const fn invocation_sequence(self) -> Option<&'static str> {
        match self {
            Self::G1 => Some("\x1b~"),
            Self::G2 => None,
            Self::G3 => Some("\x1b|"),
        }
    }
}

impl Charset {
    pub(super) const fn designation_final(self) -> Option<u8> {
        match self {
            Self::Utf8 => None,
            Self::Ascii => Some(b'B'),
            Self::British => Some(b'A'),
            Self::DecSpecial => Some(b'0'),
        }
    }

    pub(super) const fn table(self) -> Option<&'static [u16; 256]> {
        match self {
            Self::Utf8 => None,
            Self::Ascii => Some(&ASCII_TABLE),
            Self::British => Some(&BRITISH_TABLE),
            Self::DecSpecial => Some(&DEC_SPECIAL_TABLE),
        }
    }
}

const fn ascii_table() -> [u16; 256] {
    let mut table = [0; 256];
    let mut idx = 0;
    while idx < 256 {
        table[idx] = idx as u16;
        idx += 1;
    }
    table
}

const fn british_table() -> [u16; 256] {
    let mut table = ascii_table();
    table[0x23] = 0x00a3;
    table
}

const fn dec_special_table() -> [u16; 256] {
    let mut table = ascii_table();
    table[0x60] = 0x25c6;
    table[0x61] = 0x2592;
    table[0x62] = 0x2409;
    table[0x63] = 0x240c;
    table[0x64] = 0x240d;
    table[0x65] = 0x240a;
    table[0x66] = 0x00b0;
    table[0x67] = 0x00b1;
    table[0x68] = 0x2424;
    table[0x69] = 0x240b;
    table[0x6a] = 0x2518;
    table[0x6b] = 0x2510;
    table[0x6c] = 0x250c;
    table[0x6d] = 0x2514;
    table[0x6e] = 0x253c;
    table[0x6f] = 0x23ba;
    table[0x70] = 0x23bb;
    table[0x71] = 0x2500;
    table[0x72] = 0x23bc;
    table[0x73] = 0x23bd;
    table[0x74] = 0x251c;
    table[0x75] = 0x2524;
    table[0x76] = 0x2534;
    table[0x77] = 0x252c;
    table[0x78] = 0x2502;
    table[0x79] = 0x2264;
    table[0x7a] = 0x2265;
    table[0x7b] = 0x03c0;
    table[0x7c] = 0x2260;
    table[0x7d] = 0x00a3;
    table[0x7e] = 0x00b7;
    table
}

const ASCII_TABLE: [u16; 256] = ascii_table();
const BRITISH_TABLE: [u16; 256] = british_table();
const DEC_SPECIAL_TABLE: [u16; 256] = dec_special_table();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charset_tables_have_upstream_length() {
        for charset in [Charset::Ascii, Charset::British, Charset::DecSpecial] {
            assert_eq!(charset.table().unwrap().len(), 256);
        }
        assert!(Charset::Utf8.table().is_none());
    }

    #[test]
    fn charset_ascii_table_maps_bytes_to_themselves() {
        let table = Charset::Ascii.table().unwrap();
        for (idx, value) in table.iter().enumerate() {
            assert_eq!(*value, idx as u16);
        }
    }

    #[test]
    fn charset_british_table_matches_upstream_pound_mapping() {
        let table = Charset::British.table().unwrap();

        assert_eq!(table[0x23], 0x00a3);
        assert_eq!(table[0x24], 0x24);
    }

    #[test]
    fn charset_dec_special_table_matches_upstream_key_mappings() {
        let table = Charset::DecSpecial.table().unwrap();

        assert_eq!(table[0x60], 0x25c6);
        assert_eq!(table[0x6a], 0x2518);
        assert_eq!(table[0x6b], 0x2510);
        assert_eq!(table[0x6c], 0x250c);
        assert_eq!(table[0x6d], 0x2514);
        assert_eq!(table[0x71], 0x2500);
        assert_eq!(table[0x78], 0x2502);
        assert_eq!(table[0x7e], 0x00b7);
    }

    #[test]
    fn charset_designation_final_bytes_match_upstream() {
        assert_eq!(Charset::Utf8.designation_final(), None);
        assert_eq!(Charset::Ascii.designation_final(), Some(b'B'));
        assert_eq!(Charset::British.designation_final(), Some(b'A'));
        assert_eq!(Charset::DecSpecial.designation_final(), Some(b'0'));
    }

    #[test]
    fn charset_gr_slot_invocation_sequences_match_upstream() {
        assert_eq!(CharsetGrSlot::G1.invocation_sequence(), Some("\x1b~"));
        assert_eq!(CharsetGrSlot::G2.invocation_sequence(), None);
        assert_eq!(CharsetGrSlot::G3.invocation_sequence(), Some("\x1b|"));
    }
}
