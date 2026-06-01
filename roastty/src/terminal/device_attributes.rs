//! Terminal device attribute responses.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Request {
    Primary,
    Secondary,
    Tertiary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Attributes {
    pub(super) primary: Primary,
    pub(super) secondary: Secondary,
    pub(super) tertiary: Tertiary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Primary {
    pub(super) conformance_level: u16,
    pub(super) features: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Secondary {
    pub(super) device_type: u16,
    pub(super) firmware_version: u16,
    pub(super) rom_cartridge: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Tertiary {
    pub(super) unit_id: u32,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            primary: Primary::default(),
            secondary: Secondary::default(),
            tertiary: Tertiary::default(),
        }
    }
}

impl Default for Primary {
    fn default() -> Self {
        Self {
            conformance_level: 62,
            features: vec![22],
        }
    }
}

impl Default for Secondary {
    fn default() -> Self {
        Self {
            device_type: 1,
            firmware_version: 0,
            rom_cartridge: 0,
        }
    }
}

impl Default for Tertiary {
    fn default() -> Self {
        Self { unit_id: 0 }
    }
}

impl Attributes {
    pub(super) fn encode_vt(self, request: Request) -> String {
        match request {
            Request::Primary => self.primary.encode_vt(),
            Request::Secondary => self.secondary.encode_vt(),
            Request::Tertiary => self.tertiary.encode_vt(),
        }
    }
}

impl Primary {
    fn encode_vt(self) -> String {
        let mut response = format!("\x1b[?{}", self.conformance_level);
        for feature in self.features {
            response.push_str(&format!(";{feature}"));
        }
        response.push('c');
        response
    }
}

impl Secondary {
    fn encode_vt(self) -> String {
        format!(
            "\x1b[>{};{};{}c",
            self.device_type, self.firmware_version, self.rom_cartridge
        )
    }
}

impl Tertiary {
    fn encode_vt(self) -> String {
        format!("\x1bP!|{:08X}\x1b\\", self.unit_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_attributes_primary_default_and_custom_encoding() {
        assert_eq!(
            Attributes::default().encode_vt(Request::Primary),
            "\x1b[?62;22c"
        );
        let attrs = Attributes {
            primary: Primary {
                conformance_level: 64,
                features: vec![1, 6, 22],
            },
            ..Attributes::default()
        };
        assert_eq!(attrs.encode_vt(Request::Primary), "\x1b[?64;1;6;22c");

        let unknown_attrs = Attributes {
            primary: Primary {
                conformance_level: 777,
                features: vec![444, 555],
            },
            ..Attributes::default()
        };
        assert_eq!(
            unknown_attrs.encode_vt(Request::Primary),
            "\x1b[?777;444;555c"
        );
    }

    #[test]
    fn device_attributes_secondary_default_and_custom_encoding() {
        assert_eq!(
            Attributes::default().encode_vt(Request::Secondary),
            "\x1b[>1;0;0c"
        );
        let attrs = Attributes {
            secondary: Secondary {
                device_type: 41,
                firmware_version: 100,
                rom_cartridge: 0,
            },
            ..Attributes::default()
        };
        assert_eq!(attrs.encode_vt(Request::Secondary), "\x1b[>41;100;0c");

        let unknown_attrs = Attributes {
            secondary: Secondary {
                device_type: 777,
                firmware_version: 100,
                rom_cartridge: 2,
            },
            ..Attributes::default()
        };
        assert_eq!(
            unknown_attrs.encode_vt(Request::Secondary),
            "\x1b[>777;100;2c"
        );
    }

    #[test]
    fn device_attributes_tertiary_default_and_custom_encoding() {
        assert_eq!(
            Attributes::default().encode_vt(Request::Tertiary),
            "\x1bP!|00000000\x1b\\"
        );
        let attrs = Attributes {
            tertiary: Tertiary {
                unit_id: 0xAABBCCDD,
            },
            ..Attributes::default()
        };
        assert_eq!(attrs.encode_vt(Request::Tertiary), "\x1bP!|AABBCCDD\x1b\\");
    }
}
