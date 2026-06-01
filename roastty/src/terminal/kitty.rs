//! Kitty keyboard protocol state.

const KEY_FLAG_STACK_LEN: usize = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct KeyFlags {
    pub(super) disambiguate: bool,
    pub(super) report_events: bool,
    pub(super) report_alternates: bool,
    pub(super) report_all: bool,
    pub(super) report_associated: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum KeySetMode {
    #[default]
    Set,
    Or,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct KeyFlagStack {
    flags: [KeyFlags; KEY_FLAG_STACK_LEN],
    idx: usize,
}

impl KeyFlags {
    pub(super) const DISABLED: Self = Self {
        disambiguate: false,
        report_events: false,
        report_alternates: false,
        report_all: false,
        report_associated: false,
    };

    pub(super) const TRUE: Self = Self {
        disambiguate: true,
        report_events: true,
        report_alternates: true,
        report_all: true,
        report_associated: true,
    };

    pub(super) const fn int(self) -> u8 {
        (self.disambiguate as u8)
            | ((self.report_events as u8) << 1)
            | ((self.report_alternates as u8) << 2)
            | ((self.report_all as u8) << 3)
            | ((self.report_associated as u8) << 4)
    }

    pub(super) const fn is_disabled(self) -> bool {
        self.int() == Self::DISABLED.int()
    }

    const fn from_int(value: u8) -> Self {
        Self {
            disambiguate: value & 0b00001 != 0,
            report_events: value & 0b00010 != 0,
            report_alternates: value & 0b00100 != 0,
            report_all: value & 0b01000 != 0,
            report_associated: value & 0b10000 != 0,
        }
    }
}

impl KeyFlagStack {
    pub(super) const fn current(self) -> KeyFlags {
        self.flags[self.idx]
    }

    #[cfg(test)]
    pub(super) fn set(&mut self, mode: KeySetMode, flags: KeyFlags) {
        let current = self.current();
        self.flags[self.idx] = match mode {
            KeySetMode::Set => flags,
            KeySetMode::Or => KeyFlags::from_int(current.int() | flags.int()),
            KeySetMode::Not => KeyFlags::from_int(current.int() & !flags.int()),
        };
    }

    #[cfg(test)]
    pub(super) fn push(&mut self, flags: KeyFlags) {
        self.idx = (self.idx + 1) % self.flags.len();
        self.flags[self.idx] = flags;
    }

    #[cfg(test)]
    pub(super) fn pop(&mut self, n: usize) {
        if n >= self.flags.len() {
            *self = Self::default();
            return;
        }

        for _ in 0..n {
            self.flags[self.idx] = KeyFlags::DISABLED;
            self.idx = (self.idx + self.flags.len() - 1) % self.flags.len();
        }
    }
}

impl Default for KeyFlagStack {
    fn default() -> Self {
        Self {
            flags: [KeyFlags::DISABLED; KEY_FLAG_STACK_LEN],
            idx: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(value: u8) -> KeyFlags {
        KeyFlags::from_int(value)
    }

    #[test]
    fn key_flags_pack_bits_in_upstream_order() {
        assert_eq!(
            KeyFlags {
                disambiguate: true,
                ..KeyFlags::DISABLED
            }
            .int(),
            0b00001
        );
        assert_eq!(
            KeyFlags {
                report_events: true,
                ..KeyFlags::DISABLED
            }
            .int(),
            0b00010
        );
        assert_eq!(
            KeyFlags {
                report_alternates: true,
                ..KeyFlags::DISABLED
            }
            .int(),
            0b00100
        );
        assert_eq!(
            KeyFlags {
                report_all: true,
                ..KeyFlags::DISABLED
            }
            .int(),
            0b01000
        );
        assert_eq!(
            KeyFlags {
                report_associated: true,
                ..KeyFlags::DISABLED
            }
            .int(),
            0b10000
        );
        assert_eq!(KeyFlags::TRUE.int(), 0b11111);
    }

    #[test]
    fn key_flag_stack_defaults_to_disabled() {
        let stack = KeyFlagStack::default();

        assert_eq!(stack.current(), KeyFlags::DISABLED);
        assert!(stack.current().is_disabled());
    }

    #[test]
    fn key_flag_stack_set_replaces_current_flags() {
        let mut stack = KeyFlagStack::default();

        stack.set(KeySetMode::Set, flags(0b10001));

        assert_eq!(stack.current().int(), 0b10001);
    }

    #[test]
    fn key_flag_stack_or_adds_flags_to_current() {
        let mut stack = KeyFlagStack::default();
        stack.set(KeySetMode::Set, flags(0b00001));

        stack.set(KeySetMode::Or, flags(0b01010));

        assert_eq!(stack.current().int(), 0b01011);
    }

    #[test]
    fn key_flag_stack_not_clears_flags_from_current() {
        let mut stack = KeyFlagStack::default();
        stack.set(KeySetMode::Set, KeyFlags::TRUE);

        stack.set(KeySetMode::Not, flags(0b01010));

        assert_eq!(stack.current().int(), 0b10101);
    }

    #[test]
    fn key_flag_stack_push_and_pop_restore_previous_flags() {
        let mut stack = KeyFlagStack::default();
        stack.set(KeySetMode::Set, flags(0b00001));
        stack.push(flags(0b00010));

        assert_eq!(stack.current().int(), 0b00010);

        stack.pop(1);

        assert_eq!(stack.current().int(), 0b00001);
    }

    #[test]
    fn key_flag_stack_pop_exact_length_resets_to_disabled() {
        let mut stack = KeyFlagStack::default();
        for value in 1..=8 {
            stack.push(flags(value));
        }

        stack.pop(8);

        assert_eq!(stack.current(), KeyFlags::DISABLED);
        assert_eq!(stack, KeyFlagStack::default());
    }

    #[test]
    fn key_flag_stack_pop_larger_than_length_resets_to_disabled() {
        let mut stack = KeyFlagStack::default();
        stack.push(flags(0b11111));

        stack.pop(100);

        assert_eq!(stack.current(), KeyFlags::DISABLED);
        assert_eq!(stack, KeyFlagStack::default());
    }

    #[test]
    fn key_flag_stack_wraps_and_evicts_oldest_slot() {
        let mut stack = KeyFlagStack::default();
        for value in 1..=9 {
            stack.push(flags(value));
        }

        assert_eq!(stack.current().int(), 9);

        for expected in (3..=9).rev() {
            stack.pop(1);
            assert_eq!(stack.current().int(), expected - 1);
        }

        stack.pop(1);
        assert_eq!(stack.current(), KeyFlags::DISABLED);
    }
}
