//! Terminal device attribute responses.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Request {
    Primary,
    Secondary,
    Tertiary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Attributes {
    pub(super) primary: Primary,
    pub(super) secondary: Secondary,
    pub(super) tertiary: Tertiary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Primary {
    pub(super) conformance_level: ConformanceLevel,
    pub(super) features: &'static [Feature],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Secondary {
    pub(super) device_type: DeviceType,
    pub(super) firmware_version: u16,
    pub(super) rom_cartridge: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Tertiary {
    pub(super) unit_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub(super) enum ConformanceLevel {
    Vt100 = 1,
    Level2 = 62,
    Level3 = 63,
    Level4 = 64,
    Level5 = 65,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub(super) enum Feature {
    Columns132 = 1,
    SelectiveErase = 6,
    AnsiColor = 22,
    Clipboard = 52,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub(super) enum DeviceType {
    Vt220 = 1,
    Vt420 = 41,
}

const DEFAULT_PRIMARY_FEATURES: &[Feature] = &[Feature::AnsiColor];

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
            conformance_level: ConformanceLevel::Level2,
            features: DEFAULT_PRIMARY_FEATURES,
        }
    }
}

impl Default for Secondary {
    fn default() -> Self {
        Self {
            device_type: DeviceType::Vt220,
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
        let mut response = format!("\x1b[?{}", self.conformance_level as u16);
        for feature in self.features {
            response.push_str(&format!(";{}", *feature as u16));
        }
        response.push('c');
        response
    }
}

impl Secondary {
    fn encode_vt(self) -> String {
        format!(
            "\x1b[>{};{};{}c",
            self.device_type as u16, self.firmware_version, self.rom_cartridge
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
                conformance_level: ConformanceLevel::Level4,
                features: &[
                    Feature::Columns132,
                    Feature::SelectiveErase,
                    Feature::AnsiColor,
                ],
            },
            ..Attributes::default()
        };
        assert_eq!(attrs.encode_vt(Request::Primary), "\x1b[?64;1;6;22c");
    }

    #[test]
    fn device_attributes_secondary_default_and_custom_encoding() {
        assert_eq!(
            Attributes::default().encode_vt(Request::Secondary),
            "\x1b[>1;0;0c"
        );
        let attrs = Attributes {
            secondary: Secondary {
                device_type: DeviceType::Vt420,
                firmware_version: 100,
                rom_cartridge: 0,
            },
            ..Attributes::default()
        };
        assert_eq!(attrs.encode_vt(Request::Secondary), "\x1b[>41;100;0c");
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
