//! Single-step slice rotations and byte search helpers (ports of the relevant upstream `fastmem`
//! and `simd/index_of` surfaces).
//!
//! Upstream also wraps libc `memmove` / `memcpy` (`move` / `copy`) purely to prefer them over the
//! Zig builtins for speed. Rust's `slice::copy_within` / `copy_from_slice` already lower to those
//! intrinsics, so only the rotation helpers are ported here. Upstream `simd/index_of` dispatches
//! to a C++ SIMD helper when available; Roastty uses `memchr`, which provides vector-accelerated
//! search on supported Rust targets.

/// Find the first `needle` byte in `input` (upstream `simd/index_of.indexOf`).
pub(crate) fn index_of(input: &[u8], needle: u8) -> Option<usize> {
    memchr::memchr(needle, input)
}

/// Moves the first item to the end: `0 1 2 3` → `1 2 3 0` (upstream `rotateOnce`). The slice must
/// be non-empty.
pub(crate) fn rotate_once<T>(items: &mut [T]) {
    items.rotate_left(1);
}

/// Moves the last item to the start: `0 1 2 3` → `3 0 1 2` (upstream `rotateOnceR`). The slice must
/// be non-empty.
pub(crate) fn rotate_once_r<T>(items: &mut [T]) {
    items.rotate_right(1);
}

/// Rotates `item` in at the end, returning the displaced first item: rotating `4` into `0 1 2 3`
/// gives `1 2 3 4` and returns `0` (upstream `rotateIn`). The slice must be non-empty.
pub(crate) fn rotate_in<T>(items: &mut [T], item: T) -> T {
    // Put `item` at the front, take the old first out, then rotate it to the end.
    let removed = std::mem::replace(&mut items[0], item);
    items.rotate_left(1);
    removed
}

/// Rotates `item` in at the start, returning the displaced last item: rotating `4` into `0 1 2 3`
/// gives `4 0 1 2` and returns `3` (upstream `rotateInR`). The slice must be non-empty.
pub(crate) fn rotate_in_r<T>(items: &mut [T], item: T) -> T {
    // Put `item` at the back, take the old last out, then rotate it to the front.
    let n = items.len();
    let removed = std::mem::replace(&mut items[n - 1], item);
    items.rotate_right(1);
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_of_finds_no_match() {
        assert_eq!(index_of(b"hello", b' '), None);
    }

    #[test]
    fn index_of_finds_short_match() {
        assert_eq!(index_of(b"hi lo", b' '), Some(2));
    }

    #[test]
    fn index_of_finds_larger_matches() {
        assert_eq!(index_of(b"hello world", b' '), Some(5));
        assert_eq!(
            index_of(
                b"abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz abc",
                b' '
            ),
            Some(52)
        );
    }

    #[test]
    #[ignore = "release-mode perf probe"]
    fn simd_fast_path_perf_index_of() {
        let mut input = vec![b'a'; 1024 * 1024];
        let iterations = 200;

        let scalar_miss = time_iterations(iterations, || {
            assert_eq!(scalar_index_of(&input, b'!'), None);
        });
        let fast_miss = time_iterations(iterations, || {
            assert_eq!(index_of(&input, b'!'), None);
        });
        assert_speedup("index_of_miss", scalar_miss, fast_miss);

        *input.last_mut().unwrap() = b'!';
        let scalar_hit = time_iterations(iterations, || {
            assert_eq!(scalar_index_of(&input, b'!'), Some(input.len() - 1));
        });
        let fast_hit = time_iterations(iterations, || {
            assert_eq!(index_of(&input, b'!'), Some(input.len() - 1));
        });
        assert_speedup("index_of_late_hit", scalar_hit, fast_hit);
    }

    fn scalar_index_of(input: &[u8], needle: u8) -> Option<usize> {
        input.iter().position(|&byte| byte == needle)
    }

    fn time_iterations(iterations: usize, mut f: impl FnMut()) -> std::time::Duration {
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            f();
        }
        start.elapsed()
    }

    fn assert_speedup(label: &str, scalar: std::time::Duration, fast: std::time::Duration) {
        let ratio = scalar.as_secs_f64() / fast.as_secs_f64();
        eprintln!("{label}: scalar={scalar:?} fast={fast:?} ratio={ratio:.2}x");
        assert!(
            ratio >= 1.05,
            "{label} fast path ratio {ratio:.2}x below 1.05x"
        );
    }

    #[test]
    fn rotate_once_moves_first_to_end() {
        let mut items = [0, 1, 2, 3];
        rotate_once(&mut items);
        assert_eq!(items, [1, 2, 3, 0]);
    }

    #[test]
    fn rotate_once_r_moves_last_to_start() {
        let mut items = [0, 1, 2, 3];
        rotate_once_r(&mut items);
        assert_eq!(items, [3, 0, 1, 2]);
    }

    #[test]
    fn rotate_in_appends_and_returns_first() {
        let mut items = [0, 1, 2, 3];
        let removed = rotate_in(&mut items, 4);
        assert_eq!(items, [1, 2, 3, 4]);
        assert_eq!(removed, 0);
    }

    #[test]
    fn rotate_in_r_prepends_and_returns_last() {
        let mut items = [0, 1, 2, 3];
        let removed = rotate_in_r(&mut items, 4);
        assert_eq!(items, [4, 0, 1, 2]);
        assert_eq!(removed, 3);
    }

    #[test]
    fn rotate_once_and_r_are_inverses() {
        let original = [10, 20, 30, 40, 50];

        let mut items = original;
        rotate_once(&mut items);
        rotate_once_r(&mut items);
        assert_eq!(items, original);

        let mut items = original;
        rotate_once_r(&mut items);
        rotate_once(&mut items);
        assert_eq!(items, original);
    }

    #[test]
    fn single_element_is_identity() {
        let mut items = [7];
        rotate_once(&mut items);
        assert_eq!(items, [7]);
        rotate_once_r(&mut items);
        assert_eq!(items, [7]);

        let mut items = [7];
        assert_eq!(rotate_in(&mut items, 9), 7);
        assert_eq!(items, [9]);

        let mut items = [7];
        assert_eq!(rotate_in_r(&mut items, 9), 7);
        assert_eq!(items, [9]);
    }

    #[test]
    fn works_for_non_copy_elements() {
        // The generic (non-`Copy`) surface: rotate owned `String`s.
        let mut items = [String::from("a"), String::from("b"), String::from("c")];
        rotate_once(&mut items);
        assert_eq!(items, ["b", "c", "a"]);

        let removed = rotate_in(&mut items, String::from("d"));
        assert_eq!(items, ["c", "a", "d"]);
        assert_eq!(removed, "b");
    }
}
