//! Terminal size report encoding.

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Size {
    pub(crate) rows: u16,
    pub(crate) columns: u16,
    pub(crate) cell_width: u32,
    pub(crate) cell_height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Mode2048,
    Csi14T,
    Csi16T,
    Csi18T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Request {
    Csi14T,
    Csi16T,
    Csi18T,
    Csi21T,
}

pub(crate) fn style_from_int(value: i32) -> Option<Style> {
    match value {
        0 => Some(Style::Mode2048),
        1 => Some(Style::Csi14T),
        2 => Some(Style::Csi16T),
        3 => Some(Style::Csi18T),
        _ => None,
    }
}

pub(crate) fn encode(style: Style, size: Size) -> Vec<u8> {
    let width_px = u64::from(size.columns).saturating_mul(u64::from(size.cell_width));
    let height_px = u64::from(size.rows).saturating_mul(u64::from(size.cell_height));

    match style {
        Style::Mode2048 => {
            format!(
                "\x1b[48;{};{};{};{}t",
                size.rows, size.columns, height_px, width_px
            )
        }
        Style::Csi14T => format!("\x1b[4;{height_px};{width_px}t"),
        Style::Csi16T => format!("\x1b[6;{};{}t", size.cell_height, size.cell_width),
        Style::Csi18T => format!("\x1b[8;{};{}t", size.rows, size.columns),
    }
    .into_bytes()
}

impl Request {
    pub(crate) fn report_style(self) -> Option<Style> {
        match self {
            Request::Csi14T => Some(Style::Csi14T),
            Request::Csi16T => Some(Style::Csi16T),
            Request::Csi18T => Some(Style::Csi18T),
            Request::Csi21T => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_size() -> Size {
        Size {
            rows: 24,
            columns: 80,
            cell_width: 9,
            cell_height: 18,
        }
    }

    #[test]
    fn size_report_encodes_all_styles() {
        assert_eq!(
            encode(Style::Mode2048, test_size()),
            b"\x1b[48;24;80;432;720t"
        );
        assert_eq!(encode(Style::Csi14T, test_size()), b"\x1b[4;432;720t");
        assert_eq!(encode(Style::Csi16T, test_size()), b"\x1b[6;18;9t");
        assert_eq!(encode(Style::Csi18T, test_size()), b"\x1b[8;24;80t");
    }

    #[test]
    fn size_report_encodes_max_values_without_panicking() {
        let max = Size {
            rows: u16::MAX,
            columns: u16::MAX,
            cell_width: u32::MAX,
            cell_height: u32::MAX,
        };

        assert_eq!(
            encode(Style::Mode2048, max),
            b"\x1b[48;65535;65535;281470681677825;281470681677825t"
        );
    }
}
