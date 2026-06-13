#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Rgb {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct CRgb {
    r: u8,
    g: u8,
    b: u8,
}

pub(crate) type Palette = [Rgb; 256];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct DynamicRgb {
    override_rgb: Option<Rgb>,
    default_rgb: Option<Rgb>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DynamicPalette {
    current: Palette,
    original: Palette,
    mask: PaletteMask,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PaletteMask {
    words: [u64; 4],
}

impl Rgb {
    pub(crate) const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub(super) const fn from_c(c: CRgb) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
        }
    }

    pub(super) const fn cval(self) -> CRgb {
        CRgb {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }

    pub(super) fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }

        if let Some(hex) = bytes.strip_prefix(b"#") {
            return parse_hash_rgb(hex);
        }

        if let Some(rgb) = super::x11_color::get(bytes) {
            return Some(rgb);
        }

        if !bytes.starts_with(b"rgb") {
            return None;
        }
        let mut offset = 3;
        let use_intensity = if bytes.get(offset) == Some(&b'i') {
            offset += 1;
            true
        } else {
            false
        };
        if bytes.get(offset) != Some(&b':') {
            return None;
        }
        offset += 1;

        let mut parts = bytes[offset..].split(|byte| *byte == b'/');
        let r = parse_component(parts.next()?, use_intensity)?;
        let g = parse_component(parts.next()?, use_intensity)?;
        let b = parse_component(parts.next()?, use_intensity)?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self::new(r, g, b))
    }

    pub(crate) fn luminance(self) -> f64 {
        fn component(c: u8) -> f64 {
            let normalized = f64::from(c) / 255.0;
            if normalized <= 0.03928 {
                normalized / 12.92
            } else {
                ((normalized + 0.055) / 1.055).powf(2.4)
            }
        }

        0.2126 * component(self.r) + 0.7152 * component(self.g) + 0.0722 * component(self.b)
    }
}

/// Generate the extended 256-color palette from the base16 theme colors,
/// terminal background, and terminal foreground (upstream
/// `terminal.color.generate256Color`).
pub(crate) fn generate_256_color(
    base: Palette,
    skip: PaletteMask,
    bg: Rgb,
    fg: Rgb,
    harmonious: bool,
) -> Palette {
    let mut base8 = [
        Lab::from_rgb(bg),
        Lab::from_rgb(base[1]),
        Lab::from_rgb(base[2]),
        Lab::from_rgb(base[3]),
        Lab::from_rgb(base[4]),
        Lab::from_rgb(base[5]),
        Lab::from_rgb(base[6]),
        Lab::from_rgb(fg),
    ];

    let is_light_theme = base8[7].l < base8[0].l;
    if is_light_theme && !harmonious {
        base8.swap(0, 7);
    }

    let mut result = base;
    let mut idx = 16usize;
    for ri in 0..6 {
        let tr = ri as f32 / 5.0;
        let c0 = Lab::lerp(tr, base8[0], base8[1]);
        let c1 = Lab::lerp(tr, base8[2], base8[3]);
        let c2 = Lab::lerp(tr, base8[4], base8[5]);
        let c3 = Lab::lerp(tr, base8[6], base8[7]);
        for gi in 0..6 {
            let tg = gi as f32 / 5.0;
            let c4 = Lab::lerp(tg, c0, c1);
            let c5 = Lab::lerp(tg, c2, c3);
            for bi in 0..6 {
                if !skip.get(idx as u8) {
                    result[idx] = Lab::lerp(bi as f32 / 5.0, c4, c5).to_rgb();
                }
                idx += 1;
            }
        }
    }

    for i in 0..24 {
        if !skip.get(idx as u8) {
            result[idx] = Lab::lerp((i + 1) as f32 / 25.0, base8[0], base8[7]).to_rgb();
        }
        idx += 1;
    }

    result
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Lab {
    l: f32,
    a: f32,
    b: f32,
}

impl Lab {
    fn from_rgb(rgb: Rgb) -> Self {
        let mut r = f32::from(rgb.r) / 255.0;
        let mut g = f32::from(rgb.g) / 255.0;
        let mut b = f32::from(rgb.b) / 255.0;

        r = if r > 0.04045 {
            ((r + 0.055) / 1.055).powf(2.4)
        } else {
            r / 12.92
        };
        g = if g > 0.04045 {
            ((g + 0.055) / 1.055).powf(2.4)
        } else {
            g / 12.92
        };
        b = if b > 0.04045 {
            ((b + 0.055) / 1.055).powf(2.4)
        } else {
            b / 12.92
        };

        let mut x = (r * 0.4124564 + g * 0.3575761 + b * 0.1804375) / 0.95047;
        let mut y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
        let mut z = (r * 0.0193339 + g * 0.1191920 + b * 0.9503041) / 1.08883;

        x = if x > 0.008856 {
            x.cbrt()
        } else {
            7.787 * x + 16.0 / 116.0
        };
        y = if y > 0.008856 {
            y.cbrt()
        } else {
            7.787 * y + 16.0 / 116.0
        };
        z = if z > 0.008856 {
            z.cbrt()
        } else {
            7.787 * z + 16.0 / 116.0
        };

        Self {
            l: 116.0 * y - 16.0,
            a: 500.0 * (x - y),
            b: 200.0 * (y - z),
        }
    }

    fn to_rgb(self) -> Rgb {
        let y = (self.l + 16.0) / 116.0;
        let x = self.a / 500.0 + y;
        let z = y - self.b / 200.0;

        let x3 = x * x * x;
        let y3 = y * y * y;
        let z3 = z * z * z;
        let xf = if x3 > 0.008856 {
            x3
        } else {
            (x - 16.0 / 116.0) / 7.787
        } * 0.95047;
        let yf = if y3 > 0.008856 {
            y3
        } else {
            (y - 16.0 / 116.0) / 7.787
        };
        let zf = if z3 > 0.008856 {
            z3
        } else {
            (z - 16.0 / 116.0) / 7.787
        } * 1.08883;

        let mut r = xf * 3.2404542 - yf * 1.5371385 - zf * 0.4985314;
        let mut g = -xf * 0.9692660 + yf * 1.8760108 + zf * 0.0415560;
        let mut b = xf * 0.0556434 - yf * 0.2040259 + zf * 1.0572252;

        r = if r > 0.0031308 {
            1.055 * r.powf(1.0 / 2.4) - 0.055
        } else {
            12.92 * r
        };
        g = if g > 0.0031308 {
            1.055 * g.powf(1.0 / 2.4) - 0.055
        } else {
            12.92 * g
        };
        b = if b > 0.0031308 {
            1.055 * b.powf(1.0 / 2.4) - 0.055
        } else {
            12.92 * b
        };

        Rgb::new(to_u8(r), to_u8(g), to_u8(b))
    }

    fn lerp(t: f32, a: Lab, b: Lab) -> Lab {
        Lab {
            l: a.l + t * (b.l - a.l),
            a: a.a + t * (b.a - a.a),
            b: a.b + t * (b.b - a.b),
        }
    }
}

fn to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn parse_hash_rgb(hex: &[u8]) -> Option<Rgb> {
    let width = match hex.len() {
        3 => 1,
        6 => 2,
        9 => 3,
        12 => 4,
        _ => return None,
    };
    Some(Rgb::new(
        parse_hex_channel(&hex[..width])?,
        parse_hex_channel(&hex[width..width * 2])?,
        parse_hex_channel(&hex[width * 2..])?,
    ))
}

fn parse_component(bytes: &[u8], use_intensity: bool) -> Option<u8> {
    if use_intensity {
        parse_intensity_channel(bytes)
    } else {
        parse_hex_channel(bytes)
    }
}

fn parse_hex_channel(bytes: &[u8]) -> Option<u8> {
    if !(1..=4).contains(&bytes.len()) {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let value = u16::from_str_radix(text, 16).ok()? as u32;
    let max = match bytes.len() {
        1 => 0x0f,
        2 => 0xff,
        3 => 0x0fff,
        4 => 0xffff,
        _ => return None,
    };
    Some(((value * 0xff) / max) as u8)
}

fn parse_intensity_channel(bytes: &[u8]) -> Option<u8> {
    let text = std::str::from_utf8(bytes).ok()?;
    let value = text.parse::<f64>().ok()?;
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return None;
    }
    Some((value * 255.0) as u8)
}

impl DynamicRgb {
    pub(super) const fn unset() -> Self {
        Self {
            override_rgb: None,
            default_rgb: None,
        }
    }

    pub(super) const fn init(default: Rgb) -> Self {
        Self {
            override_rgb: None,
            default_rgb: Some(default),
        }
    }

    pub(super) const fn get(self) -> Option<Rgb> {
        match self.override_rgb {
            Some(rgb) => Some(rgb),
            None => self.default_rgb,
        }
    }

    pub(super) fn set(&mut self, rgb: Rgb) {
        self.override_rgb = Some(rgb);
    }

    pub(super) fn set_default(&mut self, rgb: Option<Rgb>) {
        self.default_rgb = rgb;
    }

    pub(super) const fn default_rgb(self) -> Option<Rgb> {
        self.default_rgb
    }

    pub(super) fn reset(&mut self) {
        self.override_rgb = self.default_rgb;
    }
}

impl DynamicPalette {
    pub(super) const fn init(default: Palette) -> Self {
        Self {
            current: default,
            original: default,
            mask: PaletteMask::empty(),
        }
    }

    pub(super) const fn current(&self) -> &Palette {
        &self.current
    }

    pub(super) const fn original(&self) -> &Palette {
        &self.original
    }

    pub(super) fn set(&mut self, index: u8, rgb: Rgb) {
        self.current[index as usize] = rgb;
        self.mask.set(index);
    }

    pub(super) fn reset(&mut self, index: u8) {
        self.current[index as usize] = self.original[index as usize];
        self.mask.unset(index);
    }

    pub(super) fn reset_all(&mut self) {
        *self = Self::init(self.original);
    }

    pub(super) fn change_default(&mut self, default: Palette) {
        self.original = default;
        if self.mask.is_empty() {
            self.current = self.original;
            return;
        }

        let old_current = self.current;
        let mut current = default;
        for index in self.mask.iter_set() {
            current[index as usize] = old_current[index as usize];
        }
        self.current = current;
    }
}

impl PaletteMask {
    pub(crate) const fn empty() -> Self {
        Self { words: [0; 4] }
    }

    pub(crate) fn is_empty(self) -> bool {
        self.words.iter().all(|word| *word == 0)
    }

    pub(crate) fn set(&mut self, index: u8) {
        let index = index as usize;
        self.words[index / 64] |= 1 << (index % 64);
    }

    fn unset(&mut self, index: u8) {
        let index = index as usize;
        self.words[index / 64] &= !(1 << (index % 64));
    }

    pub(crate) fn get(self, index: u8) -> bool {
        let index = index as usize;
        self.words[index / 64] & (1 << (index % 64)) != 0
    }

    fn iter_set(self) -> impl Iterator<Item = u8> {
        (0u16..256).filter_map(move |index| {
            let index = index as u8;
            self.get(index).then_some(index)
        })
    }
}

pub(crate) const DEFAULT_PALETTE: Palette = default_palette();

const fn default_palette() -> Palette {
    let mut result = [Rgb::new(0, 0, 0); 256];
    let mut i = 0;
    while i < 16 {
        result[i] = default_named(i as u8);
        i += 1;
    }

    let mut r = 0;
    while r < 6 {
        let mut g = 0;
        while g < 6 {
            let mut b = 0;
            while b < 6 {
                result[i] = Rgb::new(cube_value(r), cube_value(g), cube_value(b));
                i += 1;
                b += 1;
            }
            g += 1;
        }
        r += 1;
    }

    i = 232;
    while i < 256 {
        let value = ((i - 232) * 10 + 8) as u8;
        result[i] = Rgb::new(value, value, value);
        i += 1;
    }

    result
}

const fn cube_value(value: usize) -> u8 {
    if value == 0 {
        0
    } else {
        (value as u8) * 40 + 55
    }
}

const fn default_named(index: u8) -> Rgb {
    match index {
        0 => Rgb::new(0x1d, 0x1f, 0x21),
        1 => Rgb::new(0xcc, 0x66, 0x66),
        2 => Rgb::new(0xb5, 0xbd, 0x68),
        3 => Rgb::new(0xf0, 0xc6, 0x74),
        4 => Rgb::new(0x81, 0xa2, 0xbe),
        5 => Rgb::new(0xb2, 0x94, 0xbb),
        6 => Rgb::new(0x8a, 0xbe, 0xb7),
        7 => Rgb::new(0xc5, 0xc8, 0xc6),
        8 => Rgb::new(0x66, 0x66, 0x66),
        9 => Rgb::new(0xd5, 0x4e, 0x53),
        10 => Rgb::new(0xb9, 0xca, 0x4a),
        11 => Rgb::new(0xe7, 0xc5, 0x47),
        12 => Rgb::new(0x7a, 0xa6, 0xda),
        13 => Rgb::new(0xc3, 0x97, 0xd8),
        14 => Rgb::new(0x70, 0xc0, 0xb1),
        15 => Rgb::new(0xea, 0xea, 0xea),
        _ => panic!("only the first 16 palette entries have named defaults"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    #[test]
    fn rgb_c_conversion() {
        let rgb = Rgb::new(1, 2, 3);
        let c = rgb.cval();

        assert_eq!(Rgb::from_c(c), rgb);
    }

    #[test]
    fn rgb_c_layout() {
        assert_eq!(size_of::<CRgb>(), 3);
        assert_eq!(align_of::<CRgb>(), 1);
    }

    #[test]
    fn dynamic_rgb_set_reset_and_unset() {
        let mut rgb = DynamicRgb::unset();
        assert_eq!(rgb.get(), None);

        rgb.set(Rgb::new(1, 2, 3));
        assert_eq!(rgb.get(), Some(Rgb::new(1, 2, 3)));

        rgb.reset();
        assert_eq!(rgb.get(), None);
    }

    #[test]
    fn dynamic_rgb_reset_restores_default() {
        let mut rgb = DynamicRgb::init(Rgb::new(4, 5, 6));
        assert_eq!(rgb.get(), Some(Rgb::new(4, 5, 6)));

        rgb.set(Rgb::new(7, 8, 9));
        assert_eq!(rgb.get(), Some(Rgb::new(7, 8, 9)));

        rgb.reset();
        assert_eq!(rgb.get(), Some(Rgb::new(4, 5, 6)));
    }

    #[test]
    fn dynamic_palette_init() {
        let palette = DynamicPalette::init(DEFAULT_PALETTE);

        assert_eq!(palette.current(), &DEFAULT_PALETTE);
        assert_eq!(palette.original(), &DEFAULT_PALETTE);
        assert!(palette.mask.is_empty());
    }

    #[test]
    fn dynamic_palette_set() {
        let mut palette = DynamicPalette::init(DEFAULT_PALETTE);

        palette.set(3, Rgb::new(1, 2, 3));

        assert_eq!(palette.current()[3], Rgb::new(1, 2, 3));
        assert_eq!(palette.original()[3], DEFAULT_PALETTE[3]);
        assert!(palette.mask.get(3));
    }

    #[test]
    fn dynamic_palette_reset() {
        let mut palette = DynamicPalette::init(DEFAULT_PALETTE);
        palette.set(3, Rgb::new(1, 2, 3));

        palette.reset(3);

        assert_eq!(palette.current()[3], DEFAULT_PALETTE[3]);
        assert_eq!(palette.original()[3], DEFAULT_PALETTE[3]);
        assert!(!palette.mask.get(3));
    }

    #[test]
    fn dynamic_palette_reset_all() {
        let mut palette = DynamicPalette::init(DEFAULT_PALETTE);
        palette.set(3, Rgb::new(1, 2, 3));
        palette.set(4, Rgb::new(4, 5, 6));

        palette.reset_all();

        assert_eq!(palette.current(), &DEFAULT_PALETTE);
        assert_eq!(palette.original(), &DEFAULT_PALETTE);
        assert!(palette.mask.is_empty());
    }

    #[test]
    fn dynamic_palette_change_default_with_no_changes() {
        let mut palette = DynamicPalette::init(DEFAULT_PALETTE);
        let mut new_palette = DEFAULT_PALETTE;
        new_palette[3] = Rgb::new(1, 2, 3);

        palette.change_default(new_palette);

        assert_eq!(palette.current(), &new_palette);
        assert_eq!(palette.original(), &new_palette);
    }

    #[test]
    fn dynamic_palette_change_default_preserves_one_changed_entry() {
        let mut palette = DynamicPalette::init(DEFAULT_PALETTE);
        let override_rgb = Rgb::new(1, 2, 3);
        palette.set(3, override_rgb);
        let mut new_palette = DEFAULT_PALETTE;
        new_palette[3] = Rgb::new(4, 5, 6);
        new_palette[4] = Rgb::new(7, 8, 9);

        palette.change_default(new_palette);

        assert_eq!(palette.current()[3], override_rgb);
        assert_eq!(palette.current()[4], new_palette[4]);
        assert_eq!(palette.original(), &new_palette);
        assert!(palette.mask.get(3));
    }

    #[test]
    fn dynamic_palette_change_default_preserves_multiple_changed_entries() {
        let mut palette = DynamicPalette::init(DEFAULT_PALETTE);
        let override_three = Rgb::new(1, 2, 3);
        let override_four = Rgb::new(4, 5, 6);
        palette.set(3, override_three);
        palette.set(4, override_four);
        let mut new_palette = DEFAULT_PALETTE;
        new_palette[3] = Rgb::new(7, 8, 9);
        new_palette[4] = Rgb::new(10, 11, 12);
        new_palette[5] = Rgb::new(13, 14, 15);

        palette.change_default(new_palette);

        assert_eq!(palette.current()[3], override_three);
        assert_eq!(palette.current()[4], override_four);
        assert_eq!(palette.current()[5], new_palette[5]);
        assert_eq!(palette.original(), &new_palette);
        assert!(palette.mask.get(3));
        assert!(palette.mask.get(4));
    }

    #[test]
    fn rgb_parse_hex_forms() {
        assert_eq!(Rgb::parse(b"rgb:f/ff/fff"), Some(Rgb::new(255, 255, 255)));
        assert_eq!(Rgb::parse(b"rgb:7f/a0a0/0"), Some(Rgb::new(127, 160, 0)));
        assert_eq!(Rgb::parse(b"#fff"), Some(Rgb::new(255, 255, 255)));
        assert_eq!(Rgb::parse(b"#ffffff"), Some(Rgb::new(255, 255, 255)));
        assert_eq!(Rgb::parse(b"#fffffffff"), Some(Rgb::new(255, 255, 255)));
        assert_eq!(Rgb::parse(b"#ffffffffffff"), Some(Rgb::new(255, 255, 255)));
        assert_eq!(Rgb::parse(b"#ff0010"), Some(Rgb::new(255, 0, 16)));
    }

    #[test]
    fn rgb_parse_rgbi_intensity_forms() {
        assert_eq!(Rgb::parse(b"rgbi:1.0/0/0"), Some(Rgb::new(255, 0, 0)));
        assert_eq!(Rgb::parse(b"rgbi:0.5/0.25/0"), Some(Rgb::new(127, 63, 0)));
    }

    #[test]
    fn rgb_parse_x11_named_colors() {
        assert_eq!(Rgb::parse(b"red"), Some(Rgb::new(255, 0, 0)));
        assert_eq!(Rgb::parse(b"white"), Some(Rgb::new(255, 255, 255)));
        assert_eq!(Rgb::parse(b"black"), Some(Rgb::new(0, 0, 0)));
        assert_eq!(Rgb::parse(b"blue"), Some(Rgb::new(0, 0, 255)));
        assert_eq!(
            Rgb::parse(b"medium spring green"),
            Some(Rgb::new(0, 250, 154))
        );
        assert_eq!(
            Rgb::parse(b"mediumspringgreen"),
            Some(Rgb::new(0, 250, 154))
        );
        assert_eq!(Rgb::parse(b"ForestGreen"), Some(Rgb::new(34, 139, 34)));
        assert_eq!(Rgb::parse(b"FoReStGReen"), Some(Rgb::new(34, 139, 34)));
        assert_eq!(Rgb::parse(b" red "), Some(Rgb::new(255, 0, 0)));
    }

    #[test]
    fn rgb_parse_rejects_invalid_forms() {
        assert_eq!(Rgb::parse(b""), None);
        assert_eq!(Rgb::parse(b"RGB:f/0/0"), None);
        assert_eq!(Rgb::parse(b"RGBI:1.0/0/0"), None);
        assert_eq!(Rgb::parse(b"rgb:"), None);
        assert_eq!(Rgb::parse(b"rgb:a/a/a/"), None);
        assert_eq!(Rgb::parse(b"rgb:00000///"), None);
        assert_eq!(Rgb::parse(b"rgb:000/"), None);
        assert_eq!(Rgb::parse(b"rgbi:a/a/a"), None);
        assert_eq!(Rgb::parse(b"rgbi:-0.1/0/0"), None);
        assert_eq!(Rgb::parse(b"rgbi:1.1/0/0"), None);
        assert_eq!(Rgb::parse(b"rgbi:NaN/0/0"), None);
        assert_eq!(Rgb::parse(b"rgbi:inf/0/0"), None);
        assert_eq!(Rgb::parse(b"rgbi:0.5/0.0/1.0/0"), None);
        assert_eq!(Rgb::parse(b"rgb:0.5/0.0/1.0"), None);
        assert_eq!(Rgb::parse(b"rgb:not/hex/zz"), None);
        assert_eq!(Rgb::parse(b"#"), None);
        assert_eq!(Rgb::parse(b"#ff"), None);
        assert_eq!(Rgb::parse(b"#ffff"), None);
        assert_eq!(Rgb::parse(b"#fffff"), None);
        assert_eq!(Rgb::parse(b"#gggggg"), None);
        assert_eq!(Rgb::parse(b"\tred\t"), None);
        assert_eq!(Rgb::parse(b"\nred\n"), None);
        assert_eq!(Rgb::parse(b"nosuchcolor"), None);
    }

    #[test]
    fn default_palette_named_entries() {
        assert_eq!(DEFAULT_PALETTE[1], Rgb::new(204, 102, 102));
        assert_eq!(DEFAULT_PALETTE[2], Rgb::new(181, 189, 104));
        assert_eq!(DEFAULT_PALETTE[3], Rgb::new(240, 198, 116));
        assert_eq!(DEFAULT_PALETTE[7], Rgb::new(197, 200, 198));
    }

    #[test]
    fn default_palette_cube_and_grayscale_entries() {
        assert_eq!(DEFAULT_PALETTE[16], Rgb::new(0, 0, 0));
        assert_eq!(DEFAULT_PALETTE[17], Rgb::new(0, 0, 95));
        assert_eq!(DEFAULT_PALETTE[21], Rgb::new(0, 0, 255));
        assert_eq!(DEFAULT_PALETTE[232], Rgb::new(8, 8, 8));
        assert_eq!(DEFAULT_PALETTE[255], Rgb::new(238, 238, 238));
    }

    fn assert_palette_samples(palette: Palette, expected: &[(usize, Rgb)]) {
        for (index, rgb) in expected {
            assert_eq!(palette[*index], *rgb, "palette index {index}");
        }
    }

    #[test]
    fn generate_256_color_dark_theme_matches_upstream_samples() {
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        let palette =
            generate_256_color(DEFAULT_PALETTE, PaletteMask::empty(), black, white, false);

        for index in 0..16 {
            assert_eq!(palette[index], DEFAULT_PALETTE[index]);
        }
        assert_eq!(palette[16], black);
        assert_eq!(palette[231], white);
        assert_palette_samples(
            palette,
            &[
                (17, Rgb::new(0x1e, 0x22, 0x27)),
                (52, Rgb::new(0x2a, 0x19, 0x19)),
                (88, Rgb::new(0x4e, 0x2b, 0x2a)),
                (100, Rgb::new(0x7f, 0x64, 0x44)),
                (160, Rgb::new(0xa0, 0x52, 0x51)),
                (232, Rgb::new(0x0e, 0x0e, 0x0e)),
                (240, Rgb::new(0x55, 0x55, 0x55)),
                (255, Rgb::new(0xf3, 0xf3, 0xf3)),
            ],
        );

        let mut previous = 0.0;
        for rgb in &palette[232..256] {
            let luminance = rgb.luminance();
            assert!(luminance >= previous);
            previous = luminance;
        }
    }

    #[test]
    fn generate_256_color_light_theme_inverts_without_harmonious() {
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        let palette =
            generate_256_color(DEFAULT_PALETTE, PaletteMask::empty(), white, black, false);

        assert_eq!(palette[16], black);
        assert_eq!(palette[231], white);
        assert_palette_samples(
            palette,
            &[
                (17, Rgb::new(0x1e, 0x22, 0x27)),
                (52, Rgb::new(0x2a, 0x19, 0x19)),
                (88, Rgb::new(0x4e, 0x2b, 0x2a)),
                (100, Rgb::new(0x7f, 0x64, 0x44)),
                (160, Rgb::new(0xa0, 0x52, 0x51)),
                (232, Rgb::new(0x0e, 0x0e, 0x0e)),
                (240, Rgb::new(0x55, 0x55, 0x55)),
                (255, Rgb::new(0xf3, 0xf3, 0xf3)),
            ],
        );
    }

    #[test]
    fn generate_256_color_harmonious_light_theme_keeps_orientation() {
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        let palette = generate_256_color(DEFAULT_PALETTE, PaletteMask::empty(), white, black, true);

        assert_eq!(palette[16], white);
        assert_eq!(palette[231], black);
        assert_palette_samples(
            palette,
            &[
                (17, Rgb::new(0xe6, 0xec, 0xf2)),
                (52, Rgb::new(0xf8, 0xe0, 0xdf)),
                (88, Rgb::new(0xf0, 0xc1, 0xbf)),
                (100, Rgb::new(0xe3, 0xc1, 0x9e)),
                (160, Rgb::new(0xda, 0x85, 0x83)),
                (232, Rgb::new(0xf3, 0xf3, 0xf3)),
                (240, Rgb::new(0x9b, 0x9b, 0x9b)),
                (255, Rgb::new(0x0e, 0x0e, 0x0e)),
            ],
        );

        let mut previous = 1.0;
        for rgb in &palette[232..256] {
            let luminance = rgb.luminance();
            assert!(luminance <= previous);
            previous = luminance;
        }
    }

    #[test]
    fn generate_256_color_skip_mask_preserves_user_entries() {
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        let mut skip = PaletteMask::empty();
        skip.set(20);
        skip.set(100);
        skip.set(240);
        let palette = generate_256_color(DEFAULT_PALETTE, skip, black, white, false);

        assert_eq!(palette[20], DEFAULT_PALETTE[20]);
        assert_eq!(palette[100], DEFAULT_PALETTE[100]);
        assert_eq!(palette[240], DEFAULT_PALETTE[240]);
        assert_palette_samples(
            palette,
            &[
                (17, Rgb::new(0x1e, 0x22, 0x27)),
                (52, Rgb::new(0x2a, 0x19, 0x19)),
                (88, Rgb::new(0x4e, 0x2b, 0x2a)),
                (100, Rgb::new(0x87, 0x87, 0x00)),
                (160, Rgb::new(0xa0, 0x52, 0x51)),
                (232, Rgb::new(0x0e, 0x0e, 0x0e)),
                (240, Rgb::new(0x58, 0x58, 0x58)),
                (255, Rgb::new(0xf3, 0xf3, 0xf3)),
            ],
        );
    }
}
