use super::key_mods::Mods;
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum KeyAction {
    Release = 0,
    Press = 1,
    Repeat = 2,
}
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Key {
    Unidentified = 0,
    Backquote = 1,
    Backslash = 2,
    BracketLeft = 3,
    BracketRight = 4,
    Comma = 5,
    Digit0 = 6,
    Digit1 = 7,
    Digit2 = 8,
    Digit3 = 9,
    Digit4 = 10,
    Digit5 = 11,
    Digit6 = 12,
    Digit7 = 13,
    Digit8 = 14,
    Digit9 = 15,
    Equal = 16,
    IntlBackslash = 17,
    IntlRo = 18,
    IntlYen = 19,
    KeyA = 20,
    KeyB = 21,
    KeyC = 22,
    KeyD = 23,
    KeyE = 24,
    KeyF = 25,
    KeyG = 26,
    KeyH = 27,
    KeyI = 28,
    KeyJ = 29,
    KeyK = 30,
    KeyL = 31,
    KeyM = 32,
    KeyN = 33,
    KeyO = 34,
    KeyP = 35,
    KeyQ = 36,
    KeyR = 37,
    KeyS = 38,
    KeyT = 39,
    KeyU = 40,
    KeyV = 41,
    KeyW = 42,
    KeyX = 43,
    KeyY = 44,
    KeyZ = 45,
    Minus = 46,
    Period = 47,
    Quote = 48,
    Semicolon = 49,
    Slash = 50,
    AltLeft = 51,
    AltRight = 52,
    Backspace = 53,
    CapsLock = 54,
    ContextMenu = 55,
    ControlLeft = 56,
    ControlRight = 57,
    Enter = 58,
    MetaLeft = 59,
    MetaRight = 60,
    ShiftLeft = 61,
    ShiftRight = 62,
    Space = 63,
    Tab = 64,
    Convert = 65,
    KanaMode = 66,
    NonConvert = 67,
    Delete = 68,
    End = 69,
    Help = 70,
    Home = 71,
    Insert = 72,
    PageDown = 73,
    PageUp = 74,
    ArrowDown = 75,
    ArrowLeft = 76,
    ArrowRight = 77,
    ArrowUp = 78,
    NumLock = 79,
    Numpad0 = 80,
    Numpad1 = 81,
    Numpad2 = 82,
    Numpad3 = 83,
    Numpad4 = 84,
    Numpad5 = 85,
    Numpad6 = 86,
    Numpad7 = 87,
    Numpad8 = 88,
    Numpad9 = 89,
    NumpadAdd = 90,
    NumpadBackspace = 91,
    NumpadClear = 92,
    NumpadClearEntry = 93,
    NumpadComma = 94,
    NumpadDecimal = 95,
    NumpadDivide = 96,
    NumpadEnter = 97,
    NumpadEqual = 98,
    NumpadMemoryAdd = 99,
    NumpadMemoryClear = 100,
    NumpadMemoryRecall = 101,
    NumpadMemoryStore = 102,
    NumpadMemorySubtract = 103,
    NumpadMultiply = 104,
    NumpadParenLeft = 105,
    NumpadParenRight = 106,
    NumpadSubtract = 107,
    NumpadSeparator = 108,
    NumpadUp = 109,
    NumpadDown = 110,
    NumpadRight = 111,
    NumpadLeft = 112,
    NumpadBegin = 113,
    NumpadHome = 114,
    NumpadEnd = 115,
    NumpadInsert = 116,
    NumpadDelete = 117,
    NumpadPageUp = 118,
    NumpadPageDown = 119,
    Escape = 120,
    F1 = 121,
    F2 = 122,
    F3 = 123,
    F4 = 124,
    F5 = 125,
    F6 = 126,
    F7 = 127,
    F8 = 128,
    F9 = 129,
    F10 = 130,
    F11 = 131,
    F12 = 132,
    F13 = 133,
    F14 = 134,
    F15 = 135,
    F16 = 136,
    F17 = 137,
    F18 = 138,
    F19 = 139,
    F20 = 140,
    F21 = 141,
    F22 = 142,
    F23 = 143,
    F24 = 144,
    F25 = 145,
    Fn = 146,
    FnLock = 147,
    PrintScreen = 148,
    ScrollLock = 149,
    Pause = 150,
    BrowserBack = 151,
    BrowserFavorites = 152,
    BrowserForward = 153,
    BrowserHome = 154,
    BrowserRefresh = 155,
    BrowserSearch = 156,
    BrowserStop = 157,
    Eject = 158,
    LaunchApp1 = 159,
    LaunchApp2 = 160,
    LaunchMail = 161,
    MediaPlayPause = 162,
    MediaSelect = 163,
    MediaStop = 164,
    MediaTrackNext = 165,
    MediaTrackPrevious = 166,
    Power = 167,
    Sleep = 168,
    AudioVolumeDown = 169,
    AudioVolumeMute = 170,
    AudioVolumeUp = 171,
    WakeUp = 172,
    Copy = 173,
    Cut = 174,
    Paste = 175,
}
pub(crate) const KEY_COUNT: usize = 176;
pub(crate) const ALL_KEYS: [Key; KEY_COUNT] = [
    Key::Unidentified,
    Key::Backquote,
    Key::Backslash,
    Key::BracketLeft,
    Key::BracketRight,
    Key::Comma,
    Key::Digit0,
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
    Key::Digit7,
    Key::Digit8,
    Key::Digit9,
    Key::Equal,
    Key::IntlBackslash,
    Key::IntlRo,
    Key::IntlYen,
    Key::KeyA,
    Key::KeyB,
    Key::KeyC,
    Key::KeyD,
    Key::KeyE,
    Key::KeyF,
    Key::KeyG,
    Key::KeyH,
    Key::KeyI,
    Key::KeyJ,
    Key::KeyK,
    Key::KeyL,
    Key::KeyM,
    Key::KeyN,
    Key::KeyO,
    Key::KeyP,
    Key::KeyQ,
    Key::KeyR,
    Key::KeyS,
    Key::KeyT,
    Key::KeyU,
    Key::KeyV,
    Key::KeyW,
    Key::KeyX,
    Key::KeyY,
    Key::KeyZ,
    Key::Minus,
    Key::Period,
    Key::Quote,
    Key::Semicolon,
    Key::Slash,
    Key::AltLeft,
    Key::AltRight,
    Key::Backspace,
    Key::CapsLock,
    Key::ContextMenu,
    Key::ControlLeft,
    Key::ControlRight,
    Key::Enter,
    Key::MetaLeft,
    Key::MetaRight,
    Key::ShiftLeft,
    Key::ShiftRight,
    Key::Space,
    Key::Tab,
    Key::Convert,
    Key::KanaMode,
    Key::NonConvert,
    Key::Delete,
    Key::End,
    Key::Help,
    Key::Home,
    Key::Insert,
    Key::PageDown,
    Key::PageUp,
    Key::ArrowDown,
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::ArrowUp,
    Key::NumLock,
    Key::Numpad0,
    Key::Numpad1,
    Key::Numpad2,
    Key::Numpad3,
    Key::Numpad4,
    Key::Numpad5,
    Key::Numpad6,
    Key::Numpad7,
    Key::Numpad8,
    Key::Numpad9,
    Key::NumpadAdd,
    Key::NumpadBackspace,
    Key::NumpadClear,
    Key::NumpadClearEntry,
    Key::NumpadComma,
    Key::NumpadDecimal,
    Key::NumpadDivide,
    Key::NumpadEnter,
    Key::NumpadEqual,
    Key::NumpadMemoryAdd,
    Key::NumpadMemoryClear,
    Key::NumpadMemoryRecall,
    Key::NumpadMemoryStore,
    Key::NumpadMemorySubtract,
    Key::NumpadMultiply,
    Key::NumpadParenLeft,
    Key::NumpadParenRight,
    Key::NumpadSubtract,
    Key::NumpadSeparator,
    Key::NumpadUp,
    Key::NumpadDown,
    Key::NumpadRight,
    Key::NumpadLeft,
    Key::NumpadBegin,
    Key::NumpadHome,
    Key::NumpadEnd,
    Key::NumpadInsert,
    Key::NumpadDelete,
    Key::NumpadPageUp,
    Key::NumpadPageDown,
    Key::Escape,
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
    Key::F13,
    Key::F14,
    Key::F15,
    Key::F16,
    Key::F17,
    Key::F18,
    Key::F19,
    Key::F20,
    Key::F21,
    Key::F22,
    Key::F23,
    Key::F24,
    Key::F25,
    Key::Fn,
    Key::FnLock,
    Key::PrintScreen,
    Key::ScrollLock,
    Key::Pause,
    Key::BrowserBack,
    Key::BrowserFavorites,
    Key::BrowserForward,
    Key::BrowserHome,
    Key::BrowserRefresh,
    Key::BrowserSearch,
    Key::BrowserStop,
    Key::Eject,
    Key::LaunchApp1,
    Key::LaunchApp2,
    Key::LaunchMail,
    Key::MediaPlayPause,
    Key::MediaSelect,
    Key::MediaStop,
    Key::MediaTrackNext,
    Key::MediaTrackPrevious,
    Key::Power,
    Key::Sleep,
    Key::AudioVolumeDown,
    Key::AudioVolumeMute,
    Key::AudioVolumeUp,
    Key::WakeUp,
    Key::Copy,
    Key::Cut,
    Key::Paste,
];
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyEvent {
    pub(crate) action: KeyAction,
    pub(crate) key: Key,
    pub(crate) mods: Mods,
    pub(crate) consumed_mods: Mods,
    pub(crate) composing: bool,
    pub(crate) utf8: Vec<u8>,
    pub(crate) unshifted_codepoint: u32,
}
impl Default for KeyEvent {
    fn default() -> Self {
        Self {
            action: KeyAction::Press,
            key: Key::Unidentified,
            mods: Mods::new(),
            consumed_mods: Mods::new(),
            composing: false,
            utf8: Vec::new(),
            unshifted_codepoint: 0,
        }
    }
}
impl KeyEvent {
    pub(crate) fn effective_mods(&self) -> Mods {
        if self.utf8.is_empty() {
            self.mods
        } else {
            self.mods.unset(self.consumed_mods)
        }
    }

    pub(crate) fn binding_hash(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        hash = fnv1a(hash, self.key as i32 as u64);
        hash = fnv1a(hash, self.unshifted_codepoint as u64);
        hash = fnv1a(hash, self.mods.binding().int() as u64);
        hash
    }
}

fn fnv1a(mut hash: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
impl Key {
    pub(crate) fn from_ascii(ch: u8) -> Option<Self> {
        CODEPOINT_MAP.iter().find_map(|(cp, key)| {
            if *cp == ch as u32 && !key.keypad() {
                Some(*key)
            } else {
                None
            }
        })
    }

    pub(crate) fn codepoint(self) -> Option<u32> {
        CODEPOINT_MAP
            .iter()
            .find_map(|(cp, key)| if *key == self { Some(*cp) } else { None })
    }

    pub(crate) fn keypad(self) -> bool {
        self.w3c().starts_with("Numpad")
    }

    pub(crate) fn w3c(self) -> &'static str {
        KEY_INFOS[self as usize].w3c
    }

    pub(crate) fn snake(self) -> &'static str {
        KEY_INFOS[self as usize].snake
    }

    pub(crate) fn from_w3c(code: &str) -> Option<Self> {
        KEY_INFOS
            .iter()
            .find_map(|info| {
                if info.w3c == code || info.snake == code {
                    Some(info.key)
                } else {
                    None
                }
            })
            .or_else(|| {
                let normalized = normalize_w3c_to_snake(code)?;
                KEY_INFOS.iter().find_map(|info| {
                    if info.snake == normalized {
                        Some(info.key)
                    } else {
                        None
                    }
                })
            })
    }

    pub(crate) fn ctrl_or_super(self) -> bool {
        matches!(self, Self::MetaLeft | Self::MetaRight)
    }

    pub(crate) fn left_or_right_shift(self) -> bool {
        matches!(self, Self::ShiftLeft | Self::ShiftRight)
    }

    pub(crate) fn left_or_right_alt(self) -> bool {
        matches!(self, Self::AltLeft | Self::AltRight)
    }
}

fn normalize_w3c_to_snake(code: &str) -> Option<String> {
    let mut normalized = String::with_capacity(code.len() + 8);
    for (index, ch) in code.chars().enumerate() {
        match ch {
            'a'..='z' => normalized.push(ch),
            'A'..='Z' => {
                if index > 0 {
                    normalized.push('_');
                }
                normalized.push(ch.to_ascii_lowercase());
            }
            '0'..='9' => {
                if index > 0 {
                    normalized.push('_');
                }
                normalized.push(ch);
            }
            _ => return None,
        }
    }
    Some(normalized)
}

struct KeyInfo {
    key: Key,
    snake: &'static str,
    w3c: &'static str,
}
const KEY_INFOS: [KeyInfo; KEY_COUNT] = [
    KeyInfo {
        key: Key::Unidentified,
        snake: "unidentified",
        w3c: "Unidentified",
    },
    KeyInfo {
        key: Key::Backquote,
        snake: "backquote",
        w3c: "Backquote",
    },
    KeyInfo {
        key: Key::Backslash,
        snake: "backslash",
        w3c: "Backslash",
    },
    KeyInfo {
        key: Key::BracketLeft,
        snake: "bracket_left",
        w3c: "BracketLeft",
    },
    KeyInfo {
        key: Key::BracketRight,
        snake: "bracket_right",
        w3c: "BracketRight",
    },
    KeyInfo {
        key: Key::Comma,
        snake: "comma",
        w3c: "Comma",
    },
    KeyInfo {
        key: Key::Digit0,
        snake: "digit_0",
        w3c: "Digit0",
    },
    KeyInfo {
        key: Key::Digit1,
        snake: "digit_1",
        w3c: "Digit1",
    },
    KeyInfo {
        key: Key::Digit2,
        snake: "digit_2",
        w3c: "Digit2",
    },
    KeyInfo {
        key: Key::Digit3,
        snake: "digit_3",
        w3c: "Digit3",
    },
    KeyInfo {
        key: Key::Digit4,
        snake: "digit_4",
        w3c: "Digit4",
    },
    KeyInfo {
        key: Key::Digit5,
        snake: "digit_5",
        w3c: "Digit5",
    },
    KeyInfo {
        key: Key::Digit6,
        snake: "digit_6",
        w3c: "Digit6",
    },
    KeyInfo {
        key: Key::Digit7,
        snake: "digit_7",
        w3c: "Digit7",
    },
    KeyInfo {
        key: Key::Digit8,
        snake: "digit_8",
        w3c: "Digit8",
    },
    KeyInfo {
        key: Key::Digit9,
        snake: "digit_9",
        w3c: "Digit9",
    },
    KeyInfo {
        key: Key::Equal,
        snake: "equal",
        w3c: "Equal",
    },
    KeyInfo {
        key: Key::IntlBackslash,
        snake: "intl_backslash",
        w3c: "IntlBackslash",
    },
    KeyInfo {
        key: Key::IntlRo,
        snake: "intl_ro",
        w3c: "IntlRo",
    },
    KeyInfo {
        key: Key::IntlYen,
        snake: "intl_yen",
        w3c: "IntlYen",
    },
    KeyInfo {
        key: Key::KeyA,
        snake: "key_a",
        w3c: "KeyA",
    },
    KeyInfo {
        key: Key::KeyB,
        snake: "key_b",
        w3c: "KeyB",
    },
    KeyInfo {
        key: Key::KeyC,
        snake: "key_c",
        w3c: "KeyC",
    },
    KeyInfo {
        key: Key::KeyD,
        snake: "key_d",
        w3c: "KeyD",
    },
    KeyInfo {
        key: Key::KeyE,
        snake: "key_e",
        w3c: "KeyE",
    },
    KeyInfo {
        key: Key::KeyF,
        snake: "key_f",
        w3c: "KeyF",
    },
    KeyInfo {
        key: Key::KeyG,
        snake: "key_g",
        w3c: "KeyG",
    },
    KeyInfo {
        key: Key::KeyH,
        snake: "key_h",
        w3c: "KeyH",
    },
    KeyInfo {
        key: Key::KeyI,
        snake: "key_i",
        w3c: "KeyI",
    },
    KeyInfo {
        key: Key::KeyJ,
        snake: "key_j",
        w3c: "KeyJ",
    },
    KeyInfo {
        key: Key::KeyK,
        snake: "key_k",
        w3c: "KeyK",
    },
    KeyInfo {
        key: Key::KeyL,
        snake: "key_l",
        w3c: "KeyL",
    },
    KeyInfo {
        key: Key::KeyM,
        snake: "key_m",
        w3c: "KeyM",
    },
    KeyInfo {
        key: Key::KeyN,
        snake: "key_n",
        w3c: "KeyN",
    },
    KeyInfo {
        key: Key::KeyO,
        snake: "key_o",
        w3c: "KeyO",
    },
    KeyInfo {
        key: Key::KeyP,
        snake: "key_p",
        w3c: "KeyP",
    },
    KeyInfo {
        key: Key::KeyQ,
        snake: "key_q",
        w3c: "KeyQ",
    },
    KeyInfo {
        key: Key::KeyR,
        snake: "key_r",
        w3c: "KeyR",
    },
    KeyInfo {
        key: Key::KeyS,
        snake: "key_s",
        w3c: "KeyS",
    },
    KeyInfo {
        key: Key::KeyT,
        snake: "key_t",
        w3c: "KeyT",
    },
    KeyInfo {
        key: Key::KeyU,
        snake: "key_u",
        w3c: "KeyU",
    },
    KeyInfo {
        key: Key::KeyV,
        snake: "key_v",
        w3c: "KeyV",
    },
    KeyInfo {
        key: Key::KeyW,
        snake: "key_w",
        w3c: "KeyW",
    },
    KeyInfo {
        key: Key::KeyX,
        snake: "key_x",
        w3c: "KeyX",
    },
    KeyInfo {
        key: Key::KeyY,
        snake: "key_y",
        w3c: "KeyY",
    },
    KeyInfo {
        key: Key::KeyZ,
        snake: "key_z",
        w3c: "KeyZ",
    },
    KeyInfo {
        key: Key::Minus,
        snake: "minus",
        w3c: "Minus",
    },
    KeyInfo {
        key: Key::Period,
        snake: "period",
        w3c: "Period",
    },
    KeyInfo {
        key: Key::Quote,
        snake: "quote",
        w3c: "Quote",
    },
    KeyInfo {
        key: Key::Semicolon,
        snake: "semicolon",
        w3c: "Semicolon",
    },
    KeyInfo {
        key: Key::Slash,
        snake: "slash",
        w3c: "Slash",
    },
    KeyInfo {
        key: Key::AltLeft,
        snake: "alt_left",
        w3c: "AltLeft",
    },
    KeyInfo {
        key: Key::AltRight,
        snake: "alt_right",
        w3c: "AltRight",
    },
    KeyInfo {
        key: Key::Backspace,
        snake: "backspace",
        w3c: "Backspace",
    },
    KeyInfo {
        key: Key::CapsLock,
        snake: "caps_lock",
        w3c: "CapsLock",
    },
    KeyInfo {
        key: Key::ContextMenu,
        snake: "context_menu",
        w3c: "ContextMenu",
    },
    KeyInfo {
        key: Key::ControlLeft,
        snake: "control_left",
        w3c: "ControlLeft",
    },
    KeyInfo {
        key: Key::ControlRight,
        snake: "control_right",
        w3c: "ControlRight",
    },
    KeyInfo {
        key: Key::Enter,
        snake: "enter",
        w3c: "Enter",
    },
    KeyInfo {
        key: Key::MetaLeft,
        snake: "meta_left",
        w3c: "MetaLeft",
    },
    KeyInfo {
        key: Key::MetaRight,
        snake: "meta_right",
        w3c: "MetaRight",
    },
    KeyInfo {
        key: Key::ShiftLeft,
        snake: "shift_left",
        w3c: "ShiftLeft",
    },
    KeyInfo {
        key: Key::ShiftRight,
        snake: "shift_right",
        w3c: "ShiftRight",
    },
    KeyInfo {
        key: Key::Space,
        snake: "space",
        w3c: "Space",
    },
    KeyInfo {
        key: Key::Tab,
        snake: "tab",
        w3c: "Tab",
    },
    KeyInfo {
        key: Key::Convert,
        snake: "convert",
        w3c: "Convert",
    },
    KeyInfo {
        key: Key::KanaMode,
        snake: "kana_mode",
        w3c: "KanaMode",
    },
    KeyInfo {
        key: Key::NonConvert,
        snake: "non_convert",
        w3c: "NonConvert",
    },
    KeyInfo {
        key: Key::Delete,
        snake: "delete",
        w3c: "Delete",
    },
    KeyInfo {
        key: Key::End,
        snake: "end",
        w3c: "End",
    },
    KeyInfo {
        key: Key::Help,
        snake: "help",
        w3c: "Help",
    },
    KeyInfo {
        key: Key::Home,
        snake: "home",
        w3c: "Home",
    },
    KeyInfo {
        key: Key::Insert,
        snake: "insert",
        w3c: "Insert",
    },
    KeyInfo {
        key: Key::PageDown,
        snake: "page_down",
        w3c: "PageDown",
    },
    KeyInfo {
        key: Key::PageUp,
        snake: "page_up",
        w3c: "PageUp",
    },
    KeyInfo {
        key: Key::ArrowDown,
        snake: "arrow_down",
        w3c: "ArrowDown",
    },
    KeyInfo {
        key: Key::ArrowLeft,
        snake: "arrow_left",
        w3c: "ArrowLeft",
    },
    KeyInfo {
        key: Key::ArrowRight,
        snake: "arrow_right",
        w3c: "ArrowRight",
    },
    KeyInfo {
        key: Key::ArrowUp,
        snake: "arrow_up",
        w3c: "ArrowUp",
    },
    KeyInfo {
        key: Key::NumLock,
        snake: "num_lock",
        w3c: "NumLock",
    },
    KeyInfo {
        key: Key::Numpad0,
        snake: "numpad_0",
        w3c: "Numpad0",
    },
    KeyInfo {
        key: Key::Numpad1,
        snake: "numpad_1",
        w3c: "Numpad1",
    },
    KeyInfo {
        key: Key::Numpad2,
        snake: "numpad_2",
        w3c: "Numpad2",
    },
    KeyInfo {
        key: Key::Numpad3,
        snake: "numpad_3",
        w3c: "Numpad3",
    },
    KeyInfo {
        key: Key::Numpad4,
        snake: "numpad_4",
        w3c: "Numpad4",
    },
    KeyInfo {
        key: Key::Numpad5,
        snake: "numpad_5",
        w3c: "Numpad5",
    },
    KeyInfo {
        key: Key::Numpad6,
        snake: "numpad_6",
        w3c: "Numpad6",
    },
    KeyInfo {
        key: Key::Numpad7,
        snake: "numpad_7",
        w3c: "Numpad7",
    },
    KeyInfo {
        key: Key::Numpad8,
        snake: "numpad_8",
        w3c: "Numpad8",
    },
    KeyInfo {
        key: Key::Numpad9,
        snake: "numpad_9",
        w3c: "Numpad9",
    },
    KeyInfo {
        key: Key::NumpadAdd,
        snake: "numpad_add",
        w3c: "NumpadAdd",
    },
    KeyInfo {
        key: Key::NumpadBackspace,
        snake: "numpad_backspace",
        w3c: "NumpadBackspace",
    },
    KeyInfo {
        key: Key::NumpadClear,
        snake: "numpad_clear",
        w3c: "NumpadClear",
    },
    KeyInfo {
        key: Key::NumpadClearEntry,
        snake: "numpad_clear_entry",
        w3c: "NumpadClearEntry",
    },
    KeyInfo {
        key: Key::NumpadComma,
        snake: "numpad_comma",
        w3c: "NumpadComma",
    },
    KeyInfo {
        key: Key::NumpadDecimal,
        snake: "numpad_decimal",
        w3c: "NumpadDecimal",
    },
    KeyInfo {
        key: Key::NumpadDivide,
        snake: "numpad_divide",
        w3c: "NumpadDivide",
    },
    KeyInfo {
        key: Key::NumpadEnter,
        snake: "numpad_enter",
        w3c: "NumpadEnter",
    },
    KeyInfo {
        key: Key::NumpadEqual,
        snake: "numpad_equal",
        w3c: "NumpadEqual",
    },
    KeyInfo {
        key: Key::NumpadMemoryAdd,
        snake: "numpad_memory_add",
        w3c: "NumpadMemoryAdd",
    },
    KeyInfo {
        key: Key::NumpadMemoryClear,
        snake: "numpad_memory_clear",
        w3c: "NumpadMemoryClear",
    },
    KeyInfo {
        key: Key::NumpadMemoryRecall,
        snake: "numpad_memory_recall",
        w3c: "NumpadMemoryRecall",
    },
    KeyInfo {
        key: Key::NumpadMemoryStore,
        snake: "numpad_memory_store",
        w3c: "NumpadMemoryStore",
    },
    KeyInfo {
        key: Key::NumpadMemorySubtract,
        snake: "numpad_memory_subtract",
        w3c: "NumpadMemorySubtract",
    },
    KeyInfo {
        key: Key::NumpadMultiply,
        snake: "numpad_multiply",
        w3c: "NumpadMultiply",
    },
    KeyInfo {
        key: Key::NumpadParenLeft,
        snake: "numpad_paren_left",
        w3c: "NumpadParenLeft",
    },
    KeyInfo {
        key: Key::NumpadParenRight,
        snake: "numpad_paren_right",
        w3c: "NumpadParenRight",
    },
    KeyInfo {
        key: Key::NumpadSubtract,
        snake: "numpad_subtract",
        w3c: "NumpadSubtract",
    },
    KeyInfo {
        key: Key::NumpadSeparator,
        snake: "numpad_separator",
        w3c: "NumpadSeparator",
    },
    KeyInfo {
        key: Key::NumpadUp,
        snake: "numpad_up",
        w3c: "NumpadUp",
    },
    KeyInfo {
        key: Key::NumpadDown,
        snake: "numpad_down",
        w3c: "NumpadDown",
    },
    KeyInfo {
        key: Key::NumpadRight,
        snake: "numpad_right",
        w3c: "NumpadRight",
    },
    KeyInfo {
        key: Key::NumpadLeft,
        snake: "numpad_left",
        w3c: "NumpadLeft",
    },
    KeyInfo {
        key: Key::NumpadBegin,
        snake: "numpad_begin",
        w3c: "NumpadBegin",
    },
    KeyInfo {
        key: Key::NumpadHome,
        snake: "numpad_home",
        w3c: "NumpadHome",
    },
    KeyInfo {
        key: Key::NumpadEnd,
        snake: "numpad_end",
        w3c: "NumpadEnd",
    },
    KeyInfo {
        key: Key::NumpadInsert,
        snake: "numpad_insert",
        w3c: "NumpadInsert",
    },
    KeyInfo {
        key: Key::NumpadDelete,
        snake: "numpad_delete",
        w3c: "NumpadDelete",
    },
    KeyInfo {
        key: Key::NumpadPageUp,
        snake: "numpad_page_up",
        w3c: "NumpadPageUp",
    },
    KeyInfo {
        key: Key::NumpadPageDown,
        snake: "numpad_page_down",
        w3c: "NumpadPageDown",
    },
    KeyInfo {
        key: Key::Escape,
        snake: "escape",
        w3c: "Escape",
    },
    KeyInfo {
        key: Key::F1,
        snake: "f1",
        w3c: "F1",
    },
    KeyInfo {
        key: Key::F2,
        snake: "f2",
        w3c: "F2",
    },
    KeyInfo {
        key: Key::F3,
        snake: "f3",
        w3c: "F3",
    },
    KeyInfo {
        key: Key::F4,
        snake: "f4",
        w3c: "F4",
    },
    KeyInfo {
        key: Key::F5,
        snake: "f5",
        w3c: "F5",
    },
    KeyInfo {
        key: Key::F6,
        snake: "f6",
        w3c: "F6",
    },
    KeyInfo {
        key: Key::F7,
        snake: "f7",
        w3c: "F7",
    },
    KeyInfo {
        key: Key::F8,
        snake: "f8",
        w3c: "F8",
    },
    KeyInfo {
        key: Key::F9,
        snake: "f9",
        w3c: "F9",
    },
    KeyInfo {
        key: Key::F10,
        snake: "f10",
        w3c: "F10",
    },
    KeyInfo {
        key: Key::F11,
        snake: "f11",
        w3c: "F11",
    },
    KeyInfo {
        key: Key::F12,
        snake: "f12",
        w3c: "F12",
    },
    KeyInfo {
        key: Key::F13,
        snake: "f13",
        w3c: "F13",
    },
    KeyInfo {
        key: Key::F14,
        snake: "f14",
        w3c: "F14",
    },
    KeyInfo {
        key: Key::F15,
        snake: "f15",
        w3c: "F15",
    },
    KeyInfo {
        key: Key::F16,
        snake: "f16",
        w3c: "F16",
    },
    KeyInfo {
        key: Key::F17,
        snake: "f17",
        w3c: "F17",
    },
    KeyInfo {
        key: Key::F18,
        snake: "f18",
        w3c: "F18",
    },
    KeyInfo {
        key: Key::F19,
        snake: "f19",
        w3c: "F19",
    },
    KeyInfo {
        key: Key::F20,
        snake: "f20",
        w3c: "F20",
    },
    KeyInfo {
        key: Key::F21,
        snake: "f21",
        w3c: "F21",
    },
    KeyInfo {
        key: Key::F22,
        snake: "f22",
        w3c: "F22",
    },
    KeyInfo {
        key: Key::F23,
        snake: "f23",
        w3c: "F23",
    },
    KeyInfo {
        key: Key::F24,
        snake: "f24",
        w3c: "F24",
    },
    KeyInfo {
        key: Key::F25,
        snake: "f25",
        w3c: "F25",
    },
    KeyInfo {
        key: Key::Fn,
        snake: "fn",
        w3c: "Fn",
    },
    KeyInfo {
        key: Key::FnLock,
        snake: "fn_lock",
        w3c: "FnLock",
    },
    KeyInfo {
        key: Key::PrintScreen,
        snake: "print_screen",
        w3c: "PrintScreen",
    },
    KeyInfo {
        key: Key::ScrollLock,
        snake: "scroll_lock",
        w3c: "ScrollLock",
    },
    KeyInfo {
        key: Key::Pause,
        snake: "pause",
        w3c: "Pause",
    },
    KeyInfo {
        key: Key::BrowserBack,
        snake: "browser_back",
        w3c: "BrowserBack",
    },
    KeyInfo {
        key: Key::BrowserFavorites,
        snake: "browser_favorites",
        w3c: "BrowserFavorites",
    },
    KeyInfo {
        key: Key::BrowserForward,
        snake: "browser_forward",
        w3c: "BrowserForward",
    },
    KeyInfo {
        key: Key::BrowserHome,
        snake: "browser_home",
        w3c: "BrowserHome",
    },
    KeyInfo {
        key: Key::BrowserRefresh,
        snake: "browser_refresh",
        w3c: "BrowserRefresh",
    },
    KeyInfo {
        key: Key::BrowserSearch,
        snake: "browser_search",
        w3c: "BrowserSearch",
    },
    KeyInfo {
        key: Key::BrowserStop,
        snake: "browser_stop",
        w3c: "BrowserStop",
    },
    KeyInfo {
        key: Key::Eject,
        snake: "eject",
        w3c: "Eject",
    },
    KeyInfo {
        key: Key::LaunchApp1,
        snake: "launch_app_1",
        w3c: "LaunchApp1",
    },
    KeyInfo {
        key: Key::LaunchApp2,
        snake: "launch_app_2",
        w3c: "LaunchApp2",
    },
    KeyInfo {
        key: Key::LaunchMail,
        snake: "launch_mail",
        w3c: "LaunchMail",
    },
    KeyInfo {
        key: Key::MediaPlayPause,
        snake: "media_play_pause",
        w3c: "MediaPlayPause",
    },
    KeyInfo {
        key: Key::MediaSelect,
        snake: "media_select",
        w3c: "MediaSelect",
    },
    KeyInfo {
        key: Key::MediaStop,
        snake: "media_stop",
        w3c: "MediaStop",
    },
    KeyInfo {
        key: Key::MediaTrackNext,
        snake: "media_track_next",
        w3c: "MediaTrackNext",
    },
    KeyInfo {
        key: Key::MediaTrackPrevious,
        snake: "media_track_previous",
        w3c: "MediaTrackPrevious",
    },
    KeyInfo {
        key: Key::Power,
        snake: "power",
        w3c: "Power",
    },
    KeyInfo {
        key: Key::Sleep,
        snake: "sleep",
        w3c: "Sleep",
    },
    KeyInfo {
        key: Key::AudioVolumeDown,
        snake: "audio_volume_down",
        w3c: "AudioVolumeDown",
    },
    KeyInfo {
        key: Key::AudioVolumeMute,
        snake: "audio_volume_mute",
        w3c: "AudioVolumeMute",
    },
    KeyInfo {
        key: Key::AudioVolumeUp,
        snake: "audio_volume_up",
        w3c: "AudioVolumeUp",
    },
    KeyInfo {
        key: Key::WakeUp,
        snake: "wake_up",
        w3c: "WakeUp",
    },
    KeyInfo {
        key: Key::Copy,
        snake: "copy",
        w3c: "Copy",
    },
    KeyInfo {
        key: Key::Cut,
        snake: "cut",
        w3c: "Cut",
    },
    KeyInfo {
        key: Key::Paste,
        snake: "paste",
        w3c: "Paste",
    },
];
const CODEPOINT_MAP: &[(u32, Key)] = &[
    ('a' as u32, Key::KeyA),
    ('b' as u32, Key::KeyB),
    ('c' as u32, Key::KeyC),
    ('d' as u32, Key::KeyD),
    ('e' as u32, Key::KeyE),
    ('f' as u32, Key::KeyF),
    ('g' as u32, Key::KeyG),
    ('h' as u32, Key::KeyH),
    ('i' as u32, Key::KeyI),
    ('j' as u32, Key::KeyJ),
    ('k' as u32, Key::KeyK),
    ('l' as u32, Key::KeyL),
    ('m' as u32, Key::KeyM),
    ('n' as u32, Key::KeyN),
    ('o' as u32, Key::KeyO),
    ('p' as u32, Key::KeyP),
    ('q' as u32, Key::KeyQ),
    ('r' as u32, Key::KeyR),
    ('s' as u32, Key::KeyS),
    ('t' as u32, Key::KeyT),
    ('u' as u32, Key::KeyU),
    ('v' as u32, Key::KeyV),
    ('w' as u32, Key::KeyW),
    ('x' as u32, Key::KeyX),
    ('y' as u32, Key::KeyY),
    ('z' as u32, Key::KeyZ),
    ('0' as u32, Key::Digit0),
    ('1' as u32, Key::Digit1),
    ('2' as u32, Key::Digit2),
    ('3' as u32, Key::Digit3),
    ('4' as u32, Key::Digit4),
    ('5' as u32, Key::Digit5),
    ('6' as u32, Key::Digit6),
    ('7' as u32, Key::Digit7),
    ('8' as u32, Key::Digit8),
    ('9' as u32, Key::Digit9),
    (';' as u32, Key::Semicolon),
    (' ' as u32, Key::Space),
    ('\'' as u32, Key::Quote),
    (',' as u32, Key::Comma),
    ('`' as u32, Key::Backquote),
    ('.' as u32, Key::Period),
    ('/' as u32, Key::Slash),
    ('-' as u32, Key::Minus),
    ('=' as u32, Key::Equal),
    ('[' as u32, Key::BracketLeft),
    (']' as u32, Key::BracketRight),
    ('\\' as u32, Key::Backslash),
    ('\t' as u32, Key::Tab),
    ('0' as u32, Key::Numpad0),
    ('1' as u32, Key::Numpad1),
    ('2' as u32, Key::Numpad2),
    ('3' as u32, Key::Numpad3),
    ('4' as u32, Key::Numpad4),
    ('5' as u32, Key::Numpad5),
    ('6' as u32, Key::Numpad6),
    ('7' as u32, Key::Numpad7),
    ('8' as u32, Key::Numpad8),
    ('9' as u32, Key::Numpad9),
    ('.' as u32, Key::NumpadDecimal),
    ('/' as u32, Key::NumpadDivide),
    ('*' as u32, Key::NumpadMultiply),
    ('-' as u32, Key::NumpadSubtract),
    ('+' as u32, Key::NumpadAdd),
    ('=' as u32, Key::NumpadEqual),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::key_mods::{ModSides, Mods, Side};

    #[test]
    fn key_event_effective_mods_follow_consumed_text_rule() {
        let mods = Mods {
            shift: true,
            alt: true,
            ..Mods::new()
        };
        let consumed = Mods {
            shift: true,
            ..Mods::new()
        };
        let mut event = KeyEvent {
            mods,
            consumed_mods: consumed,
            ..KeyEvent::default()
        };

        assert_eq!(event.effective_mods(), mods);
        event.utf8 = b"A".to_vec();
        assert_eq!(
            event.effective_mods(),
            Mods {
                alt: true,
                ..Mods::new()
            }
        );
    }

    #[test]
    fn key_event_binding_hash_uses_binding_fields_only() {
        let base = KeyEvent {
            key: Key::KeyA,
            mods: Mods {
                shift: true,
                caps_lock: true,
                ..Mods::new()
            },
            unshifted_codepoint: 'a' as u32,
            utf8: b"A".to_vec(),
            action: KeyAction::Press,
            ..KeyEvent::default()
        };
        let mut changed_action = base.clone();
        changed_action.action = KeyAction::Release;
        changed_action.utf8 = b"ignored".to_vec();
        assert_eq!(base.binding_hash(), changed_action.binding_hash());

        let mut changed_key = base.clone();
        changed_key.key = Key::KeyB;
        assert_ne!(base.binding_hash(), changed_key.binding_hash());

        let mut changed_codepoint = base.clone();
        changed_codepoint.unshifted_codepoint = 'b' as u32;
        assert_ne!(base.binding_hash(), changed_codepoint.binding_hash());

        let mut changed_mods = base.clone();
        changed_mods.mods.ctrl = true;
        assert_ne!(base.binding_hash(), changed_mods.binding_hash());
    }

    #[test]
    fn key_enum_order_matches_upstream_boundaries() {
        assert_eq!(KEY_COUNT, 176);
        assert_eq!(KeyAction::Release as i32, 0);
        assert_eq!(KeyAction::Press as i32, 1);
        assert_eq!(KeyAction::Repeat as i32, 2);
        assert_eq!(Key::Unidentified as i32, 0);
        assert_eq!(Key::Backquote as i32, 1);
        assert_eq!(Key::AltLeft as i32, 51);
        assert_eq!(Key::Delete as i32, 68);
        assert_eq!(Key::ArrowDown as i32, 75);
        assert_eq!(Key::NumLock as i32, 79);
        assert_eq!(Key::Escape as i32, 120);
        assert_eq!(Key::BrowserBack as i32, 151);
        assert_eq!(Key::Paste as i32, 175);
    }

    #[test]
    fn key_from_ascii_prefers_non_keypad_keys() {
        assert_eq!(Key::from_ascii(b'0'), Some(Key::Digit0));
        assert_eq!(Key::from_ascii(b'*'), None);
        assert_eq!(Key::from_ascii(b'a'), Some(Key::KeyA));
        assert_eq!(Key::from_ascii(b'\t'), Some(Key::Tab));
    }

    #[test]
    fn key_codepoint_covers_printable_keypad_and_nonprintable() {
        assert_eq!(Key::KeyA.codepoint(), Some('a' as u32));
        assert_eq!(Key::Digit0.codepoint(), Some('0' as u32));
        assert_eq!(Key::Numpad0.codepoint(), Some('0' as u32));
        assert_eq!(Key::NumpadAdd.codepoint(), Some('+' as u32));
        assert_eq!(Key::Escape.codepoint(), None);
    }

    #[test]
    fn key_keypad_detection_matches_numpad_prefix() {
        assert!(Key::Numpad0.keypad());
        assert!(Key::NumpadPageDown.keypad());
        assert!(!Key::Digit1.keypad());
    }

    #[test]
    fn key_w3c_round_trips_all_known_keys() {
        for key in ALL_KEYS {
            let w3c = key.w3c();
            assert_eq!(Key::from_w3c(w3c), Some(key), "{w3c}");
        }
        assert_eq!(Key::from_w3c("Digit0"), Some(Key::Digit0));
        assert_eq!(Key::from_w3c("digit0"), Some(Key::Digit0));
        assert_eq!(Key::from_w3c("Numpad0"), Some(Key::Numpad0));
        assert_eq!(Key::from_w3c("numpad0"), Some(Key::Numpad0));
        assert_eq!(Key::from_w3c("KeyA"), Some(Key::KeyA));
        assert_eq!(Key::from_w3c("key_a"), Some(Key::KeyA));
        assert_eq!(Key::from_w3c("does-not-exist"), None);
    }

    #[test]
    fn key_macos_ctrl_or_super_uses_meta_not_control() {
        assert!(Key::MetaLeft.ctrl_or_super());
        assert!(Key::MetaRight.ctrl_or_super());
        assert!(!Key::ControlLeft.ctrl_or_super());
        assert!(!Key::ControlRight.ctrl_or_super());
        assert!(Key::ShiftLeft.left_or_right_shift());
        assert!(Key::ShiftRight.left_or_right_shift());
        assert!(Key::AltLeft.left_or_right_alt());
        assert!(Key::AltRight.left_or_right_alt());
        assert!(!Key::KeyA.left_or_right_alt());
    }

    #[test]
    fn key_mods_side_bits_are_available_to_key_events() {
        let event = KeyEvent {
            mods: Mods {
                alt: true,
                sides: ModSides {
                    alt: Side::Right,
                    ..ModSides::default()
                },
                ..Mods::new()
            },
            ..KeyEvent::default()
        };
        assert_eq!(event.mods.sides.alt, Side::Right);
    }
}

// Native (macOS) keycode -> physical Key, ported from vendor/ghostty/src/input/keycodes.zig
// (Issue 802 / Exp 8). raw_entries[mac] -> W3C code -> Key (same variant name).
pub(crate) const NATIVE_TO_KEY: &[(u32, Key)] = &[
    (0x0000, Key::KeyA),
    (0x0001, Key::KeyS),
    (0x0002, Key::KeyD),
    (0x0003, Key::KeyF),
    (0x0004, Key::KeyH),
    (0x0005, Key::KeyG),
    (0x0006, Key::KeyZ),
    (0x0007, Key::KeyX),
    (0x0008, Key::KeyC),
    (0x0009, Key::KeyV),
    (0x000a, Key::IntlBackslash),
    (0x000b, Key::KeyB),
    (0x000c, Key::KeyQ),
    (0x000d, Key::KeyW),
    (0x000e, Key::KeyE),
    (0x000f, Key::KeyR),
    (0x0010, Key::KeyY),
    (0x0011, Key::KeyT),
    (0x0012, Key::Digit1),
    (0x0013, Key::Digit2),
    (0x0014, Key::Digit3),
    (0x0015, Key::Digit4),
    (0x0016, Key::Digit6),
    (0x0017, Key::Digit5),
    (0x0018, Key::Equal),
    (0x0019, Key::Digit9),
    (0x001a, Key::Digit7),
    (0x001b, Key::Minus),
    (0x001c, Key::Digit8),
    (0x001d, Key::Digit0),
    (0x001e, Key::BracketRight),
    (0x001f, Key::KeyO),
    (0x0020, Key::KeyU),
    (0x0021, Key::BracketLeft),
    (0x0022, Key::KeyI),
    (0x0023, Key::KeyP),
    (0x0024, Key::Enter),
    (0x0025, Key::KeyL),
    (0x0026, Key::KeyJ),
    (0x0027, Key::Quote),
    (0x0028, Key::KeyK),
    (0x0029, Key::Semicolon),
    (0x002a, Key::Backslash),
    (0x002b, Key::Comma),
    (0x002c, Key::Slash),
    (0x002d, Key::KeyN),
    (0x002e, Key::KeyM),
    (0x002f, Key::Period),
    (0x0030, Key::Tab),
    (0x0031, Key::Space),
    (0x0032, Key::Backquote),
    (0x0033, Key::Backspace),
    (0x0035, Key::Escape),
    (0x0036, Key::MetaRight),
    (0x0037, Key::MetaLeft),
    (0x0038, Key::ShiftLeft),
    (0x0039, Key::CapsLock),
    (0x003a, Key::AltLeft),
    (0x003b, Key::ControlLeft),
    (0x003c, Key::ShiftRight),
    (0x003d, Key::AltRight),
    (0x003e, Key::ControlRight),
    (0x0040, Key::F17),
    (0x0041, Key::NumpadDecimal),
    (0x0043, Key::NumpadMultiply),
    (0x0045, Key::NumpadAdd),
    (0x0047, Key::NumLock),
    (0x0048, Key::AudioVolumeUp),
    (0x0049, Key::AudioVolumeDown),
    (0x004a, Key::AudioVolumeMute),
    (0x004b, Key::NumpadDivide),
    (0x004c, Key::NumpadEnter),
    (0x004e, Key::NumpadSubtract),
    (0x004f, Key::F18),
    (0x0050, Key::F19),
    (0x0051, Key::NumpadEqual),
    (0x0052, Key::Numpad0),
    (0x0053, Key::Numpad1),
    (0x0054, Key::Numpad2),
    (0x0055, Key::Numpad3),
    (0x0056, Key::Numpad4),
    (0x0057, Key::Numpad5),
    (0x0058, Key::Numpad6),
    (0x0059, Key::Numpad7),
    (0x005a, Key::F20),
    (0x005b, Key::Numpad8),
    (0x005c, Key::Numpad9),
    (0x005d, Key::IntlYen),
    (0x005e, Key::IntlRo),
    (0x005f, Key::NumpadComma),
    (0x0060, Key::F5),
    (0x0061, Key::F6),
    (0x0062, Key::F7),
    (0x0063, Key::F3),
    (0x0064, Key::F8),
    (0x0065, Key::F9),
    (0x0067, Key::F11),
    (0x0069, Key::F13),
    (0x006a, Key::F16),
    (0x006b, Key::F14),
    (0x006d, Key::F10),
    (0x006e, Key::ContextMenu),
    (0x006f, Key::F12),
    (0x0071, Key::F15),
    (0x0072, Key::Insert),
    (0x0073, Key::Home),
    (0x0074, Key::PageUp),
    (0x0075, Key::Delete),
    (0x0076, Key::F4),
    (0x0077, Key::End),
    (0x0078, Key::F2),
    (0x0079, Key::PageDown),
    (0x007a, Key::F1),
    (0x007b, Key::ArrowLeft),
    (0x007c, Key::ArrowRight),
    (0x007d, Key::ArrowDown),
    (0x007e, Key::ArrowUp),
];

/// Resolve a native platform keycode to the physical Key (matches ghostty
/// `apprt/embedded.zig` `KeyEvent.core()`), defaulting to Unidentified.
pub(crate) fn key_from_native(native: u32) -> Key {
    NATIVE_TO_KEY
        .iter()
        .copied()
        .find(|(n, _)| *n == native)
        .map(|(_, k)| k)
        .unwrap_or(Key::Unidentified)
}
