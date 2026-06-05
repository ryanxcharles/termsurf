//! A clickable terminal link: a regex over terminal text that triggers an action (port of
//! upstream `input/Link`).

use super::key_mods::Mods;

/// The action triggered when a link is clicked (upstream `Link.Action`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// Open the full matched value with the default opener (e.g. `open` on macOS).
    Open,
    /// Open the OSC8 hyperlink under the mouse. Internal-only (upstream's leading-underscore
    /// `_open_osc8` — not user-specifiable).
    OpenOsc8,
}

/// When a link is highlighted (and thus clickable) (upstream `Link.Highlight`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Highlight {
    /// Always highlight the link.
    Always,
    /// Only highlight while the mouse hovers over it.
    Hover,
    /// Highlight whenever the given modifiers are held (regardless of hover). Note: "shift" never
    /// matches in TUI programs that capture the mouse (the capture strips shift).
    AlwaysMods(Mods),
    /// Highlight while hovering with the given modifiers held.
    HoverMods(Mods),
}

/// A clickable link: a regex match over terminal text that triggers an action (upstream `Link`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Link {
    /// The regex used to match the link (byte string, mirroring upstream `[]const u8`).
    pub(crate) regex: Vec<u8>,
    /// The action triggered when the link is clicked.
    pub(crate) action: Action,
    /// When the link is highlighted / clickable.
    pub(crate) highlight: Highlight,
}

impl Link {
    /// Whether two links are equal (upstream `equal`): same action, highlight, and regex bytes.
    /// Delegates to the derived `PartialEq`, which compares all three fields.
    pub(crate) fn equal(&self, other: &Link) -> bool {
        self == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Link {
        Link {
            regex: b"https?://.*".to_vec(),
            action: Action::Open,
            highlight: Highlight::Hover,
        }
    }

    #[test]
    fn equal_compares_all_three_fields() {
        let a = sample();
        let b = sample();
        assert!(a.equal(&b));

        let mut differ_regex = sample();
        differ_regex.regex = b"ftp://.*".to_vec();
        assert!(!a.equal(&differ_regex));

        let mut differ_action = sample();
        differ_action.action = Action::OpenOsc8;
        assert!(!a.equal(&differ_action));

        let mut differ_highlight = sample();
        differ_highlight.highlight = Highlight::Always;
        assert!(!a.equal(&differ_highlight));
    }

    #[test]
    fn clone_is_a_deep_copy() {
        let a = sample();
        let b = a.clone();
        assert!(a.equal(&b));
        assert_eq!(b.regex, b"https?://.*");
        // The clones own separate buffers (mutating one does not affect the other).
        let mut b = b;
        b.regex.push(b'!');
        assert_ne!(a.regex, b.regex);
    }

    #[test]
    fn highlight_mods_compare_by_value() {
        let ctrl = Mods {
            ctrl: true,
            ..Mods::default()
        };
        let shift = Mods {
            shift: true,
            ..Mods::default()
        };
        assert_eq!(Highlight::AlwaysMods(ctrl), Highlight::AlwaysMods(ctrl));
        assert_ne!(Highlight::AlwaysMods(ctrl), Highlight::AlwaysMods(shift));
        // `AlwaysMods` and `HoverMods` with the same mods are still distinct variants.
        assert_ne!(Highlight::AlwaysMods(ctrl), Highlight::HoverMods(ctrl));
    }

    #[test]
    fn action_variants_are_distinct() {
        assert_ne!(Action::Open, Action::OpenOsc8);
    }
}
