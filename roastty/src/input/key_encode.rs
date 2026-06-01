use super::key::{Key, KeyAction, KeyEvent};
use super::key_mods::{Mods, OptionAsAlt, Side};
use crate::terminal::kitty::KeyFlags;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Options {
    pub(crate) cursor_key_application: bool,
    pub(crate) keypad_key_application: bool,
    pub(crate) backarrow_key_mode: bool,
    pub(crate) ignore_keypad_with_numlock: bool,
    pub(crate) alt_esc_prefix: bool,
    pub(crate) modify_other_keys_state_2: bool,
    pub(crate) kitty_flags: KeyFlags,
    pub(crate) macos_option_as_alt: OptionAsAlt,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            cursor_key_application: false,
            keypad_key_application: false,
            backarrow_key_mode: false,
            ignore_keypad_with_numlock: false,
            alt_esc_prefix: false,
            modify_other_keys_state_2: false,
            kitty_flags: KeyFlags::DISABLED,
            macos_option_as_alt: OptionAsAlt::False,
        }
    }
}

pub(crate) fn encode(event: &KeyEvent, opts: Options) -> Vec<u8> {
    let mut output = String::new();
    if opts.kitty_flags.int() != 0 {
        kitty(&mut output, event, opts);
    } else {
        legacy(&mut output, event, opts);
    }
    output.into_bytes()
}

fn kitty(output: &mut String, event: &KeyEvent, opts: Options) {
    if opts.kitty_flags.is_disabled() {
        legacy(output, event, opts);
        return;
    }

    if event.action == KeyAction::Release {
        if !opts.kitty_flags.report_events {
            return;
        }
        if !opts.kitty_flags.report_all
            && matches!(event.key, Key::Enter | Key::Backspace | Key::Tab)
        {
            return;
        }
    }

    let all_mods = event.mods;
    let binding_mods = event.effective_mods().binding();
    let entry = kitty_entry(event).or_else(|| {
        if event.unshifted_codepoint > 0 {
            Some(KittyEntry {
                key: event.key,
                code: event.unshifted_codepoint,
                final_byte: 'u',
                modifier: false,
            })
        } else {
            None
        }
    });

    if event.composing {
        if !entry.is_some_and(|entry| entry.modifier) {
            return;
        }
    } else if !opts.kitty_flags.report_all {
        if event.utf8.is_empty() && binding_mods.empty() {
            match event.key {
                Key::Enter => {
                    output.push('\r');
                    return;
                }
                Key::Tab => {
                    output.push('\t');
                    return;
                }
                Key::Backspace => {
                    output.push('\u{7f}');
                    return;
                }
                _ => {}
            }
        }

        if !event.utf8.is_empty() && binding_mods.empty() && event.action != KeyAction::Release {
            if event.utf8.iter().all(|byte| !is_control(*byte as u32)) {
                push_utf8(output, &event.utf8);
                return;
            }
        }
    } else if !event.utf8.is_empty()
        && matches!(event.key, Key::Enter | Key::Backspace)
        && !is_control_utf8(&event.utf8)
    {
        if event.key == Key::Enter {
            push_utf8(output, &event.utf8);
        }
        return;
    }

    let Some(entry) = entry else {
        if !event.utf8.is_empty() {
            push_utf8(output, &event.utf8);
        }
        return;
    };
    if entry.modifier && !opts.kitty_flags.report_all {
        return;
    }

    let mut seq = KittySequence {
        key: entry.code,
        final_byte: entry.final_byte,
        mods: KittyMods::from_input(event.action, event.key, all_mods),
        event: KittyEvent::None,
        alternates: [None, None],
        text: Vec::new(),
    };

    if opts.kitty_flags.report_events {
        seq.event = match event.action {
            KeyAction::Press => KittyEvent::Press,
            KeyAction::Repeat => KittyEvent::Repeat,
            KeyAction::Release => KittyEvent::Release,
        };
    }

    if opts.kitty_flags.report_alternates && !is_control(seq.key) {
        let chars = utf8_chars(&event.utf8);
        if let Some(cp1) = chars.first().copied() {
            if cp1 != seq.key && seq.mods.shift {
                seq.alternates[0] = Some(cp1);
            }
            if chars.len() == 1 {
                if let Some(base) = event.key.codepoint() {
                    if base != seq.key && cp1 != base {
                        seq.alternates[1] = Some(base);
                    }
                }
            }
        } else if let Some(base) = event.key.codepoint() {
            if base != seq.key {
                seq.alternates[1] = Some(base);
            }
        }
    }

    if opts.kitty_flags.report_associated && seq.event != KittyEvent::Release {
        let alt_prevents_text = match opts.macos_option_as_alt {
            OptionAsAlt::False => false,
            OptionAsAlt::True => true,
            OptionAsAlt::Left => all_mods.sides.alt == Side::Left,
            OptionAsAlt::Right => all_mods.sides.alt == Side::Right,
        };
        if !seq.mods.prevents_text(alt_prevents_text) {
            seq.text = event.utf8.clone();
        }
    }

    seq.encode(output);
}

fn legacy(output: &mut String, event: &KeyEvent, opts: Options) {
    if event.action != KeyAction::Press && event.action != KeyAction::Repeat {
        return;
    }
    if event.composing {
        return;
    }

    let all_mods = event.mods;
    let binding_mods = event.effective_mods().binding();

    if let Some(sequence) = pc_style_function_key(event.key, all_mods, opts) {
        if !event.utf8.is_empty()
            && matches!(event.key, Key::Backspace | Key::Enter | Key::Escape)
            && !is_control_utf8(&event.utf8)
        {
            if event.key == Key::Backspace {
                return;
            }
        } else {
            output.push_str(sequence);
            return;
        }
    }

    if let Some(byte) = ctrl_seq(event.key, &event.utf8, event.unshifted_codepoint, all_mods) {
        if binding_mods.alt {
            output.push('\x1b');
        }
        output.push(byte as char);
        return;
    }

    if event.utf8.is_empty() {
        if let Some(byte) = legacy_alt_prefix(event, binding_mods, all_mods, opts) {
            output.push('\x1b');
            output.push(byte as char);
        }
        return;
    }

    if opts.modify_other_keys_state_2 {
        let chars = utf8_chars(&event.utf8);
        if chars.len() == 1 {
            let codepoint = chars[0];
            let mut mods = event.mods.binding();
            match opts.macos_option_as_alt {
                OptionAsAlt::False => mods.alt = false,
                OptionAsAlt::True => {}
                OptionAsAlt::Left if event.mods.sides.alt == Side::Right => mods.alt = false,
                OptionAsAlt::Right if event.mods.sides.alt == Side::Left => mods.alt = false,
                OptionAsAlt::Left | OptionAsAlt::Right => {}
            }
            let mut mods_no_shift = mods;
            mods_no_shift.shift = false;
            let should_modify = (0x40..=0x7f).contains(&codepoint)
                || !mods_no_shift.empty()
                || codepoint == ' ' as u32;
            if should_modify {
                if let Some(code) = modifier_code(mods) {
                    output.push_str(&format!("\x1b[27;{code};{codepoint}~"));
                    return;
                }
            }
        }
    }

    if event.mods.ctrl {
        if let Some((mods, cp)) = csi_u_for_ctrl(event) {
            output.push_str(&format!("\x1b[{cp};{}u", csi_u_seq(mods)));
            return;
        }
    }

    if let Some(byte) = legacy_alt_prefix(event, binding_mods, all_mods, opts) {
        output.push('\x1b');
        output.push(byte as char);
        return;
    }

    if all_mods.super_ {
        return;
    }

    push_utf8(output, &event.utf8);
}

#[derive(Clone, Copy)]
struct KittyEntry {
    key: Key,
    code: u32,
    final_byte: char,
    modifier: bool,
}

fn kitty_entry(event: &KeyEvent) -> Option<KittyEntry> {
    let (code, final_byte, modifier) = match event.key {
        Key::Escape => (27, 'u', false),
        Key::Enter => (13, 'u', false),
        Key::Tab => (9, 'u', false),
        Key::Backspace => (127, 'u', false),
        Key::Delete => (3, '~', false),
        Key::ArrowUp => (1, 'A', false),
        Key::ArrowDown => (1, 'B', false),
        Key::ArrowRight => (1, 'C', false),
        Key::ArrowLeft => (1, 'D', false),
        Key::Numpad1 => (57400, 'u', false),
        Key::ShiftLeft => (57441, 'u', true),
        Key::ControlLeft => (57442, 'u', true),
        Key::AltLeft => (57443, 'u', true),
        Key::MetaLeft => (57444, 'u', true),
        Key::ShiftRight => (57447, 'u', true),
        Key::ControlRight => (57448, 'u', true),
        Key::AltRight => (57449, 'u', true),
        Key::MetaRight => (57450, 'u', true),
        _ => return None,
    };
    Some(KittyEntry {
        key: event.key,
        code,
        final_byte,
        modifier,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KittyEvent {
    None,
    Press,
    Repeat,
    Release,
}

impl KittyEvent {
    fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Press => 1,
            Self::Repeat => 2,
            Self::Release => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct KittyMods {
    shift: bool,
    alt: bool,
    ctrl: bool,
    super_: bool,
    caps_lock: bool,
    num_lock: bool,
}

impl KittyMods {
    fn from_input(_action: KeyAction, _key: Key, mods: Mods) -> Self {
        Self {
            shift: mods.shift,
            alt: mods.alt,
            ctrl: mods.ctrl,
            super_: mods.super_,
            caps_lock: mods.caps_lock,
            num_lock: mods.num_lock,
        }
    }

    fn int(self) -> u8 {
        self.shift as u8
            | ((self.alt as u8) << 1)
            | ((self.ctrl as u8) << 2)
            | ((self.super_ as u8) << 3)
            | ((self.caps_lock as u8) << 6)
            | ((self.num_lock as u8) << 7)
    }

    fn seq_int(self) -> u16 {
        self.int() as u16 + 1
    }

    fn prevents_text(self, alt_prevents_text: bool) -> bool {
        (self.alt && alt_prevents_text) || self.ctrl || self.super_
    }
}

struct KittySequence {
    key: u32,
    final_byte: char,
    mods: KittyMods,
    event: KittyEvent,
    alternates: [Option<u32>; 2],
    text: Vec<u8>,
}

impl KittySequence {
    fn encode(self, output: &mut String) {
        if self.final_byte == 'u' || self.final_byte == '~' {
            self.encode_full(output);
        } else {
            self.encode_special(output);
        }
    }

    fn encode_full(self, output: &mut String) {
        output.push_str(&format!("\x1b[{}", self.key));
        if let Some(shifted) = self.alternates[0] {
            output.push_str(&format!(":{shifted}"));
        }
        if let Some(base) = self.alternates[1] {
            if self.alternates[0].is_none() {
                output.push_str("::");
            } else {
                output.push(':');
            }
            output.push_str(&base.to_string());
        }

        let mods = self.mods.seq_int();
        let mut emitted_prior = false;
        if self.event != KittyEvent::None && self.event != KittyEvent::Press {
            output.push_str(&format!(";{}:{}", mods, self.event.code()));
            emitted_prior = true;
        } else if mods > 1 {
            output.push_str(&format!(";{mods}"));
            emitted_prior = true;
        }

        let text_chars = utf8_chars(&self.text);
        let printable: Vec<u32> = text_chars
            .into_iter()
            .filter(|cp| !is_control(*cp))
            .collect();
        if !printable.is_empty() {
            if !emitted_prior {
                output.push(';');
            }
            output.push(';');
            for (idx, cp) in printable.into_iter().enumerate() {
                if idx > 0 {
                    output.push(':');
                }
                output.push_str(&cp.to_string());
            }
        }

        output.push(self.final_byte);
    }

    fn encode_special(self, output: &mut String) {
        let mods = self.mods.seq_int();
        match self.event {
            KittyEvent::None if mods == 1 => {
                output.push_str("\x1b[");
                output.push(self.final_byte);
            }
            KittyEvent::None => {
                output.push_str(&format!("\x1b[1;{mods}{}", self.final_byte));
            }
            event => {
                output.push_str(&format!(
                    "\x1b[1;{}:{}{}",
                    mods,
                    event.code(),
                    self.final_byte
                ));
            }
        }
    }
}

fn pc_style_function_key(key: Key, mods: Mods, opts: Options) -> Option<&'static str> {
    let mods = mods.binding();
    match key {
        Key::Backspace => match (opts.backarrow_key_mode, mods.ctrl) {
            (false, false) => Some("\x7f"),
            (false, true) => Some("\x08"),
            (true, false) => Some("\x08"),
            (true, true) => Some("\x7f"),
        },
        Key::Tab if mods.shift => Some("\x1b[Z"),
        Key::Enter if !mods.shift && !mods.ctrl && !mods.alt && !mods.super_ => Some("\r"),
        Key::Escape if !mods.shift && !mods.ctrl && !mods.alt && !mods.super_ => Some("\x1b"),
        Key::ArrowUp if mods.shift => Some("\x1b[1;2A"),
        Key::ArrowUp if opts.cursor_key_application => Some("\x1bOA"),
        Key::ArrowUp => Some("\x1b[A"),
        Key::Delete if !mods.shift && !mods.ctrl && !mods.alt && !mods.super_ => Some("\x1b[3~"),
        Key::F1 if mods.shift => Some("\x1b[1;2P"),
        Key::F1 if mods.ctrl => Some("\x1b[1;5P"),
        Key::F1 if !mods.shift && !mods.ctrl && !mods.alt && !mods.super_ => Some("\x1bOP"),
        Key::F2 if mods.ctrl => Some("\x1b[1;5Q"),
        Key::NumpadEnter => Some("\r"),
        Key::Numpad1 if opts.keypad_key_application && !opts.ignore_keypad_with_numlock => {
            Some("\x1bOq")
        }
        _ => None,
    }
}

fn ctrl_seq(key: Key, utf8: &[u8], unshifted_codepoint: u32, mods: Mods) -> Option<u8> {
    if !mods.ctrl {
        return None;
    }
    let mut unset_mods = mods.binding();
    unset_mods.alt = false;

    let char_byte = if utf8.len() == 1 {
        utf8[0]
    } else if let Some(cp) = key.codepoint() {
        if cp <= u8::MAX as u32 {
            if unset_mods.int()
                == (Mods {
                    ctrl: true,
                    ..Mods::new()
                })
                .int()
            {
                cp as u8
            } else {
                return None;
            }
        } else {
            return None;
        }
    } else if unshifted_codepoint <= u8::MAX as u32 {
        unshifted_codepoint as u8
    } else {
        return None;
    };

    match char_byte {
        b'a'..=b'z'
            if unset_mods.int()
                == Mods {
                    ctrl: true,
                    ..Mods::new()
                }
                .int() =>
        {
            Some(char_byte - b'a' + 1)
        }
        b'A'..=b'Z'
            if unset_mods.int()
                == Mods {
                    ctrl: true,
                    ..Mods::new()
                }
                .int() =>
        {
            Some(char_byte - b'A' + 1)
        }
        b' ' if unset_mods.int()
            == Mods {
                ctrl: true,
                ..Mods::new()
            }
            .int() =>
        {
            Some(0)
        }
        b'_' if mods.shift && mods.ctrl => Some(0x1f),
        _ => None,
    }
}

fn csi_u_for_ctrl(event: &KeyEvent) -> Option<(Mods, u32)> {
    let chars = utf8_chars(&event.utf8);
    if chars.len() != 1 {
        return None;
    }
    let mut char = chars[0];
    let mut mods = event.mods;
    if (b'A' as u32..=b'Z' as u32).contains(&char) && mods.shift {
        char = char.to_ascii_lowercase();
    }
    if event.unshifted_codepoint != 0 && event.unshifted_codepoint != char {
        mods.shift = false;
    }
    Some((mods, char))
}

fn csi_u_seq(mods: Mods) -> u8 {
    1 + mods.shift as u8 + ((mods.alt as u8) << 1) + ((mods.ctrl as u8) << 2)
}

fn modifier_code(mods: Mods) -> Option<u8> {
    let mods = mods.binding();
    Some(match (mods.shift, mods.alt, mods.ctrl, mods.super_) {
        (true, false, false, false) => 2,
        (false, true, false, false) => 3,
        (true, true, false, false) => 4,
        (false, false, true, false) => 5,
        (true, false, true, false) => 6,
        (false, true, true, false) => 7,
        (true, true, true, false) => 8,
        (false, false, false, true) => 9,
        (true, false, false, true) => 10,
        (false, true, false, true) => 11,
        (true, true, false, true) => 12,
        (false, false, true, true) => 13,
        (true, false, true, true) => 14,
        (false, true, true, true) => 15,
        (true, true, true, true) => 16,
        _ => return None,
    })
}

fn legacy_alt_prefix(
    event: &KeyEvent,
    binding_mods: Mods,
    mods: Mods,
    opts: Options,
) -> Option<u8> {
    if !binding_mods.alt || !opts.alt_esc_prefix {
        return None;
    }
    match opts.macos_option_as_alt {
        OptionAsAlt::False => return None,
        OptionAsAlt::Left if mods.sides.alt == Side::Right => return None,
        OptionAsAlt::Right if mods.sides.alt == Side::Left => return None,
        OptionAsAlt::True | OptionAsAlt::Left | OptionAsAlt::Right => {}
    }

    if event.utf8.len() == 1 {
        return Some(event.utf8[0]);
    }
    if event.unshifted_codepoint > 0 && event.unshifted_codepoint <= u8::MAX as u32 {
        return Some(event.unshifted_codepoint as u8);
    }
    None
}

fn utf8_chars(bytes: &[u8]) -> Vec<u32> {
    std::str::from_utf8(bytes)
        .map(|text| text.chars().map(|ch| ch as u32).collect())
        .unwrap_or_default()
}

fn push_utf8(output: &mut String, bytes: &[u8]) {
    output.push_str(std::str::from_utf8(bytes).unwrap_or_default());
}

fn is_control_utf8(bytes: &[u8]) -> bool {
    let chars = utf8_chars(bytes);
    !chars.is_empty() && chars.iter().all(|cp| is_control(*cp))
}

fn is_control(cp: u32) -> bool {
    cp < 0x20 || cp == 0x7f
}

trait AsciiLower {
    fn to_ascii_lowercase(self) -> Self;
}

impl AsciiLower for u32 {
    fn to_ascii_lowercase(self) -> Self {
        if (b'A' as u32..=b'Z' as u32).contains(&self) {
            self + 32
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::key_mods::ModSides;

    fn event(key: Key) -> KeyEvent {
        KeyEvent {
            key,
            ..KeyEvent::default()
        }
    }

    fn text_event(key: Key, text: &str) -> KeyEvent {
        KeyEvent {
            key,
            utf8: text.as_bytes().to_vec(),
            ..KeyEvent::default()
        }
    }

    fn encoded(event: KeyEvent, opts: Options) -> String {
        String::from_utf8(encode(&event, opts)).unwrap()
    }

    fn kitty_flags() -> KeyFlags {
        KeyFlags {
            disambiguate: true,
            ..KeyFlags::DISABLED
        }
    }

    fn kitty_all() -> KeyFlags {
        KeyFlags {
            disambiguate: true,
            report_events: true,
            report_alternates: true,
            report_all: true,
            report_associated: true,
        }
    }

    #[test]
    fn key_encode_options_default_to_upstream_values() {
        assert_eq!(Options::default().kitty_flags, KeyFlags::DISABLED);
        assert!(!Options::default().cursor_key_application);
        assert!(!Options::default().keypad_key_application);
        assert!(!Options::default().backarrow_key_mode);
        assert!(!Options::default().ignore_keypad_with_numlock);
        assert!(!Options::default().alt_esc_prefix);
        assert!(!Options::default().modify_other_keys_state_2);
        assert_eq!(Options::default().macos_option_as_alt, OptionAsAlt::False);
    }

    #[test]
    fn key_encode_kitty_plain_text_and_repeat_with_disambiguate() {
        let opts = Options {
            kitty_flags: kitty_flags(),
            ..Options::default()
        };
        assert_eq!(encoded(text_event(Key::KeyA, "abcd"), opts), "abcd");

        let repeat = KeyEvent {
            action: KeyAction::Repeat,
            key: Key::KeyA,
            utf8: b"a".to_vec(),
            ..KeyEvent::default()
        };
        assert_eq!(encoded(repeat, opts), "a");
    }

    #[test]
    fn key_encode_kitty_enter_backspace_tab_report_all_off_and_on() {
        let opts = Options {
            kitty_flags: kitty_flags(),
            ..Options::default()
        };
        assert_eq!(encoded(event(Key::Enter), opts), "\r");
        assert_eq!(encoded(event(Key::Backspace), opts), "\u{7f}");
        assert_eq!(encoded(event(Key::Tab), opts), "\t");

        let release_enter = KeyEvent {
            action: KeyAction::Release,
            key: Key::Enter,
            ..KeyEvent::default()
        };
        let release_backspace = KeyEvent {
            action: KeyAction::Release,
            key: Key::Backspace,
            ..KeyEvent::default()
        };
        let release_tab = KeyEvent {
            action: KeyAction::Release,
            key: Key::Tab,
            ..KeyEvent::default()
        };
        assert_eq!(
            encoded(
                release_enter.clone(),
                Options {
                    kitty_flags: KeyFlags {
                        disambiguate: true,
                        report_events: true,
                        ..KeyFlags::DISABLED
                    },
                    ..Options::default()
                }
            ),
            ""
        );
        assert_eq!(
            encoded(
                release_enter,
                Options {
                    kitty_flags: kitty_all(),
                    ..Options::default()
                }
            ),
            "\x1b[13;1:3u"
        );
        assert_eq!(
            encoded(
                release_backspace,
                Options {
                    kitty_flags: kitty_all(),
                    ..Options::default()
                }
            ),
            "\x1b[127;1:3u"
        );
        assert_eq!(
            encoded(
                release_tab,
                Options {
                    kitty_flags: kitty_all(),
                    ..Options::default()
                }
            ),
            "\x1b[9;1:3u"
        );
    }

    #[test]
    fn key_encode_kitty_shift_specials_delete_arrow_composing_and_keypad() {
        let opts = Options {
            kitty_flags: kitty_flags(),
            ..Options::default()
        };
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::Backspace,
                    mods: Mods {
                        shift: true,
                        ..Mods::new()
                    },
                    ..KeyEvent::default()
                },
                opts
            ),
            "\x1b[127;2u"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::Enter,
                    mods: Mods {
                        shift: true,
                        ..Mods::new()
                    },
                    ..KeyEvent::default()
                },
                opts
            ),
            "\x1b[13;2u"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::Tab,
                    mods: Mods {
                        shift: true,
                        ..Mods::new()
                    },
                    ..KeyEvent::default()
                },
                opts
            ),
            "\x1b[9;2u"
        );
        assert_eq!(encoded(text_event(Key::Delete, "\u{7f}"), opts), "\x1b[3~");
        assert_eq!(encoded(event(Key::ArrowUp), opts), "\x1b[A");
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyA,
                    mods: Mods {
                        shift: true,
                        ..Mods::new()
                    },
                    composing: true,
                    ..KeyEvent::default()
                },
                opts
            ),
            ""
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::ShiftLeft,
                    mods: Mods {
                        shift: true,
                        ..Mods::new()
                    },
                    composing: true,
                    ..KeyEvent::default()
                },
                Options {
                    kitty_flags: KeyFlags {
                        disambiguate: true,
                        report_all: true,
                        ..KeyFlags::DISABLED
                    },
                    ..Options::default()
                }
            ),
            "\x1b[57441;2u"
        );
        assert_eq!(
            encoded(
                text_event(Key::Numpad1, "1"),
                Options {
                    kitty_flags: kitty_all(),
                    ..Options::default()
                }
            ),
            "\x1b[57400;;49u"
        );
    }

    #[test]
    fn key_encode_kitty_alternates_and_associated_text() {
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyA,
                    mods: Mods {
                        shift: true,
                        ..Mods::new()
                    },
                    utf8: b"A".to_vec(),
                    unshifted_codepoint: 'a' as u32,
                    ..KeyEvent::default()
                },
                Options {
                    kitty_flags: KeyFlags {
                        disambiguate: true,
                        report_alternates: true,
                        ..KeyFlags::DISABLED
                    },
                    ..Options::default()
                }
            ),
            "\x1b[97:65;2u"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::Semicolon,
                    utf8: "ч".as_bytes().to_vec(),
                    unshifted_codepoint: 1095,
                    ..KeyEvent::default()
                },
                Options {
                    kitty_flags: kitty_all(),
                    ..Options::default()
                }
            ),
            "\x1b[1095::59;;1095u"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyW,
                    mods: Mods {
                        alt: true,
                        ..Mods::new()
                    },
                    utf8: "∑".as_bytes().to_vec(),
                    unshifted_codepoint: 'w' as u32,
                    ..KeyEvent::default()
                },
                Options {
                    kitty_flags: kitty_all(),
                    macos_option_as_alt: OptionAsAlt::False,
                    ..Options::default()
                }
            ),
            "\x1b[119;3;8721u"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyW,
                    mods: Mods {
                        alt: true,
                        ..Mods::new()
                    },
                    utf8: "∑".as_bytes().to_vec(),
                    unshifted_codepoint: 'w' as u32,
                    ..KeyEvent::default()
                },
                Options {
                    kitty_flags: kitty_all(),
                    macos_option_as_alt: OptionAsAlt::True,
                    ..Options::default()
                }
            ),
            "\x1b[119;3u"
        );
    }

    #[test]
    fn key_encode_legacy_control_alt_and_option_as_alt() {
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyC,
                    mods: Mods {
                        ctrl: true,
                        ..Mods::new()
                    },
                    utf8: b"c".to_vec(),
                    ..KeyEvent::default()
                },
                Options::default()
            ),
            "\u{3}"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyC,
                    mods: Mods {
                        alt: true,
                        ..Mods::new()
                    },
                    utf8: b"c".to_vec(),
                    ..KeyEvent::default()
                },
                Options {
                    alt_esc_prefix: true,
                    macos_option_as_alt: OptionAsAlt::True,
                    ..Options::default()
                }
            ),
            "\x1bc"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::Digit8,
                    mods: Mods {
                        alt: true,
                        ..Mods::new()
                    },
                    consumed_mods: Mods {
                        alt: true,
                        ..Mods::new()
                    },
                    utf8: b"[".to_vec(),
                    ..KeyEvent::default()
                },
                Options {
                    alt_esc_prefix: true,
                    macos_option_as_alt: OptionAsAlt::False,
                    ..Options::default()
                }
            ),
            "["
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyC,
                    mods: Mods {
                        alt: true,
                        sides: ModSides {
                            alt: Side::Right,
                            ..ModSides::default()
                        },
                        ..Mods::new()
                    },
                    utf8: b"c".to_vec(),
                    ..KeyEvent::default()
                },
                Options {
                    alt_esc_prefix: true,
                    macos_option_as_alt: OptionAsAlt::Left,
                    ..Options::default()
                }
            ),
            "c"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyC,
                    mods: Mods {
                        alt: true,
                        sides: ModSides {
                            alt: Side::Left,
                            ..ModSides::default()
                        },
                        ..Mods::new()
                    },
                    utf8: b"c".to_vec(),
                    ..KeyEvent::default()
                },
                Options {
                    alt_esc_prefix: true,
                    macos_option_as_alt: OptionAsAlt::Left,
                    ..Options::default()
                }
            ),
            "\x1bc"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::Space,
                    mods: Mods {
                        ctrl: true,
                        ..Mods::new()
                    },
                    utf8: b" ".to_vec(),
                    ..KeyEvent::default()
                },
                Options::default()
            ),
            "\0"
        );
    }

    #[test]
    fn key_encode_legacy_backspace_modify_other_and_function_keys() {
        assert_eq!(encoded(event(Key::Backspace), Options::default()), "\u{7f}");
        assert_eq!(
            encoded(
                event(Key::Backspace),
                Options {
                    backarrow_key_mode: true,
                    ..Options::default()
                }
            ),
            "\u{8}"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyH,
                    mods: Mods {
                        ctrl: true,
                        shift: true,
                        ..Mods::new()
                    },
                    utf8: b"H".to_vec(),
                    ..KeyEvent::default()
                },
                Options {
                    modify_other_keys_state_2: true,
                    ..Options::default()
                }
            ),
            "\x1b[27;6;72~"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyH,
                    mods: Mods {
                        ctrl: true,
                        shift: true,
                        ..Mods::new()
                    },
                    consumed_mods: Mods {
                        shift: true,
                        ..Mods::new()
                    },
                    utf8: b"H".to_vec(),
                    ..KeyEvent::default()
                },
                Options {
                    modify_other_keys_state_2: true,
                    ..Options::default()
                }
            ),
            "\x1b[27;6;72~"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::F1,
                    ..KeyEvent::default()
                },
                Options::default()
            ),
            "\x1bOP"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::F1,
                    mods: Mods {
                        ctrl: true,
                        ..Mods::new()
                    },
                    ..KeyEvent::default()
                },
                Options::default()
            ),
            "\x1b[1;5P"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::F1,
                    mods: Mods {
                        shift: true,
                        ..Mods::new()
                    },
                    consumed_mods: Mods {
                        shift: true,
                        ..Mods::new()
                    },
                    ..KeyEvent::default()
                },
                Options::default()
            ),
            "\x1b[1;2P"
        );
        assert_eq!(encoded(event(Key::ArrowUp), Options::default()), "\x1b[A");
        assert_eq!(
            encoded(
                event(Key::ArrowUp),
                Options {
                    cursor_key_application: true,
                    ..Options::default()
                }
            ),
            "\x1bOA"
        );
    }

    #[test]
    fn key_encode_legacy_keypad_and_super_text() {
        assert_eq!(encoded(event(Key::NumpadEnter), Options::default()), "\r");
        assert_eq!(
            encoded(text_event(Key::Numpad1, "1"), Options::default()),
            "1"
        );
        assert_eq!(
            encoded(
                text_event(Key::Numpad1, "1"),
                Options {
                    keypad_key_application: true,
                    ..Options::default()
                }
            ),
            "\x1bOq"
        );
        assert_eq!(
            encoded(
                text_event(Key::Numpad1, "1"),
                Options {
                    keypad_key_application: true,
                    ignore_keypad_with_numlock: true,
                    ..Options::default()
                }
            ),
            "1"
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyB,
                    mods: Mods {
                        super_: true,
                        ..Mods::new()
                    },
                    utf8: b"b".to_vec(),
                    ..KeyEvent::default()
                },
                Options::default()
            ),
            ""
        );
        assert_eq!(
            encoded(
                KeyEvent {
                    key: Key::KeyB,
                    mods: Mods {
                        super_: true,
                        shift: true,
                        ..Mods::new()
                    },
                    utf8: b"B".to_vec(),
                    ..KeyEvent::default()
                },
                Options::default()
            ),
            ""
        );
    }
}
