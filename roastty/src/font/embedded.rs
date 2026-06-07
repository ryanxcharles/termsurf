//! Resource-backed embedded fonts.
//!
//! This ports the `@embedFile("res/...")` entries from upstream
//! `font/embedded.zig`. Generated build dependency blobs such as the default
//! JetBrains variable fonts and Symbols Nerd Font are intentionally not included
//! here yet.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum License {
    Ofl,
    Mit,
    Bsd2Clause,
}

impl License {
    pub(crate) fn text(self) -> &'static str {
        match self {
            License::Ofl => include_str!("../../../vendor/ghostty/src/font/res/OFL.txt"),
            License::Mit => include_str!("../../../vendor/ghostty/src/font/res/MIT.txt"),
            License::Bsd2Clause => {
                include_str!("../../../vendor/ghostty/src/font/res/BSD-2-Clause.txt")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Emoji,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmbeddedFont {
    Emoji,
    EmojiText,
    Arabic,
    TestNerdFont,
    CodeNewRoman,
    Inconsolata,
    GeistMono,
    JetBrainsMono,
    JuliaMono,
    Cozette,
    MonaspaceNeon,
    TerminusTtf,
    SpleenBdf,
    SpleenPcf,
    SpleenOtb,
}

impl EmbeddedFont {
    pub(crate) const ALL: &'static [EmbeddedFont] = &[
        EmbeddedFont::Emoji,
        EmbeddedFont::EmojiText,
        EmbeddedFont::Arabic,
        EmbeddedFont::TestNerdFont,
        EmbeddedFont::CodeNewRoman,
        EmbeddedFont::Inconsolata,
        EmbeddedFont::GeistMono,
        EmbeddedFont::JetBrainsMono,
        EmbeddedFont::JuliaMono,
        EmbeddedFont::Cozette,
        EmbeddedFont::MonaspaceNeon,
        EmbeddedFont::TerminusTtf,
        EmbeddedFont::SpleenBdf,
        EmbeddedFont::SpleenPcf,
        EmbeddedFont::SpleenOtb,
    ];

    pub(crate) fn upstream_name(self) -> &'static str {
        match self {
            EmbeddedFont::Emoji => "emoji",
            EmbeddedFont::EmojiText => "emoji_text",
            EmbeddedFont::Arabic => "arabic",
            EmbeddedFont::TestNerdFont => "test_nerd_font",
            EmbeddedFont::CodeNewRoman => "code_new_roman",
            EmbeddedFont::Inconsolata => "inconsolata",
            EmbeddedFont::GeistMono => "geist_mono",
            EmbeddedFont::JetBrainsMono => "jetbrains_mono",
            EmbeddedFont::JuliaMono => "julia_mono",
            EmbeddedFont::Cozette => "cozette",
            EmbeddedFont::MonaspaceNeon => "monaspace_neon",
            EmbeddedFont::TerminusTtf => "terminus_ttf",
            EmbeddedFont::SpleenBdf => "spleen_bdf",
            EmbeddedFont::SpleenPcf => "spleen_pcf",
            EmbeddedFont::SpleenOtb => "spleen_otb",
        }
    }

    pub(crate) fn file_name(self) -> &'static str {
        match self {
            EmbeddedFont::Emoji => "NotoColorEmoji.ttf",
            EmbeddedFont::EmojiText => "NotoEmoji-Regular.ttf",
            EmbeddedFont::Arabic => "KawkabMono-Regular.ttf",
            EmbeddedFont::TestNerdFont => "JetBrainsMonoNerdFont-Regular.ttf",
            EmbeddedFont::CodeNewRoman => "CodeNewRoman-Regular.otf",
            EmbeddedFont::Inconsolata => "Inconsolata-Regular.ttf",
            EmbeddedFont::GeistMono => "GeistMono-Regular.ttf",
            EmbeddedFont::JetBrainsMono => "JetBrainsMonoNoNF-Regular.ttf",
            EmbeddedFont::JuliaMono => "JuliaMono-Regular.ttf",
            EmbeddedFont::Cozette => "CozetteVector.ttf",
            EmbeddedFont::MonaspaceNeon => "MonaspaceNeon-Regular.otf",
            EmbeddedFont::TerminusTtf => "TerminusTTF-Regular.ttf",
            EmbeddedFont::SpleenBdf => "spleen-8x16.bdf",
            EmbeddedFont::SpleenPcf => "spleen-8x16.pcf",
            EmbeddedFont::SpleenOtb => "spleen-8x16.otb",
        }
    }

    pub(crate) fn license(self) -> License {
        match self {
            EmbeddedFont::Cozette => License::Mit,
            EmbeddedFont::SpleenBdf | EmbeddedFont::SpleenPcf | EmbeddedFont::SpleenOtb => {
                License::Bsd2Clause
            }
            EmbeddedFont::Emoji
            | EmbeddedFont::EmojiText
            | EmbeddedFont::Arabic
            | EmbeddedFont::TestNerdFont
            | EmbeddedFont::CodeNewRoman
            | EmbeddedFont::Inconsolata
            | EmbeddedFont::GeistMono
            | EmbeddedFont::JetBrainsMono
            | EmbeddedFont::JuliaMono
            | EmbeddedFont::MonaspaceNeon
            | EmbeddedFont::TerminusTtf => License::Ofl,
        }
    }

    pub(crate) fn role(self) -> Role {
        match self {
            EmbeddedFont::Emoji | EmbeddedFont::EmojiText => Role::Emoji,
            EmbeddedFont::Arabic
            | EmbeddedFont::TestNerdFont
            | EmbeddedFont::CodeNewRoman
            | EmbeddedFont::Inconsolata
            | EmbeddedFont::GeistMono
            | EmbeddedFont::JetBrainsMono
            | EmbeddedFont::JuliaMono
            | EmbeddedFont::Cozette
            | EmbeddedFont::MonaspaceNeon
            | EmbeddedFont::TerminusTtf
            | EmbeddedFont::SpleenBdf
            | EmbeddedFont::SpleenPcf
            | EmbeddedFont::SpleenOtb => Role::Test,
        }
    }

    pub(crate) fn bytes(self) -> &'static [u8] {
        match self {
            EmbeddedFont::Emoji => {
                include_bytes!("../../../vendor/ghostty/src/font/res/NotoColorEmoji.ttf")
            }
            EmbeddedFont::EmojiText => {
                include_bytes!("../../../vendor/ghostty/src/font/res/NotoEmoji-Regular.ttf")
            }
            EmbeddedFont::Arabic => {
                include_bytes!("../../../vendor/ghostty/src/font/res/KawkabMono-Regular.ttf")
            }
            EmbeddedFont::TestNerdFont => include_bytes!(
                "../../../vendor/ghostty/src/font/res/JetBrainsMonoNerdFont-Regular.ttf"
            ),
            EmbeddedFont::CodeNewRoman => {
                include_bytes!("../../../vendor/ghostty/src/font/res/CodeNewRoman-Regular.otf")
            }
            EmbeddedFont::Inconsolata => {
                include_bytes!("../../../vendor/ghostty/src/font/res/Inconsolata-Regular.ttf")
            }
            EmbeddedFont::GeistMono => {
                include_bytes!("../../../vendor/ghostty/src/font/res/GeistMono-Regular.ttf")
            }
            EmbeddedFont::JetBrainsMono => {
                include_bytes!("../../../vendor/ghostty/src/font/res/JetBrainsMonoNoNF-Regular.ttf")
            }
            EmbeddedFont::JuliaMono => {
                include_bytes!("../../../vendor/ghostty/src/font/res/JuliaMono-Regular.ttf")
            }
            EmbeddedFont::Cozette => {
                include_bytes!("../../../vendor/ghostty/src/font/res/CozetteVector.ttf")
            }
            EmbeddedFont::MonaspaceNeon => {
                include_bytes!("../../../vendor/ghostty/src/font/res/MonaspaceNeon-Regular.otf")
            }
            EmbeddedFont::TerminusTtf => {
                include_bytes!("../../../vendor/ghostty/src/font/res/TerminusTTF-Regular.ttf")
            }
            EmbeddedFont::SpleenBdf => {
                include_bytes!("../../../vendor/ghostty/src/font/res/spleen-8x16.bdf")
            }
            EmbeddedFont::SpleenPcf => {
                include_bytes!("../../../vendor/ghostty/src/font/res/spleen-8x16.pcf")
            }
            EmbeddedFont::SpleenOtb => {
                include_bytes!("../../../vendor/ghostty/src/font/res/spleen-8x16.otb")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED: &[(&str, &str)] = &[
        ("emoji", "NotoColorEmoji.ttf"),
        ("emoji_text", "NotoEmoji-Regular.ttf"),
        ("arabic", "KawkabMono-Regular.ttf"),
        ("test_nerd_font", "JetBrainsMonoNerdFont-Regular.ttf"),
        ("code_new_roman", "CodeNewRoman-Regular.otf"),
        ("inconsolata", "Inconsolata-Regular.ttf"),
        ("geist_mono", "GeistMono-Regular.ttf"),
        ("jetbrains_mono", "JetBrainsMonoNoNF-Regular.ttf"),
        ("julia_mono", "JuliaMono-Regular.ttf"),
        ("cozette", "CozetteVector.ttf"),
        ("monaspace_neon", "MonaspaceNeon-Regular.otf"),
        ("terminus_ttf", "TerminusTTF-Regular.ttf"),
        ("spleen_bdf", "spleen-8x16.bdf"),
        ("spleen_pcf", "spleen-8x16.pcf"),
        ("spleen_otb", "spleen-8x16.otb"),
    ];

    #[test]
    fn embedded_inventory_matches_upstream_res_symbols() {
        assert_eq!(EmbeddedFont::ALL.len(), EXPECTED.len());

        for (font, &(name, file)) in EmbeddedFont::ALL.iter().zip(EXPECTED) {
            assert_eq!(font.upstream_name(), name);
            assert_eq!(font.file_name(), file);
        }
    }

    #[test]
    fn embedded_inventory_has_unique_upstream_names_and_files() {
        let mut names = HashSet::new();
        let mut files = HashSet::new();

        for font in EmbeddedFont::ALL {
            assert!(names.insert(font.upstream_name()));
            assert!(files.insert(font.file_name()));
        }
    }

    #[test]
    fn embedded_bytes_are_nonempty_and_have_expected_font_signatures() {
        for font in EmbeddedFont::ALL {
            let bytes = font.bytes();
            assert!(
                !bytes.is_empty(),
                "empty embedded font {}",
                font.file_name()
            );

            match font {
                EmbeddedFont::SpleenBdf => assert!(bytes.starts_with(b"STARTFONT")),
                EmbeddedFont::SpleenPcf => assert!(bytes.starts_with(&[0x01, b'f', b'c', b'p'])),
                EmbeddedFont::MonaspaceNeon => assert!(bytes.starts_with(b"OTTO")),
                _ => assert!(
                    bytes.starts_with(&[0x00, 0x01, 0x00, 0x00]) || bytes.starts_with(b"ttcf"),
                    "unexpected sfnt signature for {}",
                    font.file_name()
                ),
            }
        }
    }

    #[test]
    fn embedded_license_texts_are_available() {
        for license in [License::Ofl, License::Mit, License::Bsd2Clause] {
            assert!(!license.text().is_empty());
        }

        assert!(License::Ofl.text().contains("SIL OPEN FONT LICENSE"));
        assert!(License::Mit.text().contains("MIT License"));
        assert!(License::Bsd2Clause
            .text()
            .contains("Redistribution and use"));
    }

    #[test]
    fn embedded_roles_match_upstream_comments() {
        assert_eq!(EmbeddedFont::Emoji.role(), Role::Emoji);
        assert_eq!(EmbeddedFont::EmojiText.role(), Role::Emoji);

        for font in &EmbeddedFont::ALL[2..] {
            assert_eq!(font.role(), Role::Test);
        }
    }
}
