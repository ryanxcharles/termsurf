//! Byte-oriented fuzzy ranking and highlighting.
//!
//! Rust port of the core `zf` library used by Ghostty's theme-list filtering.
//! The lower a rank is, the better the match.

use std::path::MAIN_SEPARATOR as PATH_SEPARATOR;

const SEP: u8 = PATH_SEPARATOR as u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RankOptions {
    pub case_sensitive: bool,
    pub plain: bool,
}

impl Default for RankOptions {
    fn default() -> Self {
        RankOptions {
            case_sensitive: true,
            plain: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RankNeedleOptions<'a> {
    pub case_sensitive: bool,
    pub strict_path: bool,
    pub filename: Option<&'a [u8]>,
}

impl Default for RankNeedleOptions<'_> {
    fn default() -> Self {
        RankNeedleOptions {
            case_sensitive: true,
            strict_path: false,
            filename: None,
        }
    }
}

pub(crate) fn rank(haystack: &[u8], needles: &[&[u8]], opts: RankOptions) -> Option<f64> {
    let filename = (!opts.plain).then(|| basename(haystack));

    let mut sum = 0.0;
    for needle in needles {
        let strict_path = !opts.plain && has_separator(needle);
        let rank = rank_needle(
            haystack,
            needle,
            RankNeedleOptions {
                case_sensitive: opts.case_sensitive,
                strict_path,
                filename,
            },
        )?;
        sum += rank;
    }

    Some(sum)
}

pub(crate) fn rank_needle(
    haystack: &[u8],
    needle: &[u8],
    opts: RankNeedleOptions<'_>,
) -> Option<f64> {
    if haystack.is_empty() || needle.is_empty() {
        return None;
    }

    if opts.strict_path {
        let mut best_rank = None;
        let mut start = 0;

        for segment in PathSegments::new(needle) {
            let first = *segment.first()?;
            let mut iter = IndexIterator::new(haystack, first, opts.case_sensitive);
            iter.index = start;

            let mut found = false;
            while let Some(start_index) = iter.next() {
                if let Some(scan) = scan_to_end(
                    haystack,
                    &segment[1..],
                    start_index,
                    0,
                    None,
                    opts.case_sensitive,
                    true,
                ) {
                    let path_segment_len = segment_len(haystack, start_index);
                    let coverage = 1.0 - (segment.len() as f64 / path_segment_len as f64);
                    let rank = coverage * scan.rank;
                    best_rank = Some(best_rank.unwrap_or(0.0) + rank);
                    start = scan.index;
                    found = true;
                    break;
                }
            }

            if !found {
                return None;
            }
        }

        return best_rank;
    }

    if let Some(filename) = opts.filename {
        let mut best_rank = None;
        let mut iter = IndexIterator::new(filename, needle[0], opts.case_sensitive);
        while let Some(start_index) = iter.next() {
            if let Some(scan) = scan_to_end(
                filename,
                &needle[1..],
                start_index,
                0,
                None,
                opts.case_sensitive,
                false,
            ) {
                if best_rank.is_none_or(|best| scan.rank < best) {
                    best_rank = Some(scan.rank);
                }
            } else {
                break;
            }
        }

        if let Some(mut rank) = best_rank {
            rank /= 2.0;
            if needle.len() == filename.len() {
                rank /= 2.0;
            } else {
                let coverage = 1.0 - (needle.len() as f64 / filename.len() as f64);
                rank *= coverage;
            }
            return Some(rank);
        }
    }

    let mut best_rank = None;
    let mut iter = IndexIterator::new(haystack, needle[0], opts.case_sensitive);
    while let Some(start_index) = iter.next() {
        if let Some(scan) = scan_to_end(
            haystack,
            &needle[1..],
            start_index,
            0,
            None,
            opts.case_sensitive,
            false,
        ) {
            if best_rank.is_none_or(|best| scan.rank < best) {
                best_rank = Some(scan.rank);
            }
        } else {
            break;
        }
    }

    best_rank
}

pub(crate) fn highlight(haystack: &[u8], needles: &[&[u8]], opts: RankOptions) -> Vec<usize> {
    let filename = (!opts.plain).then(|| basename(haystack));

    let mut matches = Vec::new();
    for needle in needles {
        let strict_path = !opts.plain && has_separator(needle);
        let mut matched = highlight_needle(
            haystack,
            needle,
            RankNeedleOptions {
                case_sensitive: opts.case_sensitive,
                strict_path,
                filename,
            },
        );
        matches.append(&mut matched);
    }

    if needles.len() > 1 {
        matches.sort_unstable();
        matches.dedup();
    }

    matches
}

pub(crate) fn highlight_needle(
    haystack: &[u8],
    needle: &[u8],
    opts: RankNeedleOptions<'_>,
) -> Vec<usize> {
    if haystack.is_empty() || needle.is_empty() {
        return Vec::new();
    }

    if opts.strict_path {
        let mut best_matched = Vec::new();
        let mut start = 0;

        for segment in PathSegments::new(needle) {
            let Some(&first) = segment.first() else {
                return Vec::new();
            };
            let mut iter = IndexIterator::new(haystack, first, opts.case_sensitive);
            iter.index = start;

            let mut found = false;
            while let Some(start_index) = iter.next() {
                let mut matched = vec![start_index];
                if let Some(scan) = scan_to_end(
                    haystack,
                    &segment[1..],
                    start_index,
                    0,
                    Some(&mut matched),
                    opts.case_sensitive,
                    true,
                ) {
                    best_matched.extend(matched);
                    start = scan.index;
                    found = true;
                    break;
                }
            }

            if !found {
                return Vec::new();
            }
        }

        return best_matched;
    }

    let mut best_rank = None;
    let mut best_matched = Vec::new();

    if let Some(filename) = opts.filename {
        let offset = haystack
            .len()
            .saturating_sub(filename.len())
            .saturating_sub(usize::from(haystack.last() == Some(&SEP)));

        let mut iter = IndexIterator::new(filename, needle[0], opts.case_sensitive);
        while let Some(start_index) = iter.next() {
            let mut matched = vec![start_index + offset];
            if let Some(scan) = scan_to_end(
                filename,
                &needle[1..],
                start_index,
                offset,
                Some(&mut matched),
                opts.case_sensitive,
                false,
            ) {
                if best_rank.is_none_or(|best| scan.rank < best) {
                    best_rank = Some(scan.rank);
                    best_matched = matched;
                }
            } else {
                break;
            }
        }

        if best_rank.is_some() {
            return best_matched;
        }
    }

    best_rank = None;
    let mut iter = IndexIterator::new(haystack, needle[0], opts.case_sensitive);
    while let Some(start_index) = iter.next() {
        let mut matched = vec![start_index];
        if let Some(scan) = scan_to_end(
            haystack,
            &needle[1..],
            start_index,
            0,
            Some(&mut matched),
            opts.case_sensitive,
            false,
        ) {
            if best_rank.is_none_or(|best| scan.rank < best) {
                best_rank = Some(scan.rank);
                best_matched = matched;
            }
        } else {
            break;
        }
    }

    best_matched
}

fn basename(path: &[u8]) -> &[u8] {
    if path.is_empty() {
        return path;
    }

    let end = if path.last() == Some(&SEP) && path.len() > 1 {
        path.len() - 1
    } else {
        path.len()
    };
    let path = &path[..end];
    match path.iter().rposition(|&byte| byte == SEP) {
        Some(index) => &path[index + 1..],
        None => path,
    }
}

fn has_separator(bytes: &[u8]) -> bool {
    bytes.contains(&SEP)
}

struct IndexIterator<'a> {
    bytes: &'a [u8],
    needle: u8,
    index: usize,
    case_sensitive: bool,
}

impl<'a> IndexIterator<'a> {
    fn new(bytes: &'a [u8], needle: u8, case_sensitive: bool) -> Self {
        IndexIterator {
            bytes,
            needle,
            index: 0,
            case_sensitive,
        }
    }
}

impl Iterator for IndexIterator<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let needle = if self.case_sensitive {
            self.needle
        } else {
            self.needle.to_ascii_lowercase()
        };

        while self.index < self.bytes.len() {
            let index = self.index;
            self.index += 1;
            let hay = if self.case_sensitive {
                self.bytes[index]
            } else {
                self.bytes[index].to_ascii_lowercase()
            };
            if hay == needle {
                return Some(index);
            }
        }

        None
    }
}

struct PathSegments<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> PathSegments<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        PathSegments { bytes, index: 0 }
    }
}

impl<'a> Iterator for PathSegments<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.bytes.len() {
            return None;
        }

        let start = self.index;
        if self.bytes[self.index] == SEP {
            self.index += 1;
            return Some(&self.bytes[start..self.index]);
        }

        while self.index < self.bytes.len() {
            if self.bytes[self.index] == SEP {
                return Some(&self.bytes[start..self.index]);
            }
            self.index += 1;
        }

        Some(&self.bytes[start..])
    }
}

fn segment_len(bytes: &[u8], index: usize) -> usize {
    if bytes[index] == SEP {
        return 1;
    }

    let mut start = index;
    let mut end = index;
    while start > 0 {
        if bytes[start - 1] == SEP {
            break;
        }
        start -= 1;
    }
    while end < bytes.len() && bytes[end] != SEP {
        end += 1;
    }

    end - start
}

fn is_start_of_word(byte: u8) -> bool {
    matches!(byte, b'/' | b'_' | b'-' | b'.' | b' ')
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ScanResult {
    rank: f64,
    index: usize,
}

fn scan_to_end(
    haystack: &[u8],
    needle: &[u8],
    start_index: usize,
    offset: usize,
    mut matched_indices: Option<&mut Vec<usize>>,
    case_sensitive: bool,
    strict_path: bool,
) -> Option<ScanResult> {
    let mut rank = 1.0;
    let mut last_index = start_index;
    let mut last_sequential = false;

    if start_index > 0 && !is_start_of_word(haystack[start_index - 1]) {
        rank += 2.0;
    }

    for &byte in needle {
        let mut iter = IndexIterator::new(haystack, byte, case_sensitive);
        iter.index = last_index + 1;
        let index = iter.next()?;

        if strict_path && has_separator(&haystack[last_index..=index]) {
            return None;
        }

        if let Some(matches) = matched_indices.as_deref_mut() {
            matches.push(index + offset);
        }

        if index == last_index + 1 {
            if !last_sequential {
                last_sequential = true;
                rank += 1.0;
            }
        } else {
            if !is_start_of_word(haystack[index - 1]) {
                rank += 2.0;
            }
            last_sequential = false;
            rank += (index - last_index) as f64;
        }

        last_index = index;
    }

    Some(ScanResult {
        rank,
        index: last_index + 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rank_bytes(haystack: &str, needle: &str, opts: RankNeedleOptions<'_>) -> Option<f64> {
        rank_needle(haystack.as_bytes(), needle.as_bytes(), opts)
    }

    fn case_insensitive_plain() -> RankOptions {
        RankOptions {
            case_sensitive: false,
            plain: true,
        }
    }

    #[test]
    fn zf_rank_library_interface_matches_upstream() {
        assert_eq!(
            rank(b"abcdefg", &[b"a", b"z"], RankOptions::default()),
            None
        );
        assert!(rank(b"abcdefg", &[b"a", b"b"], RankOptions::default()).is_some());
        assert_eq!(
            rank(b"abcdefg", &[b"a", b"B"], RankOptions::default()),
            None
        );
        assert!(rank(b"aBcdefg", &[b"a", b"B"], RankOptions::default()).is_some());
        assert_eq!(
            rank(b"a/path/to/file", &[b"zig"], RankOptions::default()),
            None
        );
        assert!(rank(
            b"a/path/to/file",
            &[b"path", b"file"],
            RankOptions::default()
        )
        .is_some());

        assert!(rank_needle(b"abcdefg", b"a", RankNeedleOptions::default()).is_some());
        assert_eq!(
            rank_needle(b"abcdefg", b"z", RankNeedleOptions::default()),
            None
        );
        assert!(rank_needle(b"abcdefG", b"G", RankNeedleOptions::default()).is_some());
        assert_eq!(
            rank_needle(b"abcdefg", b"A", RankNeedleOptions::default()),
            None
        );
        assert!(rank_needle(
            b"a/path/to/file",
            b"file",
            RankNeedleOptions {
                filename: Some(b"file"),
                ..RankNeedleOptions::default()
            },
        )
        .is_some());
        assert_eq!(
            rank_needle(
                b"a/path/to/file",
                b"zig",
                RankNeedleOptions {
                    filename: Some(b"file"),
                    ..RankNeedleOptions::default()
                },
            ),
            None
        );

        assert_eq!(rank(b"", &[b"a"], RankOptions::default()), None);
        assert_eq!(rank_needle(b"", b"a", RankNeedleOptions::default()), None);
        assert_eq!(rank(b"a", &[b""], RankOptions::default()), None);
        assert_eq!(rank_needle(b"a", b"", RankNeedleOptions::default()), None);
    }

    #[test]
    fn zf_filter_rank_needle_match_shapes_match_upstream() {
        let insensitive = RankNeedleOptions {
            case_sensitive: false,
            ..RankNeedleOptions::default()
        };

        assert_eq!(rank_needle(b"", b"", insensitive), None);
        assert_eq!(rank_needle(b"", b"b", insensitive), None);
        assert_eq!(rank_needle(b"a", b"", insensitive), None);
        assert_eq!(rank_needle(b"a", b"b", insensitive), None);
        assert_eq!(rank_needle(b"aaa", b"aab", insensitive), None);
        assert_eq!(rank_needle(b"abbba", b"abab", insensitive), None);

        assert!(rank_needle(b"a", b"a", insensitive).is_some());
        assert!(rank_needle(b"abc", b"abc", insensitive).is_some());
        assert!(rank_needle(b"aaabbbccc", b"abc", insensitive).is_some());
        assert!(rank_needle(b"azbycx", b"x", insensitive).is_some());
        assert!(rank_needle(b"azbycx", b"ax", insensitive).is_some());

        let filename = |name| RankNeedleOptions {
            case_sensitive: false,
            filename: Some(name),
            ..RankNeedleOptions::default()
        };
        assert_eq!(rank_needle(b"", b"", filename(b"")), None);
        assert_eq!(rank_needle(b"/a", b"b", filename(b"a")), None);
        assert_eq!(rank_needle(b"c/a", b"b", filename(b"a")), None);
        assert_eq!(rank_needle(b"/file.ext", b"z", filename(b"file.ext")), None);
        assert_eq!(
            rank_needle(b"/file.ext", b"fext.", filename(b"file.ext")),
            None
        );
        assert_eq!(rank_needle(b"/a/b/c", b"d", filename(b"c")), None);
        assert!(rank_needle(b"/b", b"b", filename(b"b")).is_some());
        assert!(rank_needle(b"/a/b/c", b"c", filename(b"c")).is_some());
        assert!(rank_needle(b"/file.ext", b"ext", filename(b"file.ext")).is_some());
        assert!(rank_needle(b"path/to/file.ext", b"file", filename(b"file.ext")).is_some());
        assert!(rank_needle(b"path/to/file.ext", b"to", filename(b"file.ext")).is_some());
        assert!(rank_needle(b"path/to/file.ext", b"path", filename(b"file.ext")).is_some());
        assert!(rank_needle(b"path/to/file.ext", b"pfile", filename(b"file.ext")).is_some());
        assert!(rank_needle(b"path/to/file.ext", b"ptf", filename(b"file.ext")).is_some());
        assert!(rank_needle(b"path/to/file.ext", b"p/t/f", filename(b"file.ext")).is_some());

        let strict = |name| RankNeedleOptions {
            case_sensitive: false,
            strict_path: true,
            filename: Some(name),
        };
        assert_eq!(rank_needle(b"a/b", b"ab", strict(b"b")), None);
        assert_eq!(rank_needle(b"a/b/c", b"abc", strict(b"c")), None);
        assert_eq!(
            rank_needle(
                b"app/monsters/dungeon/foo/bar/baz.rb",
                b"mod/",
                strict(b"baz.rb")
            ),
            None
        );
        assert_eq!(
            rank_needle(
                b"app/models/foo/bar/baz.rb",
                b"mod/barbaz",
                strict(b"baz.rb")
            ),
            None
        );
        assert_eq!(
            rank_needle(b"/some/path/here", b"/somepath", strict(b"here")),
            None
        );
        assert!(rank_needle(b"a/b/c", b"a/c", strict(b"c")).is_some());
        assert!(rank_needle(b"a/b/c", b"//", strict(b"c")).is_some());
        assert!(rank_needle(b"src/config/__init__.py", b"con/i", strict(b"__init__.py")).is_some());
        assert!(rank_needle(b"a/b/c/d", b"a/b/c", strict(b"d")).is_some());
        assert!(rank_needle(
            b"./app/models/foo/bar/baz.rb",
            b"a/m/f/b/baz",
            strict(b"baz.rb")
        )
        .is_some());
        assert!(rank_needle(
            b"/app/monsters/dungeon/foo/bar/baz.rb",
            b"a/m/f/b/baz",
            strict(b"baz.rb")
        )
        .is_some());
        assert!(rank_needle(
            b"app/models/foo/bar/baz.rb",
            b"mod/baz.rb",
            strict(b"baz.rb")
        )
        .is_some());
    }

    #[test]
    fn zf_highlight_library_interface_matches_upstream() {
        assert_eq!(
            highlight(b"abcdef", &[b"a", b"f"], RankOptions::default()),
            vec![0, 5]
        );
        assert_eq!(
            highlight(b"abcdeF", &[b"a", b"F"], RankOptions::default()),
            vec![0, 5]
        );
        assert_eq!(
            highlight(
                b"a/path/to/file",
                &[b"path", b"file"],
                RankOptions::default()
            ),
            vec![2, 3, 4, 5, 10, 11, 12, 13]
        );
        assert_eq!(
            highlight(
                b"lib/ziglyph/zig.mod",
                &[b"ziglyph"],
                RankOptions::default()
            ),
            vec![4, 5, 6, 7, 8, 9, 10]
        );

        assert_eq!(
            highlight_needle(b"abcdef", b"a", RankNeedleOptions::default()),
            vec![0]
        );
        assert_eq!(
            highlight_needle(b"abcdeF", b"F", RankNeedleOptions::default()),
            vec![5]
        );
        assert_eq!(
            highlight_needle(
                b"a/path/to/file",
                b"file",
                RankNeedleOptions {
                    filename: Some(b"file"),
                    ..RankNeedleOptions::default()
                },
            ),
            vec![10, 11, 12, 13]
        );
        assert_eq!(
            highlight_needle(
                b"s/",
                b"s",
                RankNeedleOptions {
                    filename: Some(b"s"),
                    ..RankNeedleOptions::default()
                },
            ),
            vec![0]
        );
        assert_eq!(
            highlight_needle(
                b"/this/is/path/not/a/file/",
                b"file",
                RankNeedleOptions {
                    filename: Some(b"file"),
                    ..RankNeedleOptions::default()
                },
            ),
            vec![20, 21, 22, 23]
        );
        assert_eq!(
            highlight(b"ababab", &[b"aab"], RankOptions::default()),
            vec![0, 2, 3]
        );
        assert_eq!(
            highlight(b"abbbbbabab", &[b"aab"], RankOptions::default()),
            vec![6, 8, 9]
        );
        assert_eq!(
            highlight(b"abcdefg", &[b"acg"], RankOptions::default()),
            vec![0, 2, 6]
        );
        assert_eq!(
            highlight(b"__init__.py", &[b"initpy"], RankOptions::default()),
            vec![2, 3, 4, 5, 9, 10]
        );
        assert_eq!(
            highlight(b"", &[b"a"], RankOptions::default()),
            Vec::<usize>::new()
        );
        assert_eq!(
            highlight_needle(b"", b"a", RankNeedleOptions::default()),
            Vec::<usize>::new()
        );
        assert_eq!(
            highlight(b"a", &[b""], RankOptions::default()),
            Vec::<usize>::new()
        );
        assert_eq!(
            highlight_needle(b"a", b"", RankNeedleOptions::default()),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn zf_rank_ordering_covers_scoring_rules() {
        let filename_match = rank_bytes(
            "src/foo/bar.rs",
            "bar",
            RankNeedleOptions {
                case_sensitive: false,
                filename: Some(b"bar.rs"),
                ..RankNeedleOptions::default()
            },
        )
        .unwrap();
        let path_fallback = rank_bytes(
            "bar/src/foo.rs",
            "bar",
            RankNeedleOptions {
                case_sensitive: false,
                filename: Some(b"foo.rs"),
                ..RankNeedleOptions::default()
            },
        )
        .unwrap();
        assert!(filename_match < path_fallback);

        let exact_filename = rank_bytes(
            "/tmp/file",
            "file",
            RankNeedleOptions {
                case_sensitive: false,
                filename: Some(b"file"),
                ..RankNeedleOptions::default()
            },
        )
        .unwrap();
        let partial_filename = rank_bytes(
            "/tmp/file_extension_rs",
            "file",
            RankNeedleOptions {
                case_sensitive: false,
                filename: Some(b"file_extension_rs"),
                ..RankNeedleOptions::default()
            },
        )
        .unwrap();
        assert!(exact_filename < partial_filename);

        let word_boundary = rank_bytes("foo-bar", "bar", RankNeedleOptions::default()).unwrap();
        let middle_of_word = rank_bytes("fooxbar", "bar", RankNeedleOptions::default()).unwrap();
        assert!(word_boundary < middle_of_word);

        let sequential = rank_bytes("abc", "abc", RankNeedleOptions::default()).unwrap();
        let scattered = rank_bytes("a_b_c", "abc", RankNeedleOptions::default()).unwrap();
        assert!(sequential < scattered);

        let short_segment = rank_bytes(
            "src/mod/file.rs",
            "mod/file",
            RankNeedleOptions {
                case_sensitive: false,
                strict_path: true,
                filename: Some(b"file.rs"),
            },
        )
        .unwrap();
        let long_segment = rank_bytes(
            "src/module/file.rs",
            "mod/file",
            RankNeedleOptions {
                case_sensitive: false,
                strict_path: true,
                filename: Some(b"file.rs"),
            },
        )
        .unwrap();
        assert!(short_segment < long_segment);
    }

    #[test]
    fn zf_theme_list_plain_case_insensitive_shape_matches_ghostty_call_site() {
        let tokens: &[&[u8]] = &[b"solar", b"dark"];

        let solarized = rank(b"Solarized Dark", tokens, case_insensitive_plain());
        let one_dark = rank(b"One Dark", tokens, case_insensitive_plain());
        let solar_light = rank(b"Solarized Light", tokens, case_insensitive_plain());

        assert!(solarized.is_some());
        assert_eq!(one_dark, None);
        assert_eq!(solar_light, None);
    }
}
