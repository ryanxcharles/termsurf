#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mod {
    Shift,
    Ctrl,
    Alt,
    Super,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OptionAsAlt {
    False,
    True,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ModSides {
    pub(crate) shift: Side,
    pub(crate) ctrl: Side,
    pub(crate) alt: Side,
    pub(crate) super_: Side,
}

impl Default for Side {
    fn default() -> Self {
        Self::Left
    }
}

impl ModSides {
    fn int(self) -> u16 {
        ((self.shift as u16) << 6)
            | ((self.ctrl as u16) << 7)
            | ((self.alt as u16) << 8)
            | ((self.super_ as u16) << 9)
    }

    fn from_int(value: u16) -> Self {
        Self {
            shift: side_from_bit(value, 6),
            ctrl: side_from_bit(value, 7),
            alt: side_from_bit(value, 8),
            super_: side_from_bit(value, 9),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ModKeys {
    pub(crate) shift: bool,
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
    pub(crate) super_: bool,
}

impl ModKeys {
    pub(crate) fn int(self) -> u8 {
        self.shift as u8
            | ((self.ctrl as u8) << 1)
            | ((self.alt as u8) << 2)
            | ((self.super_ as u8) << 3)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Mods {
    pub(crate) shift: bool,
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
    pub(crate) super_: bool,
    pub(crate) caps_lock: bool,
    pub(crate) num_lock: bool,
    pub(crate) sides: ModSides,
}

impl Mods {
    pub(crate) const fn new() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            super_: false,
            caps_lock: false,
            num_lock: false,
            sides: ModSides {
                shift: Side::Left,
                ctrl: Side::Left,
                alt: Side::Left,
                super_: Side::Left,
            },
        }
    }

    pub(crate) fn for_mod(modifier: Mod, side: Side) -> Self {
        let mut mods = Self::new();
        match modifier {
            Mod::Shift => {
                mods.shift = true;
                mods.sides.shift = side;
            }
            Mod::Ctrl => {
                mods.ctrl = true;
                mods.sides.ctrl = side;
            }
            Mod::Alt => {
                mods.alt = true;
                mods.sides.alt = side;
            }
            Mod::Super => {
                mods.super_ = true;
                mods.sides.super_ = side;
            }
        }
        mods
    }

    pub(crate) fn int(self) -> u16 {
        self.shift as u16
            | ((self.ctrl as u16) << 1)
            | ((self.alt as u16) << 2)
            | ((self.super_ as u16) << 3)
            | ((self.caps_lock as u16) << 4)
            | ((self.num_lock as u16) << 5)
            | self.sides.int()
    }

    pub(crate) fn from_int(value: u16) -> Self {
        Self {
            shift: value & (1 << 0) != 0,
            ctrl: value & (1 << 1) != 0,
            alt: value & (1 << 2) != 0,
            super_: value & (1 << 3) != 0,
            caps_lock: value & (1 << 4) != 0,
            num_lock: value & (1 << 5) != 0,
            sides: ModSides::from_int(value),
        }
    }

    pub(crate) fn empty(self) -> bool {
        self.int() == 0
    }

    pub(crate) fn keys(self) -> ModKeys {
        ModKeys {
            shift: self.shift,
            ctrl: self.ctrl,
            alt: self.alt,
            super_: self.super_,
        }
    }

    pub(crate) fn binding(self) -> Self {
        Self {
            shift: self.shift,
            ctrl: self.ctrl,
            alt: self.alt,
            super_: self.super_,
            ..Self::new()
        }
    }

    pub(crate) fn unset(self, other: Self) -> Self {
        Self::from_int(self.int() & !other.int())
    }

    pub(crate) fn without_locks(self) -> Self {
        Self {
            caps_lock: false,
            num_lock: false,
            ..self
        }
    }

    pub(crate) fn translation(self, option_as_alt: OptionAsAlt) -> Self {
        let mut result = self;
        if !self.alt {
            return result;
        }

        match option_as_alt {
            OptionAsAlt::False => return result,
            OptionAsAlt::True => {}
            OptionAsAlt::Left if self.sides.alt == Side::Right => return result,
            OptionAsAlt::Right if self.sides.alt == Side::Left => return result,
            OptionAsAlt::Left | OptionAsAlt::Right => {}
        }

        result.alt = false;
        result
    }

    pub(crate) fn ctrl_or_super(self) -> bool {
        self.super_
    }
}

fn side_from_bit(value: u16, bit: u16) -> Side {
    if value & (1 << bit) == 0 {
        Side::Left
    } else {
        Side::Right
    }
}

pub(crate) fn ctrl_or_super(mut mods: Mods) -> Mods {
    mods.super_ = true;
    mods
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_mods_layout_matches_upstream_examples() {
        assert_eq!(Mods::new().int(), 0);
        assert_eq!(
            Mods {
                shift: true,
                ..Mods::new()
            }
            .int(),
            0b0000_0001
        );
        assert_eq!(Mods::for_mod(Mod::Shift, Side::Right).int(), 0b0100_0001);
        assert_eq!(Mods::for_mod(Mod::Alt, Side::Right).int(), 0b1_0000_0100);
    }

    #[test]
    fn key_mods_helpers_match_upstream_shape() {
        let mods = Mods {
            shift: true,
            alt: true,
            caps_lock: true,
            num_lock: true,
            sides: ModSides {
                alt: Side::Right,
                ..ModSides::default()
            },
            ..Mods::new()
        };

        assert!(!mods.empty());
        assert_eq!(mods.keys().int(), 0b0101);
        assert_eq!(
            mods.binding(),
            Mods {
                shift: true,
                alt: true,
                ..Mods::new()
            }
        );
        assert_eq!(
            mods.without_locks(),
            Mods {
                caps_lock: false,
                num_lock: false,
                ..mods
            }
        );
        assert_eq!(
            mods.unset(Mods {
                shift: true,
                ..Mods::new()
            }),
            Mods {
                alt: true,
                caps_lock: true,
                num_lock: true,
                sides: ModSides {
                    alt: Side::Right,
                    ..ModSides::default()
                },
                ..Mods::new()
            }
        );
    }

    #[test]
    fn key_mods_translation_macos_option_as_alt() {
        let left_alt = Mods::for_mod(Mod::Alt, Side::Left);
        let right_alt = Mods::for_mod(Mod::Alt, Side::Right);

        assert_eq!(left_alt.translation(OptionAsAlt::False), left_alt);
        assert!(!left_alt.translation(OptionAsAlt::True).alt);
        assert!(!left_alt.translation(OptionAsAlt::Left).alt);
        assert_eq!(left_alt.translation(OptionAsAlt::Right), left_alt);
        assert_eq!(right_alt.translation(OptionAsAlt::Left), right_alt);
        assert!(!right_alt.translation(OptionAsAlt::Right).alt);

        let shifted_alt = Mods {
            shift: true,
            ..left_alt
        };
        assert_eq!(
            shifted_alt.translation(OptionAsAlt::True),
            Mods {
                shift: true,
                ..Mods::new()
            }
        );
    }

    #[test]
    fn key_mods_ctrl_or_super_is_macos_super() {
        assert!(Mods {
            super_: true,
            ..Mods::new()
        }
        .ctrl_or_super());
        assert!(!Mods {
            ctrl: true,
            ..Mods::new()
        }
        .ctrl_or_super());
        assert_eq!(
            ctrl_or_super(Mods::new()),
            Mods {
                super_: true,
                ..Mods::new()
            }
        );
    }
}
