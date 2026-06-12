//! macOS keyboard layout translation.
//!
//! This is the Rust foundation for upstream `input/KeymapDarwin.zig`. The
//! copied Swift app still provides text through AppKit; this module owns only
//! the platform translation primitive for later ABI/app wiring.

use crate::input::key_mods::Mods;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Error {
    Unsupported,
    GetInputSourceFailed,
    GetUnicodeLayoutFailed,
    TranslateFailed,
    Utf16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct State {
    pub(crate) dead_key: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Translation {
    pub(crate) text: String,
    pub(crate) composing: bool,
    pub(crate) mods: Mods,
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{Error, Mods, State, Translation};
    use libc::{c_ulong, c_void};
    use objc2_core_foundation::CFString;
    use std::ptr;

    const K_UC_KEY_ACTION_DOWN: u16 = 0;
    const K_UC_KEY_TRANSLATE_NO_DEAD_KEYS_BIT: u32 = 0;
    const K_UC_KEY_TRANSLATE_NO_DEAD_KEYS_MASK: u32 = 1 << K_UC_KEY_TRANSLATE_NO_DEAD_KEYS_BIT;
    const SPACE_KEYCODE: u16 = 0x31;

    #[repr(C)]
    struct UCKeyboardLayout {
        _private: [u8; 0],
    }

    pub(crate) struct KeymapDarwin {
        source: *mut c_void,
        unicode_layout: *const UCKeyboardLayout,
    }

    impl KeymapDarwin {
        pub(crate) fn new() -> Result<Self, Error> {
            let mut keymap = Self {
                source: ptr::null_mut(),
                unicode_layout: ptr::null(),
            };
            keymap.reinit()?;
            Ok(keymap)
        }

        pub(crate) fn reload(&mut self) -> Result<(), Error> {
            self.release_source();
            self.reinit()
        }

        pub(crate) fn source_id(&self) -> Option<String> {
            unsafe {
                let id = TISGetInputSourceProperty(self.source, kTISPropertyInputSourceID);
                if id.is_null() {
                    None
                } else {
                    Some((&*(id.cast::<CFString>())).to_string())
                }
            }
        }

        pub(crate) fn translate(
            &self,
            state: &mut State,
            code: u16,
            input_mods: Mods,
        ) -> Result<Translation, Error> {
            let mods = translation_mods(input_mods);
            let modifier_state = carbon_modifier_state(mods);
            let mut chars = [0u16; 4];
            let mut char_count: c_ulong = 0;

            let status = unsafe {
                UCKeyTranslate(
                    self.unicode_layout,
                    code,
                    K_UC_KEY_ACTION_DOWN,
                    modifier_state,
                    LMGetKbdType(),
                    K_UC_KEY_TRANSLATE_NO_DEAD_KEYS_BIT,
                    &mut state.dead_key,
                    chars.len() as c_ulong,
                    &mut char_count,
                    chars.as_mut_ptr(),
                )
            };
            if status != 0 {
                return Err(Error::TranslateFailed);
            }

            let composing = if state.dead_key != 0 && char_count == 0 {
                let mut dead_key_ignore = state.dead_key;
                let status = unsafe {
                    UCKeyTranslate(
                        self.unicode_layout,
                        SPACE_KEYCODE,
                        K_UC_KEY_ACTION_DOWN,
                        modifier_state,
                        LMGetKbdType(),
                        K_UC_KEY_TRANSLATE_NO_DEAD_KEYS_MASK,
                        &mut dead_key_ignore,
                        chars.len() as c_ulong,
                        &mut char_count,
                        chars.as_mut_ptr(),
                    )
                };
                if status != 0 {
                    return Err(Error::TranslateFailed);
                }
                true
            } else {
                false
            };

            let text = utf16_text(&chars, char_count)?;
            Ok(Translation {
                text,
                composing,
                mods,
            })
        }

        fn reinit(&mut self) -> Result<(), Error> {
            unsafe {
                self.source = TISCopyCurrentKeyboardLayoutInputSource();
                if self.source.is_null() {
                    return Err(Error::GetInputSourceFailed);
                }

                let data = TISGetInputSourceProperty(self.source, kTISPropertyUnicodeKeyLayoutData);
                if data.is_null() {
                    self.release_source();
                    return Err(Error::GetUnicodeLayoutFailed);
                }

                self.unicode_layout = cf_data_get_byte_ptr(data.cast()).cast();
                if self.unicode_layout.is_null() {
                    self.release_source();
                    return Err(Error::GetUnicodeLayoutFailed);
                }
            }

            Ok(())
        }

        fn release_source(&mut self) {
            if !self.source.is_null() {
                unsafe {
                    CFRelease(self.source);
                }
                self.source = ptr::null_mut();
                self.unicode_layout = ptr::null();
            }
        }
    }

    impl Drop for KeymapDarwin {
        fn drop(&mut self) {
            self.release_source();
        }
    }

    pub(crate) fn utf16_text(chars: &[u16; 4], count: c_ulong) -> Result<String, Error> {
        let count = usize::try_from(count).map_err(|_| Error::TranslateFailed)?;
        if count > chars.len() {
            return Err(Error::TranslateFailed);
        }
        String::from_utf16(&chars[..count]).map_err(|_| Error::Utf16)
    }

    pub(crate) fn translation_mods(mut mods: Mods) -> Mods {
        mods.ctrl = false;
        mods
    }

    pub(crate) fn carbon_modifier_state(mods: Mods) -> u32 {
        let mut value = 0u32;
        if mods.super_ {
            value |= 0x100;
        }
        if mods.shift {
            value |= 0x200;
        }
        if mods.caps_lock {
            value |= 0x400;
        }
        if mods.alt {
            value |= 0x800;
        }
        if mods.ctrl {
            value |= 0x1000;
        }
        (value >> 8) & 0xff
    }

    unsafe fn cf_data_get_byte_ptr(data: *const c_void) -> *const u8 {
        unsafe extern "C" {
            fn CFDataGetBytePtr(theData: *const c_void) -> *const u8;
        }
        unsafe { CFDataGetBytePtr(data) }
    }

    unsafe extern "C" {
        #[allow(non_upper_case_globals)]
        static kTISPropertyUnicodeKeyLayoutData: *const c_void;
        #[allow(non_upper_case_globals)]
        static kTISPropertyInputSourceID: *const c_void;

        fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut c_void;
        fn TISGetInputSourceProperty(source: *mut c_void, key: *const c_void) -> *const c_void;
        fn LMGetKbdType() -> u32;
        fn UCKeyTranslate(
            key_layout_ptr: *const UCKeyboardLayout,
            virtual_key_code: u16,
            key_action: u16,
            modifier_key_state: u32,
            keyboard_type: u32,
            key_translate_options: u32,
            dead_key_state: *mut u32,
            max_string_length: c_ulong,
            actual_string_length: *mut c_ulong,
            unicode_string: *mut u16,
        ) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(cf: *mut c_void);
    }

    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {}
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{Error, Mods, State, Translation};

    pub(crate) struct KeymapDarwin;

    impl KeymapDarwin {
        pub(crate) fn new() -> Result<Self, Error> {
            Err(Error::Unsupported)
        }

        pub(crate) fn reload(&mut self) -> Result<(), Error> {
            Err(Error::Unsupported)
        }

        pub(crate) fn source_id(&self) -> Option<String> {
            None
        }

        pub(crate) fn translate(
            &self,
            _state: &mut State,
            _code: u16,
            _input_mods: Mods,
        ) -> Result<Translation, Error> {
            Err(Error::Unsupported)
        }
    }

    pub(crate) fn translation_mods(mut mods: Mods) -> Mods {
        mods.ctrl = false;
        mods
    }

    pub(crate) fn carbon_modifier_state(mods: Mods) -> u32 {
        let mut value = 0u32;
        if mods.super_ {
            value |= 0x100;
        }
        if mods.shift {
            value |= 0x200;
        }
        if mods.caps_lock {
            value |= 0x400;
        }
        if mods.alt {
            value |= 0x800;
        }
        if mods.ctrl {
            value |= 0x1000;
        }
        (value >> 8) & 0xff
    }
}

#[allow(unused_imports)]
pub(crate) use platform::KeymapDarwin;

#[cfg(test)]
mod tests {
    use super::*;

    fn mods_for_carbon() -> Mods {
        Mods {
            shift: true,
            ctrl: true,
            alt: true,
            super_: true,
            caps_lock: true,
            ..Mods::new()
        }
    }

    #[test]
    fn keymap_darwin_carbon_modifier_state_matches_upstream_bits() {
        let mut mods = Mods::new();
        mods.super_ = true;
        assert_eq!(platform::carbon_modifier_state(mods), 0x01);

        mods = Mods::new();
        mods.shift = true;
        assert_eq!(platform::carbon_modifier_state(mods), 0x02);

        mods = Mods::new();
        mods.caps_lock = true;
        assert_eq!(platform::carbon_modifier_state(mods), 0x04);

        mods = Mods::new();
        mods.alt = true;
        assert_eq!(platform::carbon_modifier_state(mods), 0x08);

        mods = Mods::new();
        mods.ctrl = true;
        assert_eq!(platform::carbon_modifier_state(mods), 0x10);

        assert_eq!(platform::carbon_modifier_state(mods_for_carbon()), 0x1f);
    }

    #[test]
    fn keymap_darwin_translation_mods_strip_control_only() {
        let translated = platform::translation_mods(mods_for_carbon());
        assert!(translated.shift);
        assert!(!translated.ctrl);
        assert!(translated.alt);
        assert!(translated.super_);
        assert!(translated.caps_lock);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn keymap_darwin_utf16_text_validates_count_and_encoding() {
        let mut chars = [0u16; 4];
        chars[0] = 'a' as u16;
        assert_eq!(platform::utf16_text(&chars, 1).unwrap(), "a");

        chars[0] = 0xd83d;
        chars[1] = 0xde00;
        assert_eq!(platform::utf16_text(&chars, 2).unwrap(), "😀");

        assert_eq!(platform::utf16_text(&chars, 5), Err(Error::TranslateFailed));

        chars[0] = 0xd83d;
        chars[1] = 0;
        assert_eq!(platform::utf16_text(&chars, 1), Err(Error::Utf16));
    }

    #[test]
    fn keymap_darwin_non_macos_stub_is_unsupported() {
        #[cfg(not(target_os = "macos"))]
        {
            let mut keymap = platform::KeymapDarwin;
            assert_eq!(
                platform::KeymapDarwin::new().err(),
                Some(Error::Unsupported)
            );
            assert_eq!(keymap.reload().err(), Some(Error::Unsupported));
            assert_eq!(keymap.source_id(), None);
            assert_eq!(
                keymap
                    .translate(&mut State::default(), 0, Mods::new())
                    .err(),
                Some(Error::Unsupported),
            );
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn keymap_darwin_host_smoke_translate_current_layout() {
        let mut keymap = match platform::KeymapDarwin::new() {
            Ok(keymap) => keymap,
            Err(Error::GetInputSourceFailed | Error::GetUnicodeLayoutFailed) => return,
            Err(err) => panic!("unexpected keymap init error: {err:?}"),
        };
        assert!(keymap.reload().is_ok());

        let source_id = keymap.source_id();
        if let Some(source_id) = &source_id {
            assert!(!source_id.is_empty());
        }

        let mut state = State::default();
        let translated = keymap
            .translate(&mut state, 0, Mods::new())
            .expect("current layout should translate keycode 0");
        assert!(translated.text.len() <= 8);
        assert_eq!(translated.mods, Mods::new());

        if matches!(
            source_id.as_deref(),
            Some("com.apple.keylayout.US" | "com.apple.keylayout.USInternational")
        ) {
            assert_eq!(translated.text, "a");

            let mut shift = Mods::new();
            shift.shift = true;
            let shifted = keymap
                .translate(&mut State::default(), 0, shift)
                .expect("current layout should translate shifted keycode 0");
            assert_eq!(shifted.text, "A");
            assert_eq!(shifted.mods, shift);
        }

        let mut ctrl = Mods::new();
        ctrl.ctrl = true;
        let ctrl_translated = keymap
            .translate(&mut State::default(), 0, ctrl)
            .expect("current layout should translate control-stripped keycode 0");
        assert!(!ctrl_translated.mods.ctrl);
    }
}
