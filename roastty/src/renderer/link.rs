//! Renderer-side link highlighting (port of upstream `renderer/link.zig`).

use crate::input::key_mods::Mods;
use crate::input::link;
use crate::terminal::point;
use crate::terminal::string_map::ViewportStringMap;

#[derive(Debug)]
struct RendererLink {
    regex: onig::Regex,
    highlight: link::Highlight,
}

/// A compiled renderer link set. Regex compile failures are non-fatal in
/// Roastty's live reload path: bad patterns are skipped and rendering continues.
#[derive(Debug, Default)]
pub(crate) struct RendererLinkSet {
    config: Vec<link::Link>,
    links: Vec<RendererLink>,
}

impl RendererLinkSet {
    pub(crate) fn sync_from_config(&mut self, config: &[link::Link]) {
        if self.config == config {
            return;
        }

        self.config = config.to_vec();
        self.links.clear();
        for link in config {
            let pattern = match std::str::from_utf8(&link.regex) {
                Ok(pattern) => pattern,
                Err(err) => {
                    eprintln!("[roastty] invalid link regex utf8: {err}");
                    continue;
                }
            };
            match onig::Regex::new(pattern) {
                Ok(regex) => self.links.push(RendererLink {
                    regex,
                    highlight: link.highlight,
                }),
                Err(err) => {
                    eprintln!("[roastty] invalid link regex {pattern:?}: {err}");
                }
            }
        }
    }

    pub(crate) fn render_ranges(
        &self,
        map: &ViewportStringMap,
        rows: usize,
        mouse_viewport: Option<point::Coordinate>,
        mouse_mods: Mods,
    ) -> Vec<Vec<[u16; 2]>> {
        let mut cells = Vec::new();

        for link in &self.links {
            if !highlight_condition_may_match(link.highlight, mouse_viewport, mouse_mods) {
                continue;
            }

            for (start, end) in link.regex.find_iter(&map.string) {
                if end <= start || end > map.map.len() {
                    continue;
                }

                let matched = &map.map[start..end];
                if !highlight_match_applies(link.highlight, matched, mouse_viewport, mouse_mods) {
                    continue;
                }
                cells.extend_from_slice(matched);
            }
        }

        ranges_from_cells(rows, cells)
    }
}

pub(crate) fn ranges_from_cells(
    rows: usize,
    cells: impl IntoIterator<Item = point::Coordinate>,
) -> Vec<Vec<[u16; 2]>> {
    let mut by_row = vec![Vec::new(); rows];
    for cell in cells {
        let row = cell.y as usize;
        if row >= rows {
            continue;
        }
        by_row[row].push(cell.x);
    }

    for row in &mut by_row {
        row.sort_unstable();
        row.dedup();
    }

    by_row
        .into_iter()
        .map(|cols| {
            let mut ranges = Vec::new();
            let mut iter = cols.into_iter();
            let Some(mut start) = iter.next() else {
                return ranges;
            };
            let mut end = start;
            for col in iter {
                if col == end.saturating_add(1) {
                    end = col;
                } else {
                    ranges.push([start, end]);
                    start = col;
                    end = col;
                }
            }
            ranges.push([start, end]);
            ranges
        })
        .collect()
}

pub(crate) fn merge_ranges(
    mut base: Vec<Vec<[u16; 2]>>,
    extra: &[Vec<[u16; 2]>],
) -> Vec<Vec<[u16; 2]>> {
    for (row, ranges) in extra.iter().enumerate() {
        if row >= base.len() {
            break;
        }
        for &[start, end] in ranges {
            base[row].extend((start..=end).map(|col| [col, col]));
        }
    }

    base.into_iter()
        .map(|ranges| {
            let cells = ranges
                .into_iter()
                .flat_map(|[start, end]| start..=end)
                .map(|x| point::Coordinate::new(x, 0));
            ranges_from_cells(1, cells)
                .into_iter()
                .next()
                .unwrap_or_default()
        })
        .collect()
}

fn highlight_condition_may_match(
    highlight: link::Highlight,
    mouse_viewport: Option<point::Coordinate>,
    mouse_mods: Mods,
) -> bool {
    match highlight {
        link::Highlight::Always => true,
        link::Highlight::AlwaysMods(mods) => mouse_mods == mods,
        link::Highlight::Hover => mouse_viewport.is_some(),
        link::Highlight::HoverMods(mods) => mouse_viewport.is_some() && mouse_mods == mods,
    }
}

fn highlight_match_applies(
    highlight: link::Highlight,
    matched: &[point::Coordinate],
    mouse_viewport: Option<point::Coordinate>,
    mouse_mods: Mods,
) -> bool {
    match highlight {
        link::Highlight::Always => true,
        link::Highlight::AlwaysMods(mods) => mouse_mods == mods,
        link::Highlight::Hover => mouse_viewport.is_some_and(|mouse| matched.contains(&mouse)),
        link::Highlight::HoverMods(mods) => {
            mouse_mods == mods && mouse_viewport.is_some_and(|mouse| matched.contains(&mouse))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::input::key_mods;
    use crate::input::link::{Action, Highlight, Link};

    fn coord(x: u16, y: u32) -> point::Coordinate {
        point::Coordinate::new(x, y)
    }

    fn map(text: &str, y: u32) -> ViewportStringMap {
        let coords = text
            .as_bytes()
            .iter()
            .enumerate()
            .map(|(x, _)| coord(x as u16, y))
            .collect();
        ViewportStringMap::new(text.to_string(), coords)
    }

    fn link(regex: &str, highlight: Highlight) -> Link {
        Link {
            regex: regex.as_bytes().to_vec(),
            action: Action::Open,
            highlight,
        }
    }

    #[test]
    fn renderer_link_ranges_merge_contiguous_cells() {
        let ranges = ranges_from_cells(2, [coord(3, 0), coord(1, 0), coord(2, 0), coord(7, 1)]);
        assert_eq!(ranges[0], vec![[1, 3]]);
        assert_eq!(ranges[1], vec![[7, 7]]);
    }

    #[test]
    fn renderer_link_always_match_produces_ranges() {
        let mut set = RendererLinkSet::default();
        set.sync_from_config(&[link("AB", Highlight::Always)]);
        let ranges = set.render_ranges(&map("1ABCD", 0), 1, None, Mods::new());
        assert_eq!(ranges[0], vec![[1, 2]]);
    }

    #[test]
    fn renderer_link_hover_requires_mouse_inside_match() {
        let mut set = RendererLinkSet::default();
        set.sync_from_config(&[link("AB", Highlight::Hover)]);
        assert!(
            set.render_ranges(&map("1ABCD", 0), 1, Some(coord(4, 0)), Mods::new())[0].is_empty()
        );
        assert_eq!(
            set.render_ranges(&map("1ABCD", 0), 1, Some(coord(1, 0)), Mods::new())[0],
            vec![[1, 2]]
        );
    }

    #[test]
    fn renderer_link_mods_require_exact_match() {
        let ctrl = key_mods::Mods {
            ctrl: true,
            ..Mods::new()
        };
        let shift = key_mods::Mods {
            shift: true,
            ..Mods::new()
        };
        let mut set = RendererLinkSet::default();
        set.sync_from_config(&[link("AB", Highlight::AlwaysMods(ctrl))]);
        assert!(set.render_ranges(&map("1ABCD", 0), 1, None, shift)[0].is_empty());
        assert_eq!(
            set.render_ranges(&map("1ABCD", 0), 1, None, ctrl)[0],
            vec![[1, 2]]
        );
    }

    #[test]
    fn default_url_link_regex_compiles_and_matches() {
        let config = Config::default();
        let mut set = RendererLinkSet::default();
        set.sync_from_config(&config.link);
        let text = "go https://example.com now";
        let ranges = set.render_ranges(
            &map(text, 0),
            1,
            Some(coord(3, 0)),
            key_mods::ctrl_or_super(Mods::new()),
        );
        assert_eq!(ranges[0], vec![[3, 21]]);
    }

    #[test]
    fn invalid_regex_is_skipped() {
        let mut set = RendererLinkSet::default();
        set.sync_from_config(&[link("(", Highlight::Always), link("OK", Highlight::Always)]);
        let ranges = set.render_ranges(&map("OK", 0), 1, None, Mods::new());
        assert_eq!(ranges[0], vec![[0, 1]]);
    }
}
